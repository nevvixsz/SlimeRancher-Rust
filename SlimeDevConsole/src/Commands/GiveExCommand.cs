using System;
using System.Collections.Generic;
using UnityEngine;

namespace SlimeDevConsole.Commands
{
    internal sealed class GiveExCommand : DevCommandBase
    {
        public GiveExCommand()
            : base("выдать",
                "выдать <предмет/гаджет> [кол-во] [слот 1-5]",
                "Выдаёт предмет в вакуумпак (сверх лимита стака!), ресурс в переработку или гаджет в инвентарь по-русски и по-английски.")
        {
        }

        public override List<string> AutoComplete(int argIndex, string argText)
        {
            if (argIndex == 0)
            {
                return RussianCatalog.GetAutoCompleteList();
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
                CommandHelpers.Log("Примеры: 'выдать фазовый_лимон 50' | 'выдать древняя_вода 20 5' | 'выдать дрон 4'");
                return true;
            }

            RussianCatalog.EnsureInitialized();

            int count = 10;
            int explicitSlot = -1;
            int nameEnd = args.Length;

            if (nameEnd >= 2 && CommandHelpers.TryParseInt(args[nameEnd - 1], out int lastInt))
            {
                if (nameEnd >= 3 && CommandHelpers.TryParseInt(args[nameEnd - 2], out int prevInt))
                {
                    count = Mathf.Clamp(prevInt, 1, 9999);
                    explicitSlot = Mathf.Clamp(lastInt - 1, 0, 4);
                    nameEnd -= 2;
                }
                else
                {
                    count = Mathf.Clamp(lastInt, 1, 9999);
                    nameEnd -= 1;
                }
            }

            string query = string.Join(" ", args, 0, nameEnd);
            CatalogEntry entry;
            List<string> suggestions;
            if (!RussianCatalog.TryResolveAny(query, out entry, out suggestions))
            {
                CommandHelpers.LogError("Предмет не найден: '" + query + "'.");
                if (suggestions.Count > 0)
                {
                    CommandHelpers.Log("Похожие варианты: " + string.Join(", ", suggestions.ToArray()));
                }
                return true;
            }

            if (entry.Kind == CatalogKind.Gadget)
            {
                if (ctx.GadgetDirector == null)
                {
                    CommandHelpers.LogError("GadgetDirector недоступен.");
                    return true;
                }

                if (!ctx.GadgetDirector.HasBlueprint(entry.GadgetId))
                {
                    ctx.GadgetDirector.AddBlueprint(entry.GadgetId);
                }

                int giveCount = Mathf.Min(count, 100);
                for (int i = 0; i < giveCount; i++)
                {
                    ctx.GadgetDirector.AddGadget(entry.GadgetId);
                }
                CommandHelpers.LogSuccess(string.Format("Выдано в инвентарь гаджетов (клавиша T): '{0}' [{1}] x{2} (+открыт чертёж).",
                    entry.DisplayRu, entry.GadgetId, giveCount));
                return true;
            }

            if (entry.Kind == CatalogKind.ResourceSpawner)
            {
                CommandHelpers.LogError("Деревья и грядки нельзя положить в инвентарь — используйте 'спавн " + query + "'.");
                return true;
            }

            Identifiable.Id id = entry.ActorId;
            if (id == Identifiable.Id.KEY)
            {
                int keysToAdd = Mathf.Min(count, 20);
                for (int i = 0; i < keysToAdd; i++)
                {
                    ctx.PlayerState.KeyAdded();
                }
                CommandHelpers.LogSuccess(string.Format("Добавлено слаймовых ключей: +{0} шт.", keysToAdd));
                return true;
            }

            // If user explicitly requests water/magic water without slot, default to slot 5 (index 4)
            if (explicitSlot < 0 && (id == Identifiable.Id.WATER_LIQUID || id == Identifiable.Id.MAGIC_WATER_LIQUID))
            {
                explicitSlot = 4;
            }

            Ammo ammo = ctx.PlayerState.Ammo;
            if (ammo == null)
            {
                CommandHelpers.LogError("Инвентарь (Ammo) недоступен.");
                return true;
            }

            bool added = false;
            if (explicitSlot >= 0)
            {
                added = ammo.MaybeAddToSpecificSlot(id, null, explicitSlot, count, true);
            }
            else
            {
                // First try matching an existing slot or empty slot with overflow=true
                int usableSlots = ammo.GetUsableSlotCount();
                for (int s = 0; s < usableSlots; s++)
                {
                    if (ammo.GetSlotName(s) == id)
                    {
                        added = ammo.MaybeAddToSpecificSlot(id, null, s, count, true);
                        if (added) break;
                    }
                }
                if (!added)
                {
                    for (int s = 0; s < usableSlots; s++)
                    {
                        if (ammo.GetSlotCount(s) == 0)
                        {
                            added = ammo.MaybeAddToSpecificSlot(id, null, s, count, true);
                            if (added) break;
                        }
                    }
                }
                if (!added)
                {
                    int cur = ammo.GetSelectedAmmoIdx();
                    added = ammo.MaybeAddToSpecificSlot(id, null, cur, count, true);
                }
            }

            if (added)
            {
                CommandHelpers.LogSuccess(string.Format("Выдано в вакуумпак: '{0}' [{1}] x{2}.", entry.DisplayRu, id, count));
                return true;
            }

            // Fallback: maybe it's a refinery resource (Plort / Craft)
            if (ctx.GadgetDirector != null && (Identifiable.IsPlort(id) || Identifiable.IsCraft(id)))
            {
                int refAdded = ctx.GadgetDirector.AddToRefinery(id, count, true);
                if (refAdded > 0)
                {
                    CommandHelpers.LogSuccess(string.Format("Добавлено прямо в Переработку (Refinery): '{0}' [{1}] x{2}.", entry.DisplayRu, id, refAdded));
                    return true;
                }
            }

            CommandHelpers.LogError(string.Format("'{0}' [{1}] нельзя поместить в слот вакуумпака (нет пустого слота или объект не переносится). Попробуйте: 'спавн {2}'.",
                entry.DisplayRu, id, query));
            return true;
        }
    }

    internal sealed class RefineryCommand : DevCommandBase
    {
        public RefineryCommand()
            : base("переработка",
                "переработка <ресурс/плорт> [кол-во] | переработка все [кол-во]",
                "Добавляет ресурсы или плорты напрямую в переработку Лаборатории (или сразу все ресурсы).")
        {
        }

        public override List<string> AutoComplete(int argIndex, string argText)
        {
            if (argIndex == 0) return RussianCatalog.GetAutoCompleteList();
            return null;
        }

        public override bool Execute(string[] args)
        {
            SceneContext ctx;
            GameContext gctx;
            if (!CommandHelpers.EnsureInGame(out ctx, out gctx) || ctx.GadgetDirector == null)
            {
                return true;
            }

            if (args == null || args.Length == 0)
            {
                CommandHelpers.Log("Использование: " + Usage);
                return true;
            }

            string first = RussianCatalog.Normalize(args[0]);
            if (first == "все" || first == "all")
            {
                int amt = 50;
                if (args.Length >= 2 && CommandHelpers.TryParseInt(args[1], out int parsed))
                {
                    amt = Mathf.Clamp(parsed, 1, 999);
                }
                int totalTypes = 0;
                foreach (Identifiable.Id id in Enum.GetValues(typeof(Identifiable.Id)))
                {
                    if (GadgetDirector.IsRefineryResource(id) && ctx.GadgetDirector.AddToRefinery(id, amt, true) > 0)
                    {
                        totalTypes++;
                    }
                }
                CommandHelpers.LogSuccess(string.Format("В переработку добавлено +{0} шт. для всех {1} видов ресурсов и плортов!", amt, totalTypes));
                return true;
            }

            int count = 50;
            int nameEnd = args.Length;
            if (nameEnd >= 2 && CommandHelpers.TryParseInt(args[nameEnd - 1], out int c))
            {
                count = Mathf.Clamp(c, 1, 9999);
                nameEnd--;
            }

            string query = string.Join(" ", args, 0, nameEnd);
            Identifiable.Id resId;
            string disp;
            if (!RussianCatalog.TryResolveActor(query, out resId, out disp))
            {
                CommandHelpers.LogError("Не найдено: '" + query + "'.");
                return true;
            }

            int added = ctx.GadgetDirector.AddToRefinery(resId, count, true);
            if (added > 0)
            {
                CommandHelpers.LogSuccess(string.Format("В переработку добавлено '{0}' [{1}] x{2}.", disp, resId, added));
            }
            else
            {
                CommandHelpers.LogError("Этот объект не принимается переработкой (" + resId + ").");
            }
            return true;
        }
    }
}
