using System.Collections.Generic;
using HarmonyLib;
using UnityEngine;
using UnityEngine.Rendering;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(Identifiable), "Awake")]
    internal static class SmallActorShadowPatch
    {
        private static readonly List<Renderer> TempRenderers = new List<Renderer>(8);

        [HarmonyPostfix]
        private static void Postfix(Identifiable __instance)
        {
            if (!OptimizerConfig.EnableSmallObjectShadowCull || __instance == null)
            {
                return;
            }

            Identifiable.Id id = __instance.id;
            if (!Identifiable.IsPlort(id) &&
                !Identifiable.IsFood(id) &&
                !Identifiable.IsCraft(id) &&
                !Identifiable.IsChick(id))
            {
                return;
            }

            TempRenderers.Clear();
            __instance.GetComponentsInChildren<Renderer>(true, TempRenderers);
            for (int i = 0; i < TempRenderers.Count; i++)
            {
                Renderer r = TempRenderers[i];
                if (r != null && r.shadowCastingMode != ShadowCastingMode.Off)
                {
                    r.shadowCastingMode = ShadowCastingMode.Off;
                    if (OptimizerConfig.EnableStatsCollection)
                    {
                        OptimizerStats.SmallActorShadowsCulled++;
                    }
                }
            }
            TempRenderers.Clear();
        }
    }
}
