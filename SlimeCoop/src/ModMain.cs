using SlimeCoop.Commands;
using SlimeCoop.Patches;
using SlimeCoop.UI;
using SlimeCoop.World;
using SRML;
using SRML.Console;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace SlimeCoop
{
    public class ModMain : ModEntryPoint
    {
        private static GameObject _driverGo;
        internal static SRML.Console.Console.ConsoleInstance LogConsole { get; private set; }

        public override void PreLoad()
        {
            LogConsole = ConsoleInstance;
            CoopLog.Initialize(Application.persistentDataPath);
            HarmonyInstance.PatchAll(typeof(ModMain).Assembly);
            CoopHarmonyPatches.Install(HarmonyInstance);
            CoopLog.Info("Harmony patches installed.");
        }

        public override void Load()
        {
            LogConsole = ConsoleInstance;
            SRML.Console.Console.RegisterCommand(new CoopCommand("coop"));
            SRML.Console.Console.RegisterCommand(new CoopCommand("кооп"));
            SRML.Console.Console.RegisterButton("coop.ui", new ConsoleButton("Coop: Меню (F9)", "coop ui"));
        }

        public override void PostLoad()
        {
            LogConsole = ConsoleInstance;
            ClientProfileStore.LoadLocalPrefs();
            EnsureDriver();
            SceneManager.sceneLoaded += OnSceneLoaded;
            LogConsole?.Log("SlimeCoop v1.0.0 загружен! Нажмите F9 для меню мультиплеера или введите 'coop'.");
        }

        private static void OnSceneLoaded(Scene scene, LoadSceneMode mode)
        {
            bool hasCtx = SRSingleton<SceneContext>.Instance != null;
            CoopLog.Info("SceneLoaded: '" + scene.name + "', hasSceneContext=" + hasCtx + ", role=" + CoopSession.Role);
            if (hasCtx && !string.Equals(scene.name, "MainMenu", System.StringComparison.OrdinalIgnoreCase))
            {
                CoopSession.OnGameSceneLoaded();
            }
        }

        private static void EnsureDriver()
        {
            if (_driverGo != null) return;
            _driverGo = new GameObject("SlimeCoopDriver");
            Object.DontDestroyOnLoad(_driverGo);
            _driverGo.AddComponent<CoopDriver>();
        }
    }

    internal sealed class CoopDriver : MonoBehaviour
    {
        private void Update()
        {
            CoopSession.TickPerFrame();
        }

        private void OnGUI()
        {
            Event ev = Event.current;
            if (ev != null && ev.type == EventType.KeyDown && ev.keyCode == KeyCode.F9)
            {
                CoopOverlayUI.Toggle();
                ev.Use();
            }
            CoopOverlayUI.Draw();
        }

        private void OnApplicationQuit()
        {
            CoopSession.Disconnect();
        }
    }
}
