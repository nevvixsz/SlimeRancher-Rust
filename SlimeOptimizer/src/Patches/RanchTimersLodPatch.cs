using System;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    internal static class RanchTimersLodHelper
    {
        public static bool ShouldRun(Component component)
        {
            if (!OptimizerConfig.EnableRanchTimersLod)
            {
                return true;
            }

            int interval = Math.Max(2, OptimizerConfig.RanchTimersLodInterval);
            int phase = (Time.frameCount + (component.GetInstanceID() & 0x7FFFFFFF)) % interval;
            if (phase == 0)
            {
                return true;
            }

            if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.RanchTimersSkipped++;
            }
            return false;
        }
    }

    [HarmonyPatch(typeof(ResourceCycle), nameof(ResourceCycle.RegistryUpdate))]
    internal static class ResourceCycleLodPatch
    {
        private static bool Prefix(
            ResourceCycle __instance,
            bool ___preparingToRelease,
            Material ___rotInProgressMat,
            object ___additionalRipenessDelegate)
        {
            if (___preparingToRelease || ___rotInProgressMat != null || ___additionalRipenessDelegate != null)
            {
                return true;
            }

            return RanchTimersLodHelper.ShouldRun(__instance);
        }
    }

    [HarmonyPatch(typeof(DestroyPlortAfterTime), nameof(DestroyPlortAfterTime.RegistryUpdate))]
    internal static class DestroyPlortAfterTimeLodPatch
    {
        private static bool Prefix(DestroyPlortAfterTime __instance)
        {
            return RanchTimersLodHelper.ShouldRun(__instance);
        }
    }

    [HarmonyPatch(typeof(DestroyAfterTime), nameof(DestroyAfterTime.RegistryUpdate))]
    internal static class DestroyAfterTimeLodPatch
    {
        private static bool Prefix(DestroyAfterTime __instance)
        {
            return RanchTimersLodHelper.ShouldRun(__instance);
        }
    }

    [HarmonyPatch(typeof(GlintController), nameof(GlintController.RegistryUpdate))]
    internal static class GlintControllerLodPatch
    {
        private static bool Prefix(GlintController __instance)
        {
            return RanchTimersLodHelper.ShouldRun(__instance);
        }
    }
}
