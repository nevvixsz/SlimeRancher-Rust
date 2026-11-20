# SlimeOptimizer (SRML + Harmony)

Мод оптимизации производительности и снижения микрофризов (GC) для **Slime Rancher** на базе **SRML** (`0.2.1.0+`) и **Harmony**.

## Что включено (v0.8.0)

### Меню настроек в стиле Sodium (`Options` → вкладка `Расширенное` / `Advanced`) + FPS HUD
- В официальное меню **Настройки** (и в главном меню, и в меню паузы в игре) добавлена структурированная вкладка **«Расширенное»** (`AdvancedOptionsPatch`):
  - **5 подвкладок-категорий (Sodium-style):** `Общее` | `Графика` | `Мир/Трава` | `Слаймы` | `Движок`.
  - **Живая строка FPS и сэкономленной работы** прямо вверху меню настроек.
  - **Лимитер FPS:** выбор потолка кадров (`30 / 60 / 90 / 120 / 144 / 165 / 240 / 300 / Без лимита`).
  - **Быстрые пресеты в 1 клик:**
    - **`Макс. FPS`**: скручивает **и все ванильные настройки графики (`SRQualitySettings` → `LOWEST`, тени Выкл, AO Выкл, Bloom Выкл)**, **и все параметры мода** на абсолютный максимум FPS без необходимости лезть в другие вкладки!
    - **`Баланс`**, **`Качество`**, **`Ваниль (Выкл.)`**.
  - **Встроенный FPS / Frametime HUD (`FpsHudOverlay`)**: лёгкий оверлей сверху по центру (`FPS | min | ms`), включается прямо во вкладке или через `slimeopt toggle hud`.
  - **База настроек (`SettingsDatabase`)**: все изменения автоматически сохраняются в `SlimeOptimizer.db` (`SRML/Config/slimeoptimizer/SlimeOptimizer.db`).

### GPU / Рендеринг (против просадок на общем виде ранчо, в густой траве и в замесах)
1. **Защита min FPS от взрывов и толпы частиц (`ParticleCapPatch`)**:
   - Ограничивает `maxParticles` для каждого `ParticleSystem`, запускаемого через `fxPool`, чтобы 1% low не падал при бум-слаймах и массовом засасывании.
2. **Ускорение `Bloom` (`BloomFastPatch`) + `SoftParticles` / `Anisotropic` (`QualityCapsPatch`)** — 1 проход блюра `Bloom`, отключение `softParticles`, ограничение `particleRaycastBudget` и `anisotropicFiltering`.
3. **Ускорение SSAO (`SsaoFastPatch`)** — половинное разрешение (`Downsampling = 2`, `BlurDownsampling = true`), кэш `Shader.PropertyToID` и `shaderKeywords`.
4. **Slime Render LOD**, **Оптимизатор густой травы (+ отключение ветрового шейдера травы)**, **Послойный куллинг камеры**, **Умные лимиты графики**, **Отключение теней у мелких объектов**.

### CPU / Таймеры / Физика / ИИ / GC
5. **LOD таймеров ранчо (`RanchTimersLodPatch`)** — прореживает фоновые проверки гниения/исчезновения и блёсток у `ResourceCycle`, `DestroyPlortAfterTime`, `DestroyAfterTime`, `GlintController` (не трогая объекты в вакууме или в процессе гниения).
6. **Куллинг аниматоров за спиной камеры (`AnimatorCullingPatch`)** и **Физика ранчо (`RanchPhysicsLodPatch` + PhysX Solver)**.
7. **ИИ LOD для дальних слаймов**, **LOD лиц и эмоций**, **Быстрый поиск еды**, **RubberBone LOD**, и все **GC/кэш-патчи**.

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
- `slimeopt toggle <module>` — включить/выключить на лету и сохранить в базу (`rubber`, `renderlod`, `ailod`, `facelod`, `foodsearch`, `grass`, `grasswind`, `ssao`, `bloom`, `particles`, `timers`, `animcull`, `physopt`, `hud`, `cameracull`, `smallshadows`, `qcaps`, `expr`, `plort`, `cell`, `collisions`, `billboard`, `effects`, `plexer`, `stats`)
- `slimeopt reset` — обнулить счётчики

Файл конфигурации:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\Config\slimeoptimizer\SlimeOptimizer.ini`

Лог SRML:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\srml.log`
