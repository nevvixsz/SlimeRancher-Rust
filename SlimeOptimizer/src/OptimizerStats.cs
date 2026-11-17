namespace SlimeOptimizer
{
    public static class OptimizerStats
    {
        public static long RubberActiveCalls;
        public static long RubberNotVisibleCalls;
        public static long RubberCulledCalls;
        public static long RubberSkippedFrames;
        public static long SlimeShadowChanges;
        public static long SlimeSkinLodChanges;
        public static long SmallActorShadowsCulled;
        public static long ExpressionNoOpsAvoided;
        public static long ExpressionArrayAllocsAvoided;
        public static long PlortCollectorListsReused;
        public static long LinqCallsAvoided;
        public static long TrackCollisionsEmptyFastPaths;
        public static long ColliderListsReused;
        public static long BillboardCacheHits;
        public static long EffectsGetComponentsAvoided;
        public static long PlexerDelegatesReused;
        public static long SlimeAiSlowedRethinks;
        public static long SlimeAiSlowedGroundChecks;
        public static long FastFoodSearches;
        public static long FastFoodEntriesScanned;
        public static long GrassShadowsDisabled;
        public static long GrassLodGroupsAdjusted;
        public static long GrassShadersCapped;
        public static long CamerasLayerCulled;

        public static void Reset()
        {
            RubberActiveCalls = 0;
            RubberNotVisibleCalls = 0;
            RubberCulledCalls = 0;
            RubberSkippedFrames = 0;
            SlimeShadowChanges = 0;
            SlimeSkinLodChanges = 0;
            SmallActorShadowsCulled = 0;
            ExpressionNoOpsAvoided = 0;
            ExpressionArrayAllocsAvoided = 0;
            PlortCollectorListsReused = 0;
            LinqCallsAvoided = 0;
            TrackCollisionsEmptyFastPaths = 0;
            ColliderListsReused = 0;
            BillboardCacheHits = 0;
            EffectsGetComponentsAvoided = 0;
            PlexerDelegatesReused = 0;
            SlimeAiSlowedRethinks = 0;
            SlimeAiSlowedGroundChecks = 0;
            FastFoodSearches = 0;
            FastFoodEntriesScanned = 0;
            GrassShadowsDisabled = 0;
            GrassLodGroupsAdjusted = 0;
            GrassShadersCapped = 0;
            CamerasLayerCulled = 0;
        }
    }
}
