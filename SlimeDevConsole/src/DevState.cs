using System;
using UnityEngine;

namespace SlimeDevConsole
{
    internal static class DevState
    {
        public static bool GodMode;
        public static bool InfiniteEnergy;
        public static bool InfiniteVac;
        public static bool FreezeTime;
        public static bool FlyMode;
        public static float FlySpeedMultiplier = 2.0f;
        public static float WalkSpeedMultiplier = 1.0f;
        public static float WorldTimeScale = 1.0f;

        private static bool _capturedMoveDefaults;
        private static float _baseMotorAccel = 0.18f;
        private static float _baseJumpForce = 0.18f;
        private static int _savedPlayerLayer = -1;
        private static GameObject _lastPlayerObj;

        public static void ToggleFly(float? customSpeed = null)
        {
            GameObject player = SRSingleton<SceneContext>.Instance != null
                ? SRSingleton<SceneContext>.Instance.Player
                : null;
            if (player == null)
            {
                return;
            }

            vp_FPController ctrl = player.GetComponentInChildren<vp_FPController>();
            if (ctrl == null)
            {
                return;
            }

            CaptureMoveDefaults(ctrl);

            if (customSpeed.HasValue)
            {
                FlySpeedMultiplier = Mathf.Clamp(customSpeed.Value, 0.2f, 20f);
                if (!ctrl.MotorFreeFly)
                {
                    EnableFly(player, ctrl);
                }
                else
                {
                    ApplyMovementSpeed(ctrl);
                }
                return;
            }

            if (ctrl.MotorFreeFly)
            {
                DisableFly(player, ctrl);
            }
            else
            {
                EnableFly(player, ctrl);
            }
        }

        private static void EnableFly(GameObject player, vp_FPController ctrl)
        {
            if (_savedPlayerLayer < 0)
            {
                _savedPlayerLayer = player.layer;
            }

            ctrl.MotorFreeFly = true;
            int raycastOnly = LayerMask.NameToLayer("RaycastOnly");
            if (raycastOnly >= 0)
            {
                player.layer = raycastOnly;
            }
            FlyMode = true;
            ApplyMovementSpeed(ctrl);
        }

        private static void DisableFly(GameObject player, vp_FPController ctrl)
        {
            ctrl.MotorFreeFly = false;
            if (_savedPlayerLayer >= 0)
            {
                player.layer = _savedPlayerLayer;
            }
            FlyMode = false;
            ApplyMovementSpeed(ctrl);
        }

        public static void SetWalkSpeed(float mult)
        {
            WalkSpeedMultiplier = Mathf.Clamp(mult, 0.2f, 15f);
            GameObject player = SRSingleton<SceneContext>.Instance != null
                ? SRSingleton<SceneContext>.Instance.Player
                : null;
            if (player == null)
            {
                return;
            }

            vp_FPController ctrl = player.GetComponentInChildren<vp_FPController>();
            if (ctrl != null)
            {
                CaptureMoveDefaults(ctrl);
                ApplyMovementSpeed(ctrl);
            }
        }

        private static void CaptureMoveDefaults(vp_FPController ctrl)
        {
            if (_capturedMoveDefaults || ctrl == null)
            {
                return;
            }
            _capturedMoveDefaults = true;
            _baseMotorAccel = ctrl.MotorAcceleration > 0.001f ? ctrl.MotorAcceleration : 0.18f;
            _baseJumpForce = ctrl.MotorJumpForce > 0.001f ? ctrl.MotorJumpForce : 0.18f;
        }

        public static void ApplyMovementSpeed(vp_FPController ctrl)
        {
            if (ctrl == null)
            {
                return;
            }
            CaptureMoveDefaults(ctrl);

            float mult = ctrl.MotorFreeFly ? FlySpeedMultiplier : WalkSpeedMultiplier;
            ctrl.MotorAcceleration = _baseMotorAccel * mult;
            ctrl.MotorJumpForce = _baseJumpForce * Mathf.Sqrt(Mathf.Max(1f, WalkSpeedMultiplier));
        }

        public static void TickPerFrame()
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.PlayerState == null)
            {
                return;
            }

            if (ctx.Player != null && !ReferenceEquals(_lastPlayerObj, ctx.Player))
            {
                _lastPlayerObj = ctx.Player;
                _capturedMoveDefaults = false;
                _savedPlayerLayer = -1;
                FlyMode = false;
                if (Math.Abs(WalkSpeedMultiplier - 1.0f) > 0.01f)
                {
                    vp_FPController ctrl = ctx.Player.GetComponentInChildren<vp_FPController>();
                    if (ctrl != null)
                    {
                        ApplyMovementSpeed(ctrl);
                    }
                }
            }

            if (GodMode)
            {
                int maxHp = ctx.PlayerState.GetMaxHealth();
                if (ctx.PlayerState.GetCurrHealth() < maxHp)
                {
                    ctx.PlayerState.SetHealth(maxHp);
                }
                if (ctx.PlayerState.GetCurrRad() > 0)
                {
                    ctx.PlayerState.SetRad(0);
                }
            }

            if (InfiniteEnergy)
            {
                int maxEn = ctx.PlayerState.GetMaxEnergy();
                if (ctx.PlayerState.GetCurrEnergy() < maxEn)
                {
                    ctx.PlayerState.SetEnergy(maxEn);
                }
            }

            if (Math.Abs(WorldTimeScale - 1.0f) > 0.01f && Time.timeScale > 0.0f)
            {
                if (Math.Abs(Time.timeScale - WorldTimeScale) > 0.01f)
                {
                    Time.timeScale = WorldTimeScale;
                }
            }
        }
    }
}
