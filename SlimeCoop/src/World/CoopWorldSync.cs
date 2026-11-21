using System;
using System.Collections.Generic;
using HarmonyLib;
using MonomiPark.SlimeRancher.DataModel;
using MonomiPark.SlimeRancher.Regions;
using SlimeCoop.Protocol;
using UnityEngine;

namespace SlimeCoop.World
{
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

        private static readonly AccessTools.FieldRef<ActorModel, Transform> ActorTransformRef =
            AccessTools.FieldRefAccess<ActorModel, Transform>("transform");

        private static readonly AccessTools.FieldRef<GordoEat, GordoModel> GordoModelRef =
            AccessTools.FieldRefAccess<GordoEat, GordoModel>("gordoModel");

        private static readonly AccessTools.FieldRef<GordoEat, GordoRewardsBase> GordoRewardsRef =
            AccessTools.FieldRefAccess<GordoEat, GordoRewardsBase>("rewards");

        private static readonly AccessTools.FieldRef<TimeDirector, WorldModel> TimeWorldModelRef =
            AccessTools.FieldRefAccess<TimeDirector, WorldModel>("worldModel");

        private static readonly System.Reflection.MethodInfo GordoReachedTargetMethod =
            AccessTools.Method(typeof(GordoEat), "ReachedTarget");

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

        public static Vector3? PendingActorSpawnVelocity;

        private struct ActiveSyncItem
        {
            public long Id;
            public CachedNetActor Actor;
        }

        private static readonly List<ActiveSyncItem> PendingHostSyncActors = new List<ActiveSyncItem>(128);
        private static int _actorBatchCursor;

        public static void ClearCaches()
        {
            NetActors.Clear();
            ReverseActors.Clear();
            PlotCache.Clear();
            DoorCache.Clear();
            GordoCache.Clear();
            OasisCache.Clear();
            PodCache.Clear();
            GadgetSiteCache.Clear();
            PendingHostSyncActors.Clear();
            _actorBatchCursor = 0;
        }

        public static void ClearLocalActorsBeforeSnapshot()
        {
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

        public static void RebuildSceneObjectCaches()
        {
            PlotCache.Clear();
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

        public static void ApplyTimeWeather(double worldTime, AmbianceDirector.Weather weather)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.TimeDirector == null) return;

            WorldModel wm = TimeWorldModelRef(ctx.TimeDirector);
            if (wm != null)
            {
                bool isLocked = SRSingleton<LockOnDeath>.Instance != null && SRSingleton<LockOnDeath>.Instance.Locked();
                if (!ctx.TimeDirector.IsFastForwarding() && !isLocked && Math.Abs(wm.worldTime - worldTime) > 1.5)
                {
                    wm.worldTime = worldTime;
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
                }
            }
            finally
            {
                NetSyncGuard.SuppressPersonalProgressDepth--;
                NetSyncGuard.ApplyingRemoteDepth--;
            }
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

        public static void ApplySiloSlot(string plotId, int slotIdx, Identifiable.Id itemId, int count)
        {
            LandPlotLocation loc = FindPlot(plotId);
            if (loc == null) return;

            SiloStorage[] silos = loc.GetComponentsInChildren<SiloStorage>(true);
            if (silos == null || silos.Length == 0) return;

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                Ammo ammo = silos[0].GetRelevantAmmo();
                if (ammo == null) return;

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

        public static void ApplyGordoEat(string gordoId, int eatenCount)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx != null && ctx.GameModel != null && ctx.GameModel.AllGordos().TryGetValue(gordoId, out GordoModel gm) && gm != null)
            {
                gm.gordoEatenCount = eatenCount;
            }

            if (!GordoCache.TryGetValue(gordoId, out GordoEat gordo) || gordo == null)
            {
                RebuildSceneObjectCaches();
                GordoCache.TryGetValue(gordoId, out gordo);
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
                    else if (eatenCount >= gordo.GetTargetCount() && gordo.GetTargetCount() > 0 && GordoReachedTargetMethod != null)
                    {
                        var routine = GordoReachedTargetMethod.Invoke(gordo, null) as System.Collections.IEnumerator;
                        if (routine != null) gordo.StartCoroutine(routine);
                    }
                }
                finally
                {
                    NetSyncGuard.SuppressPersonalProgressDepth--;
                    NetSyncGuard.ApplyingRemoteDepth--;
                }
            }
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

        public static void ApplyGadgetPlace(string siteId, Gadget.Id gadgetId, float yaw)
        {
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
            if (def == null || def.prefab == null) return;

            NetSyncGuard.ApplyingRemoteDepth++;
            NetSyncGuard.SuppressPersonalProgressDepth++;
            try
            {
                if (siteModel.HasAttached() && GadgetSiteCache.TryGetValue(siteId, out GadgetSite site) && site != null)
                {
                    site.DestroyAttached();
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

        public static void SendFullSnapshotToClient(Action<PacketWriter> sendReliable)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.GameModel == null || sendReliable == null) return;

            RebuildSceneObjectCaches();
            PacketWriter w = new PacketWriter(1200);

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

                // Silo slots if present
                LandPlotLocation loc = FindPlot(kv.Key);
                if (loc != null)
                {
                    SiloStorage[] silos = loc.GetComponentsInChildren<SiloStorage>(true);
                    if (silos != null && silos.Length > 0 && silos[0].GetRelevantAmmo() != null)
                    {
                        Ammo a = silos[0].GetRelevantAmmo();
                        int sc = Math.Min(12, a.GetUsableSlotCount());
                        w.WriteByte((byte)sc);
                        for (int s = 0; s < sc; s++)
                        {
                            w.WriteInt((int)a.GetSlotName(s));
                            w.WriteInt(a.GetSlotCount(s));
                        }
                    }
                    else
                    {
                        w.WriteByte(0);
                    }
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

            // 4. Gordos
            foreach (KeyValuePair<string, GordoModel> kv in ctx.GameModel.AllGordos())
            {
                if (kv.Value != null && kv.Value.gordoEatenCount != 0)
                {
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
            foreach (KeyValuePair<string, GadgetSiteModel> kv in ctx.GameModel.AllGadgetSites())
            {
                if (kv.Value != null && kv.Value.attached != null)
                {
                    w.Reset();
                    w.WriteByte(7); // Gadget
                    w.WriteString(kv.Key);
                    w.WriteInt((int)kv.Value.attached.ident);
                    w.WriteFloat(kv.Value.attached.transform != null ? kv.Value.attached.transform.eulerAngles.y : 0f);
                    sendReliable(w);
                }
            }

            // 8. Nearby Actors (sorted by distance to host, up to 350 closest so players see all nearby slimes/food)
            Vector3 hostPos = ctx.Player != null ? ctx.Player.transform.position : Vector3.zero;
            List<KeyValuePair<float, KeyValuePair<long, ActorModel>>> nearby = new List<KeyValuePair<float, KeyValuePair<long, ActorModel>>>(512);
            foreach (KeyValuePair<long, ActorModel> kv in ctx.GameModel.AllActors())
            {
                ActorModel am = kv.Value;
                if (am == null || am.ident == Identifiable.Id.NONE || am.ident == Identifiable.Id.PLAYER) continue;
                Transform tr = ActorTransformRef(am);
                if (tr == null) continue;
                float sqr = (tr.position - hostPos).sqrMagnitude;
                if (sqr > 150f * 150f) continue;
                nearby.Add(new KeyValuePair<float, KeyValuePair<long, ActorModel>>(sqr, kv));
            }

            nearby.Sort((a, b) => a.Key.CompareTo(b.Key));
            int limit = Math.Min(350, nearby.Count);
            for (int i = 0; i < limit; i++)
            {
                long aid = nearby[i].Value.Key;
                ActorModel am = nearby[i].Value.Value;
                Transform tr = ActorTransformRef(am);
                if (tr == null) continue;

                w.Reset();
                w.WriteByte(8); // ActorSpawn
                w.WriteLong(aid);
                w.WriteInt((int)am.ident);
                w.WriteVector3(tr.position);
                w.WriteQuaternion(tr.rotation);
                Rigidbody rb = tr.GetComponent<Rigidbody>();
                w.WriteVector3(rb != null ? rb.velocity : Vector3.zero);
                w.WriteByte((byte)am.currRegionSetId);
                sendReliable(w);
            }
        }

        public static void ApplySnapshotChunk(ref PacketReader r)
        {
            byte kind = r.ReadByte();
            switch (kind)
            {
                case 1: // TimeWeather
                    double wt = r.ReadDouble();
                    AmbianceDirector.Weather weather = (AmbianceDirector.Weather)r.ReadByte();
                    ApplyTimeWeather(wt, weather);
                    break;
                case 2: // LandPlot
                    string plotId = r.ReadString();
                    LandPlot.Id typeId = (LandPlot.Id)r.ReadInt();
                    Identifiable.Id cropId = (Identifiable.Id)r.ReadInt();
                    byte upgCount = r.ReadByte();
                    ApplyPlotBuild(plotId, typeId);
                    for (int i = 0; i < upgCount; i++)
                    {
                        ApplyPlotUpgrade(plotId, (LandPlot.Upgrade)r.ReadInt());
                    }
                    if (cropId != Identifiable.Id.NONE)
                    {
                        ApplyPlotPlant(plotId, cropId);
                    }
                    byte slotCount = r.ReadByte();
                    for (int s = 0; s < slotCount; s++)
                    {
                        Identifiable.Id itemId = (Identifiable.Id)r.ReadInt();
                        int cnt = r.ReadInt();
                        ApplySiloSlot(plotId, s, itemId, cnt);
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
                    ApplyGadgetPlace(r.ReadString(), (Gadget.Id)r.ReadInt(), r.ReadFloat());
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
