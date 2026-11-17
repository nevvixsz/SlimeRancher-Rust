# SlimeOptimizer (SRML + Harmony)

Мод оптимизации производительности и снижения микрофризов (GC) для **Slime Rancher** на базе **SRML** (`0.2.1.0+`) и **Harmony**.

## Что включено

1. **RubberBone LOD (`RubberBoneLodPatch`)** — главный прирост FPS на ранчо:
   - Вблизи (`< 18 м`) и при захвате вакпаком желе-физика работает на полной частоте без изменений.
   - В средней зоне (`18–36 м`) обновление желе переводится на каждый `N`-й кадр (по умолчанию `3`), с равномерным распределением слаймов по кадрам.
   - Дальше `36 м` кости один раз сбрасываются в покой (`Reset()`), и `RubberBoneEffect.LateUpdate` пропускается до приближения игрока.
2. **Кэш материалов лица слаймов (`ExpressionCachePatch`)** — убирает покадровый GC-мусор при моргании и смене эмоций:
   - Переиспользует кэшированный `Material[]` на `Renderer` и не переприсваивает `sharedMaterials`, если ссылки `Eyes`/`Mouth` не изменились.
   - Автоматически сбрасывает кэш при `ClearAppearance()` (поддержка пула объектов и переключения Secret Style).
3. **Пул списка в `PlortCollector.Update` (`PlortCollectorPatch`)** — Harmony-транспайлер (совместимый с `PlortCollectorUpdatePatch` из SRML), убирающий создание `new List<JointReference>()` на каждом кадре во время сбора плортов.
4. **Быстрый подсчёт в `CellDirector.Update` (`CellDirectorPatch`)** — замена LINQ `.Count<Entry>()` на прямое чтение `ICollection.Count`.
5. **Пул `List<Collider>` и быстрый выход в `TrackCollisions` (`TrackCollisionsPatch`)** — переиспользование списков коллайдеров и ранний возврат при пустом триггере.
6. **Кэш камеры для билбордов (`BillboardCachePatch`)** — замена повторных поисков `Camera.main` в `CameraFacingBillboard` на кэш раз в кадр (`CameraCache`).

---

## Как собрать (Windows)

Требуются установленные **Slime Rancher** с **SRML** и **.NET SDK** (например, .NET 6/8 SDK) или Visual Studio / MSBuild.

1. Клонируйте или обновите репозиторий:
   ```powershell
   git pull
   ```
2. Укажите путь к папке с игрой одним из способов:
   - **Вариант A (через файл):** скопируйте `SlimeOptimizer\Directory.Build.props.example` в `SlimeOptimizer\Directory.Build.props` (в `.gitignore`) и впишите свой путь в `<SlimeRancherDir>`.
   - **Вариант B (параметром в терминале):** передайте путь напрямую при сборке:
     ```powershell
     dotnet build SlimeOptimizer\SlimeOptimizer.csproj -c Release -p:SlimeRancherDir="D:\Games\Slime Rancher"
     ```
3. Готовый мод (один файл с вшитым `modinfo.json`) появится по пути:
   ```text
   SlimeOptimizer\bin\Release\SlimeOptimizer.dll
   ```

---

## Как установить

1. Убедитесь, что в папке с игрой установлен **SRML** (в `SlimeRancher_Data\Managed\` есть `SRML.dll` и `0Harmony.dll`).
2. Скопируйте `SlimeOptimizer.dll` в папку:
   ```text
   <Папка с игрой>\SRML\Mods\SlimeOptimizer.dll
   ```
3. Запустите игру. В логе/консоли SRML появится:
   `SlimeOptimizer initialized (Harmony patches applied). Use 'slimeopt' in console.`

---

## Консольные команды и настройка

Откройте консоль SRML в игре (`Ctrl + Tab` или `~` в зависимости от раскладки) и введите:

- `slimeopt status` — какие модули активны и текущие пороги LOD
- `slimeopt stats` — сколько вызовов и аллокаций сэкономлено с момента запуска
- `slimeopt reset` — обнулить счётчики
- `slimeopt toggle rubber` (или `expr`, `plort`, `cell`, `collisions`, `billboard`, `stats`) — мгновенно включать/выключать конкретный патч прямо в игре для замера разницы FPS

Файл конфигурации создаётся автоматически после первого запуска игры в:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\Config\slimeoptimizer\SlimeOptimizer.ini`
(там можно подкрутить `RubberLodDistance`, `RubberDisableDistance`, `RubberLodInterval` и тумблеры патчей).
