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
        public bool Pooled;
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
            public byte[] WriteScratch = new byte[2048];
            public volatile bool Closed;
        }

        private const int PoolMaxPerBucket = 64;
        private static readonly byte[] EmptyBytes = new byte[0];

        private Socket _socket;
        private TcpListener _tcpListener;
        private Thread _recvThread;
        private Thread _tcpAcceptThread;
        private volatile bool _running;
        private readonly object _sendLock = new object();
        private readonly object _udpSendLock = new object();
        private readonly object _poolLock = new object();
        private readonly Dictionary<IPEndPoint, PeerReliability> _peers = new Dictionary<IPEndPoint, PeerReliability>();
        private readonly ConcurrentDictionary<IPEndPoint, TcpPeerConn> _tcpPeers = new ConcurrentDictionary<IPEndPoint, TcpPeerConn>();
        private readonly ConcurrentDictionary<IPEndPoint, byte> _tcpJustConnected = new ConcurrentDictionary<IPEndPoint, byte>();
        private readonly ConcurrentQueue<IncomingPacket> _inbox = new ConcurrentQueue<IncomingPacket>();
        private readonly ConcurrentQueue<PendingReliable> _ackRecycleQueue = new ConcurrentQueue<PendingReliable>();
        private readonly Stack<PendingReliable> _pendingPool = new Stack<PendingReliable>(64);
        private readonly List<KeyValuePair<IPEndPoint, PendingReliable>> _retryScratch = new List<KeyValuePair<IPEndPoint, PendingReliable>>(16);
        private readonly List<ushort> _expiredScratch = new List<ushort>(8);
        private readonly Stack<byte[]> _pool128 = new Stack<byte[]>(PoolMaxPerBucket);
        private readonly Stack<byte[]> _pool1536 = new Stack<byte[]>(PoolMaxPerBucket);
        private readonly Stack<byte[]> _pool8192 = new Stack<byte[]>(PoolMaxPerBucket);
        private readonly Stack<byte[]> _pool65536 = new Stack<byte[]>(PoolMaxPerBucket);
        private readonly byte[] _recvScratch = new byte[65536];
        private readonly byte[] _ackScratch = new byte[HeaderSize];
        private byte[] _udpSendScratch = new byte[2048];
        private int _pendingReliableCount;
        private long _packetsSent;
        private long _packetsRecv;
        private volatile string _lastTcpError = string.Empty;
        private volatile bool _tcpCancelled;

        private byte[] RentBuffer(int minSize)
        {
            if (minSize <= 0) return EmptyBytes;
            lock (_poolLock)
            {
                if (minSize <= 128)
                {
                    return _pool128.Count > 0 ? _pool128.Pop() : new byte[128];
                }
                if (minSize <= 1536)
                {
                    return _pool1536.Count > 0 ? _pool1536.Pop() : new byte[1536];
                }
                if (minSize <= 8192)
                {
                    return _pool8192.Count > 0 ? _pool8192.Pop() : new byte[8192];
                }
                if (minSize <= 65536)
                {
                    return _pool65536.Count > 0 ? _pool65536.Pop() : new byte[65536];
                }
            }
            return new byte[minSize];
        }

        private void ReturnBuffer(byte[] buf)
        {
            if (buf == null) return;
            int len = buf.Length;
            if (len != 128 && len != 1536 && len != 8192 && len != 65536) return;
            lock (_poolLock)
            {
                if (len == 128)
                {
                    if (_pool128.Count < PoolMaxPerBucket) _pool128.Push(buf);
                }
                else if (len == 1536)
                {
                    if (_pool1536.Count < PoolMaxPerBucket) _pool1536.Push(buf);
                }
                else if (len == 8192)
                {
                    if (_pool8192.Count < PoolMaxPerBucket) _pool8192.Push(buf);
                }
                else
                {
                    if (_pool65536.Count < PoolMaxPerBucket) _pool65536.Push(buf);
                }
            }
        }

        public void ReturnIncomingPacket(ref IncomingPacket pkt)
        {
            if (pkt.Pooled && pkt.Data != null)
            {
                ReturnBuffer(pkt.Data);
            }
            pkt.Data = null;
            pkt.Pooled = false;
        }

        private PendingReliable RentPending()
        {
            return _pendingPool.Count > 0 ? _pendingPool.Pop() : new PendingReliable();
        }

        private void ReturnPending(PendingReliable pr)
        {
            if (pr == null) return;
            if (pr.Frame != null)
            {
                ReturnBuffer(pr.Frame);
                pr.Frame = null;
            }
            pr.Length = 0;
            pr.Retries = 0;
            if (_pendingPool.Count < 128)
            {
                _pendingPool.Push(pr);
            }
        }

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
                foreach (PeerReliability peer in _peers.Values)
                {
                    foreach (PendingReliable pr in peer.Outgoing.Values)
                    {
                        ReturnPending(pr);
                    }
                    peer.Outgoing.Clear();
                }
                _peers.Clear();
            }
            Interlocked.Exchange(ref _pendingReliableCount, 0);
            while (_ackRecycleQueue.TryDequeue(out PendingReliable recycled))
            {
                lock (_sendLock)
                {
                    ReturnPending(recycled);
                }
            }
            while (_inbox.TryDequeue(out IncomingPacket leftover))
            {
                ReturnIncomingPacket(ref leftover);
            }
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
                if (_peers.TryGetValue(ep, out PeerReliability peer))
                {
                    int removed = peer.Outgoing.Count;
                    foreach (PendingReliable pr in peer.Outgoing.Values)
                    {
                        ReturnPending(pr);
                    }
                    peer.Outgoing.Clear();
                    if (removed > 0)
                    {
                        Interlocked.Add(ref _pendingReliableCount, -removed);
                    }
                    _peers.Remove(ep);
                }
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

            if (_tcpPeers.TryGetValue(target, out TcpPeerConn tcp) && tcp != null && !tcp.Closed)
            {
                SendTcpFrame(tcp, type, 0, 0, payload != null ? payload.Buffer : null, payloadLen);
                return;
            }

            if (_socket == null) return;
            int frameLen = HeaderSize + payloadLen;
            lock (_udpSendLock)
            {
                if (frameLen > _udpSendScratch.Length)
                {
                    _udpSendScratch = new byte[Math.Max(frameLen, _udpSendScratch.Length * 2)];
                }
                _udpSendScratch[0] = Magic0;
                _udpSendScratch[1] = Magic1;
                _udpSendScratch[2] = (byte)type;
                _udpSendScratch[3] = 0;
                _udpSendScratch[4] = 0;
                _udpSendScratch[5] = 0;
                if (payloadLen > 0)
                {
                    Buffer.BlockCopy(payload.Buffer, 0, _udpSendScratch, HeaderSize, payloadLen);
                }
                SendRaw(target, _udpSendScratch, frameLen);
            }
        }

        public void SendReliable(IPEndPoint target, PacketType type, PacketWriter payload)
        {
            if (!_running || target == null) return;
            int payloadLen = payload != null ? payload.Length : 0;

            if (_tcpPeers.TryGetValue(target, out TcpPeerConn tcp) && tcp != null && !tcp.Closed)
            {
                SendTcpFrame(tcp, type, 0, 0, payload != null ? payload.Buffer : null, payloadLen);
                return;
            }

            if (_socket == null) return;
            int frameLen = HeaderSize + payloadLen;
            byte[] frame = RentBuffer(frameLen);
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

                PendingReliable pr = RentPending();
                pr.Seq = seq;
                pr.Frame = frame;
                pr.Length = frameLen;
                pr.LastSendTicks = DateTime.UtcNow.Ticks;
                pr.Retries = 0;
                peer.Outgoing[seq] = pr;
                Interlocked.Increment(ref _pendingReliableCount);
            }

            SendRaw(target, frame, frameLen);
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

            if (!_ackRecycleQueue.IsEmpty)
            {
                lock (_sendLock)
                {
                    while (_ackRecycleQueue.TryDequeue(out PendingReliable recycled))
                    {
                        ReturnPending(recycled);
                    }
                }
            }

            if (Volatile.Read(ref _pendingReliableCount) <= 0) return;

            long now = DateTime.UtcNow.Ticks;
            long retryThreshold = TimeSpan.FromMilliseconds(130).Ticks;

            _retryScratch.Clear();
            lock (_sendLock)
            {
                foreach (KeyValuePair<IPEndPoint, PeerReliability> kv in _peers)
                {
                    PeerReliability peer = kv.Value;
                    if (peer.Outgoing.Count == 0) continue;

                    if (_tcpPeers.TryGetValue(kv.Key, out TcpPeerConn tcp) && tcp != null && !tcp.Closed)
                    {
                        int cleared = peer.Outgoing.Count;
                        foreach (PendingReliable pr in peer.Outgoing.Values)
                        {
                            ReturnPending(pr);
                        }
                        peer.Outgoing.Clear();
                        if (cleared > 0)
                        {
                            Interlocked.Add(ref _pendingReliableCount, -cleared);
                        }
                        continue;
                    }

                    _expiredScratch.Clear();
                    foreach (PendingReliable pr in peer.Outgoing.Values)
                    {
                        if (now - pr.LastSendTicks >= retryThreshold)
                        {
                            if (pr.Retries >= 45)
                            {
                                _expiredScratch.Add(pr.Seq);
                                continue;
                            }
                            pr.LastSendTicks = now;
                            pr.Retries++;
                            _retryScratch.Add(new KeyValuePair<IPEndPoint, PendingReliable>(kv.Key, pr));
                        }
                    }

                    if (_expiredScratch.Count > 0)
                    {
                        for (int i = 0; i < _expiredScratch.Count; i++)
                        {
                            ushort expSeq = _expiredScratch[i];
                            if (peer.Outgoing.TryGetValue(expSeq, out PendingReliable prExp))
                            {
                                if (prExp.Length > 2)
                                {
                                    CoopLog.Warn("Reliable packet timeout (no ACK after 45 retries): to=" + kv.Key + ", type=" + (PacketType)prExp.Frame[2] + ", seq=" + expSeq);
                                }
                                peer.Outgoing.Remove(expSeq);
                                Interlocked.Decrement(ref _pendingReliableCount);
                                ReturnPending(prExp);
                            }
                        }
                        _expiredScratch.Clear();
                    }
                }
            }

            if (_retryScratch.Count > 0)
            {
                for (int i = 0; i < _retryScratch.Count; i++)
                {
                    PendingReliable pr = _retryScratch[i].Value;
                    if (pr.Frame != null && pr.Length > 0)
                    {
                        SendRaw(_retryScratch[i].Key, pr.Frame, pr.Length);
                    }
                }
                _retryScratch.Clear();
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
            _ackScratch[0] = Magic0;
            _ackScratch[1] = Magic1;
            _ackScratch[2] = (byte)PacketType.ReliableAck;
            _ackScratch[3] = 0;
            _ackScratch[4] = (byte)(seq & 0xFF);
            _ackScratch[5] = (byte)((seq >> 8) & 0xFF);
            SendRaw(target, _ackScratch, HeaderSize);
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

        private void SendTcpFrame(TcpPeerConn conn, PacketType type, byte flags, ushort seq, byte[] payload, int payloadLen)
        {
            if (conn == null || conn.Closed || !_running) return;
            int frameLen = HeaderSize + payloadLen;
            int totalLen = 4 + frameLen;

            lock (conn.WriteLock)
            {
                if (conn.Closed || conn.Socket == null) return;
                if (totalLen > conn.WriteScratch.Length)
                {
                    conn.WriteScratch = new byte[Math.Max(totalLen, conn.WriteScratch.Length * 2)];
                }

                byte[] packet = conn.WriteScratch;
                packet[0] = (byte)(frameLen & 0xFF);
                packet[1] = (byte)((frameLen >> 8) & 0xFF);
                packet[2] = (byte)((frameLen >> 16) & 0xFF);
                packet[3] = (byte)((frameLen >> 24) & 0xFF);
                packet[4] = Magic0;
                packet[5] = Magic1;
                packet[6] = (byte)type;
                packet[7] = flags;
                packet[8] = (byte)(seq & 0xFF);
                packet[9] = (byte)((seq >> 8) & 0xFF);
                if (payloadLen > 0 && payload != null)
                {
                    Buffer.BlockCopy(payload, 0, packet, 4 + HeaderSize, payloadLen);
                }

                try
                {
                    int sent = 0;
                    while (sent < totalLen)
                    {
                        int n = conn.Socket.Send(packet, sent, totalLen - sent, SocketFlags.None);
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
            byte[] frame = new byte[4096];
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

                    if (frameLen > frame.Length)
                    {
                        frame = new byte[Math.Max(frameLen, frame.Length * 2)];
                    }
                    if (!ReadExact(conn.Socket, frame, frameLen)) break;

                    if (frame[0] != Magic0 || frame[1] != Magic1) continue;
                    Interlocked.Increment(ref _packetsRecv);

                    PacketType type = (PacketType)frame[2];
                    if (type == PacketType.ReliableAck) continue;

                    int payloadLen = frameLen - HeaderSize;
                    byte[] copy = payloadLen > 0 ? RentBuffer(payloadLen) : EmptyBytes;
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
                        Length = payloadLen,
                        Pooled = payloadLen > 0
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
                            if (_peers.TryGetValue(sender, out PeerReliability peer) &&
                                peer.Outgoing.TryGetValue(seq, out PendingReliable pr))
                            {
                                peer.Outgoing.Remove(seq);
                                Interlocked.Decrement(ref _pendingReliableCount);
                                _ackRecycleQueue.Enqueue(pr);
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
                    byte[] copy = payloadLen > 0 ? RentBuffer(payloadLen) : EmptyBytes;
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
                        Length = payloadLen,
                        Pooled = payloadLen > 0
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
