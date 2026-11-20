using System;
using System.Diagnostics;
using System.Reflection;
using System.Text;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer
{
    internal static class FrameProfiler
    {
        private const string ProfilerHarmonyId = "slimeoptimizer.frameprofiler";

        private enum Bucket
        {
            ActorFixed = 0,
            ActorUpdate = 1,
            ActorLate = 2,
            RaycastBatcher = 3,
            SlimePlexer = 4,
            RubberBone = 5,
            RubberEffect = 6,
            PlortCollector = 7,
            GifRecorder = 8,
            AudioProp = 9,
            Ssao = 10,
            Count = 11
        }

        private static readonly string[] BucketNames =
        {
            "ActorRegistry.FixedUpdate",
            "ActorRegistry.Update",
            "ActorRegistry.LateUpdate",
            "RaycastBatcher.FixedUpdate",
            "SlimePlexer.FixedUpdate",
            "RubberBoneEffect.LateUpdate",
            "RubberEffect.LateUpdate",
            "PlortCollector.Update",
            "GifRecorder.Update",
            "SECTR_Propagation.Update",
            "SSAOPro.OnRenderImage"
        };

        private static readonly long[] Ticks = new long[(int)Bucket.Count];
        private static readonly int[] Calls = new int[(int)Bucket.Count];
        private static readonly long[] ActiveStart = new long[(int)Bucket.Count];

        private static Harmony _harmony;
        private static float _remainingSeconds;
        private static float _elapsedSeconds;
        private static int _frames;
        private static readonly MethodInfo[] PatchedMethods = new MethodInfo[(int)Bucket.Count];

        public static bool IsRunning { get; private set; }
        public static string LastSummary { get; private set; } = "Not run yet (click button or run 'slimeopt profile')";

        public static void Start(float durationSeconds = 3.0f)
        {
            if (IsRunning)
            {
                return;
            }

            Array.Clear(Ticks, 0, Ticks.Length);
            Array.Clear(Calls, 0, Calls.Length);
            Array.Clear(ActiveStart, 0, ActiveStart.Length);
            _frames = 0;
            _elapsedSeconds = 0f;
            _remainingSeconds = Mathf.Clamp(durationSeconds, 1.0f, 10.0f);

            if (_harmony == null)
            {
                _harmony = new Harmony(ProfilerHarmonyId);
            }

            Hook(Bucket.ActorFixed, typeof(ActorRegistry), "FixedUpdate", nameof(Pre0), nameof(Post0));
            Hook(Bucket.ActorUpdate, typeof(ActorRegistry), "Update", nameof(Pre1), nameof(Post1));
            Hook(Bucket.ActorLate, typeof(ActorRegistry), "LateUpdate", nameof(Pre2), nameof(Post2));
            Hook(Bucket.RaycastBatcher, typeof(RaycastBatcher), "FixedUpdate", nameof(Pre3), nameof(Post3));
            Hook(Bucket.SlimePlexer, typeof(SlimeSubbehaviourPlexer), "RegistryFixedUpdate", nameof(Pre4), nameof(Post4));
            Hook(Bucket.RubberBone, typeof(RubberBoneEffect), "LateUpdate", nameof(Pre5), nameof(Post5));
            Hook(Bucket.RubberEffect, typeof(RubberEffect), "LateUpdate", nameof(Pre6), nameof(Post6));
            Hook(Bucket.PlortCollector, typeof(PlortCollector), "Update", nameof(Pre7), nameof(Post7));
            Hook(Bucket.GifRecorder, typeof(GifRecorder), "Update", nameof(Pre8), nameof(Post8));
            Hook(Bucket.AudioProp, typeof(SECTR_PropagationSource), "Update", nameof(Pre9), nameof(Post9));
            Hook(Bucket.Ssao, typeof(SSAOPro), "OnRenderImage", nameof(Pre10), nameof(Post10));

            IsRunning = true;
            LastSummary = "Profiling for " + _remainingSeconds.ToString("0.0") + "s...";
            ModMain.Logger?.Log("SlimeOptimizer: FrameProfiler started (" + _remainingSeconds.ToString("0.0") + "s)...");
        }

        private static void Hook(Bucket b, Type targetType, string methodName, string preName, string postName)
        {
            MethodInfo target = AccessTools.Method(targetType, methodName);
            if (target == null)
            {
                PatchedMethods[(int)b] = null;
                return;
            }

            MethodInfo pre = AccessTools.Method(typeof(FrameProfiler), preName);
            MethodInfo post = AccessTools.Method(typeof(FrameProfiler), postName);
            _harmony.Patch(target, prefix: new HarmonyMethod(pre), postfix: new HarmonyMethod(post));
            PatchedMethods[(int)b] = target;
        }

        public static void TickFrame(float dt)
        {
            if (!IsRunning)
            {
                return;
            }

            _frames++;
            _elapsedSeconds += dt;
            _remainingSeconds -= dt;

            if (_remainingSeconds <= 0f)
            {
                StopAndReport();
            }
        }

        private static void StopAndReport()
        {
            IsRunning = false;

            for (int i = 0; i < PatchedMethods.Length; i++)
            {
                MethodInfo m = PatchedMethods[i];
                if (m != null)
                {
                    _harmony.Unpatch(m, HarmonyPatchType.All, ProfilerHarmonyId);
                    PatchedMethods[i] = null;
                }
            }

            int frames = Math.Max(1, _frames);
            float avgFrameMs = (_elapsedSeconds / frames) * 1000f;
            float avgFps = _elapsedSeconds > 0.001f ? frames / _elapsedSeconds : 0f;
            double ticksToMs = 1000.0 / Stopwatch.Frequency;

            double sumTopLevelMs = 0.0;
            for (int i = 0; i <= 3; i++)
            {
                sumTopLevelMs += (Ticks[i] * ticksToMs) / frames;
            }
            for (int i = 5; i < (int)Bucket.Count; i++)
            {
                sumTopLevelMs += (Ticks[i] * ticksToMs) / frames;
            }

            double otherMs = Math.Max(0.0, avgFrameMs - sumTopLevelMs);

            StringBuilder sb = new StringBuilder(512);
            sb.AppendLine(string.Format(
                "=== FrameProfiler ({0} frames / {1:0.00}s => {2:0.0} FPS, {3:0.00} ms/frame) ===",
                frames, _elapsedSeconds, avgFps, avgFrameMs));

            for (int i = 0; i < (int)Bucket.Count; i++)
            {
                double msPerFrame = (Ticks[i] * ticksToMs) / frames;
                float callsPerFrame = (float)Calls[i] / frames;
                sb.AppendLine(string.Format("  {0,-28} {1,6:0.00} ms/f  ({2,6:0.1} calls/f)", BucketNames[i], msPerFrame, callsPerFrame));
            }
            sb.AppendLine(string.Format("  {0,-28} {1,6:0.00} ms/f", "PhysX/Render/UnityEngine", otherMs));

            LastSummary = string.Format(
                "Avg {0:0.0} FPS ({1:0.00} ms) | ActorF={2:0.00} U={3:0.00} L={4:0.00} | Plexer={5:0.00} | Rubber={6:0.00} | Ray={7:0.00} | Gif={8:0.00} | Render/PhysX={9:0.00} ms",
                avgFps,
                avgFrameMs,
                (Ticks[(int)Bucket.ActorFixed] * ticksToMs) / frames,
                (Ticks[(int)Bucket.ActorUpdate] * ticksToMs) / frames,
                (Ticks[(int)Bucket.ActorLate] * ticksToMs) / frames,
                (Ticks[(int)Bucket.SlimePlexer] * ticksToMs) / frames,
                (Ticks[(int)Bucket.RubberBone] * ticksToMs) / frames,
                (Ticks[(int)Bucket.RaycastBatcher] * ticksToMs) / frames,
                (Ticks[(int)Bucket.GifRecorder] * ticksToMs) / frames,
                otherMs);

            ModMain.Logger?.LogSuccess(sb.ToString());
        }

        private static void Begin(int idx)
        {
            ActiveStart[idx] = Stopwatch.GetTimestamp();
            Calls[idx]++;
        }

        private static void End(int idx)
        {
            Ticks[idx] += Stopwatch.GetTimestamp() - ActiveStart[idx];
        }

        private static void Pre0() => Begin(0);
        private static void Post0() => End(0);
        private static void Pre1() => Begin(1);
        private static void Post1() => End(1);
        private static void Pre2() => Begin(2);
        private static void Post2() => End(2);
        private static void Pre3() => Begin(3);
        private static void Post3() => End(3);
        private static void Pre4() => Begin(4);
        private static void Post4() => End(4);
        private static void Pre5() => Begin(5);
        private static void Post5() => End(5);
        private static void Pre6() => Begin(6);
        private static void Post6() => End(6);
        private static void Pre7() => Begin(7);
        private static void Post7() => End(7);
        private static void Pre8() => Begin(8);
        private static void Post8() => End(8);
        private static void Pre9() => Begin(9);
        private static void Post9() => End(9);
        private static void Pre10() => Begin(10);
        private static void Post10() => End(10);
    }
}
