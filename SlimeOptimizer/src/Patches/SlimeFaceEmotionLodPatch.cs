using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    internal static class SlimeFaceEmotionLodHelper
    {
        private sealed class LodEntry
        {
            public Transform Transform;
            public int Hash;
            public int NextCheckFrame;
            public byte Tier; // 0 = near, 1 = far, 2 = ultra
        }

        private static readonly FastObjCache<Component, LodEntry> Cache =
            new FastObjCache<Component, LodEntry>(512);

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static bool ShouldRunThisFrame(Component component, bool isFaceAnimator)
        {
            if (!OptimizerConfig.EnableSlimeFaceEmotionLod || !CameraCache.HasCamera)
            {
                return true;
            }

            int frame = CameraCache.FrameCount;
            if (!Cache.TryGetValue(component, out LodEntry entry))
            {
                entry = new LodEntry
                {
                    Transform = component.transform,
                    Hash = FastObjCache<Component, LodEntry>.MixHash(component),
                    NextCheckFrame = -1,
                    Tier = 0
                };
                Cache.Set(component, entry);
            }

            if (frame >= entry.NextCheckFrame)
            {
                entry.NextCheckFrame = frame + 16 + (entry.Hash & 7);
                float distSq = CameraCache.SqrDistanceToCamera(entry.Transform);
                float nearDist = OptimizerConfig.FaceEmotionLodDistance;
                if (distSq < nearDist * nearDist)
                {
                    entry.Tier = 0;
                }
                else
                {
                    float ultraDist = OptimizerConfig.FaceEmotionUltraDistance;
                    entry.Tier = distSq >= ultraDist * ultraDist ? (byte)2 : (byte)1;
                }
            }

            byte tier = entry.Tier;
            if (tier == 0)
            {
                return true;
            }

            int mask = tier == 2 ? 15 : 7;
            if (((frame + entry.Hash) & mask) == 0)
            {
                return true;
            }

            if (OptimizerConfig.EnableStatsCollection)
            {
                if (isFaceAnimator)
                {
                    OptimizerStats.FaceAnimCallsSkipped++;
                }
                else
                {
                    OptimizerStats.EmotionsCallsSkipped++;
                }
            }

            return false;
        }
    }

    [HarmonyPatch(typeof(SlimeFaceAnimator), nameof(SlimeFaceAnimator.RegistryUpdate))]
    internal static class SlimeFaceAnimatorLodPatch
    {
        private static bool Prefix(SlimeFaceAnimator __instance)
        {
            return SlimeFaceEmotionLodHelper.ShouldRunThisFrame(__instance, true);
        }
    }

    [HarmonyPatch(typeof(SlimeEmotions), nameof(SlimeEmotions.RegistryUpdate))]
    internal static class SlimeEmotionsLodPatch
    {
        private static bool Prefix(SlimeEmotions __instance)
        {
            return SlimeFaceEmotionLodHelper.ShouldRunThisFrame(__instance, false);
        }
    }
}
