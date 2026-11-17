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
            Logger?.LogSuccess("SlimeOptimizer v0.4.0 pre-loaded (Harmony patches applied). Use 'slimeopt' in console.");
        }

        public override void Load()
        {
            QualityCapsPatch.Install(HarmonyInstance);
        }

        public override void PostLoad()
        {
            QualityCapsPatch.ApplyCapsIfEnabled();
            GrassOptimizerPatch.Initialize();
            CameraCullDistancePatch.MarkDirty();
        }

        public override void Update()
        {
            CameraCache.EnsureCurrentFrame();
            CameraCullDistancePatch.UpdateActiveCamera();
        }
    }
}
