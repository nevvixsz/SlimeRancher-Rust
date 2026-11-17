namespace SlimeOptimizer
{
    public static class OptimizerStats
    {
        public static long RubberCulledCalls;
        public static long RubberSkippedFrames;
        public static long ExpressionNoOpsAvoided;
        public static long ExpressionArrayAllocsAvoided;
        public static long PlortCollectorListsReused;
        public static long LinqCallsAvoided;
        public static long TrackCollisionsEmptyFastPaths;
        public static long ColliderListsReused;
        public static long BillboardCacheHits;

        public static void Reset()
        {
            RubberCulledCalls = 0;
            RubberSkippedFrames = 0;
            ExpressionNoOpsAvoided = 0;
            ExpressionArrayAllocsAvoided = 0;
            PlortCollectorListsReused = 0;
            LinqCallsAvoided = 0;
            TrackCollisionsEmptyFastPaths = 0;
            ColliderListsReused = 0;
            BillboardCacheHits = 0;
        }
    }
}
