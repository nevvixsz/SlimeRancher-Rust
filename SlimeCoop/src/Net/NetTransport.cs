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

        private Socket _socket;
        private Thread _recvThread;
        private volatile bool _running;
        private readonly object _sendLock = new object();
        private readonly Dictionary<IPEndPoint, PeerReliability> _peers = new Dictionary<IPEndPoint, PeerReliability>();
        private readonly ConcurrentQueue<IncomingPacket> _inbox = new ConcurrentQueue<IncomingPacket>();
        private readonly byte[] _recvScratch = new byte[65536];
        private long _packetsSent;
        private long _packetsRecv;

        public int LocalPort { get; private set; }
        public bool IsActive => _running && _socket != null;
        public long PacketsSent => Interlocked.Read(ref _packetsSent);
        public long PacketsReceived => Interlocked.Read(ref _packetsRecv);

        public bool Start(int port)
        {
            Stop();
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
                CoopLog.Info("NetTransport.Start OK: bound 0.0.0.0:" + LocalPort + " (requested " + port + ")");
                return true;
            }
            catch (Exception ex)
            {
                CoopLog.Error("NetTransport.Start(" + port + ") FAILED: " + ex.GetType().Name + ": " + ex.Message);
                Stop();
                return false;
            }
        }

        public void Stop()
        {
            _running = false;
            Interlocked.Exchange(ref _packetsSent, 0);
            Interlocked.Exchange(ref _packetsRecv, 0);
            try
            {
                _socket?.Close();
            }
            catch
            {
            }
            _socket = null;
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
        }

        public void SendUnreliable(IPEndPoint target, PacketType type, PacketWriter payload)
        {
            if (!_running || _socket == null || target == null) return;
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

            SendRaw(target, frame, frame.Length);
        }

        public void SendReliable(IPEndPoint target, PacketType type, PacketWriter payload)
        {
            if (!_running || _socket == null || target == null) return;
            int payloadLen = payload != null ? payload.Length : 0;
            byte[] frame = new byte[HeaderSize + payloadLen];

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
                    Interlocked.Increment(ref _packetsRecv);

                    IPEndPoint sender = (IPEndPoint)remoteEp;
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

                    if (type == PacketType.LanDiscoveryReq || type == PacketType.LanDiscoveryResp)
                    {
                        byte[] copyDisc = new byte[received - HeaderSize];
                        if (copyDisc.Length > 0)
                        {
                            Buffer.BlockCopy(_recvScratch, HeaderSize, copyDisc, 0, copyDisc.Length);
                        }
                        _inbox.Enqueue(new IncomingPacket
                        {
                            Sender = sender,
                            Type = type,
                            Data = copyDisc,
                            Offset = 0,
                            Length = copyDisc.Length
                        });
                        continue;
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
