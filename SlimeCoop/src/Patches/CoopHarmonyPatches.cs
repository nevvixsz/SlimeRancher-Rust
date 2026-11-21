using System;
using System.Reflection;
using HarmonyLib;
using MonomiPark.SlimeRancher.DataModel;
using MonomiPark.SlimeRancher.Regions;
using SlimeCoop.Protocol;
using SlimeCoop.World;
using UnityEngine;

namespace SlimeCoop.Patches
{
    internal static class CoopHarmonyPatches
    {
        private static readonly PacketWriter SyncWriter = new PacketWriter(512);
        private static bool _clientExpelling;
        private static GameObject _lastExpelledObj;

        public static void Install(Harmony harmony)
        {
            if (harmony == null) return;

            MethodInfo suppressPrefix = AccessTools.Method(typeof(CoopHarmonyPatches), nameof(SuppressPersonalProgressPrefix));
            foreach (MethodInfo m in typeof(AchievementsDirector).GetMethods(BindingFlags.Public | BindingFlags.Instance))
            {
                if (m.Name == nameof(AchievementsDirector.AddToStat) ||
                    m.Name == nameof(AchievementsDirector.MaybeUpdateMaxStat) ||
                    m.Name == nameof(AchievementsDirector.SetStat) ||
                    m.Name == nameof(AchievementsDirector.CheckAchievement))
                {
                    harmony.Patch(m, prefix: new HarmonyMethod(suppressPrefix));
                }
            }
        }

        private static bool SuppressPersonalProgressPrefix()
        {
            return !NetSyncGuard.SuppressPersonalProgress;
        }

        internal static void NotifyExpelledCreated(GameObject go)
        {
            if (_clientExpelling)
            {
                _lastExpelledObj = go;
            }
        }

        // --- Personal Slimepedia Isolation ---
        [HarmonyPatch(typeof(PediaDirector), nameof(PediaDirector.Unlock))]
        internal static class PediaUnlockPatch
        {
            private static bool Prefix()
            {
                return !NetSyncGuard.SuppressPersonalProgress;
            }
        }

        // --- Prevent connected client from overwriting singleplayer save ---
        [HarmonyPatch(typeof(AutoSaveDirector), "SaveGame")]
        internal static class ClientSaveGuardPatch
        {
            private static bool Prefix()
            {
                if (CoopSession.IsClient && !string.IsNullOrEmpty(CoopSession.HostServerId))
                {
                    ClientProfileStore.SaveCurrentClientProfile(CoopSession.HostServerId);
                    return false;
                }
                return true;
            }
        }

        // --- Disable wild spawners on clients (host is authoritative for world spawns) ---
        [HarmonyPatch(typeof(DirectedActorSpawner), nameof(DirectedActorSpawner.CanSpawn))]
        internal static class ClientDisableDirectedSpawnerPatch
        {
            private static bool Prefix(ref bool __result)
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote)
                {
                    __result = false;
                    return false;
                }
                return true;
            }
        }

        [HarmonyPatch(typeof(SpawnResource), nameof(SpawnResource.Update))]
        internal static class ClientDisableResourceSpawnerPatch
        {
            private static bool Prefix()
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote)
                {
                    return false;
                }
                return true;
            }
        }

        // --- Shared Ranch: Plot Build / Replace ---
        [HarmonyPatch(typeof(LandPlotLocation), nameof(LandPlotLocation.Replace))]
        internal static class PlotReplaceSyncPatch
        {
            private static void Postfix(LandPlotLocation __instance, GameObject __result)
            {
                if (!CoopSession.IsActive || NetSyncGuard.IsApplyingRemote || __instance == null || __result == null) return;
                LandPlot lp = __result.GetComponentInChildren<LandPlot>();
                if (lp == null) return;

                SyncWriter.Reset();
                SyncWriter.WriteString(__instance.id);
                SyncWriter.WriteInt((int)lp.typeId);

                if (CoopSession.IsHost) CoopSession.BroadcastReliable(PacketType.PlotBuild, SyncWriter);
                else CoopSession.SendToHostReliable(PacketType.PlotBuild, SyncWriter);
            }
        }

        // --- Shared Ranch: Plot Upgrade ---
        [HarmonyPatch(typeof(LandPlot), nameof(LandPlot.AddUpgrade))]
        internal static class PlotUpgradeSyncPatch
        {
            private static void Postfix(LandPlot __instance, LandPlot.Upgrade upgrade)
            {
                if (!CoopSession.IsActive || NetSyncGuard.IsApplyingRemote || __instance == null) return;
                LandPlotLocation loc = __instance.GetComponentInParent<LandPlotLocation>();
                if (loc == null) return;

                SyncWriter.Reset();
                SyncWriter.WriteString(loc.id);
                SyncWriter.WriteInt((int)upgrade);

                if (CoopSession.IsHost) CoopSession.BroadcastReliable(PacketType.PlotUpgrade, SyncWriter);
                else CoopSession.SendToHostReliable(PacketType.PlotUpgrade, SyncWriter);
            }
        }

        // --- Shared Ranch: Garden Plant ---
        [HarmonyPatch(typeof(GardenCatcher), nameof(GardenCatcher.Plant))]
        internal static class GardenPlantSyncPatch
        {
            private static void Postfix(LandPlot ___activator, Identifiable.Id cropId)
            {
                if (!CoopSession.IsActive || NetSyncGuard.IsApplyingRemote || ___activator == null) return;
                LandPlotLocation loc = ___activator.GetComponentInParent<LandPlotLocation>();
                if (loc == null) return;

                SyncWriter.Reset();
                SyncWriter.WriteString(loc.id);
                SyncWriter.WriteInt((int)cropId);

                if (CoopSession.IsHost) CoopSession.BroadcastReliable(PacketType.PlotPlant, SyncWriter);
                else CoopSession.SendToHostReliable(PacketType.PlotPlant, SyncWriter);
            }
        }

        // --- Shared Ranch: Silo Storage ---
        private static void SyncSiloSlot(SiloStorage silo, int slotIdx)
        {
            if (!CoopSession.IsActive || NetSyncGuard.IsApplyingRemote || silo == null) return;
            LandPlotLocation loc = silo.GetComponentInParent<LandPlotLocation>();
            Ammo ammo = silo.GetRelevantAmmo();
            if (loc == null || ammo == null || slotIdx < 0 || slotIdx >= ammo.GetUsableSlotCount()) return;

            SyncWriter.Reset();
            SyncWriter.WriteString(loc.id);
            SyncWriter.WriteByte((byte)slotIdx);
            SyncWriter.WriteInt((int)ammo.GetSlotName(slotIdx));
            SyncWriter.WriteInt(ammo.GetSlotCount(slotIdx));

            if (CoopSession.IsHost) CoopSession.BroadcastReliable(PacketType.SiloSlotSync, SyncWriter);
            else CoopSession.SendToHostReliable(PacketType.SiloSlotSync, SyncWriter);
        }

        [HarmonyPatch(typeof(SiloStorage), nameof(SiloStorage.MaybeAddIdentifiable), new[] { typeof(Identifiable.Id), typeof(int), typeof(int), typeof(bool) })]
        internal static class SiloAddSyncPatch
        {
            private static void Postfix(SiloStorage __instance, int slotIdx, bool __result)
            {
                if (__result) SyncSiloSlot(__instance, slotIdx);
            }
        }

        [HarmonyPatch(typeof(SiloStorage), nameof(SiloStorage.MaybeAddIdentifiable), new[] { typeof(Identifiable.Id) })]
        internal static class SiloAddAnySyncPatch
        {
            private static void Postfix(SiloStorage __instance, bool __result)
            {
                if (!__result || !CoopSession.IsActive || NetSyncGuard.IsApplyingRemote || __instance == null) return;
                Ammo ammo = __instance.GetRelevantAmmo();
                if (ammo == null) return;
                int count = ammo.GetUsableSlotCount();
                for (int i = 0; i < count; i++)
                {
                    SyncSiloSlot(__instance, i);
                }
            }
        }

        // --- Shared World: Doors ---
        [HarmonyPatch(typeof(AccessDoor), nameof(AccessDoor.CurrState), MethodType.Setter)]
        internal static class DoorSyncPatch
        {
            private static void Postfix(AccessDoor __instance, AccessDoor.State value)
            {
                if (!CoopSession.IsActive || NetSyncGuard.IsApplyingRemote || __instance == null || string.IsNullOrEmpty(__instance.id)) return;

                SyncWriter.Reset();
                SyncWriter.WriteString(__instance.id);
                SyncWriter.WriteByte((byte)value);

                if (CoopSession.IsHost) CoopSession.BroadcastReliable(PacketType.DoorState, SyncWriter);
                else CoopSession.SendToHostReliable(PacketType.DoorState, SyncWriter);
            }
        }

        // --- Shared World: Gordos ---
        [HarmonyPatch(typeof(GordoEat), "SetEatenCount")]
        internal static class GordoEatSyncPatch
        {
            private static void Postfix(GordoEat __instance, int eatenCount)
            {
                if (!CoopSession.IsActive || NetSyncGuard.IsApplyingRemote || __instance == null || string.IsNullOrEmpty(__instance.id)) return;

                SyncWriter.Reset();
                SyncWriter.WriteString(__instance.id);
                SyncWriter.WriteInt(eatenCount);

                if (CoopSession.IsHost) CoopSession.BroadcastReliable(PacketType.GordoEatSync, SyncWriter);
                else CoopSession.SendToHostReliable(PacketType.GordoEatSync, SyncWriter);
            }
        }

        // --- Shared World: Oases ---
        [HarmonyPatch(typeof(Oasis), nameof(Oasis.SetLive))]
        internal static class OasisSyncPatch
        {
            private static void Postfix(Oasis __instance)
            {
                if (!CoopSession.IsActive || NetSyncGuard.IsApplyingRemote || __instance == null || string.IsNullOrEmpty(__instance.id)) return;

                SyncWriter.Reset();
                SyncWriter.WriteString(__instance.id);

                if (CoopSession.IsHost) CoopSession.BroadcastReliable(PacketType.OasisSync, SyncWriter);
                else CoopSession.SendToHostReliable(PacketType.OasisSync, SyncWriter);
            }
        }

        // --- Shared World: Treasure Pods ---
        [HarmonyPatch(typeof(TreasurePod), nameof(TreasurePod.CurrState), MethodType.Setter)]
        internal static class PodSyncPatch
        {
            private static void Postfix(TreasurePod __instance, TreasurePod.State value)
            {
                if (!CoopSession.IsActive || NetSyncGuard.IsApplyingRemote || __instance == null || string.IsNullOrEmpty(__instance.id)) return;

                SyncWriter.Reset();
                SyncWriter.WriteString(__instance.id);
                SyncWriter.WriteByte((byte)value);

                if (CoopSession.IsHost) CoopSession.BroadcastReliable(PacketType.PodSync, SyncWriter);
                else CoopSession.SendToHostReliable(PacketType.PodSync, SyncWriter);
            }
        }

        // --- Shared World: Gadgets ---
        [HarmonyPatch(typeof(GameModel), nameof(GameModel.InstantiateGadget), new[] { typeof(GameObject), typeof(GadgetSiteModel) })]
        internal static class GadgetPlaceSyncPatch
        {
            private static void Postfix(GadgetSiteModel site, GameObject __result)
            {
                if (!CoopSession.IsActive || NetSyncGuard.IsApplyingRemote || site == null || __result == null) return;
                Gadget g = __result.GetComponent<Gadget>();
                if (g == null) return;

                SyncWriter.Reset();
                SyncWriter.WriteString(site.id);
                SyncWriter.WriteInt((int)g.id);
                SyncWriter.WriteFloat(__result.transform.eulerAngles.y);

                if (CoopSession.IsHost) CoopSession.BroadcastReliable(PacketType.GadgetPlace, SyncWriter);
                else CoopSession.SendToHostReliable(PacketType.GadgetPlace, SyncWriter);
            }
        }

        [HarmonyPatch(typeof(GadgetSite), nameof(GadgetSite.DestroyAttached))]
        internal static class GadgetDestroySyncPatch
        {
            private static void Prefix(GadgetSite __instance)
            {
                if (!CoopSession.IsActive || NetSyncGuard.IsApplyingRemote || __instance == null || string.IsNullOrEmpty(__instance.id)) return;

                SyncWriter.Reset();
                SyncWriter.WriteString(__instance.id);

                if (CoopSession.IsHost) CoopSession.BroadcastReliable(PacketType.GadgetDestroy, SyncWriter);
                else CoopSession.SendToHostReliable(PacketType.GadgetDestroy, SyncWriter);
            }
        }

        // --- Actors Spawn & Destroy ---
        [HarmonyPatch]
        internal static class ActorInstantiateSyncPatch
        {
            private static MethodBase TargetMethod()
            {
                return AccessTools.Method(
                    typeof(GameModel),
                    "InstantiateActor",
                    new[]
                    {
                        typeof(GameObject), typeof(RegionRegistry.RegionSetId),
                        typeof(Vector3), typeof(Quaternion), typeof(bool)
                    });
            }

            private static void Postfix(
                RegionRegistry.RegionSetId regionSetId,
                Vector3 pos,
                Quaternion rot,
                GameObject __result)
            {
                if (__result == null) return;

                if (_clientExpelling)
                {
                    NotifyExpelledCreated(__result);
                    return;
                }

                if (!CoopSession.IsHost || NetSyncGuard.IsApplyingRemote) return;

                if (!CoopWorldSync.TryGetActorId(__result, out long actorId) || actorId <= 0) return;

                Identifiable ident = __result.GetComponent<Identifiable>();
                if (ident == null || ident.id == Identifiable.Id.NONE || ident.id == Identifiable.Id.PLAYER) return;

                if (CoopSession.ConnectedPeers.Count == 0) return;

                Vector3 vel;
                if (CoopWorldSync.PendingActorSpawnVelocity.HasValue)
                {
                    vel = CoopWorldSync.PendingActorSpawnVelocity.Value;
                }
                else
                {
                    Rigidbody rb = __result.GetComponent<Rigidbody>();
                    vel = rb != null ? rb.velocity : Vector3.zero;
                }

                SyncWriter.Reset();
                SyncWriter.WriteLong(actorId);
                SyncWriter.WriteInt((int)ident.id);
                SyncWriter.WriteVector3(pos);
                SyncWriter.WriteQuaternion(rot);
                SyncWriter.WriteVector3(vel);
                SyncWriter.WriteByte((byte)regionSetId);
                CoopSession.BroadcastReliable(PacketType.ActorSpawn, SyncWriter);
            }
        }

        [HarmonyPatch(typeof(GameModel), nameof(GameModel.DestroyActorModel))]
        internal static class ActorDestroySyncPatch
        {
            private static void Prefix(GameObject gameObj)
            {
                if (gameObj == null) return;
                long actorId = 0;
                if (CoopSession.IsHost && !NetSyncGuard.IsApplyingRemote && CoopSession.ConnectedPeers.Count > 0)
                {
                    CoopWorldSync.TryGetActorId(gameObj, out actorId);
                }
                CoopWorldSync.UnregisterActor(gameObj);
                if (actorId <= 0) return;

                SyncWriter.Reset();
                SyncWriter.WriteLong(actorId);
                CoopSession.BroadcastReliable(PacketType.ActorDestroy, SyncWriter);
            }
        }

        // --- Client Vacuum Consume & Expel ---
        [HarmonyPatch(typeof(Vacuumable), nameof(Vacuumable.TryConsume))]
        internal static class ClientVacConsumePatch
        {
            private static void Prefix(Vacuumable __instance, out long __state)
            {
                __state = 0;
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote && __instance != null)
                {
                    CoopWorldSync.TryGetActorId(__instance.gameObject, out __state);
                }
            }

            private static void Postfix(Vacuumable __instance, bool __result, long __state)
            {
                if (__result && CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote)
                {
                    if (__instance != null)
                    {
                        CoopWorldSync.UnregisterActor(__instance.gameObject);
                    }
                    if (__state > 0)
                    {
                        SyncWriter.Reset();
                        SyncWriter.WriteLong(__state);
                        CoopSession.SendToHostReliable(PacketType.ClientVacCapture, SyncWriter);
                    }
                }
            }
        }

        [HarmonyPatch(typeof(WeaponVacuum), nameof(WeaponVacuum.Expel), new[] { typeof(GameObject), typeof(bool) })]
        internal static class ClientVacExpelPatch
        {
            private static void Prefix()
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote)
                {
                    _clientExpelling = true;
                    _lastExpelledObj = null;
                }
            }

            private static void Postfix(WeaponVacuum __instance, GameObject toExpel)
            {
                if (!_clientExpelling) return;
                _clientExpelling = false;

                GameObject tempObj = _lastExpelledObj;
                _lastExpelledObj = null;

                Identifiable.Id identId = Identifiable.Id.NONE;
                Vector3 pos = Vector3.zero;
                Vector3 vel = Vector3.zero;
                RegionRegistry.RegionSetId setId = RegionRegistry.RegionSetId.HOME;

                if (tempObj != null)
                {
                    Identifiable ident = tempObj.GetComponent<Identifiable>();
                    if (ident != null) identId = ident.id;
                    pos = tempObj.transform.position;
                    Rigidbody rb = tempObj.GetComponent<Rigidbody>();
                    vel = rb != null ? rb.velocity : Vector3.zero;
                    RegionMember rm = tempObj.GetComponent<RegionMember>();
                    if (rm != null && rm.setId != RegionRegistry.RegionSetId.UNSET) setId = rm.setId;
                }
                else if (toExpel != null && __instance != null && __instance.vacOrigin != null)
                {
                    Identifiable ident = toExpel.GetComponent<Identifiable>();
                    if (ident != null) identId = ident.id;
                    Transform orig = __instance.vacOrigin.transform;
                    pos = orig.position + orig.up * 0.8f;
                    vel = orig.up * __instance.ejectSpeed;
                    RegionMember rm = __instance.GetComponentInParent<RegionMember>();
                    if (rm != null && rm.setId != RegionRegistry.RegionSetId.UNSET) setId = rm.setId;
                }

                if (identId != Identifiable.Id.NONE && identId != Identifiable.Id.PLAYER)
                {
                    SyncWriter.Reset();
                    SyncWriter.WriteInt((int)identId);
                    SyncWriter.WriteVector3(pos);
                    SyncWriter.WriteVector3(vel);
                    SyncWriter.WriteByte((byte)setId);
                    CoopSession.SendToHostReliable(PacketType.ClientExpelRequest, SyncWriter);
                }

                if (tempObj != null)
                {
                    NetSyncGuard.ApplyingRemoteDepth++;
                    try
                    {
                        Destroyer.DestroyActor(tempObj, "SlimeCoop.ClientExpelProxy", true);
                    }
                    finally
                    {
                        NetSyncGuard.ApplyingRemoteDepth--;
                    }
                }
            }
        }

        // --- Shared Market Income Notice ---
        [HarmonyPatch(typeof(ScorePlort), nameof(ScorePlort.Deposit))]
        internal static class MarketDepositSyncPatch
        {
            private static void Postfix(ScorePlort.Deposit_Response __result)
            {
                if (!CoopSession.IsHost || !CoopSession.ShareMarketIncome || CoopSession.ConnectedPeers.Count == 0) return;
                if (__result == null || __result.currency <= 0) return;

                SyncWriter.Reset();
                SyncWriter.WriteInt(__result.currency);
                SyncWriter.WriteString(CoopSession.LocalNick);
                CoopSession.BroadcastReliable(PacketType.MarketDepositNotice, SyncWriter);
            }
        }
    }
}
