using System;
using System.Reflection;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(SRQualitySettings), nameof(SRQualitySettings.GetDepthTextureMode))]
    internal static class DepthModeOptPatch
    {
        [HarmonyPostfix]
        private static void Postfix(ref DepthTextureMode __result)
        {
            if (OptimizerConfig.DisableStylizedFogAndDepth && !SRQualitySettings.AmbientOcclusion)
            {
                __result = DepthTextureMode.None;
            }
        }
    }

    [HarmonyPatch(typeof(StylizedFog), "OnRenderImage")]
    internal static class StylizedFogOptPatch
    {
        [HarmonyPrefix]
        private static bool Prefix(RenderTexture source, RenderTexture destination)
        {
            if (!OptimizerConfig.DisableStylizedFogAndDepth)
            {
                return true;
            }

            Graphics.Blit(source, destination);
            if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.FogPassesSkipped++;
            }
            return false;
        }
    }

    [HarmonyPatch(typeof(SECTR_CullingCamera), "OnPreCull")]
    internal static class SectrShadowCullOptPatch
    {
        private sealed class SectrOrig
        {
            public bool CullShadows;
        }

        private static readonly FastObjCache<SECTR_CullingCamera, SectrOrig> Cache =
            new FastObjCache<SECTR_CullingCamera, SectrOrig>(8);

        private static SECTR_CullingCamera _lastCam;
        private static SectrOrig _lastOrig;

        [HarmonyPrefix]
        private static void Prefix(SECTR_CullingCamera __instance)
        {
            if (ReferenceEquals(__instance, null))
            {
                return;
            }

            SectrOrig orig;
            if (ReferenceEquals(__instance, _lastCam))
            {
                orig = _lastOrig;
            }
            else
            {
                if (!Cache.TryGetValue(__instance, out orig))
                {
                    orig = new SectrOrig { CullShadows = __instance.CullShadows };
                    Cache.Set(__instance, orig);
                }
                _lastCam = __instance;
                _lastOrig = orig;
            }

            bool disableShadows = OptimizerConfig.DisableSectrShadowCulling &&
                                  QualitySettings.shadowCascades == 0 &&
                                  QualitySettings.shadowDistance <= 25f;

            bool target = disableShadows ? false : orig.CullShadows;
            if (__instance.CullShadows != target)
            {
                __instance.CullShadows = target;
            }
        }
    }

    internal static class RenderExtraPatches
    {
        public static void SyncCameraDepthAndFog()
        {
            Camera cam = CameraCache.MainCamera;
            if (cam == null)
            {
                return;
            }

            bool disable = OptimizerConfig.DisableStylizedFogAndDepth;
            StylizedFog fog = cam.GetComponent<StylizedFog>();
            if (fog != null)
            {
                if (disable && fog.enabled)
                {
                    fog.enabled = false;
                }
                else if (!disable && !fog.enabled)
                {
                    fog.enabled = true;
                }
            }

            DepthTextureMode mode = SRQualitySettings.GetDepthTextureMode();
            if (cam.depthTextureMode != mode)
            {
                cam.depthTextureMode = mode;
            }
        }
    }

    internal static class TerrainOptPatch
    {
        private sealed class TerrainOrig
        {
            public float DetailDist;
            public float DetailDensity;
            public float BasemapDist;
            public float PixelError;
            public bool CastShadows;
            public bool Applied;
        }

        private static bool _resolved;
        private static Type _terrainType;
        private static PropertyInfo _activeTerrainsProp;
        private static PropertyInfo _detailDistProp;
        private static PropertyInfo _detailDensityProp;
        private static PropertyInfo _basemapDistProp;
        private static PropertyInfo _pixelErrorProp;
        private static PropertyInfo _castShadowsProp;

        private static readonly FastObjCache<object, TerrainOrig> Originals =
            new FastObjCache<object, TerrainOrig>(16);

        private static void EnsureReflection()
        {
            if (_resolved)
            {
                return;
            }
            _resolved = true;

            _terrainType = AccessTools.TypeByName("UnityEngine.Terrain");
            if (_terrainType == null)
            {
                return;
            }

            _activeTerrainsProp = _terrainType.GetProperty("activeTerrains", BindingFlags.Public | BindingFlags.Static);
            _detailDistProp = _terrainType.GetProperty("detailObjectDistance", BindingFlags.Public | BindingFlags.Instance);
            _detailDensityProp = _terrainType.GetProperty("detailObjectDensity", BindingFlags.Public | BindingFlags.Instance);
            _basemapDistProp = _terrainType.GetProperty("basemapDistance", BindingFlags.Public | BindingFlags.Instance);
            _pixelErrorProp = _terrainType.GetProperty("heightmapPixelError", BindingFlags.Public | BindingFlags.Instance);
            _castShadowsProp = _terrainType.GetProperty("castShadows", BindingFlags.Public | BindingFlags.Instance);
        }

        public static void ApplyToAllTerrains()
        {
            EnsureReflection();
            if (_terrainType == null || _activeTerrainsProp == null)
            {
                return;
            }

            Array terrains = _activeTerrainsProp.GetValue(null, null) as Array;
            if (terrains == null || terrains.Length == 0)
            {
                return;
            }

            bool optimize = OptimizerConfig.OptimizeTerrain;
            bool aggressive = OptimizerConfig.MaxShadowCascades == 0 && OptimizerConfig.MaxShadowDistance <= 25f;

            for (int i = 0; i < terrains.Length; i++)
            {
                object t = terrains.GetValue(i);
                if (t == null)
                {
                    continue;
                }

                if (!Originals.TryGetValue(t, out TerrainOrig orig))
                {
                    orig = new TerrainOrig
                    {
                        DetailDist = _detailDistProp != null ? (float)_detailDistProp.GetValue(t, null) : 80f,
                        DetailDensity = _detailDensityProp != null ? (float)_detailDensityProp.GetValue(t, null) : 1f,
                        BasemapDist = _basemapDistProp != null ? (float)_basemapDistProp.GetValue(t, null) : 1000f,
                        PixelError = _pixelErrorProp != null ? (float)_pixelErrorProp.GetValue(t, null) : 5f,
                        CastShadows = _castShadowsProp != null && (bool)_castShadowsProp.GetValue(t, null),
                        Applied = false
                    };
                    Originals.Set(t, orig);
                }

                if (optimize)
                {
                    float targetDetailDist = aggressive ? Mathf.Min(orig.DetailDist, 25f) : Mathf.Min(orig.DetailDist, 45f);
                    float targetDetailDensity = aggressive ? Mathf.Min(orig.DetailDensity, 0.25f) : Mathf.Min(orig.DetailDensity, 0.5f);
                    float targetBasemap = aggressive ? Mathf.Min(orig.BasemapDist, 60f) : Mathf.Min(orig.BasemapDist, 180f);
                    float targetPixelErr = aggressive ? Mathf.Max(orig.PixelError, 16f) : Mathf.Max(orig.PixelError, 9f);

                    if (_detailDistProp != null) _detailDistProp.SetValue(t, targetDetailDist, null);
                    if (_detailDensityProp != null) _detailDensityProp.SetValue(t, targetDetailDensity, null);
                    if (_basemapDistProp != null) _basemapDistProp.SetValue(t, targetBasemap, null);
                    if (_pixelErrorProp != null) _pixelErrorProp.SetValue(t, targetPixelErr, null);
                    if (_castShadowsProp != null && aggressive) _castShadowsProp.SetValue(t, false, null);

                    if (!orig.Applied && OptimizerConfig.EnableStatsCollection)
                    {
                        OptimizerStats.TerrainsOptimized++;
                    }
                    orig.Applied = true;
                }
                else if (orig.Applied)
                {
                    if (_detailDistProp != null) _detailDistProp.SetValue(t, orig.DetailDist, null);
                    if (_detailDensityProp != null) _detailDensityProp.SetValue(t, orig.DetailDensity, null);
                    if (_basemapDistProp != null) _basemapDistProp.SetValue(t, orig.BasemapDist, null);
                    if (_pixelErrorProp != null) _pixelErrorProp.SetValue(t, orig.PixelError, null);
                    if (_castShadowsProp != null) _castShadowsProp.SetValue(t, orig.CastShadows, null);
                    orig.Applied = false;
                }
            }
        }
    }
}
