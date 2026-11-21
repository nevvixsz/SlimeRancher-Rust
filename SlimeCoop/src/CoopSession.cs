using System;
using System.Collections.Generic;
using System.Net;
using MonomiPark.SlimeRancher.DataModel;
using MonomiPark.SlimeRancher.Regions;
using SlimeCoop.Net;
using SlimeCoop.Protocol;
using SlimeCoop.World;
using UnityEngine;

namespace SlimeCoop
{
    internal enum CoopRole
    {
        None = 0,
        Host = 1,
        Client = 2
    }

    internal sealed class CoopPeerInfo
    {
        public int PlayerId;
        public IPEndPoint EndPoint;
        public string Name;
        public int ColorIndex;
        public ZoneDirector.Zone CurrentZone = ZoneDirector.Zone.RANCH;
        public long LastHeardTicks;
        public bool Authenticated;
    }

    internal sealed class CoopChatEntry
    {
        public string Sender;
        public string Text;
        public bool IsSystem;
    }

    internal static class CoopSession
    {
        public static CoopRole Role { get; private set; } = CoopRole.None;
        public static bool IsActive => Role != CoopRole.None;
        public static bool IsHost => Role == CoopRole.Host;
        public static bool IsClient => Role == CoopRole.Client;

        public static string LocalNick = "SlimePlayer";
        public static int LocalColorIndex = 0;
        public static int HostPort = 7777;
        public static string HostPassword = string.Empty;
        public static string RoomName = "Моё Ранчо";
        public static int MaxPlayers = 4;
        public static bool UseUpnp = true;
        public static bool ShareMarketIncome = true;

        public static string HostServerId = string.Empty;
        public static int LocalPlayerId = 1;
        public static string StatusMessage = "Не подключено";
        public static int ClientPingMs;
        public static bool WaitingWorldLoad;
        public static long PacketsSent => Transport.PacketsSent;
        public static long PacketsReceived => Transport.PacketsReceived;

        public static readonly List<CoopPeerInfo> ConnectedPeers = new List<CoopPeerInfo>();
        public static readonly List<LanServerEntry> DiscoveredServers = new List<LanServerEntry>();
        public static readonly List<CoopChatEntry> ChatLog = new List<CoopChatEntry>();
        public static readonly List<string> HostRoomCodes = new List<string>();
        public static readonly List<NetworkEndpointEntry> HostEndpoints = new List<NetworkEndpointEntry>();

        private static readonly NetTransport Transport = new NetTransport();
        private static readonly PacketWriter SharedWriter = new PacketWriter(1400);
        private static readonly Dictionary<IPEndPoint, CoopPeerInfo> PeersByEp = new Dictionary<IPEndPoint, CoopPeerInfo>();
        private static readonly Dictionary<IPEndPoint, string> PendingChallenges = new Dictionary<IPEndPoint, string>();

        private static IPEndPoint _serverEp;
        private static string _pendingJoinPass = string.Empty;
        private static int _nextPlayerId = 2;

        private static float _nextTransformSendTime;
        private static float _nextActorBatchTime;
        private static float _nextTimeSyncTime;
        private static float _nextKeepAliveTime;
        private static float _nextProfileAutoSaveTime;
        private static bool _clientProfileLoaded;
        private static bool _clientHandshakeDone;
        private static bool _clientSnapshotDone;
        private static float _connectStartTime;
        private static float _nextHelloRetryTime;
        private static float _lastDiscoveryTime;
        private static PlayerZoneTracker _cachedZoneTracker;

        public static bool IsInGameSession()
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.Player == null) return false;
            string sceneName = UnityEngine.SceneManagement.SceneManager.GetActiveScene().name;
            return !string.Equals(sceneName, "MainMenu", StringComparison.OrdinalIgnoreCase);
        }

        public static bool CanClientSendGameplaySync =>
            IsClient && _clientHandshakeDone && !WaitingWorldLoad && _clientSnapshotDone;

        private static ZoneDirector.Zone GetLocalZone(SceneContext ctx)
        {
            if (ctx == null || ctx.Player == null) return ZoneDirector.Zone.RANCH;
            if (_cachedZoneTracker == null)
            {
                _cachedZoneTracker = ctx.Player.GetComponent<PlayerZoneTracker>();
            }
            return _cachedZoneTracker != null ? _cachedZoneTracker.GetCurrentZone() : ZoneDirector.Zone.RANCH;
        }

        public static string GetPrimaryRoomCode()
        {
            if (!string.IsNullOrEmpty(UpnpMapper.ExternalIp))
            {
                return RoomCode.Encode(UpnpMapper.ExternalIp, HostPort);
            }
            return HostRoomCodes.Count > 0 ? HostRoomCodes[0] : string.Empty;
        }

        public static void AddChat(string sender, string text, bool isSystem = false)
        {
            ChatLog.Add(new CoopChatEntry { Sender = sender, Text = text, IsSystem = isSystem });
            if (ChatLog.Count > 60)
            {
                ChatLog.RemoveAt(0);
            }
        }

        public static bool StartHost(int port, string password, string roomName, int maxPlayers, bool useUpnp)
        {
            ClientProfileStore.SaveLocalPrefs();
            Disconnect();

            HostPort = Mathf.Clamp(port, 1024, 65535);
            HostPassword = password ?? string.Empty;
            RoomName = string.IsNullOrEmpty(roomName) ? ("Ранчо " + LocalNick) : roomName;
            MaxPlayers = Mathf.Clamp(maxPlayers, 2, 8);
            UseUpnp = useUpnp;

            if (!Transport.Start(HostPort))
            {
                StatusMessage = "Не удалось открыть UDP порт " + HostPort + " (возможно занят)";
                return false;
            }

            Role = CoopRole.Host;
            LocalPlayerId = 1;
            _nextPlayerId = 2;

            string saveName = SRSingleton<GameContext>.Instance != null &&
                              SRSingleton<GameContext>.Instance.AutoSaveDirector != null &&
                              SRSingleton<GameContext>.Instance.AutoSaveDirector.SavedGame != null
                ? SRSingleton<GameContext>.Instance.AutoSaveDirector.SavedGame.GetName()
                : null;
            HostServerId = !string.IsNullOrEmpty(saveName) ? ("host_" + saveName) : ("host_" + Guid.NewGuid().ToString("N").Substring(0, 8));

            HostRoomCodes.Clear();
            HostEndpoints.Clear();
            List<NetworkEndpointEntry> eps = RoomCode.GetNetworkEndpoints(HostPort);
            for (int i = 0; i < eps.Count; i++)
            {
                HostEndpoints.Add(eps[i]);
                if (!string.IsNullOrEmpty(eps[i].RoomCode)) HostRoomCodes.Add(eps[i].RoomCode);
                string tag = eps[i].IsVirtualHypervisor ? "VIRT" : (eps[i].IsVpn ? "VPN" : "LAN");
                CoopLog.Info(string.Format("Host adapter [{0}] '{1}' -> {2}:{3} (code={4})",
                    tag, eps[i].InterfaceName, eps[i].Ip, eps[i].Port, eps[i].RoomCode));
            }

            if (UseUpnp)
            {
                UpnpMapper.OpenPortAsync(HostPort);
            }

            StatusMessage = "Сервер запущен на UDP :" + HostPort;
            CoopLog.Info("StartHost OK: room='" + RoomName + "', port=" + HostPort + ", hasPass=" + (!string.IsNullOrEmpty(HostPassword)) + ", serverId=" + HostServerId);
            AddChat("Система", "Сервер создан! Код комнаты: " + GetPrimaryRoomCode(), true);

            // If currently in main menu, auto-start/continue game on host
            if (SRSingleton<SceneContext>.Instance == null)
            {
                EnsureWorldLoadedForCoop(false);
            }
            else
            {
                CoopWorldSync.RebuildSceneObjectCaches();
            }

            return true;
        }

        public static bool ConnectTo(string endpointOrCode, string password)
        {
            ClientProfileStore.SaveLocalPrefs();
            Disconnect();

            CoopLog.Info("ConnectTo requested: input='" + endpointOrCode + "', nick='" + LocalNick + "', hasPass=" + (!string.IsNullOrEmpty(password)));
            IPEndPoint ep;
            if (!RoomCode.TryParseEndpoint(endpointOrCode, 7777, out ep))
            {
                StatusMessage = "Неверный код комнаты или IP:порт";
                CoopLog.Warn("ConnectTo failed: TryParseEndpoint returned false for '" + endpointOrCode + "'");
                return false;
            }

            if (!Transport.Start(0))
            {
                StatusMessage = "Ошибка создания UDP сокета";
                CoopLog.Error("ConnectTo failed: Transport.Start(0) returned false");
                return false;
            }

            Role = CoopRole.Client;
            _serverEp = ep;
            _pendingJoinPass = password ?? string.Empty;
            _clientProfileLoaded = false;
            _clientHandshakeDone = false;
            _clientSnapshotDone = false;
            _connectStartTime = Time.unscaledTime;
            _nextHelloRetryTime = _connectStartTime + 1.5f;
            StatusMessage = "Подключение к " + ep + "...";

            CoopLog.Info("Sending Hello to " + _serverEp + " from local UDP :" + Transport.LocalPort);
            SharedWriter.Reset();
            SharedWriter.WriteInt(1); // Protocol version
            Transport.SendReliable(_serverEp, PacketType.Hello, SharedWriter);
            return true;
        }

        public static void Disconnect()
        {
            if (Role != CoopRole.None)
            {
                CoopLog.Info("Disconnect called (previous role=" + Role + ")");
            }
            if (Role == CoopRole.Client && _clientProfileLoaded && !string.IsNullOrEmpty(HostServerId))
            {
                ClientProfileStore.SaveCurrentClientProfile(HostServerId);
            }

            if (Role == CoopRole.Host)
            {
                SharedWriter.Reset();
                SharedWriter.WriteString("Хост закрыл сервер");
                BroadcastReliable(PacketType.Disconnect, SharedWriter, null);
                UpnpMapper.ClosePortAsync();
            }
            else if (Role == CoopRole.Client && _serverEp != null && Transport.IsActive)
            {
                SharedWriter.Reset();
                SharedWriter.WriteString("Игрок вышел");
                Transport.SendReliable(_serverEp, PacketType.Disconnect, SharedWriter);
            }

            Role = CoopRole.None;
            WaitingWorldLoad = false;
            _clientProfileLoaded = false;
            _clientHandshakeDone = false;
            _clientSnapshotDone = false;
            _serverEp = null;
            ConnectedPeers.Clear();
            PeersByEp.Clear();
            PendingChallenges.Clear();
            HostRoomCodes.Clear();
            HostEndpoints.Clear();
            RemotePlayerManager.ClearAll();
            CoopWorldSync.ClearCaches();
            Transport.Stop();
            StatusMessage = "Не подключено";
        }

        public static void RefreshLanDiscovery()
        {
            float now = Time.unscaledTime;
            if (now - _lastDiscoveryTime < 0.8f && Transport.IsActive) return;
            _lastDiscoveryTime = now;

            CoopLog.Info("RefreshLanDiscovery called (isActive=" + IsActive + ", localPort=" + Transport.LocalPort + ")");
            if (!Transport.IsActive)
            {
                Transport.Start(0);
            }
            DiscoveredServers.Clear();
            Transport.BroadcastDiscovery(7777);
            Transport.BroadcastDiscovery(7778);
        }

        public static void OnGameSceneLoaded()
        {
            _cachedZoneTracker = null;
            if (!IsActive) return;

            CoopLog.Info("OnGameSceneLoaded: role=" + Role + ", waitingWorldLoad=" + WaitingWorldLoad);
            CoopWorldSync.RebuildSceneObjectCaches();
            if (IsHost)
            {
                string saveName = SRSingleton<GameContext>.Instance != null &&
                                  SRSingleton<GameContext>.Instance.AutoSaveDirector != null &&
                                  SRSingleton<GameContext>.Instance.AutoSaveDirector.SavedGame != null
                    ? SRSingleton<GameContext>.Instance.AutoSaveDirector.SavedGame.GetName()
                    : null;
                if (!string.IsNullOrEmpty(saveName))
                {
                    HostServerId = "host_" + saveName;
                }
            }
            else if (IsClient && WaitingWorldLoad)
            {
                WaitingWorldLoad = false;
                ApplyClientProfileAndRequestSnapshot();
            }
        }

        private static void ApplyClientProfileAndRequestSnapshot()
        {
            if (!_clientProfileLoaded && !string.IsNullOrEmpty(HostServerId))
            {
                NetSyncGuard.SuppressPersonalProgressDepth++;
                try
                {
                    ClientProfileStore.LoadOrInitClientProfile(HostServerId);
                }
                finally
                {
                    NetSyncGuard.SuppressPersonalProgressDepth--;
                }
                _clientProfileLoaded = true;
            }

            if (_serverEp != null)
            {
                CoopWorldSync.ClearLocalActorsBeforeSnapshot();
                CoopLog.Info("Client sending RequestSnapshot to " + _serverEp);
                SharedWriter.Reset();
                Transport.SendReliable(_serverEp, PacketType.RequestSnapshot, SharedWriter);
            }
        }

        private static void EnsureWorldLoadedForCoop(bool isClientSandbox)
        {
            GameContext gctx = SRSingleton<GameContext>.Instance;
            if (gctx == null || gctx.AutoSaveDirector == null)
            {
                CoopLog.Error("EnsureWorldLoadedForCoop: GameContext/AutoSaveDirector is null!");
                return;
            }

            CoopLog.Info("EnsureWorldLoadedForCoop: isClientSandbox=" + isClientSandbox);
            WaitingWorldLoad = true;
            AutoSaveDirector asd = gctx.AutoSaveDirector;

            if (!isClientSandbox && asd.HasContinue())
            {
                GameData.Summary summary = asd.GetSaveToContinue();
                if (summary != null)
                {
                    CoopLog.Info("Loading host continue save: " + summary.name + " / " + summary.saveName);
                    asd.BeginLoad(summary.name, summary.saveName, () =>
                    {
                        StatusMessage = "Ошибка загрузки мира";
                        CoopLog.Error("BeginLoad failed for " + summary.saveName);
                    });
                    return;
                }
            }

            string name = isClientSandbox ? "SlimeCoop_ClientSandbox" : "SlimeCoop_HostWorld";
            CoopLog.Info("Calling LoadNewGame: " + name);
            asd.LoadNewGame(name, Identifiable.Id.PINK_SLIME, PlayerState.GameMode.CLASSIC, () =>
            {
                StatusMessage = "Ошибка создания мира";
                CoopLog.Error("LoadNewGame failed for " + name);
            });
        }

        public static void SendChatMessage(string text)
        {
            if (!IsActive || string.IsNullOrEmpty(text)) return;
            string clean = text.Trim();
            if (clean.Length == 0) return;

            AddChat(LocalNick, clean, false);

            SharedWriter.Reset();
            SharedWriter.WriteString(LocalNick);
            SharedWriter.WriteString(clean);
            SharedWriter.WriteBool(false);

            if (IsHost)
            {
                BroadcastReliable(PacketType.ChatMessage, SharedWriter, null);
            }
            else if (_serverEp != null)
            {
                Transport.SendReliable(_serverEp, PacketType.ChatMessage, SharedWriter);
            }
        }

        public static void BroadcastReliable(PacketType type, PacketWriter payload, IPEndPoint except = null)
        {
            if (!IsHost) return;
            for (int i = 0; i < ConnectedPeers.Count; i++)
            {
                CoopPeerInfo p = ConnectedPeers[i];
                if (p.Authenticated && p.EndPoint != null && !p.EndPoint.Equals(except))
                {
                    Transport.SendReliable(p.EndPoint, type, payload);
                }
            }
        }

        public static void BroadcastUnreliable(PacketType type, PacketWriter payload, IPEndPoint except = null)
        {
            if (!IsHost) return;
            for (int i = 0; i < ConnectedPeers.Count; i++)
            {
                CoopPeerInfo p = ConnectedPeers[i];
                if (p.Authenticated && p.EndPoint != null && !p.EndPoint.Equals(except))
                {
                    Transport.SendUnreliable(p.EndPoint, type, payload);
                }
            }
        }

        public static void SendToHostReliable(PacketType type, PacketWriter payload)
        {
            if (IsClient && _serverEp != null && _clientHandshakeDone && !WaitingWorldLoad)
            {
                if (type == PacketType.RequestSnapshot || type == PacketType.ChatMessage || _clientSnapshotDone)
                {
                    Transport.SendReliable(_serverEp, type, payload);
                }
            }
        }

        public static void TickPerFrame()
        {
            if (WaitingWorldLoad &&
                SRSingleton<SceneContext>.Instance != null &&
                SRSingleton<GameContext>.Instance != null &&
                SRSingleton<GameContext>.Instance.AutoSaveDirector != null &&
                !SRSingleton<GameContext>.Instance.AutoSaveDirector.IsLoadingGame())
            {
                OnGameSceneLoaded();
            }

            // Pump incoming packets even in main menu (for LAN discovery / handshake)
            if (Transport.IsActive)
            {
                Transport.PumpReliableRetries();
                int processed = 0;
                while (processed < 128 && Transport.TryDequeue(out IncomingPacket pkt))
                {
                    HandleIncomingPacket(ref pkt);
                    processed++;
                }
            }

            if (!IsActive) return;

            RemotePlayerManager.TickInterpolate();

            float now = Time.unscaledTime;
            SceneContext ctx = SRSingleton<SceneContext>.Instance;

            // Send local player transform at 20 Hz
            if (ctx != null && ctx.Player != null && now >= _nextTransformSendTime)
            {
                _nextTransformSendTime = now + 0.05f;
                SendLocalTransform(ctx);
            }

            if (IsHost)
            {
                // Broadcast active world actors at 20 Hz
                if (ctx != null && ctx.Player != null && ConnectedPeers.Count > 0 && now >= _nextActorBatchTime)
                {
                    _nextActorBatchTime = now + 0.05f;
                    CoopWorldSync.WriteActiveActorBatch(SharedWriter, ctx.Player.transform.position, 24);
                    if (SharedWriter.Length > 2)
                    {
                        BroadcastUnreliable(PacketType.ActorBatchUpdate, SharedWriter, null);
                    }
                }

                // Sync world time & weather every 2s
                if (ctx != null && ctx.TimeDirector != null && ConnectedPeers.Count > 0 && now >= _nextTimeSyncTime)
                {
                    _nextTimeSyncTime = now + 2.0f;
                    SharedWriter.Reset();
                    SharedWriter.WriteDouble(ctx.TimeDirector.WorldTime());
                    byte w = 0;
                    if (ctx.GameModel != null && ctx.GameModel.GetWorldModel() != null)
                    {
                        w = (byte)ctx.GameModel.GetWorldModel().currWeather;
                    }
                    SharedWriter.WriteByte(w);
                    BroadcastUnreliable(PacketType.TimeWeatherSync, SharedWriter, null);
                }

                // Timeout dead peers (12s)
                long timeoutTicks = TimeSpan.FromSeconds(12).Ticks;
                long utcNow = DateTime.UtcNow.Ticks;
                for (int i = ConnectedPeers.Count - 1; i >= 0; i--)
                {
                    CoopPeerInfo peer = ConnectedPeers[i];
                    if (utcNow - peer.LastHeardTicks > timeoutTicks)
                    {
                        RemovePeer(peer, "таймаут соединения");
                    }
                }
            }
            else if (IsClient)
            {
                if (!_clientHandshakeDone && _serverEp != null)
                {
                    int elapsed = Mathf.Max(0, (int)(now - _connectStartTime));
                    if (elapsed >= 6)
                    {
                        StatusMessage = "Нет ответа от " + _serverEp + " — проверьте VPN IP/код и брандмауэр хоста (UDP)!";
                    }
                    else
                    {
                        StatusMessage = "Подключение к " + _serverEp + "... (" + elapsed + "с)";
                    }

                    if (now >= _nextHelloRetryTime)
                    {
                        _nextHelloRetryTime = now + 2.0f;
                        SharedWriter.Reset();
                        SharedWriter.WriteInt(1);
                        Transport.SendReliable(_serverEp, PacketType.Hello, SharedWriter);
                    }
                }

                if (_clientHandshakeDone && now >= _nextKeepAliveTime && _serverEp != null)
                {
                    _nextKeepAliveTime = now + 2.5f;
                    SharedWriter.Reset();
                    SharedWriter.WriteLong(DateTime.UtcNow.Ticks);
                    Transport.SendUnreliable(_serverEp, PacketType.Ping, SharedWriter);
                }

                if (_clientProfileLoaded && !string.IsNullOrEmpty(HostServerId) && now >= _nextProfileAutoSaveTime)
                {
                    _nextProfileAutoSaveTime = now + 15f;
                    ClientProfileStore.SaveCurrentClientProfile(HostServerId);
                }
            }
        }

        private static void SendLocalTransform(SceneContext ctx)
        {
            if (IsClient && (!_clientHandshakeDone || WaitingWorldLoad)) return;
            Transform pt = ctx.Player.transform;
            Camera cam = Camera.main;
            float yaw = pt.eulerAngles.y;
            float pitch = cam != null ? cam.transform.localEulerAngles.x : 0f;
            if (pitch > 180f) pitch -= 360f;

            vp_FPController ctrl = ctx.Player.GetComponentInChildren<vp_FPController>();
            bool jet = ctrl != null && ctrl.MotorFreeFly;
            if (!jet && ctx.PlayerState != null)
            {
                // Detect airborne + holding jump/jetpack roughly via vertical velocity
                jet = ctrl != null && !ctrl.Grounded && ctrl.Velocity.y > 1.5f;
            }

            WeaponVacuum vac = ctx.Player.GetComponentInChildren<WeaponVacuum>();
            bool vacActive = vac != null && vac.InVacMode();

            ZoneDirector.Zone zone = GetLocalZone(ctx);

            SharedWriter.Reset();
            SharedWriter.WriteInt(LocalPlayerId);
            SharedWriter.WriteString(LocalNick);
            SharedWriter.WriteInt(LocalColorIndex);
            SharedWriter.WriteVector3(pt.position);
            SharedWriter.WriteFloat(yaw);
            SharedWriter.WriteFloat(pitch);
            SharedWriter.WriteBool(jet);
            SharedWriter.WriteBool(vacActive);
            SharedWriter.WriteByte((byte)zone);

            if (IsHost)
            {
                if (ConnectedPeers.Count > 0)
                {
                    BroadcastUnreliable(PacketType.PlayerTransform, SharedWriter, null);
                }
            }
            else if (_serverEp != null)
            {
                Transport.SendUnreliable(_serverEp, PacketType.PlayerTransform, SharedWriter);
            }
        }

        private static void HandleIncomingPacket(ref IncomingPacket pkt)
        {
            PacketReader r = new PacketReader(pkt.Data, pkt.Offset, pkt.Length);

            if (pkt.Type == PacketType.LanDiscoveryReq)
            {
                if (IsHost)
                {
                    CoopLog.Info("Host received LanDiscoveryReq from " + pkt.Sender);
                    SharedWriter.Reset();
                    SharedWriter.WriteString(RoomName);
                    SharedWriter.WriteByte((byte)(ConnectedPeers.Count + 1));
                    SharedWriter.WriteByte((byte)MaxPlayers);
                    SharedWriter.WriteBool(!string.IsNullOrEmpty(HostPassword));
                    SharedWriter.WriteInt(HostPort);
                    Transport.SendUnreliable(pkt.Sender, PacketType.LanDiscoveryResp, SharedWriter);
                }
                return;
            }

            if (pkt.Type == PacketType.LanDiscoveryResp)
            {
                string rName = r.ReadString();
                int players = r.ReadByte();
                int maxP = r.ReadByte();
                bool hasPass = r.ReadBool();
                int hostPort = r.ReadInt();
                IPEndPoint srvEp = new IPEndPoint(pkt.Sender.Address, hostPort > 0 ? hostPort : pkt.Sender.Port);
                CoopLog.Info("Client received LanDiscoveryResp from " + pkt.Sender + " -> srvEp=" + srvEp + ", room='" + rName + "'");

                LanServerEntry existing = null;
                for (int i = 0; i < DiscoveredServers.Count; i++)
                {
                    if (DiscoveredServers[i].EndPoint.Equals(srvEp))
                    {
                        existing = DiscoveredServers[i];
                        break;
                    }
                }
                if (existing == null)
                {
                    existing = new LanServerEntry { EndPoint = srvEp };
                    DiscoveredServers.Add(existing);
                }
                existing.RoomName = rName;
                existing.Players = players;
                existing.MaxPlayers = maxP;
                existing.HasPassword = hasPass;
                existing.LastSeenUtc = DateTime.UtcNow;
                return;
            }

            if (IsHost)
            {
                HandleHostPacket(pkt.Sender, pkt.Type, ref r);
            }
            else if (IsClient)
            {
                HandleClientPacket(pkt.Sender, pkt.Type, ref r);
            }
        }

        private static void HandleHostPacket(IPEndPoint sender, PacketType type, ref PacketReader r)
        {
            PeersByEp.TryGetValue(sender, out CoopPeerInfo peer);
            if (peer != null)
            {
                peer.LastHeardTicks = DateTime.UtcNow.Ticks;
            }

            switch (type)
            {
                case PacketType.Hello:
                    int protoVer = r.ReadInt();
                    CoopLog.Info("Host received Hello from " + sender + " (proto=" + protoVer + ")");
                    if (protoVer != 1)
                    {
                        SharedWriter.Reset();
                        SharedWriter.WriteString("Несовместимая версия протокола SlimeCoop");
                        Transport.SendReliable(sender, PacketType.JoinReject, SharedWriter);
                        return;
                    }
                    string challenge = Guid.NewGuid().ToString("N").Substring(0, 16);
                    PendingChallenges[sender] = challenge;
                    SharedWriter.Reset();
                    SharedWriter.WriteBool(!string.IsNullOrEmpty(HostPassword));
                    SharedWriter.WriteString(challenge);
                    Transport.SendReliable(sender, PacketType.AuthChallenge, SharedWriter);
                    break;

                case PacketType.JoinRequest:
                    string nick = r.ReadString();
                    int colorIdx = r.ReadInt();
                    string passHash = r.ReadString();
                    CoopLog.Info("Host received JoinRequest from " + sender + " nick='" + nick + "'");

                    if (ConnectedPeers.Count + 1 >= MaxPlayers)
                    {
                        SharedWriter.Reset();
                        SharedWriter.WriteString("Сервер переполнен");
                        Transport.SendReliable(sender, PacketType.JoinReject, SharedWriter);
                        return;
                    }

                    if (!string.IsNullOrEmpty(HostPassword))
                    {
                        if (!PendingChallenges.TryGetValue(sender, out string chal))
                        {
                            SharedWriter.Reset();
                            SharedWriter.WriteString("Сессия авторизации устарела, повторите");
                            Transport.SendReliable(sender, PacketType.JoinReject, SharedWriter);
                            return;
                        }
                        string expected = NetTransport.ComputePasswordHash(chal, HostPassword);
                        if (!string.Equals(expected, passHash, StringComparison.OrdinalIgnoreCase))
                        {
                            SharedWriter.Reset();
                            SharedWriter.WriteString("Неверный пароль!");
                            Transport.SendReliable(sender, PacketType.JoinReject, SharedWriter);
                            return;
                        }
                    }

                    PendingChallenges.Remove(sender);
                    if (peer == null)
                    {
                        peer = new CoopPeerInfo
                        {
                            PlayerId = _nextPlayerId++,
                            EndPoint = sender,
                            Name = string.IsNullOrEmpty(nick) ? "Friend" : nick,
                            ColorIndex = colorIdx,
                            Authenticated = true,
                            LastHeardTicks = DateTime.UtcNow.Ticks
                        };
                        ConnectedPeers.Add(peer);
                        PeersByEp[sender] = peer;
                    }

                    SharedWriter.Reset();
                    SharedWriter.WriteInt(peer.PlayerId);
                    SharedWriter.WriteString(HostServerId);
                    SharedWriter.WriteBool(ShareMarketIncome);
                    SharedWriter.WriteString(RoomName);
                    Transport.SendReliable(sender, PacketType.JoinAccept, SharedWriter);

                    AddChat("Система", peer.Name + " присоединился к ранчо!", true);
                    BroadcastPlayerList();
                    break;

                case PacketType.RequestSnapshot:
                    if (peer == null) return;
                    CoopLog.Info("Host sending WorldSnapshot to " + peer.Name + " (" + sender + ")");
                    int chunkCount = 0;
                    CoopWorldSync.SendFullSnapshotToClient(chunk =>
                    {
                        chunkCount++;
                        Transport.SendReliable(sender, PacketType.WorldSnapshotChunk, chunk);
                    });

                    SceneContext hctx = SRSingleton<SceneContext>.Instance;
                    bool hasHostPos = hctx != null && hctx.Player != null;
                    Vector3 hostSpawnPos = hasHostPos ? hctx.Player.transform.position : Vector3.zero;
                    RegionMember hrm = hasHostPos ? hctx.Player.GetComponent<RegionMember>() : null;
                    RegionRegistry.RegionSetId hSet = hrm != null ? hrm.setId : RegionRegistry.RegionSetId.HOME;

                    SharedWriter.Reset();
                    SharedWriter.WriteBool(hasHostPos);
                    SharedWriter.WriteVector3(hostSpawnPos);
                    SharedWriter.WriteByte((byte)hSet);
                    Transport.SendReliable(sender, PacketType.WorldSnapshotEnd, SharedWriter);
                    CoopLog.Info("Host sent WorldSnapshot (" + chunkCount + " chunks) to " + peer.Name);
                    break;

                case PacketType.Ping:
                    long ts = r.ReadLong();
                    SharedWriter.Reset();
                    SharedWriter.WriteLong(ts);
                    Transport.SendUnreliable(sender, PacketType.Pong, SharedWriter);
                    break;

                case PacketType.Disconnect:
                    if (peer != null)
                    {
                        RemovePeer(peer, "вышел из игры");
                    }
                    break;

                case PacketType.ChatMessage:
                    if (peer == null) return;
                    string cSender = r.ReadString();
                    string cText = r.ReadString();
                    bool cSys = r.ReadBool();
                    AddChat(peer.Name, cText, cSys);
                    SharedWriter.Reset();
                    SharedWriter.WriteString(peer.Name);
                    SharedWriter.WriteString(cText);
                    SharedWriter.WriteBool(cSys);
                    BroadcastReliable(PacketType.ChatMessage, SharedWriter, sender);
                    break;

                case PacketType.PlayerTransform:
                    if (peer == null) return;
                    r.ReadInt(); // ignore client-supplied id, trust peer.PlayerId
                    string pName = r.ReadString();
                    int pColor = r.ReadInt();
                    Vector3 pos = r.ReadVector3();
                    float yaw = r.ReadFloat();
                    float pitch = r.ReadFloat();
                    bool jet = r.ReadBool();
                    bool vac = r.ReadBool();
                    ZoneDirector.Zone z = (ZoneDirector.Zone)r.ReadByte();
                    peer.CurrentZone = z;

                    RemotePlayerManager.UpdateTransform(peer.PlayerId, peer.Name, pColor, pos, yaw, pitch, jet, vac);

                    SharedWriter.Reset();
                    SharedWriter.WriteInt(peer.PlayerId);
                    SharedWriter.WriteString(peer.Name);
                    SharedWriter.WriteInt(pColor);
                    SharedWriter.WriteVector3(pos);
                    SharedWriter.WriteFloat(yaw);
                    SharedWriter.WriteFloat(pitch);
                    SharedWriter.WriteBool(jet);
                    SharedWriter.WriteBool(vac);
                    SharedWriter.WriteByte((byte)z);
                    BroadcastUnreliable(PacketType.PlayerTransform, SharedWriter, sender);
                    break;

                case PacketType.PlotBuild:
                    if (peer == null) return;
                    string bPlotId = r.ReadString();
                    LandPlot.Id bType = (LandPlot.Id)r.ReadInt();
                    CoopWorldSync.ApplyPlotBuild(bPlotId, bType);
                    SharedWriter.Reset();
                    SharedWriter.WriteString(bPlotId);
                    SharedWriter.WriteInt((int)bType);
                    BroadcastReliable(PacketType.PlotBuild, SharedWriter, sender);
                    break;

                case PacketType.PlotUpgrade:
                    if (peer == null) return;
                    string uPlotId = r.ReadString();
                    LandPlot.Upgrade uType = (LandPlot.Upgrade)r.ReadInt();
                    CoopWorldSync.ApplyPlotUpgrade(uPlotId, uType);
                    SharedWriter.Reset();
                    SharedWriter.WriteString(uPlotId);
                    SharedWriter.WriteInt((int)uType);
                    BroadcastReliable(PacketType.PlotUpgrade, SharedWriter, sender);
                    break;

                case PacketType.PlotPlant:
                    if (peer == null) return;
                    string pPlotId = r.ReadString();
                    Identifiable.Id pCrop = (Identifiable.Id)r.ReadInt();
                    CoopWorldSync.ApplyPlotPlant(pPlotId, pCrop);
                    SharedWriter.Reset();
                    SharedWriter.WriteString(pPlotId);
                    SharedWriter.WriteInt((int)pCrop);
                    BroadcastReliable(PacketType.PlotPlant, SharedWriter, sender);
                    break;

                case PacketType.DoorState:
                    if (peer == null) return;
                    string dId = r.ReadString();
                    AccessDoor.State dState = (AccessDoor.State)r.ReadByte();
                    CoopWorldSync.ApplyDoorState(dId, dState);
                    SharedWriter.Reset();
                    SharedWriter.WriteString(dId);
                    SharedWriter.WriteByte((byte)dState);
                    BroadcastReliable(PacketType.DoorState, SharedWriter, sender);
                    break;

                case PacketType.GordoEatSync:
                    if (peer == null) return;
                    string gId = r.ReadString();
                    int gCnt = r.ReadInt();
                    CoopWorldSync.ApplyGordoEat(gId, gCnt);
                    SharedWriter.Reset();
                    SharedWriter.WriteString(gId);
                    SharedWriter.WriteInt(gCnt);
                    BroadcastReliable(PacketType.GordoEatSync, SharedWriter, sender);
                    break;

                case PacketType.OasisSync:
                    if (peer == null) return;
                    string oId = r.ReadString();
                    CoopWorldSync.ApplyOasisLive(oId);
                    SharedWriter.Reset();
                    SharedWriter.WriteString(oId);
                    BroadcastReliable(PacketType.OasisSync, SharedWriter, sender);
                    break;

                case PacketType.PodSync:
                    if (peer == null) return;
                    string podId = r.ReadString();
                    TreasurePod.State podState = (TreasurePod.State)r.ReadByte();
                    CoopWorldSync.ApplyPodState(podId, podState);
                    SharedWriter.Reset();
                    SharedWriter.WriteString(podId);
                    SharedWriter.WriteByte((byte)podState);
                    BroadcastReliable(PacketType.PodSync, SharedWriter, sender);
                    break;

                case PacketType.GadgetPlace:
                    if (peer == null) return;
                    string gsId = r.ReadString();
                    Gadget.Id gadId = (Gadget.Id)r.ReadInt();
                    float gadYaw = r.ReadFloat();
                    CoopWorldSync.ApplyGadgetPlace(gsId, gadId, gadYaw);
                    SharedWriter.Reset();
                    SharedWriter.WriteString(gsId);
                    SharedWriter.WriteInt((int)gadId);
                    SharedWriter.WriteFloat(gadYaw);
                    BroadcastReliable(PacketType.GadgetPlace, SharedWriter, sender);
                    break;

                case PacketType.GadgetDestroy:
                    if (peer == null) return;
                    string gdId = r.ReadString();
                    CoopWorldSync.ApplyGadgetDestroy(gdId);
                    SharedWriter.Reset();
                    SharedWriter.WriteString(gdId);
                    BroadcastReliable(PacketType.GadgetDestroy, SharedWriter, sender);
                    break;

                case PacketType.ClientVacCapture:
                    if (peer == null) return;
                    long capActorId = r.ReadLong();
                    CoopWorldSync.ApplyActorDestroy(capActorId);
                    SharedWriter.Reset();
                    SharedWriter.WriteLong(capActorId);
                    BroadcastReliable(PacketType.ActorDestroy, SharedWriter, null);
                    break;

                case PacketType.ClientExpelRequest:
                    if (peer == null) return;
                    Identifiable.Id expId = (Identifiable.Id)r.ReadInt();
                    Vector3 expPos = r.ReadVector3();
                    Vector3 expVel = r.ReadVector3();
                    RegionRegistry.RegionSetId expSet = (RegionRegistry.RegionSetId)r.ReadByte();
                    SpawnExpelledFromClient(expId, expPos, expVel, expSet);
                    break;
            }
        }

        private static void SpawnExpelledFromClient(Identifiable.Id id, Vector3 pos, Vector3 vel, RegionRegistry.RegionSetId setId)
        {
            GameContext gctx = SRSingleton<GameContext>.Instance;
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (gctx == null || ctx == null || gctx.LookupDirector == null) return;

            GameObject prefab = null;
            try { prefab = gctx.LookupDirector.GetPrefab(id); } catch { }
            if (prefab == null) return;

            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                GameObject go = SRBehaviour.InstantiateActor(prefab, setId, pos, Quaternion.identity, false);
                if (go != null)
                {
                    Rigidbody rb = go.GetComponent<Rigidbody>();
                    if (rb != null) rb.velocity = vel;
                }
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
            }
        }

        private static void HandleClientPacket(IPEndPoint sender, PacketType type, ref PacketReader r)
        {
            switch (type)
            {
                case PacketType.AuthChallenge:
                    bool requiresPass = r.ReadBool();
                    string challenge = r.ReadString();
                    CoopLog.Info("Client received AuthChallenge from " + sender + " (requiresPass=" + requiresPass + ")");
                    if (requiresPass && string.IsNullOrEmpty(_pendingJoinPass))
                    {
                        StatusMessage = "Сервер требует пароль! Введите пароль и подключитесь снова.";
                        Disconnect();
                        return;
                    }
                    string hash = requiresPass ? NetTransport.ComputePasswordHash(challenge, _pendingJoinPass) : string.Empty;
                    SharedWriter.Reset();
                    SharedWriter.WriteString(LocalNick);
                    SharedWriter.WriteInt(LocalColorIndex);
                    SharedWriter.WriteString(hash);
                    Transport.SendReliable(_serverEp, PacketType.JoinRequest, SharedWriter);
                    break;

                case PacketType.JoinReject:
                    string reason = r.ReadString();
                    CoopLog.Warn("Client received JoinReject: " + reason);
                    StatusMessage = "Отказ: " + reason;
                    AddChat("Система", StatusMessage, true);
                    Disconnect();
                    StatusMessage = "Отказ: " + reason;
                    break;

                case PacketType.JoinAccept:
                    _clientHandshakeDone = true;
                    LocalPlayerId = r.ReadInt();
                    HostServerId = r.ReadString();
                    ShareMarketIncome = r.ReadBool();
                    RoomName = r.ReadString();
                    CoopLog.Info("Client received JoinAccept! playerId=" + LocalPlayerId + ", serverId=" + HostServerId + ", room='" + RoomName + "'");
                    StatusMessage = "Подключено к '" + RoomName + "'";
                    AddChat("Система", "Вы подключились к '" + RoomName + "'! Ваш прогресс апгрейдов, Слаймопедии и достижений личный.", true);

                    if (SRSingleton<SceneContext>.Instance == null)
                    {
                        EnsureWorldLoadedForCoop(true);
                    }
                    else
                    {
                        CoopWorldSync.RebuildSceneObjectCaches();
                        ApplyClientProfileAndRequestSnapshot();
                    }
                    break;

                case PacketType.Disconnect:
                    string discMsg = r.ReadString();
                    AddChat("Система", "Отключено: " + discMsg, true);
                    Disconnect();
                    StatusMessage = "Отключено: " + discMsg;
                    break;

                case PacketType.Pong:
                    long sentTicks = r.ReadLong();
                    ClientPingMs = (int)Math.Max(0, (DateTime.UtcNow.Ticks - sentTicks) / TimeSpan.TicksPerMillisecond);
                    break;

                case PacketType.PlayerList:
                    ConnectedPeers.Clear();
                    int cnt = r.ReadByte();
                    for (int i = 0; i < cnt; i++)
                    {
                        ConnectedPeers.Add(new CoopPeerInfo
                        {
                            PlayerId = r.ReadInt(),
                            Name = r.ReadString(),
                            ColorIndex = r.ReadInt(),
                            CurrentZone = (ZoneDirector.Zone)r.ReadByte(),
                            Authenticated = true
                        });
                    }
                    break;

                case PacketType.ChatMessage:
                    string senderName = r.ReadString();
                    string txt = r.ReadString();
                    bool sys = r.ReadBool();
                    AddChat(senderName, txt, sys);
                    break;

                case PacketType.PlayerTransform:
                    int pid = r.ReadInt();
                    if (pid == LocalPlayerId) return;
                    string pName = r.ReadString();
                    int pColor = r.ReadInt();
                    Vector3 pos = r.ReadVector3();
                    float yaw = r.ReadFloat();
                    float pitch = r.ReadFloat();
                    bool jet = r.ReadBool();
                    bool vac = r.ReadBool();
                    ZoneDirector.Zone z = (ZoneDirector.Zone)r.ReadByte();
                    RemotePlayerManager.UpdateTransform(pid, pName, pColor, pos, yaw, pitch, jet, vac);
                    for (int i = 0; i < ConnectedPeers.Count; i++)
                    {
                        if (ConnectedPeers[i].PlayerId == pid)
                        {
                            ConnectedPeers[i].CurrentZone = z;
                            break;
                        }
                    }
                    break;

                case PacketType.WorldSnapshotChunk:
                    if (!WaitingWorldLoad)
                    {
                        CoopWorldSync.ApplySnapshotChunk(ref r);
                    }
                    break;

                case PacketType.WorldSnapshotEnd:
                    _clientSnapshotDone = true;
                    bool hasHostSpawn = r.Remaining > 0 && r.ReadBool();
                    Vector3 hostSpawn = hasHostSpawn ? r.ReadVector3() : Vector3.zero;
                    RegionRegistry.RegionSetId hostSet = hasHostSpawn ? (RegionRegistry.RegionSetId)r.ReadByte() : RegionRegistry.RegionSetId.HOME;
                    CoopLog.Info("Client received WorldSnapshotEnd (ranch synced, hasHostSpawn=" + hasHostSpawn + ", pos=" + hostSpawn + ").");
                    if (hasHostSpawn)
                    {
                        SceneContext cctx = SRSingleton<SceneContext>.Instance;
                        if (cctx != null && cctx.Player != null)
                        {
                            Vector3 targetPos = hostSpawn + new Vector3(1.8f, 0.8f, 0f);
                            TeleportablePlayer tp = cctx.Player.GetComponent<TeleportablePlayer>();
                            if (tp != null)
                            {
                                tp.TeleportTo(targetPos, hostSet, null, false, true);
                            }
                            else
                            {
                                cctx.Player.transform.position = targetPos;
                            }
                        }
                    }
                    AddChat("Система", "Вы в мире хоста! Общее ранчо синхронизировано.", true);
                    break;

                case PacketType.TimeWeatherSync:
                    CoopWorldSync.ApplyTimeWeather(r.ReadDouble(), (AmbianceDirector.Weather)r.ReadByte());
                    break;

                case PacketType.PlotBuild:
                    CoopWorldSync.ApplyPlotBuild(r.ReadString(), (LandPlot.Id)r.ReadInt());
                    break;

                case PacketType.PlotUpgrade:
                    CoopWorldSync.ApplyPlotUpgrade(r.ReadString(), (LandPlot.Upgrade)r.ReadInt());
                    break;

                case PacketType.PlotPlant:
                    CoopWorldSync.ApplyPlotPlant(r.ReadString(), (Identifiable.Id)r.ReadInt());
                    break;

                case PacketType.SiloSlotSync:
                    CoopWorldSync.ApplySiloSlot(r.ReadString(), r.ReadByte(), (Identifiable.Id)r.ReadInt(), r.ReadInt());
                    break;

                case PacketType.DoorState:
                    CoopWorldSync.ApplyDoorState(r.ReadString(), (AccessDoor.State)r.ReadByte());
                    break;

                case PacketType.GordoEatSync:
                    CoopWorldSync.ApplyGordoEat(r.ReadString(), r.ReadInt());
                    break;

                case PacketType.OasisSync:
                    CoopWorldSync.ApplyOasisLive(r.ReadString());
                    break;

                case PacketType.PodSync:
                    CoopWorldSync.ApplyPodState(r.ReadString(), (TreasurePod.State)r.ReadByte());
                    break;

                case PacketType.GadgetPlace:
                    CoopWorldSync.ApplyGadgetPlace(r.ReadString(), (Gadget.Id)r.ReadInt(), r.ReadFloat());
                    break;

                case PacketType.GadgetDestroy:
                    CoopWorldSync.ApplyGadgetDestroy(r.ReadString());
                    break;

                case PacketType.ActorSpawn:
                    CoopWorldSync.ApplyActorSpawn(
                        r.ReadLong(),
                        (Identifiable.Id)r.ReadInt(),
                        r.ReadVector3(),
                        r.ReadQuaternion(),
                        r.ReadVector3(),
                        (RegionRegistry.RegionSetId)r.ReadByte());
                    break;

                case PacketType.ActorDestroy:
                    CoopWorldSync.ApplyActorDestroy(r.ReadLong());
                    break;

                case PacketType.ActorBatchUpdate:
                    CoopWorldSync.ApplyActorBatch(ref r);
                    break;

                case PacketType.MarketDepositNotice:
                    int amount = r.ReadInt();
                    string seller = r.ReadString();
                    if (ShareMarketIncome && amount > 0 && SRSingleton<SceneContext>.Instance != null && SRSingleton<SceneContext>.Instance.PlayerState != null)
                    {
                        NetSyncGuard.SuppressPersonalProgressDepth++;
                        try
                        {
                            SRSingleton<SceneContext>.Instance.PlayerState.AddCurrency(amount, PlayerState.CoinsType.NORM);
                        }
                        finally
                        {
                            NetSyncGuard.SuppressPersonalProgressDepth--;
                        }
                    }
                    break;
            }
        }

        private static void RemovePeer(CoopPeerInfo peer, string reason)
        {
            ConnectedPeers.Remove(peer);
            if (peer.EndPoint != null)
            {
                PeersByEp.Remove(peer.EndPoint);
                Transport.RemovePeer(peer.EndPoint);
            }
            RemotePlayerManager.RemovePlayer(peer.PlayerId);
            AddChat("Система", peer.Name + " отключился (" + reason + ").", true);
            BroadcastPlayerList();
        }

        private static void BroadcastPlayerList()
        {
            if (!IsHost) return;
            SharedWriter.Reset();
            int total = 1 + ConnectedPeers.Count;
            SharedWriter.WriteByte((byte)total);

            // Host first
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            ZoneDirector.Zone hostZone = GetLocalZone(ctx);
            SharedWriter.WriteInt(LocalPlayerId);
            SharedWriter.WriteString(LocalNick);
            SharedWriter.WriteInt(LocalColorIndex);
            SharedWriter.WriteByte((byte)hostZone);

            for (int i = 0; i < ConnectedPeers.Count; i++)
            {
                CoopPeerInfo p = ConnectedPeers[i];
                SharedWriter.WriteInt(p.PlayerId);
                SharedWriter.WriteString(p.Name);
                SharedWriter.WriteInt(p.ColorIndex);
                SharedWriter.WriteByte((byte)p.CurrentZone);
            }

            BroadcastReliable(PacketType.PlayerList, SharedWriter, null);
        }
    }
}
