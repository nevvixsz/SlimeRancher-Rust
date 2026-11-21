using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Net;
using System.Net.Sockets;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using SlimeCoop.Protocol;

namespace SlimeCoop.Net
{
    internal struct IncomingPacket
    {
        public IPEndPoint Sender;
        public PacketType Type;
        public byte[] Data;
        public int Offset;
        public int Length;
    }

    internal sealed class LanServerEntry
    {
        public IPEndPoint EndPoint;
        public string RoomName;
        public int Players;
        public int MaxPlayers;
        public bool HasPassword;
        public DateTime LastSeenUtc;
    }

    internal sealed class NetTransport : IDisposable
    {
        public const byte Magic0 = 0x53; // 'S'
        public const byte Magic1 = 0x43; // 'C'
        public const int HeaderSize = 6; // [0..1] magic, [2] type, [3] flags, [4..5] seq
        private const int MaxTcpFrameSize = 4 * 1024 * 1024;

        private sealed class PendingReliable
        {
            public ushort Seq;
            public byte[] Frame;
            public int Length;
            public long LastSendTicks;
            public int Retries;
        }

        private sealed class PeerReliability
        {
            public ushort NextOutSeq = 1;
            public readonly Dictionary<ushort, PendingReliable> Outgoing = new Dictionary<ushort, PendingReliable>();
            public readonly HashSet<ushort> RecentInSeqs = new HashSet<ushort>();
            public readonly Queue<ushort> RecentInOrder = new Queue<ushort>();
        }

        private sealed class TcpPeerConn
        {
            public IPEndPoint RemoteEndPoint;
            public Socket Socket;
            public readonly object WriteLock = new object();
            public volatile bool Closed;
        }

        private Socket _socket;
        private TcpListener _tcpListener;
        private Thread _recvThread;
        private Thread _tcpAcceptThread;
        private volatile bool _running;
        private readonly object _sendLock = new object();
        private readonly Dictionary<IPEndPoint, PeerReliability> _peers = new Dictionary<IPEndPoint, PeerReliability>();
        private readonly ConcurrentDictionary<IPEndPoint, TcpPeerConn> _tcpPeers = new ConcurrentDictionary<IPEndPoint, TcpPeerConn>();
        private readonly ConcurrentDictionary<IPEndPoint, byte> _tcpJustConnected = new ConcurrentDictionary<IPEndPoint, byte>();
        private readonly ConcurrentQueue<IncomingPacket> _inbox = new ConcurrentQueue<IncomingPacket>();
        private readonly byte[] _recvScratch = new byte[65536];
        private long _packetsSent;
        private long _packetsRecv;
        private volatile string _lastTcpError = string.Empty;
        private volatile bool _tcpCancelled;

        public int LocalPort { get; private set; }
        public bool IsActive => _running && _socket != null;
        public long PacketsSent => Interlocked.Read(ref _packetsSent);
        public long PacketsReceived => Interlocked.Read(ref _packetsRecv);
        public string LastTcpError => _lastTcpError;

        public bool IsTcpPeer(IPEndPoint ep)
        {
            if (ep == null) return false;
            return _tcpPeers.TryGetValue(ep, out TcpPeerConn conn) && conn != null && !conn.Closed;
        }

        public bool ConsumeTcpJustConnected(IPEndPoint ep)
        {
            if (ep == null) return false;
            return _tcpJustConnected.TryRemove(ep, out _);
        }

        public void CancelTcpFallback(IPEndPoint ep)
        {
            _tcpCancelled = true;
            if (ep != null)
            {
                _tcpJustConnected.TryRemove(ep, out _);
                if (_tcpPeers.TryRemove(ep, out TcpPeerConn conn))
                {
                    CloseTcpConn(conn);
                }
            }
        }

        public bool Start(int port)
        {
            Stop();
            _lastTcpError = string.Empty;
            _tcpCancelled = false;
            try
            {
                _socket = new Socket(AddressFamily.InterNetwork, SocketType.Dgram, ProtocolType.Udp);
                _socket.SetSocketOption(SocketOptionLevel.Socket, SocketOptionName.ReuseAddress, true);
                try
                {
                    _socket.EnableBroadcast = true;
                }
                catch
                {
                }
                // Ignore ICMP Port Unreachable exceptions on Windows UDP sockets (SIO_UDP_CONNRESET)
                try
                {
                    const int SIO_UDP_CONNRESET = -1744830452;
                    _socket.IOControl((IOControlCode)SIO_UDP_CONNRESET, new byte[] { 0 }, null);
                }
                catch
                {
                }

                _socket.Bind(new IPEndPoint(IPAddress.Any, port));
                LocalPort = ((IPEndPoint)_socket.LocalEndPoint).Port;
                _running = true;

                _recvThread = new Thread(RecvLoop)
                {
                    IsBackground = true,
                    Name = "SlimeCoop_UdpRecv"
                };
                _recvThread.Start();

                if (port > 0)
                {
                    try
                    {
                        _tcpListener = new TcpListener(IPAddress.Any, LocalPort);
                        _tcpListener.Server.SetSocketOption(SocketOptionLevel.Socket, SocketOptionName.ReuseAddress, true);
                        _tcpListener.Start();
                        _tcpAcceptThread = new Thread(TcpAcceptLoop)
                        {
                            IsBackground = true,
                            Name = "SlimeCoop_TcpAccept"
                        };
                        _tcpAcceptThread.Start();
                        CoopLog.Info("NetTransport.Start OK: bound UDP+TCP 0.0.0.0:" + LocalPort);
                    }
                    catch (Exception tex)
                    {
                        CoopLog.Warn("NetTransport TCP listener failed on :" + LocalPort + " (UDP still active): " + tex.Message);
                    }
                }
                else
                {
                    CoopLog.Info("NetTransport.Start OK: bound 0.0.0.0:" + LocalPort + " (requested " + port + ")");
                }

                return true;
            }
            catch (Exception ex)
            {
                CoopLog.Error("NetTransport.Start(" + port + ") FAILED: " + ex.GetType().Name + ": " + ex.Message);
                Stop();
                return false;
            }
        }

        public void BeginTcpFallback(IPEndPoint target)
        {
            if (!_running || target == null || _tcpCancelled) return;
            if (IsTcpPeer(target)) return;

            _lastTcpError = string.Empty;
            CoopLog.Info("Starting TCP fallback connect to " + target + "...");
            Thread t = new Thread(() =>
            {
                Socket s = null;
                try
                {
                    s = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
                    s.NoDelay = true;
                    IAsyncResult ar = s.BeginConnect(target, null, null);
                    bool signaled = ar.AsyncWaitHandle.WaitOne(3500, false);
                    if (!signaled)
                    {
                        _lastTcpError = "TimedOut";
                        CoopLog.Warn("TCP fallback connect to " + target + " timed out (3.5s)");
                        try { s.Close(); } catch { }
                        return;
                    }

                    s.EndConnect(ar);
                    if (!_running || _tcpCancelled)
                    {
                        try { s.Close(); } catch { }
                        return;
                    }

                    TcpPeerConn conn = new TcpPeerConn
                    {
                        RemoteEndPoint = target,
                        Socket = s
                    };
                    _tcpPeers[target] = conn;
                    _tcpJustConnected[target] = 1;
                    CoopLog.Info("TCP fallback connected to " + target + "!");

                    Thread recv = new Thread(() => TcpRecvLoop(conn))
                    {
                        IsBackground = true,
                        Name = "SlimeCoop_TcpClientRecv"
                    };
                    recv.Start();
                }
                catch (SocketException sex)
                {
                    _lastTcpError = sex.SocketErrorCode.ToString();
                    CoopLog.Warn("TCP fallback connect to " + target + " failed: " + sex.SocketErrorCode + " (" + sex.Message + ")");
                    try { s?.Close(); } catch { }
                }
                catch (Exception ex)
                {
                    _lastTcpError = ex.Message;
                    CoopLog.Warn("TCP fallback connect to " + target + " error: " + ex.Message);
                    try { s?.Close(); } catch { }
                }
            })
            {
                IsBackground = true,
                Name = "SlimeCoop_TcpConnect"
            };
            t.Start();
        }

        public void Stop()
        {
            _running = false;
            Interlocked.Exchange(ref _packetsSent, 0);
            Interlocked.Exchange(ref _packetsRecv, 0);
            _lastTcpError = string.Empty;
            try
            {
                _socket?.Close();
            }
            catch
            {
            }
            _socket = null;

            try
            {
                _tcpListener?.Stop();
            }
            catch
            {
            }
            _tcpListener = null;

            foreach (KeyValuePair<IPEndPoint, TcpPeerConn> kv in _tcpPeers)
            {
                CloseTcpConn(kv.Value);
            }
            _tcpPeers.Clear();
            _tcpJustConnected.Clear();

            lock (_sendLock)
            {
                _peers.Clear();
            }
            while (_inbox.TryDequeue(out _)) { }
        }

        public void Dispose()
        {
            Stop();
        }

        public void RemovePeer(IPEndPoint ep)
        {
            if (ep == null) return;
            lock (_sendLock)
            {
                _peers.Remove(ep);
            }
            if (_tcpPeers.TryRemove(ep, out TcpPeerConn conn))
            {
                CloseTcpConn(conn);
            }
            _tcpJustConnected.TryRemove(ep, out _);
        }

        public void SendUnreliable(IPEndPoint target, PacketType type, PacketWriter payload)
        {
            if (!_running || target == null) return;
            int payloadLen = payload != null ? payload.Length : 0;
            byte[] frame = new byte[HeaderSize + payloadLen];
            frame[0] = Magic0;
            frame[1] = Magic1;
            frame[2] = (byte)type;
            frame[3] = 0;
            frame[4] = 0;
            frame[5] = 0;
            if (payloadLen > 0)
            {
                Buffer.BlockCopy(payload.Buffer, 0, frame, HeaderSize, payloadLen);
            }

            if (_tcpPeers.TryGetValue(target, out TcpPeerConn tcp) && tcp != null && !tcp.Closed)
            {
                SendTcpFrame(tcp, frame, frame.Length);
                return;
            }

            if (_socket == null) return;
            SendRaw(target, frame, frame.Length);
        }

        public void SendReliable(IPEndPoint target, PacketType type, PacketWriter payload)
        {
            if (!_running || target == null) return;
            int payloadLen = payload != null ? payload.Length : 0;
            byte[] frame = new byte[HeaderSize + payloadLen];

            if (_tcpPeers.TryGetValue(target, out TcpPeerConn tcp) && tcp != null && !tcp.Closed)
            {
                frame[0] = Magic0;
                frame[1] = Magic1;
                frame[2] = (byte)type;
                frame[3] = 0; // TCP already guarantees delivery & ordering
                frame[4] = 0;
                frame[5] = 0;
                if (payloadLen > 0)
                {
                    Buffer.BlockCopy(payload.Buffer, 0, frame, HeaderSize, payloadLen);
                }
                SendTcpFrame(tcp, frame, frame.Length);
                return;
            }

            if (_socket == null) return;
            ushort seq;
            lock (_sendLock)
            {
                PeerReliability peer = GetOrCreatePeer(target);
                seq = peer.NextOutSeq++;
                if (peer.NextOutSeq == 0) peer.NextOutSeq = 1;

                frame[0] = Magic0;
                frame[1] = Magic1;
                frame[2] = (byte)type;
                frame[3] = 1;
                frame[4] = (byte)(seq & 0xFF);
                frame[5] = (byte)((seq >> 8) & 0xFF);
                if (payloadLen > 0)
                {
                    Buffer.BlockCopy(payload.Buffer, 0, frame, HeaderSize, payloadLen);
                }

                peer.Outgoing[seq] = new PendingReliable
                {
                    Seq = seq,
                    Frame = frame,
                    Length = frame.Length,
                    LastSendTicks = DateTime.UtcNow.Ticks,
                    Retries = 0
                };
            }

            SendRaw(target, frame, frame.Length);
        }

        public void BroadcastDiscovery(int targetPort)
        {
            if (!_running || _socket == null) return;
            byte[] frame = new byte[HeaderSize];
            frame[0] = Magic0;
            frame[1] = Magic1;
            frame[2] = (byte)PacketType.LanDiscoveryReq;
            frame[3] = 0;
            frame[4] = 0;
            frame[5] = 0;

            try
            {
                _socket.SendTo(frame, frame.Length, SocketFlags.None, new IPEndPoint(IPAddress.Broadcast, targetPort));
                CoopLog.Info("BroadcastDiscovery -> 255.255.255.255:" + targetPort);
            }
            catch (Exception ex)
            {
                CoopLog.Warn("BroadcastDiscovery 255.255.255.255:" + targetPort + " error: " + ex.Message);
            }

            List<IPAddress> subnetBroadcasts = RoomCode.GetSubnetBroadcastAddresses();
            for (int i = 0; i < subnetBroadcasts.Count; i++)
            {
                try
                {
                    _socket.SendTo(frame, frame.Length, SocketFlags.None, new IPEndPoint(subnetBroadcasts[i], targetPort));
                    CoopLog.Info("BroadcastDiscovery -> " + subnetBroadcasts[i] + ":" + targetPort);
                }
                catch (Exception ex)
                {
                    CoopLog.Warn("BroadcastDiscovery " + subnetBroadcasts[i] + ":" + targetPort + " error: " + ex.Message);
                }
            }
        }

        public void PumpReliableRetries()
        {
            if (!_running || _socket == null) return;
            long now = DateTime.UtcNow.Ticks;
            long retryThreshold = TimeSpan.FromMilliseconds(130).Ticks;

            List<KeyValuePair<IPEndPoint, PendingReliable>> toResend = null;
            lock (_sendLock)
            {
                foreach (KeyValuePair<IPEndPoint, PeerReliability> kv in _peers)
                {
                    if (_tcpPeers.TryGetValue(kv.Key, out TcpPeerConn tcp) && tcp != null && !tcp.Closed)
                    {
                        if (kv.Value.Outgoing.Count > 0) kv.Value.Outgoing.Clear();
                        continue;
                    }

                    PeerReliability peer = kv.Value;
                    if (peer.Outgoing.Count == 0) continue;

                    List<ushort> expired = null;
                    foreach (PendingReliable pr in peer.Outgoing.Values)
                    {
                        if (now - pr.LastSendTicks >= retryThreshold)
                        {
                            if (pr.Retries >= 45)
                            {
                                if (expired == null) expired = new List<ushort>();
                                expired.Add(pr.Seq);
                                continue;
                            }
                            if (toResend == null) toResend = new List<KeyValuePair<IPEndPoint, PendingReliable>>();
                            pr.LastSendTicks = now;
                            pr.Retries++;
                            toResend.Add(new KeyValuePair<IPEndPoint, PendingReliable>(kv.Key, pr));
                        }
                    }

                    if (expired != null)
                    {
                        for (int i = 0; i < expired.Count; i++)
                        {
                            ushort expSeq = expired[i];
                            if (peer.Outgoing.TryGetValue(expSeq, out PendingReliable prExp) && prExp.Length > 2)
                            {
                                CoopLog.Warn("Reliable packet timeout (no ACK after 45 retries): to=" + kv.Key + ", type=" + (PacketType)prExp.Frame[2] + ", seq=" + expSeq);
                            }
                            peer.Outgoing.Remove(expSeq);
                        }
                    }
                }
            }

            if (toResend != null)
            {
                for (int i = 0; i < toResend.Count; i++)
                {
                    SendRaw(toResend[i].Key, toResend[i].Value.Frame, toResend[i].Value.Length);
                }
            }
        }

        public bool TryDequeue(out IncomingPacket pkt)
        {
            return _inbox.TryDequeue(out pkt);
        }

        private PeerReliability GetOrCreatePeer(IPEndPoint ep)
        {
            if (!_peers.TryGetValue(ep, out PeerReliability peer))
            {
                peer = new PeerReliability();
                _peers[ep] = peer;
            }
            return peer;
        }

        private void SendAck(IPEndPoint target, ushort seq)
        {
            byte[] ack = new byte[HeaderSize];
            ack[0] = Magic0;
            ack[1] = Magic1;
            ack[2] = (byte)PacketType.ReliableAck;
            ack[3] = 0;
            ack[4] = (byte)(seq & 0xFF);
            ack[5] = (byte)((seq >> 8) & 0xFF);
            SendRaw(target, ack, ack.Length);
        }

        private void SendRaw(IPEndPoint target, byte[] data, int len)
        {
            Socket s = _socket;
            if (s == null || !_running) return;
            try
            {
                s.SendTo(data, len, SocketFlags.None, target);
                Interlocked.Increment(ref _packetsSent);
            }
            catch (SocketException sex)
            {
                CoopLog.Warn("SendRaw SocketException to " + target + ": code=" + sex.SocketErrorCode + " (" + sex.Message + ")");
            }
            catch (Exception ex)
            {
                CoopLog.Warn("SendRaw error to " + target + ": " + ex.Message);
            }
        }

        private void SendTcpFrame(TcpPeerConn conn, byte[] frame, int len)
        {
            if (conn == null || conn.Closed || !_running) return;
            byte[] packet = new byte[4 + len];
            packet[0] = (byte)(len & 0xFF);
            packet[1] = (byte)((len >> 8) & 0xFF);
            packet[2] = (byte)((len >> 16) & 0xFF);
            packet[3] = (byte)((len >> 24) & 0xFF);
            Buffer.BlockCopy(frame, 0, packet, 4, len);

            lock (conn.WriteLock)
            {
                if (conn.Closed || conn.Socket == null) return;
                try
                {
                    int sent = 0;
                    while (sent < packet.Length)
                    {
                        int n = conn.Socket.Send(packet, sent, packet.Length - sent, SocketFlags.None);
                        if (n <= 0)
                        {
                            throw new SocketException((int)SocketError.ConnectionReset);
                        }
                        sent += n;
                    }
                    Interlocked.Increment(ref _packetsSent);
                }
                catch (Exception ex)
                {
                    CoopLog.Warn("SendTcpFrame to " + conn.RemoteEndPoint + " error: " + ex.Message);
                    CloseTcpConn(conn);
                }
            }
        }

        private static void CloseTcpConn(TcpPeerConn conn)
        {
            if (conn == null || conn.Closed) return;
            conn.Closed = true;
            try
            {
                conn.Socket?.Shutdown(SocketShutdown.Both);
            }
            catch
            {
            }
            try
            {
                conn.Socket?.Close();
            }
            catch
            {
            }
        }

        private void TcpAcceptLoop()
        {
            while (_running && _tcpListener != null)
            {
                try
                {
                    Socket clientSock = _tcpListener.AcceptSocket();
                    if (!_running)
                    {
                        try { clientSock.Close(); } catch { }
                        break;
                    }

                    clientSock.NoDelay = true;
                    IPEndPoint remote = clientSock.RemoteEndPoint as IPEndPoint;
                    if (remote == null)
                    {
                        try { clientSock.Close(); } catch { }
                        continue;
                    }

                    TcpPeerConn conn = new TcpPeerConn
                    {
                        RemoteEndPoint = remote,
                        Socket = clientSock
                    };
                    _tcpPeers[remote] = conn;
                    CoopLog.Info("Host accepted TCP client from " + remote);

                    Thread recv = new Thread(() => TcpRecvLoop(conn))
                    {
                        IsBackground = true,
                        Name = "SlimeCoop_TcpHostRecv"
                    };
                    recv.Start();
                }
                catch (SocketException)
                {
                    if (!_running) break;
                }
                catch (Exception ex)
                {
                    if (!_running) break;
                    CoopLog.Warn("TcpAcceptLoop error: " + ex.Message);
                }
            }
        }

        private void TcpRecvLoop(TcpPeerConn conn)
        {
            byte[] lenBuf = new byte[4];
            try
            {
                while (_running && !conn.Closed)
                {
                    if (!ReadExact(conn.Socket, lenBuf, 4)) break;
                    int frameLen = lenBuf[0] | (lenBuf[1] << 8) | (lenBuf[2] << 16) | (lenBuf[3] << 24);
                    if (frameLen < HeaderSize || frameLen > MaxTcpFrameSize)
                    {
                        CoopLog.Warn("TcpRecvLoop invalid frameLen=" + frameLen + " from " + conn.RemoteEndPoint);
                        break;
                    }

                    byte[] frame = new byte[frameLen];
                    if (!ReadExact(conn.Socket, frame, frameLen)) break;

                    if (frame[0] != Magic0 || frame[1] != Magic1) continue;
                    Interlocked.Increment(ref _packetsRecv);

                    PacketType type = (PacketType)frame[2];
                    if (type == PacketType.ReliableAck) continue;

                    int payloadLen = frameLen - HeaderSize;
                    byte[] copy = new byte[payloadLen];
                    if (payloadLen > 0)
                    {
                        Buffer.BlockCopy(frame, HeaderSize, copy, 0, payloadLen);
                    }

                    _inbox.Enqueue(new IncomingPacket
                    {
                        Sender = conn.RemoteEndPoint,
                        Type = type,
                        Data = copy,
                        Offset = 0,
                        Length = payloadLen
                    });
                }
            }
            catch (Exception ex)
            {
                if (_running && !conn.Closed)
                {
                    CoopLog.Warn("TcpRecvLoop error (" + conn.RemoteEndPoint + "): " + ex.Message);
                }
            }
            finally
            {
                bool wasActive = !conn.Closed;
                CloseTcpConn(conn);
                if (_tcpPeers.TryGetValue(conn.RemoteEndPoint, out TcpPeerConn current) && ReferenceEquals(current, conn))
                {
                    _tcpPeers.TryRemove(conn.RemoteEndPoint, out _);
                }

                if (_running && wasActive)
                {
                    // Notify session layer that the TCP peer disconnected
                    byte[] reasonBytes = Encoding.UTF8.GetBytes("TCP-соединение закрыто");
                    byte[] discPayload = new byte[2 + reasonBytes.Length];
                    discPayload[0] = (byte)(reasonBytes.Length & 0xFF);
                    discPayload[1] = (byte)((reasonBytes.Length >> 8) & 0xFF);
                    Buffer.BlockCopy(reasonBytes, 0, discPayload, 2, reasonBytes.Length);
                    _inbox.Enqueue(new IncomingPacket
                    {
                        Sender = conn.RemoteEndPoint,
                        Type = PacketType.Disconnect,
                        Data = discPayload,
                        Offset = 0,
                        Length = discPayload.Length
                    });
                }
            }
        }

        private bool ReadExact(Socket s, byte[] buf, int count)
        {
            int read = 0;
            while (read < count)
            {
                if (!_running) return false;
                int n = s.Receive(buf, read, count - read, SocketFlags.None);
                if (n <= 0) return false;
                read += n;
            }
            return true;
        }

        private void RecvLoop()
        {
            EndPoint remoteEp = new IPEndPoint(IPAddress.Any, 0);
            while (_running && _socket != null)
            {
                try
                {
                    if (!_socket.Poll(10000, SelectMode.SelectRead))
                    {
                        continue;
                    }

                    int received = _socket.ReceiveFrom(_recvScratch, 0, _recvScratch.Length, SocketFlags.None, ref remoteEp);
                    if (received < HeaderSize) continue;
                    if (_recvScratch[0] != Magic0 || _recvScratch[1] != Magic1) continue;

                    IPEndPoint sender = (IPEndPoint)remoteEp;
                    if (IsTcpPeer(sender)) continue;

                    Interlocked.Increment(ref _packetsRecv);

                    PacketType type = (PacketType)_recvScratch[2];
                    byte flags = _recvScratch[3];
                    ushort seq = (ushort)(_recvScratch[4] | (_recvScratch[5] << 8));

                    if (type == PacketType.ReliableAck)
                    {
                        lock (_sendLock)
                        {
                            if (_peers.TryGetValue(sender, out PeerReliability peer))
                            {
                                peer.Outgoing.Remove(seq);
                            }
                        }
                        continue;
                    }

                    if ((flags & 1) != 0)
                    {
                        SendAck(sender, seq);
                        bool duplicate;
                        lock (_sendLock)
                        {
                            PeerReliability peer = GetOrCreatePeer(sender);
                            if (peer.RecentInSeqs.Contains(seq))
                            {
                                duplicate = true;
                            }
                            else
                            {
                                duplicate = false;
                                peer.RecentInSeqs.Add(seq);
                                peer.RecentInOrder.Enqueue(seq);
                                if (peer.RecentInOrder.Count > 128)
                                {
                                    ushort old = peer.RecentInOrder.Dequeue();
                                    peer.RecentInSeqs.Remove(old);
                                }
                            }
                        }

                        if (duplicate)
                        {
                            continue;
                        }
                    }

                    int payloadLen = received - HeaderSize;
                    byte[] copy = new byte[payloadLen];
                    if (payloadLen > 0)
                    {
                        Buffer.BlockCopy(_recvScratch, HeaderSize, copy, 0, payloadLen);
                    }

                    _inbox.Enqueue(new IncomingPacket
                    {
                        Sender = sender,
                        Type = type,
                        Data = copy,
                        Offset = 0,
                        Length = payloadLen
                    });
                }
                catch (SocketException sex)
                {
                    if (!_running) break;
                    CoopLog.Warn("RecvLoop SocketException: code=" + sex.SocketErrorCode + " (" + sex.Message + ")");
                }
                catch (Exception ex)
                {
                    if (!_running) break;
                    CoopLog.Warn("RecvLoop error: " + ex.Message);
                }
            }
        }

        public static string ComputePasswordHash(string challenge, string password)
        {
            string input = (challenge ?? string.Empty) + ":" + (password ?? string.Empty);
            using (SHA256 sha = SHA256.Create())
            {
                byte[] bytes = sha.ComputeHash(Encoding.UTF8.GetBytes(input));
                StringBuilder sb = new StringBuilder(bytes.Length * 2);
                for (int i = 0; i < bytes.Length; i++)
                {
                    sb.Append(bytes[i].ToString("x2"));
                }
                return sb.ToString();
            }
        }
    }
}
