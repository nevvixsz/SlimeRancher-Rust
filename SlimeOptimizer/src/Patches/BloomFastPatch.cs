using System;
using System.Reflection;
using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    internal static class BloomFastPatch
    {
        private sealed class BloomOriginal
        {
            public int BlurIterations;
            public object Quality;
            public object Resolution;
            public bool Applied;
        }

        private static bool _installed;
        private static FieldInfo _blurIterField;
        private static FieldInfo _qualityField;
        private static FieldInfo _resolutionField;
        private static object _cheapQualityValue;
        private static object _lowResolutionValue;

        private static readonly ConditionalWeakTable<object, BloomOriginal> Originals =
            new ConditionalWeakTable<object, BloomOriginal>();

        public static void Install(Harmony harmony)
        {
            if (_installed || harmony == null)
            {
                return;
            }

            Type bloomType = AccessTools.TypeByName("UnityStandardAssets.ImageEffects.Bloom");
            if (bloomType == null)
            {
                return;
            }

            MethodInfo onRender = AccessTools.Method(bloomType, "OnRenderImage",
                new[] { typeof(RenderTexture), typeof(RenderTexture) });
            if (onRender == null)
            {
                return;
            }

            _blurIterField = AccessTools.Field(bloomType, "bloomBlurIterations") ??
                             AccessTools.Field(bloomType, "blurIterations");
            _qualityField = AccessTools.Field(bloomType, "quality");
            _resolutionField = AccessTools.Field(bloomType, "resolution");

            if (_qualityField != null && _qualityField.FieldType.IsEnum)
            {
                _cheapQualityValue = Enum.ToObject(_qualityField.FieldType, 0);
            }
            if (_resolutionField != null && _resolutionField.FieldType.IsEnum)
            {
                _lowResolutionValue = Enum.ToObject(_resolutionField.FieldType, 0);
            }

            MethodInfo prefix = AccessTools.Method(typeof(BloomFastPatch), nameof(Prefix));
            if (prefix != null)
            {
                harmony.Patch(onRender, prefix: new HarmonyMethod(prefix));
                _installed = true;
            }
        }

        private static void Prefix(object __instance)
        {
            if (__instance == null)
            {
                return;
            }

            BloomOriginal orig = Originals.GetValue(__instance, inst => new BloomOriginal
            {
                BlurIterations = _blurIterField != null ? (int)_blurIterField.GetValue(inst) : 2,
                Quality = _qualityField != null ? _qualityField.GetValue(inst) : null,
                Resolution = _resolutionField != null ? _resolutionField.GetValue(inst) : null,
                Applied = false
            });

            if (OptimizerConfig.EnableBloomOptimizer)
            {
                if (!orig.Applied)
                {
                    if (_blurIterField != null && (int)_blurIterField.GetValue(__instance) > 1)
                    {
                        _blurIterField.SetValue(__instance, 1);
                    }
                    if (_qualityField != null && _cheapQualityValue != null)
                    {
                        _qualityField.SetValue(__instance, _cheapQualityValue);
                    }
                    if (_resolutionField != null && _lowResolutionValue != null)
                    {
                        _resolutionField.SetValue(__instance, _lowResolutionValue);
                    }
                    orig.Applied = true;
                }

                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.BloomFramesOptimized++;
                }
            }
            else if (orig.Applied)
            {
                if (_blurIterField != null)
                {
                    _blurIterField.SetValue(__instance, orig.BlurIterations);
                }
                if (_qualityField != null && orig.Quality != null)
                {
                    _qualityField.SetValue(__instance, orig.Quality);
                }
                if (_resolutionField != null && orig.Resolution != null)
                {
                    _resolutionField.SetValue(__instance, orig.Resolution);
                }
                orig.Applied = false;
            }
        }
    }
}
