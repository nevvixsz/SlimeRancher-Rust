using System;
using System.Collections;
using System.Collections.Generic;
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
        private static readonly List<ExchangeDirector.NonIdentReward> CrateSpecScratch = new List<ExchangeDirector.NonIdentReward>(16);
        private static bool _clientExpelling;
        private static GameObject _lastExpelledObj;
        private static float _nextExchangeActionTime;
        private static long _pendingExchangeActorId;

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
                if (CoopSession.IsClient)
                {
                    if (CoopSession.IsClientProfileLoaded && !string.IsNullOrEmpty(CoopSession.HostServerId))
                    {
                        ClientProfileStore.SaveCurrentClientProfile(CoopSession.HostServerId);
                    }
                    return false;
                }
                if (CoopSession.BlockLocalSaveUntilMenu)
                {
                    return false;
                }
                return true;
            }
        }

        [HarmonyPatch(typeof(AutoSaveDirector), nameof(AutoSaveDirector.BeginLoad))]
        internal static class ResetBlockOnBeginLoadPatch
        {
            private static void Prefix()
            {
                CoopSession.BlockLocalSaveUntilMenu = false;
            }
        }

        [HarmonyPatch(typeof(AutoSaveDirector), nameof(AutoSaveDirector.LoadNewGame))]
        internal static class ResetBlockOnLoadNewGamePatch
        {
            private static void Prefix()
            {
                CoopSession.BlockLocalSaveUntilMenu = false;
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

        // --- Host-authoritative Slime Eating, Tarr Transformation & Actor Lifetime on Clients ---
        [HarmonyPatch(typeof(SlimeEat), nameof(SlimeEat.MaybeChomp))]
        internal static class ClientDisableSlimeChompPatch
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

        [HarmonyPatch(typeof(SlimeEat), nameof(SlimeEat.MaybeSpinAndChomp))]
        internal static class ClientDisableSlimeSpinChompPatch
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

        [HarmonyPatch(typeof(DestroyAfterTime), nameof(DestroyAfterTime.RegistryUpdate))]
        internal static class ClientDisableDestroyAfterTimePatch
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

        [HarmonyPatch(typeof(DestroyPlortAfterTime), nameof(DestroyPlortAfterTime.RegistryUpdate))]
        internal static class ClientDisableDestroyPlortAfterTimePatch
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

        [HarmonyPatch(typeof(DestroyOutsideHoursOfDay), nameof(DestroyOutsideHoursOfDay.Update))]
        internal static class ClientDisableDestroyOutsideHoursPatch
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

        [HarmonyPatch(typeof(MaybeCullOnReenable), "DoCullCheck")]
        internal static class ClientDisableCullOnReenablePatch
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

        [HarmonyPatch(typeof(Destroyer), nameof(Destroyer.DestroyActor), new[] { typeof(GameObject), typeof(string), typeof(bool) })]
        internal static class ClientProtectNetActorDestroyPatch
        {
            private static bool Prefix(GameObject actorObj, string source)
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote &&
                    source != "Vacuumable.consume" &&
                    actorObj != null &&
                    CoopWorldSync.TryGetActorId(actorObj, out long aid) && aid > 0)
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
                if (!CoopSession.IsActive || __instance == null) return;
                CoopWorldSync.InvalidatePlotSilos(__instance.id);
                CoopWorldSync.ResetPlotSiloTracking(__instance.id);
                if (NetSyncGuard.IsApplyingRemote || __result == null) return;
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
                if (!CoopSession.IsActive || __instance == null) return;
                LandPlotLocation loc = __instance.GetComponentInParent<LandPlotLocation>();
                if (loc == null) return;
                CoopWorldSync.InvalidatePlotSilos(loc.id);
                if (NetSyncGuard.IsApplyingRemote) return;

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

        // --- Shared Ranch: Silo Storage (Host-Authoritative + Versioned) ---
        [HarmonyPatch(typeof(SiloStorage), nameof(SiloStorage.MaybeAddIdentifiable), new[] { typeof(Identifiable.Id), typeof(int), typeof(int), typeof(bool) })]
        internal static class SiloAddSyncPatch
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

            private static void Postfix(SiloStorage __instance, int slotIdx, bool __result)
            {
                if (!__result || !CoopSession.IsHost || NetSyncGuard.IsApplyingRemote || __instance == null) return;
                CoopWorldSync.PushHostSiloSlot(__instance, slotIdx, false);
            }
        }

        [HarmonyPatch(typeof(SiloStorage), nameof(SiloStorage.MaybeAddIdentifiable), new[] { typeof(Identifiable.Id) })]
        internal static class SiloAddAnySyncPatch
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

            private static void Postfix(SiloStorage __instance, bool __result)
            {
                if (!__result || !CoopSession.IsHost || NetSyncGuard.IsApplyingRemote || __instance == null) return;
                Ammo ammo = __instance.GetRelevantAmmo();
                if (ammo == null) return;
                int count = ammo.GetUsableSlotCount();
                for (int i = 0; i < count; i++)
                {
                    CoopWorldSync.PushHostSiloSlot(__instance, i, false);
                }
            }
        }

        [HarmonyPatch(typeof(SiloCatcher), "Insert")]
        internal static class SiloCatcherInsertClientBlockPatch
        {
            private static bool Prefix(SiloCatcher __instance, ref bool __result)
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote && __instance != null &&
                    (__instance.type == SiloCatcher.Type.SILO_DEFAULT || __instance.type == SiloCatcher.Type.SILO_OUTPUT_ONLY))
                {
                    __result = false;
                    return false;
                }
                return true;
            }
        }

        [HarmonyPatch(typeof(SiloCatcher), "Remove")]
        internal static class SiloRemoveSyncPatch
        {
            private static bool Prefix(
                SiloCatcher __instance,
                SiloStorage ___storageSilo,
                ref float ___nextEject,
                VacAccelerationHelper ___accelerationOutput,
                ref Identifiable.Id id,
                ref bool __result)
            {
                if (!CoopSession.IsClient || NetSyncGuard.IsApplyingRemote || __instance == null)
                {
                    return true;
                }

                if (__instance.type != SiloCatcher.Type.SILO_DEFAULT &&
                    __instance.type != SiloCatcher.Type.SILO_OUTPUT_ONLY)
                {
                    return true;
                }

                id = Identifiable.Id.NONE;
                __result = false;
                if (___storageSilo == null || Time.time < ___nextEject)
                {
                    return false;
                }

                Ammo ammo = ___storageSilo.GetRelevantAmmo();
                if (ammo == null) return false;
                int slotIdx = __instance.slotIdx;
                if (slotIdx < 0 || slotIdx >= ammo.GetUsableSlotCount()) return false;

                int slotCount = ammo.GetSlotCount(slotIdx);
                Identifiable.Id slotItem = slotCount > 0 ? ammo.GetSlotName(slotIdx) : Identifiable.Id.NONE;
                if (slotCount <= 0 || slotItem == Identifiable.Id.NONE) return false;

                SceneContext ctx = SRSingleton<SceneContext>.Instance;
                if (ctx == null || ctx.PlayerState == null || ctx.PlayerState.Ammo == null) return false;
                if (!ctx.PlayerState.Ammo.CouldAddToSlot(slotItem))
                {
                    ___nextEject = Time.time + 0.25f;
                    return false;
                }

                float factor = ___accelerationOutput != null ? Math.Max(0.1f, ___accelerationOutput.Factor) : 1f;
                ___nextEject = Time.time + 0.25f / factor;
                if (___accelerationOutput != null)
                {
                    ___accelerationOutput.OnTriggered();
                }

                CoopWorldSync.SendClientSiloTake(___storageSilo, slotIdx, slotItem);
                return false;
            }

            private static void Postfix(SiloCatcher __instance, SiloStorage ___storageSilo, bool __result)
            {
                if (!__result || !CoopSession.IsHost || NetSyncGuard.IsApplyingRemote || __instance == null || ___storageSilo == null) return;
                if (__instance.type == SiloCatcher.Type.SILO_DEFAULT || __instance.type == SiloCatcher.Type.SILO_OUTPUT_ONLY)
                {
                    CoopWorldSync.PushHostSiloSlot(___storageSilo, __instance.slotIdx, false);
                }
            }
        }

        [HarmonyPatch(typeof(PlortCollector), "DoCollection")]
        internal static class PlortCollectorDoCollectionPatch
        {
            private static bool Prefix()
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote) return false;
                return true;
            }
        }

        [HarmonyPatch(typeof(PlortCollector), nameof(PlortCollector.FastForward))]
        internal static class PlortCollectorFastForwardPatch
        {
            private static bool Prefix()
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote) return false;
                return true;
            }
        }

        [HarmonyPatch(typeof(PlortCollectorActivator), nameof(PlortCollectorActivator.Activate))]
        internal static class PlortCollectorActivatePatch
        {
            private static void Prefix(PlortCollectorActivator __instance, float ___nextAllowedActivationTime)
            {
                if (!CoopSession.IsClient || NetSyncGuard.IsApplyingRemote || __instance == null) return;
                if (___nextAllowedActivationTime > Time.time) return;

                LandPlotLocation loc = __instance.GetComponentInParent<LandPlotLocation>();
                if (loc == null || string.IsNullOrEmpty(loc.id)) return;

                SyncWriter.Reset();
                SyncWriter.WriteString(loc.id);
                CoopSession.SendToHostReliable(PacketType.PlotCollectorActivate, SyncWriter);
            }
        }

        [HarmonyPatch(typeof(SlimeFeeder), "ProcessFeedOperation")]
        internal static class SlimeFeederSyncPatch
        {
            private static bool Prefix()
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote) return false;
                return true;
            }

            private static void Postfix(SiloStorage ___storage)
            {
                if (CoopSession.IsHost && !NetSyncGuard.IsApplyingRemote && ___storage != null)
                {
                    CoopWorldSync.PushHostSiloSlot(___storage, 0, false);
                }
            }
        }

        // --- Shared World: Range Exchange Quests ---
        private static void BroadcastHostExchangeState()
        {
            if (!CoopSession.IsHost || NetSyncGuard.IsApplyingRemote || CoopSession.ConnectedPeers.Count == 0) return;
            SyncWriter.Reset();
            if (CoopWorldSync.WriteExchangeState(SyncWriter))
            {
                CoopSession.BroadcastReliable(PacketType.ExchangeStateSync, SyncWriter);
            }
        }

        [HarmonyPatch(typeof(ExchangeDirector), nameof(ExchangeDirector.Update))]
        internal static class ExchangeDirectorUpdatePatch
        {
            private static bool Prefix()
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote) return false;
                return true;
            }
        }

        [HarmonyPatch(typeof(ExchangeDirector), "OfferDidChange")]
        internal static class ExchangeOfferDidChangePatch
        {
            private static void Postfix()
            {
                BroadcastHostExchangeState();
            }
        }

        [HarmonyPatch(typeof(ExchangeDirector), nameof(ExchangeDirector.SelectDailyOffer))]
        internal static class ExchangeSelectDailyOfferPatch
        {
            private static bool Prefix(string rancherId, bool isFirstOffer, ref bool __result)
            {
                if (!CoopSession.IsClient || NetSyncGuard.IsApplyingRemote) return true;
                if (Time.unscaledTime >= _nextExchangeActionTime)
                {
                    _nextExchangeActionTime = Time.unscaledTime + 0.4f;
                    SyncWriter.Reset();
                    SyncWriter.WriteByte(0); // SelectDailyOffer
                    SyncWriter.WriteString(rancherId ?? string.Empty);
                    SyncWriter.WriteBool(isFirstOffer);
                    CoopSession.SendToHostReliable(PacketType.ExchangeAction, SyncWriter);
                }
                __result = false;
                return false;
            }
        }

        [HarmonyPatch(typeof(ExchangeDirector), nameof(ExchangeDirector.MaybeStartNext))]
        internal static class ExchangeMaybeStartNextPatch
        {
            private static bool Prefix(ExchangeDirector __instance, ExchangeDirector.OfferType offerType, ref bool __result)
            {
                if (!CoopSession.IsClient || NetSyncGuard.IsApplyingRemote) return true;
                if (__instance != null && __instance.GetOfferId(offerType) != null)
                {
                    __result = false;
                    return false;
                }
                if (Time.unscaledTime >= _nextExchangeActionTime)
                {
                    _nextExchangeActionTime = Time.unscaledTime + 0.4f;
                    SyncWriter.Reset();
                    SyncWriter.WriteByte(1); // MaybeStartNext
                    SyncWriter.WriteByte((byte)offerType);
                    CoopSession.SendToHostReliable(PacketType.ExchangeAction, SyncWriter);
                }
                __result = true; // wait for host ExchangeStateSync + ExchangeOpenChat instead of falling through to stale chat
                return false;
            }
        }

        [HarmonyPatch(typeof(ExchangeDirector), nameof(ExchangeDirector.CreateRancherChatUI))]
        internal static class ExchangeSuppressHostChatPatch
        {
            private static bool Prefix(ref bool __result)
            {
                if (CoopWorldSync.SuppressLocalRancherChatUI)
                {
                    __result = true;
                    return false;
                }
                return true;
            }
        }

        [HarmonyPatch(typeof(ExchangeAcceptor), nameof(ExchangeAcceptor.OnTriggerEnter))]
        internal static class ExchangeAcceptorEnterPatch
        {
            private static void Prefix(Collider col)
            {
                _pendingExchangeActorId = 0;
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote && col != null && !col.isTrigger)
                {
                    CoopWorldSync.TryGetActorId(col.gameObject, out _pendingExchangeActorId);
                }
            }

            private static void Postfix()
            {
                _pendingExchangeActorId = 0;
            }
        }

        [HarmonyPatch(typeof(ExchangeDirector), nameof(ExchangeDirector.TryAccept))]
        internal static class ExchangeTryAcceptPatch
        {
            private static void Postfix(ExchangeDirector.OfferType type, Identifiable.Id id, bool __result)
            {
                if (!__result || !CoopSession.IsClient || NetSyncGuard.IsApplyingRemote) return;
                SyncWriter.Reset();
                SyncWriter.WriteByte(2); // TryAccept
                SyncWriter.WriteByte((byte)type);
                SyncWriter.WriteInt((int)id);
                SyncWriter.WriteLong(_pendingExchangeActorId);
                CoopSession.SendToHostReliable(PacketType.ExchangeAction, SyncWriter);
                _pendingExchangeActorId = 0;
            }
        }

        [HarmonyPatch(typeof(ExchangeEjector), nameof(ExchangeEjector.AwardIfType))]
        internal static class ExchangeEjectorClientBlockPatch
        {
            private static bool Prefix()
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote) return false;
                return true;
            }
        }

        [HarmonyPatch(typeof(DirectRewardEjector), nameof(DirectRewardEjector.AwardIfType))]
        internal static class DirectRewardEjectorClientBlockPatch
        {
            private static bool Prefix()
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote) return false;
                return true;
            }
        }

        [HarmonyPatch(typeof(RancherProgressAwarder), nameof(RancherProgressAwarder.AwardIfType))]
        internal static class RancherProgressAwarderClientBlockPatch
        {
            private static bool Prefix()
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote) return false;
                return true;
            }
        }

        [HarmonyPatch(typeof(ExchangeBreakOnImpact), "BreakOpen")]
        internal static class ExchangeCrateBreakPatch
        {
            private static bool Prefix(ExchangeBreakOnImpact __instance, bool ___breaking)
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote) return false;

                if (CoopSession.IsHost && !NetSyncGuard.IsApplyingRemote && !___breaking && __instance != null && CoopSession.ConnectedPeers.Count > 0)
                {
                    if (CoopWorldSync.TryGetActorId(__instance.gameObject, out long crateId) && crateId > 0)
                    {
                        SceneContext ctx = SRSingleton<SceneContext>.Instance;
                        List<ExchangeDirector.ItemEntry> rewards = ctx != null && ctx.ExchangeDirector != null
                            ? ctx.ExchangeDirector.GetOfferRewards(ExchangeDirector.OfferType.GENERAL)
                            : null;
                        CrateSpecScratch.Clear();
                        if (rewards != null)
                        {
                            for (int i = 0; i < rewards.Count; i++)
                            {
                                if (rewards[i] != null && rewards[i].specReward != ExchangeDirector.NonIdentReward.NONE)
                                {
                                    CrateSpecScratch.Add(rewards[i].specReward);
                                }
                            }
                        }

                        SyncWriter.Reset();
                        SyncWriter.WriteLong(crateId);
                        int count = Math.Min(16, CrateSpecScratch.Count);
                        SyncWriter.WriteByte((byte)count);
                        for (int i = 0; i < count; i++)
                        {
                            SyncWriter.WriteInt((int)CrateSpecScratch[i]);
                        }
                        CrateSpecScratch.Clear();
                        CoopSession.BroadcastReliable(PacketType.ExchangeCrateBreak, SyncWriter);
                    }
                }
                return true;
            }
        }

        [HarmonyPatch(typeof(DirectRewardEjector), nameof(DirectRewardEjector.Eject))]
        internal static class DirectRewardEjectorSyncPatch
        {
            private static void Postfix(DirectRewardEjector __instance, bool ___rewardIsActor)
            {
                if (!CoopSession.IsHost || NetSyncGuard.IsApplyingRemote || __instance == null || ___rewardIsActor) return;
                if (CoopSession.ConnectedPeers.Count == 0) return;

                SyncWriter.Reset();
                SyncWriter.WriteByte((byte)__instance.offerType);
                CoopSession.BroadcastReliable(PacketType.ExchangeDirectCoin, SyncWriter);
            }
        }

        [HarmonyPatch(typeof(ProgressDirector), nameof(ProgressDirector.SetUniqueProgress))]
        internal static class HostUniqueProgressExchangeSyncPatch
        {
            private static void Postfix(bool __result, ProgressDirector.ProgressType type)
            {
                if (!__result || !CoopSession.IsHost || NetSyncGuard.IsApplyingRemote) return;
                if (type == ProgressDirector.ProgressType.UNLOCK_WILDS ||
                    type == ProgressDirector.ProgressType.UNLOCK_VALLEY ||
                    type == ProgressDirector.ProgressType.UNLOCK_SLIMULATIONS ||
                    type == ProgressDirector.ProgressType.OGDEN_SEEN_FINAL_CHAT ||
                    type == ProgressDirector.ProgressType.MOCHI_SEEN_FINAL_CHAT ||
                    type == ProgressDirector.ProgressType.VIKTOR_SEEN_FINAL_CHAT)
                {
                    BroadcastHostExchangeState();
                }
            }
        }

        [HarmonyPatch(typeof(ProgressDirector), nameof(ProgressDirector.AddProgress))]
        internal static class HostAddProgressExchangeSyncPatch
        {
            private static void Postfix(ProgressDirector.ProgressType type)
            {
                if (!CoopSession.IsHost || NetSyncGuard.IsApplyingRemote) return;
                if (type == ProgressDirector.ProgressType.UNLOCK_OGDEN_MISSIONS ||
                    type == ProgressDirector.ProgressType.UNLOCK_MOCHI_MISSIONS ||
                    type == ProgressDirector.ProgressType.UNLOCK_VIKTOR_MISSIONS ||
                    type == ProgressDirector.ProgressType.OGDEN_REWARDS ||
                    type == ProgressDirector.ProgressType.MOCHI_REWARDS ||
                    type == ProgressDirector.ProgressType.VIKTOR_REWARDS)
                {
                    BroadcastHostExchangeState();
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

        // --- Shared World: Gordos & Gordo Snares ---
        private static void BroadcastHostSnareState(GordoSnare snare)
        {
            if (!CoopSession.IsHost || snare == null) return;
            if (CoopWorldSync.TryGetSnareState(snare, out string siteId, out Identifiable.Id baitId, out Identifiable.Id gordoId, out int eatenCount))
            {
                SyncWriter.Reset();
                SyncWriter.WriteString(siteId);
                SyncWriter.WriteInt((int)baitId);
                SyncWriter.WriteInt((int)gordoId);
                SyncWriter.WriteInt(eatenCount);
                CoopSession.BroadcastReliable(PacketType.SnareStateSync, SyncWriter);
            }
        }

        [HarmonyPatch(typeof(GordoEat), nameof(GordoEat.MaybeEat))]
        internal static class GordoEatClientBlockPatch
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

        [HarmonyPatch(typeof(GordoEat), nameof(GordoEat.SetModel), new[] { typeof(GordoModel) })]
        internal static class GordoEatSetModelClientPatch
        {
            private static void Postfix(GordoEat __instance)
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote)
                {
                    CoopWorldSync.OnClientGordoModelSet(__instance);
                }
            }
        }

        [HarmonyPatch(typeof(GordoEat), "SetEatenCount")]
        internal static class GordoEatSyncPatch
        {
            private static void Postfix(GordoEat __instance, int eatenCount)
            {
                if (!CoopSession.IsActive || NetSyncGuard.IsApplyingRemote || __instance == null) return;

                if (string.IsNullOrEmpty(__instance.id))
                {
                    if (CoopSession.IsHost)
                    {
                        GordoSnare snare = __instance.GetComponentInParent<GordoSnare>();
                        if (snare != null)
                        {
                            BroadcastHostSnareState(snare);
                        }
                    }
                    return;
                }

                SyncWriter.Reset();
                SyncWriter.WriteString(__instance.id);
                SyncWriter.WriteInt(eatenCount);

                if (CoopSession.IsHost) CoopSession.BroadcastReliable(PacketType.GordoEatSync, SyncWriter);
                else CoopSession.SendToHostReliable(PacketType.GordoEatSync, SyncWriter);
            }
        }

        [HarmonyPatch(typeof(GordoSnare), nameof(GordoSnare.OnTriggerEnter))]
        internal static class GordoSnareTriggerClientBlockPatch
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

        [HarmonyPatch(typeof(GordoSnare), "AttachBait", new[] { typeof(Identifiable.Id) })]
        internal static class GordoSnareAttachBaitSyncPatch
        {
            private static void Postfix(GordoSnare __instance)
            {
                if (CoopSession.IsHost && !NetSyncGuard.IsApplyingRemote)
                {
                    BroadcastHostSnareState(__instance);
                }
            }
        }

        [HarmonyPatch(typeof(GordoSnare), "SnareGordo", new[] { typeof(Identifiable.Id) })]
        internal static class GordoSnareSnareGordoSyncPatch
        {
            private static void Postfix(GordoSnare __instance)
            {
                if (CoopSession.IsHost && !NetSyncGuard.IsApplyingRemote)
                {
                    BroadcastHostSnareState(__instance);
                }
            }
        }

        [HarmonyPatch(typeof(GordoSnare), "ClearBait")]
        internal static class GordoSnareClearBaitSyncPatch
        {
            private static void Postfix(GordoSnare __instance)
            {
                if (CoopSession.IsHost && !NetSyncGuard.IsApplyingRemote)
                {
                    BroadcastHostSnareState(__instance);
                }
            }
        }

        [HarmonyPatch(typeof(GadgetSite), nameof(GadgetSite.SetModel))]
        internal static class GadgetSiteSetModelClientPatch
        {
            private static void Postfix(GadgetSite __instance)
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote)
                {
                    CoopWorldSync.OnClientGadgetSiteModelSet(__instance);
                }
            }
        }

        [HarmonyPatch(typeof(GameModel), nameof(GameModel.RegisterGordo))]
        internal static class GameModelRegisterGordoClientPatch
        {
            private static void Postfix(string gordoId, GameObject gordoObj)
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote)
                {
                    CoopWorldSync.OnClientGordoRegistered(gordoId, gordoObj);
                }
            }
        }

        [HarmonyPatch(typeof(GameModel), nameof(GameModel.RegisterGadgetSite))]
        internal static class GameModelRegisterGadgetSiteClientPatch
        {
            private static void Postfix(string siteId, GameObject gameObject)
            {
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote)
                {
                    CoopWorldSync.OnClientGadgetSiteRegistered(siteId, gameObject);
                }
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
                if (!CoopSession.IsActive || __result == null) return;

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
                if (!CoopSession.IsActive || gameObj == null) return;
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

        [HarmonyPatch(typeof(WeaponVacuum), "ExpelHeld")]
        internal static class ClientVacExpelHeldPatch
        {
            private static readonly AccessTools.FieldRef<WeaponVacuum, GameObject> VacHeldRef =
                AccessTools.FieldRefAccess<WeaponVacuum, GameObject>("held");

            private static void Prefix(WeaponVacuum __instance, out GameObject __state)
            {
                __state = null;
                if (CoopSession.IsClient && !NetSyncGuard.IsApplyingRemote && __instance != null)
                {
                    __state = VacHeldRef(__instance);
                }
            }

            private static void Postfix(WeaponVacuum __instance, GameObject __state)
            {
                if (__state == null || !CoopSession.IsClient || NetSyncGuard.IsApplyingRemote || __instance == null)
                {
                    return;
                }
                if (VacHeldRef(__instance) == null && CoopWorldSync.TryGetActorId(__state, out long actorId) && actorId > 0)
                {
                    Rigidbody rb = __state.GetComponent<Rigidbody>();
                    Vector3 vel = rb != null ? rb.velocity : Vector3.zero;
                    SyncWriter.Reset();
                    SyncWriter.WriteLong(actorId);
                    SyncWriter.WriteVector3(__state.transform.position);
                    SyncWriter.WriteVector3(vel);
                    CoopSession.SendToHostReliable(PacketType.ClientThrowHeld, SyncWriter);
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

        // --- Prevent client knockout/sleep from hanging forever in FastForwardTo ---
        // Host owns worldTime and constantly syncs it to clients; if client calls FastForwardTo,
        // host's worldTime keeps resetting it before unlockWorldTime is reached (infinite clock ticking).
        internal static class ClientLockOnDeathState
        {
            public static bool Active;
            public static float UnlockRealtime;
            public static readonly AccessTools.FieldRef<LockOnDeath, double> UnlockWorldTimeRef =
                AccessTools.FieldRefAccess<LockOnDeath, double>("unlockWorldTime");

            public static IEnumerator EmptyCoroutine()
            {
                yield break;
            }
        }

        [HarmonyPatch(typeof(LockOnDeath), nameof(LockOnDeath.LockUntil))]
        internal static class ClientLockUntilPatch
        {
            private static void Prefix(ref double unlockWorldTime, float delayTime)
            {
                if (!CoopSession.IsClient) return;

                ClientLockOnDeathState.Active = true;
                // Wait out knockout UI fade (~5s) or brief sleep fade (0.5s), then wake without warping host time
                ClientLockOnDeathState.UnlockRealtime = Time.unscaledTime + (delayTime > 0.1f ? delayTime + 0.3f : 0.5f);
                unlockWorldTime = double.MaxValue;
                CoopLog.Info("Client LockOnDeath.LockUntil intercepted (delay=" + delayTime.ToString("F1") + "s)");
            }
        }

        [HarmonyPatch(typeof(LockOnDeath), "DelayedFastForward")]
        internal static class ClientLockDelayedFastForwardPatch
        {
            private static bool Prefix(ref IEnumerator __result)
            {
                if (!CoopSession.IsClient) return true;
                __result = ClientLockOnDeathState.EmptyCoroutine();
                return false;
            }
        }

        [HarmonyPatch(typeof(LockOnDeath), nameof(LockOnDeath.Update))]
        internal static class ClientLockUpdatePatch
        {
            private static void Prefix(LockOnDeath __instance)
            {
                if (__instance == null || !__instance.Locked())
                {
                    ClientLockOnDeathState.Active = false;
                    return;
                }

                if (!CoopSession.IsClient)
                {
                    if (ClientLockOnDeathState.Active)
                    {
                        ClientLockOnDeathState.Active = false;
                        SceneContext ctx = SRSingleton<SceneContext>.Instance;
                        if (ctx != null && ctx.TimeDirector != null)
                        {
                            ClientLockOnDeathState.UnlockWorldTimeRef(__instance) = ctx.TimeDirector.WorldTime() - 1.0;
                        }
                    }
                    return;
                }

                if (ClientLockOnDeathState.Active && Time.unscaledTime >= ClientLockOnDeathState.UnlockRealtime)
                {
                    ClientLockOnDeathState.Active = false;
                    SceneContext ctx = SRSingleton<SceneContext>.Instance;
                    if (ctx != null && ctx.TimeDirector != null)
                    {
                        ClientLockOnDeathState.UnlockWorldTimeRef(__instance) = ctx.TimeDirector.WorldTime() - 1.0;
                    }
                    CoopLog.Info("Client LockOnDeath unlocked cleanly");
                }
            }
        }
    }
}
