using System.Reflection;
using SRML;
using SRML.Console;
using SlimeOptimizer.Commands;

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
            Logger?.LogSuccess("SlimeOptimizer initialized (Harmony patches applied). Use 'slimeopt' in console.");
        }

        public override void Update()
        {
            CameraCache.EnsureCurrentFrame();
        }
    }
}
