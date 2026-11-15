// Optional single-file SRML mod to dump exact scene coordinates and LookupDirector item colors
// into `world_dump.json` for `SlimeRancher-Rust` (`local_assets/world_dump.json`).
//
// Usage with SRML:
// 1. Compile against Assembly-CSharp.dll, SRML.dll, UnityEngine.CoreModule.dll, UnityEngine.InputLegacyModule.dll
//    (or drop into an SRML mod project).
// 2. Load into the Ranch in-game and press F9.
// 3. Copy the generated `world_dump.json` from the game folder into `<SlimeRancher-Rust>/local_assets/world_dump.json`.

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using SRML;
using UnityEngine;

namespace SrWorldDumper
{
    public class ModEntry : ModEntryPoint
    {
        public override void PostLoad()
        {
            var go = new GameObject("SrWorldDumperHost");
            UnityEngine.Object.DontDestroyOnLoad(go);
            go.AddComponent<DumperBehaviour>();
        }
    }

    public class DumperBehaviour : MonoBehaviour
    {
        private void Update()
        {
            if (Input.GetKeyDown(KeyCode.F9))
            {
                DumpNow();
            }
        }

        private static string Esc(string s)
        {
            if (string.IsNullOrEmpty(s)) return "";
            return s.Replace("\\", "\\\\").Replace("\"", "\\\"");
        }

        private static string F(float v) => v.ToString("R", CultureInfo.InvariantCulture);

        private static void AppendObj(StringBuilder sb, ref bool first, string id, string kind, Transform t, string zone)
        {
            if (!first) sb.Append(",");
            first = false;
            Vector3 p = t.position;
            Vector3 e = t.eulerAngles;
            sb.AppendFormat(
                "{{\"id\":\"{0}\",\"kind\":\"{1}\",\"pos\":{{\"x\":{2},\"y\":{3},\"z\":{4}}},\"rot_euler_deg\":{{\"x\":{5},\"y\":{6},\"z\":{7}}},\"zone\":\"{8}\"}}",
                Esc(id), Esc(kind), F(p.x), F(p.y), F(p.z), F(e.x), F(e.y), F(e.z), Esc(zone)
            );
        }

        private static void DumpNow()
        {
            var sb = new StringBuilder();
            sb.Append("{\n");

            // LandPlots
            sb.Append("  \"land_plots\": [");
            bool first = true;
            foreach (var p in UnityEngine.Object.FindObjectsOfType<LandPlotLocation>())
                AppendObj(sb, ref first, p.id, "LandPlotLocation", p.transform, "");
            sb.Append("],\n");

            // GadgetSites
            sb.Append("  \"gadget_sites\": [");
            first = true;
            foreach (var g in UnityEngine.Object.FindObjectsOfType<GadgetSite>())
                AppendObj(sb, ref first, g.id, "GadgetSite", g.transform, "");
            sb.Append("],\n");

            // Gordos
            sb.Append("  \"gordos\": [");
            first = true;
            foreach (var g in UnityEngine.Object.FindObjectsOfType<GordoEat>())
                AppendObj(sb, ref first, g.id, "GordoEat", g.transform, g.GetZoneId().ToString());
            sb.Append("],\n");

            // TreasurePods
            sb.Append("  \"treasure_pods\": [");
            first = true;
            foreach (var t in UnityEngine.Object.FindObjectsOfType<TreasurePod>())
                AppendObj(sb, ref first, t.id, "TreasurePod", t.transform, t.GetZoneId().ToString());
            sb.Append("],\n");

            // AccessDoors
            sb.Append("  \"access_doors\": [");
            first = true;
            foreach (var d in UnityEngine.Object.FindObjectsOfType<AccessDoor>())
                AppendObj(sb, ref first, d.id, "AccessDoor", d.transform, "");
            sb.Append("],\n");

            // Item Colors
            sb.Append("  \"item_colors\": {");
            first = true;
            if (SRSingleton<GameContext>.Instance != null && SRSingleton<GameContext>.Instance.LookupDirector != null)
            {
                var lookup = SRSingleton<GameContext>.Instance.LookupDirector;
                foreach (Identifiable.Id id in Enum.GetValues(typeof(Identifiable.Id)))
                {
                    if (id == Identifiable.Id.NONE) continue;
                    Color c = lookup.GetColor(id);
                    if (!first) sb.Append(",");
                    first = false;
                    sb.AppendFormat("\"{0}\":[{1},{2},{3},{4}]", (int)id, F(c.r), F(c.g), F(c.b), F(c.a));
                }
            }
            sb.Append("}\n}\n");

            string outPath = Path.Combine(Application.dataPath, "..", "world_dump.json");
            File.WriteAllText(outPath, sb.ToString(), Encoding.UTF8);
            Debug.Log("[SrWorldDumper] Exported world_dump.json to " + outPath);
        }
    }
}
