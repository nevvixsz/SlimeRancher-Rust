using System.Reflection;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    internal static class QualityCapsPatch
    {
        private static bool _installed;
        private static MethodInfo _updateFromLevelsMethod;

        public static void Install(Harmony harmony)
        {
            if (_installed || harmony == null)
            {
                return;
            }

            _updateFromLevelsMethod = AccessTools.Method(typeof(SRQualitySettings), "UpdateFromLevels");
            MethodInfo postfix = AccessTools.Method(typeof(QualityCapsPatch), nameof(Postfix));
            if (_updateFromLevelsMethod != null && postfix != null)
            {
                harmony.Patch(_updateFromLevelsMethod, postfix: new HarmonyMethod(postfix));
                _installed = true;
            }
        }

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

            float maxLodBias = OptimizerConfig.MaxLodBias;
            if (maxLodBias > 0f && QualitySettings.lodBias > maxLodBias)
            {
                QualitySettings.lodBias = maxLodBias;
            }

            int maxShadowRes = OptimizerConfig.MaxShadowResolution;
            if (maxShadowRes >= 0 && maxShadowRes <= 3 && (int)QualitySettings.shadowResolution > maxShadowRes)
            {
                QualitySettings.shadowResolution = (ShadowResolution)maxShadowRes;
            }

            int maxPixelLights = OptimizerConfig.MaxPixelLightCount;
            if (maxPixelLights >= 0 && QualitySettings.pixelLightCount > maxPixelLights)
            {
                QualitySettings.pixelLightCount = maxPixelLights;
            }

            int maxGlobalShaderLod = OptimizerConfig.MaxGlobalShaderLod;
            if (maxGlobalShaderLod > 0 && Shader.globalMaximumLOD > maxGlobalShaderLod)
            {
                Shader.globalMaximumLOD = maxGlobalShaderLod;
            }
        }

        public static void ReapplyQualitySettings()
        {
            if (_updateFromLevelsMethod == null)
            {
                _updateFromLevelsMethod = AccessTools.Method(typeof(SRQualitySettings), "UpdateFromLevels");
            }

            if (_updateFromLevelsMethod != null)
            {
                _updateFromLevelsMethod.Invoke(null, null);
            }
            else
            {
                ApplyCapsIfEnabled();
            }
        }
    }
}
