using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;
using UnityEngine.Rendering;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(RubberBoneEffect), "LateUpdate")]
    internal static class RubberBoneLodPatch
    {
        private sealed class RenderLodState
        {
            public bool Initialized;
            public ShadowCastingMode OriginalShadowMode;
            public SkinQuality OriginalSkinQuality;
            public bool ShadowsDisabled;
            public bool ReducedSkinning;
        }

        private static readonly ConditionalWeakTable<Renderer, RenderLodState> RenderStates =
            new ConditionalWeakTable<Renderer, RenderLodState>();

        [HarmonyPrefix]
        private static bool Prefix(
            RubberBoneEffect __instance,
            Rigidbody ___ownBody,
            Renderer ___ownRenderer,
            Vacuumable ___vacuumable,
            object ___verts,
            ref bool ___sleeping,
            ref bool ___wasSleeping)
        {
            bool rubberEnabled = OptimizerConfig.EnableRubberBoneLod && ___verts != null;
            bool renderLodEnabled = OptimizerConfig.EnableSlimeRenderLod && ___ownRenderer != null;

            if (!rubberEnabled && !renderLodEnabled)
            {
                return true;
            }

            bool isHeld = ___vacuumable != null && ___vacuumable.isHeld();
            if (isHeld)
            {
                if (renderLodEnabled)
                {
                    RestoreRenderLod(___ownRenderer);
                }
                return true;
            }

            bool bodySleeping = ___ownBody != null && ___ownBody.IsSleeping();
            bool isVisible = ___ownRenderer == null || ___ownRenderer.isVisible;

            if (!isVisible)
            {
                return true;
            }

            CameraCache.EnsureCurrentFrame();
            if (!CameraCache.HasCamera)
            {
                return true;
            }

            Vector3 pos = __instance.transform.position;
            Vector3 cam = CameraCache.Position;
            float dx = pos.x - cam.x;
            float dy = pos.y - cam.y;
            float dz = pos.z - cam.z;
            float sqrDist = dx * dx + dy * dy + dz * dz;

            if (___ownRenderer != null && (((__instance.GetInstanceID() + Time.frameCount) & 7) == 0))
            {
                if (OptimizerConfig.EnableSlimeRenderLod)
                {
                    ApplyRenderLod(___ownRenderer, sqrDist);
                }
                else
                {
                    RestoreRenderLod(___ownRenderer);
                }
            }

            if (!rubberEnabled || bodySleeping)
            {
                return true;
            }

            float disableDist = OptimizerConfig.RubberDisableDistance;
            if (disableDist > 0f && sqrDist >= disableDist * disableDist)
            {
                if (!___sleeping)
                {
                    __instance.Reset();
                    ___sleeping = true;
                }

                ___wasSleeping = true;
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.RubberCulledCalls++;
                }

                return false;
            }

            int interval = OptimizerConfig.RubberLodInterval;
            float lodDist = OptimizerConfig.RubberLodDistance;
            if (interval > 1 && lodDist >= 0f && sqrDist >= lodDist * lodDist)
            {
                int phase = (__instance.GetInstanceID() & 0x7FFFFFFF) % interval;
                if ((Time.frameCount % interval) != phase)
                {
                    if (OptimizerConfig.EnableStatsCollection)
                    {
                        OptimizerStats.RubberSkippedFrames++;
                    }

                    return false;
                }
            }

            return true;
        }

        private static void ApplyRenderLod(Renderer renderer, float sqrDist)
        {
            if (!RenderStates.TryGetValue(renderer, out RenderLodState state))
            {
                state = new RenderLodState();
                RenderStates.Add(renderer, state);
            }

            SkinnedMeshRenderer smr = renderer as SkinnedMeshRenderer;
            if (!state.Initialized)
            {
                state.OriginalShadowMode = renderer.shadowCastingMode;
                state.OriginalSkinQuality = smr != null ? smr.quality : SkinQuality.Auto;
                state.Initialized = true;
            }

            float shadowDist = OptimizerConfig.SlimeShadowCullDistance;
            bool shouldCullShadow = shadowDist > 0f && sqrDist >= shadowDist * shadowDist;
            if (shouldCullShadow != state.ShadowsDisabled)
            {
                renderer.shadowCastingMode = shouldCullShadow ? ShadowCastingMode.Off : state.OriginalShadowMode;
                state.ShadowsDisabled = shouldCullShadow;
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.SlimeShadowChanges++;
                }
            }

            if (smr != null)
            {
                float skinDist = OptimizerConfig.SlimeSkinLodDistance;
                bool shouldReduceSkin = skinDist > 0f && sqrDist >= skinDist * skinDist;
                if (shouldReduceSkin != state.ReducedSkinning)
                {
                    smr.quality = shouldReduceSkin ? SkinQuality.Bone1 : state.OriginalSkinQuality;
                    state.ReducedSkinning = shouldReduceSkin;
                    if (OptimizerConfig.EnableStatsCollection)
                    {
                        OptimizerStats.SlimeSkinLodChanges++;
                    }
                }
            }
        }

        private static void RestoreRenderLod(Renderer renderer)
        {
            if (renderer == null || !RenderStates.TryGetValue(renderer, out RenderLodState state) || !state.Initialized)
            {
                return;
            }

            if (state.ShadowsDisabled)
            {
                renderer.shadowCastingMode = state.OriginalShadowMode;
                state.ShadowsDisabled = false;
            }

            if (state.ReducedSkinning && renderer is SkinnedMeshRenderer smr)
            {
                smr.quality = state.OriginalSkinQuality;
                state.ReducedSkinning = false;
            }
        }
    }
}
