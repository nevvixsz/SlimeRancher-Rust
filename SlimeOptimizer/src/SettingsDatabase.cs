using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Text;

namespace SlimeOptimizer
{
    internal enum OptimizerPreset
    {
        MaxFps,
        Balanced,
        HighQuality,
        VanillaOff
    }

    internal static class SettingsDatabase
    {
        private static readonly Dictionary<string, object> DefaultValues = new Dictionary<string, object>(StringComparer.OrdinalIgnoreCase);
        private static readonly FieldInfo[] ConfigFields;
        private static bool _defaultsCaptured;
        private static string _dbPath;

        static SettingsDatabase()
        {
            ConfigFields = typeof(OptimizerConfig).GetFields(BindingFlags.Public | BindingFlags.Static);
            CaptureDefaults();
        }

        public static string DatabasePath
        {
            get
            {
                if (string.IsNullOrEmpty(_dbPath))
                {
                    _dbPath = Path.Combine(SRML.FileSystem.GetMyConfigPath(), "SlimeOptimizer.db");
                }
                return _dbPath;
            }
        }

        private static void CaptureDefaults()
        {
            if (_defaultsCaptured)
            {
                return;
            }

            _defaultsCaptured = true;
            for (int i = 0; i < ConfigFields.Length; i++)
            {
                FieldInfo f = ConfigFields[i];
                DefaultValues[f.Name] = f.GetValue(null);
            }
        }

        public static void LoadOrCreate()
        {
            CaptureDefaults();
            try
            {
                string path = DatabasePath;
                if (File.Exists(path))
                {
                    string[] lines = File.ReadAllLines(path, Encoding.UTF8);
                    Dictionary<string, FieldInfo> map = new Dictionary<string, FieldInfo>(ConfigFields.Length, StringComparer.OrdinalIgnoreCase);
                    for (int i = 0; i < ConfigFields.Length; i++)
                    {
                        map[ConfigFields[i].Name] = ConfigFields[i];
                    }

                    for (int i = 0; i < lines.Length; i++)
                    {
                        string line = lines[i].Trim();
                        if (line.Length == 0 || line[0] == '#' || line[0] == ';')
                        {
                            continue;
                        }

                        int eq = line.IndexOf('=');
                        if (eq <= 0)
                        {
                            continue;
                        }

                        string key = line.Substring(0, eq).Trim();
                        string val = line.Substring(eq + 1).Trim();
                        if (!map.TryGetValue(key, out FieldInfo field))
                        {
                            continue;
                        }

                        object parsed = ParseValue(field.FieldType, val);
                        if (parsed != null)
                        {
                            field.SetValue(null, parsed);
                        }
                    }
                }

                Save();
            }
            catch (Exception ex)
            {
                ModMain.Logger?.LogWarning("Failed to load SlimeOptimizer.db: " + ex.Message);
            }
        }

        public static void Save()
        {
            try
            {
                string path = DatabasePath;
                string dir = Path.GetDirectoryName(path);
                if (!string.IsNullOrEmpty(dir) && !Directory.Exists(dir))
                {
                    Directory.CreateDirectory(dir);
                }

                StringBuilder sb = new StringBuilder(2048);
                sb.AppendLine("# SlimeOptimizer Settings Database (auto-saved from in-game Options -> Advanced)");
                sb.AppendLine("# Schema=1");

                for (int i = 0; i < ConfigFields.Length; i++)
                {
                    FieldInfo f = ConfigFields[i];
                    object val = f.GetValue(null);
                    sb.Append(f.Name).Append('=').AppendLine(FormatValue(val));
                }

                File.WriteAllText(path, sb.ToString(), Encoding.UTF8);
            }
            catch (Exception ex)
            {
                ModMain.Logger?.LogWarning("Failed to save SlimeOptimizer.db: " + ex.Message);
            }
        }

        public static void ResetToDefaults()
        {
            CaptureDefaults();
            for (int i = 0; i < ConfigFields.Length; i++)
            {
                FieldInfo f = ConfigFields[i];
                if (DefaultValues.TryGetValue(f.Name, out object defVal))
                {
                    f.SetValue(null, defVal);
                }
            }
        }

        public static void ApplyPreset(OptimizerPreset preset)
        {
            ResetToDefaults();

            switch (preset)
            {
                case OptimizerPreset.Balanced:
                    if (IsGameContextReady() &&
                        (SRQualitySettings.CurrentLevel == SRQualitySettings.Level.LOWEST ||
                         (SRQualitySettings.Shadows == SRQualitySettings.ShadowsLevel.NONE &&
                          !SRQualitySettings.AmbientOcclusion && !SRQualitySettings.Bloom)))
                    {
                        SRQualitySettings.CurrentLevel = SRQualitySettings.Level.HIGH;
                    }
                    break;

                case OptimizerPreset.MaxFps:
                    if (IsGameContextReady())
                    {
                        SRQualitySettings.CurrentLevel = SRQualitySettings.Level.LOWEST;
                        SRQualitySettings.Shadows = SRQualitySettings.ShadowsLevel.NONE;
                        SRQualitySettings.AmbientOcclusion = false;
                        SRQualitySettings.Bloom = false;
                    }

                    OptimizerConfig.EnableSmartQualityCaps = true;
                    OptimizerConfig.MaxShadowCascades = 0;
                    OptimizerConfig.MaxShadowResolution = 0;
                    OptimizerConfig.MaxShadowDistance = 20f;
                    OptimizerConfig.MaxAntiAliasing = 0;
                    OptimizerConfig.MaxPixelLightCount = 0;
                    OptimizerConfig.MaxLodBias = 0.5f;
                    OptimizerConfig.MaxGlobalShaderLod = 400;
                    OptimizerConfig.DisableSoftParticles = true;
                    OptimizerConfig.MaxParticleRaycastBudget = 16;
                    OptimizerConfig.MaxAnisoMode = 0;

                    OptimizerConfig.EnableCameraLayerCull = true;
                    OptimizerConfig.LayerActorCullDistance = 45f;
                    OptimizerConfig.MaxFarClipPlane = 220f;

                    OptimizerConfig.EnableSsaoOptimizer = true;
                    OptimizerConfig.SsaoDownsampling = 4;
                    OptimizerConfig.SsaoBlurDownsampling = true;
                    OptimizerConfig.SsaoUseGaussianBlur = true;

                    OptimizerConfig.EnableBloomOptimizer = true;

                    OptimizerConfig.EnableGrassOptimizer = true;
                    OptimizerConfig.DisableGrassShadows = true;
                    OptimizerConfig.DisableGrassWindShader = true;
                    OptimizerConfig.GrassShaderMaxLod = 400;
                    OptimizerConfig.GrassLodMultiplier = 2.5f;

                    OptimizerConfig.EnableSlimeRenderLod = true;
                    OptimizerConfig.SlimeSkinLodDistance = 8f;
                    OptimizerConfig.SlimeShadowCullDistance = 10f;

                    OptimizerConfig.EnableRubberBoneLod = true;
                    OptimizerConfig.RubberLodDistance = 5f;
                    OptimizerConfig.RubberDisableDistance = 12f;
                    OptimizerConfig.RubberLodInterval = 4;

                    OptimizerConfig.EnableSlimeAiLod = true;
                    OptimizerConfig.FarAiDistance = 14f;
                    OptimizerConfig.UltraFarAiDistance = 28f;

                    OptimizerConfig.EnableSlimeFaceEmotionLod = true;
                    OptimizerConfig.FaceEmotionLodDistance = 12f;
                    OptimizerConfig.FaceEmotionUltraDistance = 24f;

                    OptimizerConfig.EnableAnimatorCulling = true;
                    OptimizerConfig.EnablePhysicsOpt = true;
                    OptimizerConfig.PhysicsSolverIterations = 3;
                    OptimizerConfig.EnableKeepUprightOpt = true;
                    OptimizerConfig.EnableChickenAiLod = true;
                    OptimizerConfig.EnableSmallObjectShadowCull = true;
                    break;

                case OptimizerPreset.HighQuality:
                    if (IsGameContextReady())
                    {
                        SRQualitySettings.CurrentLevel = SRQualitySettings.Level.VERY_HIGH;
                    }

                    OptimizerConfig.EnableSmartQualityCaps = false;
                    OptimizerConfig.MaxShadowCascades = 4;
                    OptimizerConfig.MaxShadowResolution = 3;
                    OptimizerConfig.MaxShadowDistance = 150f;
                    OptimizerConfig.MaxAntiAliasing = 8;
                    OptimizerConfig.MaxPixelLightCount = 4;
                    OptimizerConfig.MaxLodBias = 2.0f;
                    OptimizerConfig.MaxGlobalShaderLod = -1;
                    OptimizerConfig.DisableSoftParticles = false;
                    OptimizerConfig.MaxParticleRaycastBudget = 1024;
                    OptimizerConfig.MaxAnisoMode = 2;

                    OptimizerConfig.EnableCameraLayerCull = false;
                    OptimizerConfig.EnableSmallObjectShadowCull = false;

                    OptimizerConfig.EnableSsaoOptimizer = true;
                    OptimizerConfig.SsaoDownsampling = 1;
                    OptimizerConfig.SsaoBlurDownsampling = false;
                    OptimizerConfig.SsaoUseGaussianBlur = false;
                    OptimizerConfig.EnableBloomOptimizer = false;

                    OptimizerConfig.EnableGrassOptimizer = true;
                    OptimizerConfig.DisableGrassShadows = true;
                    OptimizerConfig.DisableGrassWindShader = false;
                    OptimizerConfig.GrassLodMultiplier = 1.2f;

                    OptimizerConfig.EnableSlimeRenderLod = true;
                    OptimizerConfig.SlimeSkinLodDistance = 24f;
                    OptimizerConfig.SlimeShadowCullDistance = 36f;

                    OptimizerConfig.EnablePhysicsOpt = false;
                    OptimizerConfig.PhysicsSolverIterations = 6;
                    break;

                case OptimizerPreset.VanillaOff:
                    if (IsGameContextReady() && SRQualitySettings.CurrentLevel == SRQualitySettings.Level.LOWEST)
                    {
                        SRQualitySettings.CurrentLevel = SRQualitySettings.Level.HIGH;
                    }

                    OptimizerConfig.EnableRubberBoneLod = false;
                    OptimizerConfig.EnableSlimeRenderLod = false;
                    OptimizerConfig.EnableSmallObjectShadowCull = false;
                    OptimizerConfig.EnableSmartQualityCaps = false;
                    OptimizerConfig.EnableCameraLayerCull = false;
                    OptimizerConfig.EnableSlimeAiLod = false;
                    OptimizerConfig.EnableFastFoodSearch = false;
                    OptimizerConfig.EnableGrassOptimizer = false;
                    OptimizerConfig.DisableGrassShadows = false;
                    OptimizerConfig.DisableGrassWindShader = false;
                    OptimizerConfig.EnableSsaoOptimizer = false;
                    OptimizerConfig.EnableBloomOptimizer = false;
                    OptimizerConfig.EnableAnimatorCulling = false;
                    OptimizerConfig.EnablePhysicsOpt = false;
                    OptimizerConfig.EnableKeepUprightOpt = false;
                    OptimizerConfig.EnableChickenAiLod = false;
                    OptimizerConfig.EnableSlimeFaceEmotionLod = false;
                    OptimizerConfig.EnableExpressionCache = false;
                    OptimizerConfig.EnablePlortCollectorPool = false;
                    OptimizerConfig.EnableCellDirectorFastCount = false;
                    OptimizerConfig.EnableTrackCollisionsPool = false;
                    OptimizerConfig.EnableBillboardCameraCache = false;
                    OptimizerConfig.EnableEffectsComponentCache = false;
                    OptimizerConfig.EnablePlexerDelegateCache = false;
                    break;
            }

            ApplyRuntimeEffects();
            Save();
        }

        private static bool IsGameContextReady()
        {
            return SRSingleton<GameContext>.Instance != null &&
                   SRSingleton<GameContext>.Instance.OptionsDirector != null;
        }

        public static void ApplyQualityCapsOnly()
        {
            Patches.QualityCapsPatch.ReapplyQualitySettings();
        }

        public static void ApplyCameraCullOnly()
        {
            Patches.CameraCullDistancePatch.MarkDirty();
            Patches.CameraCullDistancePatch.UpdateActiveCamera();
        }

        public static void ApplyRuntimeEffects()
        {
            Patches.QualityCapsPatch.ReapplyQualitySettings();
            Patches.GrassOptimizerPatch.ReapplyAllScenes();
            Patches.AnimatorCullingPatch.ReapplyAll();
            Patches.CameraCullDistancePatch.MarkDirty();
            Patches.CameraCullDistancePatch.UpdateActiveCamera();
        }

        private static string FormatValue(object val)
        {
            if (val is bool b)
            {
                return b ? "true" : "false";
            }
            if (val is float f)
            {
                return f.ToString("R", CultureInfo.InvariantCulture);
            }
            if (val is int n)
            {
                return n.ToString(CultureInfo.InvariantCulture);
            }
            return val != null ? val.ToString() : string.Empty;
        }

        private static object ParseValue(Type type, string raw)
        {
            if (type == typeof(bool))
            {
                if (bool.TryParse(raw, out bool b))
                {
                    return b;
                }
                return null;
            }
            if (type == typeof(int))
            {
                if (int.TryParse(raw, NumberStyles.Integer, CultureInfo.InvariantCulture, out int n))
                {
                    return n;
                }
                return null;
            }
            if (type == typeof(float))
            {
                if (float.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out float f))
                {
                    return f;
                }
                return null;
            }
            return null;
        }
    }
}
