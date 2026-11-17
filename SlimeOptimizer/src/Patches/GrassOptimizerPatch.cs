using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;
using UnityEngine.Rendering;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(SECTR_Sector), "OnEnable")]
    internal static class GrassOptimizerPatch
    {
        private sealed class GrassRendererRecord
        {
            public MeshRenderer Renderer;
            public ShadowCastingMode OriginalShadowMode;
            public bool ShadowsDisabled;
        }

        private sealed class GrassLodRecord
        {
            public LODGroup Group;
            public LOD[] OriginalLods;
            public bool LodAdjusted;
        }

        private sealed class SectorGrassCache
        {
            public GrassRendererRecord[] Renderers;
            public GrassLodRecord[] LodGroups;
        }

        private static readonly ConditionalWeakTable<SECTR_Sector, SectorGrassCache> SectorCaches =
            new ConditionalWeakTable<SECTR_Sector, SectorGrassCache>();

        private static readonly List<MeshRenderer> RendererBuffer = new List<MeshRenderer>(256);
        private static readonly List<LODGroup> LodGroupBuffer = new List<LODGroup>(128);
        private static readonly List<GrassRendererRecord> TempRendererRecords = new List<GrassRendererRecord>(128);
        private static readonly List<GrassLodRecord> TempLodRecords = new List<GrassLodRecord>(64);

        [HarmonyPostfix]
        private static void Postfix(SECTR_Sector __instance)
        {
            if (__instance == null)
            {
                return;
            }

            SectorGrassCache cache = GetOrBuildCache(__instance);
            ApplyToCache(cache, OptimizerConfig.EnableGrassOptimizer);
        }

        public static void ReapplyAllSectors()
        {
            List<SECTR_Sector> sectors = SECTR_Sector.All;
            if (sectors == null)
            {
                return;
            }

            bool enabled = OptimizerConfig.EnableGrassOptimizer;
            for (int i = 0; i < sectors.Count; i++)
            {
                SECTR_Sector sector = sectors[i];
                if (sector != null)
                {
                    SectorGrassCache cache = GetOrBuildCache(sector);
                    ApplyToCache(cache, enabled);
                }
            }
        }

        private static SectorGrassCache GetOrBuildCache(SECTR_Sector sector)
        {
            if (SectorCaches.TryGetValue(sector, out SectorGrassCache cache))
            {
                return cache;
            }

            RendererBuffer.Clear();
            TempRendererRecords.Clear();
            sector.GetComponentsInChildren<MeshRenderer>(true, RendererBuffer);

            for (int i = 0; i < RendererBuffer.Count; i++)
            {
                MeshRenderer mr = RendererBuffer[i];
                if (mr != null && IsGrassRenderer(mr))
                {
                    TempRendererRecords.Add(new GrassRendererRecord
                    {
                        Renderer = mr,
                        OriginalShadowMode = mr.shadowCastingMode
                    });
                }
            }
            RendererBuffer.Clear();

            LodGroupBuffer.Clear();
            TempLodRecords.Clear();
            sector.GetComponentsInChildren<LODGroup>(true, LodGroupBuffer);

            for (int i = 0; i < LodGroupBuffer.Count; i++)
            {
                LODGroup lg = LodGroupBuffer[i];
                if (lg != null && IsGrassLodGroup(lg))
                {
                    LOD[] lods = lg.GetLODs();
                    if (lods != null && lods.Length > 0)
                    {
                        TempLodRecords.Add(new GrassLodRecord
                        {
                            Group = lg,
                            OriginalLods = lods
                        });
                    }
                }
            }
            LodGroupBuffer.Clear();

            cache = new SectorGrassCache
            {
                Renderers = TempRendererRecords.ToArray(),
                LodGroups = TempLodRecords.ToArray()
            };

            TempRendererRecords.Clear();
            TempLodRecords.Clear();
            SectorCaches.Add(sector, cache);
            return cache;
        }

        private static void ApplyToCache(SectorGrassCache cache, bool enabled)
        {
            if (cache == null)
            {
                return;
            }

            bool disableShadows = enabled && OptimizerConfig.DisableGrassShadows;
            GrassRendererRecord[] renderers = cache.Renderers;
            if (renderers != null)
            {
                for (int i = 0; i < renderers.Length; i++)
                {
                    GrassRendererRecord rec = renderers[i];
                    if (rec.Renderer == null)
                    {
                        continue;
                    }

                    if (disableShadows)
                    {
                        if (!rec.ShadowsDisabled && rec.Renderer.shadowCastingMode != ShadowCastingMode.Off)
                        {
                            rec.Renderer.shadowCastingMode = ShadowCastingMode.Off;
                            rec.ShadowsDisabled = true;
                            if (OptimizerConfig.EnableStatsCollection)
                            {
                                OptimizerStats.GrassShadowsDisabled++;
                            }
                        }
                    }
                    else if (rec.ShadowsDisabled)
                    {
                        rec.Renderer.shadowCastingMode = rec.OriginalShadowMode;
                        rec.ShadowsDisabled = false;
                    }
                }
            }

            float mult = OptimizerConfig.GrassLodMultiplier;
            bool adjustLod = enabled && mult > 1.01f;
            GrassLodRecord[] lodGroups = cache.LodGroups;
            if (lodGroups != null)
            {
                for (int i = 0; i < lodGroups.Length; i++)
                {
                    GrassLodRecord rec = lodGroups[i];
                    if (rec.Group == null || rec.OriginalLods == null)
                    {
                        continue;
                    }

                    if (adjustLod)
                    {
                        if (!rec.LodAdjusted)
                        {
                            LOD[] orig = rec.OriginalLods;
                            LOD[] adjusted = new LOD[orig.Length];
                            float maxH = 0.95f;
                            for (int j = 0; j < orig.Length; j++)
                            {
                                adjusted[j] = orig[j];
                                float h = Mathf.Min(maxH, orig[j].screenRelativeTransitionHeight * mult);
                                adjusted[j].screenRelativeTransitionHeight = h;
                                maxH = Mathf.Max(0.001f, h - 0.005f);
                            }
                            rec.Group.SetLODs(adjusted);
                            rec.LodAdjusted = true;
                            if (OptimizerConfig.EnableStatsCollection)
                            {
                                OptimizerStats.GrassLodGroupsAdjusted++;
                            }
                        }
                    }
                    else if (rec.LodAdjusted)
                    {
                        rec.Group.SetLODs(rec.OriginalLods);
                        rec.LodAdjusted = false;
                    }
                }
            }
        }

        private static bool IsGrassName(string name)
        {
            if (string.IsNullOrEmpty(name))
            {
                return false;
            }

            return name.IndexOf("grass", StringComparison.OrdinalIgnoreCase) >= 0 ||
                   name.IndexOf("sprig", StringComparison.OrdinalIgnoreCase) >= 0;
        }

        private static bool IsGrassRenderer(MeshRenderer mr)
        {
            if (IsGrassName(mr.gameObject.name))
            {
                return true;
            }

            MeshFilter mf = mr.GetComponent<MeshFilter>();
            if (mf != null && mf.sharedMesh != null && IsGrassName(mf.sharedMesh.name))
            {
                return true;
            }

            return false;
        }

        private static bool IsGrassLodGroup(LODGroup lg)
        {
            if (IsGrassName(lg.gameObject.name))
            {
                return true;
            }

            LOD[] lods = lg.GetLODs();
            if (lods != null && lods.Length > 0 && lods[0].renderers != null)
            {
                Renderer[] first = lods[0].renderers;
                for (int i = 0; i < first.Length; i++)
                {
                    if (first[i] != null && IsGrassName(first[i].name))
                    {
                        return true;
                    }
                }
            }

            return false;
        }
    }
}
