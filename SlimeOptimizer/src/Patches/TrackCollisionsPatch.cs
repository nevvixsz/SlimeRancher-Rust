using System.Collections.Generic;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(TrackCollisions))]
    internal static class TrackCollisionsPatch
    {
        private const int MaxPoolSize = 256;
        private static readonly Stack<List<Collider>> ColliderListPool = new Stack<List<Collider>>(64);

        [HarmonyPatch("OnTriggerEnter")]
        [HarmonyPrefix]
        private static bool OnTriggerEnterPrefix(
            Collider other,
            Dictionary<GameObject, List<Collider>> ___currColliders,
            HashSet<GameObject> ___gameObjSet)
        {
            if (!OptimizerConfig.EnableTrackCollisionsPool || other == null)
            {
                return true;
            }

            GameObject go = other.gameObject;
            if (!___currColliders.TryGetValue(go, out List<Collider> list))
            {
                if (ColliderListPool.Count > 0)
                {
                    list = ColliderListPool.Pop();
                    if (OptimizerConfig.EnableStatsCollection)
                    {
                        OptimizerStats.ColliderListsReused++;
                    }
                }
                else
                {
                    list = new List<Collider>(4);
                }

                ___currColliders[go] = list;
            }

            list.Add(other);
            ___gameObjSet.Add(go);
            return false;
        }

        [HarmonyPatch("OnTriggerExit")]
        [HarmonyPrefix]
        private static bool OnTriggerExitPrefix(
            Collider other,
            Dictionary<GameObject, List<Collider>> ___currColliders,
            HashSet<GameObject> ___gameObjSet)
        {
            if (!OptimizerConfig.EnableTrackCollisionsPool || other == null)
            {
                return true;
            }

            GameObject go = other.gameObject;
            if (___currColliders.TryGetValue(go, out List<Collider> list))
            {
                list.Remove(other);
                if (list.Count == 0)
                {
                    ___currColliders.Remove(go);
                    ___gameObjSet.Remove(go);
                    if (ColliderListPool.Count < MaxPoolSize)
                    {
                        ColliderListPool.Push(list);
                    }
                }
            }

            return false;
        }

        [HarmonyPatch("CurrColliders")]
        [HarmonyPrefix]
        private static bool CurrCollidersPrefix(
            Dictionary<GameObject, List<Collider>> ___currColliders,
            HashSet<GameObject> ___gameObjSet,
            ref HashSet<GameObject> __result)
        {
            if (!OptimizerConfig.EnableTrackCollisionsPool)
            {
                return true;
            }

            if (___currColliders.Count == 0)
            {
                __result = ___gameObjSet;
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.TrackCollisionsEmptyFastPaths++;
                }
                return false;
            }

            return true;
        }
    }
}
