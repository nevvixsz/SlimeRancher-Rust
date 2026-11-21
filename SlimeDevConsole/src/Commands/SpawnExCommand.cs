using System;
using System.Collections.Generic;
using UnityEngine;

namespace SlimeDevConsole.Commands
{
    internal sealed class SpawnExCommand : DevCommandBase
    {
        public SpawnExCommand()
            : base("спавн",
                "спавн <предмет/слайм/гордо/гаджет/ларго s1 s2> [кол-во] [дистанция]",
                "Спавнит любые объекты по-русски и по-английски (включая Гордо, Ларго, гаджеты, фазовый лимон, огненный столб, глитч-порталы).")
        {
        }

        public override List<string> AutoComplete(int argIndex, string argText)
        {
            if (argIndex == 0 || argIndex == 1)
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
                CommandHelpers.Log("Примеры: 'спавн фазовый_лимон 5' | 'спавн золотой_гордо' | 'спавн ларго розовый квантовый 3' | 'спавн дрон'");
                return true;
            }

            RussianCatalog.EnsureInitialized();

            // Support: спавн ларго <slime1> <slime2> [count]
            string firstNorm = RussianCatalog.Normalize(args[0]);
            if (firstNorm == "ларго" || firstNorm == "largo")
            {
                return SpawnLargoFromArgs(ctx, gctx, args, 1);
            }

            // Support: спавн розовый+квантовый [count]
            if (args[0].Contains("+"))
            {
                string[] parts = args[0].Split(new[] { '+' }, 2);
                if (parts.Length == 2)
                {
                    int count = 1;
                    if (args.Length >= 2)
                    {
                        CommandHelpers.TryParseInt(args[1], out count);
                    }
                    return SpawnLargoPair(ctx, gctx, parts[0], parts[1], Mathf.Clamp(count, 1, 100), 4.5f);
                }
            }

            // Parse trailing numeric args ([count] [distance]) while allowing multi-word Russian names!
            int spawnCount = 1;
            float distance = 4.5f;
            int nameEnd = args.Length;

            if (nameEnd >= 2 && CommandHelpers.TryParseFloat(args[nameEnd - 1], out float maybeLastNum))
            {
                if (nameEnd >= 3 && CommandHelpers.TryParseInt(args[nameEnd - 2], out int maybeCount))
                {
                    spawnCount = Mathf.Clamp(maybeCount, 1, 200);
                    distance = Mathf.Clamp(maybeLastNum, 1.5f, 60f);
                    nameEnd -= 2;
                }
                else
                {
                    spawnCount = Mathf.Clamp(Mathf.RoundToInt(maybeLastNum), 1, 200);
                    nameEnd -= 1;
                }
            }

            string query = string.Join(" ", args, 0, nameEnd);
            CatalogEntry entry;
            List<string> suggestions;
            if (!RussianCatalog.TryResolveAny(query, out entry, out suggestions))
            {
                CommandHelpers.LogError("Не найдено: '" + query + "'.");
                if (suggestions.Count > 0)
                {
                    CommandHelpers.Log("Похожие варианты: " + string.Join(", ", suggestions.ToArray()));
                }
                return true;
            }

            return SpawnResolvedEntry(ctx, gctx, entry, spawnCount, distance);
        }

        internal static bool SpawnLargoFromArgs(SceneContext ctx, GameContext gctx, string[] args, int startIdx)
        {
            if (args == null || args.Length < startIdx + 2)
            {
                CommandHelpers.LogError("Укажите два слайма для Ларго: например 'ларго розовый квантовый [кол-во]'.");
                return true;
            }

            int count = 1;
            if (args.Length >= startIdx + 3 && CommandHelpers.TryParseInt(args[startIdx + 2], out int parsed))
            {
                count = Mathf.Clamp(parsed, 1, 100);
            }

            return SpawnLargoPair(ctx, gctx, args[startIdx], args[startIdx + 1], count, 4.5f);
        }

        internal static bool SpawnLargoPair(SceneContext ctx, GameContext gctx, string s1, string s2, int count, float distance)
        {
            Identifiable.Id id1, id2;
            if (!RussianCatalog.TryResolveBaseSlime(s1, out id1))
            {
                CommandHelpers.LogError("Неизвестный первый слайм для Ларго: '" + s1 + "'.");
                return true;
            }
            if (!RussianCatalog.TryResolveBaseSlime(s2, out id2))
            {
                CommandHelpers.LogError("Неизвестный второй слайм для Ларго: '" + s2 + "'.");
                return true;
            }

            SlimeDefinitions defs = gctx.SlimeDefinitions;
            SlimeDefinition def1 = defs.GetSlimeByIdentifiableId(id1);
            SlimeDefinition def2 = defs.GetSlimeByIdentifiableId(id2);
            if (def1 == null || def2 == null)
            {
                CommandHelpers.LogError("У одного из выбранных типов нет SlimeDefinition.");
                return true;
            }

            SlimeDefinition largoDef = defs.GetLargoByBaseSlimes(def1, def2);
            if (largoDef == null)
            {
                CommandHelpers.LogError(string.Format("Комбинация Ларго ({0} + {1}) не поддерживается игрой (например, лужа/огонь/золото не скрещиваются).", id1, id2));
                return true;
            }

            GameObject prefab = gctx.LookupDirector.GetPrefab(largoDef.IdentifiableId);
            if (prefab == null)
            {
                CommandHelpers.LogError("Префаб Ларго не найден: " + largoDef.IdentifiableId);
                return true;
            }

            Quaternion rot;
            Vector3 center = CommandHelpers.GetAimSpawnPoint(ctx, distance, out rot);
            RegionRegistry.RegionSetId setId = CommandHelpers.GetCurrentRegionSetId(ctx);

            int spawned = 0;
            for (int i = 0; i < count; i++)
            {
                Vector3 jitter = new Vector3(
                    UnityEngine.Random.Range(-0.8f, 0.8f),
                    0.25f * (i % 3),
                    UnityEngine.Random.Range(-0.8f, 0.8f));
                GameObject obj = SRBehaviour.InstantiateActor(prefab, setId, center + jitter, rot, false);
                if (obj != null)
                {
                    spawned++;
                }
            }

            CommandHelpers.LogSuccess(string.Format("Заспавнено Ларго ({0}): {1} шт.", largoDef.IdentifiableId, spawned));
            return true;
        }

        private static bool SpawnResolvedEntry(SceneContext ctx, GameContext gctx, CatalogEntry entry, int count, float distance)
        {
            Quaternion rot;
            Vector3 basePos = CommandHelpers.GetAimSpawnPoint(ctx, distance, out rot);
            RegionRegistry.RegionSetId setId = CommandHelpers.GetCurrentRegionSetId(ctx);
            LookupDirector lookup = gctx.LookupDirector;

            if (entry.Kind == CatalogKind.Gadget)
            {
                GadgetDefinition gdef = lookup.GetGadgetDefinition(entry.GadgetId);
                if (gdef == null || gdef.prefab == null)
                {
                    CommandHelpers.LogError("У гаджета " + entry.GadgetId + " нет префаба.");
                    return true;
                }

                Vector3 pos = CommandHelpers.SnapToGround(basePos);
                int ok = 0;
                for (int i = 0; i < Mathf.Min(count, 10); i++)
                {
                    Vector3 offset = new Vector3(i * 1.4f, 0f, 0f);
                    GameObject go = UnityEngine.Object.Instantiate(gdef.prefab, pos + offset, rot);
                    if (go != null)
                    {
                        Gadget g = go.GetComponent<Gadget>();
                        if (g != null && ctx.GameModel != null)
                        {
                            string gid = "devgadget_" + Guid.NewGuid().ToString("N").Substring(0, 10);
                            IdHandler ih = go.GetComponent<IdHandler>();
                            if (ih != null) ih.id = gid;
                        }
                        ok++;
                    }
                }
                CommandHelpers.LogSuccess(string.Format("Заспавнен гаджет '{0}' ({1}): {2} шт.", entry.DisplayRu, entry.GadgetId, ok));
                return true;
            }

            if (entry.Kind == CatalogKind.ResourceSpawner)
            {
                GameObject resPrefab = lookup.GetResourcePrefab(entry.ResourceId);
                if (resPrefab == null)
                {
                    CommandHelpers.LogError("Префаб ресурса " + entry.ResourceId + " не найден.");
                    return true;
                }
                Vector3 pos = CommandHelpers.SnapToGround(basePos);
                UnityEngine.Object.Instantiate(resPrefab, pos, rot);
                CommandHelpers.LogSuccess(string.Format("Заспавнен ресурс '{0}' ({1}) на земле.", entry.DisplayRu, entry.ResourceId));
                return true;
            }

            // Actor or Gordo
            Identifiable.Id id = entry.ActorId;
            if (Identifiable.IsGordo(id))
            {
                return SpawnGordo(ctx, lookup, id, entry.DisplayRu, basePos, rot, Mathf.Min(count, 5));
            }

            GameObject prefab = null;
            try
            {
                prefab = lookup.GetPrefab(id);
            }
            catch
            {
            }

            if (prefab == null)
            {
                CommandHelpers.LogError(string.Format("Для '{0}' ({1}) не зарегистрирован Actor-префаб в LookupDirector.", entry.DisplayRu, id));
                return true;
            }

            bool hasIdentifiable = prefab.GetComponent<Identifiable>() != null;
            int spawned = 0;
            for (int i = 0; i < count; i++)
            {
                Vector3 spread = count == 1
                    ? Vector3.zero
                    : new Vector3(UnityEngine.Random.Range(-0.9f, 0.9f), (i % 4) * 0.25f, UnityEngine.Random.Range(-0.9f, 0.9f));

                GameObject actor = null;
                try
                {
                    actor = SRBehaviour.InstantiateActor(prefab, setId, basePos + spread, rot, !hasIdentifiable);
                }
                catch
                {
                    actor = UnityEngine.Object.Instantiate(prefab, basePos + spread, rot);
                }

                if (actor != null)
                {
                    spawned++;
                }
            }

            CommandHelpers.LogSuccess(string.Format("Заспавнено '{0}' [{1}]: {2} шт.", entry.DisplayRu, id, spawned));
            return true;
        }

        private static bool SpawnGordo(SceneContext ctx, LookupDirector lookup, Identifiable.Id gordoId, string display, Vector3 pos, Quaternion rot, int count)
        {
            GameObject prefab = null;
            try
            {
                prefab = lookup.GetGordo(gordoId);
            }
            catch
            {
            }

            if (prefab == null)
            {
                CommandHelpers.LogError("Не найден префаб Гордо для " + gordoId);
                return true;
            }

            Vector3 groundPos = CommandHelpers.SnapToGround(pos);
            int spawned = 0;
            for (int i = 0; i < count; i++)
            {
                Vector3 gPos = groundPos + new Vector3(i * 3.5f, 0f, 0f);
                GameObject gordoObj = UnityEngine.Object.Instantiate(prefab, gPos, rot);
                if (gordoObj == null)
                {
                    continue;
                }

                string uniqueId = "devgordo_" + Guid.NewGuid().ToString("N").Substring(0, 12);
                IdHandler idHandler = gordoObj.GetComponent<IdHandler>();
                if (idHandler != null)
                {
                    idHandler.id = uniqueId;
                }

                if (ctx.GameModel != null)
                {
                    ctx.GameModel.RegisterGordo(uniqueId, gordoObj);
                }
                spawned++;
            }

            CommandHelpers.LogSuccess(string.Format("Заспавнен Гордо '{0}' [{1}]: {2} шт. (можно кормить до взрыва!).", display, gordoId, spawned));
            return true;
        }
    }

    internal sealed class LargoCommand : DevCommandBase
    {
        public LargoCommand()
            : base("ларго",
                "ларго <слайм1> <слайм2> [кол-во]",
                "Создаёт любого Ларго по двум названиям слаймов (напр. 'ларго розовый квантовый 3').")
        {
        }

        public override List<string> AutoComplete(int argIndex, string argText)
        {
            if (argIndex == 0 || argIndex == 1)
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
            return SpawnExCommand.SpawnLargoFromArgs(ctx, gctx, args, 0);
        }
    }
}
