using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(CameraFacingBillboard), "FaceCamera")]
    internal static class BillboardCachePatch
    {
        [HarmonyPrefix]
        private static bool Prefix(CameraFacingBillboard __instance, ref Camera ___mainCamera)
        {
            if (!OptimizerConfig.EnableBillboardCameraCache)
            {
                return true;
            }

            Camera cam = ___mainCamera;
            if (cam == null)
            {
                cam = CameraCache.MainCamera;
                if (cam == null)
                {
                    return false;
                }

                ___mainCamera = cam;
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.BillboardCacheHits++;
                }
            }

            __instance.transform.LookAt(cam.transform);
            return false;
        }
    }
}
