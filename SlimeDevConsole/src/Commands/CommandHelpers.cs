using System;
using System.Collections.Generic;
using System.Globalization;
using HarmonyLib;
using MonomiPark.SlimeRancher.Regions;
using SRML.Console;
using UnityEngine;

namespace SlimeDevConsole.Commands
{
    internal static class CommandHelpers
    {
        private static readonly AccessTools.FieldRef<IdDirector, Dictionary<IdHandler, string>> PersistenceDictRef =
            AccessTools.FieldRefAccess<IdDirector, Dictionary<IdHandler, string>>("persistenceDict");

        public static void AssignUniquePersistenceId(GameObject go, string uniqueId)
        {
            if (go == null || string.IsNullOrEmpty(uniqueId))
            {
                return;
            }

            IdHandler ih = go.GetComponent<IdHandler>();
            if (ih == null)
            {
                return;
            }

            IdDirector dir = go.GetComponentInParent<IdDirector>();
            if (dir == null)
            {
                dir = go.AddComponent<IdDirector>();
            }

            Dictionary<IdHandler, string> dict = PersistenceDictRef(dir);
            if (dict != null)
            {
                dict[ih] = uniqueId;
            }
        }

        public static void Log(string msg)
        {
            if (ModMain.LogConsole != null)
            {
                ModMain.LogConsole.Log(msg);
            }
            else
            {
                Debug.Log("[SlimeDevConsole] " + msg);
            }
        }

        public static void LogError(string msg)
        {
            if (ModMain.LogConsole != null)
            {
                ModMain.LogConsole.LogError(msg);
            }
            else
            {
                Debug.LogError("[SlimeDevConsole] " + msg);
            }
        }

        public static void LogSuccess(string msg)
        {
            if (ModMain.LogConsole != null)
            {
                ModMain.LogConsole.LogSuccess(msg);
            }
            else
            {
                Debug.Log("[SlimeDevConsole] " + msg);
            }
        }

        public static bool EnsureInGame(out SceneContext ctx, out GameContext gctx)
        {
            gctx = SRSingleton<GameContext>.Instance;
            ctx = SRSingleton<SceneContext>.Instance;
            if (gctx == null || ctx == null || ctx.Player == null || ctx.PlayerState == null)
            {
                LogError("Нужно загрузить сохранение / мир (игрок ещё не создан).");
                return false;
            }
            return true;
        }

        public static bool TryParseInt(string s, out int value)
        {
            return int.TryParse(s, NumberStyles.Integer, CultureInfo.InvariantCulture, out value);
        }

        public static bool TryParseFloat(string s, out float value)
        {
            if (string.IsNullOrEmpty(s))
            {
                value = 0f;
                return false;
            }
            string norm = s.Replace(',', '.');
            return float.TryParse(norm, NumberStyles.Float, CultureInfo.InvariantCulture, out value);
        }

        public static RegionRegistry.RegionSetId GetCurrentRegionSetId(SceneContext ctx)
        {
            if (ctx != null && ctx.Player != null)
            {
                RegionMember rm = ctx.Player.GetComponent<RegionMember>();
                if (rm != null)
                {
                    return rm.setId;
                }
            }
            if (ctx != null && ctx.RegionRegistry != null)
            {
                return ctx.RegionRegistry.GetCurrentRegionSetId();
            }
            return RegionRegistry.RegionSetId.HOME;
        }

        public static Vector3 GetAimSpawnPoint(SceneContext ctx, float fallbackDistance, out Quaternion yawRotation)
        {
            Camera cam = Camera.main;
            Vector3 playerPos = ctx.Player.transform.position;
            Vector3 forward = cam != null ? cam.transform.forward : ctx.Player.transform.forward;
            Vector3 origin = cam != null ? cam.transform.position : (playerPos + Vector3.up * 1.5f);

            Vector3 flatForward = new Vector3(forward.x, 0f, forward.z);
            if (flatForward.sqrMagnitude < 0.0001f)
            {
                flatForward = Vector3.forward;
            }
            yawRotation = Quaternion.LookRotation(-flatForward.normalized, Vector3.up);

            RaycastHit hit;
            if (Physics.Raycast(origin + forward * 0.6f, forward, out hit, Mathf.Max(2f, fallbackDistance), ~0, QueryTriggerInteraction.Ignore))
            {
                return hit.point + hit.normal * 0.35f;
            }

            return origin + forward * fallbackDistance;
        }

        public static Vector3 SnapToGround(Vector3 pos, float maxDrop = 30f)
        {
            RaycastHit hit;
            if (Physics.Raycast(pos + Vector3.up * 4f, Vector3.down, out hit, maxDrop + 4f, ~0, QueryTriggerInteraction.Ignore))
            {
                return hit.point + Vector3.up * 0.1f;
            }
            return pos;
        }
    }

    internal abstract class DevCommandBase : ConsoleCommand
    {
        private readonly string _id;
        private readonly string _usage;
        private readonly string _description;

        protected DevCommandBase(string id, string usage, string description)
        {
            _id = id.ToLowerInvariant();
            _usage = usage;
            _description = description;
        }

        public override string ID { get { return _id; } }
        public override string Usage { get { return _usage; } }
        public override string Description { get { return _description; } }

        public virtual List<string> AutoComplete(int argIndex, string argText)
        {
            return null;
        }

        public override List<string> GetAutoComplete(int argIndex, string argText)
        {
            return AutoComplete(argIndex, argText);
        }
    }

    internal sealed class AliasCommand : ConsoleCommand
    {
        private readonly string _id;
        private readonly DevCommandBase _target;
        private readonly string _customUsage;

        public AliasCommand(string id, DevCommandBase target, string customUsage = null)
        {
            _id = id.ToLowerInvariant();
            _target = target;
            _customUsage = customUsage;
        }

        public override string ID { get { return _id; } }
        public override string Usage { get { return _customUsage ?? _target.Usage; } }
        public override string Description { get { return _target.Description + " [alias -> " + _target.ID + "]"; } }

        public override bool Execute(string[] args)
        {
            return _target.Execute(args);
        }

        public override List<string> GetAutoComplete(int argIndex, string argText)
        {
            return _target.AutoComplete(argIndex, argText);
        }
    }
}
