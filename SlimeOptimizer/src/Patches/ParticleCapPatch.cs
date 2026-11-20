using System.Collections.Generic;
using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(SRBehaviour), nameof(SRBehaviour.PlayFX), new[] { typeof(GameObject) })]
    internal static class ParticleCapPatch
    {
        private struct PsSlot
        {
            public ParticleSystem System;
            public int OriginalMaxParticles;
        }

        private sealed class FxCacheEntry
        {
            public PsSlot[] Slots;
            public int AppliedCap;
            public bool AppliedEnabled;
        }

        private static readonly FastObjCache<GameObject, FxCacheEntry> Cache =
            new FastObjCache<GameObject, FxCacheEntry>(256);

        private static readonly List<ParticleSystem> TempSystems = new List<ParticleSystem>(8);
        private static readonly PsSlot[] EmptySlots = new PsSlot[0];
        private static bool _anyEverCapped;

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        private static void Prefix(GameObject fxObject)
        {
            if (ReferenceEquals(fxObject, null))
            {
                return;
            }

            bool capEnabled = OptimizerConfig.EnableParticleCap;
            if (!capEnabled && !_anyEverCapped)
            {
                return;
            }

            int cap = OptimizerConfig.MaxParticlesPerSystem;
            if (cap < 8)
            {
                cap = 8;
            }

            if (!Cache.TryGetValue(fxObject, out FxCacheEntry entry))
            {
                if (!capEnabled)
                {
                    return;
                }

                TempSystems.Clear();
                fxObject.GetComponentsInChildren<ParticleSystem>(true, TempSystems);
                int count = TempSystems.Count;
                PsSlot[] slots;
                if (count == 0)
                {
                    slots = EmptySlots;
                }
                else
                {
                    slots = new PsSlot[count];
                    for (int i = 0; i < count; i++)
                    {
                        ParticleSystem ps = TempSystems[i];
                        slots[i] = new PsSlot
                        {
                            System = ps,
                            OriginalMaxParticles = ps.main.maxParticles
                        };
                    }
                }
                TempSystems.Clear();

                entry = new FxCacheEntry
                {
                    Slots = slots,
                    AppliedCap = -1,
                    AppliedEnabled = false
                };
                Cache.Set(fxObject, entry);
            }

            if (entry.Slots.Length == 0)
            {
                return;
            }

            if (entry.AppliedEnabled == capEnabled && (!capEnabled || entry.AppliedCap == cap))
            {
                return;
            }

            PsSlot[] arr = entry.Slots;
            if (capEnabled)
            {
                _anyEverCapped = true;
                for (int i = 0; i < arr.Length; i++)
                {
                    ParticleSystem ps = arr[i].System;
                    if (ps == null)
                    {
                        continue;
                    }

                    if (arr[i].OriginalMaxParticles > cap)
                    {
                        ParticleSystem.MainModule main = ps.main;
                        main.maxParticles = cap;
                        if (OptimizerConfig.EnableStatsCollection)
                        {
                            OptimizerStats.ParticlesCapped++;
                        }
                    }
                }
                entry.AppliedEnabled = true;
                entry.AppliedCap = cap;
            }
            else if (entry.AppliedEnabled)
            {
                for (int i = 0; i < arr.Length; i++)
                {
                    ParticleSystem ps = arr[i].System;
                    if (ps != null)
                    {
                        ParticleSystem.MainModule main = ps.main;
                        main.maxParticles = arr[i].OriginalMaxParticles;
                    }
                }
                entry.AppliedEnabled = false;
                entry.AppliedCap = -1;
            }
        }
    }
}
