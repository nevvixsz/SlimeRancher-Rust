using UnityEngine;

namespace SlimeOptimizer
{
    public static class CameraCache
    {
        private static int _lastFrame = -1;
        private static Camera _mainCamera;
        private static Vector3 _position;
        private static bool _hasCamera;

        public static Camera MainCamera
        {
            get
            {
                EnsureCurrentFrame();
                return _mainCamera;
            }
        }

        public static Vector3 Position
        {
            get
            {
                EnsureCurrentFrame();
                return _position;
            }
        }

        public static bool HasCamera
        {
            get
            {
                EnsureCurrentFrame();
                return _hasCamera;
            }
        }

        public static void EnsureCurrentFrame()
        {
            int frame = Time.frameCount;
            if (frame == _lastFrame)
            {
                return;
            }

            _lastFrame = frame;

            if (_mainCamera == null || !_mainCamera.isActiveAndEnabled)
            {
                _mainCamera = Camera.main;
            }

            if (_mainCamera != null)
            {
                _position = _mainCamera.transform.position;
                _hasCamera = true;
            }
            else
            {
                _hasCamera = false;
            }
        }
    }
}
