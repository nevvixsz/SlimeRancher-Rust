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
            SettingsDatabase.LoadOrCreate();
            HarmonyInstance.PatchAll(Assembly.GetExecutingAssembly());
            Console.RegisterCommand(new SlimeOptCommand());
            Logger?.LogSuccess("SlimeOptimizer v0.9.2 pre-loaded (FastCache + Depth/Fog/Terrain + 20-bucket Profiler). Use 'slimeopt' in console.");
        }

        public override void Load()
        {
            QualityCapsPatch.Install(HarmonyInstance);
            BloomFastPatch.Install(HarmonyInstance);
        }

        public override void PostLoad()
        {
            QualityCapsPatch.ApplyCapsIfEnabled();
            QualityCapsPatch.ApplyPhysicsSettings();
            QualityCapsPatch.ApplyTargetFrameRate();
            GrassOptimizerPatch.Initialize();
            AnimatorCullingPatch.Initialize();
            CameraCullDistancePatch.MarkDirty();
            FpsHudOverlay.EnsureCreated();
        }

        public override void Update()
        {
            CameraCache.EnsureCurrentFrame();
            CameraCullDistancePatch.UpdateActiveCamera();
        }
    }
}
