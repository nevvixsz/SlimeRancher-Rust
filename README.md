# SlimeOptimizer (SRML + Harmony)

Мод оптимизации производительности и снижения микрофризов (GC) для **Slime Rancher** на базе **SRML** (`0.2.1.0+`) и **Harmony**.

## Что включено (v0.3.0)

### GPU / Рендеринг (против просадок на общем виде ранчо и в густой траве)
1. **Slime Render LOD (`RubberBoneLodPatch` + `SlimeAppearanceLodTrackerPatch`)**:
   - Отслеживает все уровни `LOD0`–`LOD3` и аксессуары слайма.
   - На дистанции `>= 18 м` переключает скиннинг на 1-костный (`SkinQuality.Bone1`), на `>= 24 м` отключает отбрасывание теней.
2. **Оптимизатор густой травы (`GrassOptimizerPatch`)**:
   - При загрузке секторов (`SECTR_Sector`) отключает отбрасывание теней у декоративной травы (`grass*`, `sprig*`) и немного сдвигает пороги переключения её `LODGroup` (не затрагивая здания, слаймов и деревья).
   - Можно переключать прямо посреди травы командой `slimeopt toggle grass`.
3. **Отключение теней у мелких объектов (`SmallActorShadowPatch`)**:
   - Отключает отбрасывание теней у плортов, овощей, фруктов, мяса, цыплят и крафт-ресурсов при спавне.
4. **Умные лимиты теней, MSAA и `lodBias` (`QualityCapsPatch`)**:
   - На максимальных настройках игра ставит `4` каскада теней, `150 м` дальности теней, `8x MSAA` и `lodBias = 2.0`.
   - По умолчанию мод оставляет максимальные шейдеры, текстуры, объёмный свет, SSAO и Bloom, но ограничивает потолок до `2` каскадов, `85 м`, `2x MSAA` и `lodBias = 1.2` (мгновенно переключается на лету командой `slimeopt toggle qcaps`).

### CPU / ИИ / GC (против лагов и статтеров от сотни слаймов)
5. **ИИ LOD для дальних слаймов (`SlimeAiLodPatch`)** — на дистанции `>= 24 м` замедляет пересчёт решений ИИ (`1.0s → 2.8s`) и рейкасты земли (`0.25s → 0.6s`), на `>= 48 м` — до `5.0s / 1.0s` (в вакууме и при перемотке времени работает на 100%).
6. **Быстрый поиск еды (`FindConsumableFastPatch`)** — убирает `GetComponent<Identifiable>()` на каждом плорте/еде в радиусе (`ActorModel` берётся напрямую из `Entry`) и устраняет боксинг перечисления ключей `searchIds`.
7. **RubberBone LOD (`RubberBoneLodPatch`)** — дистанционный LOD и усыпление желе-физики костей (`< 10 м` 100%, `10–20 м` раз в 3 кадра, `>= 20 м` `Reset()` + пропуск).
8. **Кэш материалов лица (`ExpressionCachePatch`)** — убирает покадровый GC-мусор `Material[]` в `SlimeAppearanceApplicator.SetExpression()`.
9. **Пул списка в `PlortCollector.Update` (`PlortCollectorPatch`)** — Harmony-транспайлер (совместимый с SRML), переиспользующий один `List<JointReference>`.
10. **Быстрый подсчёт в `CellDirector.Update` (`CellDirectorPatch`)** — замена LINQ `.Count<Entry>()` на `ICollection.Count`.
11. **Пул `List<Collider>` и быстрый выход в `TrackCollisions` (`TrackCollisionsPatch`)**.
12. **Кэш камеры для билбордов (`BillboardCachePatch`)** и **кэш компонентов камеры (`DisableEffectsCachePatch`)** в `DisableEffectsOnLowQuality`.
13. **Кэш делегата `Action<RaycastHit>` (`SlimePlexerCachePatch`)** в `SlimeSubbehaviourPlexer.RegistryFixedUpdate()`.

---

## Как собрать (Windows)

```powershell
git pull
dotnet build SlimeOptimizer\SlimeOptimizer.csproj -c Release -p:SlimeRancherDir="D:\Slime Rancher"
```

Готовый файл (один DLL со вшитым `modinfo.json`):
`SlimeOptimizer\bin\Release\SlimeOptimizer.dll`

---

## Как установить

Скопируйте `SlimeOptimizer.dll` в:
```text
<Папка с игрой>\SRML\Mods\SlimeOptimizer.dll
```

---

## Консоль и конфиг

В консоли SRML (`Ctrl + Tab` или `~`):
- `slimeopt status` — активные модули, пороги и текущие `QualitySettings` (`cascades`, `dist`, `MSAA`)
- `slimeopt stats` — статистика сэкономленных вызовов и аллокаций
- `slimeopt toggle <module>` — включить/выключить на лету (`rubber`, `renderlod`, `ailod`, `foodsearch`, `grass`, `smallshadows`, `qcaps`, `expr`, `plort`, `cell`, `collisions`, `billboard`, `effects`, `plexer`, `stats`)
- `slimeopt reset` — обнулить счётчики

Файл конфигурации:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\Config\slimeoptimizer\SlimeOptimizer.ini`

Лог SRML:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\srml.log`
