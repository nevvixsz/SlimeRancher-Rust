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
            "reset",
            "toggle"
        };

        private static readonly List<string> Modules = new List<string>
        {
            "rubber",
            "renderlod",
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

        public override string Usage => "slimeopt <status|stats|reset|toggle <module>>";

        public override string Description => "Shows SlimeOptimizer status/stats or toggles optimization modules at runtime.";

        public override string ExtendedDescription =>
            "Subcommands:\n" +
            "  status              - Show active modules and LOD/quality thresholds\n" +
            "  stats               - Show saved calls and avoided GC allocations\n" +
            "  reset               - Reset statistics counters\n" +
            "  toggle <module>     - Toggle: rubber, renderlod, smallshadows, qcaps, expr, plort, cell, collisions, billboard, effects, plexer, stats";

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
                case "reset":
                    OptimizerStats.Reset();
                    Log("SlimeOptimizer stats counters reset.");
                    return true;
                case "toggle":
                    if (args.Length < 2)
                    {
                        LogWarn("Usage: slimeopt toggle <rubber|renderlod|smallshadows|qcaps|expr|plort|cell|collisions|billboard|effects|plexer|stats>");
                        return false;
                    }
                    return ToggleModule(args[1].ToLowerInvariant());
                default:
                    LogWarn("Unknown subcommand. Usage: " + Usage);
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
                case "renderlod":
                    OptimizerConfig.EnableSlimeRenderLod = !OptimizerConfig.EnableSlimeRenderLod;
                    Log("SlimeRenderLod = " + OptimizerConfig.EnableSlimeRenderLod);
                    return true;
                case "smallshadows":
                    OptimizerConfig.EnableSmallObjectShadowCull = !OptimizerConfig.EnableSmallObjectShadowCull;
                    Log("SmallObjectShadowCull = " + OptimizerConfig.EnableSmallObjectShadowCull + " (applies to newly spawned actors)");
                    return true;
                case "qcaps":
                    OptimizerConfig.EnableSmartQualityCaps = !OptimizerConfig.EnableSmartQualityCaps;
                    QualityCapsPatch.ReapplyQualitySettings();
                    Log("SmartQualityCaps = " + OptimizerConfig.EnableSmartQualityCaps +
                        $" (cascades={UnityEngine.QualitySettings.shadowCascades}, dist={UnityEngine.QualitySettings.shadowDistance}m, MSAA={UnityEngine.QualitySettings.antiAliasing}x)");
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
                "SlimeOptimizer v0.2.0 Status:\n" +
                $"  RubberBoneLOD:      {OptimizerConfig.EnableRubberBoneLod} (LOD >= {OptimizerConfig.RubberLodDistance}m every {OptimizerConfig.RubberLodInterval} frames, Cull >= {OptimizerConfig.RubberDisableDistance}m)\n" +
                $"  SlimeRenderLOD:     {OptimizerConfig.EnableSlimeRenderLod} (Skin1 >= {OptimizerConfig.SlimeSkinLodDistance}m, NoShadow >= {OptimizerConfig.SlimeShadowCullDistance}m)\n" +
                $"  SmallShadowCull:    {OptimizerConfig.EnableSmallObjectShadowCull}\n" +
                $"  SmartQualityCaps:   {OptimizerConfig.EnableSmartQualityCaps} (now: cascades={UnityEngine.QualitySettings.shadowCascades}, dist={UnityEngine.QualitySettings.shadowDistance}m, MSAA={UnityEngine.QualitySettings.antiAliasing}x)\n" +
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
