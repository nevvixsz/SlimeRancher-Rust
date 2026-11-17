using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(RubberBoneEffect), "LateUpdate")]
    internal static class RubberBoneLodPatch
    {
        [HarmonyPrefix]
        private static bool Prefix(
            RubberBoneEffect __instance,
            Rigidbody ___ownBody,
            Renderer ___ownRenderer,
            Vacuumable ___vacuumable,
            object ___verts,
            ref bool ___sleeping,
            ref bool ___wasSleeping)
        {
            if (!OptimizerConfig.EnableRubberBoneLod || ___verts == null)
            {
                return true;
            }

            if (___vacuumable != null && ___vacuumable.isHeld())
            {
                return true;
            }

            if ((___ownBody != null && ___ownBody.IsSleeping()) || (___ownRenderer != null && !___ownRenderer.isVisible))
            {
                return true;
            }

            CameraCache.EnsureCurrentFrame();
            if (!CameraCache.HasCamera)
            {
                return true;
            }

            Vector3 pos = __instance.transform.position;
            Vector3 cam = CameraCache.Position;
            float dx = pos.x - cam.x;
            float dy = pos.y - cam.y;
            float dz = pos.z - cam.z;
            float sqrDist = dx * dx + dy * dy + dz * dz;

            float disableDist = OptimizerConfig.RubberDisableDistance;
            if (disableDist > 0f && sqrDist >= disableDist * disableDist)
            {
                if (!___sleeping)
                {
                    __instance.Reset();
                    ___sleeping = true;
                }

                ___wasSleeping = true;
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.RubberCulledCalls++;
                }

                return false;
            }

            int interval = OptimizerConfig.RubberLodInterval;
            float lodDist = OptimizerConfig.RubberLodDistance;
            if (interval > 1 && lodDist >= 0f && sqrDist >= lodDist * lodDist)
            {
                int phase = (__instance.GetInstanceID() & 0x7FFFFFFF) % interval;
                if ((Time.frameCount % interval) != phase)
                {
                    if (OptimizerConfig.EnableStatsCollection)
                    {
                        OptimizerStats.RubberSkippedFrames++;
                    }

                    return false;
                }
            }

            return true;
        }
    }
}
