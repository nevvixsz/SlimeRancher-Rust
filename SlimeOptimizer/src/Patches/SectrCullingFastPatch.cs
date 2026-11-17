using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using System.Runtime.CompilerServices;
using HarmonyLib;
using UnityEngine;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(SECTR_LOD), "SelectLOD")]
    internal static class SectrLodThrottlePatch
    {
        [HarmonyPrefix]
        private static bool Prefix(SECTR_LOD __instance, int ___activeLOD)
        {
            if (!OptimizerConfig.EnableSectrCullingOpt || ___activeLOD < 0)
            {
                return true;
            }

            int interval = OptimizerConfig.SectrLodInterval;
            if (interval > 1)
            {
                int phase = ((__instance.GetInstanceID() & 0x7FFFFFFF) + Time.frameCount) % interval;
                if (phase != 0)
                {
                    if (OptimizerConfig.EnableStatsCollection)
                    {
                        OptimizerStats.SectrLodCallsSkipped++;
                    }
                    return false;
                }
            }

            return true;
        }
    }

    [HarmonyPatch(typeof(SECTR_CullingCamera), "OnPreCull")]
    public static class SectrCullingFastPatch
    {
        private sealed class CachedComp<T> where T : Component
        {
            public bool Resolved;
            public T Value;
        }

        private static readonly Plane[] SharedPlanes = new Plane[6];

        private static readonly ConditionalWeakTable<Component, CachedComp<Camera>> CameraCacheTable =
            new ConditionalWeakTable<Component, CachedComp<Camera>>();

        private static readonly ConditionalWeakTable<Component, CachedComp<SECTR_Member>> MemberCacheTable =
            new ConditionalWeakTable<Component, CachedComp<SECTR_Member>>();

        private static readonly ConditionalWeakTable<Component, CachedComp<SECTR_CullingCamera>> CullingCamCacheTable =
            new ConditionalWeakTable<Component, CachedComp<SECTR_CullingCamera>>();

        private static readonly MethodInfo CalcPlanesMethod =
            AccessTools.Method(typeof(GeometryUtility), "CalculateFrustumPlanes", new[] { typeof(Camera) });

        private static readonly MethodInfo GetFrustumPlanesMethod =
            AccessTools.Method(typeof(SectrCullingFastPatch), nameof(GetFrustumPlanes));

        private static readonly MethodInfo GetCachedCameraMethod =
            AccessTools.Method(typeof(SectrCullingFastPatch), nameof(GetCachedCamera));

        private static readonly MethodInfo GetCachedMemberMethod =
            AccessTools.Method(typeof(SectrCullingFastPatch), nameof(GetCachedMember));

        private static readonly MethodInfo GetCachedCullingCamMethod =
            AccessTools.Method(typeof(SectrCullingFastPatch), nameof(GetCachedCullingCamera));

        [HarmonyPrepare]
        private static bool Prepare()
        {
            return CalcPlanesMethod != null &&
                   GetFrustumPlanesMethod != null &&
                   GetCachedCameraMethod != null &&
                   GetCachedMemberMethod != null &&
                   GetCachedCullingCamMethod != null;
        }

        [HarmonyTranspiler]
        private static IEnumerable<CodeInstruction> Transpiler(IEnumerable<CodeInstruction> instructions)
        {
            foreach (CodeInstruction instr in instructions)
            {
                if ((instr.opcode == OpCodes.Call || instr.opcode == OpCodes.Callvirt) &&
                    instr.operand is MethodInfo mi)
                {
                    if (mi == CalcPlanesMethod)
                    {
                        instr.opcode = OpCodes.Call;
                        instr.operand = GetFrustumPlanesMethod;
                    }
                    else if (mi.IsGenericMethod &&
                             mi.Name == "GetComponent" &&
                             mi.GetParameters().Length == 0 &&
                             mi.DeclaringType == typeof(Component))
                    {
                        System.Type argType = mi.GetGenericArguments()[0];
                        if (argType == typeof(Camera))
                        {
                            instr.opcode = OpCodes.Call;
                            instr.operand = GetCachedCameraMethod;
                        }
                        else if (argType == typeof(SECTR_Member))
                        {
                            instr.opcode = OpCodes.Call;
                            instr.operand = GetCachedMemberMethod;
                        }
                        else if (argType == typeof(SECTR_CullingCamera))
                        {
                            instr.opcode = OpCodes.Call;
                            instr.operand = GetCachedCullingCamMethod;
                        }
                    }
                }

                yield return instr;
            }
        }

        public static Plane[] GetFrustumPlanes(Camera cam)
        {
            if (!OptimizerConfig.EnableSectrCullingOpt)
            {
                return GeometryUtility.CalculateFrustumPlanes(cam);
            }

            GeometryUtility.CalculateFrustumPlanes(cam, SharedPlanes);
            if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.SectrFrustumPlanesSaved++;
            }
            return SharedPlanes;
        }

        public static Camera GetCachedCamera(Component self)
        {
            if (!OptimizerConfig.EnableSectrCullingOpt || self == null)
            {
                return self != null ? self.GetComponent<Camera>() : null;
            }

            if (!CameraCacheTable.TryGetValue(self, out CachedComp<Camera> entry))
            {
                entry = new CachedComp<Camera>();
                CameraCacheTable.Add(self, entry);
            }

            if (!entry.Resolved)
            {
                entry.Value = self.GetComponent<Camera>();
                entry.Resolved = true;
            }
            else if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.SectrGetComponentsAvoided++;
            }

            return entry.Value;
        }

        public static SECTR_Member GetCachedMember(Component self)
        {
            if (!OptimizerConfig.EnableSectrCullingOpt || self == null)
            {
                return self != null ? self.GetComponent<SECTR_Member>() : null;
            }

            if (!MemberCacheTable.TryGetValue(self, out CachedComp<SECTR_Member> entry))
            {
                entry = new CachedComp<SECTR_Member>();
                MemberCacheTable.Add(self, entry);
            }

            if (!entry.Resolved)
            {
                entry.Value = self.GetComponent<SECTR_Member>();
                entry.Resolved = true;
            }
            else if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.SectrGetComponentsAvoided++;
            }

            return entry.Value;
        }

        public static SECTR_CullingCamera GetCachedCullingCamera(Component self)
        {
            if (!OptimizerConfig.EnableSectrCullingOpt || self == null)
            {
                return self != null ? self.GetComponent<SECTR_CullingCamera>() : null;
            }

            if (!CullingCamCacheTable.TryGetValue(self, out CachedComp<SECTR_CullingCamera> entry))
            {
                entry = new CachedComp<SECTR_CullingCamera>();
                CullingCamCacheTable.Add(self, entry);
            }

            if (!entry.Resolved)
            {
                entry.Value = self.GetComponent<SECTR_CullingCamera>();
                entry.Resolved = true;
            }
            else if (OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.SectrGetComponentsAvoided++;
            }

            return entry.Value;
        }
    }
}
