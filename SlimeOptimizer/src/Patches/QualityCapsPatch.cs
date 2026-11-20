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

            int maxCascades = NormalizeCascades(OptimizerConfig.MaxShadowCascades);
            if (maxCascades >= 0 && QualitySettings.shadowCascades > maxCascades)
            {
                QualitySettings.shadowCascades = maxCascades;
            }

            float maxShadowDist = OptimizerConfig.MaxShadowDistance;
            if (maxShadowDist > 0f && QualitySettings.shadowDistance > maxShadowDist)
            {
                QualitySettings.shadowDistance = maxShadowDist;
            }

            if (QualitySettings.shadowCascades == 0 || QualitySettings.shadowDistance < 60f)
            {
                QualitySettings.shadowProjection = ShadowProjection.CloseFit;
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

            if (OptimizerConfig.DisableSoftParticles && QualitySettings.softParticles)
            {
                QualitySettings.softParticles = false;
            }

            int maxBudget = OptimizerConfig.MaxParticleRaycastBudget;
            if (maxBudget >= 0 && QualitySettings.particleRaycastBudget > maxBudget)
            {
                QualitySettings.particleRaycastBudget = maxBudget;
            }

            int maxAniso = OptimizerConfig.MaxAnisoMode;
            if (maxAniso >= 0 && maxAniso <= 2 && (int)QualitySettings.anisotropicFiltering > maxAniso)
            {
                QualitySettings.anisotropicFiltering = (AnisotropicFiltering)maxAniso;
            }
        }

        public static void ApplyPhysicsSettings()
        {
            if (OptimizerConfig.EnablePhysicsOpt)
            {
                int target = Mathf.Clamp(OptimizerConfig.PhysicsSolverIterations, 2, 6);
                if (Physics.defaultSolverIterations != target)
                {
                    Physics.defaultSolverIterations = target;
                }
                if (Physics.defaultSolverVelocityIterations != 1)
                {
                    Physics.defaultSolverVelocityIterations = 1;
                }
            }
            else if (Physics.defaultSolverIterations != 6)
            {
                Physics.defaultSolverIterations = 6;
            }
        }

        public static void ApplyTargetFrameRate()
        {
            int target = OptimizerConfig.TargetFrameRate;
            int desired = target > 0 ? target : -1;
            if (Application.targetFrameRate != desired)
            {
                Application.targetFrameRate = desired;
            }
        }

        private static int NormalizeCascades(int cascades)
        {
            if (cascades <= 0) return 0;
            if (cascades <= 2) return 2;
            return 4;
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
            ApplyPhysicsSettings();
            ApplyTargetFrameRate();
        }
    }

    [HarmonyPatch(typeof(OptionsDirector), nameof(OptionsDirector.SetVsync))]
    internal static class OptionsDirectorVsyncPatch
    {
        private static void Postfix()
        {
            QualityCapsPatch.ApplyTargetFrameRate();
        }
    }
}
