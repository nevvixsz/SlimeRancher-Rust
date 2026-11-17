using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;
using UnityStandardAssets.ImageEffects;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(DisableEffectsOnLowQuality), "CheckQuality")]
    internal static class DisableEffectsCachePatch
    {
        private sealed class CachedComponents
        {
            public SSAOPro Ssao;
            public Bloom Bloom;
            public Camera Camera;
        }

        private static readonly ConditionalWeakTable<DisableEffectsOnLowQuality, CachedComponents> Cache =
            new ConditionalWeakTable<DisableEffectsOnLowQuality, CachedComponents>();

        [HarmonyPrefix]
        private static bool Prefix(DisableEffectsOnLowQuality __instance, ref DepthTextureMode ___lastDepthMode)
        {
            if (!OptimizerConfig.EnableEffectsComponentCache)
            {
                return true;
            }

            if (!Cache.TryGetValue(__instance, out CachedComponents cached))
            {
                cached = new CachedComponents
                {
                    Ssao = __instance.GetComponent<SSAOPro>(),
                    Bloom = __instance.GetComponent<Bloom>(),
                    Camera = __instance.GetComponent<Camera>()
                };
                Cache.Add(__instance, cached);
            }

            if (cached.Ssao != null && cached.Ssao.enabled != SRQualitySettings.AmbientOcclusion)
            {
                cached.Ssao.enabled = SRQualitySettings.AmbientOcclusion;
            }

            if (cached.Bloom != null && cached.Bloom.enabled != SRQualitySettings.Bloom)
            {
                cached.Bloom.enabled = SRQualitySettings.Bloom;
            }

            DepthTextureMode mode = SRQualitySettings.GetDepthTextureMode();
            if (mode != ___lastDepthMode && cached.Camera != null)
            {
                cached.Camera.depthTextureMode = mode;
                ___lastDepthMode = mode;
            }

            if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.EffectsGetComponentsAvoided += 3;
            }

            return false;
        }
    }
}
