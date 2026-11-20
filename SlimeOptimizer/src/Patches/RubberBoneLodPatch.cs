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
            public Transform Transform;
            public int Hash;
            public int NextDistCheckFrame = -1;
            public byte Tier; // 0 = active/near, 1 = lod, 2 = culled
            public RendererEntry[] Entries;
        }

        private static readonly FastObjCache<RubberBoneEffect, SlimeRenderGroup> Groups =
            new FastObjCache<RubberBoneEffect, SlimeRenderGroup>(512);

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
                    group = new SlimeRenderGroup
                    {
                        Transform = rbe.transform,
                        Hash = RuntimeHelpers.GetHashCode(rbe) & 0x7FFFFFFF
                    };
                    Groups.Set(rbe, group);
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
            bool rubberEnabled = OptimizerConfig.EnableRubberBoneLod && !ReferenceEquals(___verts, null);
            bool renderLodEnabled = OptimizerConfig.EnableSlimeRenderLod;

            if (!rubberEnabled && !renderLodEnabled)
            {
                return true;
            }

            if (___vacuumable != null && ___vacuumable.isHeld())
            {
                if (renderLodEnabled && Groups.TryGetValue(__instance, out SlimeRenderGroup heldGroup) && heldGroup.Entries != null)
                {
                    RestoreGroup(heldGroup);
                }
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.RubberActiveCalls++;
                }
                return true;
            }

            if (!Groups.TryGetValue(__instance, out SlimeRenderGroup group))
            {
                group = CreateGroup(__instance, ___ownRenderer);
            }

            int frame = CameraCache.FrameCount;
            if (CameraCache.HasCamera && frame >= group.NextDistCheckFrame)
            {
                group.NextDistCheckFrame = frame + 8 + (group.Hash & 3);
                float sqrDist = CameraCache.SqrDistanceToCamera(group.Transform);

                if (renderLodEnabled)
                {
                    ApplyRenderLod(group, sqrDist);
                }
                else if (group.Entries != null)
                {
                    RestoreGroup(group);
                }

                float disableDist = OptimizerConfig.RubberDisableDistance;
                float lodDist = OptimizerConfig.RubberLodDistance;
                if (disableDist > 0f && sqrDist >= disableDist * disableDist)
                {
                    group.Tier = 2;
                }
                else if (lodDist >= 0f && sqrDist >= lodDist * lodDist)
                {
                    group.Tier = 1;
                }
                else
                {
                    group.Tier = 0;
                }
            }

            if (!rubberEnabled)
            {
                return true;
            }

            byte tier = group.Tier;
            if (tier == 2)
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

            if (tier == 1)
            {
                int interval = OptimizerConfig.RubberLodInterval;
                if (interval > 1)
                {
                    int mask = interval >= 4 ? 3 : 1;
                    if (((frame + group.Hash) & mask) != 0)
                    {
                        if (OptimizerConfig.EnableStatsCollection)
                        {
                            OptimizerStats.RubberSkippedFrames++;
                        }
                        return false;
                    }
                }
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

            if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.RubberActiveCalls++;
            }

            return true;
        }

        private static SlimeRenderGroup CreateGroup(RubberBoneEffect rbe, Renderer fallbackRenderer)
        {
            RendererEntry[] entries = null;
            if (fallbackRenderer != null)
            {
                SkinnedMeshRenderer smr = fallbackRenderer as SkinnedMeshRenderer;
                entries = new[]
                {
                    new RendererEntry
                    {
                        Renderer = fallbackRenderer,
                        SkinnedRenderer = smr,
                        OriginalShadowMode = fallbackRenderer.shadowCastingMode,
                        OriginalSkinQuality = smr != null ? smr.quality : SkinQuality.Auto
                    }
                };
            }

            SlimeRenderGroup group = new SlimeRenderGroup
            {
                Transform = rbe.transform,
                Hash = RuntimeHelpers.GetHashCode(rbe) & 0x7FFFFFFF,
                NextDistCheckFrame = -1,
                Entries = entries
            };
            Groups.Set(rbe, group);
            return group;
        }

        private static void ApplyRenderLod(SlimeRenderGroup group, float sqrDist)
        {
            RendererEntry[] entries = group.Entries;
            if (entries == null)
            {
                return;
            }

            float shadowDist = OptimizerConfig.SlimeShadowCullDistance;
            bool shouldCullShadow = shadowDist > 0f && sqrDist >= shadowDist * shadowDist;

            float skinDist = OptimizerConfig.SlimeSkinLodDistance;
            bool shouldReduceSkin = skinDist > 0f && sqrDist >= skinDist * skinDist;

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

        private static void RestoreGroup(SlimeRenderGroup group)
        {
            RendererEntry[] entries = group.Entries;
            if (entries == null)
            {
                return;
            }

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
