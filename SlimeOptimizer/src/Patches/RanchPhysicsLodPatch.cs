using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(KeepUpright), nameof(KeepUpright.RegistryFixedUpdate))]
    internal static class KeepUprightLodPatch
    {
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
                Vector3 diff = __instance.transform.position - CameraCache.Position;
                if (diff.sqrMagnitude >= 625f) // >= 25m
                {
                    int phase = (Time.frameCount + (__instance.GetInstanceID() & 0x7FFFFFFF)) % 3;
                    if (phase != 0)
                    {
                        if (OptimizerConfig.EnableStatsCollection)
                        {
                            OptimizerStats.KeepUprightSkipped++;
                        }
                        return false;
                    }
                }
            }

            return true;
        }
    }

    [HarmonyPatch(typeof(ChickenRandomMove), nameof(ChickenRandomMove.RegistryUpdate))]
    internal static class ChickenAiLodPatch
    {
        private static bool Prefix(ChickenRandomMove __instance)
        {
            if (!OptimizerConfig.EnableChickenAiLod || !CameraCache.HasCamera)
            {
                return true;
            }

            Vector3 diff = __instance.transform.position - CameraCache.Position;
            float distSq = diff.sqrMagnitude;
            if (distSq < 400f) // < 20m
            {
                return true;
            }

            int interval = distSq >= 1600f ? 10 : 5;
            int phase = (Time.frameCount + (__instance.GetInstanceID() & 0x7FFFFFFF)) % interval;
            if (phase == 0)
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
