using System.Collections.Generic;
using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;
using UnityEngine.Rendering;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(RubberBoneEffect), "LateUpdate")]
    internal static class RubberBoneLodPatch
    {
        private sealed class RendererEntry
        {
            public Renderer Renderer;
            public SkinnedMeshRenderer SkinnedRenderer;
            public ShadowCastingMode OriginalShadowMode;
            public SkinQuality OriginalSkinQuality;
            public bool ShadowsDisabled;
            public bool ReducedSkinning;
        }

        private sealed class SlimeRenderGroup
        {
            public RendererEntry[] Entries;
        }

        private static readonly ConditionalWeakTable<RubberBoneEffect, SlimeRenderGroup> Groups =
            new ConditionalWeakTable<RubberBoneEffect, SlimeRenderGroup>();

        private static readonly List<RendererEntry> BuildBuffer = new List<RendererEntry>(16);
        private static readonly HashSet<Renderer> SeenRenderers = new HashSet<Renderer>();

        internal static void RegisterApplicator(SlimeAppearanceApplicator applicator)
        {
            if (applicator == null || applicator.RootAppearanceObject == null || applicator.LODGroup == null)
            {
                return;
            }

            RubberBoneEffect rbe = applicator.RootAppearanceObject.GetComponent<RubberBoneEffect>();
            if (rbe == null)
            {
                return;
            }

            BuildBuffer.Clear();
            SeenRenderers.Clear();

            LOD[] lods = applicator.LODGroup.GetLODs();
            if (lods != null)
            {
                for (int i = 0; i < lods.Length; i++)
                {
                    Renderer[] renderers = lods[i].renderers;
                    if (renderers == null)
                    {
                        continue;
                    }

                    for (int j = 0; j < renderers.Length; j++)
                    {
                        Renderer r = renderers[j];
                        if (r != null && SeenRenderers.Add(r))
                        {
                            SkinnedMeshRenderer smr = r as SkinnedMeshRenderer;
                            BuildBuffer.Add(new RendererEntry
                            {
                                Renderer = r,
                                SkinnedRenderer = smr,
                                OriginalShadowMode = r.shadowCastingMode,
                                OriginalSkinQuality = smr != null ? smr.quality : SkinQuality.Auto
                            });
                        }
                    }
                }
            }

            if (BuildBuffer.Count > 0)
            {
                if (!Groups.TryGetValue(rbe, out SlimeRenderGroup group))
                {
                    group = new SlimeRenderGroup();
                    Groups.Add(rbe, group);
                }
                group.Entries = BuildBuffer.ToArray();
            }

            BuildBuffer.Clear();
            SeenRenderers.Clear();
        }

        internal static void ClearApplicator(SlimeAppearanceApplicator applicator)
        {
            if (applicator == null || applicator.RootAppearanceObject == null)
            {
                return;
            }

            RubberBoneEffect rbe = applicator.RootAppearanceObject.GetComponent<RubberBoneEffect>();
            if (rbe != null && Groups.TryGetValue(rbe, out SlimeRenderGroup group) && group.Entries != null)
            {
                RestoreGroup(group);
                group.Entries = null;
            }
        }

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
            bool renderLodEnabled = OptimizerConfig.EnableSlimeRenderLod;

            if (!rubberEnabled && !renderLodEnabled)
            {
                return true;
            }

            bool isHeld = ___vacuumable != null && ___vacuumable.isHeld();
            if (isHeld)
            {
                if (renderLodEnabled)
                {
                    RestoreForEffect(__instance, ___ownRenderer);
                }
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.RubberActiveCalls++;
                }
                return true;
            }

            CameraCache.EnsureCurrentFrame();
            bool hasCamera = CameraCache.HasCamera;
            float sqrDist = 0f;
            if (hasCamera)
            {
                Vector3 pos = __instance.transform.position;
                Vector3 cam = CameraCache.Position;
                float dx = pos.x - cam.x;
                float dy = pos.y - cam.y;
                float dz = pos.z - cam.z;
                sqrDist = dx * dx + dy * dy + dz * dz;

                if (((__instance.GetInstanceID() + Time.frameCount) & 7) == 0)
                {
                    if (renderLodEnabled)
                    {
                        ApplyForEffect(__instance, ___ownRenderer, sqrDist);
                    }
                    else
                    {
                        RestoreForEffect(__instance, ___ownRenderer);
                    }
                }
            }

            if (!rubberEnabled)
            {
                return true;
            }

            bool bodySleeping = ___ownBody != null && ___ownBody.IsSleeping();
            bool isVisible = ___ownRenderer == null || ___ownRenderer.isVisible;

            if (bodySleeping || !isVisible)
            {
                if (!isVisible && OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.RubberNotVisibleCalls++;
                }
                return true;
            }

            if (!hasCamera)
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

            if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.RubberActiveCalls++;
            }

            return true;
        }

        private static SlimeRenderGroup GetOrCreateFallbackGroup(RubberBoneEffect rbe, Renderer fallbackRenderer)
        {
            if (!Groups.TryGetValue(rbe, out SlimeRenderGroup group))
            {
                if (fallbackRenderer == null)
                {
                    return null;
                }

                SkinnedMeshRenderer smr = fallbackRenderer as SkinnedMeshRenderer;
                group = new SlimeRenderGroup
                {
                    Entries = new[]
                    {
                        new RendererEntry
                        {
                            Renderer = fallbackRenderer,
                            SkinnedRenderer = smr,
                            OriginalShadowMode = fallbackRenderer.shadowCastingMode,
                            OriginalSkinQuality = smr != null ? smr.quality : SkinQuality.Auto
                        }
                    }
                };
                Groups.Add(rbe, group);
            }

            return group;
        }

        private static void ApplyForEffect(RubberBoneEffect rbe, Renderer fallbackRenderer, float sqrDist)
        {
            SlimeRenderGroup group = GetOrCreateFallbackGroup(rbe, fallbackRenderer);
            if (group == null || group.Entries == null)
            {
                return;
            }

            float shadowDist = OptimizerConfig.SlimeShadowCullDistance;
            bool shouldCullShadow = shadowDist > 0f && sqrDist >= shadowDist * shadowDist;

            float skinDist = OptimizerConfig.SlimeSkinLodDistance;
            bool shouldReduceSkin = skinDist > 0f && sqrDist >= skinDist * skinDist;

            RendererEntry[] entries = group.Entries;
            for (int i = 0; i < entries.Length; i++)
            {
                RendererEntry entry = entries[i];
                if (entry.Renderer == null)
                {
                    continue;
                }

                if (shouldCullShadow != entry.ShadowsDisabled)
                {
                    entry.Renderer.shadowCastingMode = shouldCullShadow ? ShadowCastingMode.Off : entry.OriginalShadowMode;
                    entry.ShadowsDisabled = shouldCullShadow;
                    if (OptimizerConfig.EnableStatsCollection)
                    {
                        OptimizerStats.SlimeShadowChanges++;
                    }
                }

                if (entry.SkinnedRenderer != null && shouldReduceSkin != entry.ReducedSkinning)
                {
                    entry.SkinnedRenderer.quality = shouldReduceSkin ? SkinQuality.Bone1 : entry.OriginalSkinQuality;
                    entry.ReducedSkinning = shouldReduceSkin;
                    if (OptimizerConfig.EnableStatsCollection)
                    {
                        OptimizerStats.SlimeSkinLodChanges++;
                    }
                }
            }
        }

        private static void RestoreForEffect(RubberBoneEffect rbe, Renderer fallbackRenderer)
        {
            if (Groups.TryGetValue(rbe, out SlimeRenderGroup group) && group.Entries != null)
            {
                RestoreGroup(group);
                return;
            }

            if (fallbackRenderer != null)
            {
                // Nothing modified yet if not in Groups
            }
        }

        private static void RestoreGroup(SlimeRenderGroup group)
        {
            RendererEntry[] entries = group.Entries;
            for (int i = 0; i < entries.Length; i++)
            {
                RendererEntry entry = entries[i];
                if (entry.Renderer == null)
                {
                    continue;
                }

                if (entry.ShadowsDisabled)
                {
                    entry.Renderer.shadowCastingMode = entry.OriginalShadowMode;
                    entry.ShadowsDisabled = false;
                }

                if (entry.ReducedSkinning && entry.SkinnedRenderer != null)
                {
                    entry.SkinnedRenderer.quality = entry.OriginalSkinQuality;
                    entry.ReducedSkinning = false;
                }
            }
        }
    }

    [HarmonyPatch(typeof(SlimeAppearanceApplicator))]
    internal static class SlimeAppearanceLodTrackerPatch
    {
        [HarmonyPatch("ApplyAppearance")]
        [HarmonyPostfix]
        private static void ApplyAppearancePostfix(SlimeAppearanceApplicator __instance)
        {
            RubberBoneLodPatch.RegisterApplicator(__instance);
        }

        [HarmonyPatch("ClearAppearance")]
        [HarmonyPrefix]
        private static void ClearAppearancePrefix(SlimeAppearanceApplicator __instance)
        {
            RubberBoneLodPatch.ClearApplicator(__instance);
        }
    }
}
