using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Net;
using HarmonyLib;
using MonomiPark.SlimeRancher.DataModel;
using MonomiPark.SlimeRancher.Regions;
using SlimeCoop.Net;
using SlimeCoop.Protocol;
using UnityEngine;

namespace SlimeCoop.World
{
    internal struct SiloSlotKey : IEquatable<SiloSlotKey>
    {
        public string PlotId;
        public SiloStorage.StorageType StorageType;
        public int SlotIdx;

        public bool Equals(SiloSlotKey other)
        {
            return SlotIdx == other.SlotIdx &&
                   StorageType == other.StorageType &&
                   string.Equals(PlotId, other.PlotId, StringComparison.Ordinal);
        }

        public override bool Equals(object obj)
        {
            return obj is SiloSlotKey other && Equals(other);
        }

        public override int GetHashCode()
        {
            unchecked
            {
                int h = PlotId != null ? PlotId.GetHashCode() : 0;
                h = (h * 397) ^ (int)StorageType;
                h = (h * 397) ^ SlotIdx;
                return h;
            }
        }
    }

    internal static class NetSyncGuard
    {
        public static int ApplyingRemoteDepth;
        public static int SuppressPersonalProgressDepth;

        public static bool IsApplyingRemote => ApplyingRemoteDepth > 0;
        public static bool SuppressPersonalProgress => SuppressPersonalProgressDepth > 0;
    }

    internal static class CoopWorldSync
    {
        private static readonly AccessTools.FieldRef<LandPlotModel, GameObject> PlotModelGoRef =
            AccessTools.FieldRefAccess<LandPlotModel, GameObject>("gameObj");

        private static readonly AccessTools.FieldRef<LandPlot, LandPlotModel> PlotModelRef =
            AccessTools.FieldRefAccess<LandPlot, LandPlotModel>("model");

        private static readonly AccessTools.FieldRef<ActorModel, Transform> ActorTransformRef =
            AccessTools.FieldRefAccess<ActorModel, Transform>("transform");

        private static readonly AccessTools.FieldRef<GordoEat, GordoModel> GordoModelRef =
            AccessTools.FieldRefAccess<GordoEat, GordoModel>("gordoModel");

        private static readonly AccessTools.FieldRef<GordoEat, GordoRewardsBase> GordoRewardsRef =
            AccessTools.FieldRefAccess<GordoEat, GordoRewardsBase>("rewards");

        private static readonly AccessTools.FieldRef<GordoSnare, SnareModel> GordoSnareModelRef =
            AccessTools.FieldRefAccess<GordoSnare, SnareModel>("model");

        private static readonly AccessTools.FieldRef<GordoSnare, bool> GordoSnareIsSnaredRef =
            AccessTools.FieldRefAccess<GordoSnare, bool>("isSnared");

        private static readonly System.Reflection.MethodInfo GordoSnareAttachBaitMethod =
            AccessTools.Method(typeof(GordoSnare), "AttachBait", new[] { typeof(Identifiable.Id) });

        private static readonly System.Reflection.MethodInfo GordoSnareSnareGordoMethod =
            AccessTools.Method(typeof(GordoSnare), "SnareGordo", new[] { typeof(Identifiable.Id) });

        private static readonly System.Reflection.MethodInfo GordoSnareClearBaitMethod =
            AccessTools.Method(typeof(GordoSnare), "ClearBait");

        private static readonly AccessTools.FieldRef<TimeDirector, WorldModel> TimeWorldModelRef =
            AccessTools.FieldRefAccess<TimeDirector, WorldModel>("worldModel");

        private static readonly AccessTools.FieldRef<PlayerState, PlayerModel> PlayerModelRef =
            AccessTools.FieldRefAccess<PlayerState, PlayerModel>("model");

        private static readonly System.Reflection.MethodInfo GordoReachedTargetMethod =
            AccessTools.Method(typeof(GordoEat), "ReachedTarget");

        private static readonly System.Reflection.MethodInfo ExchangeOfferDidChangeMethod =
            AccessTools.Method(typeof(ExchangeDirector), "OfferDidChange");

        private static readonly AccessTools.FieldRef<ExchangeBreakOnImpact, bool> ExchangeBreakBreakingRef =
            AccessTools.FieldRefAccess<ExchangeBreakOnImpact, bool>("breaking");

        private static readonly System.Reflection.MethodInfo ExchangeBreakSpawnSpecMethod =
            AccessTools.Method(typeof(ExchangeBreakOnImpact), "SpawnSpecReward");

        private static readonly System.Reflection.MethodInfo WeaponVacCaptureEffectMethod =
            AccessTools.Method(typeof(WeaponVacuum), "CaptureEffect");

        private static readonly AccessTools.FieldRef<DirectRewardEjector, bool> DirectEjectorRewardIsActorRef =
            AccessTools.FieldRefAccess<DirectRewardEjector, bool>("rewardIsActor");

        public static bool SuppressLocalRancherChatUI;

        private static readonly ProgressDirector.ProgressType[] SyncedExchangeProgressTypes = new[]
        {
            ProgressDirector.ProgressType.UNLOCK_QUARRY,
            ProgressDirector.ProgressType.UNLOCK_MOSS,
            ProgressDirector.ProgressType.UNLOCK_DESERT,
            ProgressDirector.ProgressType.UNLOCK_RUINS,
            ProgressDirector.ProgressType.UNLOCK_WILDS,
            ProgressDirector.ProgressType.UNLOCK_VALLEY,
            ProgressDirector.ProgressType.UNLOCK_SLIMULATIONS,
            ProgressDirector.ProgressType.UNLOCK_OGDEN_MISSIONS,
            ProgressDirector.ProgressType.UNLOCK_MOCHI_MISSIONS,
            ProgressDirector.ProgressType.UNLOCK_VIKTOR_MISSIONS,
            ProgressDirector.ProgressType.EXCHANGE_THORA,
            ProgressDirector.ProgressType.EXCHANGE_VIKTOR,
            ProgressDirector.ProgressType.EXCHANGE_OGDEN,
            ProgressDirector.ProgressType.EXCHANGE_MOCHI,
            ProgressDirector.ProgressType.EXCHANGE_BOB,
            ProgressDirector.ProgressType.OGDEN_REWARDS,
            ProgressDirector.ProgressType.OGDEN_SEEN_FINAL_CHAT,
            ProgressDirector.ProgressType.MOCHI_REWARDS,
            ProgressDirector.ProgressType.MOCHI_SEEN_FINAL_CHAT,
            ProgressDirector.ProgressType.VIKTOR_REWARDS,
            ProgressDirector.ProgressType.VIKTOR_SEEN_FINAL_CHAT
        };

        private struct CachedNetActor
        {
            public GameObject Go;
            public Transform Tr;
            public Rigidbody Rb;
            public Vacuumable Vac;
        }

        private static readonly Dictionary<long, CachedNetActor> NetActors = new Dictionary<long, CachedNetActor>(512);
        private static readonly Dictionary<GameObject, long> ReverseActors = new Dictionary<GameObject, long>(512);

        private static readonly Dictionary<string, LandPlotLocation> PlotCache =
            new Dictionary<string, LandPlotLocation>(64);

        private static readonly Dictionary<string, AccessDoor> DoorCache =
            new Dictionary<string, AccessDoor>(32);

        private static readonly Dictionary<string, GordoEat> GordoCache =
            new Dictionary<string, GordoEat>(32);

        private static readonly Dictionary<string, Oasis> OasisCache =
            new Dictionary<string, Oasis>(32);

        private static readonly Dictionary<string, TreasurePod> PodCache =
            new Dictionary<string, TreasurePod>(64);

        private static readonly Dictionary<string, GadgetSite> GadgetSiteCache =
            new Dictionary<string, GadgetSite>(128);

        private struct PendingGadgetPlace
        {
            public Gadget.Id GadgetId;
            public float Yaw;
        }

        private struct PendingSnareState
        {
            public Identifiable.Id BaitTypeId;
            public Identifiable.Id GordoTypeId;
            public int GordoEatenCount;
        }

        private static readonly Dictionary<string, int> PendingGordoStates =
            new Dictionary<string, int>(32);

        private static readonly Dictionary<string, PendingGadgetPlace> PendingGadgetStates =
            new Dictionary<string, PendingGadgetPlace>(32);

        private static readonly Dictionary<string, PendingSnareState> PendingSnareStates =
            new Dictionary<string, PendingSnareState>(16);

        public static Vector3? PendingActorSpawnVelocity;

        private struct ActiveSyncItem
        {
            public long Id;
            public CachedNetActor Actor;
        }

        private struct HostSlotState
        {
            public Identifiable.Id ItemId;
            public int Count;
            public uint Revision;
        }

        private struct CachedHostSilo
        {
            public string PlotId;
            public SiloStorage Silo;
        }

        private static readonly Dictionary<string, SiloStorage[]> PlotSilosCache =
            new Dictionary<string, SiloStorage[]>(64);

        private static readonly List<CachedHostSilo> HostSiloEntries =
            new List<CachedHostSilo>(64);

        private static bool _hostSiloEntriesDirty = true;

        private static readonly PacketWriter SnapshotWriter = new PacketWriter(1200);
        private static readonly PacketWriter ClientTakeGrantWriter = new PacketWriter(64);
        private static readonly List<SiloStorage> SnapshotSiloScratch = new List<SiloStorage>(8);
        private static readonly List<LandPlot.Upgrade> SnapshotUpgradesScratch = new List<LandPlot.Upgrade>(8);
        private static readonly List<SiloSlotKey> SiloKeyScratch = new List<SiloSlotKey>(32);
        private static readonly List<KeyValuePair<float, KeyValuePair<long, ActorModel>>> SnapshotActorsScratch =
            new List<KeyValuePair<float, KeyValuePair<long, ActorModel>>>(512);

        private static readonly Dictionary<SiloSlotKey, HostSlotState> HostSiloStates =
            new Dictionary<SiloSlotKey, HostSlotState>(64);

        private static readonly Dictionary<SiloSlotKey, uint> ClientLastSlotRevision =
            new Dictionary<SiloSlotKey, uint>(64);

        private static uint _nextHostSiloRevision = 1;

        private struct QueuedSnapshotPacket
        {
            public PacketType Type;
            public byte[] Data;
        }

        private sealed class PeerSnapshotJob
        {
            public IPEndPoint EndPoint;
            public readonly Queue<QueuedSnapshotPacket> Packets = new Queue<QueuedSnapshotPacket>(128);
            public int TotalPackets;
        }

        private static readonly List<PeerSnapshotJob> ActiveSnapshotJobs = new List<PeerSnapshotJob>(4);
        private static readonly Queue<byte[]> ClientSnapshotApplyQueue = new Queue<byte[]>(128);
        private static readonly Stopwatch SnapshotBudgetTimer = new Stopwatch();
        private static bool _pendingSnapshotEnd;
        private static bool _pendingHostSpawn;
        private static Vector3 _pendingHostSpawnPos;
        private static RegionRegistry.RegionSetId _pendingHostSpawnSet;

        private static readonly List<ActiveSyncItem> PendingHostSyncActors = new List<ActiveSyncItem>(128);
        private static int _actorBatchCursor;

        public static void ClearCaches()
        {
            NetActors.Clear();
            ReverseActors.Clear();
            PlotCache.Clear();
            PlotSilosCache.Clear();
            HostSiloEntries.Clear();
            _hostSiloEntriesDirty = true;
            DoorCache.Clear();
            GordoCache.Clear();
            OasisCache.Clear();
            PodCache.Clear();
            GadgetSiteCache.Clear();
            PendingGordoStates.Clear();
            PendingGadgetStates.Clear();
            PendingSnareStates.Clear();
            PendingHostSyncActors.Clear();
            ActiveSnapshotJobs.Clear();
            ClientSnapshotApplyQueue.Clear();
            _pendingSnapshotEnd = false;
            _pendingHostSpawn = false;
            _actorBatchCursor = 0;
            HostSiloStates.Clear();
            ClientLastSlotRevision.Clear();
            _nextHostSiloRevision = 1;
        }

        public static void ClearLocalActorsBeforeSnapshot()
        {
            ClientLastSlotRevision.Clear();
            PendingGordoStates.Clear();
            PendingGadgetStates.Clear();
            PendingSnareStates.Clear();
            ClientSnapshotApplyQueue.Clear();
            _pendingSnapshotEnd = false;
            _pendingHostSpawn = false;

            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.GameModel == null) return;

            List<GameObject> toRemove = new List<GameObject>(128);
            foreach (KeyValuePair<long, ActorModel> kv in ctx.GameModel.AllActors())
            {
                ActorModel am = kv.Value;
                if (am == null || am.ident == Identifiable.Id.NONE || am.ident == Identifiable.Id.PLAYER) continue;
                Transform tr = ActorTransformRef(am);
                if (tr != null && tr.gameObject != null)
                {
                    toRemove.Add(tr.gameObject);
                }
            }

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                for (int i = 0; i < toRemove.Count; i++)
                {
                    if (toRemove[i] != null)
                    {
                        try
                        {
                            Destroyer.DestroyActor(toRemove[i], "SlimeCoop.SnapshotClear", true);
                        }
                        catch
                        {
                        }
                    }
                }
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
                NetSyncGuard.ApplyingRemoteDepth--;
            }

            NetActors.Clear();
            ReverseActors.Clear();
        }

        public static void RegisterActor(long actorId, GameObject go)
        {
            if (actorId <= 0 || go == null) return;
            NetActors[actorId] = new CachedNetActor
            {
                Go = go,
                Tr = go.transform,
                Rb = go.GetComponent<Rigidbody>(),
                Vac = go.GetComponent<Vacuumable>()
            };
            ReverseActors[go] = actorId;
        }

        public static void UnregisterActor(GameObject go)
        {
            if (go == null) return;
            if (ReverseActors.TryGetValue(go, out long id))
            {
                ReverseActors.Remove(go);
                if (NetActors.TryGetValue(id, out CachedNetActor cur) && ReferenceEquals(cur.Go, go))
                {
                    NetActors.Remove(id);
                }
            }
        }

        public static bool TryGetActorId(GameObject go, out long actorId)
        {
            if (go == null)
            {
                actorId = 0;
                return false;
            }
            if (ReverseActors.TryGetValue(go, out actorId))
            {
                return true;
            }
            try
            {
                actorId = Identifiable.GetActorId(go);
            }
            catch
            {
                actorId = 0;
            }
            if (actorId > 0)
            {
                RegisterActor(actorId, go);
                return true;
            }
            return false;
        }

        public static bool TryGetActorObject(long actorId, out GameObject go)
        {
            if (NetActors.TryGetValue(actorId, out CachedNetActor cached) && cached.Go != null)
            {
                go = cached.Go;
                return true;
            }
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx != null && ctx.GameModel != null && ctx.GameModel.AllActors().TryGetValue(actorId, out ActorModel am) && am != null)
            {
                Transform tr = ActorTransformRef(am);
                if (tr != null)
                {
                    go = tr.gameObject;
                    RegisterActor(actorId, go);
                    return true;
                }
            }
            go = null;
            return false;
        }

        private static bool TryGetCachedActor(long actorId, out CachedNetActor cached)
        {
            if (NetActors.TryGetValue(actorId, out cached) && cached.Go != null)
            {
                return true;
            }
            if (TryGetActorObject(actorId, out GameObject _) && NetActors.TryGetValue(actorId, out cached) && cached.Go != null)
            {
                return true;
            }
            cached = default(CachedNetActor);
            return false;
        }

        public static void EnsureSceneObjectCaches()
        {
            if (PlotCache.Count > 0 || DoorCache.Count > 0 || GordoCache.Count > 0) return;
            RebuildSceneObjectCaches();
        }

        public static void RebuildSceneObjectCaches()
        {
            PlotCache.Clear();
            PlotSilosCache.Clear();
            HostSiloEntries.Clear();
            _hostSiloEntriesDirty = true;
            LandPlotLocation[] plots = UnityEngine.Object.FindObjectsOfType<LandPlotLocation>();
            for (int i = 0; i < plots.Length; i++)
            {
                if (plots[i] != null && !string.IsNullOrEmpty(plots[i].id))
                {
                    PlotCache[plots[i].id] = plots[i];
                }
            }

            DoorCache.Clear();
            AccessDoor[] doors = UnityEngine.Object.FindObjectsOfType<AccessDoor>();
            for (int i = 0; i < doors.Length; i++)
            {
                if (doors[i] != null && !string.IsNullOrEmpty(doors[i].id))
                {
                    DoorCache[doors[i].id] = doors[i];
                }
            }

            GordoCache.Clear();
            GordoEat[] gordos = UnityEngine.Object.FindObjectsOfType<GordoEat>();
            for (int i = 0; i < gordos.Length; i++)
            {
                if (gordos[i] != null && !string.IsNullOrEmpty(gordos[i].id))
                {
                    GordoCache[gordos[i].id] = gordos[i];
                }
            }

            OasisCache.Clear();
            Oasis[] oases = UnityEngine.Object.FindObjectsOfType<Oasis>();
            for (int i = 0; i < oases.Length; i++)
            {
                if (oases[i] != null && !string.IsNullOrEmpty(oases[i].id))
                {
                    OasisCache[oases[i].id] = oases[i];
                }
            }

            PodCache.Clear();
            TreasurePod[] pods = UnityEngine.Object.FindObjectsOfType<TreasurePod>();
            for (int i = 0; i < pods.Length; i++)
            {
                if (pods[i] != null && !string.IsNullOrEmpty(pods[i].id))
                {
                    PodCache[pods[i].id] = pods[i];
                }
            }

            GadgetSiteCache.Clear();
            GadgetSite[] sites = UnityEngine.Object.FindObjectsOfType<GadgetSite>();
            for (int i = 0; i < sites.Length; i++)
            {
                if (sites[i] != null && !string.IsNullOrEmpty(sites[i].id))
                {
                    GadgetSiteCache[sites[i].id] = sites[i];
                }
            }

            // Also index currently spawned actors
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx != null && ctx.GameModel != null)
            {
                foreach (KeyValuePair<long, ActorModel> kv in ctx.GameModel.AllActors())
                {
                    if (kv.Value == null) continue;
                    Transform tr = ActorTransformRef(kv.Value);
                    if (tr != null)
                    {
                        RegisterActor(kv.Key, tr.gameObject);
                    }
                }
            }
        }

        private static LandPlotLocation FindPlot(string plotId)
        {
            if (string.IsNullOrEmpty(plotId)) return null;
            if (PlotCache.TryGetValue(plotId, out LandPlotLocation loc) && loc != null)
            {
                return loc;
            }
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx != null && ctx.GameModel != null && ctx.GameModel.AllLandPlots().TryGetValue(plotId, out LandPlotModel model) && model != null)
            {
                GameObject go = PlotModelGoRef(model);
                if (go != null)
                {
                    loc = go.GetComponent<LandPlotLocation>();
                    if (loc != null)
                    {
                        PlotCache[plotId] = loc;
                        return loc;
                    }
                }
            }
            RebuildSceneObjectCaches();
            PlotCache.TryGetValue(plotId, out loc);
            return loc;
        }

        public static void ApplyTimeWeather(double worldTime, AmbianceDirector.Weather weather, bool forceSnap = false)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.TimeDirector == null) return;

            WorldModel wm = TimeWorldModelRef(ctx.TimeDirector);
            if (wm != null)
            {
                bool isLocked = SRSingleton<LockOnDeath>.Instance != null && SRSingleton<LockOnDeath>.Instance.Locked();
                double threshold = forceSnap ? 1.5 : 120.0;
                if (!ctx.TimeDirector.IsFastForwarding() && !isLocked && Math.Abs(wm.worldTime - worldTime) > threshold)
                {
                    double oldTime = wm.worldTime;
                    double delta = worldTime - oldTime;
                    wm.worldTime = worldTime;

                    PlayerModel pm = ctx.PlayerState != null ? PlayerModelRef(ctx.PlayerState) : null;
                    if (pm != null)
                    {
                        if (!double.IsInfinity(pm.energyRecoverAfter) && !double.IsNaN(pm.energyRecoverAfter))
                        {
                            pm.energyRecoverAfter = Math.Min(pm.energyRecoverAfter + delta, worldTime + 300.0);
                        }
                        if (!double.IsInfinity(pm.healthBurstAfter) && !double.IsNaN(pm.healthBurstAfter))
                        {
                            pm.healthBurstAfter = Math.Min(pm.healthBurstAfter + delta, worldTime + 300.0);
                        }
                        if (!double.IsInfinity(pm.radRecoverAfter) && !double.IsNaN(pm.radRecoverAfter))
                        {
                            pm.radRecoverAfter = Math.Min(pm.radRecoverAfter + delta, worldTime + 600.0);
                        }
                    }
                }

                // Safety net: ensure personal recovery timers can never get stranded far in the future.
                PlayerModel pModel = ctx.PlayerState != null ? PlayerModelRef(ctx.PlayerState) : null;
                if (pModel != null)
                {
                    if (pModel.currEnergy < (float)pModel.maxEnergy &&
                        (double.IsInfinity(pModel.energyRecoverAfter) || double.IsNaN(pModel.energyRecoverAfter) || pModel.energyRecoverAfter > wm.worldTime + 300.0))
                    {
                        pModel.energyRecoverAfter = wm.worldTime + 300.0;
                    }
                    if (pModel.currHealth < (float)pModel.maxHealth &&
                        (double.IsInfinity(pModel.healthBurstAfter) || double.IsNaN(pModel.healthBurstAfter) || pModel.healthBurstAfter > wm.worldTime + 300.0))
                    {
                        pModel.healthBurstAfter = wm.worldTime + 300.0;
                    }
                    if (pModel.currRads > 0f &&
                        (double.IsInfinity(pModel.radRecoverAfter) || double.IsNaN(pModel.radRecoverAfter) || pModel.radRecoverAfter > wm.worldTime + 600.0))
                    {
                        pModel.radRecoverAfter = wm.worldTime + 600.0;
                    }
                }

                wm.currWeather = weather;
            }
        }

        public static void ApplyPlotBuild(string plotId, LandPlot.Id typeId)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            GameContext gctx = SRSingleton<GameContext>.Instance;
            if (ctx == null || gctx == null || gctx.LookupDirector == null) return;

            LandPlotLocation loc = FindPlot(plotId);
            if (loc == null) return;

            LandPlot cur = loc.GetComponentInChildren<LandPlot>();
            if (cur != null && cur.typeId == typeId) return;

            GameObject prefab = gctx.LookupDirector.GetPlotPrefab(typeId);
            if (prefab == null) return;

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                if (cur != null)
                {
                    loc.Replace(cur, prefab);
                    InvalidatePlotSilos(plotId);
                    ResetPlotSiloTracking(plotId);
                }
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
                NetSyncGuard.ApplyingRemoteDepth--;
            }
        }

        private static bool HasExtraPlotUpgrades(LandPlot cur, List<LandPlot.Upgrade> hostUpgrades)
        {
            if (cur == null) return false;
            LandPlotModel model = PlotModelRef(cur);
            if (model == null || model.upgrades == null || model.upgrades.Count == 0) return false;

            foreach (LandPlot.Upgrade u in model.upgrades)
            {
                if (!hostUpgrades.Contains(u))
                {
                    return true;
                }
            }
            return false;
        }

        public static void ClearPlotSilos(string plotId)
        {
            LandPlotLocation loc = FindPlot(plotId);
            if (loc == null) return;

            SiloStorage[] silos = GetPlotSilos(plotId, loc);
            if (silos == null || silos.Length == 0) return;

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                for (int i = 0; i < silos.Length; i++)
                {
                    SiloStorage ss = silos[i];
                    if (ss == null) continue;
                    Ammo ammo = ss.GetRelevantAmmo();
                    if (ammo == null) continue;
                    int sc = ammo.GetUsableSlotCount();
                    for (int s = 0; s < sc; s++)
                    {
                        if (ammo.GetSlotCount(s) > 0 || ammo.GetSlotName(s) != Identifiable.Id.NONE)
                        {
                            ammo.Clear(s);
                        }
                    }
                }
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
                NetSyncGuard.ApplyingRemoteDepth--;
            }
        }

        public static void ApplySnapshotLandPlot(string plotId, LandPlot.Id typeId, List<LandPlot.Upgrade> hostUpgrades, Identifiable.Id cropId)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            GameContext gctx = SRSingleton<GameContext>.Instance;
            if (ctx == null || gctx == null || gctx.LookupDirector == null) return;

            LandPlotLocation loc = FindPlot(plotId);
            if (loc == null) return;

            LandPlot cur = loc.GetComponentInChildren<LandPlot>();
            bool needReplace = cur == null || cur.typeId != typeId || HasExtraPlotUpgrades(cur, hostUpgrades);
            if (needReplace)
            {
                GameObject prefab = gctx.LookupDirector.GetPlotPrefab(typeId);
                if (prefab != null && cur != null)
                {
                    NetSyncGuard.ApplyingRemoteDepth++;
                    NetSyncGuard.SuppressPersonalProgressDepth++;
                    try
                    {
                        loc.Replace(cur, prefab);
                        InvalidatePlotSilos(plotId);
                    }
                    finally
                    {
                        NetSyncGuard.SuppressPersonalProgressDepth--;
                        NetSyncGuard.ApplyingRemoteDepth--;
                    }
                }
            }

            for (int i = 0; i < hostUpgrades.Count; i++)
            {
                ApplyPlotUpgrade(plotId, hostUpgrades[i]);
            }

            ApplyPlotPlant(plotId, cropId);
            ResetPlotSiloTracking(plotId);
            ClearPlotSilos(plotId);
        }

        public static void ApplyPlotUpgrade(string plotId, LandPlot.Upgrade upgrade)
        {
            LandPlotLocation loc = FindPlot(plotId);
            if (loc == null) return;

            LandPlot plot = loc.GetComponentInChildren<LandPlot>();
            if (plot == null || plot.HasUpgrade(upgrade)) return;

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                plot.AddUpgrade(upgrade);
                InvalidatePlotSilos(plotId);
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
                NetSyncGuard.ApplyingRemoteDepth--;
            }
        }

        public static void ApplyPlotPlant(string plotId, Identifiable.Id cropId)
        {
            LandPlotLocation loc = FindPlot(plotId);
            if (loc == null) return;

            LandPlot plot = loc.GetComponentInChildren<LandPlot>();
            if (plot == null) return;

            if (plot.GetAttachedCropId() == cropId) return;

            GardenCatcher catcher = plot.GetComponentInChildren<GardenCatcher>();
            if (catcher == null) return;

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                bool isRepl = plot.HasAttached();
                if (isRepl)
                {
                    plot.DestroyAttached();
                }
                if (cropId != Identifiable.Id.NONE)
                {
                    catcher.Plant(cropId, isRepl);
                }
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
                NetSyncGuard.ApplyingRemoteDepth--;
            }
        }

        private static void RemovePlotKeysFromDict<T>(Dictionary<SiloSlotKey, T> dict, string plotId)
        {
            if (dict == null || dict.Count == 0 || string.IsNullOrEmpty(plotId)) return;
            SiloKeyScratch.Clear();
            foreach (KeyValuePair<SiloSlotKey, T> kv in dict)
            {
                if (string.Equals(kv.Key.PlotId, plotId, StringComparison.Ordinal))
                {
                    SiloKeyScratch.Add(kv.Key);
                }
            }
            for (int i = 0; i < SiloKeyScratch.Count; i++)
            {
                dict.Remove(SiloKeyScratch[i]);
            }
            SiloKeyScratch.Clear();
        }

        public static void ResetPlotSiloTracking(string plotId)
        {
            if (string.IsNullOrEmpty(plotId)) return;
            RemovePlotKeysFromDict(HostSiloStates, plotId);
            RemovePlotKeysFromDict(ClientLastSlotRevision, plotId);
            CoopSession.ClearPeerSiloTakesForPlot(plotId);
        }

        public static void InvalidatePlotSilos(string plotId)
        {
            if (!string.IsNullOrEmpty(plotId))
            {
                PlotSilosCache.Remove(plotId);
            }
            _hostSiloEntriesDirty = true;
        }

        private static SiloStorage[] GetPlotSilos(string plotId, LandPlotLocation loc)
        {
            if (loc == null) return null;
            if (!string.IsNullOrEmpty(plotId) && PlotSilosCache.TryGetValue(plotId, out SiloStorage[] cached) && cached != null)
            {
                bool valid = true;
                for (int i = 0; i < cached.Length; i++)
                {
                    if (cached[i] == null)
                    {
                        valid = false;
                        break;
                    }
                }
                if (valid) return cached;
            }

            SiloStorage[] fresh = loc.GetComponentsInChildren<SiloStorage>(true);
            if (!string.IsNullOrEmpty(plotId) && fresh != null)
            {
                PlotSilosCache[plotId] = fresh;
                _hostSiloEntriesDirty = true;
            }
            return fresh;
        }

        private static void RebuildHostSiloEntries()
        {
            HostSiloEntries.Clear();
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.GameModel == null)
            {
                _hostSiloEntriesDirty = false;
                return;
            }

            foreach (KeyValuePair<string, LandPlotModel> kv in ctx.GameModel.AllLandPlots())
            {
                LandPlotLocation loc = FindPlot(kv.Key);
                if (loc == null) continue;
                SiloStorage[] silos = GetPlotSilos(kv.Key, loc);
                if (silos == null || silos.Length == 0) continue;

                for (int si = 0; si < silos.Length; si++)
                {
                    if (silos[si] != null)
                    {
                        HostSiloEntries.Add(new CachedHostSilo
                        {
                            PlotId = kv.Key,
                            Silo = silos[si]
                        });
                    }
                }
            }
            _hostSiloEntriesDirty = false;
        }

        public static uint RecordAndGetHostSlotRevision(string plotId, SiloStorage.StorageType storageType, int slotIdx, Identifiable.Id itemId, int count, out bool changed)
        {
            if (count <= 0 || itemId == Identifiable.Id.NONE)
            {
                itemId = Identifiable.Id.NONE;
                count = 0;
            }

            SiloSlotKey key = new SiloSlotKey
            {
                PlotId = plotId ?? string.Empty,
                StorageType = storageType,
                SlotIdx = slotIdx
            };

            if (HostSiloStates.TryGetValue(key, out HostSlotState state))
            {
                if (state.ItemId == itemId && state.Count == count)
                {
                    changed = false;
                    return state.Revision;
                }
            }

            uint nextRev = _nextHostSiloRevision++;
            if (nextRev == 0) nextRev = _nextHostSiloRevision++;
            HostSiloStates[key] = new HostSlotState
            {
                ItemId = itemId,
                Count = count,
                Revision = nextRev
            };
            changed = true;
            return nextRev;
        }

        public static void PushHostSiloSlot(SiloStorage silo, int slotIdx, bool forceBroadcast)
        {
            if (!CoopSession.IsHost || silo == null) return;
            LandPlotLocation loc = silo.GetComponentInParent<LandPlotLocation>();
            if (loc == null || string.IsNullOrEmpty(loc.id)) return;
            PushHostSiloSlotInternal(loc.id, silo, slotIdx, forceBroadcast);
        }

        private static void PushHostSiloSlotInternal(string plotId, SiloStorage silo, int slotIdx, bool forceBroadcast)
        {
            if (!CoopSession.IsHost || silo == null || string.IsNullOrEmpty(plotId)) return;

            Ammo ammo = silo.GetRelevantAmmo();
            if (ammo == null || slotIdx < 0 || slotIdx >= ammo.GetUsableSlotCount()) return;

            int count = ammo.GetSlotCount(slotIdx);
            Identifiable.Id itemId = count > 0 ? ammo.GetSlotName(slotIdx) : Identifiable.Id.NONE;
            if (itemId == Identifiable.Id.NONE) count = 0;

            uint rev = RecordAndGetHostSlotRevision(plotId, silo.type, slotIdx, itemId, count, out bool changed);
            if ((changed || forceBroadcast) && CoopSession.ConnectedPeers.Count > 0)
            {
                CoopSession.BroadcastHostSiloSlot(plotId, silo.type, slotIdx, itemId, count, rev);
            }
        }

        public static void HandleHostSiloSlotTake(CoopPeerInfo peer, string plotId, SiloStorage.StorageType storageType, int slotIdx, Identifiable.Id expectedItem)
        {
            if (!CoopSession.IsHost || peer == null) return;

            LandPlotLocation loc = FindPlot(plotId);
            if (loc == null) return;

            SiloStorage[] silos = GetPlotSilos(plotId, loc);
            if (silos == null || silos.Length == 0) return;

            SiloStorage target = null;
            for (int i = 0; i < silos.Length; i++)
            {
                if (silos[i] != null && silos[i].type == storageType)
                {
                    target = silos[i];
                    break;
                }
            }
            if (target == null) return;

            Ammo ammo = target.GetRelevantAmmo();
            if (ammo == null || slotIdx < 0 || slotIdx >= ammo.GetUsableSlotCount()) return;

            int curCount = ammo.GetSlotCount(slotIdx);
            Identifiable.Id curId = curCount > 0 ? ammo.GetSlotName(slotIdx) : Identifiable.Id.NONE;
            if (curCount > 0 && curId != Identifiable.Id.NONE &&
                (expectedItem == Identifiable.Id.NONE || expectedItem == curId))
            {
                SiloSlotKey slotKey = new SiloSlotKey
                {
                    PlotId = plotId ?? string.Empty,
                    StorageType = storageType,
                    SlotIdx = slotIdx
                };
                peer.IncrementSiloTakesProcessed(slotKey);

                NetSyncGuard.ApplyingRemoteDepth++;
                NetSyncGuard.SuppressPersonalProgressDepth++;
                try
                {
                    ammo.Decrement(slotIdx, 1);
                }
                finally
                {
                    NetSyncGuard.SuppressPersonalProgressDepth--;
                    NetSyncGuard.ApplyingRemoteDepth--;
                }

                PushHostSiloSlotInternal(plotId, target, slotIdx, true);
                CoopSession.SendSiloTakeGrant(peer, curId);
                return;
            }

            // Host state differs from client expectation (empty or different plort): force exact slot sync to this peer.
            uint rev = RecordAndGetHostSlotRevision(plotId, target.type, slotIdx, curId, curCount, out bool changed);
            if (changed && CoopSession.ConnectedPeers.Count > 0)
            {
                CoopSession.BroadcastHostSiloSlot(plotId, target.type, slotIdx, curId, curCount, rev);
            }
            else
            {
                CoopSession.SendHostSiloSlotToPeer(peer, plotId, target.type, slotIdx, curId, curCount, rev);
            }
        }

        public static void SyncDirtyHostSilos()
        {
            if (!CoopSession.IsHost || CoopSession.ConnectedPeers.Count == 0) return;
            if (_hostSiloEntriesDirty)
            {
                RebuildHostSiloEntries();
            }

            for (int i = 0; i < HostSiloEntries.Count; i++)
            {
                CachedHostSilo entry = HostSiloEntries[i];
                if (entry.Silo == null)
                {
                    _hostSiloEntriesDirty = true;
                    continue;
                }
                Ammo ammo = entry.Silo.GetRelevantAmmo();
                if (ammo == null) continue;
                int sc = ammo.GetUsableSlotCount();
                for (int s = 0; s < sc; s++)
                {
                    PushHostSiloSlotInternal(entry.PlotId, entry.Silo, s, false);
                }
            }
        }

        public static void SendClientSiloTake(SiloStorage silo, int slotIdx, Identifiable.Id itemId)
        {
            if (!CoopSession.IsClient || silo == null) return;
            LandPlotLocation loc = silo.GetComponentInParent<LandPlotLocation>();
            if (loc == null || string.IsNullOrEmpty(loc.id)) return;

            CoopSession.SendSiloSlotTake(loc.id, silo.type, slotIdx, itemId);
        }

        public static void ApplySnapshotSiloSlot(string plotId, SiloStorage.StorageType storageType, int slotIdx, Identifiable.Id itemId, int hostCount, uint hostRevision)
        {
            if (hostCount <= 0 || itemId == Identifiable.Id.NONE)
            {
                itemId = Identifiable.Id.NONE;
                hostCount = 0;
            }

            SiloSlotKey key = new SiloSlotKey
            {
                PlotId = plotId ?? string.Empty,
                StorageType = storageType,
                SlotIdx = slotIdx
            };
            ClientLastSlotRevision[key] = hostRevision;
            ApplySiloSlot(plotId, storageType, slotIdx, itemId, hostCount);
        }

        public static void ApplyHostSiloSlot(string plotId, SiloStorage.StorageType storageType, int slotIdx, Identifiable.Id itemId, int hostCount, uint hostRevision, uint ackedTakesCount)
        {
            if (hostCount <= 0 || itemId == Identifiable.Id.NONE)
            {
                itemId = Identifiable.Id.NONE;
                hostCount = 0;
            }

            SiloSlotKey key = new SiloSlotKey
            {
                PlotId = plotId ?? string.Empty,
                StorageType = storageType,
                SlotIdx = slotIdx
            };

            if (ClientLastSlotRevision.TryGetValue(key, out uint lastRev) && hostRevision < lastRev)
            {
                return;
            }
            ClientLastSlotRevision[key] = hostRevision;
            ApplySiloSlot(plotId, storageType, slotIdx, itemId, hostCount);
        }

        public static void ApplySiloTakeGrant(Identifiable.Id itemId)
        {
            if (!CoopSession.IsClient || itemId == Identifiable.Id.NONE) return;
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.PlayerState == null || ctx.PlayerState.Ammo == null) return;

            WeaponVacuum vac = ctx.Player != null ? ctx.Player.GetComponentInChildren<WeaponVacuum>() : null;
            if (ctx.PlayerState.Ammo.MaybeAddToSlot(itemId, null))
            {
                if (vac != null && WeaponVacCaptureEffectMethod != null)
                {
                    try
                    {
                        WeaponVacCaptureEffectMethod.Invoke(vac, null);
                    }
                    catch
                    {
                    }
                }
                return;
            }

            // Vacpack filled up while request was in flight: drop the item in front of the player via host.
            Vector3 pos = ctx.Player != null ? ctx.Player.transform.position + ctx.Player.transform.forward * 1.1f + Vector3.up * 0.9f : Vector3.zero;
            Vector3 vel = ctx.Player != null ? ctx.Player.transform.forward * 3.5f : Vector3.zero;
            if (vac != null && vac.vacOrigin != null)
            {
                pos = vac.vacOrigin.transform.position + vac.vacOrigin.transform.up * 0.8f;
                vel = vac.vacOrigin.transform.up * 3.5f;
            }
            RegionMember rm = ctx.Player != null ? ctx.Player.GetComponent<RegionMember>() : null;
            RegionRegistry.RegionSetId setId = rm != null && rm.setId != RegionRegistry.RegionSetId.UNSET
                ? rm.setId
                : RegionRegistry.RegionSetId.HOME;

            ClientTakeGrantWriter.Reset();
            ClientTakeGrantWriter.WriteInt((int)itemId);
            ClientTakeGrantWriter.WriteVector3(pos);
            ClientTakeGrantWriter.WriteVector3(vel);
            ClientTakeGrantWriter.WriteByte((byte)setId);
            CoopSession.SendToHostReliable(PacketType.ClientExpelRequest, ClientTakeGrantWriter);
        }

        public static void ApplySiloSlot(string plotId, SiloStorage.StorageType storageType, int slotIdx, Identifiable.Id itemId, int count)
        {
            if (count <= 0 || itemId == Identifiable.Id.NONE)
            {
                itemId = Identifiable.Id.NONE;
                count = 0;
            }

            LandPlotLocation loc = FindPlot(plotId);
            if (loc == null) return;

            SiloStorage[] silos = GetPlotSilos(plotId, loc);
            if (silos == null || silos.Length == 0) return;

            SiloStorage target = null;
            for (int i = 0; i < silos.Length; i++)
            {
                if (silos[i] != null && silos[i].type == storageType)
                {
                    target = silos[i];
                    break;
                }
            }
            if (target == null) return;

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                Ammo ammo = target.GetRelevantAmmo();
                if (ammo == null || slotIdx < 0 || slotIdx >= ammo.GetUsableSlotCount()) return;

                Identifiable.Id curId = ammo.GetSlotName(slotIdx);
                int curCount = ammo.GetSlotCount(slotIdx);
                if (curId == itemId && curCount == count) return;

                ammo.Clear(slotIdx);
                if (itemId != Identifiable.Id.NONE && count > 0)
                {
                    ammo.MaybeAddToSpecificSlot(itemId, null, slotIdx, count, true);
                }
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
                NetSyncGuard.ApplyingRemoteDepth--;
            }
        }

        public static void ApplyPlotCollectorActivate(string plotId)
        {
            LandPlotLocation loc = FindPlot(plotId);
            if (loc == null) return;

            PlortCollector collector = loc.GetComponentInChildren<PlortCollector>(false);
            if (collector != null && collector.isActiveAndEnabled)
            {
                collector.StartCollection();
            }
        }

        public static bool WriteExchangeState(PacketWriter w)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.TimeDirector == null || ctx.ExchangeDirector == null) return false;

            WorldModel wm = TimeWorldModelRef(ctx.TimeDirector);
            if (wm == null) return false;

            w.WriteDouble(wm.nextDailyOfferCreateTime);

            int pendingCount = Math.Min(32, wm.pendingOfferRancherIds.Count);
            w.WriteByte((byte)pendingCount);
            for (int i = 0; i < pendingCount; i++)
            {
                w.WriteString(wm.pendingOfferRancherIds[i] ?? string.Empty);
            }

            int lastCount = Math.Min(32, wm.lastOfferRancherIds.Count);
            w.WriteByte((byte)lastCount);
            for (int i = 0; i < lastCount; i++)
            {
                w.WriteString(wm.lastOfferRancherIds[i] ?? string.Empty);
            }

            int offerCount = Math.Min(32, wm.currOffers.Count);
            w.WriteByte((byte)offerCount);
            int writtenOffers = 0;
            foreach (KeyValuePair<ExchangeDirector.OfferType, ExchangeDirector.Offer> kv in wm.currOffers)
            {
                if (writtenOffers >= offerCount) break;
                writtenOffers++;
                ExchangeDirector.Offer o = kv.Value;
                w.WriteByte((byte)kv.Key);
                w.WriteString(o != null && o.offerId != null ? o.offerId : string.Empty);
                w.WriteString(o != null && o.rancherId != null ? o.rancherId : string.Empty);
                w.WriteDouble(o != null ? o.expireTime : double.PositiveInfinity);
                w.WriteDouble(o != null ? o.earlyExchangeTime : double.NegativeInfinity);

                int reqCount = o != null && o.requests != null ? Math.Min(32, o.requests.Count) : 0;
                w.WriteByte((byte)reqCount);
                for (int r = 0; r < reqCount; r++)
                {
                    ExchangeDirector.RequestedItemEntry req = o.requests[r];
                    w.WriteInt((int)req.id);
                    w.WriteInt(req.count);
                    w.WriteInt(req.progress);
                    w.WriteInt((int)req.specReward);
                }

                int rewCount = o != null && o.rewards != null ? Math.Min(32, o.rewards.Count) : 0;
                w.WriteByte((byte)rewCount);
                for (int rw = 0; rw < rewCount; rw++)
                {
                    ExchangeDirector.ItemEntry rew = o.rewards[rw];
                    w.WriteInt((int)rew.id);
                    w.WriteInt(rew.count);
                    w.WriteInt((int)rew.specReward);
                }
            }

            w.WriteByte((byte)SyncedExchangeProgressTypes.Length);
            for (int i = 0; i < SyncedExchangeProgressTypes.Length; i++)
            {
                int val = ctx.ProgressDirector != null ? ctx.ProgressDirector.GetProgress(SyncedExchangeProgressTypes[i]) : 0;
                w.WriteInt(val);
            }
            return true;
        }

        public static void ApplyExchangeState(ref PacketReader r)
        {
            double nextDailyTime = r.ReadDouble();

            byte pendingCount = r.ReadByte();
            List<string> pending = new List<string>(pendingCount);
            for (int i = 0; i < pendingCount; i++)
            {
                pending.Add(r.ReadString());
            }

            byte lastCount = r.ReadByte();
            List<string> last = new List<string>(lastCount);
            for (int i = 0; i < lastCount; i++)
            {
                last.Add(r.ReadString());
            }

            byte offerCount = r.ReadByte();
            Dictionary<ExchangeDirector.OfferType, ExchangeDirector.Offer> offers =
                new Dictionary<ExchangeDirector.OfferType, ExchangeDirector.Offer>(offerCount);
            for (int i = 0; i < offerCount; i++)
            {
                ExchangeDirector.OfferType oType = (ExchangeDirector.OfferType)r.ReadByte();
                string offerId = r.ReadString();
                string rancherId = r.ReadString();
                double expireTime = r.ReadDouble();
                double earlyTime = r.ReadDouble();

                byte reqCount = r.ReadByte();
                List<ExchangeDirector.RequestedItemEntry> reqs = new List<ExchangeDirector.RequestedItemEntry>(reqCount);
                for (int j = 0; j < reqCount; j++)
                {
                    Identifiable.Id id = (Identifiable.Id)r.ReadInt();
                    int cnt = r.ReadInt();
                    int prog = r.ReadInt();
                    ExchangeDirector.NonIdentReward spec = (ExchangeDirector.NonIdentReward)r.ReadInt();
                    reqs.Add(new ExchangeDirector.RequestedItemEntry(id, cnt, prog, spec));
                }

                byte rewCount = r.ReadByte();
                List<ExchangeDirector.ItemEntry> rews = new List<ExchangeDirector.ItemEntry>(rewCount);
                for (int j = 0; j < rewCount; j++)
                {
                    Identifiable.Id id = (Identifiable.Id)r.ReadInt();
                    int cnt = r.ReadInt();
                    ExchangeDirector.NonIdentReward spec = (ExchangeDirector.NonIdentReward)r.ReadInt();
                    rews.Add(new ExchangeDirector.ItemEntry(id, cnt, spec));
                }

                offers[oType] = new ExchangeDirector.Offer(offerId, rancherId, expireTime, earlyTime, reqs, rews);
            }

            byte progCount = r.ReadByte();
            int[] progVals = new int[progCount];
            for (int i = 0; i < progCount; i++)
            {
                progVals[i] = r.ReadInt();
            }

            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.TimeDirector == null || ctx.ExchangeDirector == null) return;

            WorldModel wm = TimeWorldModelRef(ctx.TimeDirector);
            if (wm == null) return;

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                wm.nextDailyOfferCreateTime = nextDailyTime;
                wm.pendingOfferRancherIds.Clear();
                wm.pendingOfferRancherIds.AddRange(pending);
                wm.lastOfferRancherIds.Clear();
                wm.lastOfferRancherIds.AddRange(last);

                wm.currOffers.Clear();
                foreach (KeyValuePair<ExchangeDirector.OfferType, ExchangeDirector.Offer> kv in offers)
                {
                    wm.currOffers[kv.Key] = kv.Value;
                }

                if (ctx.ProgressDirector != null)
                {
                    int n = Math.Min(progCount, SyncedExchangeProgressTypes.Length);
                    for (int i = 0; i < n; i++)
                    {
                        if (progVals[i] > 0)
                        {
                            ctx.ProgressDirector.SetProgress(SyncedExchangeProgressTypes[i], progVals[i]);
                        }
                    }
                }

                ExchangeOfferDidChangeMethod?.Invoke(ctx.ExchangeDirector, null);
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
                NetSyncGuard.ApplyingRemoteDepth--;
            }
        }

        public static void ApplyExchangeOpenChat(ExchangeDirector.OfferType offerType, bool intro)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.ExchangeDirector == null) return;

            NetSyncGuard.ApplyingRemoteDepth++;
            try
            {
                ctx.ExchangeDirector.CreateRancherChatUI(offerType, intro);
            }
            finally
            {
                NetSyncGuard.ApplyingRemoteDepth--;
            }
        }

        public static void ApplyExchangeCrateBreak(long crateActorId, ExchangeDirector.NonIdentReward[] specs)
        {
            if (crateActorId <= 0 || !TryGetActorObject(crateActorId, out GameObject go) || go == null) return;

            ExchangeBreakOnImpact ebi = go.GetComponent<ExchangeBreakOnImpact>();
            if (ebi == null) return;
            if (ExchangeBreakBreakingRef(ebi)) return;

            ExchangeBreakBreakingRef(ebi) = true;
            if (ebi.breakFX != null)
            {
                SRBehaviour.SpawnAndPlayFX(ebi.breakFX, go.transform.position, go.transform.rotation);
            }

            if (specs != null && ExchangeBreakSpawnSpecMethod != null)
            {
                for (int i = 0; i < specs.Length; i++)
                {
                    if (specs[i] != ExchangeDirector.NonIdentReward.NONE)
                    {
                        ExchangeBreakSpawnSpecMethod.Invoke(ebi, new object[] { specs[i] });
                    }
                }
            }
        }

        public static void ApplyExchangeDirectCoin(ExchangeDirector.OfferType offerType)
        {
            DirectRewardEjector[] ejectors = UnityEngine.Object.FindObjectsOfType<DirectRewardEjector>();
            if (ejectors == null) return;

            NetSyncGuard.ApplyingRemoteDepth++;
            try
            {
                for (int i = 0; i < ejectors.Length; i++)
                {
                    DirectRewardEjector ej = ejectors[i];
                    if (ej != null && ej.offerType == offerType && !DirectEjectorRewardIsActorRef(ej))
                    {
                        ej.Eject();
                    }
                }
            }
            finally
            {
                NetSyncGuard.ApplyingRemoteDepth--;
            }
        }

        public static void ApplyDoorState(string doorId, AccessDoor.State state)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx != null && ctx.GameModel != null && ctx.GameModel.AllDoors().TryGetValue(doorId, out AccessDoorModel dm) && dm != null)
            {
                dm.state = state;
            }

            if (!DoorCache.TryGetValue(doorId, out AccessDoor door) || door == null)
            {
                RebuildSceneObjectCaches();
                DoorCache.TryGetValue(doorId, out door);
            }

            if (door != null && door.CurrState != state)
            {
                NetSyncGuard.ApplyingRemoteDepth++;
                NetSyncGuard.SuppressPersonalProgressDepth++;
                try
                {
                    door.CurrState = state;
                }
                finally
                {
                    NetSyncGuard.SuppressPersonalProgressDepth--;
                    NetSyncGuard.ApplyingRemoteDepth--;
                }
            }
        }

        public static void OnClientGordoRegistered(string gordoId, GameObject gordoObj)
        {
            if (!CoopSession.IsClient || NetSyncGuard.IsApplyingRemote || string.IsNullOrEmpty(gordoId) || gordoObj == null)
            {
                return;
            }
            GordoEat gordo = gordoObj.GetComponent<GordoEat>();
            if (gordo != null)
            {
                GordoCache[gordoId] = gordo;
            }
            if (PendingGordoStates.TryGetValue(gordoId, out int eatenCount))
            {
                ApplyGordoEat(gordoId, eatenCount);
            }
        }

        public static void OnClientGordoModelSet(GordoEat gordo)
        {
            if (!CoopSession.IsClient || NetSyncGuard.IsApplyingRemote || gordo == null || string.IsNullOrEmpty(gordo.id))
            {
                return;
            }
            GordoCache[gordo.id] = gordo;
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx != null && ctx.GameModel != null && ctx.GameModel.AllGordos().ContainsKey(gordo.id) &&
                PendingGordoStates.TryGetValue(gordo.id, out int eatenCount))
            {
                ApplyGordoEat(gordo.id, eatenCount);
            }
        }

        public static void ApplyGordoEat(string gordoId, int eatenCount)
        {
            if (string.IsNullOrEmpty(gordoId)) return;
            PendingGordoStates[gordoId] = eatenCount;

            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx != null && ctx.GameModel != null && ctx.GameModel.AllGordos().TryGetValue(gordoId, out GordoModel gm) && gm != null)
            {
                gm.gordoEatenCount = eatenCount;
            }

            if (!GordoCache.TryGetValue(gordoId, out GordoEat gordo) || gordo == null)
            {
                if (ctx != null && ctx.GameModel != null && ctx.GameModel.AllGordos().ContainsKey(gordoId))
                {
                    GordoEat[] all = UnityEngine.Object.FindObjectsOfType<GordoEat>();
                    for (int i = 0; i < all.Length; i++)
                    {
                        if (all[i] != null && !string.IsNullOrEmpty(all[i].id))
                        {
                            GordoCache[all[i].id] = all[i];
                        }
                    }
                    GordoCache.TryGetValue(gordoId, out gordo);
                }
            }

            if (gordo != null)
            {
                NetSyncGuard.ApplyingRemoteDepth++;
                NetSyncGuard.SuppressPersonalProgressDepth++;
                try
                {
                    GordoModel model = GordoModelRef(gordo);
                    if (model != null)
                    {
                        model.gordoEatenCount = eatenCount;
                    }

                    if (eatenCount == -1)
                    {
                        GordoRewardsBase rew = GordoRewardsRef(gordo);
                        if (rew != null) rew.SetupActiveRewards();
                        gordo.gameObject.SetActive(false);
                    }
                    else
                    {
                        if (!gordo.gameObject.activeSelf)
                        {
                            if (gordo is EventGordoEat)
                            {
                                HolidayModel hm = ctx != null && ctx.GameModel != null ? ctx.GameModel.GetHolidayModel() : null;
                                if (hm != null && IsEventGordoActive(hm, gordoId))
                                {
                                    gordo.gameObject.SetActive(true);
                                }
                            }
                            else
                            {
                                gordo.gameObject.SetActive(true);
                            }
                        }
                        if (eatenCount >= gordo.GetTargetCount() && gordo.GetTargetCount() > 0 && GordoReachedTargetMethod != null)
                        {
                            var routine = GordoReachedTargetMethod.Invoke(gordo, null) as System.Collections.IEnumerator;
                            if (routine != null) gordo.StartCoroutine(routine);
                        }
                    }
                }
                finally
                {
                    NetSyncGuard.SuppressPersonalProgressDepth--;
                    NetSyncGuard.ApplyingRemoteDepth--;
                }
            }
            else if (eatenCount != 0)
            {
                CoopLog.Info("Deferred GordoEat state id=" + gordoId + " eaten=" + eatenCount + " (sector not loaded yet)");
            }
        }

        private static bool IsEventGordoActive(HolidayModel hm, string objectId)
        {
            if (hm == null || string.IsNullOrEmpty(objectId)) return false;
            foreach (HolidayModel.EventGordo eg in hm.eventGordos)
            {
                if (eg != null && eg.objectId == objectId) return true;
            }
            return false;
        }

        public static void ApplyHolidayState(List<string> eventGordoIds, List<string> echoNoteIds)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.GameModel == null) return;
            HolidayModel hm = ctx.GameModel.GetHolidayModel();
            if (hm == null) return;

            hm.eventGordos.Clear();
            if (eventGordoIds != null)
            {
                for (int i = 0; i < eventGordoIds.Count; i++)
                {
                    string id = eventGordoIds[i];
                    for (int j = 0; j < HolidayModel.EventGordo.INSTANCES.Length; j++)
                    {
                        HolidayModel.EventGordo inst = HolidayModel.EventGordo.INSTANCES[j];
                        if (inst != null && inst.id == id)
                        {
                            hm.eventGordos.Add(inst);
                            break;
                        }
                    }
                }
            }

            hm.eventEchoNoteGordos.Clear();
            if (echoNoteIds != null)
            {
                for (int i = 0; i < echoNoteIds.Count; i++)
                {
                    string id = echoNoteIds[i];
                    for (int j = 0; j < HolidayModel.EventEchoNoteGordo.INSTANCES.Length; j++)
                    {
                        HolidayModel.EventEchoNoteGordo inst = HolidayModel.EventEchoNoteGordo.INSTANCES[j];
                        if (inst != null && inst.id == id)
                        {
                            hm.eventEchoNoteGordos.Add(inst);
                            break;
                        }
                    }
                }
            }

            foreach (HolidayModel.EventGordo eg in hm.eventGordos)
            {
                if (eg == null) continue;
                GordoModel gm = ctx.GameModel.GetGordoModel(eg.objectId);
                if (gm != null && !gm.HasPopped())
                {
                    gm.SetGameObjectActive(true);
                }
            }

            EventGordoEat[] sceneEventGordos = UnityEngine.Object.FindObjectsOfType<EventGordoEat>();
            for (int i = 0; i < sceneEventGordos.Length; i++)
            {
                EventGordoEat ev = sceneEventGordos[i];
                if (ev == null || string.IsNullOrEmpty(ev.id)) continue;
                bool active = IsEventGordoActive(hm, ev.id) && !ev.HasPopped();
                if (ev.gameObject.activeSelf != active)
                {
                    ev.gameObject.SetActive(active);
                }
            }

            CoopLog.Info("Applied HolidayState: eventGordos=" + hm.eventGordos.Count + ", echoNoteGordos=" + hm.eventEchoNoteGordos.Count);
        }

        public static void ApplyOasisLive(string oasisId)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx != null && ctx.GameModel != null && ctx.GameModel.AllOases().TryGetValue(oasisId, out OasisModel om) && om != null)
            {
                om.isLive = true;
            }

            if (!OasisCache.TryGetValue(oasisId, out Oasis oasis) || oasis == null)
            {
                RebuildSceneObjectCaches();
                OasisCache.TryGetValue(oasisId, out oasis);
            }

            if (oasis != null)
            {
                NetSyncGuard.ApplyingRemoteDepth++;
                NetSyncGuard.SuppressPersonalProgressDepth++;
                try
                {
                    oasis.SetLive(true);
                }
                finally
                {
                    NetSyncGuard.SuppressPersonalProgressDepth--;
                    NetSyncGuard.ApplyingRemoteDepth--;
                }
            }
        }

        public static void ApplyPodState(string podId, TreasurePod.State state)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx != null && ctx.GameModel != null && ctx.GameModel.AllPods().TryGetValue(podId, out TreasurePodModel pm) && pm != null)
            {
                pm.state = state;
            }

            if (!PodCache.TryGetValue(podId, out TreasurePod pod) || pod == null)
            {
                RebuildSceneObjectCaches();
                PodCache.TryGetValue(podId, out pod);
            }

            if (pod != null && pod.CurrState != state)
            {
                NetSyncGuard.ApplyingRemoteDepth++;
                NetSyncGuard.SuppressPersonalProgressDepth++;
                try
                {
                    pod.CurrState = state;
                }
                finally
                {
                    NetSyncGuard.SuppressPersonalProgressDepth--;
                    NetSyncGuard.ApplyingRemoteDepth--;
                }
            }
        }

        public static void OnClientGadgetSiteRegistered(string siteId, GameObject siteObj)
        {
            if (!CoopSession.IsClient || NetSyncGuard.IsApplyingRemote || string.IsNullOrEmpty(siteId) || siteObj == null)
            {
                return;
            }
            GadgetSite site = siteObj.GetComponent<GadgetSite>();
            if (site != null)
            {
                GadgetSiteCache[siteId] = site;
            }
            if (PendingGadgetStates.TryGetValue(siteId, out PendingGadgetPlace place))
            {
                CoopLog.Info("Applying deferred gadget site=" + siteId + " gadget=" + place.GadgetId);
                ApplyGadgetPlace(siteId, place.GadgetId, place.Yaw);
            }
            if (PendingSnareStates.TryGetValue(siteId, out PendingSnareState snareState))
            {
                CoopLog.Info("Applying deferred snare site=" + siteId + " gordo=" + snareState.GordoTypeId + " bait=" + snareState.BaitTypeId);
                ApplySnareState(siteId, snareState.BaitTypeId, snareState.GordoTypeId, snareState.GordoEatenCount);
            }
        }

        public static void OnClientGadgetSiteModelSet(GadgetSite site)
        {
            if (!CoopSession.IsClient || NetSyncGuard.IsApplyingRemote || site == null || string.IsNullOrEmpty(site.id))
            {
                return;
            }
            GadgetSiteCache[site.id] = site;
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.GameModel == null || !ctx.GameModel.AllGadgetSites().ContainsKey(site.id))
            {
                return;
            }
            if (PendingGadgetStates.TryGetValue(site.id, out PendingGadgetPlace place))
            {
                ApplyGadgetPlace(site.id, place.GadgetId, place.Yaw);
            }
            if (PendingSnareStates.TryGetValue(site.id, out PendingSnareState snareState))
            {
                ApplySnareState(site.id, snareState.BaitTypeId, snareState.GordoTypeId, snareState.GordoEatenCount);
            }
        }

        public static void ApplyGadgetPlace(string siteId, Gadget.Id gadgetId, float yaw)
        {
            if (string.IsNullOrEmpty(siteId)) return;
            PendingGadgetStates[siteId] = new PendingGadgetPlace { GadgetId = gadgetId, Yaw = yaw };

            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            GameContext gctx = SRSingleton<GameContext>.Instance;
            if (ctx == null || gctx == null || ctx.GameModel == null || gctx.LookupDirector == null) return;

            if (!ctx.GameModel.AllGadgetSites().TryGetValue(siteId, out GadgetSiteModel siteModel) || siteModel == null)
            {
                return;
            }

            if (siteModel.attached != null && siteModel.attached.ident == gadgetId)
            {
                return;
            }

            GadgetDefinition def = gctx.LookupDirector.GetGadgetDefinition(gadgetId);
            if (def == null || def.prefab == null)
            {
                CoopLog.Warn("ApplyGadgetPlace: missing GadgetDefinition/prefab for " + gadgetId + " at site " + siteId);
                return;
            }

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                if (siteModel.HasAttached())
                {
                    GadgetSite site = null;
                    if (!GadgetSiteCache.TryGetValue(siteId, out site) || site == null)
                    {
                        site = siteModel.transform != null ? siteModel.transform.GetComponent<GadgetSite>() : null;
                    }
                    if (site != null)
                    {
                        site.DestroyAttached();
                    }
                }

                GameObject go = ctx.GameModel.InstantiateGadget(def.prefab, siteModel);
                if (go != null)
                {
                    go.transform.rotation = Quaternion.Euler(0f, yaw, 0f);
                }
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
                NetSyncGuard.ApplyingRemoteDepth--;
            }
        }

        public static void ApplyGadgetDestroy(string siteId)
        {
            if (string.IsNullOrEmpty(siteId)) return;
            PendingGadgetStates.Remove(siteId);
            PendingSnareStates.Remove(siteId);

            if (!GadgetSiteCache.TryGetValue(siteId, out GadgetSite site) || site == null)
            {
                RebuildSceneObjectCaches();
                GadgetSiteCache.TryGetValue(siteId, out site);
            }

            if (site != null && site.HasAttached())
            {
                NetSyncGuard.ApplyingRemoteDepth++;
                NetSyncGuard.SuppressPersonalProgressDepth++;
                try
                {
                    site.DestroyAttached();
                }
                finally
                {
                    NetSyncGuard.SuppressPersonalProgressDepth--;
                    NetSyncGuard.ApplyingRemoteDepth--;
                }
            }
        }

        public static bool TryGetSnareState(GordoSnare snare, out string siteId, out Identifiable.Id baitTypeId, out Identifiable.Id gordoTypeId, out int gordoEatenCount)
        {
            siteId = null;
            baitTypeId = Identifiable.Id.NONE;
            gordoTypeId = Identifiable.Id.NONE;
            gordoEatenCount = 0;
            if (snare == null) return false;

            GadgetSite site = snare.GetComponentInParent<GadgetSite>();
            if (site == null || string.IsNullOrEmpty(site.id)) return false;

            SnareModel model = GordoSnareModelRef(snare);
            if (model == null) return false;

            siteId = site.id;
            baitTypeId = model.baitTypeId;
            gordoTypeId = model.gordoTypeId;
            gordoEatenCount = model.gordoEatenCount;
            return true;
        }

        public static void ApplySnareState(string siteId, Identifiable.Id baitTypeId, Identifiable.Id gordoTypeId, int gordoEatenCount)
        {
            if (string.IsNullOrEmpty(siteId)) return;
            PendingSnareStates[siteId] = new PendingSnareState
            {
                BaitTypeId = baitTypeId,
                GordoTypeId = gordoTypeId,
                GordoEatenCount = gordoEatenCount
            };

            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.GameModel == null) return;

            if (!ctx.GameModel.AllGadgetSites().TryGetValue(siteId, out GadgetSiteModel siteModel) || siteModel == null)
            {
                return;
            }

            SnareModel snareModel = siteModel.attached as SnareModel;
            if (snareModel == null)
            {
                CoopLog.Warn("ApplySnareState: attached model is not SnareModel at site " + siteId);
                return;
            }

            GordoSnare snare = null;
            if (snareModel.transform != null)
            {
                snare = snareModel.transform.GetComponentInChildren<GordoSnare>(true);
            }
            if (snare == null && GadgetSiteCache.TryGetValue(siteId, out GadgetSite cachedSite) && cachedSite != null)
            {
                snare = cachedSite.GetComponentInChildren<GordoSnare>(true);
            }
            if (snare == null && siteModel.transform != null)
            {
                snare = siteModel.transform.GetComponentInChildren<GordoSnare>(true);
            }

            if (snare == null)
            {
                CoopLog.Warn("ApplySnareState: GordoSnare component missing at site " + siteId + " (gordo=" + gordoTypeId + ")");
                snareModel.baitTypeId = baitTypeId;
                snareModel.gordoTypeId = gordoTypeId;
                snareModel.gordoEatenCount = gordoEatenCount;
                return;
            }

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                if (gordoTypeId != Identifiable.Id.NONE)
                {
                    if ((snareModel.gordoTypeId != gordoTypeId || !GordoSnareIsSnaredRef(snare)) && GordoSnareSnareGordoMethod != null)
                    {
                        GordoSnareSnareGordoMethod.Invoke(snare, new object[] { gordoTypeId });
                    }
                    snareModel.gordoEatenCount = gordoEatenCount;
                    CoopLog.Info("ApplySnareState: snared gordo=" + gordoTypeId + " eaten=" + gordoEatenCount + " at site=" + siteId);
                    GordoEat ge = snare.GetComponentInChildren<GordoEat>(true);
                    if (ge != null)
                    {
                        if (gordoEatenCount == -1)
                        {
                            GordoRewardsBase rew = GordoRewardsRef(ge);
                            if (rew != null) rew.SetupActiveRewards();
                            ge.gameObject.SetActive(false);
                        }
                        else
                        {
                            if (!ge.gameObject.activeSelf) ge.gameObject.SetActive(true);
                            if (gordoEatenCount >= ge.GetTargetCount() && ge.GetTargetCount() > 0 && GordoReachedTargetMethod != null)
                            {
                                var routine = GordoReachedTargetMethod.Invoke(ge, null) as System.Collections.IEnumerator;
                                if (routine != null) ge.StartCoroutine(routine);
                            }
                        }
                    }
                }
                else if (baitTypeId != Identifiable.Id.NONE)
                {
                    if (snareModel.baitTypeId != baitTypeId && GordoSnareAttachBaitMethod != null)
                    {
                        GordoSnareAttachBaitMethod.Invoke(snare, new object[] { baitTypeId });
                    }
                }
                else
                {
                    if (GordoSnareClearBaitMethod != null)
                    {
                        GordoSnareClearBaitMethod.Invoke(snare, null);
                    }
                    snareModel.baitTypeId = Identifiable.Id.NONE;
                }
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
                NetSyncGuard.ApplyingRemoteDepth--;
            }
        }

        public static void ApplyActorSpawn(long actorId, Identifiable.Id id, Vector3 pos, Quaternion rot, Vector3 vel, RegionRegistry.RegionSetId setId)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            GameContext gctx = SRSingleton<GameContext>.Instance;
            if (ctx == null || gctx == null || ctx.GameModel == null || gctx.LookupDirector == null) return;

            if (TryGetActorObject(actorId, out GameObject existing) && existing != null)
            {
                existing.transform.SetPositionAndRotation(pos, rot);
                Rigidbody rbEx = existing.GetComponent<Rigidbody>();
                if (rbEx != null) rbEx.velocity = vel;
                return;
            }

            GameObject prefab = null;
            try { prefab = gctx.LookupDirector.GetPrefab(id); } catch { }
            if (prefab == null) return;

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                GameObject go = ctx.GameModel.InstantiateActorWithoutNotify(actorId, prefab, setId, pos, rot);
                if (go != null)
                {
                    if (ctx.GameModel.AllActors().TryGetValue(actorId, out ActorModel am) && am != null)
                    {
                        am.NotifyParticipants(go);
                    }
                    RegisterActor(actorId, go);
                    Rigidbody rb = go.GetComponent<Rigidbody>();
                    if (rb != null)
                    {
                        rb.velocity = vel;
                    }
                }
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
                NetSyncGuard.ApplyingRemoteDepth--;
            }
        }

        public static void ApplyActorDestroy(long actorId)
        {
            if (!TryGetActorObject(actorId, out GameObject go) || go == null)
            {
                return;
            }

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                UnregisterActor(go);
                Destroyer.DestroyActor(go, "SlimeCoop.RemoteDestroy", true);
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
                NetSyncGuard.ApplyingRemoteDepth--;
            }
        }

        public static void ApplyActorBatch(ref PacketReader r)
        {
            int count = r.ReadUShort();
            for (int i = 0; i < count; i++)
            {
                long actorId = r.ReadLong();
                Vector3 pos = r.ReadVector3();
                Vector3 vel = r.ReadVector3();

                if (TryGetCachedActor(actorId, out CachedNetActor entry))
                {
                    if (!entry.Go.activeInHierarchy) continue;
                    if (entry.Vac != null && (entry.Vac.isHeld() || entry.Vac.isCaptive()))
                    {
                        continue; // local player is holding or vacuuming it
                    }

                    Vector3 diff = pos - entry.Tr.position;
                    float sqr = diff.sqrMagnitude;
                    if (sqr > 16f)
                    {
                        entry.Tr.position = pos;
                    }
                    else if (sqr > 0.0025f)
                    {
                        entry.Tr.position = Vector3.Lerp(entry.Tr.position, pos, 0.35f);
                    }

                    if (entry.Rb != null)
                    {
                        entry.Rb.velocity = vel;
                    }
                }
            }
        }

        public static void CancelSnapshotForPeer(IPEndPoint ep)
        {
            if (ep == null) return;
            for (int i = ActiveSnapshotJobs.Count - 1; i >= 0; i--)
            {
                if (ActiveSnapshotJobs[i].EndPoint != null && ActiveSnapshotJobs[i].EndPoint.Equals(ep))
                {
                    ActiveSnapshotJobs.RemoveAt(i);
                }
            }
        }

        public static int QueueFullSnapshotForPeer(IPEndPoint ep)
        {
            if (ep == null) return 0;
            CancelSnapshotForPeer(ep);

            PeerSnapshotJob job = new PeerSnapshotJob { EndPoint = ep };
            SendFullSnapshotToClient(chunk =>
            {
                byte[] copy = new byte[chunk.Length];
                Buffer.BlockCopy(chunk.Buffer, 0, copy, 0, chunk.Length);
                job.Packets.Enqueue(new QueuedSnapshotPacket
                {
                    Type = PacketType.WorldSnapshotChunk,
                    Data = copy
                });
            });

            SceneContext hctx = SRSingleton<SceneContext>.Instance;
            bool hasHostPos = hctx != null && hctx.Player != null;
            Vector3 hostSpawnPos = hasHostPos ? hctx.Player.transform.position : Vector3.zero;
            RegionMember hrm = hasHostPos ? hctx.Player.GetComponent<RegionMember>() : null;
            RegionRegistry.RegionSetId hSet = hrm != null ? hrm.setId : RegionRegistry.RegionSetId.HOME;

            PacketWriter endW = SnapshotWriter;
            endW.Reset();
            endW.WriteBool(hasHostPos);
            endW.WriteVector3(hostSpawnPos);
            endW.WriteByte((byte)hSet);
            byte[] endCopy = new byte[endW.Length];
            Buffer.BlockCopy(endW.Buffer, 0, endCopy, 0, endW.Length);
            job.Packets.Enqueue(new QueuedSnapshotPacket
            {
                Type = PacketType.WorldSnapshotEnd,
                Data = endCopy
            });

            job.TotalPackets = job.Packets.Count;
            ActiveSnapshotJobs.Add(job);
            return job.TotalPackets;
        }

        public static void PumpSnapshotSends(NetTransport transport, int maxPacketsPerFrame = 6)
        {
            if (transport == null || !transport.IsActive || ActiveSnapshotJobs.Count == 0) return;

            int sent = 0;
            int idx = 0;
            while (ActiveSnapshotJobs.Count > 0 && sent < maxPacketsPerFrame)
            {
                if (idx >= ActiveSnapshotJobs.Count) idx = 0;
                PeerSnapshotJob job = ActiveSnapshotJobs[idx];
                if (job.Packets.Count == 0)
                {
                    ActiveSnapshotJobs.RemoveAt(idx);
                    continue;
                }

                QueuedSnapshotPacket pkt = job.Packets.Dequeue();
                transport.SendReliable(job.EndPoint, pkt.Type, pkt.Data, pkt.Data != null ? pkt.Data.Length : 0);
                sent++;

                if (job.Packets.Count == 0)
                {
                    CoopLog.Info("Host finished streaming WorldSnapshot (" + job.TotalPackets + " packets) to " + job.EndPoint);
                    ActiveSnapshotJobs.RemoveAt(idx);
                }
                else
                {
                    idx++;
                }
            }
        }

        public static void QueueClientSnapshotChunk(ref PacketReader r)
        {
            int rem = r.Remaining;
            if (rem <= 0) return;
            byte[] copy = new byte[rem];
            Buffer.BlockCopy(r.Buffer, r.Offset, copy, 0, rem);
            ClientSnapshotApplyQueue.Enqueue(copy);
        }

        public static void QueueClientSnapshotEnd(bool hasHostSpawn, Vector3 hostSpawn, RegionRegistry.RegionSetId hostSet)
        {
            _pendingSnapshotEnd = true;
            _pendingHostSpawn = hasHostSpawn;
            _pendingHostSpawnPos = hostSpawn;
            _pendingHostSpawnSet = hostSet;
        }

        public static bool PumpClientSnapshotApply(out bool finishedThisFrame, out bool hasHostSpawn, out Vector3 hostSpawn, out RegionRegistry.RegionSetId hostSet)
        {
            finishedThisFrame = false;
            hasHostSpawn = false;
            hostSpawn = Vector3.zero;
            hostSet = RegionRegistry.RegionSetId.HOME;

            if (ClientSnapshotApplyQueue.Count > 0)
            {
                SnapshotBudgetTimer.Reset();
                SnapshotBudgetTimer.Start();
                int applied = 0;
                while (ClientSnapshotApplyQueue.Count > 0)
                {
                    byte[] raw = ClientSnapshotApplyQueue.Dequeue();
                    try
                    {
                        PacketReader pr = new PacketReader(raw, 0, raw.Length);
                        ApplySnapshotChunk(ref pr);
                    }
                    catch (Exception ex)
                    {
                        CoopLog.Warn("ApplySnapshotChunk error: " + ex.Message);
                    }
                    applied++;
                    if (applied >= 1 && SnapshotBudgetTimer.ElapsedMilliseconds >= 6)
                    {
                        break;
                    }
                }
                SnapshotBudgetTimer.Stop();
            }

            if (_pendingSnapshotEnd && ClientSnapshotApplyQueue.Count == 0)
            {
                _pendingSnapshotEnd = false;
                finishedThisFrame = true;
                hasHostSpawn = _pendingHostSpawn;
                hostSpawn = _pendingHostSpawnPos;
                hostSet = _pendingHostSpawnSet;
            }
            return finishedThisFrame;
        }

        public static void SendFullSnapshotToClient(Action<PacketWriter> sendReliable)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.GameModel == null || sendReliable == null) return;

            EnsureSceneObjectCaches();
            PacketWriter w = SnapshotWriter;

            // 1. Time & Weather
            if (ctx.TimeDirector != null)
            {
                WorldModel wm = TimeWorldModelRef(ctx.TimeDirector);
                w.Reset();
                w.WriteByte(1); // chunk kind: TimeWeather
                w.WriteDouble(ctx.TimeDirector.WorldTime());
                w.WriteByte(wm != null ? (byte)wm.currWeather : (byte)0);
                sendReliable(w);
            }

            // 1b. Range Exchange offers & quest progress
            w.Reset();
            w.WriteByte(9); // chunk kind: ExchangeState
            if (WriteExchangeState(w))
            {
                sendReliable(w);
            }

            // 2. LandPlots (build + upgrades + crop + silo slots)
            foreach (KeyValuePair<string, LandPlotModel> kv in ctx.GameModel.AllLandPlots())
            {
                LandPlotModel m = kv.Value;
                if (m == null) continue;

                w.Reset();
                w.WriteByte(2); // chunk kind: LandPlot
                w.WriteString(kv.Key);
                w.WriteInt((int)m.typeId);
                w.WriteInt((int)m.attachedResourceId);
                w.WriteByte((byte)m.upgrades.Count);
                foreach (LandPlot.Upgrade upg in m.upgrades)
                {
                    w.WriteInt((int)upg);
                }

                // Silo storages if present (e.g. Corral has FOOD feeder + PLORT collector)
                LandPlotLocation loc = FindPlot(kv.Key);
                if (loc != null)
                {
                    SiloStorage[] silos = GetPlotSilos(kv.Key, loc);
                    SnapshotSiloScratch.Clear();
                    if (silos != null)
                    {
                        for (int si = 0; si < silos.Length; si++)
                        {
                            if (silos[si] != null && silos[si].GetRelevantAmmo() != null)
                            {
                                SnapshotSiloScratch.Add(silos[si]);
                            }
                        }
                    }

                    int siloCount = Math.Min(16, SnapshotSiloScratch.Count);
                    w.WriteByte((byte)siloCount);
                    for (int si = 0; si < siloCount; si++)
                    {
                        SiloStorage ss = SnapshotSiloScratch[si];
                        Ammo a = ss.GetRelevantAmmo();
                        int sc = Math.Min(12, a.GetUsableSlotCount());
                        w.WriteByte((byte)ss.type);
                        w.WriteByte((byte)sc);
                        for (int s = 0; s < sc; s++)
                        {
                            int cnt = a.GetSlotCount(s);
                            Identifiable.Id itemId = cnt > 0 ? a.GetSlotName(s) : Identifiable.Id.NONE;
                            if (itemId == Identifiable.Id.NONE) cnt = 0;
                            uint rev = RecordAndGetHostSlotRevision(kv.Key, ss.type, s, itemId, cnt, out _);
                            w.WriteInt((int)itemId);
                            w.WriteInt(cnt);
                            w.WriteUInt(rev);
                        }
                    }
                    SnapshotSiloScratch.Clear();
                }
                else
                {
                    w.WriteByte(0);
                }
                sendReliable(w);
            }

            // 3. Open Doors
            foreach (KeyValuePair<string, AccessDoorModel> kv in ctx.GameModel.AllDoors())
            {
                if (kv.Value != null && kv.Value.state != AccessDoor.State.LOCKED)
                {
                    w.Reset();
                    w.WriteByte(3); // Door
                    w.WriteString(kv.Key);
                    w.WriteByte((byte)kv.Value.state);
                    sendReliable(w);
                }
            }

            // 3b. Holiday Event Gordos (Party Gordo & Twinkle Slime)
            HolidayModel hm = ctx.GameModel.GetHolidayModel();
            if (hm != null)
            {
                w.Reset();
                w.WriteByte(11); // chunk kind: HolidayState
                w.WriteByte((byte)Math.Min(255, hm.eventGordos.Count));
                int evWritten = 0;
                foreach (HolidayModel.EventGordo eg in hm.eventGordos)
                {
                    if (evWritten >= 255) break;
                    w.WriteString(eg != null ? eg.id : string.Empty);
                    evWritten++;
                }
                w.WriteByte((byte)Math.Min(255, hm.eventEchoNoteGordos.Count));
                int echoWritten = 0;
                foreach (HolidayModel.EventEchoNoteGordo en in hm.eventEchoNoteGordos)
                {
                    if (echoWritten >= 255) break;
                    w.WriteString(en != null ? en.id : string.Empty);
                    echoWritten++;
                }
                sendReliable(w);
            }

            // 4. Gordos (send all registered gordos so client matches host state even for 0-eaten / active event gordos)
            int totalGordos = 0;
            int nonZeroGordos = 0;
            foreach (KeyValuePair<string, GordoModel> kv in ctx.GameModel.AllGordos())
            {
                if (kv.Value != null)
                {
                    totalGordos++;
                    if (kv.Value.gordoEatenCount != 0) nonZeroGordos++;
                    w.Reset();
                    w.WriteByte(4); // Gordo
                    w.WriteString(kv.Key);
                    w.WriteInt(kv.Value.gordoEatenCount);
                    sendReliable(w);
                }
            }

            // 5. Oases
            foreach (KeyValuePair<string, OasisModel> kv in ctx.GameModel.AllOases())
            {
                if (kv.Value != null && kv.Value.isLive)
                {
                    w.Reset();
                    w.WriteByte(5); // Oasis
                    w.WriteString(kv.Key);
                    sendReliable(w);
                }
            }

            // 6. Pods
            foreach (KeyValuePair<string, TreasurePodModel> kv in ctx.GameModel.AllPods())
            {
                if (kv.Value != null && kv.Value.state != TreasurePod.State.LOCKED)
                {
                    w.Reset();
                    w.WriteByte(6); // Pod
                    w.WriteString(kv.Key);
                    w.WriteByte((byte)kv.Value.state);
                    sendReliable(w);
                }
            }

            // 7. Gadgets
            int totalGadgets = 0;
            int totalSnares = 0;
            foreach (KeyValuePair<string, GadgetSiteModel> kv in ctx.GameModel.AllGadgetSites())
            {
                if (kv.Value != null && kv.Value.attached != null)
                {
                    totalGadgets++;
                    w.Reset();
                    w.WriteByte(7); // Gadget
                    w.WriteString(kv.Key);
                    w.WriteInt((int)kv.Value.attached.ident);
                    w.WriteFloat(kv.Value.attached.transform != null ? kv.Value.attached.transform.eulerAngles.y : 0f);
                    SnareModel snare = kv.Value.attached as SnareModel;
                    w.WriteBool(snare != null);
                    if (snare != null)
                    {
                        totalSnares++;
                        w.WriteInt((int)snare.baitTypeId);
                        w.WriteInt((int)snare.gordoTypeId);
                        w.WriteInt(snare.gordoEatenCount);
                        CoopLog.Info("Snapshot snare site=" + kv.Key + " gadget=" + kv.Value.attached.ident +
                                     " bait=" + snare.baitTypeId + " gordo=" + snare.gordoTypeId + " eaten=" + snare.gordoEatenCount);
                    }
                    sendReliable(w);
                }
            }
            CoopLog.Info("Snapshot built: gordos=" + totalGordos + " (nonZero=" + nonZeroGordos + "), gadgets=" + totalGadgets + " (snares=" + totalSnares + ")");

            // 8. Nearby Actors (sorted by distance to host, packed in batches of 8 to avoid network stalls)
            Vector3 hostPos = ctx.Player != null ? ctx.Player.transform.position : Vector3.zero;
            SnapshotActorsScratch.Clear();
            foreach (KeyValuePair<long, ActorModel> kv in ctx.GameModel.AllActors())
            {
                ActorModel am = kv.Value;
                if (am == null || am.ident == Identifiable.Id.NONE || am.ident == Identifiable.Id.PLAYER) continue;
                Transform tr = ActorTransformRef(am);
                if (tr == null) continue;
                float sqr = (tr.position - hostPos).sqrMagnitude;
                if (sqr > 150f * 150f) continue;
                SnapshotActorsScratch.Add(new KeyValuePair<float, KeyValuePair<long, ActorModel>>(sqr, kv));
            }

            SnapshotActorsScratch.Sort((a, b) => a.Key.CompareTo(b.Key));
            int limit = Math.Min(350, SnapshotActorsScratch.Count);
            const int batchSize = 8;
            for (int start = 0; start < limit; start += batchSize)
            {
                int batchCount = Math.Min(batchSize, limit - start);
                int validCount = 0;
                for (int k = 0; k < batchCount; k++)
                {
                    Transform tr = ActorTransformRef(SnapshotActorsScratch[start + k].Value.Value);
                    if (tr != null) validCount++;
                }
                if (validCount == 0) continue;

                w.Reset();
                w.WriteByte(10); // chunk kind: ActorSpawnBatch
                w.WriteByte((byte)validCount);
                for (int k = 0; k < batchCount; k++)
                {
                    long aid = SnapshotActorsScratch[start + k].Value.Key;
                    ActorModel am = SnapshotActorsScratch[start + k].Value.Value;
                    Transform tr = ActorTransformRef(am);
                    if (tr == null) continue;
                    w.WriteLong(aid);
                    w.WriteInt((int)am.ident);
                    w.WriteVector3(tr.position);
                    w.WriteQuaternion(tr.rotation);
                    Rigidbody rb = tr.GetComponent<Rigidbody>();
                    w.WriteVector3(rb != null ? rb.velocity : Vector3.zero);
                    w.WriteByte((byte)am.currRegionSetId);
                }
                sendReliable(w);
            }
            SnapshotActorsScratch.Clear();
        }

        public static void ApplySnapshotChunk(ref PacketReader r)
        {
            byte kind = r.ReadByte();
            switch (kind)
            {
                case 1: // TimeWeather
                    double wt = r.ReadDouble();
                    AmbianceDirector.Weather weather = (AmbianceDirector.Weather)r.ReadByte();
                    ApplyTimeWeather(wt, weather, true);
                    break;
                case 2: // LandPlot
                    string plotId = r.ReadString();
                    LandPlot.Id typeId = (LandPlot.Id)r.ReadInt();
                    Identifiable.Id cropId = (Identifiable.Id)r.ReadInt();
                    byte upgCount = r.ReadByte();
                    SnapshotUpgradesScratch.Clear();
                    for (int i = 0; i < upgCount; i++)
                    {
                        SnapshotUpgradesScratch.Add((LandPlot.Upgrade)r.ReadInt());
                    }
                    ApplySnapshotLandPlot(plotId, typeId, SnapshotUpgradesScratch, cropId);
                    SnapshotUpgradesScratch.Clear();
                    byte siloCount = r.ReadByte();
                    for (int si = 0; si < siloCount; si++)
                    {
                        SiloStorage.StorageType storageType = (SiloStorage.StorageType)r.ReadByte();
                        byte slotCount = r.ReadByte();
                        for (int s = 0; s < slotCount; s++)
                        {
                            Identifiable.Id itemId = (Identifiable.Id)r.ReadInt();
                            int cnt = r.ReadInt();
                            uint rev = r.ReadUInt();
                            ApplySnapshotSiloSlot(plotId, storageType, s, itemId, cnt, rev);
                        }
                    }
                    break;
                case 3: // Door
                    ApplyDoorState(r.ReadString(), (AccessDoor.State)r.ReadByte());
                    break;
                case 4: // Gordo
                    ApplyGordoEat(r.ReadString(), r.ReadInt());
                    break;
                case 5: // Oasis
                    ApplyOasisLive(r.ReadString());
                    break;
                case 6: // Pod
                    ApplyPodState(r.ReadString(), (TreasurePod.State)r.ReadByte());
                    break;
                case 7: // Gadget
                    string gadgetSiteId = r.ReadString();
                    Gadget.Id gadgetId = (Gadget.Id)r.ReadInt();
                    float gadgetYaw = r.ReadFloat();
                    ApplyGadgetPlace(gadgetSiteId, gadgetId, gadgetYaw);
                    bool hasSnare = r.ReadBool();
                    if (hasSnare)
                    {
                        Identifiable.Id baitId = (Identifiable.Id)r.ReadInt();
                        Identifiable.Id gordoTypeId = (Identifiable.Id)r.ReadInt();
                        int snareEaten = r.ReadInt();
                        ApplySnareState(gadgetSiteId, baitId, gordoTypeId, snareEaten);
                    }
                    break;
                case 8: // ActorSpawn
                    long aid = r.ReadLong();
                    Identifiable.Id id = (Identifiable.Id)r.ReadInt();
                    Vector3 pos = r.ReadVector3();
                    Quaternion rot = r.ReadQuaternion();
                    Vector3 vel = r.ReadVector3();
                    RegionRegistry.RegionSetId setId = (RegionRegistry.RegionSetId)r.ReadByte();
                    ApplyActorSpawn(aid, id, pos, rot, vel, setId);
                    break;
                case 9: // ExchangeState
                    ApplyExchangeState(ref r);
                    break;
                case 10: // ActorSpawnBatch
                    byte actorCount = r.ReadByte();
                    for (int i = 0; i < actorCount; i++)
                    {
                        long bAid = r.ReadLong();
                        Identifiable.Id bId = (Identifiable.Id)r.ReadInt();
                        Vector3 bPos = r.ReadVector3();
                        Quaternion bRot = r.ReadQuaternion();
                        Vector3 bVel = r.ReadVector3();
                        RegionRegistry.RegionSetId bSetId = (RegionRegistry.RegionSetId)r.ReadByte();
                        try
                        {
                            ApplyActorSpawn(bAid, bId, bPos, bRot, bVel, bSetId);
                        }
                        catch
                        {
                        }
                    }
                    break;
                case 11: // HolidayState
                    byte evCnt = r.ReadByte();
                    List<string> evIds = new List<string>(evCnt);
                    for (int i = 0; i < evCnt; i++)
                    {
                        evIds.Add(r.ReadString());
                    }
                    byte echoCnt = r.ReadByte();
                    List<string> echoIds = new List<string>(echoCnt);
                    for (int i = 0; i < echoCnt; i++)
                    {
                        echoIds.Add(r.ReadString());
                    }
                    ApplyHolidayState(evIds, echoIds);
                    break;
            }
        }

        public static void WriteActiveActorBatch(PacketWriter w, Vector3 refPos, int maxActors)
        {
            w.Reset();
            PendingHostSyncActors.Clear();

            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.GameModel == null)
            {
                w.WriteUShort(0);
                return;
            }

            foreach (KeyValuePair<long, CachedNetActor> kv in NetActors)
            {
                CachedNetActor actor = kv.Value;
                if (actor.Go == null || !actor.Go.activeInHierarchy) continue;
                if ((actor.Tr.position - refPos).sqrMagnitude > 85f * 85f) continue;

                if (actor.Rb != null && actor.Rb.IsSleeping() && actor.Rb.velocity.sqrMagnitude < 0.01f) continue;

                PendingHostSyncActors.Add(new ActiveSyncItem { Id = kv.Key, Actor = actor });
            }

            int total = PendingHostSyncActors.Count;
            if (total == 0)
            {
                w.WriteUShort(0);
                return;
            }

            int sendCount = Math.Min(maxActors, total);
            w.WriteUShort((ushort)sendCount);
            for (int i = 0; i < sendCount; i++)
            {
                int idx = (_actorBatchCursor + i) % total;
                ActiveSyncItem item = PendingHostSyncActors[idx];
                w.WriteLong(item.Id);
                w.WriteVector3(item.Actor.Tr.position);
                w.WriteVector3(item.Actor.Rb != null ? item.Actor.Rb.velocity : Vector3.zero);
            }
            _actorBatchCursor = (_actorBatchCursor + sendCount) % total;
            PendingHostSyncActors.Clear();
        }
    }
}
