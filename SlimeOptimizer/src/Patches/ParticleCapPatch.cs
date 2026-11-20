using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(SRBehaviour), nameof(SRBehaviour.PlayFX), new[] { typeof(GameObject) })]
    internal static class ParticleCapPatch
    {
        private sealed class PsEntry
        {
            public ParticleSystem System;
            public int OriginalMaxParticles;
            public bool Capped;
        }

        private static readonly ConditionalWeakTable<GameObject, PsEntry[]> Cache =
            new ConditionalWeakTable<GameObject, PsEntry[]>();

        private static void Prefix(GameObject fxObject)
        {
            if (fxObject == null)
            {
                return;
            }

            PsEntry[] entries = Cache.GetValue(fxObject, go =>
            {
                ParticleSystem[] systems = go.GetComponentsInChildren<ParticleSystem>(true);
                if (systems == null || systems.Length == 0)
                {
                    return new PsEntry[0];
                }

                PsEntry[] arr = new PsEntry[systems.Length];
                for (int i = 0; i < systems.Length; i++)
                {
                    arr[i] = new PsEntry
                    {
                        System = systems[i],
                        OriginalMaxParticles = systems[i].main.maxParticles,
                        Capped = false
                    };
                }
                return arr;
            });

            if (entries.Length == 0)
            {
                return;
            }

            bool capEnabled = OptimizerConfig.EnableParticleCap;
            int cap = Mathf.Max(8, OptimizerConfig.MaxParticlesPerSystem);

            for (int i = 0; i < entries.Length; i++)
            {
                PsEntry e = entries[i];
                if (e.System == null)
                {
                    continue;
                }

                ParticleSystem.MainModule main = e.System.main;
                if (capEnabled)
                {
                    if (e.OriginalMaxParticles > cap && main.maxParticles != cap)
                    {
                        main.maxParticles = cap;
                        e.Capped = true;
                        if (OptimizerConfig.EnableStatsCollection)
                        {
                            OptimizerStats.ParticlesCapped++;
                        }
                    }
                    else if (main.maxParticles > cap)
                    {
                        main.maxParticles = cap;
                        e.Capped = true;
                    }
                }
                else if (e.Capped)
                {
                    main.maxParticles = e.OriginalMaxParticles;
                    e.Capped = false;
                }
            }
        }
    }
}
