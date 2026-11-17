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

        private static TimeDirector _cachedTimeDir;

        private static bool IsFastForwarding()
        {
            if (_cachedTimeDir == null && SRSingleton<SceneContext>.Instance != null)
            {
                _cachedTimeDir = SRSingleton<SceneContext>.Instance.TimeDirector;
            }
            return _cachedTimeDir != null && _cachedTimeDir.IsFastForwarding();
        }

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
            if (!OptimizerConfig.EnableSlimeAiLod)
            {
                return;
            }

            bool rethinkUpdated = ___nextRethinkTime > __state.Rethink;
            bool groundUpdated = ___nextGroundCheckTime > __state.Ground;
            if (!rethinkUpdated && !groundUpdated)
            {
                return;
            }

            if ((___vacuumable != null && ___vacuumable.isHeld()) || IsFastForwarding())
            {
                return;
            }

            CameraCache.EnsureCurrentFrame();
            if (!CameraCache.HasCamera)
            {
                return;
            }

            Vector3 pos = __instance.transform.position;
            Vector3 cam = CameraCache.Position;
            float dx = pos.x - cam.x;
            float dy = pos.y - cam.y;
            float dz = pos.z - cam.z;
            float sqrDist = dx * dx + dy * dy + dz * dz;

            float ultraDist = OptimizerConfig.UltraFarAiDistance;
            float farDist = OptimizerConfig.FarAiDistance;

            float rethinkInterval;
            float groundInterval;

            if (ultraDist > 0f && sqrDist >= ultraDist * ultraDist)
            {
                rethinkInterval = OptimizerConfig.UltraFarAiRethinkInterval;
                groundInterval = OptimizerConfig.UltraFarGroundCheckInterval;
            }
            else if (farDist > 0f && sqrDist >= farDist * farDist)
            {
                rethinkInterval = OptimizerConfig.FarAiRethinkInterval;
                groundInterval = OptimizerConfig.FarGroundCheckInterval;
            }
            else
            {
                return;
            }

            float jitter = (__instance.GetInstanceID() & 3) * 0.05f;
            float now = Time.fixedTime;

            if (rethinkUpdated && rethinkInterval > 1f)
            {
                ___nextRethinkTime = now + rethinkInterval + jitter;
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.SlimeAiSlowedRethinks++;
                }
            }

            if (groundUpdated && groundInterval > 0.25f)
            {
                ___nextGroundCheckTime = now + groundInterval + jitter * 0.5f;
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.SlimeAiSlowedGroundChecks++;
                }
            }
        }
    }
}
