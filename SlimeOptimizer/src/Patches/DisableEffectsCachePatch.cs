using System;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(DisableEffectsOnLowQuality), "CheckQuality")]
    internal static class DisableEffectsCachePatch
    {
        private static readonly Type BloomType = AccessTools.TypeByName("UnityStandardAssets.ImageEffects.Bloom");

        private sealed class CachedEffects
        {
            public SSAOPro Ssao;
            public Behaviour Bloom;
            public StylizedFog Fog;
            public Camera Camera;
        }

        private static readonly FastObjCache<DisableEffectsOnLowQuality, CachedEffects> Cache =
            new FastObjCache<DisableEffectsOnLowQuality, CachedEffects>(8);

        private static DisableEffectsOnLowQuality _lastKey;
        private static CachedEffects _lastVal;

        [HarmonyPrefix]
        private static bool Prefix(DisableEffectsOnLowQuality __instance, ref DepthTextureMode ___lastDepthMode)
        {
            if (!OptimizerConfig.EnableEffectsComponentCache)
            {
                return true;
            }

            CachedEffects cached;
            if (ReferenceEquals(__instance, _lastKey))
            {
                cached = _lastVal;
            }
            else
            {
                if (!Cache.TryGetValue(__instance, out cached))
                {
                    cached = new CachedEffects
                    {
                        Ssao = __instance.GetComponent<SSAOPro>(),
                        Bloom = BloomType != null ? (__instance.GetComponent(BloomType) as Behaviour) : null,
                        Fog = __instance.GetComponent<StylizedFog>(),
                        Camera = __instance.GetComponent<Camera>()
                    };
                    Cache.Set(__instance, cached);
                }
                _lastKey = __instance;
                _lastVal = cached;
            }

            bool ao = SRQualitySettings.AmbientOcclusion;
            if (cached.Ssao != null && cached.Ssao.enabled != ao)
            {
                cached.Ssao.enabled = ao;
            }

            bool bloom = SRQualitySettings.Bloom;
            if (cached.Bloom != null && cached.Bloom.enabled != bloom)
            {
                cached.Bloom.enabled = bloom;
            }

            bool fogEnabled = !OptimizerConfig.DisableStylizedFogAndDepth;
            if (cached.Fog != null && cached.Fog.enabled != fogEnabled)
            {
                cached.Fog.enabled = fogEnabled;
            }

            DepthTextureMode mode = SRQualitySettings.GetDepthTextureMode();
            if (mode != ___lastDepthMode)
            {
                if (cached.Camera != null)
                {
                    cached.Camera.depthTextureMode = mode;
                }
                ___lastDepthMode = mode;
            }

            if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.EffectsGetComponentsAvoided += 4;
            }

            return false;
        }
    }
}
