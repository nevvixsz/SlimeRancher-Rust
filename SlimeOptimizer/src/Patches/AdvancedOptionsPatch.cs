using System;
using System.Collections.Generic;
using System.Globalization;
using System.Reflection;
using System.Runtime.CompilerServices;
using HarmonyLib;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace SlimeOptimizer.Patches
{
    [HarmonyPatch(typeof(OptionsUI))]
    internal static class AdvancedOptionsPatch
    {
        private static readonly int[] FpsSteps = { 0, 30, 60, 90, 120, 144, 165, 240, 300 };

        private sealed class LiveBannerUpdater : MonoBehaviour
        {
            public TMP_Text BannerText;
            private float _timer;

            private void OnEnable()
            {
                _timer = 10f;
            }

            private void Update()
            {
                if (BannerText == null)
                {
                    return;
                }

                _timer += Time.unscaledDeltaTime;
                if (_timer < 0.5f)
                {
                    return;
                }
                _timer = 0f;

                long aiSaved = OptimizerStats.SlimeAiSlowedRethinks + OptimizerStats.SlimeAiSlowedGroundChecks +
                               OptimizerStats.FaceAnimCallsSkipped + OptimizerStats.EmotionsCallsSkipped +
                               OptimizerStats.ChickenAiSkipped;
                long gcSaved = OptimizerStats.ExpressionArrayAllocsAvoided + OptimizerStats.PlortCollectorListsReused +
                               OptimizerStats.ColliderListsReused + OptimizerStats.PlexerDelegatesReused;
                long worldSaved = OptimizerStats.GrassShadowsDisabled + OptimizerStats.AnimatorsCulled +
                                  OptimizerStats.RanchTimersSkipped + OptimizerStats.KeepUprightSkipped;

                BannerText.text = T(
                    $"FPS: {FpsHudOverlay.CurrentFps} (мин {FpsHudOverlay.MinFps}, {FpsHudOverlay.AvgFrameMs:0.0} ms)  |  Сэкономлено — ИИ: {aiSaved} • Мир/Таймеры: {worldSaved} • GC: {gcSaved}",
                    $"FPS: {FpsHudOverlay.CurrentFps} (min {FpsHudOverlay.MinFps}, {FpsHudOverlay.AvgFrameMs:0.0} ms)  |  Saved — AI: {aiSaved} • World/Timers: {worldSaved} • GC: {gcSaved}");
            }
        }

        private sealed class AdvancedUiState
        {
            public GameObject TabObject;
            public GameObject PanelObject;
            public ScrollRect Scroll;
            public TMP_Text InfoText;
            public int ActiveCategory;
            public readonly List<GameObject> CategoryContainers = new List<GameObject>();
            public readonly List<Image> SubTabImages = new List<Image>();
            public readonly List<Action> RefreshCallbacks = new List<Action>();
        }

        private static readonly ConditionalWeakTable<OptionsUI, AdvancedUiState> States =
            new ConditionalWeakTable<OptionsUI, AdvancedUiState>();

        private static readonly MethodInfo DeselectAllMethod =
            AccessTools.Method(typeof(OptionsUI), "DeselectAll");

        private static readonly MethodInfo NotifyQualityChangedMethod =
            AccessTools.Method(typeof(OptionsUI), "NotifyQualityChanged");

        [HarmonyPatch("SetupOptionsUI")]
        [HarmonyPostfix]
        private static void SetupOptionsUIPostfix(OptionsUI __instance)
        {
            if (__instance == null)
            {
                return;
            }

            FpsHudOverlay.IsOptionsMenuOpen = true;

            try
            {
                AdvancedUiState state = States.GetOrCreateValue(__instance);
                if (state.TabObject != null && state.PanelObject != null)
                {
                    RefreshState(state);
                    return;
                }

                BuildAdvancedUi(__instance, state);
            }
            catch (Exception ex)
            {
                ModMain.Logger?.LogWarning("Failed to build Options -> Advanced UI: " + ex);
            }
        }

        [HarmonyPatch("DeselectAll")]
        [HarmonyPostfix]
        private static void DeselectAllPostfix(OptionsUI __instance)
        {
            if (__instance != null && States.TryGetValue(__instance, out AdvancedUiState state))
            {
                if (state.PanelObject != null)
                {
                    state.PanelObject.SetActive(false);
                }
            }
        }

        [HarmonyPatch(nameof(OptionsUI.Close))]
        [HarmonyPostfix]
        private static void ClosePostfix()
        {
            FpsHudOverlay.IsOptionsMenuOpen = false;
            SettingsDatabase.Save();
        }

        private static bool IsRussian()
        {
            try
            {
                if (SRSingleton<GameContext>.Instance != null && SRSingleton<GameContext>.Instance.MessageDirector != null)
                {
                    CultureInfo culture = SRSingleton<GameContext>.Instance.MessageDirector.GetCulture();
                    return culture != null && string.Equals(culture.TwoLetterISOLanguageName, "ru", StringComparison.OrdinalIgnoreCase);
                }
            }
            catch
            {
            }
            return false;
        }

        private static string T(string ru, string en)
        {
            return IsRussian() ? ru : en;
        }

        private static void SelectAdvancedTab(OptionsUI ui, AdvancedUiState state)
        {
            DeselectAllMethod?.Invoke(ui, null);
            if (state.PanelObject != null)
            {
                state.PanelObject.SetActive(true);
            }
            SelectSubCategory(state, state.ActiveCategory);
            RefreshState(state);
        }

        private static void SelectSubCategory(AdvancedUiState state, int idx)
        {
            if (idx < 0 || idx >= state.CategoryContainers.Count)
            {
                idx = 0;
            }
            state.ActiveCategory = idx;

            for (int i = 0; i < state.CategoryContainers.Count; i++)
            {
                if (state.CategoryContainers[i] != null)
                {
                    state.CategoryContainers[i].SetActive(i == idx);
                }
            }

            for (int i = 0; i < state.SubTabImages.Count; i++)
            {
                if (state.SubTabImages[i] != null)
                {
                    state.SubTabImages[i].color = i == idx
                        ? new Color(0.24f, 0.58f, 0.86f)
                        : new Color(0.18f, 0.24f, 0.34f);
                }
            }

            if (state.Scroll != null)
            {
                state.Scroll.verticalNormalizedPosition = 1f;
            }
        }

        private static void RefreshState(AdvancedUiState state)
        {
            for (int i = 0; i < state.RefreshCallbacks.Count; i++)
            {
                state.RefreshCallbacks[i]();
            }

            if (state.InfoText != null)
            {
                state.InfoText.text = T(
                    "Профайлер: " + FrameProfiler.LastSummary + "\nБаза: " + SettingsDatabase.DatabasePath + " | Шейдеры: " + GrassOptimizerPatch.GetSeenShadersSummary(),
                    "Profiler: " + FrameProfiler.LastSummary + "\nDB: " + SettingsDatabase.DatabasePath + " | Shaders: " + GrassOptimizerPatch.GetSeenShadersSummary());
            }
        }

        private static void BuildAdvancedUi(OptionsUI ui, AdvancedUiState state)
        {
            if (ui.tabsPanel == null || ui.otherTab == null || ui.videoPanel == null)
            {
                return;
            }

            GameObject tabGo = UnityEngine.Object.Instantiate(ui.otherTab, ui.tabsPanel.transform, false);
            tabGo.name = "SlimeOptimizerAdvancedTab";
            tabGo.SetActive(true);
            StripXlateText(tabGo);

            TMP_Text tabLabel = tabGo.GetComponentInChildren<TMP_Text>(true);
            if (tabLabel != null)
            {
                tabLabel.text = T("Расширенное", "Advanced");
            }

            if (ui.tabsPanel.GetComponent<LayoutGroup>() == null)
            {
                RectTransform otherRt = ui.otherTab.GetComponent<RectTransform>();
                RectTransform tabRt = tabGo.GetComponent<RectTransform>();
                if (otherRt != null && tabRt != null)
                {
                    tabRt.anchoredPosition = new Vector2(otherRt.anchoredPosition.x + otherRt.rect.width + 6f, otherRt.anchoredPosition.y);
                }
            }

            Button tabBtn = tabGo.GetComponent<Button>();
            if (tabBtn != null)
            {
                tabBtn.onClick = new Button.ButtonClickedEvent();
                tabBtn.onClick.AddListener(delegate { SelectAdvancedTab(ui, state); });
            }

            Toggle tabToggle = tabGo.GetComponent<Toggle>();
            if (tabToggle != null)
            {
                tabToggle.onValueChanged = new Toggle.ToggleEvent();
                tabToggle.isOn = false;
                tabToggle.onValueChanged.AddListener(delegate(bool isOn)
                {
                    if (isOn)
                    {
                        SelectAdvancedTab(ui, state);
                    }
                });
            }

            GameObject panelGo = UnityEngine.Object.Instantiate(ui.videoPanel, ui.videoPanel.transform.parent, false);
            panelGo.name = "SlimeOptimizerAdvancedPanel";
            panelGo.SetActive(false);

            ContentSizeFitter panelCsf = panelGo.GetComponent<ContentSizeFitter>();
            if (panelCsf != null) UnityEngine.Object.DestroyImmediate(panelCsf);
            LayoutGroup panelLg = panelGo.GetComponent<LayoutGroup>();
            if (panelLg != null) UnityEngine.Object.DestroyImmediate(panelLg);

            for (int i = panelGo.transform.childCount - 1; i >= 0; i--)
            {
                Transform child = panelGo.transform.GetChild(i);
                if (child.GetComponent<Selectable>() != null ||
                    child.GetComponentInChildren<Selectable>(true) != null ||
                    child.GetComponent<TMP_Text>() != null ||
                    child.GetComponentInChildren<TMP_Text>(true) != null ||
                    child.GetComponent<LayoutGroup>() != null ||
                    child.GetComponent<ScrollRect>() != null)
                {
                    UnityEngine.Object.Destroy(child.gameObject);
                }
            }

            TMP_FontAsset font = ui.fovValText != null ? ui.fovValText.font : (tabLabel != null ? tabLabel.font : null);
            Material fontMat = ui.fovValText != null ? ui.fovValText.fontSharedMaterial : (tabLabel != null ? tabLabel.fontSharedMaterial : null);

            Transform subTabRow = CreateTopSubTabBar(panelGo.transform);
            TMP_Text liveBanner = CreateLiveBanner(panelGo.transform, font, fontMat);
            LiveBannerUpdater updater = panelGo.AddComponent<LiveBannerUpdater>();
            updater.BannerText = liveBanner;

            Transform contentParent = CreateScrollContainer(panelGo.transform, out ScrollRect scrollRect);
            state.Scroll = scrollRect;

            Transform catGeneral = CreateCategoryContainer(ui, state, subTabRow, contentParent, font, fontMat, 0, T("Общее", "General"));
            Transform catGpu = CreateCategoryContainer(ui, state, subTabRow, contentParent, font, fontMat, 1, T("Графика", "Quality"));
            Transform catWorld = CreateCategoryContainer(ui, state, subTabRow, contentParent, font, fontMat, 2, T("Мир/Трава", "World"));
            Transform catSlimes = CreateCategoryContainer(ui, state, subTabRow, contentParent, font, fontMat, 3, T("Слаймы", "Slimes"));
            Transform catEngine = CreateCategoryContainer(ui, state, subTabRow, contentParent, font, fontMat, 4, T("Движок", "Engine"));

            // --- Category 0: General & Presets ---
            AddSectionHeader(catGeneral, font, fontMat, T("Пресеты производительности (Sodium Style)", "Performance Presets"));
            AddPresetsRow(ui, state, catGeneral, font, fontMat);

            AddSectionHeader(catGeneral, font, fontMat, T("Экран и Лимит кадров", "Display & Frame Rate"));
            AddToggle(ui, state, catGeneral, font, fontMat,
                T("Показывать счётчик FPS и ms сверху (HUD)", "Show FPS / Frametime HUD Overlay"),
                () => OptimizerConfig.ShowFpsHud,
                v => OptimizerConfig.ShowFpsHud = v);
            AddIntSlider(ui, state, catGeneral, font, fontMat,
                T("Лимит FPS (если выключен VSync)", "Max Framerate Limit (VSync Off)"),
                0, FpsSteps.Length - 1,
                () => FpsToStep(OptimizerConfig.TargetFrameRate),
                s => { OptimizerConfig.TargetFrameRate = StepToFps(s); QualityCapsPatch.ApplyTargetFrameRate(); },
                s => { int f = StepToFps(s); return f <= 0 ? T("Без лимита", "Unlimited") : f + " FPS"; });

            AddResetStatsRow(ui, state, catGeneral, font, fontMat);

            state.InfoText = CreateText(catGeneral, "InfoLabel", font, fontMat, 14f, FontStyles.Normal,
                new Color(0.8f, 0.88f, 0.96f, 0.9f), TextAlignmentOptions.TopLeft);
            state.InfoText.enableWordWrapping = true;
            LayoutElement infoLe = state.InfoText.gameObject.AddComponent<LayoutElement>();
            infoLe.minHeight = 74f;

            // --- Category 1: GPU, Shadows, SSAO, Bloom, Particles ---
            AddSectionHeader(catGpu, font, fontMat, T("Тени, Свет, Сглаживание и Шейдеры", "Shadows, Lighting, AA & Shaders"));
            AddToggle(ui, state, catGpu, font, fontMat,
                T("Умные лимиты графики (тени / MSAA / свет)", "Smart GPU Quality Caps"),
                () => OptimizerConfig.EnableSmartQualityCaps,
                v => { OptimizerConfig.EnableSmartQualityCaps = v; SettingsDatabase.ApplyQualityCapsOnly(); });
            AddIntSlider(ui, state, catGpu, font, fontMat,
                T("Каскады теней (0=Выкл, 1=2 каск., 2=4 каск.)", "Max Shadow Cascades"),
                0, 2,
                () => CascadesToStep(OptimizerConfig.MaxShadowCascades),
                s => { OptimizerConfig.MaxShadowCascades = StepToCascades(s); SettingsDatabase.ApplyQualityCapsOnly(); },
                s => StepToCascades(s).ToString(CultureInfo.InvariantCulture));
            AddIntSlider(ui, state, catGpu, font, fontMat,
                T("Разрешение теней (0=Низк .. 3=Ультра)", "Max Shadow Resolution"),
                0, 3,
                () => OptimizerConfig.MaxShadowResolution,
                v => { OptimizerConfig.MaxShadowResolution = v; SettingsDatabase.ApplyQualityCapsOnly(); },
                FormatShadowRes);
            AddFloatSlider(ui, state, catGpu, font, fontMat,
                T("Дальность теней (м)", "Max Shadow Distance"),
                20f, 150f, true,
                () => OptimizerConfig.MaxShadowDistance,
                v => { OptimizerConfig.MaxShadowDistance = v; SettingsDatabase.ApplyQualityCapsOnly(); },
                v => Mathf.RoundToInt(v) + " m");
            AddIntSlider(ui, state, catGpu, font, fontMat,
                T("Сглаживание MSAA (0=Выкл, 1=2x, 2=4x, 3=8x)", "Max MSAA"),
                0, 3,
                () => AaToStep(OptimizerConfig.MaxAntiAliasing),
                s => { OptimizerConfig.MaxAntiAliasing = StepToAa(s); SettingsDatabase.ApplyQualityCapsOnly(); },
                s => { int aa = StepToAa(s); return aa == 0 ? T("Выкл", "Off") : aa + "x"; });
            AddIntSlider(ui, state, catGpu, font, fontMat,
                T("Пиксельных источников света", "Max Pixel Light Count"),
                0, 4,
                () => OptimizerConfig.MaxPixelLightCount,
                v => { OptimizerConfig.MaxPixelLightCount = v; SettingsDatabase.ApplyQualityCapsOnly(); },
                v => v.ToString(CultureInfo.InvariantCulture));
            AddFloatSlider(ui, state, catGpu, font, fontMat,
                T("Множитель дальности LOD моделей", "Max LOD Bias"),
                0.5f, 2.0f, false,
                () => OptimizerConfig.MaxLodBias,
                v => { OptimizerConfig.MaxLodBias = (float)Math.Round(v, 2); SettingsDatabase.ApplyQualityCapsOnly(); },
                v => v.ToString("0.00", CultureInfo.InvariantCulture));
            AddIntSlider(ui, state, catGpu, font, fontMat,
                T("Лимит шейдеров мира (3=Макс, 0=400, 1=700, 2=900)", "Global Shader LOD Cap"),
                0, 3,
                () => ShaderLodToStep(OptimizerConfig.MaxGlobalShaderLod),
                s => { OptimizerConfig.MaxGlobalShaderLod = StepToShaderLod(s); SettingsDatabase.ApplyQualityCapsOnly(); },
                s => { int l = StepToShaderLod(s); return l < 0 ? T("Макс", "Auto") : l.ToString(CultureInfo.InvariantCulture); });
            AddToggle(ui, state, catGpu, font, fontMat,
                T("Отключить мягкие частицы (SoftParticles)", "Disable Soft Particles"),
                () => OptimizerConfig.DisableSoftParticles,
                v => { OptimizerConfig.DisableSoftParticles = v; SettingsDatabase.ApplyQualityCapsOnly(); });
            AddIntSlider(ui, state, catGpu, font, fontMat,
                T("Анизотропная фильтрация (0=Выкл, 1=Авто, 2=Макс)", "Max Anisotropic Filtering"),
                0, 2,
                () => OptimizerConfig.MaxAnisoMode,
                v => { OptimizerConfig.MaxAnisoMode = v; SettingsDatabase.ApplyQualityCapsOnly(); },
                FormatAniso);
            AddIntSlider(ui, state, catGpu, font, fontMat,
                T("Бюджет коллизий частиц (лучи)", "Particle Raycast Budget"),
                16, 1024,
                () => OptimizerConfig.MaxParticleRaycastBudget,
                v => { OptimizerConfig.MaxParticleRaycastBudget = v; SettingsDatabase.ApplyQualityCapsOnly(); },
                v => v.ToString(CultureInfo.InvariantCulture));

            AddSectionHeader(catGpu, font, fontMat, T("Постобработка и Частицы (SSAO, Bloom, FX)", "Post-Processing & Particles"));
            AddToggle(ui, state, catGpu, font, fontMat,
                T("Оптимизатор SSAO (ускорение в 4x + без утечек)", "Fast SSAO Optimizer"),
                () => OptimizerConfig.EnableSsaoOptimizer,
                v => OptimizerConfig.EnableSsaoOptimizer = v);
            AddIntSlider(ui, state, catGpu, font, fontMat,
                T("Делитель разрешения SSAO (1=Натив, 2=Половина)", "SSAO Downsampling"),
                1, 4,
                () => OptimizerConfig.SsaoDownsampling,
                v => OptimizerConfig.SsaoDownsampling = v,
                v => v + "x");
            AddToggle(ui, state, catGpu, font, fontMat,
                T("Уменьшать разрешение блюра SSAO", "SSAO Blur Downsampling"),
                () => OptimizerConfig.SsaoBlurDownsampling,
                v => OptimizerConfig.SsaoBlurDownsampling = v);
            AddToggle(ui, state, catGpu, font, fontMat,
                T("Быстрый Gaussian блюр SSAO", "Fast SSAO Gaussian Blur"),
                () => OptimizerConfig.SsaoUseGaussianBlur,
                v => OptimizerConfig.SsaoUseGaussianBlur = v);
            AddToggle(ui, state, catGpu, font, fontMat,
                T("Ускорение Bloom (1 проход / Low Res)", "Fast Bloom Optimizer"),
                () => OptimizerConfig.EnableBloomOptimizer,
                v => OptimizerConfig.EnableBloomOptimizer = v);
            AddToggle(ui, state, catGpu, font, fontMat,
                T("Откл. фоновый GIF-рекордер (экономит GPU)", "Disable Background GIF Recorder"),
                () => OptimizerConfig.DisableGifRecorder,
                v => OptimizerConfig.DisableGifRecorder = v);
            AddToggle(ui, state, catGpu, font, fontMat,
                T("Откл. Realtime Reflection Probes", "Disable Realtime Reflection Probes"),
                () => OptimizerConfig.DisableRealtimeReflections,
                v => { OptimizerConfig.DisableRealtimeReflections = v; SettingsDatabase.ApplyQualityCapsOnly(); });
            AddToggle(ui, state, catGpu, font, fontMat,
                T("Убрать Depth-проход (х2 рендер) и StylizedFog", "Disable Depth Prepass & StylizedFog"),
                () => OptimizerConfig.DisableStylizedFogAndDepth,
                v => { OptimizerConfig.DisableStylizedFogAndDepth = v; RenderExtraPatches.SyncCameraDepthAndFog(); });
            AddToggle(ui, state, catGpu, font, fontMat,
                T("Откл. теневой проход SECTR при выкл. тенях", "Disable SECTR Shadow Cull Pass"),
                () => OptimizerConfig.DisableSectrShadowCulling,
                v => OptimizerConfig.DisableSectrShadowCulling = v);
            AddToggle(ui, state, catGpu, font, fontMat,
                T("Лимит частиц FX (защита min FPS в замесах)", "FX Particle Count Cap"),
                () => OptimizerConfig.EnableParticleCap,
                v => OptimizerConfig.EnableParticleCap = v);
            AddIntSlider(ui, state, catGpu, font, fontMat,
                T("Макс. частиц на эффект", "Max Particles Per FX"),
                16, 250,
                () => OptimizerConfig.MaxParticlesPerSystem,
                v => OptimizerConfig.MaxParticlesPerSystem = v,
                v => v.ToString(CultureInfo.InvariantCulture));

            // --- Category 2: World, Grass, Camera ---
            AddSectionHeader(catWorld, font, fontMat, T("Густая трава и Растительность", "Dense Grass & Foliage"));
            AddToggle(ui, state, catWorld, font, fontMat,
                T("Оптимизатор густой травы", "Grass Optimizer"),
                () => OptimizerConfig.EnableGrassOptimizer,
                v => { OptimizerConfig.EnableGrassOptimizer = v; GrassOptimizerPatch.ReapplyAllScenes(); });
            AddToggle(ui, state, catWorld, font, fontMat,
                T("Отключить тени от травы", "Disable Grass Shadows"),
                () => OptimizerConfig.DisableGrassShadows,
                v => { OptimizerConfig.DisableGrassShadows = v; GrassOptimizerPatch.ReapplyAllScenes(); });
            AddToggle(ui, state, catWorld, font, fontMat,
                T("Отключить тяжёлый ветер в шейдере травы", "Disable Heavy Grass Wind Shader"),
                () => OptimizerConfig.DisableGrassWindShader,
                v => { OptimizerConfig.DisableGrassWindShader = v; GrassOptimizerPatch.ReapplyAllScenes(); });
            AddIntSlider(ui, state, catWorld, font, fontMat,
                T("LOD шейдеров травы (меньше = быстрее)", "Grass Shader Max LOD"),
                100, 400,
                () => OptimizerConfig.GrassShaderMaxLod,
                v => { OptimizerConfig.GrassShaderMaxLod = (v / 100) * 100; GrassOptimizerPatch.ReapplyAllScenes(); },
                v => ((v / 100) * 100).ToString(CultureInfo.InvariantCulture));
            AddToggle(ui, state, catWorld, font, fontMat,
                T("Оптимизация рельефа (Terrain detail/error)", "Optimize Terrain Detail & Error"),
                () => OptimizerConfig.OptimizeTerrain,
                v => { OptimizerConfig.OptimizeTerrain = v; TerrainOptPatch.ApplyToAllTerrains(); });
            AddFloatSlider(ui, state, catWorld, font, fontMat,
                T("Сдвиг LOD травы", "Grass LOD Multiplier"),
                1.0f, 2.5f, false,
                () => OptimizerConfig.GrassLodMultiplier,
                v => { OptimizerConfig.GrassLodMultiplier = (float)Math.Round(v, 2); GrassOptimizerPatch.ReapplyAllScenes(); },
                v => v.ToString("0.00", CultureInfo.InvariantCulture) + "x");

            AddSectionHeader(catWorld, font, fontMat, T("Камера и Дальность объектов", "Camera & Object Culling"));
            AddToggle(ui, state, catWorld, font, fontMat,
                T("Послойный куллинг камеры (дальние акторы)", "Camera Layer Culling"),
                () => OptimizerConfig.EnableCameraLayerCull,
                v => { OptimizerConfig.EnableCameraLayerCull = v; CameraCullDistancePatch.MarkDirty(); });
            AddFloatSlider(ui, state, catWorld, font, fontMat,
                T("Дальность мелких акторов (м)", "Actor Layer Cull Distance"),
                40f, 200f, true,
                () => OptimizerConfig.LayerActorCullDistance,
                v => { OptimizerConfig.LayerActorCullDistance = v; CameraCullDistancePatch.MarkDirty(); },
                v => Mathf.RoundToInt(v) + " m");
            AddFloatSlider(ui, state, catWorld, font, fontMat,
                T("Макс. дальность прорисовки мира (м)", "Max Camera Far Clip"),
                200f, 1000f, true,
                () => OptimizerConfig.MaxFarClipPlane,
                v => { OptimizerConfig.MaxFarClipPlane = v; CameraCullDistancePatch.MarkDirty(); },
                v => Mathf.RoundToInt(v) + " m");
            AddToggle(ui, state, catWorld, font, fontMat,
                T("Убрать тени у плортов, еды и цыплят", "Cull Small Actor Shadows"),
                () => OptimizerConfig.EnableSmallObjectShadowCull,
                v => OptimizerConfig.EnableSmallObjectShadowCull = v);
            AddToggle(ui, state, catWorld, font, fontMat,
                T("Куллинг аниматоров за спиной камеры", "Animator Frustum Culling"),
                () => OptimizerConfig.EnableAnimatorCulling,
                v => { OptimizerConfig.EnableAnimatorCulling = v; AnimatorCullingPatch.ReapplyAll(); });

            // --- Category 3: Slimes & Physics ---
            AddSectionHeader(catSlimes, font, fontMat, T("Слаймы: Рендер и Желе-физика", "Slime Render & RubberBone LOD"));
            AddToggle(ui, state, catSlimes, font, fontMat,
                T("LOD скиннинга и теней слаймов", "Slime Render LOD"),
                () => OptimizerConfig.EnableSlimeRenderLod,
                v => OptimizerConfig.EnableSlimeRenderLod = v);
            AddFloatSlider(ui, state, catSlimes, font, fontMat,
                T("1-костный скиннинг слаймов с (м)", "Slime 1-Bone Skin Distance"),
                8f, 40f, true,
                () => OptimizerConfig.SlimeSkinLodDistance,
                v => OptimizerConfig.SlimeSkinLodDistance = v,
                v => Mathf.RoundToInt(v) + " m");
            AddFloatSlider(ui, state, catSlimes, font, fontMat,
                T("Убирать тени слаймов с (м)", "Slime Shadow Cull Distance"),
                10f, 60f, true,
                () => OptimizerConfig.SlimeShadowCullDistance,
                v => OptimizerConfig.SlimeShadowCullDistance = v,
                v => Mathf.RoundToInt(v) + " m");
            AddToggle(ui, state, catSlimes, font, fontMat,
                T("LOD желе-физики костей (RubberBone)", "RubberBone Jiggle LOD"),
                () => OptimizerConfig.EnableRubberBoneLod,
                v => OptimizerConfig.EnableRubberBoneLod = v);
            AddFloatSlider(ui, state, catSlimes, font, fontMat,
                T("Замедлять желе-физику с (м)", "RubberBone LOD Distance"),
                4f, 25f, true,
                () => OptimizerConfig.RubberLodDistance,
                v => OptimizerConfig.RubberLodDistance = v,
                v => Mathf.RoundToInt(v) + " m");
            AddFloatSlider(ui, state, catSlimes, font, fontMat,
                T("Усыплять желе-физику с (м)", "RubberBone Sleep Distance"),
                10f, 50f, true,
                () => OptimizerConfig.RubberDisableDistance,
                v => OptimizerConfig.RubberDisableDistance = v,
                v => Mathf.RoundToInt(v) + " m");

            AddSectionHeader(catSlimes, font, fontMat, T("ИИ, Физика загонов и Курицы", "Slime AI, Ranch Physics & Chickens"));
            AddToggle(ui, state, catSlimes, font, fontMat,
                T("LOD ИИ и рейкастов дальних слаймов", "Slime AI Distance LOD"),
                () => OptimizerConfig.EnableSlimeAiLod,
                v => OptimizerConfig.EnableSlimeAiLod = v);
            AddFloatSlider(ui, state, catSlimes, font, fontMat,
                T("Дистанция замедления ИИ (м)", "Far AI Distance"),
                12f, 50f, true,
                () => OptimizerConfig.FarAiDistance,
                v => OptimizerConfig.FarAiDistance = v,
                v => Mathf.RoundToInt(v) + " m");
            AddToggle(ui, state, catSlimes, font, fontMat,
                T("LOD анимации лиц и эмоций слаймов", "Slime Face & Emotions LOD"),
                () => OptimizerConfig.EnableSlimeFaceEmotionLod,
                v => OptimizerConfig.EnableSlimeFaceEmotionLod = v);
            AddToggle(ui, state, catSlimes, font, fontMat,
                T("Оптимизация физики загонов (PhysX Solver)", "Optimize Ranch PhysX Solver"),
                () => OptimizerConfig.EnablePhysicsOpt,
                v => { OptimizerConfig.EnablePhysicsOpt = v; QualityCapsPatch.ApplyPhysicsSettings(); });
            AddIntSlider(ui, state, catSlimes, font, fontMat,
                T("Итерации солвера физики (меньше = быстрее)", "PhysX Solver Iterations"),
                2, 6,
                () => OptimizerConfig.PhysicsSolverIterations,
                v => { OptimizerConfig.PhysicsSolverIterations = v; QualityCapsPatch.ApplyPhysicsSettings(); },
                v => v.ToString(CultureInfo.InvariantCulture));
            AddIntSlider(ui, state, catSlimes, font, fontMat,
                T("Частота тиков физики (50=ваниль, 33=быстро, 25=макс)", "Physics Tick Rate (Hz)"),
                25, 50,
                () => OptimizerConfig.PhysicsTickRate,
                v => { OptimizerConfig.PhysicsTickRate = v; QualityCapsPatch.ApplyPhysicsSettings(); },
                v => v + " Hz");
            AddToggle(ui, state, catSlimes, font, fontMat,
                T("LOD деформации мешей (RubberEffect)", "RubberEffect Mesh LOD"),
                () => OptimizerConfig.EnableRubberEffectLod,
                v => OptimizerConfig.EnableRubberEffectLod = v);
            AddToggle(ui, state, catSlimes, font, fontMat,
                T("Кэш трения слаймов (SlimeDragUpdater)", "Optimize Slime Drag Updates"),
                () => OptimizerConfig.EnableSlimeDragOpt,
                v => OptimizerConfig.EnableSlimeDragOpt = v);
            AddToggle(ui, state, catSlimes, font, fontMat,
                T("Усыплять KeepUpright у спящих/дальних тел", "Skip Sleeping/Far KeepUpright"),
                () => OptimizerConfig.EnableKeepUprightOpt,
                v => OptimizerConfig.EnableKeepUprightOpt = v);
            AddToggle(ui, state, catSlimes, font, fontMat,
                T("LOD ИИ дальних куриц (ChickenRandomMove)", "Distant Chicken AI LOD"),
                () => OptimizerConfig.EnableChickenAiLod,
                v => OptimizerConfig.EnableChickenAiLod = v);

            // --- Category 4: Engine, Timers & GC ---
            AddSectionHeader(catEngine, font, fontMat, T("Таймеры, Кэши и Память (GC)", "Timers, Caches & Memory (GC)"));
            AddToggle(ui, state, catEngine, font, fontMat,
                T("LOD таймера созревания (ResourceCycle)", "ResourceCycle Timer LOD"),
                () => OptimizerConfig.EnableRanchTimersLod,
                v => OptimizerConfig.EnableRanchTimersLod = v);
            AddIntSlider(ui, state, catEngine, font, fontMat,
                T("Интервал фоновых таймеров (кадры)", "Ranch Timers Interval (frames)"),
                2, 20,
                () => OptimizerConfig.RanchTimersLodInterval,
                v => OptimizerConfig.RanchTimersLodInterval = v,
                v => v.ToString(CultureInfo.InvariantCulture));
            AddToggle(ui, state, catEngine, font, fontMat,
                T("Быстрый поиск еды без GetComponent", "Fast Food Search"),
                () => OptimizerConfig.EnableFastFoodSearch,
                v => OptimizerConfig.EnableFastFoodSearch = v);
            AddToggle(ui, state, catEngine, font, fontMat,
                T("Кэш массивов материалов лица слайма", "Expression Material[] Cache"),
                () => OptimizerConfig.EnableExpressionCache,
                v => OptimizerConfig.EnableExpressionCache = v);
            AddToggle(ui, state, catEngine, font, fontMat,
                T("Пул списков сборщика плортов", "PlortCollector List Pool"),
                () => OptimizerConfig.EnablePlortCollectorPool,
                v => OptimizerConfig.EnablePlortCollectorPool = v);
            AddToggle(ui, state, catEngine, font, fontMat,
                T("Быстрый подсчёт в CellDirector", "CellDirector Fast Count"),
                () => OptimizerConfig.EnableCellDirectorFastCount,
                v => OptimizerConfig.EnableCellDirectorFastCount = v);
            AddToggle(ui, state, catEngine, font, fontMat,
                T("Пул коллайдеров TrackCollisions", "TrackCollisions List Pool"),
                () => OptimizerConfig.EnableTrackCollisionsPool,
                v => OptimizerConfig.EnableTrackCollisionsPool = v);
            AddToggle(ui, state, catEngine, font, fontMat,
                T("Кэш Camera.main для билбордов", "Billboard Camera Cache"),
                () => OptimizerConfig.EnableBillboardCameraCache,
                v => OptimizerConfig.EnableBillboardCameraCache = v);
            AddToggle(ui, state, catEngine, font, fontMat,
                T("Кэш компонентов в DisableEffects", "Effects Component Cache"),
                () => OptimizerConfig.EnableEffectsComponentCache,
                v => OptimizerConfig.EnableEffectsComponentCache = v);
            AddToggle(ui, state, catEngine, font, fontMat,
                T("Кэш делегатов рейкаста Plexer", "Plexer Delegate Cache"),
                () => OptimizerConfig.EnablePlexerDelegateCache,
                v => OptimizerConfig.EnablePlexerDelegateCache = v);
            AddToggle(ui, state, catEngine, font, fontMat,
                T("Сбор статистики (slimeopt stats)", "Collect Stats Counters"),
                () => OptimizerConfig.EnableStatsCollection,
                v => OptimizerConfig.EnableStatsCollection = v);

            state.TabObject = tabGo;
            state.PanelObject = panelGo;
            SelectSubCategory(state, 0);
            RefreshState(state);
        }

        private static Transform CreateTopSubTabBar(Transform panelRoot)
        {
            GameObject bar = new GameObject("SodiumSubTabs", typeof(RectTransform), typeof(HorizontalLayoutGroup));
            bar.transform.SetParent(panelRoot, false);
            RectTransform rt = bar.GetComponent<RectTransform>();
            rt.anchorMin = new Vector2(0f, 1f);
            rt.anchorMax = new Vector2(1f, 1f);
            rt.pivot = new Vector2(0.5f, 1f);
            rt.anchoredPosition = new Vector2(0f, -8f);
            rt.sizeDelta = new Vector2(-20f, 34f);

            HorizontalLayoutGroup hlg = bar.GetComponent<HorizontalLayoutGroup>();
            hlg.spacing = 6f;
            hlg.childControlWidth = true;
            hlg.childControlHeight = true;
            hlg.childForceExpandWidth = true;
            hlg.childForceExpandHeight = true;

            return bar.transform;
        }

        private static TMP_Text CreateLiveBanner(Transform panelRoot, TMP_FontAsset font, Material fontMat)
        {
            GameObject banner = new GameObject("SodiumLiveBanner", typeof(RectTransform), typeof(Image));
            banner.transform.SetParent(panelRoot, false);
            RectTransform rt = banner.GetComponent<RectTransform>();
            rt.anchorMin = new Vector2(0f, 1f);
            rt.anchorMax = new Vector2(1f, 1f);
            rt.pivot = new Vector2(0.5f, 1f);
            rt.anchoredPosition = new Vector2(0f, -46f);
            rt.sizeDelta = new Vector2(-20f, 24f);
            banner.GetComponent<Image>().color = new Color(0.04f, 0.07f, 0.12f, 0.72f);

            TMP_Text txt = CreateText(banner.transform, "Text", font, fontMat, 14f, FontStyles.Bold,
                new Color(0.65f, 0.94f, 0.75f), TextAlignmentOptions.Center);
            RectTransform trt = txt.GetComponent<RectTransform>();
            trt.anchorMin = Vector2.zero;
            trt.anchorMax = Vector2.one;
            trt.offsetMin = new Vector2(6f, 0f);
            trt.offsetMax = new Vector2(-6f, 0f);
            return txt;
        }

        private static Transform CreateCategoryContainer(
            OptionsUI ui,
            AdvancedUiState state,
            Transform subTabBar,
            Transform contentParent,
            TMP_FontAsset font,
            Material fontMat,
            int index,
            string title)
        {
            GameObject btnGo = null;
            if (ui.resetProfileButton != null)
            {
                btnGo = UnityEngine.Object.Instantiate(ui.resetProfileButton.gameObject, subTabBar, false);
                StripXlateText(btnGo);
            }
            else
            {
                btnGo = new GameObject("SubTab_" + index, typeof(RectTransform), typeof(Image), typeof(Button));
                btnGo.transform.SetParent(subTabBar, false);
                CreateText(btnGo.transform, "Text", font, fontMat, 15f, FontStyles.Bold, Color.white, TextAlignmentOptions.Center);
            }

            btnGo.SetActive(true);
            LayoutElement le = btnGo.GetComponent<LayoutElement>() ?? btnGo.AddComponent<LayoutElement>();
            le.flexibleWidth = 1f;
            le.minHeight = 32f;
            le.preferredHeight = 32f;

            Image img = btnGo.GetComponent<Image>();
            state.SubTabImages.Add(img);

            TMP_Text txt = btnGo.GetComponentInChildren<TMP_Text>(true);
            if (txt != null)
            {
                if (font != null) txt.font = font;
                txt.text = title;
                txt.fontSize = 15f;
                txt.enableAutoSizing = false;
            }

            Button btn = btnGo.GetComponent<Button>();
            if (btn != null)
            {
                btn.onClick = new Button.ButtonClickedEvent();
                int captured = index;
                btn.onClick.AddListener(delegate { SelectSubCategory(state, captured); });
            }

            GameObject container = new GameObject("Category_" + index, typeof(RectTransform), typeof(VerticalLayoutGroup), typeof(LayoutElement));
            container.transform.SetParent(contentParent, false);
            VerticalLayoutGroup vlg = container.GetComponent<VerticalLayoutGroup>();
            vlg.spacing = 6f;
            vlg.childAlignment = TextAnchor.UpperLeft;
            vlg.childControlWidth = true;
            vlg.childControlHeight = true;
            vlg.childForceExpandWidth = true;
            vlg.childForceExpandHeight = false;

            state.CategoryContainers.Add(container);
            return container.transform;
        }

        private static Transform CreateScrollContainer(Transform panelRoot, out ScrollRect scrollRect)
        {
            GameObject scrollGo = new GameObject("SodiumScrollRect", typeof(RectTransform), typeof(Image), typeof(ScrollRect));
            scrollGo.transform.SetParent(panelRoot, false);
            RectTransform scrollRt = scrollGo.GetComponent<RectTransform>();
            scrollRt.anchorMin = Vector2.zero;
            scrollRt.anchorMax = Vector2.one;
            scrollRt.offsetMin = new Vector2(10f, 10f);
            scrollRt.offsetMax = new Vector2(-10f, -74f);

            Image bg = scrollGo.GetComponent<Image>();
            bg.color = new Color(0.05f, 0.08f, 0.14f, 0.45f);

            GameObject viewportGo = new GameObject("Viewport", typeof(RectTransform), typeof(RectMask2D));
            viewportGo.transform.SetParent(scrollGo.transform, false);
            RectTransform viewportRt = viewportGo.GetComponent<RectTransform>();
            viewportRt.anchorMin = Vector2.zero;
            viewportRt.anchorMax = Vector2.one;
            viewportRt.offsetMin = new Vector2(4f, 4f);
            viewportRt.offsetMax = new Vector2(-20f, -4f);

            GameObject contentGo = new GameObject("Content", typeof(RectTransform), typeof(VerticalLayoutGroup), typeof(ContentSizeFitter));
            contentGo.transform.SetParent(viewportGo.transform, false);
            RectTransform contentRt = contentGo.GetComponent<RectTransform>();
            contentRt.anchorMin = new Vector2(0f, 1f);
            contentRt.anchorMax = new Vector2(1f, 1f);
            contentRt.pivot = new Vector2(0.5f, 1f);
            contentRt.anchoredPosition = Vector2.zero;
            contentRt.sizeDelta = new Vector2(0f, 0f);

            VerticalLayoutGroup vlg = contentGo.GetComponent<VerticalLayoutGroup>();
            vlg.padding = new RectOffset(10, 10, 8, 12);
            vlg.spacing = 6f;
            vlg.childAlignment = TextAnchor.UpperLeft;
            vlg.childControlWidth = true;
            vlg.childControlHeight = true;
            vlg.childForceExpandWidth = true;
            vlg.childForceExpandHeight = false;

            ContentSizeFitter csf = contentGo.GetComponent<ContentSizeFitter>();
            csf.horizontalFit = ContentSizeFitter.FitMode.Unconstrained;
            csf.verticalFit = ContentSizeFitter.FitMode.PreferredSize;

            GameObject sbGo = new GameObject("Scrollbar", typeof(RectTransform), typeof(Image), typeof(Scrollbar));
            sbGo.transform.SetParent(scrollGo.transform, false);
            RectTransform sbRt = sbGo.GetComponent<RectTransform>();
            sbRt.anchorMin = new Vector2(1f, 0f);
            sbRt.anchorMax = new Vector2(1f, 1f);
            sbRt.pivot = new Vector2(1f, 0.5f);
            sbRt.offsetMin = new Vector2(-16f, 6f);
            sbRt.offsetMax = new Vector2(-4f, -6f);
            sbGo.GetComponent<Image>().color = new Color(0f, 0f, 0f, 0.35f);

            GameObject handleAreaGo = new GameObject("SlidingArea", typeof(RectTransform));
            handleAreaGo.transform.SetParent(sbGo.transform, false);
            RectTransform handleAreaRt = handleAreaGo.GetComponent<RectTransform>();
            handleAreaRt.anchorMin = Vector2.zero;
            handleAreaRt.anchorMax = Vector2.one;
            handleAreaRt.offsetMin = Vector2.zero;
            handleAreaRt.offsetMax = Vector2.zero;

            GameObject handleGo = new GameObject("Handle", typeof(RectTransform), typeof(Image));
            handleGo.transform.SetParent(handleAreaGo.transform, false);
            RectTransform handleRt = handleGo.GetComponent<RectTransform>();
            handleRt.anchorMin = Vector2.zero;
            handleRt.anchorMax = Vector2.one;
            handleRt.offsetMin = Vector2.zero;
            handleRt.offsetMax = Vector2.zero;
            Image handleImg = handleGo.GetComponent<Image>();
            handleImg.color = new Color(1f, 0.78f, 0.35f, 0.8f);

            Scrollbar sb = sbGo.GetComponent<Scrollbar>();
            sb.handleRect = handleRt;
            sb.targetGraphic = handleImg;
            sb.direction = Scrollbar.Direction.BottomToTop;

            scrollRect = scrollGo.GetComponent<ScrollRect>();
            scrollRect.viewport = viewportRt;
            scrollRect.content = contentRt;
            scrollRect.horizontal = false;
            scrollRect.vertical = true;
            scrollRect.movementType = ScrollRect.MovementType.Clamped;
            scrollRect.scrollSensitivity = 36f;
            scrollRect.verticalScrollbar = sb;
            scrollRect.verticalScrollbarVisibility = ScrollRect.ScrollbarVisibility.Permanent;

            return contentGo.transform;
        }

        private static void AddPresetsRow(OptionsUI ui, AdvancedUiState state, Transform parent, TMP_FontAsset font, Material fontMat)
        {
            GameObject row = new GameObject("PresetsRow", typeof(RectTransform), typeof(HorizontalLayoutGroup), typeof(LayoutElement));
            row.transform.SetParent(parent, false);
            HorizontalLayoutGroup hlg = row.GetComponent<HorizontalLayoutGroup>();
            hlg.spacing = 8f;
            hlg.childControlWidth = true;
            hlg.childControlHeight = true;
            hlg.childForceExpandWidth = true;
            hlg.childForceExpandHeight = true;
            row.GetComponent<LayoutElement>().minHeight = 38f;
            row.GetComponent<LayoutElement>().preferredHeight = 38f;

            CreateActionButton(ui, row.transform, font, fontMat, T("Макс. FPS", "Max FPS"), new Color(0.22f, 0.65f, 0.38f), () =>
            {
                SettingsDatabase.ApplyPreset(OptimizerPreset.MaxFps);
                NotifyQualityChangedMethod?.Invoke(ui, null);
                RefreshState(state);
            });
            CreateActionButton(ui, row.transform, font, fontMat, T("Баланс", "Balanced"), new Color(0.22f, 0.52f, 0.78f), () =>
            {
                SettingsDatabase.ApplyPreset(OptimizerPreset.Balanced);
                NotifyQualityChangedMethod?.Invoke(ui, null);
                RefreshState(state);
            });
            CreateActionButton(ui, row.transform, font, fontMat, T("Качество", "High Quality"), new Color(0.68f, 0.50f, 0.20f), () =>
            {
                SettingsDatabase.ApplyPreset(OptimizerPreset.HighQuality);
                NotifyQualityChangedMethod?.Invoke(ui, null);
                RefreshState(state);
            });
            CreateActionButton(ui, row.transform, font, fontMat, T("Ваниль (Выкл.)", "Vanilla (Off)"), new Color(0.62f, 0.28f, 0.28f), () =>
            {
                SettingsDatabase.ApplyPreset(OptimizerPreset.VanillaOff);
                NotifyQualityChangedMethod?.Invoke(ui, null);
                RefreshState(state);
            });
        }

        private static void AddResetStatsRow(OptionsUI ui, AdvancedUiState state, Transform parent, TMP_FontAsset font, Material fontMat)
        {
            GameObject row = new GameObject("ActionsRow", typeof(RectTransform), typeof(HorizontalLayoutGroup), typeof(LayoutElement));
            row.transform.SetParent(parent, false);
            HorizontalLayoutGroup hlg = row.GetComponent<HorizontalLayoutGroup>();
            hlg.spacing = 8f;
            hlg.childControlWidth = true;
            hlg.childControlHeight = true;
            hlg.childForceExpandWidth = true;
            hlg.childForceExpandHeight = true;
            row.GetComponent<LayoutElement>().minHeight = 36f;
            row.GetComponent<LayoutElement>().preferredHeight = 36f;

            CreateActionButton(ui, row.transform, font, fontMat, T("Диагностика FPS (3с)", "Run 3s Profiler"), new Color(0.70f, 0.42f, 0.18f), () =>
            {
                FrameProfiler.Start(3.0f);
                RefreshState(state);
            });
            CreateActionButton(ui, row.transform, font, fontMat, T("Сбросить настройки по умолчанию", "Reset To Defaults"), new Color(0.28f, 0.45f, 0.66f), () =>
            {
                SettingsDatabase.ApplyPreset(OptimizerPreset.Balanced);
                NotifyQualityChangedMethod?.Invoke(ui, null);
                RefreshState(state);
            });
            CreateActionButton(ui, row.transform, font, fontMat, T("Сбросить счётчики статистики", "Reset Stats Counters"), new Color(0.45f, 0.45f, 0.52f), () =>
            {
                OptimizerStats.Reset();
                RefreshState(state);
            });
        }

        private static void CreateActionButton(OptionsUI ui, Transform parent, TMP_FontAsset font, Material fontMat, string label, Color tint, Action onClick)
        {
            GameObject btnGo = null;
            if (ui.resetProfileButton != null)
            {
                btnGo = UnityEngine.Object.Instantiate(ui.resetProfileButton.gameObject, parent, false);
                StripXlateText(btnGo);
            }
            else
            {
                btnGo = new GameObject("Button", typeof(RectTransform), typeof(Image), typeof(Button));
                btnGo.transform.SetParent(parent, false);
                CreateText(btnGo.transform, "Text", font, fontMat, 18, FontStyles.Bold, Color.white, TextAlignmentOptions.Center);
            }

            btnGo.SetActive(true);
            LayoutElement le = btnGo.GetComponent<LayoutElement>() ?? btnGo.AddComponent<LayoutElement>();
            le.flexibleWidth = 1f;
            le.minHeight = 36f;
            le.preferredHeight = 36f;

            Image img = btnGo.GetComponent<Image>();
            if (img != null)
            {
                img.color = tint;
            }

            TMP_Text txt = btnGo.GetComponentInChildren<TMP_Text>(true);
            if (txt != null)
            {
                if (font != null) txt.font = font;
                txt.text = label;
                txt.fontSize = 16f;
                txt.enableAutoSizing = false;
            }

            Button btn = btnGo.GetComponent<Button>();
            if (btn != null)
            {
                btn.onClick = new Button.ButtonClickedEvent();
                btn.onClick.AddListener(delegate { onClick(); });
            }
        }

        private static void AddSectionHeader(Transform parent, TMP_FontAsset font, Material fontMat, string title)
        {
            GameObject headerGo = new GameObject("Header_" + title, typeof(RectTransform), typeof(Image), typeof(HorizontalLayoutGroup), typeof(LayoutElement));
            headerGo.transform.SetParent(parent, false);
            headerGo.GetComponent<Image>().color = new Color(1f, 0.85f, 0.42f, 0.14f);

            HorizontalLayoutGroup hlg = headerGo.GetComponent<HorizontalLayoutGroup>();
            hlg.padding = new RectOffset(8, 8, 4, 4);
            hlg.childControlWidth = true;
            hlg.childControlHeight = true;
            hlg.childForceExpandWidth = true;
            hlg.childForceExpandHeight = true;

            LayoutElement le = headerGo.GetComponent<LayoutElement>();
            le.minHeight = 32f;
            le.preferredHeight = 32f;

            CreateText(headerGo.transform, "Title", font, fontMat, 20f, FontStyles.Bold,
                new Color(1f, 0.88f, 0.48f), TextAlignmentOptions.MidlineLeft).text = title;
        }

        private static void AddToggle(
            OptionsUI ui,
            AdvancedUiState state,
            Transform parent,
            TMP_FontAsset font,
            Material fontMat,
            string title,
            Func<bool> getter,
            Action<bool> setter)
        {
            if (ui.modTogglePrefab == null)
            {
                return;
            }

            GameObject go = UnityEngine.Object.Instantiate(ui.modTogglePrefab, parent, false);
            go.SetActive(true);
            StripXlateText(go);

            LayoutElement le = go.GetComponent<LayoutElement>() ?? go.AddComponent<LayoutElement>();
            le.minHeight = 32f;
            le.preferredHeight = 32f;
            le.flexibleWidth = 1f;

            Transform labelTr = go.transform.Find("Label");
            TMP_Text txt = labelTr != null ? labelTr.GetComponent<TMP_Text>() : go.GetComponentInChildren<TMP_Text>(true);
            if (txt != null)
            {
                if (font != null) txt.font = font;
                txt.text = title;
                txt.fontSize = 17f;
                txt.enableAutoSizing = false;
            }

            Toggle toggle = go.GetComponent<Toggle>();
            if (toggle == null)
            {
                return;
            }

            bool suppress = false;
            toggle.onValueChanged = new Toggle.ToggleEvent();
            toggle.isOn = getter();
            toggle.onValueChanged.AddListener(delegate(bool val)
            {
                if (suppress) return;
                setter(val);
                SettingsDatabase.Save();
            });

            state.RefreshCallbacks.Add(() =>
            {
                suppress = true;
                toggle.isOn = getter();
                suppress = false;
            });
        }

        private static void AddIntSlider(
            OptionsUI ui,
            AdvancedUiState state,
            Transform parent,
            TMP_FontAsset font,
            Material fontMat,
            string title,
            int min,
            int max,
            Func<int> getter,
            Action<int> setter,
            Func<int, string> formatter)
        {
            AddFloatSlider(ui, state, parent, font, fontMat, title, min, max, true,
                () => getter(),
                v => setter(Mathf.RoundToInt(v)),
                v => formatter(Mathf.RoundToInt(v)));
        }

        private static void AddFloatSlider(
            OptionsUI ui,
            AdvancedUiState state,
            Transform parent,
            TMP_FontAsset font,
            Material fontMat,
            string title,
            float min,
            float max,
            bool wholeNumbers,
            Func<float> getter,
            Action<float> setter,
            Func<float, string> formatter)
        {
            if (ui.fovSlider == null)
            {
                return;
            }

            GameObject row = new GameObject("SliderRow_" + title, typeof(RectTransform), typeof(HorizontalLayoutGroup), typeof(LayoutElement));
            row.transform.SetParent(parent, false);
            HorizontalLayoutGroup hlg = row.GetComponent<HorizontalLayoutGroup>();
            hlg.padding = new RectOffset(6, 6, 2, 2);
            hlg.spacing = 10f;
            hlg.childAlignment = TextAnchor.MiddleLeft;
            hlg.childControlWidth = true;
            hlg.childControlHeight = true;
            hlg.childForceExpandWidth = false;
            hlg.childForceExpandHeight = false;

            LayoutElement rowLe = row.GetComponent<LayoutElement>();
            rowLe.minHeight = 36f;
            rowLe.preferredHeight = 36f;

            TMP_Text label = CreateText(row.transform, "Label", font, fontMat, 17f, FontStyles.Normal,
                Color.white, TextAlignmentOptions.MidlineLeft);
            label.text = title;
            LayoutElement labelLe = label.gameObject.AddComponent<LayoutElement>();
            labelLe.flexibleWidth = 1f;
            labelLe.minWidth = 220f;
            labelLe.preferredHeight = 30f;

            GameObject sliderGo = UnityEngine.Object.Instantiate(ui.fovSlider.gameObject, row.transform, false);
            sliderGo.SetActive(true);
            StripXlateText(sliderGo);
            LayoutElement sliderLe = sliderGo.GetComponent<LayoutElement>() ?? sliderGo.AddComponent<LayoutElement>();
            sliderLe.minWidth = 170f;
            sliderLe.preferredWidth = 190f;
            sliderLe.minHeight = 26f;
            sliderLe.preferredHeight = 26f;

            TMP_Text valText;
            if (ui.fovValText != null)
            {
                GameObject vtGo = UnityEngine.Object.Instantiate(ui.fovValText.gameObject, row.transform, false);
                vtGo.SetActive(true);
                StripXlateText(vtGo);
                valText = vtGo.GetComponent<TMP_Text>();
            }
            else
            {
                valText = CreateText(row.transform, "Value", font, fontMat, 17f, FontStyles.Bold, Color.white, TextAlignmentOptions.MidlineRight);
            }

            if (valText != null)
            {
                if (font != null) valText.font = font;
                valText.fontSize = 17f;
                valText.alignment = TextAlignmentOptions.MidlineRight;
                LayoutElement vtLe = valText.gameObject.GetComponent<LayoutElement>() ?? valText.gameObject.AddComponent<LayoutElement>();
                vtLe.minWidth = 90f;
                vtLe.preferredWidth = 96f;
                vtLe.preferredHeight = 28f;
            }

            Slider slider = sliderGo.GetComponent<Slider>();
            if (slider == null)
            {
                return;
            }

            bool suppress = false;
            slider.onValueChanged = new Slider.SliderEvent();
            slider.minValue = min;
            slider.maxValue = max;
            slider.wholeNumbers = wholeNumbers;
            float initial = Mathf.Clamp(getter(), min, max);
            slider.value = initial;
            if (valText != null) valText.text = formatter(initial);

            slider.onValueChanged.AddListener(delegate(float val)
            {
                if (suppress) return;
                if (valText != null) valText.text = formatter(val);
                setter(val);
                SettingsDatabase.Save();
            });

            state.RefreshCallbacks.Add(() =>
            {
                suppress = true;
                float v = Mathf.Clamp(getter(), min, max);
                slider.value = v;
                if (valText != null) valText.text = formatter(v);
                suppress = false;
            });
        }

        private static TMP_Text CreateText(
            Transform parent,
            string name,
            TMP_FontAsset font,
            Material fontMat,
            float size,
            FontStyles style,
            Color color,
            TextAlignmentOptions align)
        {
            GameObject go = new GameObject(name, typeof(RectTransform), typeof(TextMeshProUGUI));
            go.transform.SetParent(parent, false);
            TextMeshProUGUI tmp = go.GetComponent<TextMeshProUGUI>();
            if (font != null) tmp.font = font;
            if (fontMat != null) tmp.fontSharedMaterial = fontMat;
            tmp.fontSize = size;
            tmp.fontStyle = style;
            tmp.color = color;
            tmp.alignment = align;
            tmp.enableAutoSizing = false;
            tmp.raycastTarget = false;
            return tmp;
        }

        private static void StripXlateText(GameObject go)
        {
            XlateText[] xlate = go.GetComponentsInChildren<XlateText>(true);
            for (int i = 0; i < xlate.Length; i++)
            {
                UnityEngine.Object.DestroyImmediate(xlate[i]);
            }
        }

        private static string FormatShadowRes(int v)
        {
            switch (v)
            {
                case 0: return T("Низк.", "Low");
                case 1: return T("Средн.", "Med");
                case 2: return T("Высок.", "High");
                default: return T("Ультра", "VeryHigh");
            }
        }

        private static string FormatAniso(int v)
        {
            switch (v)
            {
                case 0: return T("Выкл", "Off");
                case 1: return T("Авто", "Enable");
                default: return T("Макс", "Force");
            }
        }

        private static int FpsToStep(int fps)
        {
            if (fps <= 0) return 0;
            int bestIdx = 0;
            int bestDiff = int.MaxValue;
            for (int i = 1; i < FpsSteps.Length; i++)
            {
                int diff = Math.Abs(FpsSteps[i] - fps);
                if (diff < bestDiff)
                {
                    bestDiff = diff;
                    bestIdx = i;
                }
            }
            return bestIdx;
        }

        private static int StepToFps(int step)
        {
            if (step < 0 || step >= FpsSteps.Length) return 0;
            return FpsSteps[step];
        }

        private static int AaToStep(int aa)
        {
            if (aa >= 8) return 3;
            if (aa >= 4) return 2;
            if (aa >= 2) return 1;
            return 0;
        }

        private static int StepToAa(int step)
        {
            switch (step)
            {
                case 1: return 2;
                case 2: return 4;
                case 3: return 8;
                default: return 0;
            }
        }

        private static int CascadesToStep(int cascades)
        {
            if (cascades <= 0) return 0;
            if (cascades <= 2) return 1;
            return 2;
        }

        private static int StepToCascades(int step)
        {
            switch (step)
            {
                case 0: return 0;
                case 1: return 2;
                default: return 4;
            }
        }

        private static int ShaderLodToStep(int lod)
        {
            if (lod < 0) return 3;
            if (lod <= 450) return 0;
            if (lod <= 750) return 1;
            return 2;
        }

        private static int StepToShaderLod(int step)
        {
            switch (step)
            {
                case 0: return 400;
                case 1: return 700;
                case 2: return 900;
                default: return -1;
            }
        }
    }
}
