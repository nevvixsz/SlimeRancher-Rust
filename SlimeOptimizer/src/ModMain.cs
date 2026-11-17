using System.Reflection;
using SRML;
using SRML.Console;
using SlimeOptimizer.Commands;
using SlimeOptimizer.Patches;

namespace SlimeOptimizer
{
    public class ModMain : ModEntryPoint
    {
        internal static Console.ConsoleInstance Logger { get; private set; }

        public override void PreLoad()
        {
            Logger = ConsoleInstance;
            HarmonyInstance.PatchAll(Assembly.GetExecutingAssembly());
            Console.RegisterCommand(new SlimeOptCommand());
            Logger?.LogSuccess("SlimeOptimizer v0.2.0 initialized (Harmony patches applied). Use 'slimeopt' in console.");
        }

        public override void PostLoad()
        {
            QualityCapsPatch.ApplyCapsIfEnabled();
        }

        public override void Update()
        {
            CameraCache.EnsureCurrentFrame();
        }
    }
}
