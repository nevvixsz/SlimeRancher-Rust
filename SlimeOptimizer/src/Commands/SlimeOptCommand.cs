using System.Collections.Generic;
using SRML.Console;
using SlimeOptimizer.Patches;

namespace SlimeOptimizer.Commands
{
    public class SlimeOptCommand : ConsoleCommand
    {
        private static readonly List<string> Subcommands = new List<string>
        {
            "status",
            "stats",
            "profile",
            "reset",
            "preset",
            "toggle"
        };

        private static readonly List<string> Modules = new List<string>
        {
            "rubber",
            "rubbereffect",
            "drag",
            "gif",
            "fog",
            "sectrshadow",
            "terrain",
            "renderlod",
            "ailod",
            "facelod",
            "foodsearch",
            "grass",
            "grasswind",
            "ssao",
            "bloom",
            "particles",
            "timers",
            "animcull",
            "physopt",
            "hud",
            "cameracull",
            "smallshadows",
            "qcaps",
            "expr",
            "plort",
            "cell",
            "collisions",
            "billboard",
            "effects",
            "plexer",
            "stats"
        };

        public override string ID => "slimeopt";

        public override string Usage => "slimeopt <status|stats|profile|reset|preset <name>|toggle <module>>";

        public override string Description => "Shows SlimeOptimizer status/stats, runs 3s profiler, or toggles optimization modules.";

        public override string ExtendedDescription =>
            "Subcommands:\n" +
            "  status              - Show active modules and LOD/quality thresholds\n" +
            "  stats               - Show saved calls and avoided GC allocations\n" +
            "  profile             - Run a 3-second zero-overhead-afterwards frame profiler\n" +
            "  reset               - Reset statistics counters\n" +
            "  preset <name>       - Apply preset: maxfps, balanced, quality, vanilla\n" +
            "  toggle <module>     - Toggle: rubber, rubbereffect, drag, gif, fog, sectrshadow, terrain, renderlod, ailod, facelod, foodsearch, grass, grasswind, ssao, bloom, particles, timers, animcull, physopt, hud, cameracull, smallshadows, qcaps, expr, plort, cell, collisions, billboard, effects, plexer, stats";

        public override List<string> GetAutoComplete(int argIndex, string argText)
        {
            if (argIndex == 0)
            {
                return Subcommands;
            }
            if (argIndex == 1)
            {
                return Modules;
            }
            return null;
        }

        public override bool Execute(string[] args)
        {
            if (args == null || args.Length == 0 || args[0] == "status")
            {
                LogStatus();
                return true;
            }

            string sub = args[0].ToLowerInvariant();
            switch (sub)
            {
                case "stats":
                    LogStats();
                    return true;
                case "profile":
                    FrameProfiler.Start(3.0f);
                    Log("Started 3-second frame profiler. Results will print here when done.");
                    return true;
                case "reset":
                    OptimizerStats.Reset();
                    Log("SlimeOptimizer stats counters reset.");
                    return true;
                case "toggle":
                    if (args.Length < 2)
                    {
                        LogWarn("Usage: slimeopt toggle <rubber|renderlod|ailod|facelod|foodsearch|grass|grasswind|ssao|bloom|particles|timers|animcull|physopt|hud|cameracull|smallshadows|qcaps|expr|plort|cell|collisions|billboard|effects|plexer|stats>");
                        return false;
                    }
                    bool ok = ToggleModule(args[1].ToLowerInvariant());
                    if (ok)
                    {
                        SettingsDatabase.Save();
                    }
                    return ok;
                case "preset":
                    if (args.Length < 2)
                    {
                        LogWarn("Usage: slimeopt preset <maxfps|balanced|quality|vanilla>");
                        return false;
                    }
                    return ApplyPresetCommand(args[1].ToLowerInvariant());
                default:
                    LogWarn("Unknown subcommand. Usage: " + Usage);
                    return false;
            }
        }

        private static bool ApplyPresetCommand(string preset)
        {
            switch (preset)
            {
                case "maxfps":
                    SettingsDatabase.ApplyPreset(OptimizerPreset.MaxFps);
                    Log("Applied preset: MaxFps");
                    return true;
                case "balanced":
                    SettingsDatabase.ApplyPreset(OptimizerPreset.Balanced);
                    Log("Applied preset: Balanced");
                    return true;
                case "quality":
                    SettingsDatabase.ApplyPreset(OptimizerPreset.HighQuality);
                    Log("Applied preset: HighQuality");
                    return true;
                case "vanilla":
                    SettingsDatabase.ApplyPreset(OptimizerPreset.VanillaOff);
                    Log("Applied preset: VanillaOff");
                    return true;
                default:
                    LogWarn("Unknown preset. Use: maxfps, balanced, quality, vanilla");
                    return false;
            }
        }

        private static bool ToggleModule(string module)
        {
            switch (module)
            {
                case "rubber":
                    OptimizerConfig.EnableRubberBoneLod = !OptimizerConfig.EnableRubberBoneLod;
                    Log("RubberBoneLod = " + OptimizerConfig.EnableRubberBoneLod);
                    return true;
                case "rubbereffect":
                    OptimizerConfig.EnableRubberEffectLod = !OptimizerConfig.EnableRubberEffectLod;
                    Log("RubberEffectLod = " + OptimizerConfig.EnableRubberEffectLod);
                    return true;
                case "drag":
                    OptimizerConfig.EnableSlimeDragOpt = !OptimizerConfig.EnableSlimeDragOpt;
                    Log("SlimeDragOpt = " + OptimizerConfig.EnableSlimeDragOpt);
                    return true;
                case "gif":
                    OptimizerConfig.DisableGifRecorder = !OptimizerConfig.DisableGifRecorder;
                    Log("DisableGifRecorder = " + OptimizerConfig.DisableGifRecorder);
                    return true;
                case "fog":
                    OptimizerConfig.DisableStylizedFogAndDepth = !OptimizerConfig.DisableStylizedFogAndDepth;
                    RenderExtraPatches.SyncCameraDepthAndFog();
                    Log("DisableStylizedFogAndDepth = " + OptimizerConfig.DisableStylizedFogAndDepth);
                    return true;
                case "sectrshadow":
                    OptimizerConfig.DisableSectrShadowCulling = !OptimizerConfig.DisableSectrShadowCulling;
                    Log("DisableSectrShadowCulling = " + OptimizerConfig.DisableSectrShadowCulling);
                    return true;
                case "terrain":
                    OptimizerConfig.OptimizeTerrain = !OptimizerConfig.OptimizeTerrain;
                    TerrainOptPatch.ApplyToAllTerrains();
                    Log("OptimizeTerrain = " + OptimizerConfig.OptimizeTerrain);
                    return true;
                case "renderlod":
                    OptimizerConfig.EnableSlimeRenderLod = !OptimizerConfig.EnableSlimeRenderLod;
                    Log("SlimeRenderLod = " + OptimizerConfig.EnableSlimeRenderLod);
                    return true;
                case "ailod":
                    OptimizerConfig.EnableSlimeAiLod = !OptimizerConfig.EnableSlimeAiLod;
                    Log("SlimeAiLod = " + OptimizerConfig.EnableSlimeAiLod);
                    return true;
                case "facelod":
                    OptimizerConfig.EnableSlimeFaceEmotionLod = !OptimizerConfig.EnableSlimeFaceEmotionLod;
                    Log("SlimeFaceEmotionLod = " + OptimizerConfig.EnableSlimeFaceEmotionLod);
                    return true;
                case "foodsearch":
                    OptimizerConfig.EnableFastFoodSearch = !OptimizerConfig.EnableFastFoodSearch;
                    Log("FastFoodSearch = " + OptimizerConfig.EnableFastFoodSearch);
                    return true;
                case "grass":
                    OptimizerConfig.EnableGrassOptimizer = !OptimizerConfig.EnableGrassOptimizer;
                    GrassOptimizerPatch.ReapplyAllSectors();
                    Log("GrassOptimizer = " + OptimizerConfig.EnableGrassOptimizer + " (reapplied to loaded scenes)");
                    return true;
                case "grasswind":
                    OptimizerConfig.DisableGrassWindShader = !OptimizerConfig.DisableGrassWindShader;
                    GrassOptimizerPatch.ReapplyAllScenes();
                    Log("DisableGrassWindShader = " + OptimizerConfig.DisableGrassWindShader + " (shaders: " + GrassOptimizerPatch.GetSeenShadersSummary() + ")");
                    return true;
                case "ssao":
                    OptimizerConfig.EnableSsaoOptimizer = !OptimizerConfig.EnableSsaoOptimizer;
                    Log("SsaoOptimizer = " + OptimizerConfig.EnableSsaoOptimizer +
                        $" (downsample={OptimizerConfig.SsaoDownsampling}x, blurDown={OptimizerConfig.SsaoBlurDownsampling}, gaussian={OptimizerConfig.SsaoUseGaussianBlur})");
                    return true;
                case "bloom":
                    OptimizerConfig.EnableBloomOptimizer = !OptimizerConfig.EnableBloomOptimizer;
                    Log("BloomOptimizer = " + OptimizerConfig.EnableBloomOptimizer);
                    return true;
                case "particles":
                    OptimizerConfig.EnableParticleCap = !OptimizerConfig.EnableParticleCap;
                    Log("ParticleCap = " + OptimizerConfig.EnableParticleCap + " (max=" + OptimizerConfig.MaxParticlesPerSystem + ")");
                    return true;
                case "timers":
                    OptimizerConfig.EnableRanchTimersLod = !OptimizerConfig.EnableRanchTimersLod;
                    Log("RanchTimersLOD = " + OptimizerConfig.EnableRanchTimersLod + " (interval=" + OptimizerConfig.RanchTimersLodInterval + ")");
                    return true;
                case "animcull":
                    OptimizerConfig.EnableAnimatorCulling = !OptimizerConfig.EnableAnimatorCulling;
                    AnimatorCullingPatch.ReapplyAll();
                    Log("AnimatorCulling = " + OptimizerConfig.EnableAnimatorCulling);
                    return true;
                case "physopt":
                    OptimizerConfig.EnablePhysicsOpt = !OptimizerConfig.EnablePhysicsOpt;
                    QualityCapsPatch.ApplyPhysicsSettings();
                    Log("PhysicsOpt = " + OptimizerConfig.EnablePhysicsOpt + " (solverIter=" + UnityEngine.Physics.defaultSolverIterations + ")");
                    return true;
                case "hud":
                    OptimizerConfig.ShowFpsHud = !OptimizerConfig.ShowFpsHud;
                    Log("ShowFpsHud = " + OptimizerConfig.ShowFpsHud);
                    return true;
                case "cameracull":
                    OptimizerConfig.EnableCameraLayerCull = !OptimizerConfig.EnableCameraLayerCull;
                    CameraCullDistancePatch.MarkDirty();
                    Log("CameraLayerCull = " + OptimizerConfig.EnableCameraLayerCull +
                        $" (actorCull={OptimizerConfig.LayerActorCullDistance}m, maxFarClip={OptimizerConfig.MaxFarClipPlane}m)");
                    return true;
                case "smallshadows":
                    OptimizerConfig.EnableSmallObjectShadowCull = !OptimizerConfig.EnableSmallObjectShadowCull;
                    Log("SmallObjectShadowCull = " + OptimizerConfig.EnableSmallObjectShadowCull + " (applies to newly spawned actors)");
                    return true;
                case "qcaps":
                    OptimizerConfig.EnableSmartQualityCaps = !OptimizerConfig.EnableSmartQualityCaps;
                    QualityCapsPatch.ReapplyQualitySettings();
                    Log("SmartQualityCaps = " + OptimizerConfig.EnableSmartQualityCaps +
                        $" (cascades={UnityEngine.QualitySettings.shadowCascades}, res={UnityEngine.QualitySettings.shadowResolution}, dist={UnityEngine.QualitySettings.shadowDistance}m, MSAA={UnityEngine.QualitySettings.antiAliasing}x, lights={UnityEngine.QualitySettings.pixelLightCount}, lodBias={UnityEngine.QualitySettings.lodBias:0.00})");
                    return true;
                case "expr":
                    OptimizerConfig.EnableExpressionCache = !OptimizerConfig.EnableExpressionCache;
                    Log("ExpressionCache = " + OptimizerConfig.EnableExpressionCache);
                    return true;
                case "plort":
                    OptimizerConfig.EnablePlortCollectorPool = !OptimizerConfig.EnablePlortCollectorPool;
                    Log("PlortCollectorPool = " + OptimizerConfig.EnablePlortCollectorPool);
                    return true;
                case "cell":
                    OptimizerConfig.EnableCellDirectorFastCount = !OptimizerConfig.EnableCellDirectorFastCount;
                    Log("CellDirectorFastCount = " + OptimizerConfig.EnableCellDirectorFastCount);
                    return true;
                case "collisions":
                    OptimizerConfig.EnableTrackCollisionsPool = !OptimizerConfig.EnableTrackCollisionsPool;
                    Log("TrackCollisionsPool = " + OptimizerConfig.EnableTrackCollisionsPool);
                    return true;
                case "billboard":
                    OptimizerConfig.EnableBillboardCameraCache = !OptimizerConfig.EnableBillboardCameraCache;
                    Log("BillboardCameraCache = " + OptimizerConfig.EnableBillboardCameraCache);
                    return true;
                case "effects":
                    OptimizerConfig.EnableEffectsComponentCache = !OptimizerConfig.EnableEffectsComponentCache;
                    Log("EffectsComponentCache = " + OptimizerConfig.EnableEffectsComponentCache);
                    return true;
                case "plexer":
                    OptimizerConfig.EnablePlexerDelegateCache = !OptimizerConfig.EnablePlexerDelegateCache;
                    Log("PlexerDelegateCache = " + OptimizerConfig.EnablePlexerDelegateCache);
                    return true;
                case "stats":
                    OptimizerConfig.EnableStatsCollection = !OptimizerConfig.EnableStatsCollection;
                    Log("StatsCollection = " + OptimizerConfig.EnableStatsCollection);
                    return true;
                default:
                    LogWarn("Unknown module '" + module + "'.");
                    return false;
            }
        }

        private static void LogStatus()
        {
            Log(
                "SlimeOptimizer v0.9.2 Status:\n" +
                $"  FpsHudOverlay:      {OptimizerConfig.ShowFpsHud} (limit={(OptimizerConfig.TargetFrameRate <= 0 ? "Unlimited" : OptimizerConfig.TargetFrameRate + " FPS")})\n" +
                $"  RubberBoneLOD:      {OptimizerConfig.EnableRubberBoneLod} (LOD >= {OptimizerConfig.RubberLodDistance}m every {OptimizerConfig.RubberLodInterval} frames, Cull >= {OptimizerConfig.RubberDisableDistance}m)\n" +
                $"  RubberEffectLOD:    {OptimizerConfig.EnableRubberEffectLod}\n" +
                $"  SlimeDragOpt:       {OptimizerConfig.EnableSlimeDragOpt}\n" +
                $"  DisableGifRecorder: {OptimizerConfig.DisableGifRecorder}\n" +
                $"  DisableFogAndDepth: {OptimizerConfig.DisableStylizedFogAndDepth}\n" +
                $"  DisableSectrShadow: {OptimizerConfig.DisableSectrShadowCulling}\n" +
                $"  OptimizeTerrain:    {OptimizerConfig.OptimizeTerrain}\n" +
                $"  SlimeRenderLOD:     {OptimizerConfig.EnableSlimeRenderLod} (Skin1 >= {OptimizerConfig.SlimeSkinLodDistance}m, NoShadow >= {OptimizerConfig.SlimeShadowCullDistance}m)\n" +
                $"  SlimeAiLOD:         {OptimizerConfig.EnableSlimeAiLod} (Far >= {OptimizerConfig.FarAiDistance}m: {OptimizerConfig.FarAiRethinkInterval}s, Ultra >= {OptimizerConfig.UltraFarAiDistance}m: {OptimizerConfig.UltraFarAiRethinkInterval}s)\n" +
                $"  SlimeFaceEmotLOD:   {OptimizerConfig.EnableSlimeFaceEmotionLod} (>= {OptimizerConfig.FaceEmotionLodDistance}m: /{OptimizerConfig.FaceEmotionLodInterval}, >= {OptimizerConfig.FaceEmotionUltraDistance}m: /{OptimizerConfig.FaceEmotionUltraInterval})\n" +
                $"  AnimatorCulling:    {OptimizerConfig.EnableAnimatorCulling}\n" +
                $"  RanchPhysicsOpt:    {OptimizerConfig.EnablePhysicsOpt} (solverIter={UnityEngine.Physics.defaultSolverIterations}, tickHz={OptimizerConfig.PhysicsTickRate}, keepUpright={OptimizerConfig.EnableKeepUprightOpt}, chickenLod={OptimizerConfig.EnableChickenAiLod})\n" +
                $"  RanchTimersLOD:     {OptimizerConfig.EnableRanchTimersLod} (interval={OptimizerConfig.RanchTimersLodInterval})\n" +
                $"  ParticleCap:        {OptimizerConfig.EnableParticleCap} (max={OptimizerConfig.MaxParticlesPerSystem})\n" +
                $"  FastFoodSearch:     {OptimizerConfig.EnableFastFoodSearch}\n" +
                $"  GrassOptimizer:     {OptimizerConfig.EnableGrassOptimizer} (noShadows={OptimizerConfig.DisableGrassShadows}, lodMult={OptimizerConfig.GrassLodMultiplier:0.00}x, windCap={OptimizerConfig.DisableGrassWindShader}@{OptimizerConfig.GrassShaderMaxLod}, shaders=[{GrassOptimizerPatch.GetSeenShadersSummary()}])\n" +
                $"  SsaoOptimizer:      {OptimizerConfig.EnableSsaoOptimizer} (downsample={OptimizerConfig.SsaoDownsampling}x, blurDown={OptimizerConfig.SsaoBlurDownsampling}, gaussian={OptimizerConfig.SsaoUseGaussianBlur})\n" +
                $"  BloomOptimizer:     {OptimizerConfig.EnableBloomOptimizer}\n" +
                $"  CameraLayerCull:    {OptimizerConfig.EnableCameraLayerCull} (actors={OptimizerConfig.LayerActorCullDistance}m, maxFar={OptimizerConfig.MaxFarClipPlane}m)\n" +
                $"  SmallShadowCull:    {OptimizerConfig.EnableSmallObjectShadowCull}\n" +
                $"  SmartQualityCaps:   {OptimizerConfig.EnableSmartQualityCaps} (now: cascades={UnityEngine.QualitySettings.shadowCascades}, res={UnityEngine.QualitySettings.shadowResolution}, dist={UnityEngine.QualitySettings.shadowDistance}m, MSAA={UnityEngine.QualitySettings.antiAliasing}x, lights={UnityEngine.QualitySettings.pixelLightCount}, softParticles={UnityEngine.QualitySettings.softParticles}, lodBias={UnityEngine.QualitySettings.lodBias:0.00}, shaderLOD={UnityEngine.Shader.globalMaximumLOD})\n" +
                $"  ExpressionCache:    {OptimizerConfig.EnableExpressionCache}\n" +
                $"  PlortCollectorPool: {OptimizerConfig.EnablePlortCollectorPool}\n" +
                $"  CellDirectorCount:  {OptimizerConfig.EnableCellDirectorFastCount}\n" +
                $"  TrackCollisionsPool:{OptimizerConfig.EnableTrackCollisionsPool}\n" +
                $"  BillboardCache:     {OptimizerConfig.EnableBillboardCameraCache}\n" +
                $"  EffectsCache:       {OptimizerConfig.EnableEffectsComponentCache}\n" +
                $"  PlexerDelegateCache:{OptimizerConfig.EnablePlexerDelegateCache}\n" +
                $"  StatsCollection:    {OptimizerConfig.EnableStatsCollection}"
            );
        }

        private static void LogStats()
        {
            Log(
                "SlimeOptimizer Saved Work:\n" +
                $"  RubberBone active (near/full):    {OptimizerStats.RubberActiveCalls}\n" +
                $"  RubberBone LOD0 hidden (LOD1+):   {OptimizerStats.RubberNotVisibleCalls}\n" +
                $"  RubberBone culled calls:          {OptimizerStats.RubberCulledCalls}\n" +
                $"  RubberBone LOD skipped frames:    {OptimizerStats.RubberSkippedFrames}\n" +
                $"  Slime shadow LOD switches:        {OptimizerStats.SlimeShadowChanges}\n" +
                $"  Slime skinning LOD switches:      {OptimizerStats.SlimeSkinLodChanges}\n" +
                $"  Slime AI slowed rethinks:         {OptimizerStats.SlimeAiSlowedRethinks}\n" +
                $"  Slime AI slowed ground checks:    {OptimizerStats.SlimeAiSlowedGroundChecks}\n" +
                $"  Slime face anim calls skipped:    {OptimizerStats.FaceAnimCallsSkipped}\n" +
                $"  Slime emotion calls skipped:      {OptimizerStats.EmotionsCallsSkipped}\n" +
                $"  Fast food searches (no-GetComp):  {OptimizerStats.FastFoodSearches} ({OptimizerStats.FastFoodEntriesScanned} entries)\n" +
                $"  Grass shadows disabled:           {OptimizerStats.GrassShadowsDisabled}\n" +
                $"  Grass LODGroups tightened:        {OptimizerStats.GrassLodGroupsAdjusted}\n" +
                $"  Grass shaders LOD-capped:         {OptimizerStats.GrassShadersCapped}\n" +
                $"  SSAO downsampled frames:          {OptimizerStats.SsaoDownsampledFrames} (keyword resets avoided: {OptimizerStats.SsaoKeywordResetsAvoided})\n" +
                $"  Bloom frames optimized:           {OptimizerStats.BloomFramesOptimized}\n" +
                $"  FX particles capped:              {OptimizerStats.ParticlesCapped}\n" +
                $"  Ranch timers skipped:             {OptimizerStats.RanchTimersSkipped}\n" +
                $"  Animators culled off-screen:      {OptimizerStats.AnimatorsCulled}\n" +
                $"  KeepUpright sleeping/far skipped: {OptimizerStats.KeepUprightSkipped}\n" +
                $"  RubberEffect LOD skipped:         {OptimizerStats.RubberEffectSkipped}\n" +
                $"  SlimeDrag redundant skipped:      {OptimizerStats.DragUpdatesSkipped}\n" +
                $"  StylizedFog passes skipped:       {OptimizerStats.FogPassesSkipped}\n" +
                $"  Terrains optimized:               {OptimizerStats.TerrainsOptimized}\n" +
                $"  Chicken AI distant skipped:       {OptimizerStats.ChickenAiSkipped}\n" +
                $"  Cameras layer-culled:             {OptimizerStats.CamerasLayerCulled}\n" +
                $"  Small actor shadows culled:       {OptimizerStats.SmallActorShadowsCulled}\n" +
                $"  Expression no-ops skipped:        {OptimizerStats.ExpressionNoOpsAvoided}\n" +
                $"  Expression Material[] avoided:    {OptimizerStats.ExpressionArrayAllocsAvoided}\n" +
                $"  PlortCollector lists reused:      {OptimizerStats.PlortCollectorListsReused}\n" +
                $"  CellDirector LINQ Count avoided:  {OptimizerStats.LinqCallsAvoided}\n" +
                $"  TrackCollisions empty fast-paths: {OptimizerStats.TrackCollisionsEmptyFastPaths}\n" +
                $"  TrackCollisions lists reused:     {OptimizerStats.ColliderListsReused}\n" +
                $"  Billboard Camera.main hits saved: {OptimizerStats.BillboardCacheHits}\n" +
                $"  Effects GetComponent avoided:     {OptimizerStats.EffectsGetComponentsAvoided}\n" +
                $"  Plexer Raycast delegates reused:  {OptimizerStats.PlexerDelegatesReused}"
            );
        }

        private static void Log(string msg)
        {
            if (ModMain.Logger != null)
            {
                ModMain.Logger.Log(msg);
            }
            else
            {
                Console.Instance.Log(msg);
            }
        }

        private static void LogWarn(string msg)
        {
            if (ModMain.Logger != null)
            {
                ModMain.Logger.LogWarning(msg);
            }
            else
            {
                Console.Instance.LogWarning(msg);
            }
        }
    }
}
