# SlimeOptimizer (SRML + Harmony)

Мод оптимизации производительности и снижения микрофризов (GC) для **Slime Rancher** на базе **SRML** (`0.2.1.0+`) и **Harmony**.

## Что включено (v0.2.0)

### GPU / Рендеринг (против просадок на общем виде ранчо)
1. **Slime Render LOD (`RubberBoneLodPatch`)**:
   - На дистанции `>= 18 м` переключает скиннинг дальних слаймов на 1-костный (`SkinQuality.Bone1`).
   - На дистанции `>= 24 м` отключает отбрасывание теней дальними слаймами (вблизи и в вакпаке тени и полный скиннинг возвращаются автоматически).
2. **Отключение теней у мелких объектов (`SmallActorShadowPatch`)**:
   - Отключает отбрасывание теней у плортов, овощей, фруктов, мяса, цыплят и крафт-ресурсов при спавне — резко снижает нагрузку в картах теней на ранчо.
3. **Умные лимиты теней и MSAA (`QualityCapsPatch`)**:
   - На максимальных настройках игра ставит `4` каскада теней, `150 м` дальности теней и `8x MSAA`.
   - По умолчанию мод оставляет максимальные шейдеры, текстуры, объёмный свет, SSAO и Bloom, но ограничивает потолок до `2` каскадов, `85 м` и `2x MSAA` (все пороги настраиваются в `.ini`, можно мгновенно переключать на лету командой `slimeopt toggle qcaps`).

### CPU / GC (против лагов и статтеров от сотни слаймов)
4. **RubberBone LOD (`RubberBoneLodPatch`)** — дистанционный LOD и усыпление желе-физики костей (`< 10 м` 100%, `10–20 м` раз в 3 кадра, `>= 20 м` `Reset()` + пропуск).
5. **Кэш материалов лица (`ExpressionCachePatch`)** — убирает покадровый GC-мусор `Material[]` в `SlimeAppearanceApplicator.SetExpression()`.
6. **Пул списка в `PlortCollector.Update` (`PlortCollectorPatch`)** — Harmony-транспайлер (совместимый с SRML), переиспользующий один `List<JointReference>`.
7. **Быстрый подсчёт в `CellDirector.Update` (`CellDirectorPatch`)** — замена LINQ `.Count<Entry>()` на `ICollection.Count`.
8. **Пул `List<Collider>` и быстрый выход в `TrackCollisions` (`TrackCollisionsPatch`)**.
9. **Кэш камеры для билбордов (`BillboardCachePatch`)** и **кэш компонентов камеры (`DisableEffectsCachePatch`)** в `DisableEffectsOnLowQuality`.
10. **Кэш делегата `Action<RaycastHit>` (`SlimePlexerCachePatch`)** в `SlimeSubbehaviourPlexer.RegistryFixedUpdate()`.

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
- `slimeopt toggle <module>` — включить/выключить на лету (`rubber`, `renderlod`, `smallshadows`, `qcaps`, `expr`, `plort`, `cell`, `collisions`, `billboard`, `effects`, `plexer`, `stats`)
- `slimeopt reset` — обнулить счётчики

Файл конфигурации:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\Config\slimeoptimizer\SlimeOptimizer.ini`

Лог SRML:
`%UserProfile%\AppData\LocalLow\Monomi Park\Slime Rancher\SRML\srml.log`
