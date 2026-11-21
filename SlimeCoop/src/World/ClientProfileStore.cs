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

        private static string GetSettingsPath()
        {
            string dir = Path.Combine(
                Application.persistentDataPath,
                "SRML", "Config", "slimecoop");
            Directory.CreateDirectory(dir);
            return Path.Combine(dir, "settings.cfg");
        }

        private static string GetHostServersPath()
        {
            string dir = Path.Combine(
                Application.persistentDataPath,
                "SRML", "Config", "slimecoop");
            Directory.CreateDirectory(dir);
            return Path.Combine(dir, "host_servers.cfg");
        }

        public static string GetOrCreateHostServerId(string worldKey)
        {
            string key = string.IsNullOrEmpty(worldKey) ? "default" : worldKey.Trim();
            try
            {
                string path = GetHostServersPath();
                Dictionary<string, string> map = new Dictionary<string, string>(StringComparer.Ordinal);
                if (File.Exists(path))
                {
                    string[] lines = File.ReadAllLines(path, Encoding.UTF8);
                    for (int i = 0; i < lines.Length; i++)
                    {
                        int eq = lines[i].IndexOf('=');
                        if (eq <= 0) continue;
                        string k = lines[i].Substring(0, eq).Trim();
                        string v = lines[i].Substring(eq + 1).Trim();
                        if (!string.IsNullOrEmpty(k) && !string.IsNullOrEmpty(v))
                        {
                            map[k] = v;
                        }
                    }
                }

                if (map.TryGetValue(key, out string existing) && !string.IsNullOrEmpty(existing))
                {
                    return existing;
                }

                string created = "srv_" + Guid.NewGuid().ToString("N").Substring(0, 12);
                map[key] = created;

                StringBuilder sb = new StringBuilder(map.Count * 48);
                foreach (KeyValuePair<string, string> kv in map)
                {
                    sb.Append(kv.Key.Replace("=", "_").Replace("\r", "").Replace("\n", ""))
                      .Append('=')
                      .AppendLine(kv.Value);
                }
                File.WriteAllText(path, sb.ToString(), Encoding.UTF8);
                return created;
            }
            catch
            {
                return "srv_" + SanitizeId(key);
            }
        }

        public static void LoadLocalPrefs()
        {
            try
            {
                string path = GetSettingsPath();
                if (!File.Exists(path)) return;
                string[] lines = File.ReadAllLines(path, Encoding.UTF8);
                for (int i = 0; i < lines.Length; i++)
                {
                    int eq = lines[i].IndexOf('=');
                    if (eq <= 0) continue;
                    string key = lines[i].Substring(0, eq).Trim();
                    string val = lines[i].Substring(eq + 1).Trim();
                    if (key == "nick" && !string.IsNullOrEmpty(val))
                    {
                        CoopSession.LocalNick = val.Length > 20 ? val.Substring(0, 20) : val;
                    }
                    else if (key == "color" && int.TryParse(val, out int col))
                    {
                        CoopSession.LocalColorIndex = Math.Abs(col) % 8;
                    }
                }
            }
            catch
            {
            }
        }

        public static void SaveLocalPrefs()
        {
            try
            {
                string path = GetSettingsPath();
                StringBuilder sb = new StringBuilder(128);
                sb.Append("nick=").AppendLine((CoopSession.LocalNick ?? "SlimePlayer").Replace("\r", "").Replace("\n", ""));
                sb.Append("color=").AppendLine(CoopSession.LocalColorIndex.ToString(CultureInfo.InvariantCulture));
                File.WriteAllText(path, sb.ToString(), Encoding.UTF8);
            }
            catch
            {
            }
        }

        private static string GetProfilePath(string hostServerId)
        {
            string safe = SanitizeId(hostServerId);
            string dir = Path.Combine(
                Application.persistentDataPath,
                "SRML", "Config", "slimecoop", "profiles_v2");
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
            if (ctx == null || ctx.PlayerState == null) return;

            PlayerModel pModel = PlayerModelRef(ctx.PlayerState);
            if (pModel == null) return;

            PediaModel pedModel = ctx.PediaDirector != null ? PediaModelRef(ctx.PediaDirector) : null;

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
            if (pedModel != null)
            {
                bool firstPed = true;
                foreach (PediaDirector.Id pid in pedModel.unlocked)
                {
                    if (!firstPed) sb.Append(',');
                    firstPed = false;
                    sb.Append((int)pid);
                }
            }
            sb.Append('\n');

            try
            {
                File.WriteAllText(GetProfilePath(hostServerId), sb.ToString(), Encoding.UTF8);
            }
            catch (Exception ex)
            {
                CoopLog.Warn("SaveCurrentClientProfile failed: " + ex.Message);
            }
        }

        public static void LoadOrInitClientProfile(string hostServerId)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.PlayerState == null)
            {
                CoopLog.Warn("LoadOrInitClientProfile: SceneContext/PlayerState is null");
                return;
            }

            PlayerModel pModel = PlayerModelRef(ctx.PlayerState);
            if (pModel == null)
            {
                CoopLog.Warn("LoadOrInitClientProfile: PlayerModel is null");
                return;
            }

            PediaModel pedModel = ctx.PediaDirector != null ? PediaModelRef(ctx.PediaDirector) : null;

            string path = GetProfilePath(hostServerId);
            if (!File.Exists(path))
            {
                // Fresh personal character on this server: start cleanly with 250 newbucks, zero upgrades, empty vac
                pModel.SetCurrency(250);
                pModel.SetCurrencyEverCollected(250);
                pModel.currencyDisplayOverride = null;
                pModel.currencyDisplayDelta = 0;
                pModel.SetKeys(0);
                pModel.SetUpgrades(new List<PlayerState.Upgrade>());
                pModel.unlockedZoneMaps.Clear();
                pModel.unlockedZoneMaps.Add(ZoneDirector.Zone.RANCH);
                ctx.PlayerState.SetHealth(ctx.PlayerState.GetMaxHealth());
                ctx.PlayerState.SetEnergy(ctx.PlayerState.GetMaxEnergy());
                ctx.PlayerState.SetRad(0);
                if (ctx.PlayerState.Ammo != null)
                {
                    ctx.PlayerState.Ammo.Clear();
                }
                if (pedModel != null && ctx.PediaDirector != null)
                {
                    pedModel.ResetUnlocked(ctx.PediaDirector.initUnlocked ?? new PediaDirector.Id[0]);
                }

                SaveCurrentClientProfile(hostServerId);
                CoopLog.Info("Initialized new client profile for serverId='" + hostServerId + "' with 250 currency");
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

                int loadedCur = 250;
                if (kv.TryGetValue("currency", out string curStr) && int.TryParse(curStr, NumberStyles.Integer, CultureInfo.InvariantCulture, out int cur))
                {
                    loadedCur = Math.Max(0, cur);
                }
                pModel.SetCurrency(loadedCur);
                pModel.SetCurrencyEverCollected(Math.Max(loadedCur, 250));
                pModel.currencyDisplayOverride = null;
                pModel.currencyDisplayDelta = 0;

                int loadedKeys = 0;
                if (kv.TryGetValue("keys", out string keyStr) && int.TryParse(keyStr, NumberStyles.Integer, CultureInfo.InvariantCulture, out int keys))
                {
                    loadedKeys = Math.Max(0, keys);
                }
                pModel.SetKeys(loadedKeys);

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

                if (pedModel != null && ctx.PediaDirector != null)
                {
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

                CoopLog.Info("Loaded existing client profile for serverId='" + hostServerId + "' currency=" + loadedCur);
            }
            catch (Exception ex)
            {
                CoopLog.Warn("LoadOrInitClientProfile parse failed, resetting to 250: " + ex.Message);
                pModel.SetCurrency(250);
                pModel.SetCurrencyEverCollected(250);
                pModel.currencyDisplayOverride = null;
                pModel.currencyDisplayDelta = 0;
            }
        }
    }
}
