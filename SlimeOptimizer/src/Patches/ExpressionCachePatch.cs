using System.Collections.Generic;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(SlimeAppearanceApplicator))]
    internal static class ExpressionCachePatch
    {
        private sealed class CachedFaceMaterials
        {
            public Material[] Materials;
            public Material LastEyes;
            public Material LastMouth;
            public int EyeIndex;
            public int MouthIndex;
            public bool Initialized;
        }

        private sealed class ApplicatorExprState
        {
            public SlimeFace.SlimeExpression LastExpression;
            public SlimeAppearance LastAppearance;
            public bool HasExpression;
        }

        private static readonly FastObjCache<Renderer, CachedFaceMaterials> RendererCache =
            new FastObjCache<Renderer, CachedFaceMaterials>(512);

        private static readonly FastObjCache<SlimeAppearanceApplicator, ApplicatorExprState> ApplicatorCache =
            new FastObjCache<SlimeAppearanceApplicator, ApplicatorExprState>(256);

        [HarmonyPatch("ClearAppearance")]
        [HarmonyPrefix]
        private static void ClearAppearancePrefix(
            SlimeAppearanceApplicator __instance,
            List<SlimeAppearanceApplicator.FaceRenderer> ____faceRenderers)
        {
            if (ApplicatorCache.TryGetValue(__instance, out ApplicatorExprState appState))
            {
                appState.HasExpression = false;
                appState.LastAppearance = null;
            }

            if (____faceRenderers == null)
            {
                return;
            }

            for (int i = 0; i < ____faceRenderers.Count; i++)
            {
                Renderer r = ____faceRenderers[i].Renderer;
                if (r != null && RendererCache.TryGetValue(r, out CachedFaceMaterials entry))
                {
                    entry.Initialized = false;
                    entry.Materials = null;
                    entry.LastEyes = null;
                    entry.LastMouth = null;
                }
            }
        }

        [HarmonyPatch("SetExpression")]
        [HarmonyPrefix]
        private static bool SetExpressionPrefix(
            SlimeAppearanceApplicator __instance,
            SlimeFace.SlimeExpression slimeExpression,
            List<SlimeAppearanceApplicator.FaceRenderer> ____faceRenderers)
        {
            if (!OptimizerConfig.EnableExpressionCache)
            {
                return true;
            }

            SlimeAppearance appearance = __instance.Appearance;
            if (appearance == null || appearance.Face == null || ____faceRenderers == null)
            {
                return true;
            }

            if (!ApplicatorCache.TryGetValue(__instance, out ApplicatorExprState appState))
            {
                appState = new ApplicatorExprState();
                ApplicatorCache.Set(__instance, appState);
            }

            if (appState.HasExpression &&
                appState.LastExpression == slimeExpression &&
                ReferenceEquals(appState.LastAppearance, appearance))
            {
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.ExpressionNoOpsAvoided++;
                }
                return false;
            }

            SlimeExpressionFace expressionFace = appearance.Face.GetExpressionFace(slimeExpression);

            for (int i = 0; i < ____faceRenderers.Count; i++)
            {
                SlimeAppearanceApplicator.FaceRenderer fr = ____faceRenderers[i];
                Renderer r = fr.Renderer;
                if (r == null)
                {
                    continue;
                }

                if (!RendererCache.TryGetValue(r, out CachedFaceMaterials entry))
                {
                    entry = new CachedFaceMaterials();
                    RendererCache.Set(r, entry);
                }

                if (!entry.Initialized)
                {
                    Material[] shared = r.sharedMaterials;
                    if (shared == null || shared.Length == 0)
                    {
                        continue;
                    }

                    int eyeIdx = shared.Length - 2;
                    int mouthIdx = shared.Length - 1;
                    if (fr.ShowEyes != fr.ShowMouth)
                    {
                        eyeIdx = mouthIdx;
                    }

                    if (eyeIdx < 0 || mouthIdx < 0)
                    {
                        continue;
                    }

                    entry.Materials = shared;
                    entry.EyeIndex = eyeIdx;
                    entry.MouthIndex = mouthIdx;
                    entry.LastEyes = shared[eyeIdx];
                    entry.LastMouth = shared[mouthIdx];
                    entry.Initialized = true;
                }

                bool wantEyes = fr.ShowEyes && expressionFace.Eyes != null;
                bool wantMouth = fr.ShowMouth && expressionFace.Mouth != null;

                bool eyesMatch = !wantEyes || ReferenceEquals(entry.LastEyes, expressionFace.Eyes);
                bool mouthMatch = !wantMouth || ReferenceEquals(entry.LastMouth, expressionFace.Mouth);

                if (eyesMatch && mouthMatch)
                {
                    if (OptimizerConfig.EnableStatsCollection)
                    {
                        OptimizerStats.ExpressionNoOpsAvoided++;
                    }
                    continue;
                }

                if (wantEyes)
                {
                    entry.Materials[entry.EyeIndex] = expressionFace.Eyes;
                    entry.LastEyes = expressionFace.Eyes;
                }

                if (wantMouth)
                {
                    entry.Materials[entry.MouthIndex] = expressionFace.Mouth;
                    entry.LastMouth = expressionFace.Mouth;
                }

                r.sharedMaterials = entry.Materials;
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.ExpressionArrayAllocsAvoided++;
                }
            }

            appState.LastExpression = slimeExpression;
            appState.LastAppearance = appearance;
            appState.HasExpression = true;
            return false;
        }
    }
}
