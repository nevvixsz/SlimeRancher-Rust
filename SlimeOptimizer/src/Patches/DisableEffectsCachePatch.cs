using System;
using System.Runtime.CompilerServices;
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
            public bool Initialized;
            public SSAOPro Ssao;
            public Behaviour Bloom;
            public Camera Camera;
        }

        private static readonly ConditionalWeakTable<DisableEffectsOnLowQuality, CachedEffects> Cache =
            new ConditionalWeakTable<DisableEffectsOnLowQuality, CachedEffects>();

        [HarmonyPrefix]
        private static bool Prefix(DisableEffectsOnLowQuality __instance, ref DepthTextureMode ___lastDepthMode)
        {
            if (!OptimizerConfig.EnableDisableEffectsCache)
            {
                return true;
            }

            if (!Cache.TryGetValue(__instance, out CachedEffects cached))
            {
                cached = new CachedEffects();
                Cache.Add(__instance, cached);
            }

            if (!cached.Initialized)
            {
                cached.Ssao = __instance.GetComponent<SSAOPro>();
                cached.Bloom = BloomType != null ? (__instance.GetComponent(BloomType) as Behaviour) : null;
                cached.Camera = __instance.GetComponent<Camera>();
                cached.Initialized = true;
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
                OptimizerStats.DisableEffectsGetComponentSaved += 3;
            }

            return false;
        }
    }
}
