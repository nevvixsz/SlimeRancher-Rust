using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(ResourceCycle), nameof(ResourceCycle.RegistryUpdate))]
    internal static class ResourceCycleLodPatch
    {
        [HarmonyPrefix]
        private static bool Prefix(
            ResourceCycle __instance,
            bool ___preparingToRelease,
            Material ___rotInProgressMat,
            object ___additionalRipenessDelegate)
        {
            if (!OptimizerConfig.EnableRanchTimersLod ||
                ___preparingToRelease ||
                !ReferenceEquals(___rotInProgressMat, null) ||
                !ReferenceEquals(___additionalRipenessDelegate, null))
            {
                return true;
            }

            int interval = OptimizerConfig.RanchTimersLodInterval;
            int mask = interval >= 16 ? 15 : (interval >= 8 ? 7 : (interval >= 4 ? 3 : 1));
            int hash = FastObjCache<ResourceCycle, object>.MixHash(__instance);

            if (((CameraCache.FrameCount + hash) & mask) == 0)
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
}
