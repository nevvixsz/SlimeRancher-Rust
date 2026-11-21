using System;
using System.Collections.Generic;
using UnityEngine;

namespace SlimeDevConsole.Commands
{
    internal sealed class TeleportExCommand : DevCommandBase
    {
        private static readonly Dictionary<string, ZoneDirector.Zone> ZoneAliases =
            new Dictionary<string, ZoneDirector.Zone>(StringComparer.OrdinalIgnoreCase)
            {
                { "ранчо", ZoneDirector.Zone.RANCH },
                { "дом", ZoneDirector.Zone.RANCH },
                { "ranch", ZoneDirector.Zone.RANCH },
                { "home", ZoneDirector.Zone.RANCH },

                { "риф", ZoneDirector.Zone.REEF },
                { "сухой_риф", ZoneDirector.Zone.REEF },
                { "reef", ZoneDirector.Zone.REEF },

                { "карьер", ZoneDirector.Zone.QUARRY },
                { "индиго_карьер", ZoneDirector.Zone.QUARRY },
                { "quarry", ZoneDirector.Zone.QUARRY },

                { "мхи", ZoneDirector.Zone.MOSS },
                { "мшистый_покров", ZoneDirector.Zone.MOSS },
                { "moss", ZoneDirector.Zone.MOSS },

                { "пустыня", ZoneDirector.Zone.DESERT },
                { "стеклянная_пустыня", ZoneDirector.Zone.DESERT },
                { "desert", ZoneDirector.Zone.DESERT },

                { "руины", ZoneDirector.Zone.RUINS },
                { "древние_руины", ZoneDirector.Zone.RUINS },
                { "ruins", ZoneDirector.Zone.RUINS },

                { "переход_руин", ZoneDirector.Zone.RUINS_TRANSITION },
                { "ruins_transition", ZoneDirector.Zone.RUINS_TRANSITION },

                { "дебри", ZoneDirector.Zone.WILDS },
                { "дикие_дебри", ZoneDirector.Zone.WILDS },
                { "wilds", ZoneDirector.Zone.WILDS },

                { "огден", ZoneDirector.Zone.OGDEN_RANCH },
                { "ранчо_огдена", ZoneDirector.Zone.OGDEN_RANCH },
                { "ogden", ZoneDirector.Zone.OGDEN_RANCH },

                { "долина", ZoneDirector.Zone.VALLEY },
                { "мочи_долина", ZoneDirector.Zone.VALLEY },
                { "valley", ZoneDirector.Zone.VALLEY },

                { "мочи", ZoneDirector.Zone.MOCHI_RANCH },
                { "поместье_мочи", ZoneDirector.Zone.MOCHI_RANCH },
                { "mochi", ZoneDirector.Zone.MOCHI_RANCH },

                { "виктори", ZoneDirector.Zone.VIKTOR_LAB },
                { "лаборатория_виктори", ZoneDirector.Zone.VIKTOR_LAB },
                { "viktor", ZoneDirector.Zone.VIKTOR_LAB },

                { "слаймуляция", ZoneDirector.Zone.SLIMULATIONS },
                { "slimulation", ZoneDirector.Zone.SLIMULATIONS }
            };

        public TeleportExCommand()
            : base("телепорт",
                "телепорт <зона | прицел | гордо | x y z | список>",
                "Телепортирует игрока в любую зону, к ближайшему Гордо, в точку прицела или по координатам.")
        {
        }

        public override List<string> AutoComplete(int argIndex, string argText)
        {
            if (argIndex == 0)
            {
                return new List<string>
                {
                    "ранчо", "риф", "карьер", "мхи", "пустыня", "руины", "дебри",
                    "огден", "мочи", "долина", "виктори", "слаймуляция", "прицел", "гордо", "список"
                };
            }
            return null;
        }

        public override bool Execute(string[] args)
        {
            SceneContext ctx;
            GameContext gctx;
            if (!CommandHelpers.EnsureInGame(out ctx, out gctx))
            {
                return true;
            }

            if (args == null || args.Length == 0 || RussianCatalog.Normalize(args[0]) == "список" || RussianCatalog.Normalize(args[0]) == "list")
            {
                CommandHelpers.Log("Доступные точки: ранчо, риф, карьер, мхи, пустыня, руины, переход_руин, дебри, огден, мочи, долина, виктори, слаймуляция, прицел, гордо, или <x> <y> <z>.");
                return true;
            }

            TeleportablePlayer tpPlayer = ctx.Player.GetComponent<TeleportablePlayer>();
            if (tpPlayer == null)
            {
                CommandHelpers.LogError("Компонент TeleportablePlayer не найден на игроке.");
                return true;
            }

            // Coordinate teleport: телепорт x y z
            if (args.Length >= 3 &&
                CommandHelpers.TryParseFloat(args[0], out float tx) &&
                CommandHelpers.TryParseFloat(args[1], out float ty) &&
                CommandHelpers.TryParseFloat(args[2], out float tz))
            {
                RegionRegistry.RegionSetId curSet = CommandHelpers.GetCurrentRegionSetId(ctx);
                tpPlayer.TeleportTo(new Vector3(tx, ty, tz), curSet, null, false, true);
                CommandHelpers.LogSuccess(string.Format("Телепорт по координатам: ({0:0.0}, {1:0.0}, {2:0.0}).", tx, ty, tz));
                return true;
            }

            string key = RussianCatalog.Normalize(string.Join("_", args));

            if (key == "прицел" || key == "aim" || key == "ray")
            {
                Camera cam = Camera.main;
                if (cam == null) return true;
                RaycastHit hit;
                if (Physics.Raycast(cam.transform.position + cam.transform.forward, cam.transform.forward, out hit, 600f, ~0, QueryTriggerInteraction.Ignore))
                {
                    RegionRegistry.RegionSetId curSet = CommandHelpers.GetCurrentRegionSetId(ctx);
                    tpPlayer.TeleportTo(hit.point + Vector3.up * 1.2f, curSet, null, false, true);
                    CommandHelpers.LogSuccess(string.Format("Телепорт на точку прицела ({0:0.0} м).", hit.distance));
                }
                else
                {
                    CommandHelpers.LogError("Луч прицела никуда не попал (слишком далеко).");
                }
                return true;
            }

            if (key == "гордо" || key == "gordo")
            {
                return TeleportToNearestGordo(ctx, tpPlayer);
            }

            ZoneDirector.Zone targetZone;
            if (ZoneAliases.TryGetValue(key, out targetZone))
            {
                return TeleportToZone(ctx, tpPlayer, targetZone);
            }

            // Try matching TeleportDestination directly
            TeleportDestination[] allDests = UnityEngine.Object.FindObjectsOfType<TeleportDestination>();
            for (int i = 0; i < allDests.Length; i++)
            {
                TeleportDestination d = allDests[i];
                if (d != null && !string.IsNullOrEmpty(d.teleportDestinationName) &&
                    RussianCatalog.Normalize(d.teleportDestinationName).Contains(key))
                {
                    tpPlayer.TeleportTo(d.GetPosition(), d.regionSetId, d.GetEulerAngles(), true, true);
                    CommandHelpers.LogSuccess("Телепорт к точке назначения: " + d.teleportDestinationName);
                    return true;
                }
            }

            CommandHelpers.LogError("Неизвестное место: '" + string.Join(" ", args) + "'. Напишите 'телепорт список'.");
            return true;
        }

        private static bool TeleportToNearestGordo(SceneContext ctx, TeleportablePlayer tpPlayer)
        {
            GordoEat[] gordos = UnityEngine.Object.FindObjectsOfType<GordoEat>();
            Vector3 p = ctx.Player.transform.position;
            GordoEat best = null;
            float bestDist = float.MaxValue;

            for (int i = 0; i < gordos.Length; i++)
            {
                GordoEat g = gordos[i];
                if (g == null || !g.gameObject.activeInHierarchy) continue;
                float d = Vector3.SqrMagnitude(g.transform.position - p);
                if (d > 25f && d < bestDist) // skip one we are standing right next to
                {
                    bestDist = d;
                    best = g;
                }
            }

            if (best == null)
            {
                CommandHelpers.LogError("Активных Гордо поблизости (в загруженных регионах) не найдено.");
                return true;
            }

            RegionRegistry.RegionSetId setId = CommandHelpers.GetCurrentRegionSetId(ctx);
            RegionMember rm = best.GetComponent<RegionMember>();
            if (rm != null) setId = rm.setId;

            Vector3 pos = best.transform.position + best.transform.forward * 4f + Vector3.up * 1.5f;
            tpPlayer.TeleportTo(pos, setId, null, true, true);
            CommandHelpers.LogSuccess(string.Format("Телепорт к Гордо '{0}' ({1:0.0} м).", best.name, Mathf.Sqrt(bestDist)));
            return true;
        }

        private static bool TeleportToZone(SceneContext ctx, TeleportablePlayer tpPlayer, ZoneDirector.Zone zone)
        {
            RegionRegistry.RegionSetId setId = ZoneDirector.GetRegionSetId(zone);

            // 1. Prefer an active TeleportDestination in that zone/regionSet
            TeleportDestination[] dests = UnityEngine.Object.FindObjectsOfType<TeleportDestination>();
            for (int i = 0; i < dests.Length; i++)
            {
                TeleportDestination d = dests[i];
                if (d != null && d.regionSetId == setId)
                {
                    // Check if destination belongs to target zone via RegionRegistry
                    if (ctx.RegionRegistry != null)
                    {
                        List<Region> regs = new List<Region>();
                        ctx.RegionRegistry.GetContaining(setId, ref regs, d.GetPosition());
                        for (int r = 0; r < regs.Count; r++)
                        {
                            if (regs[r] != null && regs[r].GetZoneId() == zone)
                            {
                                tpPlayer.TeleportTo(d.GetPosition(), setId, d.GetEulerAngles(), true, true);
                                CommandHelpers.LogSuccess("Телепорт в зону " + zone + " (через узел " + d.teleportDestinationName + ").");
                                return true;
                            }
                        }
                    }
                }
            }

            // 2. Fallback: find Region in target zone and pick its center + raycast to ground
            if (ctx.RegionRegistry != null)
            {
                List<Region> setRegions = ctx.RegionRegistry.GetRegions(setId);
                if (setRegions != null)
                {
                    for (int i = 0; i < setRegions.Count; i++)
                    {
                        Region reg = setRegions[i];
                        if (reg != null && reg.GetZoneId() == zone)
                        {
                            Vector3 center = reg.bounds.center;
                            Vector3 safePos = FindGroundAround(center, reg.bounds.extents.y + 80f);
                            tpPlayer.TeleportTo(safePos, setId, null, true, true);
                            CommandHelpers.LogSuccess("Телепорт в зону " + zone + " (" + reg.name + ").");
                            return true;
                        }
                    }
                }
            }

            // 3. Fallback: ZoneDirector transform
            ZoneDirector zd;
            if (ZoneDirector.zones.TryGetValue(zone, out zd) && zd != null)
            {
                Vector3 safePos = FindGroundAround(zd.transform.position, 120f);
                tpPlayer.TeleportTo(safePos, setId, null, true, true);
                CommandHelpers.LogSuccess("Телепорт в зону " + zone + " (по ZoneDirector).");
                return true;
            }

            CommandHelpers.LogError("Не удалось найти координаты зоны " + zone + ".");
            return true;
        }

        private static Vector3 FindGroundAround(Vector3 center, float probeHeight)
        {
            RaycastHit hit;
            Vector3 top = new Vector3(center.x, center.y + probeHeight, center.z);
            if (Physics.Raycast(top, Vector3.down, out hit, probeHeight * 2.5f, ~0, QueryTriggerInteraction.Ignore))
            {
                return hit.point + Vector3.up * 2f;
            }
            return center + Vector3.up * 4f;
        }
    }

    internal sealed class TimeExCommand : DevCommandBase
    {
        public TimeExCommand()
            : base("время",
                "время <пауза | идет | утро | день | вечер | ночь | час N | ускорение X>",
                "Управляет мировым временем: заморозка, смена времени суток или скорость игры.")
        {
        }

        public override List<string> AutoComplete(int argIndex, string argText)
        {
            if (argIndex == 0)
            {
                return new List<string> { "пауза", "идет", "утро", "день", "вечер", "ночь", "час", "ускорение" };
            }
            return null;
        }

        public override bool Execute(string[] args)
        {
            SceneContext ctx;
            GameContext gctx;
            if (!CommandHelpers.EnsureInGame(out ctx, out gctx) || ctx.TimeDirector == null)
            {
                return true;
            }

            if (args == null || args.Length == 0)
            {
                CommandHelpers.Log(string.Format("День {0}, время {1:00.0}ч | Пауза={2} | Speed=x{3:0.##}",
                    ctx.TimeDirector.CurrDay() + 1,
                    ctx.TimeDirector.CurrDayFraction() * 24f,
                    DevState.FreezeTime ? "ВКЛ" : "ВЫКЛ",
                    DevState.WorldTimeScale));
                return true;
            }

            string sub = RussianCatalog.Normalize(args[0]);
            if (sub == "пауза" || sub == "стоп" || sub == "freeze" || sub == "pause")
            {
                DevState.FreezeTime = true;
                CommandHelpers.LogSuccess("Мировое время ЗАМОРОЖЕНО.");
                return true;
            }
            if (sub == "идет" || sub == "продолжить" || sub == "старт" || sub == "resume" || sub == "unfreeze")
            {
                DevState.FreezeTime = false;
                DevState.WorldTimeScale = 1.0f;
                Time.timeScale = 1.0f;
                CommandHelpers.LogSuccess("Мировое время восстановлено (1.0x).");
                return true;
            }
            if (sub == "ускорение" || sub == "scale" || sub == "speed")
            {
                float scale = 2.0f;
                if (args.Length >= 2) CommandHelpers.TryParseFloat(args[1], out scale);
                DevState.WorldTimeScale = Mathf.Clamp(scale, 0.05f, 10f);
                Time.timeScale = DevState.WorldTimeScale;
                CommandHelpers.LogSuccess("Масштаб скорости игры (Time.timeScale): x" + DevState.WorldTimeScale.ToString("0.##"));
                return true;
            }

            float targetHour = -1f;
            if (sub == "утро" || sub == "morning") targetHour = 6.5f;
            else if (sub == "день" || sub == "day" || sub == "noon") targetHour = 12.0f;
            else if (sub == "вечер" || sub == "evening") targetHour = 18.0f;
            else if (sub == "ночь" || sub == "night") targetHour = 23.0f;
            else if ((sub == "час" || sub == "hour") && args.Length >= 2 && CommandHelpers.TryParseFloat(args[1], out float h))
            {
                targetHour = Mathf.Repeat(h, 24f);
            }

            if (targetHour >= 0f)
            {
                double until = ctx.TimeDirector.GetHourAfter(0, targetHour);
                if (until <= ctx.TimeDirector.WorldTime())
                {
                    until = ctx.TimeDirector.GetHourAfter(1, targetHour);
                }
                ctx.TimeDirector.FastForwardTo(until);
                CommandHelpers.LogSuccess(string.Format("Перемотка времени на {0:00.0}:00...", targetHour));
                return true;
            }

            CommandHelpers.LogError("Неизвестный режим: " + Usage);
            return true;
        }
    }

    internal sealed class UnlockExCommand : DevCommandBase
    {
        public UnlockExCommand()
            : base("открыть",
                "открыть <двери | оазисы | капсулы | педия | апгрейды | карты | 7zee | все>",
                "Мгновенно открывает запертые двери, оазисы, капсулы, Слаймопедию, улучшения, карты или ранг 7Zee.")
        {
        }

        public override List<string> AutoComplete(int argIndex, string argText)
        {
            if (argIndex == 0)
            {
                return new List<string> { "двери", "оазисы", "капсулы", "педия", "апгрейды", "карты", "7zee", "все" };
            }
            return null;
        }

        public override bool Execute(string[] args)
        {
            SceneContext ctx;
            GameContext gctx;
            if (!CommandHelpers.EnsureInGame(out ctx, out gctx))
            {
                return true;
            }

            if (args == null || args.Length == 0)
            {
                CommandHelpers.Log("Использование: " + Usage);
                return true;
            }

            string what = RussianCatalog.Normalize(args[0]);
            bool doAll = (what == "все" || what == "all");

            if (doAll || what == "двери" || what == "doors")
            {
                int count = 0;
                AccessDoor[] doors = UnityEngine.Object.FindObjectsOfType<AccessDoor>();
                for (int i = 0; i < doors.Length; i++)
                {
                    if (doors[i] != null && doors[i].CurrState != AccessDoor.State.OPEN)
                    {
                        doors[i].CurrState = AccessDoor.State.OPEN;
                        count++;
                    }
                }
                if (ctx.GameModel != null)
                {
                    foreach (AccessDoorModel dm in ctx.GameModel.AllDoors().Values)
                    {
                        if (dm != null && dm.state != AccessDoor.State.OPEN)
                        {
                            dm.state = AccessDoor.State.OPEN;
                            count++;
                        }
                    }
                }
                CommandHelpers.LogSuccess("Открыто дверей/ворот: " + count);
            }

            if (doAll || what == "оазисы" || what == "oasis" || what == "oases")
            {
                int count = 0;
                Oasis[] oases = UnityEngine.Object.FindObjectsOfType<Oasis>();
                for (int i = 0; i < oases.Length; i++)
                {
                    if (oases[i] != null)
                    {
                        oases[i].SetLive(true);
                        count++;
                    }
                }
                if (ctx.GameModel != null)
                {
                    foreach (OasisModel om in ctx.GameModel.AllOases().Values)
                    {
                        if (om != null && !om.isLive)
                        {
                            om.isLive = true;
                            count++;
                        }
                    }
                }
                CommandHelpers.LogSuccess("Оживлено оазисов Стеклянной пустыни: " + count);
            }

            if (doAll || what == "капсулы" || what == "pods")
            {
                int count = 0;
                TreasurePod[] pods = UnityEngine.Object.FindObjectsOfType<TreasurePod>();
                for (int i = 0; i < pods.Length; i++)
                {
                    if (pods[i] != null && pods[i].CurrState != TreasurePod.State.OPEN)
                    {
                        pods[i].CurrState = TreasurePod.State.OPEN;
                        count++;
                    }
                }
                CommandHelpers.LogSuccess("Открыто капсул сокровищ в текущих регионах: " + count);
            }

            if (doAll || what == "педия" || what == "слаймопедия" || what == "pedia")
            {
                if (ctx.PediaDirector != null)
                {
                    Array allIds = Enum.GetValues(typeof(PediaDirector.Id));
                    List<PediaDirector.Id> list = new List<PediaDirector.Id>();
                    foreach (PediaDirector.Id pid in allIds)
                    {
                        list.Add(pid);
                    }
                    ctx.PediaDirector.Unlock(list.ToArray());
                    CommandHelpers.LogSuccess("Разблокирована вся Слаймопедия (" + list.Count + " записей).");
                }
            }

            if (doAll || what == "апгрейды" || what == "улучшения" || what == "upgrades")
            {
                int added = 0;
                foreach (PlayerState.Upgrade upg in Enum.GetValues(typeof(PlayerState.Upgrade)))
                {
                    try
                    {
                        ctx.PlayerState.AddUpgrade(upg, false);
                        added++;
                    }
                    catch
                    {
                    }
                }
                CommandHelpers.LogSuccess("Выданы все улучшения вакуумпака и ранчо (" + added + ").");
            }

            if (doAll || what == "карты" || what == "карта" || what == "map" || what == "maps")
            {
                int unlocked = 0;
                foreach (ZoneDirector.Zone z in Enum.GetValues(typeof(ZoneDirector.Zone)))
                {
                    if (z != ZoneDirector.Zone.NONE)
                    {
                        try
                        {
                            ctx.PlayerState.UnlockMap(z);
                            unlocked++;
                        }
                        catch
                        {
                        }
                    }
                }
                CommandHelpers.LogSuccess("Открыта вся карта мира (" + unlocked + " зон).");
            }

            if (doAll || what == "7zee" || what == "ранг" || what == "клуб")
            {
                if (ctx.ProgressDirector != null)
                {
                    ctx.ProgressDirector.SetProgress(ProgressDirector.ProgressType.CORPORATE_PARTNER, 28);
                    CommandHelpers.LogSuccess("Установлен максимальный ранг клуба наград 7Zee (28/28)!");
                }
            }

            return true;
        }
    }

    internal sealed class MarketCommand : DevCommandBase
    {
        public MarketCommand()
            : base("рынок",
                "рынок [сброс]",
                "Сбрасывает насыщение плорт-маркета и пересчитывает максимальные цены.")
        {
        }

        public override bool Execute(string[] args)
        {
            SceneContext ctx;
            GameContext gctx;
            if (!CommandHelpers.EnsureInGame(out ctx, out gctx))
            {
                return true;
            }

            WorldModel wm = ctx.GameModel != null ? ctx.GameModel.GetWorldModel() : null;
            if (wm != null && wm.marketSaturation != null)
            {
                wm.marketSaturation.Clear();
            }

            if (ctx.EconomyDirector != null && wm != null && ctx.TimeDirector != null)
            {
                ctx.EconomyDirector.ResetPrices(wm, ctx.TimeDirector.CurrDay());
            }

            CommandHelpers.LogSuccess("Насыщение плорт-маркета обнулено, цены пересчитаны!");
            return true;
        }
    }

    internal sealed class InspectCommand : DevCommandBase
    {
        public InspectCommand()
            : base("инспект",
                "инспект",
                "Показывает детальную отладочную информацию об объекте под прицелом.")
        {
        }

        public override bool Execute(string[] args)
        {
            SceneContext ctx;
            GameContext gctx;
            if (!CommandHelpers.EnsureInGame(out ctx, out gctx))
            {
                return true;
            }

            Camera cam = Camera.main;
            if (cam == null) return true;

            RaycastHit hit;
            if (!Physics.Raycast(cam.transform.position + cam.transform.forward * 0.4f, cam.transform.forward, out hit, 120f, ~0, QueryTriggerInteraction.Collide))
            {
                CommandHelpers.Log("Под прицелом ничего нет (дальше 120м).");
                return true;
            }

            GameObject go = hit.collider != null ? hit.collider.gameObject : null;
            if (go == null) return true;

            Identifiable ident = go.GetComponentInParent<Identifiable>();
            GordoIdentifiable gordo = go.GetComponentInParent<GordoIdentifiable>();
            Gadget gadget = go.GetComponentInParent<Gadget>();
            SlimeEmotions emotions = go.GetComponentInParent<SlimeEmotions>();
            RegionMember rm = go.GetComponentInParent<RegionMember>();

            CommandHelpers.Log(string.Format("=== ИНСПЕКТ ({0:0.0} м) ===", hit.distance));
            CommandHelpers.Log(string.Format("GameObject: {0} | Layer: {1} ({2}) | Pos: {3}",
                go.name, LayerMask.LayerToName(go.layer), go.layer, hit.point));

            if (ident != null)
            {
                string ru = string.Empty;
                try { ru = Identifiable.GetName(ident.id, false); } catch { }
                CommandHelpers.Log(string.Format("Identifiable.Id: {0} | Имя: {1}", ident.id, ru));
            }
            if (gordo != null)
            {
                GordoEat ge = gordo.GetComponent<GordoEat>();
                CommandHelpers.Log(string.Format("Gordo: {0} | Съедено: {1}/{2}",
                    gordo.id, ge != null ? ge.GetEatenCount() : -1, ge != null ? ge.GetTargetCount() : -1));
            }
            if (gadget != null)
            {
                CommandHelpers.Log("Gadget: " + gadget.id);
            }
            if (emotions != null)
            {
                CommandHelpers.Log(string.Format("Эмоции: Голод={0:0.00} | Возбуждение={1:0.00} | Страх={2:0.00}",
                    emotions.GetCurr(SlimeEmotions.Emotion.HUNGER),
                    emotions.GetCurr(SlimeEmotions.Emotion.AGITATION),
                    emotions.GetCurr(SlimeEmotions.Emotion.FEAR)));
            }
            if (rm != null)
            {
                CommandHelpers.Log("RegionSetId: " + rm.setId);
            }

            return true;
        }
    }

    internal sealed class ClearWorldCommand : DevCommandBase
    {
        public ClearWorldCommand()
            : base("очистить",
                "очистить <тарры | плорты | слаймы | все_вокруг> [радиус]",
                "Удаляет тарров, плорты или слаймов в радиусе (по умолчанию 60м; 'тарры' без радиуса чистит всех).")
        {
        }

        public override List<string> AutoComplete(int argIndex, string argText)
        {
            if (argIndex == 0)
            {
                return new List<string> { "тарры", "плорты", "слаймы", "все_вокруг" };
            }
            return null;
        }

        public override bool Execute(string[] args)
        {
            SceneContext ctx;
            GameContext gctx;
            if (!CommandHelpers.EnsureInGame(out ctx, out gctx))
            {
                return true;
            }

            string target = (args != null && args.Length > 0) ? RussianCatalog.Normalize(args[0]) : "тарры";
            float radius = -1f;
            if (args != null && args.Length >= 2 && CommandHelpers.TryParseFloat(args[1], out float r))
            {
                radius = Mathf.Max(2f, r);
            }
            else if (target != "тарры" && target != "tarr" && target != "тарр")
            {
                radius = 60f;
            }

            Vector3 pPos = ctx.Player.transform.position;
            float sqrMax = radius > 0f ? radius * radius : float.MaxValue;

            Identifiable[] all = UnityEngine.Object.FindObjectsOfType<Identifiable>();
            int removed = 0;
            for (int i = 0; i < all.Length; i++)
            {
                Identifiable item = all[i];
                if (item == null || item.id == Identifiable.Id.PLAYER || item.id == Identifiable.Id.NONE)
                {
                    continue;
                }

                if (radius > 0f && Vector3.SqrMagnitude(item.transform.position - pPos) > sqrMax)
                {
                    continue;
                }

                bool match = false;
                if (target == "тарры" || target == "тарр" || target == "tarr")
                {
                    match = (item.id == Identifiable.Id.TARR_SLIME || item.id == Identifiable.Id.GLITCH_TARR_SLIME);
                }
                else if (target == "плорты" || target == "plorts")
                {
                    match = Identifiable.IsPlort(item.id);
                }
                else if (target == "слаймы" || target == "slimes")
                {
                    match = Identifiable.IsSlime(item.id);
                }
                else if (target == "все_вокруг" || target == "all")
                {
                    match = true;
                }

                if (match)
                {
                    Destroyer.DestroyActor(item.gameObject, "SlimeDevConsole.ClearWorld", true);
                    removed++;
                }
            }

            CommandHelpers.LogSuccess(string.Format("Удалено объектов ({0}): {1} шт.", target, removed));
            return true;
        }
    }

    internal sealed class FindCatalogCommand : DevCommandBase
    {
        public FindCatalogCommand()
            : base("найти",
                "найти <часть_слова>",
                "Ищет по русскому и английскому справочнику ID предметов, слаймов, Гордо и гаджетов.")
        {
        }

        public override bool Execute(string[] args)
        {
            string filter = (args != null && args.Length > 0) ? string.Join(" ", args) : string.Empty;
            List<string> results = RussianCatalog.Search(filter, 25);
            if (results.Count == 0)
            {
                CommandHelpers.Log("Ничего не найдено по запросу: '" + filter + "'.");
                return true;
            }

            CommandHelpers.Log(string.Format("Найдено ({0}):", results.Count));
            for (int i = 0; i < results.Count; i++)
            {
                CommandHelpers.Log("  " + results[i]);
            }
            return true;
        }
    }

    internal sealed class DevHelpCommand : DevCommandBase
    {
        public DevHelpCommand()
            : base("помощь",
                "помощь",
                "Показывает список всех команд SlimeDevConsole на русском и английском.")
        {
        }

        public override bool Execute(string[] args)
        {
            CommandHelpers.Log("=== SlimeDevConsole v1.0.0 — Команды (RU / EN) ===");
            CommandHelpers.Log("  спавн / spawnex <что> [кол-во] [дист]  — любой объект (фазовый_лимон, золотой_гордо, дрон, ключ, огненный_столб...)");
            CommandHelpers.Log("  ларго / largo <слайм1> <слайм2> [кол-во] — любой Ларго (напр. 'ларго розовый квантовый 3')");
            CommandHelpers.Log("  выдать / giveex <что> [кол-во] [слот]   — предмет в вак (игнор лимита!) или гаджет в инвентарь");
            CommandHelpers.Log("  переработка / refinery <что|все> [кол]  — добавить ресурсы напрямую в переработку");
            CommandHelpers.Log("  полет / полёт / fly [скорость]          — режим свободного полёта + скорость");
            CommandHelpers.Log("  скорость / speed <множитель>            — скорость бега и прыжка");
            CommandHelpers.Log("  бессмертие / бог / god                  — режим бога (без урона и радиации)");
            CommandHelpers.Log("  энергия / infenergy                     — бесконечная энергия/джетпак");
            CommandHelpers.Log("  вакуум / infvac                         — бесконечные предметы в вакуумпаке");
            CommandHelpers.Log("  деньги / money <сумма>                  — добавить/снять ньюбаксы");
            CommandHelpers.Log("  ключ / key [кол-во] | лечить / heal     — выдать ключи / восстановить ХП и энергию");
            CommandHelpers.Log("  телепорт / тп / tp <куда|прицел|гордо>  — быстрый телепорт в любую зону (напиши 'тп список')");
            CommandHelpers.Log("  время / time <пауза|идет|утро|день|...> — управление временем суток и скоростью игры");
            CommandHelpers.Log("  открыть / unlock <двери|оазисы|капсулы|педия|апгрейды|карты|7zee|все>");
            CommandHelpers.Log("  рынок / market | инспект / inspect | очистить / clearworld | найти / find <слово>");
            return true;
        }
    }
}
