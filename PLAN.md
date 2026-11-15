# План переноса Slime Rancher (v1.4.x) на Rust

Документ составлен по результатам полного анализа декомпилированного `Assembly-CSharp` (1 831 `.cs` файл).
Главная цель — **100% идентичность логики, баланса, сюжета и геймплея оригинала** при переходе на чистую, высокопроизводительную и экономичную по памяти архитектуру на Rust.

---

## 1. Результаты анализа оригинала (`Assembly-CSharp`)

### 1.1. Состав исходников
Из 1 831 `.cs` файла:
- **Игровая логика (`Assembly-CSharp/*.cs`)** — 1 192 файла (директоры, контроллер игрока, вакпак, ИИ слаймов, ранчо, гаджеты, дроны, экономика, сюжет, UI).
- **Чистые модели состояния (`MonomiPark/SlimeRancher/DataModel`)** — 56 файлов (`GameModel`, `PlayerModel`, `WorldModel`, `RanchModel`, `ActorModel`, `SlimeModel`, `LandPlotModel`, `DroneModel`, `ProgressModel`, `MailModel` и др.).
- **Бинарная сериализация (`MonomiPark/SlimeRancher/Persist`)** — 150 файлов (версионированный бинарный формат вплоть до `GameV12`, `WorldV22`, `PlayerV14`, `RanchV07`, `ActorDataV09`, `PlacedGadgetV08`, `DroneV05`).
- **Стриминг регионов (`MonomiPark/SlimeRancher/Regions`)** — 5 файлов (`Region`, `RegionRegistry`, `RegionMember`, `RegionLoader`, `RegionInitializer`).
- **Сторонние/платформенные SDK (НЕ переносятся 1-в-1):** `rail` (WeGame SDK, 354 файла), `Sentry` (16), `Microsoft/Xbox` (5), `RichPresence` (5), `DG/Tweening` (8), `Gif` (3), `SRML`/платформенные обёртки.

### 1.2. Ключевое архитектурное преимущество оригинала
Разработчики Monomi Park уже разделили игру на:
1. **Модель (`DataModel`)** — чистое состояние мира без привязки к рендеру и `MonoBehaviour`.
2. **Участников (`*.Participant` / `Directors`)** — логику, которая синхронизирует поведение с моделью.

Это позволяет перенести игру на Rust в виде **чистого headless-ядра симуляции (`sr_sim`)**, которое может работать даже без графики (идеально для интеграционных тестов 1-в-1 с оригиналом), а поверх него подключить клиент на **Bevy + Rapier3D**.

### 1.3. Что есть в C#, а что предстоит вытащить из Unity-ассетов
- **В C#-коде уже есть:** все enum'ы (`325` `Identifiable.Id`, `15` `Zone`, `136` `Gadget.Id`, `18` `LandPlot.Upgrade`, `23` `PlayerState.Upgrade`, `37` `ProgressType`, `126` `PediaDirector.Id`), вся математика эмоций, питания, экономики (Perlin noise + market saturation), логика вакпака, конечные автоматы слаймов и дронов, триггеры сюжета Кейси (`CASEY_1..11`) и Хобсона (`HOBSON_1`), формат сохранений `GameV12`.
- **В Unity-ассетах (`resources.assets`, `sharedassets*.assets`, сцены) лежат:**
  - Числовые таблицы `ScriptableObject` (`SlimeDefinition`, `SlimeDiet`, `VacItemDefinition`, `GadgetDefinition`, `GameModeSettings`, `EconomyDirector.baseValueMap`, `CorporatePartnerUI.ranks`, `ExchangeDirector` таблицы наград);
  - Расстановка мира: границы `Region`, точки `DirectedSlimeSpawner` / `DirectedResourceSpawner`, `LandPlotLocation`, `GadgetSite`, `JournalEntry` (дроны Хобсона), `SlimeGate` / `PuzzleSlot`, `TreasurePod`, `GordoEat` префабы;
  - Тексты локализации (Starmail Кейси, записи Хобсона, Slimepedia, UI, диалоги ранчеров — в том числе официальный русский перевод `MessageDirector.Lang.RU`);
  - 3D-меши, текстуры, шейдеры, аудио-банки (SECTR).

---

## 2. Целевая архитектура Rust Workspace

```text
slime-rancher-rs/
├── Cargo.toml                  # Workspace
├── crates/
│   ├── sr_math/                # Детерминированное время, Vec3/Quat, Perlin noise (1-в-1 с Noise.cs), RNG
│   ├── sr_ids/                 # Все Enum-ы и ID (IdentifiableId, ZoneId, PlotId, GadgetId, ProgressId, PediaId...)
│   ├── sr_data/                # Структуры конфигов (SlimeDefinition, Diet, EconomyBase, GameModeSettings) + загрузка RON/JSON
│   ├── sr_model/               # Прямой порт MonomiPark.SlimeRancher.DataModel (GameModel, WorldModel, PlayerModel, ActorModel...)
│   ├── sr_persist/             # Парсер и сериализатор оригинального бинарного формата GameV12 (.sav)
│   ├── sr_world/               # RegionRegistry, Region (Proxy/Hibernate), ZoneDirector, Spawners, FastForward
│   ├── sr_ai/                  # SlimeEmotions, SlimeSubbehaviourPlexer, SlimeEat, Gordo, Tarr, уникальные абилки слаймов
│   ├── sr_gameplay/            # PlayerState, Vacpack/Ammo, LandPlot + 18 апгрейдов, Gadget + Drone, Economy, Exchange
│   ├── sr_story/               # ProgressDirector, MailDirector, PediaDirector, Journal, 7Zee, EndGame, Achievements
│   ├── sr_dlcs/                # Ogden (Wilds/Kookadoba), Mochi (Valley/Quicksilver), Viktor (Slimulation/Glitch), Holiday/Appearances
│   ├── sr_sim/                 # Единый headless игровой цикл (FixedUpdate + Update + EventBus + Rapier3D коллизии)
│   ├── sr_client/              # Bevy-клиент: камера/управление, рендер мира и слаймов, HUD/UI, аудио, локализация
│   └── sr_tools/               # CLI-утилиты: инспектор/валидатор .sav файлов, конвертер выгруженных Unity-конфигов
└── assets/                     # Выгруженные данные (config/, loc/, world/, models/, audio/)
```

**Ключевые технические принципы:**
- **Оптимизация памяти и отзывчивости:** хранение акторов в плотных ECS/arena структурах, минимум аллокаций в тике (`FixedUpdate`), батчинг рейкастов (как `RaycastBatcher` в оригинале), ленивая симуляция спящих регионов (`Hibernated` + `FastForward` по дельте `world_time`).
- **Бескомпромиссная проверка идентичности:** на каждом шаге пишутся unit/integration-тесты (включая побитовые тесты парсера `.sav` на реальных сохранениях оригинала).
- **Правило коммитов:** каждый этап (шаг) состоит из **3–6 атомарных коммитов**, каждый коммит компилируется и проходит тесты своего слоя.

---

## 3. Пошаговый план разработки

### Шаг 0. Каркас Workspace, идентификаторы и математика
*Цель: заложить фундамент типов, шума/RNG и форматов данных без единой внешней графической зависимости.*

- **Коммит 0.1 — Инициализация Cargo Workspace и `sr_math`**
  - Настроить `Cargo.toml` workspace, профили сборки, общие зависимости (`glam`, `serde`, `thiserror`, `bitflags`).
  - Портировать `Noise` (Perlin noise из `Assembly-CSharp/Noise/` — критично для 100% совпадения цен плорт-маркета!) и `Randoms`.
- **Коммит 0.2 — `sr_ids`: точный перенос всех игровых Enum'ов**
  - Портировать `Identifiable.Id` (325 значений + классификаторы `SLIME_CLASS`, `LARGO_CLASS`, `PLORT_CLASS`, `FOOD_CLASS`, `MEAT_CLASS`, `VEGGIE_CLASS`, `FRUIT_CLASS`, `CRAFT_CLASS`, `TOY_CLASS`, `ECHO_CLASS`, `ORNAMENT_CLASS` и т.д.).
  - Портировать `ZoneDirector.Zone` (15 зон) и маппинг `RegionSetId` (`HOME`, `DESERT`, `VALLEY`, `SLIMULATIONS`, `VIKTOR_LAB`).
  - Портировать `LandPlot.Id` (8), `LandPlot.Upgrade` (18), `PlayerState.Upgrade` (23), `PlayerState.GameMode` (4), `Gadget.Id` (136), `ProgressDirector.ProgressType` (37), `ProgressTrackerId` (13), `PediaDirector.Id` (126).
- **Коммит 0.3 — `sr_data`: схемы конфигураций и дефолтный реестр**
  - Описать структуры `SlimeDefinition`, `SlimeDiet`, `EatMapEntry`, `VacItemDefinition`, `GadgetDefinition`, `GameModeSettings`, `BaseValueEntry`.
  - Поддержать загрузку из `RON`/`JSON` с базовым встроенным пресетом для юнит-тестов.

---

### Шаг 1. `DataModel`, игровое время и режимы игры
*Цель: полностью перенести слой `MonomiPark.SlimeRancher.DataModel` и `TimeDirector`.*

- **Коммит 1.1 — Базовые модели акторов и мира (`sr_model`)**
  - Портировать `ActorModel`, `BasicActorModel`, `SlimeModel` (эмоции, `isFeral`, `isGlitch`), `AnimalModel` (`transformTime`, `frenzied`), `PlortModel` (`destroyTime`), `ProduceModel` (`cycleData`), `ResourceModel` (`remainingWater`), `ScienceMatModel`, `EchoModel`.
- **Коммит 1.2 — Модели игрока, ранчо, гаджетов и прогрессии (`sr_model`)**
  - Портировать `PlayerModel`, `AmmoModel` (слоты, counts, `EmotionData` для каждого захваченного слайма!), `RanchModel`, `LandPlotModel`, `GadgetsModel`, `GadgetModel`, `DroneModel`, `ExtractorModel`, `WarpDepotModel`, `SnareModel`, `ProgressModel`, `MailModel`, `PediaModel`, `TutorialsModel`.
- **Коммит 1.3 — `GameModel` и реестр участников (`sr_model`)**
  - Портировать `GameModel` (контейнеры `actors`, `landPlots`, `doors`, `gordos`, `pods`, `gadgets`, `switches`, `puzzleSlots`, `oases`, `liquidSources`) + генерацию уникальных `ActorId`.
- **Коммит 1.4 — `TimeDirector` и `GameModeConfig`**
  - Точная формула времени (`secsPerGameDay`, `ffSecsPerGameDay`, стартовое время `32400.0` = 09:00 первого дня, `WorldTime()`, `CurrDay()`, `CurrHour()`, `HasReached`, `OnPassedHour`, `FastForwardTo`).
  - Настройки всех 4 режимов: `CLASSIC` (Adventure), `CASUAL` (без тарров и диких слаймов), `TIME_LIMIT` (5-Day Rush), `TIME_LIMIT_V2` (Rush Mode с множителями за разные плорты).

---

### Шаг 2. Игрок, физика перемещения и Вакпак
*Цель: воспроизвести управление персонажем, систему статов и полный цикл всасывания/выстрела/удерживания объектов.*

- **Коммит 2.1 — Физика мира и контроллер игрока (`sr_sim` + Rapier3D)**
  - Интеграция `rapier3d`: коллизионные слои, гравитация, трение, бросок объектов, батчинг рейкастов (`RaycastBatcher`).
  - Портировать логику `vp_FPController`: бег, спринт, прыжок, реактивный ранец (`JETPACK`, `JETPACK_EFFICIENCY`), падение в море/урон (`PlayerDeathHandler`, потеря инвентаря, пропуск времени до утра следующего дня).
- **Коммит 2.2 — `PlayerState`: здоровье, энергия, радиация и 23 апгрейда**
  - Точная логика восстановления энергии (задержка `300.0` игровых секунд, скорость регена), HP, накопления/спада рад-уровня (`PlayerRadAbsorber`), ключей (`keys`), `currCurrency`, `endGameTime`.
  - Эффекты всех апгрейдов (`HEALTH_1..4`, `ENERGY_1..3`, `AMMO_1..4` — вместимость от 20 до 100, `LIQUID_SLOT`, `AIR_BURST`, `RUN_EFFICIENCY_1..2`, `GOLDEN_SURESHOT`, `TREASURE_CRACKER_1..4`).
- **Коммит 2.3 — `Ammo`: слоты инвентаря и переключение режимов**
  - 5 слотов `DEFAULT` (4 обычных + 1 водяной при `LIQUID_SLOT`) и 3 слота `NIMBLE_VALLEY`.
  - Сохранение эмоций слаймов внутри слотов вакпака (`SlimeEmotionData`).
- **Коммит 2.4 — `WeaponVacuum`: захват, удержание и выстрел**
  - Конус притягивания (`VacConeDetector` / `VacAccelerationHelper`), удержание крупных объектов (ларго, ящики, игрушки) на конце дула (`held`), выстрел (`Expel`), ударная волна (`AirBurst` — расход энергии, импульс отбрасывания, снятие дикости/атаки).
  - Взаимодействие с `LiquidSource` (вода, древняя вода, вода слимуляции), `SiloActivator`, `ExchangeAcceptor`, `Market`.
- **Коммит 2.5 — Система здоровья/смерти акторов, воды и ящиков (`Crate`, `DestroyOutsideBounds`)**
  - Логика разбивания ящиков (`BreakOnImpact`), разрушения плортов по таймеру, воды как средства против тарров и огня.
- **Коммит 2.6 — Пробная карта (`sr_sandbox`) и Dev-консоль (`Shift + Ctrl`)**
  - Минимальная 3D-арена (плоская карта с полом, стенами, тестовым загоном, грядкой, плорт-маркетом и источником воды) на базе Bevy/Rapier для раннего ручного тестирования физики и вакпака.
  - Встроенная консоль разработчика по хоткею **`Shift + Ctrl`** (и headless CLI-режим):
    - `spawn <identifiable_id> [count]` — заспавнить любой предмет/слайма/еду/плорт перед игроком;
    - `give <identifiable_id> [count]` — выдать прямо в слот вакпака;
    - `money <amount>` / `upgrade <upgrade_id>` / `time <hour|day>` / `tp <x y z>` / `clear` / `stats`.

> **Когда можно проводить тесты:**
> - **Автотесты (`cargo test`)** — уже с **Шага 0** (математика, Perlin noise, ID, диеты) и **Шага 1** (время, модели, режимы).
> - **Текстовая/CLI-песочница и спавн** — после **Шага 2**.
> - **Полноценная интерактивная пробная 3D-карта со слаймами, вакпаком и консолью `Shift + Ctrl`** — сразу после **Шагов 2–3** (не дожидаясь Шага 11!).

---

### Шаг 3. Слаймы: эмоции, ИИ (`SlimeSubbehaviourPlexer`), питание, ларго, тарры и гордо
*Цель: перенести «сердце» игры — поведение всех слаймов 1-в-1.*

- **Коммит 3.1 — `SlimeEmotions` и драйверы мотивации (`DriveCalculator`)**
  - 3 оси эмоций: `HUNGER`, `AGITATION`, `FEAR` (изменение за игровой час, влияние музыкальной шкатулки `musicBoxes`, любимых игрушек `nearbyFavoriteToyCount`, пыльцы `pollenSourceCount`, голода >= `0.99`).
  - Вычисление `Drive` для выбора еды, страха, агрессии.
- **Коммит 3.2 — `SlimeSubbehaviourPlexer` и базовые саб-поведения**
  - Портировать приоритетный плексер с интервалом переосмысления (`rethink = 1.0s`, проверка земли каждые `0.25s`, задержка активации `3.0s`, блокировка при захвате вакпаком).
  - Базовые саб-поведения: `SlimeRandomMove`, `EscapeStuck`, `FindConsumable` / `GotoConsumable`, `FleeThreats` / `SlimeFlee`, `GotoPlayer`, `PlayWithToys`, `AweTowardsAttractors` / `AweTowardsLargos`.
- **Коммит 3.3 — `SlimeEat`, `Chomper`, диеты, плорты, ларго и тарры**
  - `SlimeEat`: `FoodGroup` (`FRUIT`, `VEGGIES`, `MEAT`, `PLORTS`, `GINGER`), x2 плорта при любимой еде, секретный шанс/множитель.
  - Превращение в **Ларго** при поедании чужого плорта, превращение в **Тарра** (`TARR_SLIME`) при поедании третьего типа плорта (если режим позволяет врагов).
  - Механика **Тарров**: `TarrBite`, размножение при поедании слаймов, `TarrSterilizeOnWater`, сгорание на солнце днём.
  - Механика **Диких слаймов** (`SlimeFeral`, `FeralSlimeButtstomp`) и **Куриц** (`ChickenRandomMove`, `ChickenVampirism`, размножение, старение до Elder).
- **Коммит 3.4 — Уникальные способности слаймов (пакет 1: классические и опасные)**
  - `RockSlimeRoll` (перекат с шипами), `BoomSlimeExplode` (зарядка и взрыв с таймером), `RadSlimeExpand` (радиусное поле радиации), `CrystalSlimeLaunch` (спавн кристаллов), `SlimeHover` / `Phosphor` (светобоязнь и исчезновение днём без Solar Shield).
- **Коммит 3.5 — Уникальные способности слаймов (пакет 2: редкие, руинные и пустынные)**
  - `Hunter` (`SlimeStealth`, дикость при превращении в ларго), `QuantumSlimeSuperposition` (призрачные клоны и телепортация при высоком голоде/волнении), `DervishSlimeSpin` (торнадо `SlimenadoMoveSpiral`), `MosaicAttractor` (взрывные блики), `TentacleGrapple` (Tangle лозы и пыльца), `PuddleSlimeScoot` (выживание только в воде, стеснительность при >4 соседях), `FireSlimeIgnition` (жизнь в пепле), `GoldSlimeFlee`/`GoldSlimeProducePlorts`, `LuckySlimeFlee`/`LuckySlimeProduceCoins`.
- **Коммит 3.6 — `GordoEat`, `GordoRewards`, `SlimeKey` и `SlimeGate`**
  - Кормление Гордо (Pink, Tabby, Phosphor, Rock, Rad, Honey, Boom, Hunter, Crystal, Quantum, Dervish, Mosaic, Tangle, Gold, Party).
  - Взрыв Гордо при достижении порога, выдача наград (ключи, активация телепортов, редкие ящики).

---

### Шаг 4. Ранчо, участки (`LandPlot`) и все 18 апгрейдов
*Цель: перенести всю механику строительства и автоматизации ранчо.*

- **Коммит 4.1 — `RanchDirector`, `LandPlot` и `EmptyPlot`**
  - Покупка участков, снос постройки (`Destroy`), покупка и применение апгрейдов, расширения ранчо (`Overgrowth`, `Grotto`, `Docks`, `Lab` + локации Огдена, Мочи, Виктора).
- **Коммит 4.2 — `Corral` (Загон) и `Coop` (Курятник)**
  - Апгрейды загона: `WALLS`, `MUSIC_BOX`, `AIR_NET` (прочность, восстановление), `SOLAR_SHIELD`, `FEEDER` (скорости кормушки 1/2/3), `PLORT_COLLECTOR` (сбор и хранение до 100 плорта).
  - Апгрейды курятника: `WALLS`, `FEEDER`, `VITAMIZER` (ускорение роста и размножения), `DELUXE_COOP`.
- **Коммит 4.3 — `Garden` (Огород), `Pond` (Пруд) и `Incinerator` (Печь)**
  - Рост культур (`SpawnResource`), цикл созревания, опадание и порча, апгрейды `SOIL`, `SPRINKLER`, `SCARESLIME`, `MIRACLE_MIX`, `DELUXE_GARDEN`.
  - `Pond` (источник воды + место для Puddle-слаймов), `Incinerator` (сжигание предметов + `ASH_TROUGH` для Fire-слаймов).
- **Коммит 4.4 — `Silo` (Силос) и `LinkedSiloStorage`**
  - Апгрейды `STORAGE2..4` (от 1 до 4 сторон/слотов, 300 единиц в слоте), прием/выдача через вакпак.
- **Коммит 4.5 — `RanchCellFastForwarder` и `ToyDirector`**
  - Догоняющая симуляция (catch-up) при сне до утра или возвращении на ранчо после долгого отсутствия (рост овощей, расход корма, голод/волнение слаймов, сбор плорта).
  - Полная логика игрушек (`TOY_CLASS`) и их влияния на любимые типы слаймов.

---

### Шаг 5. Мир, регионы, спавнеры и окружение
*Цель: реализовать стриминг зон без просадок FPS и воспроизвести все мировые механики.*

- **Коммит 5.1 — `RegionRegistry`, `Region` и `RegionMember`**
  - Управление активными `RegionSetId` (`HOME`, `DESERT`, `VALLEY`, `SLIMULATIONS`, `VIKTOR_LAB`).
  - Счётчики `nonProxyRefCount` и `nonHibernateRefCount`: перевод далёких регионов в `Hibernated`, мгновенное пробуждение и `UpdateToTime` при приближении игрока.
- **Коммит 5.2 — `ZoneDirector` и `CellDirector` + спавнеры**
  - Портировать `DirectedSlimeSpawner`, `DirectedAnimalSpawner`, `DirectedAuxItemSpawner` (куры, фрукты/овощи, в т.ч. ночной спавн `GingerPatchNode` в Glass Desert), `DirectedCrateSpawner`.
  - Ежедневный ресет ящиков/предметов в 05:00 утра.
- **Коммит 5.3 — Двери, телепорты, замки и сокровищницы**
  - `AccessDoor` (расширения ранчо и двери в новые зоны), `SlimeGateActivator` (открытие за `SLIME_KEY`), `PuzzleSlot` / `PuzzleDoorLock` (древние врата с плортами в Ruins и Glass Desert), `TreasurePod` (зелёные/синие/фиолетовые капсулы с проверкой `TREASURE_CRACKER_1..4`).
- **Коммит 5.4 — Стеклянная пустыня: `FirestormController` и `Oasis`**
  - Цикл солнечных бурь (`IDLE -> PREPARING -> ACTIVE`, огненные колонны, урон игроку и спавн Fire/Mosaic слаймов).
  - Логика оживления оазисов (`Oasis`, `OasisWaterTrigger`) древней водой (`ANCIENT_WATER_LIQUID`).

---

### Шаг 6. Экономика, Плорт-Маркет, 7Zee и Обменник Ранчеров
*Цель: 1-в-1 перенести финансовую и торговую петлю игры.*

- **Коммит 6.1 — `EconomyDirector` (Плорт-Маркет)**
  - Точная формула цены: `baseValue * saturationMultiplier * marketNoise * itemNoise` (на базе `econSeed` и `Noise::PerlinNoise`).
  - Насыщение рынка (`RegisterSold`), ежедневное восстановление насыщения (`saturationRecovery = 0.25`) в полночь (`00:00`), ежедневное окно техобслуживания (`dailyShutdownMins = 5`).
- **Коммит 6.2 — `PlortMarket` UI-логика и `MarketLink`**
  - Продажа плорта в терминал ранчо и через гаджет `MARKET_LINK`, отслеживание дельты цены относительно вчерашнего дня.
- **Коммит 6.3 — 7Zee Rewards Club (`CorporatePartnerUI`)**
  - Все 28 рангов клуба, списание ньюбаксов, разблокировка апгрейдов ранчо (`RanchHouseUI`), золотых шот-улучшений и декоративных тем ранчо.
- **Коммит 6.4 — `ExchangeDirector` (Range Exchange)**
  - Ежедневная генерация квестов в 12:05 (Thora, Viktor, Ogden, Mochi, BOb), таймер истечения, проверка сдачи предметов в `ExchangeAcceptor`, выдача наград через `ExchangeEjector` и разблокировка доступа к новым зонам.

---

### Шаг 7. Слимонаука (Slime Science), гаджеты и Дроны
*Цель: перенести производство ресурсов, чертежи, телепорты и программируемых дронов.*

- **Коммит 7.1 — `GadgetDirector` (Очиститель, Фабрикатор и Чертежи)**
  - `Refinery`: хранение плортов и `CRAFT_CLASS` ресурсов (до 999), безвозвратное внесение.
  - Покупка чертежей в магазине (`BuyBlueprint`), крафт гаджетов, размещение на `GadgetSite` и демонтаж (`DestroysOnRemoval` / `DestroysLinkedPairOnRemoval`).
- **Коммит 7.2 — Экстракторы (`Extractor`: Drill, Pump, Apiary)**
  - Новичок / Продвинутый / Мастер / Титан-уровни, циклы добычи, привязка к зоне (`ZoneDirector.Zone` определяет таблицу дропа — например, Lava Dust только в Glass Desert, Deep Brine только в Reef, Wild Honey только в Moss Blanket).
- **Коммит 7.3 — Телепорты, Warp Depots, Силки Гордо, Эхо-сети и Утилиты**
  - `Teleporter` (парное связывание по цвету), `WarpDepot` (удалённый 1-слотовый инвентарь), `GordoSnare` (приманка и поимка Гордо с учётом любимой еды / качества силка), `EchoNet` и `EchoNoteGordo`, `DashPad`, `PortableWaterTap`, `PortableSlimeBait`, `ChickenCloner`, декор/лампы/Fashion Pods.
- **Коммит 7.4 — `DroneNetwork` и `DroneStation`**
  - Станции дронов (`DRONE`, `DRONE_ADVANCED`), заряд батареи водой (`DroneStationBattery`, `1` цикл = 24 игровых часа), 1 программа у обычного дрона и 2 у продвинутого.
- **Коммит 7.5 — ИИ дронов (`Drone`, `DroneSubbehaviourPlexer`, `DronePather`)**
  - Навигация по графу сети ранчо, все `DroneProgramSource` (Corral, PlortCollector, Garden, Coop, Pond, Incinerator, Silo, FreeRange, OutsidePlots) и `DroneProgramDestination` (Silo, Feeder, PlortMarket, Refinery, Incinerator), поведение `Idle`/`Rest` при разряде или отсутствии задач.

---

### Шаг 8. Сюжет, Starmail, Журналы Хобсона, Slimepedia и Финал
*Цель: гарантировать 100% сохранение нарратива, порядка событий и концовки.*

- **Коммит 8.1 — `ProgressDirector` и `DelayedProgressTracker`**
  - Полный перенос счётчиков прогресса (`SLIME_DOORS`, `PLORT_DOOR`, разблокировки зон и ранчеров) и отложенных трекеров.
- **Коммит 8.2 — `MailDirector` и сюжетная линия Кейси (`CASEY_1..11` + `HOBSON_1`)**
  - Точная цепочка условий и задержек отправки каждого письма (стартовое `welcome`, прогрессия по открытию слайм-врат и плорт-врат до `casey_11` через 6 часов после `HOBSON_END`, затем письмо Хобсона и `partner_rewards`).
- **Коммит 8.3 — Журналы Хобсона (`JournalEntry`), `PediaDirector` и `TutorialDirector`**
  - Интерактивные дроны Хобсона в мире (ключи лора + обновление прогресса, например `HOBSON_END` в конце Glass Desert).
  - Автоматическая разблокировка статей `PediaDirector` при первом всасывании, входе в зону (`PediaAreaTrigger`), чтении почты, обучении.
- **Коммит 8.4 — Финал, титры (`EndGameUI`, `EndGameV2UI`) и `AchievementsDirector`**
  - Финальный триггер после прочтения финального письма/журнала (`QueueCredits`, `QueueRanchWistfulMusic` при возвращении на ранчо).
  - Вся статистика и условия всех достижений (`AchievementsDirector`).

---

### Шаг 9. Поздние обновления контента (Ogden, Mochi, Viktor, DLC & Holidays)
*Цель: полностью перенести дополнительные зоны и режимы поздних патчей (v1.2–1.4).*

- **Коммит 9.1 — Дебри Огдена (`Ogden's Wilds`)**
  - Условие разблокировки (`UNLOCK_MOSS` + `UNLOCK_OVERGROWTH`), миссии Огдена, `KookadobaPatchNode`, обмен кукадоба-фруктов на награды и расширение ранчо Огдена.
- **Коммит 9.2 — Стремительная долина Мочи (`Mochi's Nimble Valley`)**
  - Условие разблокировки (`UNLOCK_QUARRY` + `UNLOCK_GROTTO`), `QuicksilverSlimeSpawner`, `QuicksilverEnergyGenerator`, чекпоинты энергии, электрические/замедляющие поля, отдельный режим слотов вакпака `NIMBLE_VALLEY` (`VALLEY_AMMO_1..4`), усадьба Мочи.
- **Коммит 9.3 — Слимуляция Виктора (`Viktor's Slimulation`)**
  - Условие разблокировки (`UNLOCK_LAB` + `UNLOCK_RUINS` + `TREASURE_CRACKER_2`), `GlitchCellDirector`, `GlitchSlime`, `GlitchImposto` (мимики объектов), `GlitchTarrNode` и обезвреживание отладочным спреем (`GLITCH_DEBUG_SPRAY_LIQUID`), сбор `GLITCH_BUG_REPORT`, лаборатория Виктора.
- **Коммит 9.4 — `HolidayDirector` и `SlimeAppearanceDirector` (Secret Style)**
  - Логика `Party Gordo` по выходным и `Twinkle Slime` (Wiggly Wonderland), орнаменты (`ORNAMENT_CLASS`), смена стилей слаймов (`AppearancesModel`).

---

### Шаг 10. Бинарные сохранения `sr_persist` (совместимость с `.sav` оригинала)
*Цель: научиться читать и писать оригинальные файлы сохранений Slime Rancher `GameV12`.*

- **Коммит 10.1 — Базовый бинарный ридер/райтер (`PersistedDataSet`, `VersionedPersistedDataSet`)**
  - Точная реализация заголовков (`"SRGAME"` + `Version`), разделителей секций (`ReadSectionSeparator`), примитивов `BinaryReader`/`BinaryWriter` из .NET.
- **Коммит 10.2 — Десериализаторы/сериализаторы моделей (`WorldV22`, `PlayerV14`, `RanchV07`, `ActorDataV09`)**
  - Маппинг всех полей в `sr_model` и обратно.
  - (Опционально) апгрейдеры старых версий `GameV04..V11` → `GameV12`.
- **Коммит 10.3 — `AutoSaveDirector` и тесты round-trip на реальных `.sav`**
  - Автосохранение, ротация бэкапов, загрузка/сохранение профиля.
  - Интеграционный тест `load(.sav) -> GameModel -> save() -> load()` с побайтовой/семантической проверкой `AssertAreEqual`.

---

### Шаг 11. 3D-клиент на Bevy, UI, звук, локализация и экспорт ассетов
*Цель: превратить готовую Rust-симуляцию в полноценную играбельную 3D-игру.*

- **Коммит 11.1 — Экспортёр/конвертер Unity-ассетов (`sr_tools`)**
  - Парсер выгруженных таблиц баланса, сцен (координаты, меши/коллайдеры, спавнеры) и `.properties`-файлов локализации в оптимизированные форматы для `assets/`.
- **Коммит 11.2 — Инициализация `sr_client` (Bevy + Rapier3D)**
  - Окно, инпут (мышь/клавиатура/геймпад), камера от первого лица, освещение и цикл дня/ночи (`TimeOfDay`), небо/облака, вода.
- **Коммит 11.3 — Визуализация слаймов, вакпака, ранчо и мира**
  - Слайм-шейдеры/материалы, эмоции на лицах (`SlimeFaceAnimator`), визуал вак-луча и удержания, модели участков и гаджетов.
- **Коммит 11.4 — Весь внутриигровой HUD и UI-экраны**
  - HUD (HP, энергия, рады, слоты вакпака, часы/день), `MailUI`, `PediaUI`, `JournalUI`, `CorporatePartnerUI`, `ExchangeUI`, `LandPlotUI`, `FabricateGadgetUI`, `DroneUI`, карта мира (`MapUI`), `EndGameV2UI`.
- **Коммит 11.5 — Звук, музыка (`MusicDirector`, `AmbianceDirector`), локализация и профилирование**
  - Подсистема звука и динамической музыки по зонам/событиям (тарры, слаймы рядом, финал), полная RU/EN локализация, бенчмарк потребления RAM и frame time.

---

## 4. Что понадобится на следующем шаге (для 100% точности данных)

Поскольку `Assembly-CSharp.dll` содержит только код, а сами **данные баланса, тексты и геометрия карты** сериализованы в ресурсах Unity, на этапе наполнения `assets/` нам понадобятся:
1. **Локализация** (`resources.assets` / текстовые ресурсы `mail`, `pedia`, `ui`, `journal`, `exchange`, `achieve` для `en` и `ru`).
2. **Конфиги баланса** (дамп `ScriptableObject` / префабов через AssetStudio / UABEA / небольшой мод-дампер в JSON: цены плортов, диеты слаймов, рецепты гаджетов, награды 7Zee, спавнеры и позиции `LandPlotLocation` / `GadgetSite` / `Gordo` / `JournalEntry`).
3. **Геометрия мира / коллайдеры** (экспорт мешей зон в `.glb`/`.gltf`).

Начинать программировать **Шаги 0–3** можно уже сейчас — для них достаточно только проанализированного C#-кода!
