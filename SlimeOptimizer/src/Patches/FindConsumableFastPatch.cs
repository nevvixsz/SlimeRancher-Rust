using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using HarmonyLib;
using MonomiPark.SlimeRancher.DataModel;
using MonomiPark.SlimeRancher.Regions;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch]
    internal static class FindConsumableFastPatch
    {
        private sealed class CachedKeys
        {
            public Identifiable.Id[] Keys;
            public int LastCount = -1;
        }

        private delegate ActorModel EntryActorModelGetter(GameObjectActorModelIdentifiableIndex.Entry entry);

        private static readonly EntryActorModelGetter GetEntryActorModel = BuildActorModelGetter();

        private static readonly FastObjCache<FindConsumable, CachedKeys> KeyCache =
            new FastObjCache<FindConsumable, CachedKeys>(256);

        private static EntryActorModelGetter BuildActorModelGetter()
        {
            FieldInfo field = AccessTools.Field(typeof(GameObjectActorModelIdentifiableIndex.Entry), "actorModel");
            if (field == null)
            {
                return null;
            }

            DynamicMethod dm = new DynamicMethod(
                "SlimeOpt_GetEntryActorModel",
                typeof(ActorModel),
                new[] { typeof(GameObjectActorModelIdentifiableIndex.Entry) },
                typeof(FindConsumableFastPatch).Module,
                true);

            ILGenerator il = dm.GetILGenerator();
            il.Emit(OpCodes.Ldarga_S, (byte)0);
            il.Emit(OpCodes.Ldfld, field);
            il.Emit(OpCodes.Ret);

            return (EntryActorModelGetter)dm.CreateDelegate(typeof(EntryActorModelGetter));
        }

        internal static void InvalidateKeys(FindConsumable instance)
        {
            if (instance != null && KeyCache.TryGetValue(instance, out CachedKeys cached))
            {
                cached.Keys = null;
                cached.LastCount = -1;
            }
        }

        internal static Identifiable.Id[] GetCachedKeys(FindConsumable instance, Dictionary<Identifiable.Id, DriveCalculator> searchIds)
        {
            if (!KeyCache.TryGetValue(instance, out CachedKeys cached))
            {
                cached = new CachedKeys();
                KeyCache.Set(instance, cached);
            }

            int count = searchIds.Count;
            if (cached.Keys == null || cached.LastCount != count)
            {
                Identifiable.Id[] arr = new Identifiable.Id[count];
                searchIds.Keys.CopyTo(arr, 0);
                cached.Keys = arr;
                cached.LastCount = count;
            }

            return cached.Keys;
        }

        internal static GameObject FindNearestFast(
            FindConsumable instance,
            IList<GameObjectActorModelIdentifiableIndex.Entry> gameObjects,
            Dictionary<Identifiable.Id, DriveCalculator> searchIds,
            SlimeEmotions emotions,
            out float drive)
        {
            GameObject result = null;
            float maxRad = instance.maxSearchRad;
            float bestScore = 1f / (maxRad * maxRad);
            drive = 0f;
            float minDist = instance.minDist;
            float minSqrDist = minDist * minDist;

            GameObject selfGo = instance.gameObject;
            Vector3 selfPos = instance.transform.position;
            GameObject playerGo = CameraCache.PlayerGo;

            int count = gameObjects.Count;
            if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.FastFoodSearches++;
                OptimizerStats.FastFoodEntriesScanned += count;
            }

            for (int i = 0; i < count; i++)
            {
                GameObjectActorModelIdentifiableIndex.Entry entry = gameObjects[i];
                GameObject go = entry.GameObject;
                if (ReferenceEquals(go, null) || ReferenceEquals(go, selfGo))
                {
                    continue;
                }

                Identifiable.Id id = entry.Id;
                if (!searchIds.TryGetValue(id, out DriveCalculator driveCalc))
                {
                    continue;
                }

                bool isEdible;
                if (GetEntryActorModel != null)
                {
                    ActorModel model = GetEntryActorModel(entry);
                    isEdible = model != null && model.IsEdible();
                }
                else
                {
                    isEdible = Identifiable.IsEdible(go);
                }

                if (!isEdible)
                {
                    continue;
                }

                Vector3 targetPos = go.transform.position;
                if (ReferenceEquals(go, playerGo))
                {
                    targetPos.y += 1f;
                }

                float dx = targetPos.x - selfPos.x;
                float dy = targetPos.y - selfPos.y;
                float dz = targetPos.z - selfPos.z;
                float sqrMag = dx * dx + dy * dy + dz * dz;

                if (sqrMag >= minSqrDist)
                {
                    float rawDrive = driveCalc.Drive(emotions, id);
                    float score = rawDrive / sqrMag;
                    if (score > bestScore)
                    {
                        result = go;
                        bestScore = score;
                        drive = rawDrive < 0f ? 0f : (rawDrive > 1f ? 1f : rawDrive);
                    }
                }
            }

            return result;
        }

        [HarmonyTargetMethod]
        private static MethodBase TargetMethod()
        {
            return AccessTools.Method(
                typeof(FindConsumable),
                "FindNearestConsumable",
                new[] { typeof(IList<GameObjectActorModelIdentifiableIndex.Entry>), typeof(float).MakeByRefType() });
        }

        [HarmonyPrefix]
        private static bool Prefix(
            FindConsumable __instance,
            IList<GameObjectActorModelIdentifiableIndex.Entry> gameObjects,
            out float drive,
            Dictionary<Identifiable.Id, DriveCalculator> ___searchIds,
            SlimeEmotions ___emotions,
            ref GameObject __result)
        {
            if (!OptimizerConfig.EnableFastFoodSearch || ___searchIds == null)
            {
                drive = 0f;
                return true;
            }

            __result = FindNearestFast(__instance, gameObjects, ___searchIds, ___emotions, out drive);
            return false;
        }
    }

    [HarmonyPatch]
    internal static class FindConsumableByMemberPatch
    {
        private static readonly List<GameObjectActorModelIdentifiableIndex.Entry> SharedEntries =
            new List<GameObjectActorModelIdentifiableIndex.Entry>(128);

        [HarmonyTargetMethod]
        private static MethodBase TargetMethod()
        {
            return AccessTools.Method(
                typeof(FindConsumable),
                "FindNearestConsumable",
                new[] { typeof(float).MakeByRefType() });
        }

        [HarmonyPrefix]
        private static bool Prefix(
            FindConsumable __instance,
            out float drive,
            Dictionary<Identifiable.Id, DriveCalculator> ___searchIds,
            RegionMember ___member,
            SlimeEmotions ___emotions,
            ref GameObject __result)
        {
            if (!OptimizerConfig.EnableFastFoodSearch || ___searchIds == null || ___member == null)
            {
                drive = 0f;
                return true;
            }

            Identifiable.Id[] keys = FindConsumableFastPatch.GetCachedKeys(__instance, ___searchIds);
            SharedEntries.Clear();
            CellDirector.Get(keys, ___member, SharedEntries);
            __result = FindConsumableFastPatch.FindNearestFast(__instance, SharedEntries, ___searchIds, ___emotions, out drive);
            SharedEntries.Clear();
            return false;
        }
    }

    [HarmonyPatch(typeof(FindConsumable), "UpdateSearchIds")]
    internal static class FindConsumableUpdateIdsPatch
    {
        [HarmonyPostfix]
        private static void Postfix(FindConsumable __instance)
        {
            FindConsumableFastPatch.InvalidateKeys(__instance);
        }
    }
}
