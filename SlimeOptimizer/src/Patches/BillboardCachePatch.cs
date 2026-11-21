using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(CameraFacingBillboard), "FaceCamera")]
    internal static class BillboardCachePatch
    {
        private static readonly FastObjCache<CameraFacingBillboard, Transform> SelfTransformCache =
            new FastObjCache<CameraFacingBillboard, Transform>(128);

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
                if (ReferenceEquals(cam, null))
                {
                    return false;
                }

                ___mainCamera = cam;
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.BillboardCacheHits++;
                }
            }

            if (!SelfTransformCache.TryGetValue(__instance, out Transform selfTr))
            {
                selfTr = __instance.transform;
                SelfTransformCache.Set(__instance, selfTr);
            }

            Transform camTr = ReferenceEquals(cam, CameraCache.MainCam) && !ReferenceEquals(CameraCache.MainCamTransform, null)
                ? CameraCache.MainCamTransform
                : cam.transform;

            selfTr.LookAt(camTr);
            return false;
        }
    }
}
