using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using HarmonyLib;
using MonomiPark.SlimeRancher.DataModel;
using UnityEngine;

namespace SlimeCoop.World
{
    internal static class ClientProfileStore
    {
        private static readonly AccessTools.FieldRef<PlayerState, PlayerModel> PlayerModelRef =
            AccessTools.FieldRefAccess<PlayerState, PlayerModel>("model");

        private static readonly AccessTools.FieldRef<PediaDirector, PediaModel> PediaModelRef =
            AccessTools.FieldRefAccess<PediaDirector, PediaModel>("pediaModel");

        private static string GetProfilePath(string hostServerId)
        {
            string safe = SanitizeId(hostServerId);
            string dir = Path.Combine(
                Application.persistentDataPath,
                "SRML", "Config", "slimecoop", "profiles");
            Directory.CreateDirectory(dir);
            return Path.Combine(dir, safe + ".profile");
        }

        private static string SanitizeId(string id)
        {
            if (string.IsNullOrEmpty(id)) return "default";
            StringBuilder sb = new StringBuilder(id.Length);
            for (int i = 0; i < id.Length; i++)
            {
                char c = id[i];
                if ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_' || c == '-')
                {
                    sb.Append(c);
                }
                else
                {
                    sb.Append('_');
                }
            }
            return sb.ToString();
        }

        public static void SaveCurrentClientProfile(string hostServerId)
        {
            if (string.IsNullOrEmpty(hostServerId)) return;
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.PlayerState == null || ctx.PediaDirector == null) return;

            PlayerModel pModel = PlayerModelRef(ctx.PlayerState);
            PediaModel pedModel = PediaModelRef(ctx.PediaDirector);
            if (pModel == null || pedModel == null) return;

            StringBuilder sb = new StringBuilder(1024);
            sb.Append("currency=").Append(pModel.currency).Append('\n');
            sb.Append("keys=").Append(pModel.keys).Append('\n');

            // Upgrades
            sb.Append("upgrades=");
            for (int i = 0; i < pModel.upgrades.Count; i++)
            {
                if (i > 0) sb.Append(',');
                sb.Append((int)pModel.upgrades[i]);
            }
            sb.Append('\n');

            // Unlocked map zones
            sb.Append("zones=");
            bool firstZone = true;
            foreach (ZoneDirector.Zone z in pModel.unlockedZoneMaps)
            {
                if (!firstZone) sb.Append(',');
                firstZone = false;
                sb.Append((int)z);
            }
            sb.Append('\n');

            // Personal Ammo (slots 0..4)
            Ammo ammo = ctx.PlayerState.Ammo;
            sb.Append("ammo=");
            if (ammo != null)
            {
                int slotCount = Math.Min(5, ammo.GetUsableSlotCount());
                for (int s = 0; s < slotCount; s++)
                {
                    if (s > 0) sb.Append(';');
                    sb.Append((int)ammo.GetSlotName(s)).Append(':').Append(ammo.GetSlotCount(s));
                }
            }
            sb.Append('\n');

            // Personal Slimepedia
            sb.Append("pedia=");
            bool firstPed = true;
            foreach (PediaDirector.Id pid in pedModel.unlocked)
            {
                if (!firstPed) sb.Append(',');
                firstPed = false;
                sb.Append((int)pid);
            }
            sb.Append('\n');

            try
            {
                File.WriteAllText(GetProfilePath(hostServerId), sb.ToString(), Encoding.UTF8);
            }
            catch
            {
            }
        }

        public static void LoadOrInitClientProfile(string hostServerId)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.PlayerState == null || ctx.PediaDirector == null) return;

            PlayerModel pModel = PlayerModelRef(ctx.PlayerState);
            PediaModel pedModel = PediaModelRef(ctx.PediaDirector);
            if (pModel == null || pedModel == null) return;

            string path = GetProfilePath(hostServerId);
            if (!File.Exists(path))
            {
                // Fresh personal character on this server (upgrades/pedia/inventory belong only to this player)
                pModel.SetCurrency(250);
                pModel.SetKeys(0);
                pModel.SetUpgrades(new List<PlayerState.Upgrade>());
                pModel.InitZoneMaps();
                if (ctx.PlayerState.Ammo != null)
                {
                    ctx.PlayerState.Ammo.Clear();
                }
                pedModel.ResetUnlocked(ctx.PediaDirector.initUnlocked ?? new PediaDirector.Id[0]);
                return;
            }

            try
            {
                string[] lines = File.ReadAllLines(path, Encoding.UTF8);
                Dictionary<string, string> kv = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
                for (int i = 0; i < lines.Length; i++)
                {
                    int eq = lines[i].IndexOf('=');
                    if (eq > 0)
                    {
                        kv[lines[i].Substring(0, eq).Trim()] = lines[i].Substring(eq + 1).Trim();
                    }
                }

                if (kv.TryGetValue("currency", out string curStr) && int.TryParse(curStr, NumberStyles.Integer, CultureInfo.InvariantCulture, out int cur))
                {
                    pModel.SetCurrency(cur);
                }
                if (kv.TryGetValue("keys", out string keyStr) && int.TryParse(keyStr, NumberStyles.Integer, CultureInfo.InvariantCulture, out int keys))
                {
                    pModel.SetKeys(keys);
                }

                List<PlayerState.Upgrade> upgrades = new List<PlayerState.Upgrade>();
                if (kv.TryGetValue("upgrades", out string upgStr) && !string.IsNullOrEmpty(upgStr))
                {
                    string[] parts = upgStr.Split(',');
                    for (int i = 0; i < parts.Length; i++)
                    {
                        if (int.TryParse(parts[i], out int val))
                        {
                            upgrades.Add((PlayerState.Upgrade)val);
                        }
                    }
                }
                pModel.SetUpgrades(upgrades);

                List<ZoneDirector.Zone> zones = new List<ZoneDirector.Zone>();
                if (kv.TryGetValue("zones", out string zStr) && !string.IsNullOrEmpty(zStr))
                {
                    string[] parts = zStr.Split(',');
                    for (int i = 0; i < parts.Length; i++)
                    {
                        if (int.TryParse(parts[i], out int val))
                        {
                            zones.Add((ZoneDirector.Zone)val);
                        }
                    }
                }
                if (zones.Count == 0) zones.Add(ZoneDirector.Zone.RANCH);
                pModel.SetUnlockedZoneMaps(zones);

                if (ctx.PlayerState.Ammo != null)
                {
                    ctx.PlayerState.Ammo.Clear();
                    if (kv.TryGetValue("ammo", out string ammoStr) && !string.IsNullOrEmpty(ammoStr))
                    {
                        string[] slots = ammoStr.Split(';');
                        for (int s = 0; s < slots.Length && s < 5; s++)
                        {
                            int colon = slots[s].IndexOf(':');
                            if (colon <= 0) continue;
                            if (int.TryParse(slots[s].Substring(0, colon), out int itemId) &&
                                int.TryParse(slots[s].Substring(colon + 1), out int count) &&
                                itemId > 0 && count > 0)
                            {
                                ctx.PlayerState.Ammo.MaybeAddToSpecificSlot((Identifiable.Id)itemId, null, s, count, true);
                            }
                        }
                    }
                }

                pedModel.ResetUnlocked(ctx.PediaDirector.initUnlocked ?? new PediaDirector.Id[0]);
                if (kv.TryGetValue("pedia", out string pedStr) && !string.IsNullOrEmpty(pedStr))
                {
                    string[] parts = pedStr.Split(',');
                    List<PediaDirector.Id> ids = new List<PediaDirector.Id>(parts.Length);
                    for (int i = 0; i < parts.Length; i++)
                    {
                        if (int.TryParse(parts[i], out int val))
                        {
                            ids.Add((PediaDirector.Id)val);
                        }
                    }
                    if (ids.Count > 0)
                    {
                        pedModel.Unlock(ids.ToArray());
                    }
                }
            }
            catch
            {
            }
        }
    }
}
