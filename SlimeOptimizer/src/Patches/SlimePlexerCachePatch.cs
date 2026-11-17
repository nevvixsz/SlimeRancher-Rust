using System;
using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(SlimeSubbehaviourPlexer), "RegistryFixedUpdate")]
    public static class SlimePlexerCachePatch
    {
        private static readonly MethodInfo OnGroundedMethod =
            AccessTools.Method(typeof(SlimeSubbehaviourPlexer), "OnGroundedRaycastResultReceived");

        private static readonly ConstructorInfo ActionCtor =
            typeof(Action<RaycastHit>).GetConstructor(new[] { typeof(object), typeof(IntPtr) });

        private static readonly MethodInfo GetCallbackMethod =
            AccessTools.Method(typeof(SlimePlexerCachePatch), nameof(GetGroundedCallback));

        private static readonly ConditionalWeakTable<SlimeSubbehaviourPlexer, Action<RaycastHit>> CallbackCache =
            new ConditionalWeakTable<SlimeSubbehaviourPlexer, Action<RaycastHit>>();

        [HarmonyPrepare]
        private static bool Prepare()
        {
            return OnGroundedMethod != null && ActionCtor != null && GetCallbackMethod != null;
        }

        [HarmonyTranspiler]
        private static IEnumerable<CodeInstruction> Transpiler(IEnumerable<CodeInstruction> instructions)
        {
            List<CodeInstruction> list = new List<CodeInstruction>(instructions);

            for (int i = 0; i < list.Count; i++)
            {
                if (i + 2 < list.Count &&
                    list[i].opcode == OpCodes.Ldarg_0 &&
                    list[i + 1].opcode == OpCodes.Ldftn &&
                    list[i + 1].operand is MethodInfo mi &&
                    mi == OnGroundedMethod &&
                    list[i + 2].opcode == OpCodes.Newobj &&
                    list[i + 2].operand is ConstructorInfo ci &&
                    ci == ActionCtor)
                {
                    yield return list[i];
                    yield return new CodeInstruction(OpCodes.Call, GetCallbackMethod);
                    i += 2;
                    continue;
                }

                yield return list[i];
            }
        }

        public static Action<RaycastHit> GetGroundedCallback(SlimeSubbehaviourPlexer plexer)
        {
            if (!OptimizerConfig.EnablePlexerDelegateCache || plexer == null)
            {
                return (Action<RaycastHit>)Delegate.CreateDelegate(typeof(Action<RaycastHit>), plexer, OnGroundedMethod);
            }

            if (!CallbackCache.TryGetValue(plexer, out Action<RaycastHit> callback))
            {
                callback = (Action<RaycastHit>)Delegate.CreateDelegate(typeof(Action<RaycastHit>), plexer, OnGroundedMethod);
                CallbackCache.Add(plexer, callback);
            }
            else if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.PlexerDelegatesReused++;
            }

            return callback;
        }
    }
}
