using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;
using HarmonyLib;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(CellDirector), "Update")]
    public static class CellDirectorPatch
    {
        private static readonly MethodInfo FastCountMethod =
            AccessTools.Method(typeof(CellDirectorPatch), nameof(FastEntryCount));

        [HarmonyTranspiler]
        private static IEnumerable<CodeInstruction> Transpiler(IEnumerable<CodeInstruction> instructions)
        {
            foreach (CodeInstruction instr in instructions)
            {
                if (instr.opcode == OpCodes.Call &&
                    instr.operand is MethodInfo mi &&
                    mi.DeclaringType == typeof(Enumerable) &&
                    mi.Name == "Count" &&
                    mi.IsGenericMethod &&
                    mi.GetParameters().Length == 1)
                {
                    System.Type[] genericArgs = mi.GetGenericArguments();
                    if (genericArgs.Length == 1 && genericArgs[0] == typeof(GameObjectActorModelIdentifiableIndex.Entry))
                    {
                        CodeInstruction callInstr = new CodeInstruction(OpCodes.Call, FastCountMethod);
                        callInstr.labels.AddRange(instr.labels);
                        callInstr.blocks.AddRange(instr.blocks);
                        yield return callInstr;
                        continue;
                    }
                }

                yield return instr;
            }
        }

        public static int FastEntryCount(IEnumerable<GameObjectActorModelIdentifiableIndex.Entry> source)
        {
            if (OptimizerConfig.EnableCellDirectorFastCount &&
                source is ICollection<GameObjectActorModelIdentifiableIndex.Entry> collection)
            {
                if (OptimizerConfig.EnableStatsCollection)
                {
                    OptimizerStats.LinqCallsAvoided++;
                }

                return collection.Count;
            }

            return source.Count();
        }
    }
}
