using System.Runtime.CompilerServices;
using UnityEngine;

namespace SlimeOptimizer
{
    public static class CameraCache
    {
        public static int FrameCount;
        public static float CamX;
        public static float CamY;
        public static float CamZ;
        public static bool HasCamera;
        public static Camera MainCam;
        public static GameObject PlayerGo;

        private static int _lastFrame = -1;
        private static Vector3 _position;

        public static Camera MainCamera
        {
            [MethodImpl(MethodImplOptions.AggressiveInlining)]
            get
            {
                EnsureCurrentFrame();
                return MainCam;
            }
        }

        public static Vector3 Position
        {
            [MethodImpl(MethodImplOptions.AggressiveInlining)]
            get
            {
                EnsureCurrentFrame();
                return _position;
            }
        }

        public static bool HasCameraCached
        {
            [MethodImpl(MethodImplOptions.AggressiveInlining)]
            get => HasCamera;
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static void EnsureCurrentFrame()
        {
            int frame = Time.frameCount;
            if (frame == _lastFrame)
            {
                return;
            }

            RefreshFrame(frame);
        }

        public static void RefreshFrame(int frame)
        {
            _lastFrame = frame;
            FrameCount = frame;

            if (MainCam == null || !MainCam.isActiveAndEnabled)
            {
                MainCam = Camera.main;
            }

            if (MainCam != null)
            {
                _position = MainCam.transform.position;
                CamX = _position.x;
                CamY = _position.y;
                CamZ = _position.z;
                HasCamera = true;
            }
            else
            {
                HasCamera = false;
            }

            if (PlayerGo == null && SRSingleton<SceneContext>.Instance != null)
            {
                PlayerGo = SRSingleton<SceneContext>.Instance.Player;
            }
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static float SqrDistanceToCamera(Transform tr)
        {
            Vector3 p = tr.position;
            float dx = p.x - CamX;
            float dy = p.y - CamY;
            float dz = p.z - CamZ;
            return dx * dx + dy * dy + dz * dz;
        }
    }
}
