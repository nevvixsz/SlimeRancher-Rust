using System.Reflection;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(SRQualitySettings), "UpdateFromLevels")]
    internal static class QualityCapsPatch
    {
        private static readonly MethodInfo UpdateFromLevelsMethod =
            AccessTools.Method(typeof(SRQualitySettings), "UpdateFromLevels");

        [HarmonyPostfix]
        private static void Postfix()
        {
            ApplyCapsIfEnabled();
        }

        public static void ApplyCapsIfEnabled()
        {
            if (!OptimizerConfig.EnableSmartQualityCaps)
            {
                return;
            }

            int maxCascades = OptimizerConfig.MaxShadowCascades;
            if (maxCascades >= 0 && QualitySettings.shadowCascades > maxCascades)
            {
                QualitySettings.shadowCascades = maxCascades;
            }

            float maxShadowDist = OptimizerConfig.MaxShadowDistance;
            if (maxShadowDist > 0f && QualitySettings.shadowDistance > maxShadowDist)
            {
                QualitySettings.shadowDistance = maxShadowDist;
            }

            int maxAa = OptimizerConfig.MaxAntiAliasing;
            if (maxAa >= 0 && QualitySettings.antiAliasing > maxAa)
            {
                QualitySettings.antiAliasing = maxAa;
            }
        }

        public static void ReapplyQualitySettings()
        {
            if (UpdateFromLevelsMethod != null)
            {
                UpdateFromLevelsMethod.Invoke(null, null);
            }
            else
            {
                ApplyCapsIfEnabled();
            }
        }
    }
}
