using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(SlimeSubbehaviourPlexer), "RegistryFixedUpdate")]
    internal static class SlimeAiLodPatch
    {
        private struct PrevTimes
        {
            public float Rethink;
            public float Ground;
        }

        private sealed class AiLodEntry
        {
            public Transform Transform;
            public int Hash;
            public int NextCheckFrame = -1;
            public byte Tier; // 0 = near, 1 = far, 2 = ultra
        }

        private static readonly FastObjCache<SlimeSubbehaviourPlexer, AiLodEntry> Cache =
            new FastObjCache<SlimeSubbehaviourPlexer, AiLodEntry>(512);

        [HarmonyPrefix]
        private static void Prefix(
            float ___nextRethinkTime,
            float ___nextGroundCheckTime,
            out PrevTimes __state)
        {
            __state.Rethink = ___nextRethinkTime;
            __state.Ground = ___nextGroundCheckTime;
        }

        [HarmonyPostfix]
        private static void Postfix(
            SlimeSubbehaviourPlexer __instance,
            Vacuumable ___vacuumable,
            ref float ___nextRethinkTime,
            ref float ___nextGroundCheckTime,
            PrevTimes __state)
        {
            if (!OptimizerConfig.EnableSlimeAiLod || !CameraCache.HasCamera)
            {
                return;
            }

            bool rethinkUpdated = ___nextRethinkTime > __state.Rethink;
            bool groundUpdated = ___nextGroundCheckTime > __state.Ground;
            if (!rethinkUpdated && !groundUpdated)
            {
                return;
            }

            if ((!ReferenceEquals(___vacuumable, null) && ___vacuumable.isHeld()) || CameraCache.IsFastForwarding)
            {
                return;
            }

            if (!Cache.TryGetValue(__instance, out AiLodEntry entry))
            {
                entry = new AiLodEntry
                {
                    Transform = __instance.transform,
                    Hash = FastObjCache<SlimeSubbehaviourPlexer, AiLodEntry>.MixHash(__instance),
                    NextCheckFrame = -1
                };
                Cache.Set(__instance, entry);
            }

            int frame = CameraCache.FrameCount;
            if (frame >= entry.NextCheckFrame)
            {
                entry.NextCheckFrame = frame + 16 + (entry.Hash & 7);
                float sqrDist = CameraCache.SqrDistanceToCamera(entry.Transform);
                float ultraDist = OptimizerConfig.UltraFarAiDistance;
                float farDist = OptimizerConfig.FarAiDistance;

                if (ultraDist > 0f && sqrDist >= ultraDist * ultraDist)
                {
                    entry.Tier = 2;
                }
                else if (farDist > 0f && sqrDist >= farDist * farDist)
                {
                    entry.Tier = 1;
                }
                else
                {
                    entry.Tier = 0;
                }
            }

            byte tier = entry.Tier;
            if (tier == 0)
            {
                return;
            }

            float rethinkInterval = tier == 2 ? OptimizerConfig.UltraFarAiRethinkInterval : OptimizerConfig.FarAiRethinkInterval;
            float groundInterval = tier == 2 ? OptimizerConfig.UltraFarGroundCheckInterval : OptimizerConfig.FarGroundCheckInterval;

            float jitter = (entry.Hash & 3) * 0.05f;
            float now = CameraCache.FixedTime;
            bool collectStats = OptimizerConfig.EnableStatsCollection;

            if (rethinkUpdated && rethinkInterval > 1f)
            {
                ___nextRethinkTime = now + rethinkInterval + jitter;
                if (collectStats)
                {
                    OptimizerStats.SlimeAiSlowedRethinks++;
                }
            }

            if (groundUpdated && groundInterval > 0.25f)
            {
                ___nextGroundCheckTime = now + groundInterval + jitter * 0.5f;
                if (collectStats)
                {
                    OptimizerStats.SlimeAiSlowedGroundChecks++;
                }
            }
        }
    }
}
