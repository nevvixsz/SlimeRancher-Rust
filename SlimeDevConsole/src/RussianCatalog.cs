using System;
using System.Collections.Generic;
using System.Text;

namespace SlimeDevConsole
{
    internal enum CatalogKind
    {
        ActorOrGordo,
        Gadget,
        ResourceSpawner
    }

    internal sealed class CatalogEntry
    {
        public CatalogKind Kind;
        public Identifiable.Id ActorId;
        public Gadget.Id GadgetId;
        public SpawnResource.Id ResourceId;
        public string DisplayRu;
        public string RawKey;
    }

    internal static class RussianCatalog
    {
        private static bool _initialized;
        private static readonly Dictionary<string, CatalogEntry> ExactMap =
            new Dictionary<string, CatalogEntry>(StringComparer.OrdinalIgnoreCase);
        private static readonly Dictionary<string, Identifiable.Id> BaseSlimeAliases =
            new Dictionary<string, Identifiable.Id>(StringComparer.OrdinalIgnoreCase);
        private static readonly List<string> AutoCompleteList = new List<string>(512);
        private static readonly HashSet<string> AutoCompleteSeen =
            new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        public static string Normalize(string input)
        {
            if (string.IsNullOrEmpty(input))
            {
                return string.Empty;
            }

            string s = input.Trim().ToLowerInvariant()
                .Replace('ё', 'е')
                .Replace('-', '_')
                .Replace(' ', '_');

            while (s.Contains("__"))
            {
                s = s.Replace("__", "_");
            }
            return s.Trim('_');
        }

        public static void EnsureInitialized()
        {
            if (_initialized)
            {
                return;
            }

            if (SRSingleton<GameContext>.Instance == null ||
                SRSingleton<GameContext>.Instance.LookupDirector == null)
            {
                return;
            }

            _initialized = true;
            BuildBuiltinAliases();
            BuildGameBundleAliases();
            BuildBaseSlimeAliases();
        }

        private static void BuildBaseSlimeAliases()
        {
            AddBaseSlime(Identifiable.Id.PINK_SLIME, "розовый", "pink");
            AddBaseSlime(Identifiable.Id.ROCK_SLIME, "скалистый", "каменный", "rock");
            AddBaseSlime(Identifiable.Id.PHOSPHOR_SLIME, "фосфорный", "phosphor");
            AddBaseSlime(Identifiable.Id.TABBY_SLIME, "мяукающий", "полосатый", "котик", "tabby");
            AddBaseSlime(Identifiable.Id.RAD_SLIME, "радиоактивный", "радиационный", "рад", "rad");
            AddBaseSlime(Identifiable.Id.BOOM_SLIME, "бах", "взрывной", "бум", "boom");
            AddBaseSlime(Identifiable.Id.HONEY_SLIME, "медовый", "мед", "honey");
            AddBaseSlime(Identifiable.Id.HUNTER_SLIME, "охотник", "hunter");
            AddBaseSlime(Identifiable.Id.CRYSTAL_SLIME, "кристальный", "кристалл", "crystal");
            AddBaseSlime(Identifiable.Id.QUANTUM_SLIME, "квантовый", "квант", "quantum");
            AddBaseSlime(Identifiable.Id.DERVISH_SLIME, "дервиш", "вихрь", "dervish");
            AddBaseSlime(Identifiable.Id.MOSAIC_SLIME, "мозаичный", "мозаика", "mosaic");
            AddBaseSlime(Identifiable.Id.TANGLE_SLIME, "вьюнок", "цветочный", "лиана", "tangle");
            AddBaseSlime(Identifiable.Id.SABER_SLIME, "саблезубый", "сабля", "saber");
            AddBaseSlime(Identifiable.Id.PUDDLE_SLIME, "лужа", "водяной", "puddle");
            AddBaseSlime(Identifiable.Id.FIRE_SLIME, "огненный", "огонь", "fire");
            AddBaseSlime(Identifiable.Id.GOLD_SLIME, "золотой", "голд", "gold");
            AddBaseSlime(Identifiable.Id.LUCKY_SLIME, "счастливчик", "удачливый", "lucky");
            AddBaseSlime(Identifiable.Id.QUICKSILVER_SLIME, "ртутный", "ртуть", "quicksilver");
            AddBaseSlime(Identifiable.Id.GLITCH_SLIME, "глитч", "баг", "glitch");
            AddBaseSlime(Identifiable.Id.TARR_SLIME, "тарр", "варр", "tarr");
        }

        private static void AddBaseSlime(Identifiable.Id id, params string[] names)
        {
            for (int i = 0; i < names.Length; i++)
            {
                BaseSlimeAliases[Normalize(names[i])] = id;
            }
        }

        public static bool TryResolveBaseSlime(string token, out Identifiable.Id slimeId)
        {
            EnsureInitialized();
            string norm = Normalize(token);
            if (BaseSlimeAliases.TryGetValue(norm, out slimeId))
            {
                return true;
            }

            if (TryResolveActor(token, out Identifiable.Id resolved, out _) && Identifiable.SLIME_CLASS.Contains(resolved))
            {
                slimeId = resolved;
                return true;
            }

            slimeId = Identifiable.Id.NONE;
            return false;
        }

        private static void BuildGameBundleAliases()
        {
            foreach (Identifiable.Id id in Enum.GetValues(typeof(Identifiable.Id)))
            {
                if (id == Identifiable.Id.NONE || id == Identifiable.Id.PLAYER)
                {
                    continue;
                }

                string enumName = Enum.GetName(typeof(Identifiable.Id), id);
                string locName = null;
                try
                {
                    locName = Identifiable.GetName(id, false);
                }
                catch
                {
                }

                CatalogEntry entry = new CatalogEntry
                {
                    Kind = CatalogKind.ActorOrGordo,
                    ActorId = id,
                    DisplayRu = !string.IsNullOrEmpty(locName) ? locName : enumName,
                    RawKey = enumName
                };

                RegisterKey(enumName, entry, true);

                if (!string.IsNullOrEmpty(locName) && !string.Equals(locName, enumName, StringComparison.OrdinalIgnoreCase))
                {
                    string normLoc = Normalize(locName);
                    RegisterKey(normLoc, entry, true);
                    if (normLoc.EndsWith("_слайм"))
                    {
                        RegisterKey(normLoc.Substring(0, normLoc.Length - "_слайм".Length), entry, false);
                    }
                }
            }

            foreach (Gadget.Id gid in Enum.GetValues(typeof(Gadget.Id)))
            {
                if (gid == Gadget.Id.NONE)
                {
                    continue;
                }

                string enumName = Enum.GetName(typeof(Gadget.Id), gid);
                string locName = null;
                try
                {
                    locName = Gadget.GetName(gid, false);
                }
                catch
                {
                }

                CatalogEntry entry = new CatalogEntry
                {
                    Kind = CatalogKind.Gadget,
                    GadgetId = gid,
                    DisplayRu = !string.IsNullOrEmpty(locName) ? locName : enumName,
                    RawKey = enumName
                };

                RegisterKey(enumName, entry, true);
                RegisterKey("gadget_" + enumName, entry, false);
                if (!string.IsNullOrEmpty(locName) && !string.Equals(locName, enumName, StringComparison.OrdinalIgnoreCase))
                {
                    RegisterKey(Normalize(locName), entry, true);
                }
            }

            foreach (SpawnResource.Id rid in Enum.GetValues(typeof(SpawnResource.Id)))
            {
                if (rid == SpawnResource.Id.NONE)
                {
                    continue;
                }

                string enumName = Enum.GetName(typeof(SpawnResource.Id), rid);
                CatalogEntry entry = new CatalogEntry
                {
                    Kind = CatalogKind.ResourceSpawner,
                    ResourceId = rid,
                    DisplayRu = enumName,
                    RawKey = enumName
                };
                RegisterKey(enumName, entry, true);
            }
        }

        private static void BuildBuiltinAliases()
        {
            // Slimes
            AddActor(Identifiable.Id.PINK_SLIME, "Розовый слайм", "розовый_слайм", "розовый");
            AddActor(Identifiable.Id.ROCK_SLIME, "Скалистый слайм", "скалистый_слайм", "скалистый", "каменный_слайм");
            AddActor(Identifiable.Id.PHOSPHOR_SLIME, "Фосфорный слайм", "фосфорный_слайм", "фосфорный");
            AddActor(Identifiable.Id.TABBY_SLIME, "Мяукающий слайм", "мяукающий_слайм", "мяукающий", "полосатый_слайм");
            AddActor(Identifiable.Id.RAD_SLIME, "Радиоактивный слайм", "радиоактивный_слайм", "радиоактивный", "рад_слайм");
            AddActor(Identifiable.Id.BOOM_SLIME, "Бах-слайм", "бах_слайм", "бах", "бум_слайм", "взрывной_слайм");
            AddActor(Identifiable.Id.HONEY_SLIME, "Медовый слайм", "медовый_слайм", "медовый");
            AddActor(Identifiable.Id.HUNTER_SLIME, "Слайм-охотник", "слайм_охотник", "охотник_слайм", "охотник");
            AddActor(Identifiable.Id.PUDDLE_SLIME, "Слайм-лужа", "слайм_лужа", "лужа_слайм", "лужа", "водяной_слайм");
            AddActor(Identifiable.Id.CRYSTAL_SLIME, "Кристальный слайм", "кристальный_слайм", "кристальный");
            AddActor(Identifiable.Id.QUANTUM_SLIME, "Квантовый слайм", "квантовый_слайм", "квантовый");
            AddActor(Identifiable.Id.DERVISH_SLIME, "Слайм-дервиш", "слайм_дервиш", "дервиш_слайм", "дервиш");
            AddActor(Identifiable.Id.MOSAIC_SLIME, "Мозаичный слайм", "мозаичный_слайм", "мозаичный");
            AddActor(Identifiable.Id.TANGLE_SLIME, "Слайм-вьюнок", "слайм_вьюнок", "вьюнок_слайм", "вьюнок");
            AddActor(Identifiable.Id.FIRE_SLIME, "Огненный слайм", "огненный_слайм", "огненный");
            AddActor(Identifiable.Id.SABER_SLIME, "Саблезубый слайм", "саблезубый_слайм", "саблезубый");
            AddActor(Identifiable.Id.QUICKSILVER_SLIME, "Ртутный слайм", "ртутный_слайм", "ртутный");
            AddActor(Identifiable.Id.GLITCH_SLIME, "Глитч-слайм", "глитч_слайм", "глитч");
            AddActor(Identifiable.Id.GOLD_SLIME, "Золотой слайм", "золотой_слайм", "золотой");
            AddActor(Identifiable.Id.LUCKY_SLIME, "Слайм-счастливчик", "слайм_счастливчик", "счастливчик");
            AddActor(Identifiable.Id.TARR_SLIME, "Тарр", "тарр", "варр", "смоляной_слайм");
            AddActor(Identifiable.Id.GLITCH_TARR_SLIME, "Глитч-тарр", "глитч_тарр", "глитч_варр");

            // Plorts
            AddActor(Identifiable.Id.PINK_PLORT, "Розовый плорт", "розовый_плорт");
            AddActor(Identifiable.Id.ROCK_PLORT, "Скалистый плорт", "скалистый_плорт");
            AddActor(Identifiable.Id.PHOSPHOR_PLORT, "Фосфорный плорт", "фосфорный_плорт");
            AddActor(Identifiable.Id.TABBY_PLORT, "Мяукающий плорт", "мяукающий_плорт");
            AddActor(Identifiable.Id.RAD_PLORT, "Радиоактивный плорт", "радиоактивный_плорт", "рад_плорт");
            AddActor(Identifiable.Id.BOOM_PLORT, "Бах-плорт", "бах_плорт", "бум_плорт");
            AddActor(Identifiable.Id.HONEY_PLORT, "Медовый плорт", "медовый_плорт");
            AddActor(Identifiable.Id.HUNTER_PLORT, "Плорт охотника", "плорт_охотника", "охотник_плорт");
            AddActor(Identifiable.Id.PUDDLE_PLORT, "Плорт лужи", "плорт_лужи", "лужа_плорт");
            AddActor(Identifiable.Id.CRYSTAL_PLORT, "Кристальный плорт", "кристальный_плорт");
            AddActor(Identifiable.Id.QUANTUM_PLORT, "Квантовый плорт", "квантовый_плорт");
            AddActor(Identifiable.Id.DERVISH_PLORT, "Плорт дервиша", "плорт_дервиша", "дервиш_плорт");
            AddActor(Identifiable.Id.MOSAIC_PLORT, "Мозаичный плорт", "мозаичный_плорт");
            AddActor(Identifiable.Id.TANGLE_PLORT, "Плорт вьюнка", "плорт_вьюнка", "вьюнок_плорт");
            AddActor(Identifiable.Id.FIRE_PLORT, "Огненный плорт", "огненный_плорт");
            AddActor(Identifiable.Id.SABER_PLORT, "Саблезубый плорт", "саблезубый_плорт");
            AddActor(Identifiable.Id.QUICKSILVER_PLORT, "Ртутный плорт", "ртутный_плорт");
            AddActor(Identifiable.Id.GOLD_PLORT, "Золотой плорт", "золотой_плорт", "голд_плорт");

            // Food: Fruits, Veggies, Meat, Special
            AddActor(Identifiable.Id.POGO_FRUIT, "Погофрукт", "погофрукт", "пого");
            AddActor(Identifiable.Id.CUBERRY_FRUIT, "Кубника", "кубника", "кубическая_клубника");
            AddActor(Identifiable.Id.MANGO_FRUIT, "Мята-манго", "мята_манго", "манго");
            AddActor(Identifiable.Id.PEAR_FRUIT, "Грушевник", "грушевник", "колючий_грушевник", "груша");
            AddActor(Identifiable.Id.LEMON_FRUIT, "Обычный лимон", "лимон", "обычный_лимон");
            AddActor(Identifiable.Id.LEMON_PHASE, "Фазовый лимон", "фазовый_лимон", "лимон_фазовый", "phase_lemon");
            AddActor(Identifiable.Id.KOOKADOBA_FRUIT, "Кукадумба", "кукадумба", "кокодумба");
            AddActor(Identifiable.Id.CARROT_VEGGIE, "Морковь", "морковь", "морковка");
            AddActor(Identifiable.Id.OCAOCA_VEGGIE, "Ока-ока", "ока_ока", "ока");
            AddActor(Identifiable.Id.BEET_VEGGIE, "Сердцесвекла", "сердцесвекла", "свекла");
            AddActor(Identifiable.Id.PARSNIP_VEGGIE, "Серебряный пастернак", "серебряный_пастернак", "пастернак");
            AddActor(Identifiable.Id.ONION_VEGGIE, "Чудо-лук", "чудолук", "чудо_лук", "лук");
            AddActor(Identifiable.Id.GINGER_VEGGIE, "Позолоченный имбирь", "позолоченный_имбирь", "золотой_имбирь", "имбирь", "ginger");
            AddActor(Identifiable.Id.SPICY_TOFU, "Острый тофу", "острый_тофу", "тофу", "tofu");

            AddActor(Identifiable.Id.HEN, "Курица", "курица", "курочка");
            AddActor(Identifiable.Id.ROOSTER, "Кукарекун", "кукарекун", "петух");
            AddActor(Identifiable.Id.CHICK, "Цыпленок", "цыпленок");
            AddActor(Identifiable.Id.STONY_HEN, "Каменистая курица", "каменистая_курица");
            AddActor(Identifiable.Id.STONY_CHICK, "Каменистый цыпленок", "каменистый_цыпленок");
            AddActor(Identifiable.Id.BRIAR_HEN, "Тернистая курица", "тернистая_курица");
            AddActor(Identifiable.Id.BRIAR_CHICK, "Тернистый цыпленок", "тернистый_цыпленок");
            AddActor(Identifiable.Id.PAINTED_HEN, "Расписная курица", "расписная_курица");
            AddActor(Identifiable.Id.PAINTED_CHICK, "Расписной цыпленок", "расписной_цыпленок");
            AddActor(Identifiable.Id.ELDER_HEN, "Старая курица", "старая_курица");
            AddActor(Identifiable.Id.ELDER_ROOSTER, "Старый петух", "старый_петух", "старый_кукарекун");

            // Craft resources
            AddActor(Identifiable.Id.PRIMORDY_OIL_CRAFT, "Первобытное масло", "первобытное_масло", "масло");
            AddActor(Identifiable.Id.DEEP_BRINE_CRAFT, "Глубокий рассол", "глубокий_рассол", "рассол");
            AddActor(Identifiable.Id.SPIRAL_STEAM_CRAFT, "Спиральный пар", "спиральный_пар", "пар");
            AddActor(Identifiable.Id.LAVA_DUST_CRAFT, "Лавовая пыль", "лавовая_пыль");
            AddActor(Identifiable.Id.BUZZ_WAX_CRAFT, "Жужжащий воск", "жужжащий_воск", "воск");
            AddActor(Identifiable.Id.WILD_HONEY_CRAFT, "Дикий мед", "дикий_мед");
            AddActor(Identifiable.Id.HEXACOMB_CRAFT, "Гексасоты", "гексасоты", "соты");
            AddActor(Identifiable.Id.ROYAL_JELLY_CRAFT, "Королевское желе", "королевское_желе", "желе");
            AddActor(Identifiable.Id.JELLYSTONE_CRAFT, "Желейный камень", "желейный_камень");
            AddActor(Identifiable.Id.INDIGONIUM_CRAFT, "Индигоний", "индигоний");
            AddActor(Identifiable.Id.SLIME_FOSSIL_CRAFT, "Слаймовая окаменелость", "слаймовая_окаменелость", "окаменелость");
            AddActor(Identifiable.Id.STRANGE_DIAMOND_CRAFT, "Странный алмаз", "странный_алмаз", "алмаз");
            AddActor(Identifiable.Id.SILKY_SAND_CRAFT, "Шелковистый песок", "шелковистый_песок", "песок");
            AddActor(Identifiable.Id.PEPPER_JAM_CRAFT, "Перечный джем", "перечный_джем", "джем");
            AddActor(Identifiable.Id.GLASS_SHARD_CRAFT, "Осколок стекла", "осколок_стекла");
            AddActor(Identifiable.Id.MANIFOLD_CUBE_CRAFT, "Куб множества", "куб_множества", "многогранник");

            // Special / Unspawnable-in-vanilla objects
            AddActor(Identifiable.Id.KEY, "Слаймовый ключ", "ключ", "слаймовый_ключ", "key");
            AddActor(Identifiable.Id.WATER_LIQUID, "Вода", "вода", "water");
            AddActor(Identifiable.Id.MAGIC_WATER_LIQUID, "Древняя (волшебная) вода", "древняя_вода", "волшебная_вода", "магическая_вода");
            AddActor(Identifiable.Id.FIRE_COLUMN, "Огненный столб", "огненный_столб", "столб_огня");
            AddActor(Identifiable.Id.POLLEN_CLOUD, "Облако пыльцы", "облако_пыльцы", "пыльца");
            AddActor(Identifiable.Id.GLITCH_TARR_PORTAL, "Глитч-портал", "глитч_портал", "портал_глитча");
            AddActor(Identifiable.Id.GLITCH_BUG_REPORT, "Отчет об ошибке", "отчет_об_ошибке", "баг_репорт");
            AddActor(Identifiable.Id.GLITCH_DEBUG_SPRAY_LIQUID, "Отладочный спрей", "глитч_спрей", "отладочный_спрей");
            AddActor(Identifiable.Id.PORTABLE_SCARECROW, "Портативное пугало", "пугало", "портативное_пугало");
            AddActor(Identifiable.Id.SCARECROW, "Стационарное пугало", "старое_пугало");
            AddActor(Identifiable.Id.KOOKADOBA_BALL, "Шар кукадумбы", "шар_кукадумбы");
            AddActor(Identifiable.Id.CRATE_REEF_01, "Ящик Рифа", "ящик_рифа", "ящик");
            AddActor(Identifiable.Id.CRATE_QUARRY_01, "Ящик Карьера", "ящик_карьера");
            AddActor(Identifiable.Id.CRATE_MOSS_01, "Ящик Мшистого покрова", "ящик_мхов");
            AddActor(Identifiable.Id.CRATE_DESERT_01, "Ящик Стеклянной пустыни", "ящик_пустыни");
            AddActor(Identifiable.Id.CRATE_RUINS_01, "Ящик Древних руин", "ящик_руин");
            AddActor(Identifiable.Id.CRATE_WILDS_01, "Ящик Диких дебрей", "ящик_дебрей");
            AddActor(Identifiable.Id.CRATE_PARTY_01, "Праздничный ящик", "праздничный_ящик", "тусовочный_ящик");
            AddActor(Identifiable.Id.DAILY_EXCHANGE_CRATE, "Ящик обмена", "ящик_обмена");
            AddActor(Identifiable.Id.SPECIAL_EXCHANGE_CRATE, "Особый ящик обмена", "особый_ящик_обмена");

            // Echoes
            AddActor(Identifiable.Id.RED_ECHO, "Красное эхо", "красное_эхо");
            AddActor(Identifiable.Id.GREEN_ECHO, "Зеленое эхо", "зеленое_эхо");
            AddActor(Identifiable.Id.BLUE_ECHO, "Синее эхо", "синее_эхо");
            AddActor(Identifiable.Id.GOLD_ECHO, "Золотое эхо", "золотое_эхо");
            AddActor(Identifiable.Id.ECHO_NOTE_01, "Эхо-нота 1", "эхо_нота_1");
            AddActor(Identifiable.Id.ECHO_NOTE_02, "Эхо-нота 2", "эхо_нота_2");
            AddActor(Identifiable.Id.ECHO_NOTE_03, "Эхо-нота 3", "эхо_нота_3");
            AddActor(Identifiable.Id.ECHO_NOTE_04, "Эхо-нота 4", "эхо_нота_4");
            AddActor(Identifiable.Id.ECHO_NOTE_05, "Эхо-нота 5", "эхо_нота_5");
            AddActor(Identifiable.Id.ECHO_NOTE_06, "Эхо-нота 6", "эхо_нота_6");
            AddActor(Identifiable.Id.ECHO_NOTE_07, "Эхо-нота 7", "эхо_нота_7");
            AddActor(Identifiable.Id.ECHO_NOTE_08, "Эхо-нота 8", "эхо_нота_8");
            AddActor(Identifiable.Id.ECHO_NOTE_09, "Эхо-нота 9", "эхо_нота_9");
            AddActor(Identifiable.Id.ECHO_NOTE_10, "Эхо-нота 10", "эхо_нота_10");
            AddActor(Identifiable.Id.ECHO_NOTE_11, "Эхо-нота 11", "эхо_нота_11");
            AddActor(Identifiable.Id.ECHO_NOTE_12, "Эхо-нота 12", "эхо_нота_12");
            AddActor(Identifiable.Id.ECHO_NOTE_13, "Эхо-нота 13", "эхо_нота_13");

            // All 16 Gordos
            AddActor(Identifiable.Id.PINK_GORDO, "Розовый Гордо", "розовый_гордо");
            AddActor(Identifiable.Id.ROCK_GORDO, "Скалистый Гордо", "скалистый_гордо");
            AddActor(Identifiable.Id.PHOSPHOR_GORDO, "Фосфорный Гордо", "фосфорный_гордо");
            AddActor(Identifiable.Id.TABBY_GORDO, "Мяукающий Гордо", "мяукающий_гордо");
            AddActor(Identifiable.Id.RAD_GORDO, "Радиоактивный Гордо", "радиоактивный_гордо");
            AddActor(Identifiable.Id.BOOM_GORDO, "Бах-Гордо", "бах_гордо");
            AddActor(Identifiable.Id.HONEY_GORDO, "Медовый Гордо", "медовый_гордо");
            AddActor(Identifiable.Id.HUNTER_GORDO, "Гордо-охотник", "охотник_гордо", "гордо_охотник");
            AddActor(Identifiable.Id.PUDDLE_GORDO, "Гордо-лужа", "лужа_гордо", "гордо_лужа");
            AddActor(Identifiable.Id.CRYSTAL_GORDO, "Кристальный Гордо", "кристальный_гордо");
            AddActor(Identifiable.Id.QUANTUM_GORDO, "Квантовый Гордо", "квантовый_гордо");
            AddActor(Identifiable.Id.DERVISH_GORDO, "Гордо-дервиш", "дервиш_гордо", "гордо_дервиш");
            AddActor(Identifiable.Id.MOSAIC_GORDO, "Мозаичный Гордо", "мозаичный_гордо");
            AddActor(Identifiable.Id.TANGLE_GORDO, "Гордо-вьюнок", "вьюнок_гордо", "гордо_вьюнок");
            AddActor(Identifiable.Id.GOLD_GORDO, "Золотой Гордо", "золотой_гордо", "голд_гордо");
            AddActor(Identifiable.Id.PARTY_GORDO, "Тусовочный Гордо", "тусовочный_гордо", "праздничный_гордо");

            // Gadgets
            AddGadget(Gadget.Id.DRONE, "Дрон", "дрон");
            AddGadget(Gadget.Id.DRONE_ADVANCED, "Улучшенный дрон", "улучшенный_дрон", "продвинутый_дрон");
            AddGadget(Gadget.Id.EXTRACTOR_DRILL_TITAN, "Титановый бур", "титановый_бур", "бур");
            AddGadget(Gadget.Id.EXTRACTOR_PUMP_ABYSSAL, "Глубинный насос", "глубинный_насос", "насос");
            AddGadget(Gadget.Id.EXTRACTOR_APIARY_ROYAL, "Королевская пасека", "королевская_пасека", "пасека");
            AddGadget(Gadget.Id.MARKET_LINK, "Связь с рынком", "связь_с_рынком", "рынок_гаджет");
            AddGadget(Gadget.Id.REFINERY_LINK, "Связь с переработкой", "связь_с_переработкой", "переработка_гаджет");
            AddGadget(Gadget.Id.HYDRO_TURRET, "Гидротурель", "гидротурель", "турель");
            AddGadget(Gadget.Id.SUPER_HYDRO_TURRET, "Супер-гидротурель", "супер_гидротурель");
            AddGadget(Gadget.Id.MED_STATION, "Медстанция", "медстанция");
            AddGadget(Gadget.Id.SPRING_PAD, "Трамплин", "трамплин");
            AddGadget(Gadget.Id.GORDO_SNARE_MASTER, "Мастер-ловушка Гордо", "ловушка_гордо", "мастер_ловушка");
            AddGadget(Gadget.Id.TELEPORTER_GOLD, "Золотой телепорт", "золотой_телепорт", "телепорт");
            AddGadget(Gadget.Id.ECHO_NET, "Сеть для эхо", "сеть_эхо");

            // Resource spawners (trees & patches)
            AddResource(SpawnResource.Id.LEMON_TREE, "Дерево фазового лимона", "дерево_фазового_лимона", "дерево_лимона");
            AddResource(SpawnResource.Id.LEMON_TREE_DLX, "Улучш. дерево фазового лимона", "дерево_фазового_лимона_dlx");
            AddResource(SpawnResource.Id.GINGER_PATCH, "Грядка позолоченного имбиря", "грядка_имбиря", "грядка_золотого_имбиря");
            AddResource(SpawnResource.Id.CUBERRY_TREE, "Дерево кубники", "дерево_кубники");
            AddResource(SpawnResource.Id.MANGO_TREE, "Дерево мята-манго", "дерево_манго", "дерево_мята_манго");
            AddResource(SpawnResource.Id.PEAR_TREE, "Дерево грушевника", "дерево_грушевника");
            AddResource(SpawnResource.Id.POGO_TREE, "Дерево погофрукта", "дерево_погофрукта");
            AddResource(SpawnResource.Id.CARROT_PATCH, "Грядка моркови", "грядка_моркови");
            AddResource(SpawnResource.Id.OCAOCA_PATCH, "Грядка ока-ока", "грядка_ока_ока");
            AddResource(SpawnResource.Id.PARSNIP_PATCH, "Грядка пастернака", "грядка_пастернака");
            AddResource(SpawnResource.Id.BEET_PATCH, "Грядка сердцесвеклы", "грядка_свеклы");
            AddResource(SpawnResource.Id.ONION_PATCH, "Грядка чудо-лука", "грядка_лука");
        }

        private static void AddActor(Identifiable.Id id, string displayRu, params string[] aliases)
        {
            CatalogEntry entry = new CatalogEntry
            {
                Kind = CatalogKind.ActorOrGordo,
                ActorId = id,
                DisplayRu = displayRu,
                RawKey = Enum.GetName(typeof(Identifiable.Id), id)
            };
            for (int i = 0; i < aliases.Length; i++)
            {
                RegisterKey(aliases[i], entry, true);
            }
        }

        private static void AddGadget(Gadget.Id gid, string displayRu, params string[] aliases)
        {
            CatalogEntry entry = new CatalogEntry
            {
                Kind = CatalogKind.Gadget,
                GadgetId = gid,
                DisplayRu = displayRu,
                RawKey = Enum.GetName(typeof(Gadget.Id), gid)
            };
            for (int i = 0; i < aliases.Length; i++)
            {
                RegisterKey(aliases[i], entry, true);
            }
        }

        private static void AddResource(SpawnResource.Id rid, string displayRu, params string[] aliases)
        {
            CatalogEntry entry = new CatalogEntry
            {
                Kind = CatalogKind.ResourceSpawner,
                ResourceId = rid,
                DisplayRu = displayRu,
                RawKey = Enum.GetName(typeof(SpawnResource.Id), rid)
            };
            for (int i = 0; i < aliases.Length; i++)
            {
                RegisterKey(aliases[i], entry, true);
            }
        }

        private static void RegisterKey(string rawKey, CatalogEntry entry, bool addToAutoComplete)
        {
            string norm = Normalize(rawKey);
            if (string.IsNullOrEmpty(norm))
            {
                return;
            }

            if (!ExactMap.ContainsKey(norm))
            {
                ExactMap[norm] = entry;
            }

            if (addToAutoComplete && AutoCompleteSeen.Add(norm))
            {
                AutoCompleteList.Add(norm);
            }
        }

        public static List<string> GetAutoCompleteList()
        {
            EnsureInitialized();
            return AutoCompleteList;
        }

        public static bool TryResolveActor(string query, out Identifiable.Id id, out string display)
        {
            if (TryResolveAny(query, out CatalogEntry entry, out _) && entry.Kind == CatalogKind.ActorOrGordo)
            {
                id = entry.ActorId;
                display = entry.DisplayRu;
                return true;
            }

            id = Identifiable.Id.NONE;
            display = null;
            return false;
        }

        public static bool TryResolveAny(string query, out CatalogEntry result, out List<string> suggestions)
        {
            EnsureInitialized();
            suggestions = new List<string>();
            string norm = Normalize(query);
            if (string.IsNullOrEmpty(norm))
            {
                result = null;
                return false;
            }

            if (ExactMap.TryGetValue(norm, out result))
            {
                return true;
            }

            // Partial / substring search
            List<CatalogEntry> matches = new List<CatalogEntry>();
            HashSet<string> seenDisplay = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            foreach (KeyValuePair<string, CatalogEntry> kv in ExactMap)
            {
                if (kv.Key.Contains(norm) || Normalize(kv.Value.DisplayRu).Contains(norm))
                {
                    string sig = kv.Value.Kind + ":" + kv.Value.RawKey;
                    if (seenDisplay.Add(sig))
                    {
                        matches.Add(kv.Value);
                        suggestions.Add(kv.Key + " (" + kv.Value.DisplayRu + ")");
                        if (matches.Count >= 10)
                        {
                            break;
                        }
                    }
                }
            }

            if (matches.Count == 1)
            {
                result = matches[0];
                return true;
            }

            result = null;
            return false;
        }

        public static List<string> Search(string filter, int limit = 25)
        {
            EnsureInitialized();
            string norm = Normalize(filter);
            List<string> results = new List<string>();
            HashSet<string> seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

            foreach (KeyValuePair<string, CatalogEntry> kv in ExactMap)
            {
                if (string.IsNullOrEmpty(norm) ||
                    kv.Key.Contains(norm) ||
                    Normalize(kv.Value.DisplayRu).Contains(norm) ||
                    kv.Value.RawKey.ToLowerInvariant().Contains(norm))
                {
                    string sig = kv.Value.Kind + ":" + kv.Value.RawKey;
                    if (seen.Add(sig))
                    {
                        results.Add(string.Format("[{0}] {1}  ->  {2}", kv.Value.Kind, kv.Value.DisplayRu, kv.Value.RawKey));
                        if (results.Count >= limit)
                        {
                            break;
                        }
                    }
                }
            }

            return results;
        }
    }
}
