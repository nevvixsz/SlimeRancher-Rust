using System.Collections.Generic;
using System.Runtime.CompilerServices;
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

        private static readonly ConditionalWeakTable<Renderer, CachedFaceMaterials> Cache =
            new ConditionalWeakTable<Renderer, CachedFaceMaterials>();

        [HarmonyPatch("ClearAppearance")]
        [HarmonyPrefix]
        private static void ClearAppearancePrefix(List<SlimeAppearanceApplicator.FaceRenderer> ____faceRenderers)
        {
            if (____faceRenderers == null)
            {
                return;
            }

            for (int i = 0; i < ____faceRenderers.Count; i++)
            {
                Renderer r = ____faceRenderers[i].Renderer;
                if (r != null && Cache.TryGetValue(r, out CachedFaceMaterials entry))
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

            if (__instance.Appearance == null || __instance.Appearance.Face == null || ____faceRenderers == null)
            {
                return true;
            }

            SlimeExpressionFace expressionFace = __instance.Appearance.Face.GetExpressionFace(slimeExpression);

            for (int i = 0; i < ____faceRenderers.Count; i++)
            {
                SlimeAppearanceApplicator.FaceRenderer fr = ____faceRenderers[i];
                Renderer r = fr.Renderer;
                if (r == null)
                {
                    continue;
                }

                if (!Cache.TryGetValue(r, out CachedFaceMaterials entry))
                {
                    entry = new CachedFaceMaterials();
                    Cache.Add(r, entry);
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

            return false;
        }
    }
}
