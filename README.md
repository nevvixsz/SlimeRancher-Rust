# SlimeOptimizer (SRML + Harmony)

Мод оптимизации производительности и снижения микрофризов (GC) для **Slime Rancher** на базе **SRML** (`0.2.1.0+`) и **Harmony**.

## Что включено (v0.7.0)

### Меню настроек в стиле Sodium / OptiFine (`Options` → вкладка `Расширенное` / `Advanced`) + FPS HUD
- В официальное меню **Настройки** (и в главном меню, и в меню паузы в игре) добавлена отдельная прокручиваемая вкладка **«Расширенное»** (`AdvancedOptionsPatch`):
  - **Быстрые пресеты в 1 клик:**
    - **`Макс. FPS`**: скручивает **и все ванильные настройки графики (`SRQualitySettings` → `LOWEST`, тени Выкл, AO Выкл, Bloom Выкл)**, **и все параметры мода** на абсолютный максимум FPS без необходимости лезть в другие вкладки!
    - **`Баланс`**, **`Качество`**, **`Ваниль (Выкл.)`**.
  - **Встроенный FPS / Frametime HUD (`FpsHudOverlay`)**: лёгкий оверлей сверху по центру (`FPS | min | ms`), включается прямо во вкладке или через `slimeopt toggle hud`.
  - **База настроек (`SettingsDatabase`)**: все изменения автоматически сохраняются в `SlimeOptimizer.db` (`SRML/Config/slimeoptimizer/SlimeOptimizer.db`).

### GPU / Рендеринг (против просадок на общем виде ранчо и в густой траве)
1. **Ускорение `Bloom` (`BloomFastPatch`) + `SoftParticles` / `Anisotropic` (`QualityCapsPatch`)**:
   - Оптимизирует `Bloom` (`1` проход блюра, `Low` разрешение, `Cheap` качество) при включённом эффекте.
   - Отключает тяжёлые мягкие частицы (`softParticles = false`), ограничивает `particleRaycastBudget` (`1024 → 128`) и `anisotropicFiltering`.
2. **Ускорение SSAO (`SsaoFastPatch`)**:
   - Переводит `SSAOPro` и его `HighQualityBilateral` блюр в половинное разрешение (`Downsampling = 2`, `BlurDownsampling = true` — **в 4 раза меньше пикселей на GPU**), сохраняя само объёмное затенение (`slimeopt toggle ssao`).
   - Убирает покадровый сброс `Material.shaderKeywords = null` и кэширует `Shader.PropertyToID` вместо 9 строковых поисков каждый кадр.
3. **Slime Render LOD (`RubberBoneLodPatch` + `SlimeAppearanceLodTrackerPatch`)**:
   - Отслеживает все уровни `LOD0`–`LOD3` и аксессуары слайма.
   - На дистанции `>= 18 м` переключает скиннинг на 1-костный (`SkinQuality.Bone1`), на `>= 24 м` отключает отбрасывание теней.
4. **Оптимизатор густой травы (`GrassOptimizerPatch`)**:
   - При загрузке сцен/чанков отключает отбрасывание теней у декоративной травы (`grass*`, `sprig*`, `drapeMoss`, `fern`), сдвигает пороги её `LODGroup` и ограничивает `shader.maximumLOD = 400` **только для шейдеров травы** (отключая тяжёлое ветровое колыхание без ухудшения воды, слаймов и остального мира).
   - Переключается прямо посреди травы командами `slimeopt toggle grass` и `slimeopt toggle grasswind`.
5. **Послойное отсечение камеры (`CameraCullDistancePatch`)**:
   - Настраивает сферический `layerCullDistances` (`95 м`) для слоёв мелких акторов (`Actor`, `ActorEchoes`, `ActorIgnorePlayer`, `Launched`) и ограничивает `farClipPlane` до `450 м`, не затрагивая скалы, небо и крупные постройки (`slimeopt toggle cameracull`).
6. **Отключение теней у мелких объектов (`SmallActorShadowPatch`)** и **умные лимиты графики (`QualityCapsPatch`)** (каскады `0/2/4` + `CloseFit`, разрешение теней, дальность теней, MSAA, пиксельный свет, `lodBias`).

### CPU / Физика / ИИ / GC (против лагов и статтеров от сотни слаймов)
7. **Куллинг аниматоров за спиной камеры (`AnimatorCullingPatch`)** — переводит мировых акторов (курицы, декор, экстракторы, дроны) в `AnimatorCullingMode.CullUpdateTransforms`.
8. **Физика ранчо (`RanchPhysicsLodPatch` + PhysX Solver)** — пропускает `KeepUpright` у спящих и дальних тел, замедляет `ChickenRandomMove` на дистанции и оптимизирует `Physics.defaultSolverIterations` (`6 → 4`, в `MaxFps` → `3`).
9. **ИИ LOD для дальних слаймов (`SlimeAiLodPatch`)** и **LOD лиц и эмоций (`SlimeFaceEmotionLodPatch`)**.
10. **Быстрый поиск еды (`FindConsumableFastPatch`)**, **RubberBone LOD (`RubberBoneLodPatch`)**, **кэш материалов лица (`ExpressionCachePatch`)**, **пул `PlortCollector.Update`**, **`CellDirector.Update`**, **`TrackCollisions`**, **`CameraFacingBillboard`**, **`DisableEffectsOnLowQuality`**, **кэш делегата `SlimeSubbehaviourPlexer`**.

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
- `slimeopt toggle <module>` — включить/выключить на лету и сохранить в базу (`rubber`, `renderlod`, `ailod`, `facelod`, `foodsearch`, `grass`, `grasswind`, `ssao`, `bloom`, `animcull`, `physopt`, `hud`, `cameracull`, `smallshadows`, `qcaps`, `expr`, `plort`, `cell`, `collisions`, `billboard`, `effects`, `plexer`, `stats`)
- `slimeopt reset` — обнулить счётчики

Файл конфигурации:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\Config\slimeoptimizer\SlimeOptimizer.ini`

Лог SRML:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\srml.log`
