using SRML.Config.Attributes;

namespace SlimeOptimizer
{
    [ConfigFile("SlimeOptimizer", "General")]
    public static class OptimizerConfig
    {
        [ConfigComment("Enable distance-based LOD and sleep culling for slime RubberBoneEffect jiggle physics (biggest FPS gain).")]
        public static bool EnableRubberBoneLod = true;

        [ConfigComment("Distance (in meters) at which slime jiggle physics switches to reduced frame rate.")]
        public static float RubberLodDistance = 18f;

        [ConfigComment("Distance (in meters) beyond which slime jiggle physics rests and skips updates until closer.")]
        public static float RubberDisableDistance = 36f;

        [ConfigComment("Update interval (every N frames) for slimes in the LOD zone (e.g., 3 = ~1/3 CPU cost).")]
        public static int RubberLodInterval = 3;

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

        [ConfigComment("Collect counters for the 'slimeopt stats' console command.")]
        public static bool EnableStatsCollection = true;
    }
}
