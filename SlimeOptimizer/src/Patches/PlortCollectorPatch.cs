using System;
using System.Collections.Generic;
using System.Linq.Expressions;
using System.Reflection;
using System.Reflection.Emit;
using HarmonyLib;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(PlortCollector), "Update")]
    [HarmonyAfter("net.veesus.srml")]
    public static class PlortCollectorPatch
    {
        private static Type _jointListType;
        private static ConstructorInfo _jointListCtor;
        private static object _reusableList;
        private static Action _clearList;

        [HarmonyPrepare]
        private static bool Prepare()
        {
            Type jointRefType = typeof(PlortCollector).GetNestedType("JointReference", BindingFlags.NonPublic);
            if (jointRefType == null)
            {
                return false;
            }

            _jointListType = typeof(List<>).MakeGenericType(jointRefType);
            _jointListCtor = _jointListType.GetConstructor(Type.EmptyTypes);
            MethodInfo clearMethod = _jointListType.GetMethod("Clear", BindingFlags.Public | BindingFlags.Instance);
            if (_jointListCtor == null || clearMethod == null)
            {
                return false;
            }

            _reusableList = Activator.CreateInstance(_jointListType);
            _clearList = Expression.Lambda<Action>(
                Expression.Call(Expression.Constant(_reusableList, _jointListType), clearMethod)
            ).Compile();

            return true;
        }

        [HarmonyTranspiler]
        private static IEnumerable<CodeInstruction> Transpiler(IEnumerable<CodeInstruction> instructions)
        {
            MethodInfo rentMethod = AccessTools.Method(typeof(PlortCollectorPatch), nameof(RentJointList));

            foreach (CodeInstruction instr in instructions)
            {
                if (instr.opcode == OpCodes.Newobj &&
                    instr.operand is ConstructorInfo ci &&
                    ci == _jointListCtor)
                {
                    CodeInstruction callInstr = new CodeInstruction(OpCodes.Call, rentMethod);
                    callInstr.labels.AddRange(instr.labels);
                    callInstr.blocks.AddRange(instr.blocks);
                    yield return callInstr;
                    yield return new CodeInstruction(OpCodes.Castclass, _jointListType);
                    continue;
                }

                yield return instr;
            }
        }

        [HarmonyPostfix]
        private static void Postfix()
        {
            if (OptimizerConfig.EnablePlortCollectorPool && _clearList != null)
            {
                _clearList();
            }
        }

        public static object RentJointList()
        {
            if (OptimizerConfig.EnablePlortCollectorPool)
            {
                _clearList();
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.PlortCollectorListsReused++;
                }
                return _reusableList;
            }

            return Activator.CreateInstance(_jointListType);
        }
    }
}
