using UnityEngine;

namespace SlimeOptimizer.Patches
{
    internal static class CameraCullDistancePatch
    {
        private sealed class OriginalCameraSettings
        {
            public float FarClipPlane;
            public float[] LayerCullDistances;
            public bool LayerCullSpherical;
            public bool Applied;
        }

        private static readonly FastObjCache<Camera, OriginalCameraSettings> Originals =
            new FastObjCache<Camera, OriginalCameraSettings>(8);

        private static readonly float[] WorkDistances = new float[32];

        private static Camera _lastCamera;
        private static bool _dirty = true;

        private static int _actorLayer = -2;
        private static int _actorEchoesLayer = -2;
        private static int _actorIgnorePlayerLayer = -2;
        private static int _launchedLayer = -2;

        public static void MarkDirty()
        {
            _dirty = true;
            UpdateActiveCamera();
        }

        public static void UpdateActiveCamera()
        {
            Camera cam = CameraCache.MainCamera;
            if (ReferenceEquals(cam, null))
            {
                return;
            }

            if (ReferenceEquals(cam, _lastCamera) && !_dirty)
            {
                return;
            }

            _lastCamera = cam;
            _dirty = false;
            ApplyToCamera(cam);
        }

        private static void EnsureLayerIndices()
        {
            if (_actorLayer == -2)
            {
                _actorLayer = LayerMask.NameToLayer("Actor");
                _actorEchoesLayer = LayerMask.NameToLayer("ActorEchoes");
                _actorIgnorePlayerLayer = LayerMask.NameToLayer("ActorIgnorePlayer");
                _launchedLayer = LayerMask.NameToLayer("Launched");
            }
        }

        private static void ApplyToCamera(Camera cam)
        {
            if (!Originals.TryGetValue(cam, out OriginalCameraSettings orig))
            {
                float[] existing = cam.layerCullDistances;
                float[] copy = new float[32];
                if (existing != null && existing.Length == 32)
                {
                    System.Array.Copy(existing, copy, 32);
                }

                orig = new OriginalCameraSettings
                {
                    FarClipPlane = cam.farClipPlane,
                    LayerCullDistances = copy,
                    LayerCullSpherical = cam.layerCullSpherical,
                    Applied = false
                };
                Originals.Set(cam, orig);
            }

            if (!OptimizerConfig.EnableCameraLayerCull)
            {
                if (orig.Applied)
                {
                    cam.farClipPlane = orig.FarClipPlane;
                    cam.layerCullDistances = orig.LayerCullDistances;
                    cam.layerCullSpherical = orig.LayerCullSpherical;
                    orig.Applied = false;
                }
                return;
            }

            EnsureLayerIndices();

            System.Array.Copy(orig.LayerCullDistances, WorkDistances, 32);

            float actorDist = OptimizerConfig.LayerActorCullDistance;
            if (actorDist > 0f)
            {
                SetLayerDistanceIfCloser(WorkDistances, _actorLayer, actorDist);
                SetLayerDistanceIfCloser(WorkDistances, _actorEchoesLayer, actorDist);
                SetLayerDistanceIfCloser(WorkDistances, _actorIgnorePlayerLayer, actorDist);
                SetLayerDistanceIfCloser(WorkDistances, _launchedLayer, actorDist);
            }

            cam.layerCullDistances = WorkDistances;
            cam.layerCullSpherical = true;

            float maxFar = OptimizerConfig.MaxFarClipPlane;
            if (maxFar > 0f && orig.FarClipPlane > maxFar)
            {
                cam.farClipPlane = maxFar;
            }
            else
            {
                cam.farClipPlane = orig.FarClipPlane;
            }

            if (!orig.Applied && OptimizerConfig.EnableStatsCollection)
            {
                OptimizerStats.CamerasLayerCulled++;
            }
            orig.Applied = true;
        }

        private static void SetLayerDistanceIfCloser(float[] distances, int layer, float dist)
        {
            if (layer >= 0 && layer < 32)
            {
                if (distances[layer] <= 0f || distances[layer] > dist)
                {
                    distances[layer] = dist;
                }
            }
        }
    }
}
