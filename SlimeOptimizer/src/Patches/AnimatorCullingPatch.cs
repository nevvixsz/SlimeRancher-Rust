using System.Collections.Generic;
using HarmonyLib;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace SlimeOptimizer.Patches
{
    internal static class AnimatorCullingPatch
    {
        private sealed class AnimatorRecord
        {
            public Animator Animator;
            public AnimatorCullingMode OriginalMode;
            public bool Culled;
        }

        private static bool _initialized;
        private static int _uiLayer = -1;
        private static int _weaponLayer = -1;
        private static int _heldLayer = -1;

        private static readonly Dictionary<int, AnimatorRecord[]> SceneAnimators =
            new Dictionary<int, AnimatorRecord[]>(32);

        private static readonly List<GameObject> RootBuffer = new List<GameObject>(128);
        private static readonly List<Animator> AnimatorBuffer = new List<Animator>(128);
        private static readonly List<AnimatorRecord> TempRecords = new List<AnimatorRecord>(128);

        public static void Initialize()
        {
            if (_initialized)
            {
                return;
            }

            _initialized = true;
            _uiLayer = LayerMask.NameToLayer("UI");
            _weaponLayer = LayerMask.NameToLayer("Weapon");
            _heldLayer = LayerMask.NameToLayer("Held");

            SceneManager.sceneLoaded += OnSceneLoaded;
            SceneManager.sceneUnloaded += OnSceneUnloaded;
            ReapplyAll();
        }

        private static void OnSceneLoaded(Scene scene, LoadSceneMode mode)
        {
            if (!scene.IsValid() || !scene.isLoaded)
            {
                return;
            }

            AnimatorRecord[] records = GetOrScanScene(scene);
            ApplyToRecords(records, OptimizerConfig.EnableAnimatorCulling);
        }

        private static void OnSceneUnloaded(Scene scene)
        {
            SceneAnimators.Remove(scene.handle);
        }

        public static void ReapplyAll()
        {
            bool enabled = OptimizerConfig.EnableAnimatorCulling;
            int count = SceneManager.sceneCount;
            for (int i = 0; i < count; i++)
            {
                Scene scene = SceneManager.GetSceneAt(i);
                if (scene.IsValid() && scene.isLoaded)
                {
                    AnimatorRecord[] records = GetOrScanScene(scene);
                    ApplyToRecords(records, enabled);
                }
            }
        }

        internal static void ApplyToSingle(Animator anim)
        {
            if (anim == null || !OptimizerConfig.EnableAnimatorCulling || IsProtectedAnimator(anim))
            {
                return;
            }

            if (anim.cullingMode == AnimatorCullingMode.AlwaysAnimate)
            {
                anim.cullingMode = AnimatorCullingMode.CullUpdateTransforms;
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.AnimatorsCulled++;
                }
            }
        }

        private static AnimatorRecord[] GetOrScanScene(Scene scene)
        {
            int handle = scene.handle;
            if (SceneAnimators.TryGetValue(handle, out AnimatorRecord[] cached))
            {
                return cached;
            }

            RootBuffer.Clear();
            TempRecords.Clear();
            scene.GetRootGameObjects(RootBuffer);

            for (int r = 0; r < RootBuffer.Count; r++)
            {
                GameObject root = RootBuffer[r];
                if (root == null)
                {
                    continue;
                }

                AnimatorBuffer.Clear();
                root.GetComponentsInChildren<Animator>(true, AnimatorBuffer);
                for (int i = 0; i < AnimatorBuffer.Count; i++)
                {
                    Animator anim = AnimatorBuffer[i];
                    if (anim == null || IsProtectedAnimator(anim))
                    {
                        continue;
                    }

                    TempRecords.Add(new AnimatorRecord
                    {
                        Animator = anim,
                        OriginalMode = anim.cullingMode
                    });
                }
            }

            RootBuffer.Clear();
            AnimatorBuffer.Clear();

            AnimatorRecord[] result = TempRecords.ToArray();
            TempRecords.Clear();
            SceneAnimators[handle] = result;
            return result;
        }

        private static void ApplyToRecords(AnimatorRecord[] records, bool enabled)
        {
            if (records == null)
            {
                return;
            }

            for (int i = 0; i < records.Length; i++)
            {
                AnimatorRecord rec = records[i];
                if (rec.Animator == null)
                {
                    continue;
                }

                if (enabled)
                {
                    if (!rec.Culled && rec.Animator.cullingMode == AnimatorCullingMode.AlwaysAnimate)
                    {
                        rec.Animator.cullingMode = AnimatorCullingMode.CullUpdateTransforms;
                        rec.Culled = true;
                        if (OptimizerConfig.EnableStatsCollection)
                        {
                            OptimizerStats.AnimatorsCulled++;
                        }
                    }
                }
                else if (rec.Culled)
                {
                    rec.Animator.cullingMode = rec.OriginalMode;
                    rec.Culled = false;
                }
            }
        }

        private static bool IsProtectedAnimator(Animator anim)
        {
            int layer = anim.gameObject.layer;
            if ((_uiLayer >= 0 && layer == _uiLayer) ||
                (_weaponLayer >= 0 && layer == _weaponLayer) ||
                (_heldLayer >= 0 && layer == _heldLayer))
            {
                return true;
            }

            return anim.GetComponentInParent<WeaponVacuum>() != null ||
                   anim.GetComponentInParent<PlayerState>() != null ||
                   anim.GetComponentInParent<Canvas>() != null;
        }
    }

    [HarmonyPatch(typeof(ChickenRandomMove), nameof(ChickenRandomMove.Awake))]
    internal static class ChickenAnimatorCullPatch
    {
        private static void Postfix(ChickenRandomMove __instance)
        {
            if (__instance == null || !OptimizerConfig.EnableAnimatorCulling)
            {
                return;
            }

            Animator anim = __instance.GetComponentInChildren<Animator>();
            AnimatorCullingPatch.ApplyToSingle(anim);
        }
    }
}
