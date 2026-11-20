using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(KeepUpright), nameof(KeepUpright.RegistryFixedUpdate))]
    internal static class KeepUprightLodPatch
    {
        private sealed class UprightEntry
        {
            public Transform Transform;
            public int Hash;
            public int NextCheckFrame;
            public bool IsFar;
        }

        private static readonly FastObjCache<KeepUpright, UprightEntry> Cache =
            new FastObjCache<KeepUpright, UprightEntry>(256);

        private static bool Prefix(KeepUpright __instance, Rigidbody ___body)
        {
            if (!OptimizerConfig.EnableKeepUprightOpt)
            {
                return true;
            }

            if (___body != null && ___body.IsSleeping())
            {
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.KeepUprightSkipped++;
                }
                return false;
            }

            if (CameraCache.HasCamera)
            {
                int frame = CameraCache.FrameCount;
                if (!Cache.TryGetValue(__instance, out UprightEntry entry))
                {
                    entry = new UprightEntry
                    {
                        Transform = __instance.transform,
                        Hash = RuntimeHelpers.GetHashCode(__instance) & 0x7FFFFFFF,
                        NextCheckFrame = -1
                    };
                    Cache.Set(__instance, entry);
                }

                if (frame >= entry.NextCheckFrame)
                {
                    entry.NextCheckFrame = frame + 16 + (entry.Hash & 7);
                    entry.IsFar = CameraCache.SqrDistanceToCamera(entry.Transform) >= 625f;
                }

                if (entry.IsFar && ((frame + entry.Hash) & 3) != 0)
                {
                    if (OptimizerConfig.EnableStatsCollection)
                    {
                        OptimizerStats.KeepUprightSkipped++;
                    }
                    return false;
                }
            }

            return true;
        }
    }

    [HarmonyPatch(typeof(ChickenRandomMove), nameof(ChickenRandomMove.RegistryUpdate))]
    internal static class ChickenAiLodPatch
    {
        private sealed class ChickenEntry
        {
            public Transform Transform;
            public int Hash;
            public int NextCheckFrame;
            public byte Tier;
        }

        private static readonly FastObjCache<ChickenRandomMove, ChickenEntry> Cache =
            new FastObjCache<ChickenRandomMove, ChickenEntry>(128);

        private static bool Prefix(ChickenRandomMove __instance)
        {
            if (!OptimizerConfig.EnableChickenAiLod || !CameraCache.HasCamera)
            {
                return true;
            }

            int frame = CameraCache.FrameCount;
            if (!Cache.TryGetValue(__instance, out ChickenEntry entry))
            {
                entry = new ChickenEntry
                {
                    Transform = __instance.transform,
                    Hash = RuntimeHelpers.GetHashCode(__instance) & 0x7FFFFFFF,
                    NextCheckFrame = -1
                };
                Cache.Set(__instance, entry);
            }

            if (frame >= entry.NextCheckFrame)
            {
                entry.NextCheckFrame = frame + 16 + (entry.Hash & 7);
                float distSq = CameraCache.SqrDistanceToCamera(entry.Transform);
                entry.Tier = distSq < 400f ? (byte)0 : (distSq >= 1600f ? (byte)2 : (byte)1);
            }

            byte tier = entry.Tier;
            if (tier == 0)
            {
                return true;
            }

            int mask = tier == 2 ? 7 : 3;
            if (((frame + entry.Hash) & mask) == 0)
            {
                return true;
            }

            if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.ChickenAiSkipped++;
            }
            return false;
        }
    }
}
