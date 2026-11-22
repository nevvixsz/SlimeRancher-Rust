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
        private readonly Dictionary<SiloSlotKey, uint> _siloTakesProcessed = new Dictionary<SiloSlotKey, uint>(16);

        public void IncrementSiloTakesProcessed(SiloSlotKey slotKey)
        {
            _siloTakesProcessed.TryGetValue(slotKey, out uint cur);
            _siloTakesProcessed[slotKey] = cur + 1;
        }

        public uint GetSiloTakesProcessed(SiloSlotKey slotKey)
        {
            return _siloTakesProcessed.TryGetValue(slotKey, out uint cur) ? cur : 0;
        }

        public void ClearSiloTakesForPlot(string plotId)
        {
            if (_siloTakesProcessed.Count == 0 || string.IsNullOrEmpty(plotId)) return;
            List<SiloSlotKey> keys = null;
            foreach (KeyValuePair<SiloSlotKey, uint> kv in _siloTakesProcessed)
            {
                if (string.Equals(kv.Key.PlotId, plotId, StringComparison.Ordinal))
                {
                    if (keys == null) keys = new List<SiloSlotKey>(8);
                    keys.Add(kv.Key);
                }
            }
            if (keys != null)
            {
                for (int i = 0; i < keys.Count; i++)
                {
                    _siloTakesProcessed.Remove(keys[i]);
                }
            }
        }
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
        public static bool IsClientProfileLoaded => _clientProfileLoaded;
        public static bool BlockLocalSaveUntilMenu;
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
        private static float _nextSiloSyncTime;
        private static float _nextTimeoutCheckTime;
        private static float _nextKeepAliveTime;
        private static float _nextProfileAutoSaveTime;
        private static bool _clientProfileLoaded;
        private static bool _clientHandshakeDone;
        private static bool _clientSnapshotDone;
        private static bool _tcpFallbackStarted;
        private static float _connectStartTime;
        private static float _nextHelloRetryTime;
        private static float _lastDiscoveryTime;
        private static bool _sceneIsMainMenu = true;
        private static bool _sceneCheckedOnce;
        private static PlayerZoneTracker _cachedZoneTracker;
        private static GameObject _cachedPlayerObj;
        private static Transform _cachedPlayerTransform;
        private static vp_FPController _cachedFpCtrl;
        private static WeaponVacuum _cachedWeaponVac;
        private static Camera _cachedMainCam;
        private static Transform _cachedMainCamTransform;

        public static void NotifySceneChanged(string sceneName)
        {
            _sceneCheckedOnce = true;
            _sceneIsMainMenu = string.Equals(sceneName, "MainMenu", StringComparison.OrdinalIgnoreCase);
            _cachedZoneTracker = null;
            _cachedPlayerObj = null;
            _cachedPlayerTransform = null;
            _cachedFpCtrl = null;
            _cachedWeaponVac = null;
            _cachedMainCam = null;
            _cachedMainCamTransform = null;
        }

        public static bool IsInGameSession()
        {
            if (!_sceneCheckedOnce)
            {
                _sceneCheckedOnce = true;
                string sceneName = UnityEngine.SceneManagement.SceneManager.GetActiveScene().name;
                _sceneIsMainMenu = string.Equals(sceneName, "MainMenu", StringComparison.OrdinalIgnoreCase);
            }
            if (_sceneIsMainMenu) return false;
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            return ctx != null && ctx.Player != null;
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
            HostServerId = !string.IsNullOrEmpty(saveName)
                ? ClientProfileStore.GetOrCreateHostServerId(saveName)
                : ("srv_" + Guid.NewGuid().ToString("N").Substring(0, 12));

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

            StatusMessage = "Сервер запущен (UDP+TCP :" + HostPort + ")";
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
            HostServerId = string.Empty;
            _serverEp = ep;
            _pendingJoinPass = password ?? string.Empty;
            _clientProfileLoaded = false;
            _clientHandshakeDone = false;
            _clientSnapshotDone = false;
            _tcpFallbackStarted = false;
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
            HostServerId = string.Empty;
            WaitingWorldLoad = false;
            _clientProfileLoaded = false;
            _clientHandshakeDone = false;
            _clientSnapshotDone = false;
            _tcpFallbackStarted = false;
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
            _cachedPlayerObj = null;
            _cachedFpCtrl = null;
            _cachedWeaponVac = null;
            _cachedMainCam = null;
            if (!IsActive) return;

            CoopLog.Info("OnGameSceneLoaded: role=" + Role + ", waitingWorldLoad=" + WaitingWorldLoad);
            CoopWorldSync.EnsureSceneObjectCaches();
            if (IsHost)
            {
                string saveName = SRSingleton<GameContext>.Instance != null &&
                                  SRSingleton<GameContext>.Instance.AutoSaveDirector != null &&
                                  SRSingleton<GameContext>.Instance.AutoSaveDirector.SavedGame != null
                    ? SRSingleton<GameContext>.Instance.AutoSaveDirector.SavedGame.GetName()
                    : null;
                if (!string.IsNullOrEmpty(saveName))
                {
                    HostServerId = ClientProfileStore.GetOrCreateHostServerId(saveName);
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
                BlockLocalSaveUntilMenu = true;
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

                SharedWriter.Reset();
                SharedWriter.WriteLong(DateTime.UtcNow.Ticks);
                Transport.SendUnreliable(_serverEp, PacketType.Ping, SharedWriter);
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

        public static void ClearPeerSiloTakesForPlot(string plotId)
        {
            if (string.IsNullOrEmpty(plotId)) return;
            for (int i = 0; i < ConnectedPeers.Count; i++)
            {
                if (ConnectedPeers[i] != null)
                {
                    ConnectedPeers[i].ClearSiloTakesForPlot(plotId);
                }
            }
        }

        public static void BroadcastHostSiloSlot(string plotId, SiloStorage.StorageType storageType, int slotIdx, Identifiable.Id itemId, int count, uint revision)
        {
            if (!IsHost || ConnectedPeers.Count == 0) return;
            SiloSlotKey slotKey = new SiloSlotKey
            {
                PlotId = plotId ?? string.Empty,
                StorageType = storageType,
                SlotIdx = slotIdx
            };
            for (int i = 0; i < ConnectedPeers.Count; i++)
            {
                CoopPeerInfo p = ConnectedPeers[i];
                if (p == null || !p.Authenticated || p.EndPoint == null) continue;
                SharedWriter.Reset();
                SharedWriter.WriteString(plotId);
                SharedWriter.WriteByte((byte)storageType);
                SharedWriter.WriteByte((byte)slotIdx);
                SharedWriter.WriteInt((int)itemId);
                SharedWriter.WriteInt(count);
                SharedWriter.WriteUInt(revision);
                SharedWriter.WriteUInt(p.GetSiloTakesProcessed(slotKey));
                Transport.SendReliable(p.EndPoint, PacketType.SiloSlotSync, SharedWriter);
            }
        }

        public static void SendHostSiloSlotToPeer(CoopPeerInfo peer, string plotId, SiloStorage.StorageType storageType, int slotIdx, Identifiable.Id itemId, int count, uint revision)
        {
            if (!IsHost || peer == null || !peer.Authenticated || peer.EndPoint == null) return;
            SiloSlotKey slotKey = new SiloSlotKey
            {
                PlotId = plotId ?? string.Empty,
                StorageType = storageType,
                SlotIdx = slotIdx
            };
            SharedWriter.Reset();
            SharedWriter.WriteString(plotId);
            SharedWriter.WriteByte((byte)storageType);
            SharedWriter.WriteByte((byte)slotIdx);
            SharedWriter.WriteInt((int)itemId);
            SharedWriter.WriteInt(count);
            SharedWriter.WriteUInt(revision);
            SharedWriter.WriteUInt(peer.GetSiloTakesProcessed(slotKey));
            Transport.SendReliable(peer.EndPoint, PacketType.SiloSlotSync, SharedWriter);
        }

        public static void SendSiloTakeGrant(CoopPeerInfo peer, Identifiable.Id itemId)
        {
            if (!IsHost || peer == null || !peer.Authenticated || peer.EndPoint == null) return;
            SharedWriter.Reset();
            SharedWriter.WriteInt((int)itemId);
            Transport.SendReliable(peer.EndPoint, PacketType.SiloTakeGrant, SharedWriter);
        }

        public static void SendSiloSlotTake(string plotId, SiloStorage.StorageType storageType, int slotIdx, Identifiable.Id expectedItem)
        {
            if (!IsClient) return;
            SharedWriter.Reset();
            SharedWriter.WriteString(plotId);
            SharedWriter.WriteByte((byte)storageType);
            SharedWriter.WriteByte((byte)slotIdx);
            SharedWriter.WriteInt((int)expectedItem);
            SendToHostReliable(PacketType.SiloSlotTake, SharedWriter);
        }

        public static void TickPerFrame()
        {
            if (BlockLocalSaveUntilMenu && !IsActive && !IsInGameSession())
            {
                BlockLocalSaveUntilMenu = false;
            }

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
                while (processed < 64 && Transport.TryDequeue(out IncomingPacket pkt))
                {
                    try
                    {
                        HandleIncomingPacket(ref pkt);
                    }
                    catch (Exception ex)
                    {
                        CoopLog.Warn("HandleIncomingPacket (" + pkt.Type + ") error: " + ex.Message);
                    }
                    finally
                    {
                        Transport.ReturnIncomingPacket(ref pkt);
                    }
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
                // Stream queued snapshots smoothly without freezing the host frame
                CoopWorldSync.PumpSnapshotSends(Transport, 6);

                // Broadcast active world actors at 20 Hz
                if (ctx != null && ctx.Player != null && ConnectedPeers.Count > 0 && now >= _nextActorBatchTime)
                {
                    _nextActorBatchTime = now + 0.05f;
                    Transform hostTr = _cachedPlayerTransform != null ? _cachedPlayerTransform : ctx.Player.transform;
                    CoopWorldSync.WriteActiveActorBatch(SharedWriter, hostTr.position, 24);
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

                // Reconcile any changed host silo/collector/feeder slots every 0.2s
                if (ctx != null && ConnectedPeers.Count > 0 && now >= _nextSiloSyncTime)
                {
                    _nextSiloSyncTime = now + 0.2f;
                    CoopWorldSync.SyncDirtyHostSilos();
                }

                // Timeout dead peers (30s, checked once per second; generous for scene loading)
                if (ConnectedPeers.Count > 0 && now >= _nextTimeoutCheckTime)
                {
                    _nextTimeoutCheckTime = now + 1.0f;
                    long timeoutTicks = TimeSpan.FromSeconds(30).Ticks;
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
            }
            else if (IsClient)
            {
                if (!WaitingWorldLoad &&
                    CoopWorldSync.PumpClientSnapshotApply(out bool snapDone, out bool hasHostSpawn, out Vector3 hostSpawn, out RegionRegistry.RegionSetId hostSet) &&
                    snapDone)
                {
                    _clientSnapshotDone = true;
                    CoopLog.Info("Client finished applying WorldSnapshot (hasHostSpawn=" + hasHostSpawn + ", pos=" + hostSpawn + ").");
                    if (hasHostSpawn && ctx != null && ctx.Player != null)
                    {
                        Vector3 targetPos = hostSpawn + new Vector3(1.8f, 0.8f, 0f);
                        TeleportablePlayer tp = ctx.Player.GetComponent<TeleportablePlayer>();
                        if (tp != null)
                        {
                            tp.TeleportTo(targetPos, hostSet, null, false, true);
                        }
                        else
                        {
                            ctx.Player.transform.position = targetPos;
                        }
                    }
                    AddChat("Система", "Вы в мире хоста! Общее ранчо синхронизировано.", true);
                }

                if (!_clientHandshakeDone && _serverEp != null)
                {
                    if (!_tcpFallbackStarted && now - _connectStartTime >= 0.8f)
                    {
                        _tcpFallbackStarted = true;
                        Transport.BeginTcpFallback(_serverEp);
                    }

                    if (Transport.ConsumeTcpJustConnected(_serverEp))
                    {
                        CoopLog.Info("Client sending Hello over TCP to " + _serverEp);
                        SharedWriter.Reset();
                        SharedWriter.WriteInt(1);
                        Transport.SendReliable(_serverEp, PacketType.Hello, SharedWriter);
                        _nextHelloRetryTime = now + 2.0f;
                    }

                    int elapsed = Mathf.Max(0, (int)(now - _connectStartTime));
                    if (elapsed >= 6)
                    {
                        string tcpErr = Transport.LastTcpError;
                        StatusMessage = string.IsNullOrEmpty(tcpErr)
                            ? ("Нет ответа от " + _serverEp + " — проверьте IP/VPN и брандмауэр хоста!")
                            : ("Нет ответа от " + _serverEp + " (TCP: " + tcpErr + ")");
                    }
                    else
                    {
                        string via = Transport.IsTcpPeer(_serverEp) ? " [TCP]" : "";
                        StatusMessage = "Подключение к " + _serverEp + via + "... (" + elapsed + "с)";
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
                    _nextKeepAliveTime = now + 1.5f;
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
            if (IsHost && ConnectedPeers.Count == 0) return;
            if (IsClient && (!_clientHandshakeDone || WaitingWorldLoad)) return;
            GameObject playerGo = ctx.Player;
            if (_cachedPlayerObj != playerGo)
            {
                _cachedPlayerObj = playerGo;
                _cachedPlayerTransform = playerGo.transform;
                _cachedFpCtrl = playerGo.GetComponentInChildren<vp_FPController>();
                _cachedWeaponVac = playerGo.GetComponentInChildren<WeaponVacuum>();
                _cachedMainCam = Camera.main;
                _cachedMainCamTransform = _cachedMainCam != null ? _cachedMainCam.transform : null;
            }
            if (_cachedMainCam == null || !_cachedMainCam.isActiveAndEnabled)
            {
                _cachedMainCam = Camera.main;
                _cachedMainCamTransform = _cachedMainCam != null ? _cachedMainCam.transform : null;
            }

            Transform pt = _cachedPlayerTransform;
            float yaw = pt.eulerAngles.y;
            float pitch = _cachedMainCamTransform != null ? _cachedMainCamTransform.localEulerAngles.x : 0f;
            if (pitch > 180f) pitch -= 360f;

            bool jet = _cachedFpCtrl != null && _cachedFpCtrl.MotorFreeFly;
            if (!jet && ctx.PlayerState != null)
            {
                // Detect airborne + holding jump/jetpack roughly via vertical velocity
                jet = _cachedFpCtrl != null && !_cachedFpCtrl.Grounded && _cachedFpCtrl.Velocity.y > 1.5f;
            }

            bool vacActive = _cachedWeaponVac != null && _cachedWeaponVac.InVacMode();

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
                BroadcastUnreliable(PacketType.PlayerTransform, SharedWriter, null);
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
                    peer.LastHeardTicks = DateTime.UtcNow.Ticks;
                    int queuedCount = CoopWorldSync.QueueFullSnapshotForPeer(sender);
                    CoopLog.Info("Host queued WorldSnapshot (" + queuedCount + " packets) for " + peer.Name + " (" + sender + ")");
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

                case PacketType.SiloSlotSync:
                    // Host is authoritative for silo/collector/feeder contents; ignore raw client overwrites.
                    break;

                case PacketType.SiloSlotTake:
                    if (peer == null) return;
                    string tPlotId = r.ReadString();
                    SiloStorage.StorageType tType = (SiloStorage.StorageType)r.ReadByte();
                    byte tSlot = r.ReadByte();
                    Identifiable.Id tItem = (Identifiable.Id)r.ReadInt();
                    CoopWorldSync.HandleHostSiloSlotTake(peer, tPlotId, tType, tSlot, tItem);
                    break;

                case PacketType.PlotCollectorActivate:
                    if (peer == null) return;
                    string pcPlotId = r.ReadString();
                    CoopWorldSync.ApplyPlotCollectorActivate(pcPlotId);
                    break;

                case PacketType.ExchangeAction:
                    if (peer == null) return;
                    byte exAction = r.ReadByte();
                    SceneContext exCtx = SRSingleton<SceneContext>.Instance;
                    if (exCtx == null || exCtx.ExchangeDirector == null) return;

                    if (exAction == 0) // SelectDailyOffer(rancherId, isFirstOffer)
                    {
                        string rancherId = r.ReadString();
                        bool isFirst = r.ReadBool();
                        bool accepted = exCtx.ExchangeDirector.SelectDailyOffer(rancherId, isFirst);
                        if (!accepted)
                        {
                            SharedWriter.Reset();
                            if (CoopWorldSync.WriteExchangeState(SharedWriter))
                            {
                                Transport.SendReliable(sender, PacketType.ExchangeStateSync, SharedWriter);
                            }
                        }
                        if (exCtx.ExchangeDirector.GetOfferId(ExchangeDirector.OfferType.GENERAL) != null)
                        {
                            SharedWriter.Reset();
                            SharedWriter.WriteByte((byte)ExchangeDirector.OfferType.GENERAL);
                            SharedWriter.WriteBool(false);
                            Transport.SendReliable(sender, PacketType.ExchangeOpenChat, SharedWriter);
                        }
                    }
                    else if (exAction == 1) // MaybeStartNext(offerType)
                    {
                        ExchangeDirector.OfferType oType = (ExchangeDirector.OfferType)r.ReadByte();
                        bool started = false;
                        CoopWorldSync.SuppressLocalRancherChatUI = true;
                        try
                        {
                            started = exCtx.ExchangeDirector.MaybeStartNext(oType);
                        }
                        finally
                        {
                            CoopWorldSync.SuppressLocalRancherChatUI = false;
                        }

                        if (started && exCtx.ProgressDirector != null)
                        {
                            if (oType == ExchangeDirector.OfferType.OGDEN)
                                exCtx.ProgressDirector.SetUniqueProgress(ProgressDirector.ProgressType.UNLOCK_WILDS);
                            else if (oType == ExchangeDirector.OfferType.MOCHI)
                                exCtx.ProgressDirector.SetUniqueProgress(ProgressDirector.ProgressType.UNLOCK_VALLEY);
                            else if (oType == ExchangeDirector.OfferType.VIKTOR)
                                exCtx.ProgressDirector.SetUniqueProgress(ProgressDirector.ProgressType.UNLOCK_SLIMULATIONS);
                        }

                        if (!started)
                        {
                            SharedWriter.Reset();
                            if (CoopWorldSync.WriteExchangeState(SharedWriter))
                            {
                                Transport.SendReliable(sender, PacketType.ExchangeStateSync, SharedWriter);
                            }
                        }

                        if (started || exCtx.ExchangeDirector.GetOfferId(oType) != null)
                        {
                            SharedWriter.Reset();
                            SharedWriter.WriteByte((byte)oType);
                            SharedWriter.WriteBool(started);
                            Transport.SendReliable(sender, PacketType.ExchangeOpenChat, SharedWriter);
                        }
                    }
                    else if (exAction == 2) // TryAccept(offerType, itemId, actorId)
                    {
                        ExchangeDirector.OfferType oType = (ExchangeDirector.OfferType)r.ReadByte();
                        Identifiable.Id itemId = (Identifiable.Id)r.ReadInt();
                        long actorId = r.ReadLong();

                        if (actorId > 0 && CoopWorldSync.TryGetActorObject(actorId, out GameObject actorGo) && actorGo != null)
                        {
                            Destroyer.DestroyActor(actorGo, "SlimeCoop.ExchangeAccept", false);
                        }

                        List<ExchangeDirector.Awarder> awarderList = new List<ExchangeDirector.Awarder>();
                        ExchangeEjector[] exEj = UnityEngine.Object.FindObjectsOfType<ExchangeEjector>();
                        if (exEj != null) awarderList.AddRange(exEj);
                        DirectRewardEjector[] dirEj = UnityEngine.Object.FindObjectsOfType<DirectRewardEjector>();
                        if (dirEj != null) awarderList.AddRange(dirEj);
                        RancherProgressAwarder[] progAw = UnityEngine.Object.FindObjectsOfType<RancherProgressAwarder>();
                        if (progAw != null) awarderList.AddRange(progAw);

                        if (!exCtx.ExchangeDirector.TryAccept(oType, itemId, awarderList.ToArray()))
                        {
                            SharedWriter.Reset();
                            if (CoopWorldSync.WriteExchangeState(SharedWriter))
                            {
                                Transport.SendReliable(sender, PacketType.ExchangeStateSync, SharedWriter);
                            }
                        }
                    }
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

                case PacketType.SnareStateSync:
                    // Host-authoritative snare state: discard client attempts to mutate snare directly.
                    r.ReadString();
                    r.ReadInt();
                    r.ReadInt();
                    r.ReadInt();
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

            if (setId == RegionRegistry.RegionSetId.UNSET)
            {
                RegionMember pm = ctx.Player != null ? ctx.Player.GetComponent<RegionMember>() : null;
                setId = pm != null && pm.setId != RegionRegistry.RegionSetId.UNSET ? pm.setId : RegionRegistry.RegionSetId.HOME;
            }

            NetSyncGuard.SuppressPersonalProgressDepth++;
            CoopWorldSync.PendingActorSpawnVelocity = vel;
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
                CoopWorldSync.PendingActorSpawnVelocity = null;
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
                    bool viaTcp = Transport.IsTcpPeer(sender);
                    if (!viaTcp)
                    {
                        Transport.CancelTcpFallback(_serverEp);
                    }
                    string protoUsed = viaTcp ? "TCP" : "UDP";
                    CoopLog.Info("Client received JoinAccept (" + protoUsed + ")! playerId=" + LocalPlayerId + ", serverId=" + HostServerId + ", room='" + RoomName + "'");
                    StatusMessage = "Подключено к '" + RoomName + "' (" + protoUsed + ")";
                    AddChat("Система", "Вы подключились к '" + RoomName + "'! Ваш прогресс апгрейдов, Слаймопедии и достижений личный.", true);

                    string currentSaveName = SRSingleton<GameContext>.Instance != null &&
                                             SRSingleton<GameContext>.Instance.AutoSaveDirector != null &&
                                             SRSingleton<GameContext>.Instance.AutoSaveDirector.SavedGame != null
                        ? SRSingleton<GameContext>.Instance.AutoSaveDirector.SavedGame.GetName()
                        : null;
                    bool alreadyInClientSandbox = !string.IsNullOrEmpty(currentSaveName) &&
                                                  currentSaveName.IndexOf("slimecoop", StringComparison.OrdinalIgnoreCase) >= 0;

                    if (SRSingleton<SceneContext>.Instance == null || !alreadyInClientSandbox)
                    {
                        BlockLocalSaveUntilMenu = true;
                        EnsureWorldLoadedForCoop(true);
                    }
                    else
                    {
                        CoopWorldSync.EnsureSceneObjectCaches();
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
                        CoopWorldSync.QueueClientSnapshotChunk(ref r);
                    }
                    break;

                case PacketType.WorldSnapshotEnd:
                    bool hasHostSpawn = r.Remaining > 0 && r.ReadBool();
                    Vector3 hostSpawn = hasHostSpawn ? r.ReadVector3() : Vector3.zero;
                    RegionRegistry.RegionSetId hostSet = hasHostSpawn ? (RegionRegistry.RegionSetId)r.ReadByte() : RegionRegistry.RegionSetId.HOME;
                    CoopLog.Info("Client received WorldSnapshotEnd (queued for smooth apply, hasHostSpawn=" + hasHostSpawn + ", pos=" + hostSpawn + ").");
                    CoopWorldSync.QueueClientSnapshotEnd(hasHostSpawn, hostSpawn, hostSet);
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
                {
                    string sPlotId = r.ReadString();
                    SiloStorage.StorageType sType = (SiloStorage.StorageType)r.ReadByte();
                    byte sSlot = r.ReadByte();
                    Identifiable.Id sItem = (Identifiable.Id)r.ReadInt();
                    int sCount = r.ReadInt();
                    uint sRev = r.ReadUInt();
                    uint ackTakes = r.ReadUInt();
                    CoopWorldSync.ApplyHostSiloSlot(sPlotId, sType, sSlot, sItem, sCount, sRev, ackTakes);
                    break;
                }

                case PacketType.SiloTakeGrant:
                {
                    Identifiable.Id grantedItem = (Identifiable.Id)r.ReadInt();
                    CoopWorldSync.ApplySiloTakeGrant(grantedItem);
                    break;
                }

                case PacketType.ExchangeStateSync:
                    CoopWorldSync.ApplyExchangeState(ref r);
                    break;

                case PacketType.ExchangeOpenChat:
                    CoopWorldSync.ApplyExchangeOpenChat((ExchangeDirector.OfferType)r.ReadByte(), r.ReadBool());
                    break;

                case PacketType.ExchangeCrateBreak:
                    long crateActorId = r.ReadLong();
                    byte specCount = r.ReadByte();
                    ExchangeDirector.NonIdentReward[] specs = new ExchangeDirector.NonIdentReward[specCount];
                    for (int i = 0; i < specCount; i++)
                    {
                        specs[i] = (ExchangeDirector.NonIdentReward)r.ReadInt();
                    }
                    CoopWorldSync.ApplyExchangeCrateBreak(crateActorId, specs);
                    break;

                case PacketType.ExchangeDirectCoin:
                    CoopWorldSync.ApplyExchangeDirectCoin((ExchangeDirector.OfferType)r.ReadByte());
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

                case PacketType.SnareStateSync:
                    CoopWorldSync.ApplySnareState(
                        r.ReadString(),
                        (Identifiable.Id)r.ReadInt(),
                        (Identifiable.Id)r.ReadInt(),
                        r.ReadInt());
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
                CoopWorldSync.CancelSnapshotForPeer(peer.EndPoint);
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
