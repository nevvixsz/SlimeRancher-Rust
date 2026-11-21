using System;
using HarmonyLib;
using SlimeDevConsole.Commands;
using SRML;
using SRML.Console;
using UnityEngine;

namespace SlimeDevConsole
{
    public class ModMain : ModEntryPoint
    {
        private static GameObject _driverObject;
        internal static SRML.Console.Console.ConsoleInstance LogConsole { get; private set; }

        public override void PreLoad()
        {
            LogConsole = ConsoleInstance;
            HarmonyInstance.PatchAll(typeof(ModMain).Assembly);
            DevPatches.Install(HarmonyInstance);
        }

        public override void Load()
        {
            LogConsole = ConsoleInstance;
            RegisterCommands();
            RegisterButtons();
        }

        private static void RegisterButtons()
        {
            SRML.Console.Console.RegisterButton("dev.god", new ConsoleButton("Dev: Бессмертие", "бессмертие"));
            SRML.Console.Console.RegisterButton("dev.fly", new ConsoleButton("Dev: Полёт", "полет"));
            SRML.Console.Console.RegisterButton("dev.vac", new ConsoleButton("Dev: Беск. вак", "вакуум"));
            SRML.Console.Console.RegisterButton("dev.help", new ConsoleButton("Dev: Помощь", "помощь"));
        }

        public override void PostLoad()
        {
            LogConsole = ConsoleInstance;
            RussianCatalog.EnsureInitialized();
            EnsureDriverObject();
            CommandHelpers.Log("v1.0.0 загружен! Напишите 'помощь' или 'devhelp' в консоли.");
        }

        private static void RegisterCommands()
        {
            SpawnExCommand spawnCmd = new SpawnExCommand();
            LargoCommand largoCmd = new LargoCommand();
            GiveExCommand giveCmd = new GiveExCommand();
            RefineryCommand refineryCmd = new RefineryCommand();

            FlyCommand flyCmd = new FlyCommand();
            SpeedCommand speedCmd = new SpeedCommand();
            GodCommand godCmd = new GodCommand();
            InfEnergyCommand energyCmd = new InfEnergyCommand();
            InfVacCommand vacCmd = new InfVacCommand();
            MoneyCommand moneyCmd = new MoneyCommand();
            HealCommand healCmd = new HealCommand();
            KeyCommand keyCmd = new KeyCommand();

            TeleportExCommand tpCmd = new TeleportExCommand();
            TimeExCommand timeCmd = new TimeExCommand();
            UnlockExCommand unlockCmd = new UnlockExCommand();
            MarketCommand marketCmd = new MarketCommand();
            InspectCommand inspectCmd = new InspectCommand();
            ClearWorldCommand clearCmd = new ClearWorldCommand();
            FindCatalogCommand findCmd = new FindCatalogCommand();
            DevHelpCommand helpCmd = new DevHelpCommand();

            DevCommandBase[] primaries = new DevCommandBase[]
            {
                spawnCmd, largoCmd, giveCmd, refineryCmd,
                flyCmd, speedCmd, godCmd, energyCmd, vacCmd, moneyCmd, healCmd, keyCmd,
                tpCmd, timeCmd, unlockCmd, marketCmd, inspectCmd, clearCmd, findCmd, helpCmd
            };

            for (int i = 0; i < primaries.Length; i++)
            {
                SRML.Console.Console.RegisterCommand(primaries[i]);
            }

            // Russian variants (ё/е and short forms) + English aliases (avoiding SRML built-ins)
            ConsoleCommand[] aliases = new ConsoleCommand[]
            {
                new AliasCommand("spawnex", spawnCmd),
                new AliasCommand("с", spawnCmd),
                new AliasCommand("largo", largoCmd),
                new AliasCommand("giveex", giveCmd),
                new AliasCommand("дать", giveCmd),
                new AliasCommand("refinery", refineryCmd),

                new AliasCommand("полёт", flyCmd),
                new AliasCommand("fly", flyCmd),
                new AliasCommand("speed", speedCmd),
                new AliasCommand("бог", godCmd),
                new AliasCommand("god", godCmd),
                new AliasCommand("infenergy", energyCmd),
                new AliasCommand("infvac", vacCmd),
                new AliasCommand("money", moneyCmd),
                new AliasCommand("heal", healCmd),
                new AliasCommand("key", keyCmd),

                new AliasCommand("тп", tpCmd),
                new AliasCommand("tp", tpCmd),
                new AliasCommand("time", timeCmd),
                new AliasCommand("unlock", unlockCmd),
                new AliasCommand("market", marketCmd),
                new AliasCommand("inspect", inspectCmd),
                new AliasCommand("clearworld", clearCmd),
                new AliasCommand("find", findCmd),
                new AliasCommand("devhelp", helpCmd)
            };

            for (int i = 0; i < aliases.Length; i++)
            {
                SRML.Console.Console.RegisterCommand(aliases[i]);
            }
        }

        private static void EnsureDriverObject()
        {
            if (_driverObject != null)
            {
                return;
            }
            _driverObject = new GameObject("SlimeDevConsoleDriver");
            UnityEngine.Object.DontDestroyOnLoad(_driverObject);
            _driverObject.AddComponent<DevConsoleDriver>();
        }
    }

    internal sealed class DevConsoleDriver : MonoBehaviour
    {
        private void Update()
        {
            DevState.TickPerFrame();
        }
    }
}
