# SlimeOptimizer (SRML + Harmony)

Мод оптимизации производительности и снижения микрофризов (GC) для **Slime Rancher** на базе **SRML** (`0.2.1.0+`) и **Harmony**.

## Что включено (v0.6.0)

### Меню настроек в стиле Sodium / OptiFine (`Options` → вкладка `Расширенное` / `Advanced`)
- В официальное меню **Настройки** (и в главном меню, и в меню паузы в игре) добавлена отдельная прокручиваемая вкладка **«Расширенное»** (`AdvancedOptionsPatch`):
  - **Быстрые пресеты в 1 клик:** `Макс. FPS`, `Баланс` (по умолчанию, все оптимизации включены), `Качество`, `Ваниль (Выкл.)`.
  - **Ползунки и переключатели на лету** для всех модулей GPU, SSAO, травы, камеры, слаймов, ИИ, кэшей и сбора статистики.
  - **База настроек (`SettingsDatabase`)**: все изменения из вкладки и консоли автоматически сохраняются в `SlimeOptimizer.db` (`SRML/Config/slimeoptimizer/SlimeOptimizer.db`) и применяются сразу при запуске игры.

### GPU / Рендеринг (против просадок на общем виде ранчо и в густой траве)
1. **Ускорение SSAO (`SsaoFastPatch`)**:
   - Переводит `SSAOPro` и его `HighQualityBilateral` блюр в половинное разрешение (`Downsampling = 2`, `BlurDownsampling = true` — **в 4 раза меньше пикселей на GPU**), сохраняя само объёмное затенение (`slimeopt toggle ssao`).
   - Убирает покадровый сброс `Material.shaderKeywords = null` и кэширует `Shader.PropertyToID` вместо 9 строковых поисков каждый кадр.
2. **Slime Render LOD (`RubberBoneLodPatch` + `SlimeAppearanceLodTrackerPatch`)**:
   - Отслеживает все уровни `LOD0`–`LOD3` и аксессуары слайма.
   - На дистанции `>= 18 м` переключает скиннинг на 1-костный (`SkinQuality.Bone1`), на `>= 24 м` отключает отбрасывание теней.
3. **Оптимизатор густой травы (`GrassOptimizerPatch`)**:
   - При загрузке сцен/чанков отключает отбрасывание теней у декоративной травы (`grass*`, `sprig*`, `drapeMoss`, `fern`), сдвигает пороги её `LODGroup` и ограничивает `shader.maximumLOD = 400` **только для шейдеров травы** (отключая тяжёлое ветровое колыхание без ухудшения воды, слаймов и остального мира).
   - Переключается прямо посреди травы командами `slimeopt toggle grass` и `slimeopt toggle grasswind`.
4. **Послойное отсечение камеры (`CameraCullDistancePatch`)**:
   - Настраивает сферический `layerCullDistances` (`95 м`) для слоёв мелких акторов (`Actor`, `ActorEchoes`, `ActorIgnorePlayer`, `Launched`) и ограничивает `farClipPlane` до `450 м`, не затрагивая скалы, небо и крупные постройки (`slimeopt toggle cameracull`).
5. **Отключение теней у мелких объектов (`SmallActorShadowPatch`)**:
   - Отключает отбрасывание теней у плортов, овощей, фруктов, мяса, цыплят и крафт-ресурсов при спавне.
6. **Умные лимиты графики (`QualityCapsPatch`)**:
   - На максимальных настройках игра ставит `4` каскада теней, `VeryHigh` разрешение теней, `150 м` дальности теней, `8x MSAA`, `4` пиксельных источника света и `lodBias = 2.0`.
   - По умолчанию мод ограничивает потолок до `2` каскадов, `Medium` разрешения теней (`MaxShadowResolution = 1`), `85 м`, `2x MSAA`, `2` пиксельных источников и `lodBias = 1.2` (мгновенно переключается на лету командой `slimeopt toggle qcaps`).

### CPU / ИИ / GC (против лагов и статтеров от сотни слаймов)
7. **ИИ LOD для дальних слаймов (`SlimeAiLodPatch`)** — на дистанции `>= 24 м` замедляет пересчёт решений ИИ (`1.0s → 2.8s`) и рейкасты земли (`0.25s → 0.6s`), на `>= 48 м` — до `5.0s / 1.0s`.
8. **LOD лиц и эмоций слаймов (`SlimeFaceEmotionLodPatch`)** — на дистанции `>= 20 м` обновляет `SlimeFaceAnimator.RegistryUpdate()` и `SlimeEmotions.RegistryUpdate()` раз в 6 кадров (на `>= 45 м` — раз в 12 кадров), без потери точности расчёта времени (`slimeopt toggle facelod`).
9. **Быстрый поиск еды (`FindConsumableFastPatch`)** — убирает `GetComponent<Identifiable>()` на каждом плорте/еде в радиусе (`ActorModel` берётся напрямую из `Entry`) и устраняет боксинг перечисления ключей `searchIds`.
10. **RubberBone LOD (`RubberBoneLodPatch`)** — дистанционный LOD и усыпление желе-физики костей (`< 10 м` 100%, `10–20 м` раз в 3 кадра, `>= 20 м` `Reset()` + пропуск).
11. **Кэш материалов лица (`ExpressionCachePatch`)**, **пул списка в `PlortCollector.Update` (`PlortCollectorPatch`)**, **быстрый подсчёт в `CellDirector.Update` (`CellDirectorPatch`)**, **пул `List<Collider>` (`TrackCollisionsPatch`)**, **кэш камеры и компонентов (`BillboardCachePatch`, `DisableEffectsCachePatch`)**, **кэш делегата `SlimeSubbehaviourPlexer` (`SlimePlexerCachePatch`)**.

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
- `slimeopt preset <maxfps|balanced|quality|vanilla>` — мгновенно применить пресет настроек и сохранить в базу
- `slimeopt toggle <module>` — включить/выключить на лету и сохранить в базу (`rubber`, `renderlod`, `ailod`, `facelod`, `foodsearch`, `grass`, `grasswind`, `ssao`, `cameracull`, `smallshadows`, `qcaps`, `expr`, `plort`, `cell`, `collisions`, `billboard`, `effects`, `plexer`, `stats`)
- `slimeopt reset` — обнулить счётчики

Файл конфигурации:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\Config\slimeoptimizer\SlimeOptimizer.ini`

Лог SRML:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\srml.log`
