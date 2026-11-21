using System.Reflection;
using HarmonyLib;
using UnityEngine;

namespace SlimeDevConsole
{
    internal static class DevPatches
    {
        public static void Install(Harmony harmony)
        {
            if (harmony == null)
            {
                return;
            }

            Patch(harmony, typeof(PlayerState), nameof(PlayerState.Damage),
                prefix: nameof(DamagePrefix));
            Patch(harmony, typeof(PlayerState), nameof(PlayerState.CanBeDamaged),
                prefix: nameof(CanBeDamagedPrefix));
            Patch(harmony, typeof(PlayerState), nameof(PlayerState.AddRads),
                prefix: nameof(AddRadsPrefix));
            Patch(harmony, typeof(PlayerState), nameof(PlayerState.SpendEnergy),
                prefix: nameof(SpendEnergyPrefix));

            Patch(harmony, typeof(Ammo), nameof(Ammo.DecrementSelectedAmmo),
                prefix: nameof(VoidSkipIfInfVac));
            MethodInfo decById = AccessTools.Method(typeof(Ammo), nameof(Ammo.Decrement),
                new[] { typeof(Identifiable.Id), typeof(int) });
            if (decById != null)
            {
                harmony.Patch(decById, prefix: new HarmonyMethod(AccessTools.Method(typeof(DevPatches), nameof(VoidSkipIfInfVac))));
            }
            MethodInfo decByIdx = AccessTools.Method(typeof(Ammo), nameof(Ammo.Decrement),
                new[] { typeof(int), typeof(int) });
            if (decByIdx != null)
            {
                harmony.Patch(decByIdx, prefix: new HarmonyMethod(AccessTools.Method(typeof(DevPatches), nameof(VoidSkipIfInfVac))));
            }

            Patch(harmony, typeof(TimeDirector), nameof(TimeDirector.DeltaWorldTime),
                prefix: nameof(DeltaWorldTimePrefix));
        }

        private static void Patch(Harmony harmony, System.Type targetType, string methodName, string prefix)
        {
            MethodInfo m = AccessTools.Method(targetType, methodName);
            MethodInfo pre = AccessTools.Method(typeof(DevPatches), prefix);
            if (m != null && pre != null)
            {
                harmony.Patch(m, prefix: new HarmonyMethod(pre));
            }
        }

        private static bool DamagePrefix(ref bool __result)
        {
            if (DevState.GodMode)
            {
                __result = false;
                return false;
            }
            return true;
        }

        private static bool CanBeDamagedPrefix(ref bool __result)
        {
            if (DevState.GodMode)
            {
                __result = false;
                return false;
            }
            return true;
        }

        private static bool AddRadsPrefix(ref int __result)
        {
            if (DevState.GodMode)
            {
                __result = 0;
                return false;
            }
            return true;
        }

        private static bool SpendEnergyPrefix()
        {
            return !DevState.InfiniteEnergy;
        }

        private static bool VoidSkipIfInfVac()
        {
            return !DevState.InfiniteVac;
        }

        private static bool DeltaWorldTimePrefix(ref double __result)
        {
            if (DevState.FreezeTime)
            {
                __result = 0.0;
                return false;
            }
            return true;
        }
    }
}
