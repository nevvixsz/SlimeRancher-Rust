using System;
using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(SSAOPro), "OnRenderImage")]
    internal static class SsaoFastPatch
    {
        private const int PassClear = 0;
        private const int PassGaussianBlur = 5;
        private const int PassHighQualityBilateralBlur = 6;
        private const int PassComposite = 7;

        private sealed class SsaoState
        {
            public Camera Camera;
            public Material LastMaterial;
            public int LastSamples = -1;
        }

        private static readonly ConditionalWeakTable<SSAOPro, SsaoState> States =
            new ConditionalWeakTable<SSAOPro, SsaoState>();

        private static readonly string[][] SampleKeywords =
        {
            null,
            new[] { "SAMPLES_LOW" },
            new[] { "SAMPLES_MEDIUM" },
            new[] { "SAMPLES_HIGH" },
            new[] { "SAMPLES_ULTRA" }
        };

        private static readonly int PropInverseViewProject = Shader.PropertyToID("_InverseViewProject");
        private static readonly int PropCameraModelView = Shader.PropertyToID("_CameraModelView");
        private static readonly int PropNoiseTex = Shader.PropertyToID("_NoiseTex");
        private static readonly int PropParams1 = Shader.PropertyToID("_Params1");
        private static readonly int PropParams2 = Shader.PropertyToID("_Params2");
        private static readonly int PropOcclusionColor = Shader.PropertyToID("_OcclusionColor");
        private static readonly int PropBilateralThreshold = Shader.PropertyToID("_BilateralThreshold");
        private static readonly int PropDirection = Shader.PropertyToID("_Direction");
        private static readonly int PropSsaoTex = Shader.PropertyToID("_SSAOTex");

        private static bool Prefix(SSAOPro __instance, RenderTexture source, RenderTexture destination)
        {
            if (!OptimizerConfig.EnableSsaoOptimizer)
            {
                return true;
            }

            if (__instance.ShaderSSAO == null || Mathf.Approximately(__instance.Intensity, 0f))
            {
                Graphics.Blit(source, destination);
                return false;
            }

            SsaoState state = States.GetOrCreateValue(__instance);
            if (state.Camera == null)
            {
                state.Camera = __instance.GetComponent<Camera>();
                if (state.Camera == null)
                {
                    return true;
                }
            }

            Material mat = __instance.Material;
            if (mat == null)
            {
                return true;
            }

            int sampleIndex = (int)__instance.Samples;
            if (sampleIndex < 0 || sampleIndex >= SampleKeywords.Length)
            {
                sampleIndex = 0;
            }

            if (!ReferenceEquals(state.LastMaterial, mat) || state.LastSamples != sampleIndex)
            {
                mat.shaderKeywords = SampleKeywords[sampleIndex];
                state.LastMaterial = mat;
                state.LastSamples = sampleIndex;
            }
            else if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.SsaoKeywordResetsAvoided++;
            }

            int passNum = 0;
            if (__instance.NoiseTexture != null)
            {
                passNum = 1;
            }
            if (!Mathf.Approximately(__instance.LumContribution, 0f))
            {
                passNum += 2;
            }
            passNum++;

            Camera cam = state.Camera;
            mat.SetMatrix(PropInverseViewProject, (cam.projectionMatrix * cam.worldToCameraMatrix).inverse);
            mat.SetMatrix(PropCameraModelView, cam.cameraToWorldMatrix);
            mat.SetTexture(PropNoiseTex, __instance.NoiseTexture);
            mat.SetVector(PropParams1, new Vector4(
                __instance.NoiseTexture == null ? 0f : __instance.NoiseTexture.width,
                __instance.Radius,
                __instance.Intensity,
                __instance.Distance));
            mat.SetVector(PropParams2, new Vector4(
                __instance.Bias,
                __instance.LumContribution,
                __instance.CutoffDistance,
                __instance.CutoffFalloff));
            mat.SetColor(PropOcclusionColor, __instance.OcclusionColor);

            int downsample = Math.Max(__instance.Downsampling, Math.Max(1, OptimizerConfig.SsaoDownsampling));
            int rtW = Math.Max(1, source.width / downsample);
            int rtH = Math.Max(1, source.height / downsample);

            if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.SsaoDownsampledFrames++;
            }

            if (__instance.Blur != SSAOPro.BlurMode.None)
            {
                int blurPass = (OptimizerConfig.SsaoUseGaussianBlur || __instance.Blur != SSAOPro.BlurMode.HighQualityBilateral)
                    ? PassGaussianBlur
                    : PassHighQualityBilateralBlur;

                bool blurDownsample = __instance.BlurDownsampling || OptimizerConfig.SsaoBlurDownsampling;
                int blurDiv = blurDownsample ? downsample : 1;
                int blurW = Math.Max(1, source.width / blurDiv);
                int blurH = Math.Max(1, source.height / blurDiv);

                RenderTexture temporary = RenderTexture.GetTemporary(blurW, blurH, 0, RenderTextureFormat.ARGB32);
                RenderTexture temporary2 = RenderTexture.GetTemporary(rtW, rtH, 0, RenderTextureFormat.ARGB32);

                Graphics.Blit(temporary, temporary, mat, PassClear);
                Graphics.Blit(source, temporary, mat, passNum);

                mat.SetFloat(PropBilateralThreshold, __instance.BlurBilateralThreshold * 5f);
                Vector2 dirX = new Vector2(1f / source.width, 0f);
                Vector2 dirY = new Vector2(0f, 1f / source.height);

                int blurPasses = __instance.BlurPasses;
                for (int i = 0; i < blurPasses; i++)
                {
                    mat.SetVector(PropDirection, dirX);
                    Graphics.Blit(temporary, temporary2, mat, blurPass);
                    temporary.DiscardContents();

                    mat.SetVector(PropDirection, dirY);
                    Graphics.Blit(temporary2, temporary, mat, blurPass);
                    temporary2.DiscardContents();
                }

                if (!__instance.DebugAO)
                {
                    mat.SetTexture(PropSsaoTex, temporary);
                    Graphics.Blit(source, destination, mat, PassComposite);
                }
                else
                {
                    Graphics.Blit(temporary, destination);
                }

                RenderTexture.ReleaseTemporary(temporary);
                RenderTexture.ReleaseTemporary(temporary2);
                return false;
            }

            RenderTexture temporary3 = RenderTexture.GetTemporary(rtW, rtH, 0, RenderTextureFormat.ARGB32);
            Graphics.Blit(temporary3, temporary3, mat, PassClear);
            if (__instance.DebugAO)
            {
                Graphics.Blit(source, temporary3, mat, passNum);
                Graphics.Blit(temporary3, destination);
                RenderTexture.ReleaseTemporary(temporary3);
                return false;
            }

            Graphics.Blit(source, temporary3, mat, passNum);
            mat.SetTexture(PropSsaoTex, temporary3);
            Graphics.Blit(source, destination, mat, PassComposite);
            RenderTexture.ReleaseTemporary(temporary3);
            return false;
        }
    }
}
