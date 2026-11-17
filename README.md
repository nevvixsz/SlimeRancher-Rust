# SlimeOptimizer (SRML + Harmony)

Мод оптимизации производительности и снижения микрофризов (GC) для **Slime Rancher** на базе **SRML** (`0.2.1.0+`) и **Harmony**.

## Что включено (v0.4.0)

### GPU / Рендеринг (против просадок на общем виде ранчо и в густой траве)
1. **Slime Render LOD (`RubberBoneLodPatch` + `SlimeAppearanceLodTrackerPatch`)**:
   - Отслеживает все уровни `LOD0`–`LOD3` и аксессуары слайма.
   - На дистанции `>= 18 м` переключает скиннинг на 1-костный (`SkinQuality.Bone1`), на `>= 24 м` отключает отбрасывание теней.
2. **Оптимизатор густой травы (`GrassOptimizerPatch`)**:
   - При загрузке сцен/чанков отключает отбрасывание теней у декоративной травы (`grass*`, `sprig*`, `drapeMoss`, `fern`), сдвигает пороги её `LODGroup` и ограничивает `shader.maximumLOD = 400` **только для шейдеров травы** (отключая тяжёлое ветровое колыхание без ухудшения воды, слаймов и остального мира).
   - Переключается прямо посреди травы командами `slimeopt toggle grass` и `slimeopt toggle grasswind`.
3. **Послойное отсечение камеры (`CameraCullDistancePatch`)**:
   - Настраивает сферический `layerCullDistances` (`95 м`) для слоёв мелких акторов (`Actor`, `ActorEchoes`, `ActorIgnorePlayer`, `Launched`) и ограничивает `farClipPlane` до `450 м`, не затрагивая скалы, небо и крупные постройки (`slimeopt toggle cameracull`).
4. **Отключение теней у мелких объектов (`SmallActorShadowPatch`)**:
   - Отключает отбрасывание теней у плортов, овощей, фруктов, мяса, цыплят и крафт-ресурсов при спавне.
5. **Умные лимиты графики (`QualityCapsPatch`)**:
   - На максимальных настройках игра ставит `4` каскада теней, `VeryHigh` разрешение теней, `150 м` дальности теней, `8x MSAA`, `4` пиксельных источника света и `lodBias = 2.0`.
   - По умолчанию мод ограничивает потолок до `2` каскадов, `Medium` разрешения теней (`MaxShadowResolution = 1`), `85 м`, `2x MSAA`, `2` пиксельных источников и `lodBias = 1.2` (мгновенно переключается на лету командой `slimeopt toggle qcaps`).

### CPU / ИИ / GC (против лагов и статтеров от сотни слаймов)
6. **ИИ LOD для дальних слаймов (`SlimeAiLodPatch`)** — на дистанции `>= 24 м` замедляет пересчёт решений ИИ (`1.0s → 2.8s`) и рейкасты земли (`0.25s → 0.6s`), на `>= 48 м` — до `5.0s / 1.0s`.
7. **Быстрый поиск еды (`FindConsumableFastPatch`)** — убирает `GetComponent<Identifiable>()` на каждом плорте/еде в радиусе (`ActorModel` берётся напрямую из `Entry`) и устраняет боксинг перечисления ключей `searchIds`.
8. **RubberBone LOD (`RubberBoneLodPatch`)** — дистанционный LOD и усыпление желе-физики костей (`< 10 м` 100%, `10–20 м` раз в 3 кадра, `>= 20 м` `Reset()` + пропуск).
9. **Кэш материалов лица (`ExpressionCachePatch`)**, **пул списка в `PlortCollector.Update` (`PlortCollectorPatch`)**, **быстрый подсчёт в `CellDirector.Update` (`CellDirectorPatch`)**, **пул `List<Collider>` (`TrackCollisionsPatch`)**, **кэш камеры и компонентов (`BillboardCachePatch`, `DisableEffectsCachePatch`)**, **кэш делегата `SlimeSubbehaviourPlexer` (`SlimePlexerCachePatch`)**.

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
- `slimeopt toggle <module>` — включить/выключить на лету (`rubber`, `renderlod`, `ailod`, `foodsearch`, `grass`, `grasswind`, `cameracull`, `smallshadows`, `qcaps`, `expr`, `plort`, `cell`, `collisions`, `billboard`, `effects`, `plexer`, `stats`)
- `slimeopt reset` — обнулить счётчики

Файл конфигурации:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\Config\slimeoptimizer\SlimeOptimizer.ini`

Лог SRML:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\srml.log`
