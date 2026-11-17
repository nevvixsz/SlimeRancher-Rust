using System;
using System.Collections.Generic;
using System.Text;
using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.SceneManagement;

namespace SlimeOptimizer.Patches
{
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

        private sealed class GrassShaderRecord
        {
            public Shader Shader;
            public string Name;
            public int OriginalMaxLod;
            public bool IsGeneric;
            public bool Capped;
        }

        private sealed class SceneGrassCache
        {
            public GrassRendererRecord[] Renderers;
            public GrassLodRecord[] LodGroups;
        }

        private static bool _initialized;
        private static readonly Dictionary<int, SceneGrassCache> SceneCaches = new Dictionary<int, SceneGrassCache>(32);
        private static readonly Dictionary<Shader, GrassShaderRecord> GrassShaders = new Dictionary<Shader, GrassShaderRecord>(8);

        private static readonly List<GameObject> RootBuffer = new List<GameObject>(128);
        private static readonly List<MeshRenderer> RendererBuffer = new List<MeshRenderer>(256);
        private static readonly List<LODGroup> LodGroupBuffer = new List<LODGroup>(128);
        private static readonly List<GrassRendererRecord> TempRendererRecords = new List<GrassRendererRecord>(128);
        private static readonly List<GrassLodRecord> TempLodRecords = new List<GrassLodRecord>(64);

        public static void Initialize()
        {
            if (_initialized)
            {
                return;
            }

            _initialized = true;
            SceneManager.sceneLoaded += OnSceneLoaded;
            SceneManager.sceneUnloaded += OnSceneUnloaded;
            ReapplyAllScenes();
        }

        private static void OnSceneLoaded(Scene scene, LoadSceneMode mode)
        {
            if (!scene.IsValid() || !scene.isLoaded)
            {
                return;
            }

            SceneGrassCache cache = GetOrBuildCache(scene);
            ApplyToCache(cache, OptimizerConfig.EnableGrassOptimizer);
            ApplyGrassShaders();
        }

        private static void OnSceneUnloaded(Scene scene)
        {
            SceneCaches.Remove(scene.handle);
        }

        public static void ReapplyAllSectors()
        {
            ReapplyAllScenes();
        }

        public static void ReapplyAllScenes()
        {
            bool enabled = OptimizerConfig.EnableGrassOptimizer;
            int count = SceneManager.sceneCount;
            for (int i = 0; i < count; i++)
            {
                Scene scene = SceneManager.GetSceneAt(i);
                if (scene.IsValid() && scene.isLoaded)
                {
                    SceneGrassCache cache = GetOrBuildCache(scene);
                    ApplyToCache(cache, enabled);
                }
            }

            ApplyGrassShaders();
        }

        public static string GetSeenShadersSummary()
        {
            if (GrassShaders.Count == 0)
            {
                return "none";
            }

            StringBuilder sb = new StringBuilder(64);
            foreach (GrassShaderRecord rec in GrassShaders.Values)
            {
                if (sb.Length > 0)
                {
                    sb.Append(", ");
                }
                sb.Append(rec.Name);
                if (rec.Capped && rec.Shader != null)
                {
                    sb.Append("[LOD=").Append(rec.Shader.maximumLOD).Append(']');
                }
                else if (rec.IsGeneric)
                {
                    sb.Append("[generic]");
                }
            }
            return sb.ToString();
        }

        private static SceneGrassCache GetOrBuildCache(Scene scene)
        {
            int handle = scene.handle;
            if (SceneCaches.TryGetValue(handle, out SceneGrassCache cache))
            {
                return cache;
            }

            RootBuffer.Clear();
            TempRendererRecords.Clear();
            TempLodRecords.Clear();

            scene.GetRootGameObjects(RootBuffer);
            for (int r = 0; r < RootBuffer.Count; r++)
            {
                GameObject root = RootBuffer[r];
                if (root == null)
                {
                    continue;
                }

                RendererBuffer.Clear();
                root.GetComponentsInChildren<MeshRenderer>(true, RendererBuffer);
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
                        TrackRendererShaders(mr);
                    }
                }

                LodGroupBuffer.Clear();
                root.GetComponentsInChildren<LODGroup>(true, LodGroupBuffer);
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
            }

            RootBuffer.Clear();
            RendererBuffer.Clear();
            LodGroupBuffer.Clear();

            cache = new SceneGrassCache
            {
                Renderers = TempRendererRecords.ToArray(),
                LodGroups = TempLodRecords.ToArray()
            };

            TempRendererRecords.Clear();
            TempLodRecords.Clear();
            SceneCaches[handle] = cache;
            return cache;
        }

        private static void TrackRendererShaders(MeshRenderer mr)
        {
            Material[] mats = mr.sharedMaterials;
            if (mats == null)
            {
                return;
            }

            for (int i = 0; i < mats.Length; i++)
            {
                Material mat = mats[i];
                if (mat == null)
                {
                    continue;
                }

                Shader shader = mat.shader;
                if (shader == null || GrassShaders.ContainsKey(shader))
                {
                    continue;
                }

                string name = shader.name ?? string.Empty;
                GrassShaders.Add(shader, new GrassShaderRecord
                {
                    Shader = shader,
                    Name = name,
                    OriginalMaxLod = shader.maximumLOD,
                    IsGeneric = IsGenericWorldShader(name),
                    Capped = false
                });
            }
        }

        private static void ApplyGrassShaders()
        {
            bool capEnabled = OptimizerConfig.EnableGrassOptimizer && OptimizerConfig.DisableGrassWindShader;
            int capLod = OptimizerConfig.GrassShaderMaxLod;

            foreach (GrassShaderRecord rec in GrassShaders.Values)
            {
                if (rec.Shader == null || rec.IsGeneric)
                {
                    continue;
                }

                if (capEnabled && capLod > 0)
                {
                    if (!rec.Capped || rec.Shader.maximumLOD != capLod)
                    {
                        if (rec.Shader.maximumLOD > capLod)
                        {
                            rec.Shader.maximumLOD = capLod;
                            if (!rec.Capped && OptimizerConfig.EnableStatsCollection)
                            {
                                OptimizerStats.GrassShadersCapped++;
                            }
                            rec.Capped = true;
                        }
                    }
                }
                else if (rec.Capped)
                {
                    rec.Shader.maximumLOD = rec.OriginalMaxLod;
                    rec.Capped = false;
                }
            }
        }

        private static bool IsGenericWorldShader(string name)
        {
            if (string.IsNullOrEmpty(name))
            {
                return true;
            }

            return string.Equals(name, "Standard", StringComparison.OrdinalIgnoreCase) ||
                   string.Equals(name, "Standard (Specular setup)", StringComparison.OrdinalIgnoreCase) ||
                   name.StartsWith("Hidden/", StringComparison.OrdinalIgnoreCase) ||
                   name.StartsWith("Particles/", StringComparison.OrdinalIgnoreCase) ||
                   name.StartsWith("UI/", StringComparison.OrdinalIgnoreCase) ||
                   name.StartsWith("Sprites/", StringComparison.OrdinalIgnoreCase) ||
                   name.StartsWith("Unlit/", StringComparison.OrdinalIgnoreCase) ||
                   name.StartsWith("Legacy Shaders/", StringComparison.OrdinalIgnoreCase);
        }

        private static void ApplyToCache(SceneGrassCache cache, bool enabled)
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
                   name.IndexOf("sprig", StringComparison.OrdinalIgnoreCase) >= 0 ||
                   name.IndexOf("drapeMoss", StringComparison.OrdinalIgnoreCase) >= 0 ||
                   name.IndexOf("fern", StringComparison.OrdinalIgnoreCase) >= 0;
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
                    Renderer r = first[i];
                    if (r == null)
                    {
                        continue;
                    }

                    if (IsGrassName(r.name))
                    {
                        return true;
                    }

                    MeshFilter mf = r.GetComponent<MeshFilter>();
                    if (mf != null && mf.sharedMesh != null && IsGrassName(mf.sharedMesh.name))
                    {
                        return true;
                    }
                }
            }

            return false;
        }
    }
}
