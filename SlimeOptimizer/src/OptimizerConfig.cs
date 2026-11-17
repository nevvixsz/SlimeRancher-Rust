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

        [ConfigComment("Maximum Unity LOD bias when SmartQualityCaps is enabled (vanilla High=2.0, Medium=1.0; recommended: 1.2 to stop distant grass/foliage from staying in heavy LOD0).")]
        public static float MaxLodBias = 1.2f;

        [ConfigComment("Maximum shadow map resolution when SmartQualityCaps is enabled (0=Low, 1=Medium, 2=High, 3=VeryHigh; recommended: 1 for big GPU FPS gain).")]
        public static int MaxShadowResolution = 1;

        [ConfigComment("Maximum pixel light count when SmartQualityCaps is enabled (vanilla Highest=4; recommended: 2).")]
        public static int MaxPixelLightCount = 2;

        [ConfigComment("Optimize SECTR_CullingCamera and throttle SECTR_LOD.SelectLOD (eliminates per-frame Plane[6] GC and GetComponent overhead).")]
        public static bool EnableSectrCullingOpt = true;

        [ConfigComment("Update each SECTR_LOD only every N frames after initial activation (recommended: 4 = 75% less SECTR_LOD work).")]
        public static int SectrLodInterval = 4;

        [ConfigComment("Apply per-layer camera cull distances and a smart farClipPlane cap to skip rendering distant small actors.")]
        public static bool EnableCameraLayerCull = true;

        [ConfigComment("Cull distance (in meters) for Actor/ActorEchoes/ActorIgnorePlayer/Launched layers (slimes/plorts/food far beyond sight; 0 = disabled).")]
        public static float LayerActorCullDistance = 95f;

        [ConfigComment("Maximum camera farClipPlane in meters (0 = unchanged; recommended: 450 to trim distant fogged geometry).")]
        public static float MaxFarClipPlane = 450f;

        [ConfigComment("Slow down AI rethink and ground-raycast frequency for distant slimes (>= 24m) to save CPU in large ranches.")]
        public static bool EnableSlimeAiLod = true;

        [ConfigComment("Distance (in meters) beyond which slime AI uses reduced rethink/ground-check intervals.")]
        public static float FarAiDistance = 24f;

        [ConfigComment("AI rethink interval in seconds for slimes at >= FarAiDistance (vanilla: 1.0s).")]
        public static float FarAiRethinkInterval = 2.8f;

        [ConfigComment("Ground check raycast interval in seconds for slimes at >= FarAiDistance (vanilla: 0.25s).")]
        public static float FarGroundCheckInterval = 0.6f;

        [ConfigComment("Distance (in meters) beyond which slime AI uses ultra-low rethink/ground-check intervals.")]
        public static float UltraFarAiDistance = 48f;

        [ConfigComment("AI rethink interval in seconds for slimes at >= UltraFarAiDistance (vanilla: 1.0s).")]
        public static float UltraFarAiRethinkInterval = 5.0f;

        [ConfigComment("Ground check raycast interval in seconds for slimes at >= UltraFarAiDistance (vanilla: 0.25s).")]
        public static float UltraFarGroundCheckInterval = 1.0f;

        [ConfigComment("Optimize FindConsumable food/target search (avoid per-entry GetComponent<Identifiable> and key-collection boxing).")]
        public static bool EnableFastFoodSearch = true;

        [ConfigComment("Optimize dense grass/foliage meshes when sectors load (disable grass shadow casting and tighten grass-only LOD fade).")]
        public static bool EnableGrassOptimizer = true;

        [ConfigComment("Disable shadow casting on grass meshes (big GPU win in grassy areas like Moss Blanket/Wilds).")]
        public static bool DisableGrassShadows = true;

        [ConfigComment("Multiplier applied to grass-only LODGroup transition heights (>1.0 switches grass to lighter LODs sooner; 1.0 = unchanged).")]
        public static float GrassLodMultiplier = 1.5f;

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
