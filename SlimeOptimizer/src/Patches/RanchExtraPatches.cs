using System.Collections;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(GifRecorder))]
    internal static class GifRecorderOptPatch
    {
        [HarmonyPatch("Update")]
        [HarmonyPrefix]
        private static bool UpdatePrefix(
            ICollection ___frames,
            IList ___hooks,
            RenderTexture ___renderTex)
        {
            if (!OptimizerConfig.DisableGifRecorder)
            {
                return true;
            }

            OptionsDirector dir = SRSingleton<GameContext>.Instance != null
                ? SRSingleton<GameContext>.Instance.OptionsDirector
                : null;
            if (dir != null && dir.bufferForGif)
            {
                dir.bufferForGif = false;
            }

            if ((___frames != null && ___frames.Count > 0) ||
                (___hooks != null && ___hooks.Count > 0) ||
                ___renderTex != null)
            {
                // Run vanilla cleanup path once with bufferForGif=false
                return true;
            }

            return false;
        }

        [HarmonyPatch("DoRenderImage")]
        [HarmonyPrefix]
        private static bool DoRenderImagePrefix()
        {
            return !OptimizerConfig.DisableGifRecorder;
        }
    }

    [HarmonyPatch(typeof(RubberEffect), "LateUpdate")]
    internal static class RubberEffectLodPatch
    {
        private sealed class EffectEntry
        {
            public Transform Transform;
            public int Hash;
            public int NextCheckFrame = -1;
            public byte Tier;
        }

        private static readonly FastObjCache<RubberEffect, EffectEntry> Cache =
            new FastObjCache<RubberEffect, EffectEntry>(256);

        private static readonly Dictionary<int, bool> SavedRendererNotice = new Dictionary<int, bool>(1);

        [HarmonyPrefix]
        private static bool Prefix(
            RubberEffect __instance,
            Vacuumable ___vacuumable,
            Mesh ___originalMesh,
            Mesh ___workingMesh,
            ref bool ___sleeping,
            ref bool ___wasVisible)
        {
            if (!OptimizerConfig.EnableRubberEffectLod || !CameraCache.HasCamera)
            {
                return true;
            }

            if (___vacuumable != null && ___vacuumable.isHeld())
            {
                return true;
            }

            if (!Cache.TryGetValue(__instance, out EffectEntry entry))
            {
                entry = new EffectEntry
                {
                    Transform = __instance.transform,
                    Hash = RuntimeHelpers.GetHashCode(__instance) & 0x7FFFFFFF,
                    NextCheckFrame = -1
                };
                Cache.Set(__instance, entry);
            }

            int frame = CameraCache.FrameCount;
            if (frame >= entry.NextCheckFrame)
            {
                entry.NextCheckFrame = frame + 8 + (entry.Hash & 3);
                float sqrDist = CameraCache.SqrDistanceToCamera(entry.Transform);
                float disableDist = OptimizerConfig.RubberDisableDistance;
                float lodDist = OptimizerConfig.RubberLodDistance;

                if (disableDist > 0f && sqrDist >= disableDist * disableDist)
                {
                    entry.Tier = 2;
                }
                else if (lodDist >= 0f && sqrDist >= lodDist * lodDist)
                {
                    entry.Tier = 1;
                }
                else
                {
                    entry.Tier = 0;
                }
            }

            byte tier = entry.Tier;
            if (tier == 2)
            {
                if (!___sleeping)
                {
                    if (___workingMesh != null && ___originalMesh != null)
                    {
                        ___workingMesh.vertices = ___originalMesh.vertices;
                        ___workingMesh.RecalculateBounds();
                    }
                    ___sleeping = true;
                }
                ___wasVisible = false;
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.RubberEffectSkipped++;
                }
                return false;
            }

            if (tier == 1)
            {
                int interval = OptimizerConfig.RubberLodInterval;
                if (interval > 1)
                {
                    int mask = interval >= 4 ? 3 : 1;
                    if (((frame + entry.Hash) & mask) != 0)
                    {
                        if (OptimizerConfig.EnableStatsCollection)
                        {
                            OptimizerStats.RubberEffectSkipped++;
                        }
                        return false;
                    }
                }
            }

            return true;
        }
    }

    [HarmonyPatch(typeof(SlimeDragUpdater), nameof(SlimeDragUpdater.RegistryFixedUpdate))]
    internal static class SlimeDragOptPatch
    {
        private sealed class DragState
        {
            public bool Initialized;
            public bool WasCaptive;
        }

        private static readonly FastObjCache<SlimeDragUpdater, DragState> Cache =
            new FastObjCache<SlimeDragUpdater, DragState>(512);

        [HarmonyPrefix]
        private static bool Prefix(
            SlimeDragUpdater __instance,
            Vacuumable ___vacuumable,
            Rigidbody ___body)
        {
            if (!OptimizerConfig.EnableSlimeDragOpt || ___body == null)
            {
                return true;
            }

            bool isCaptive = ___vacuumable != null && ___vacuumable.isCaptive();

            if (!Cache.TryGetValue(__instance, out DragState state))
            {
                state = new DragState
                {
                    Initialized = true,
                    WasCaptive = isCaptive
                };
                Cache.Set(__instance, state);
                ___body.drag = isCaptive ? 0f : 0.5f;
                return false;
            }

            if (state.Initialized && state.WasCaptive == isCaptive)
            {
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.DragUpdatesSkipped++;
                }
                return false;
            }

            state.WasCaptive = isCaptive;
            ___body.drag = isCaptive ? 0f : 0.5f;
            return false;
        }
    }
}
