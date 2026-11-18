using System;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    internal static class SlimeFaceEmotionLodHelper
    {
        public static bool ShouldRunThisFrame(Component component, bool isFaceAnimator)
        {
            if (!OptimizerConfig.EnableSlimeFaceEmotionLod || !CameraCache.HasCamera)
            {
                return true;
            }

            Vector3 diff = component.transform.position - CameraCache.Position;
            float distSq = diff.sqrMagnitude;

            float nearDist = OptimizerConfig.FaceEmotionLodDistance;
            if (distSq < nearDist * nearDist)
            {
                return true;
            }

            float ultraDist = OptimizerConfig.FaceEmotionUltraDistance;
            int interval = distSq >= ultraDist * ultraDist
                ? Math.Max(2, OptimizerConfig.FaceEmotionUltraInterval)
                : Math.Max(2, OptimizerConfig.FaceEmotionLodInterval);

            int phase = (Time.frameCount + (component.GetInstanceID() & 0x7FFFFFFF)) % interval;
            if (phase == 0)
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
