using SRML.Config.Attributes;

namespace SlimeOptimizer
{
    [ConfigFile("SlimeOptimizer", "General")]
    public static class OptimizerConfig
    {
        [ConfigComment("Enable distance-based LOD and sleep culling for slime RubberBoneEffect jiggle physics (biggest CPU FPS gain).")]
        public static bool EnableRubberBoneLod = true;

        [ConfigComment("Distance (in meters) at which slime jiggle physics switches to reduced frame rate.")]
        public static float RubberLodDistance = 10f;

        [ConfigComment("Distance (in meters) beyond which slime jiggle physics rests and skips updates until closer.")]
        public static float RubberDisableDistance = 20f;

        [ConfigComment("Update interval (every N frames) for slimes in the LOD zone (e.g., 3 = ~1/3 CPU cost).")]
        public static int RubberLodInterval = 3;

        [ConfigComment("Enable distance-based ShadowCasting and SkinQuality LOD on slimes (huge GPU win when looking across the ranch).")]
        public static bool EnableSlimeRenderLod = true;

        [ConfigComment("Distance (in meters) at which slime SkinnedMeshRenderer switches from full bone weights to 1-bone skinning.")]
        public static float SlimeSkinLodDistance = 18f;

        [ConfigComment("Distance (in meters) beyond which distant slimes stop casting shadows into shadow maps.")]
        public static float SlimeShadowCullDistance = 24f;

        [ConfigComment("Disable shadow casting on small dropped ranch actors (plorts, veggies, fruit, meat, chicks, craft resources).")]
        public static bool EnableSmallObjectShadowCull = true;

        [ConfigComment("Cap ultra-heavy GPU quality extremes (4 shadow cascades -> 2, 150m shadows -> 85m, 8x MSAA -> 2x) while keeping shaders/lighting/SSAO maxed.")]
        public static bool EnableSmartQualityCaps = true;

        [ConfigComment("Maximum shadow cascades allowed when SmartQualityCaps is enabled (recommended: 2 instead of 4).")]
        public static int MaxShadowCascades = 2;

        [ConfigComment("Maximum shadow distance in meters when SmartQualityCaps is enabled (recommended: 85 instead of 150).")]
        public static float MaxShadowDistance = 85f;

        [ConfigComment("Maximum MSAA level when SmartQualityCaps is enabled (0, 2, 4, or 8; recommended: 2 for high FPS).")]
        public static int MaxAntiAliasing = 2;

        [ConfigComment("Cache slime face renderer Material[] arrays and skip no-op expression assignments (eliminates GC spikes).")]
        public static bool EnableExpressionCache = true;

        [ConfigComment("Reuse a single list in PlortCollector.Update instead of allocating a new List every frame.")]
        public static bool EnablePlortCollectorPool = true;

        [ConfigComment("Replace Enumerable.Count LINQ calls in CellDirector.Update with direct ICollection.Count.")]
        public static bool EnableCellDirectorFastCount = true;

        [ConfigComment("Pool List<Collider> instances and fast-path empty triggers in TrackCollisions.")]
        public static bool EnableTrackCollisionsPool = true;

        [ConfigComment("Cache Camera.main in CameraFacingBillboard instead of searching by tag.")]
        public static bool EnableBillboardCameraCache = true;

        [ConfigComment("Cache SSAOPro/Bloom/Camera components in DisableEffectsOnLowQuality instead of GetComponent every frame.")]
        public static bool EnableEffectsComponentCache = true;

        [ConfigComment("Cache Action<RaycastHit> ground-check delegates in SlimeSubbehaviourPlexer.")]
        public static bool EnablePlexerDelegateCache = true;

        [ConfigComment("Collect counters for the 'slimeopt stats' console command.")]
        public static bool EnableStatsCollection = true;
    }
}
