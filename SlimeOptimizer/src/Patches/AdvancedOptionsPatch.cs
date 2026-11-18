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
        private sealed class AdvancedUiState
        {
            public GameObject TabObject;
            public GameObject PanelObject;
            public TMP_Text InfoText;
            public readonly List<Action> RefreshCallbacks = new List<Action>();
        }

        private static readonly ConditionalWeakTable<OptionsUI, AdvancedUiState> States =
            new ConditionalWeakTable<OptionsUI, AdvancedUiState>();

        private static readonly MethodInfo DeselectAllMethod =
            AccessTools.Method(typeof(OptionsUI), "DeselectAll");

        [HarmonyPatch("SetupOptionsUI")]
        [HarmonyPostfix]
        private static void SetupOptionsUIPostfix(OptionsUI __instance)
        {
            if (__instance == null)
            {
                return;
            }

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
            RefreshState(state);
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
                    "База настроек: " + SettingsDatabase.DatabasePath + "\nШейдеры травы: " + GrassOptimizerPatch.GetSeenShadersSummary(),
                    "Settings DB: " + SettingsDatabase.DatabasePath + "\nGrass shaders: " + GrassOptimizerPatch.GetSeenShadersSummary());
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

            Transform contentParent = CreateScrollContainer(panelGo.transform);

            TMP_FontAsset font = ui.fovValText != null ? ui.fovValText.font : (tabLabel != null ? tabLabel.font : null);
            Material fontMat = ui.fovValText != null ? ui.fovValText.fontSharedMaterial : (tabLabel != null ? tabLabel.fontSharedMaterial : null);

            AddSectionHeader(contentParent, font, fontMat, T("Пресеты производительности (Sodium Style)", "Performance Presets"));
            AddPresetsRow(ui, state, contentParent, font, fontMat);

            AddSectionHeader(contentParent, font, fontMat, T("Графика и GPU", "Video & GPU Caps"));
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Умные лимиты графики (тени / MSAA / свет)", "Smart GPU Quality Caps"),
                () => OptimizerConfig.EnableSmartQualityCaps,
                v => { OptimizerConfig.EnableSmartQualityCaps = v; SettingsDatabase.ApplyRuntimeEffects(); });
            AddIntSlider(ui, state, contentParent, font, fontMat,
                T("Каскады теней (макс.)", "Max Shadow Cascades"),
                0, 4,
                () => OptimizerConfig.MaxShadowCascades,
                v => { OptimizerConfig.MaxShadowCascades = v; SettingsDatabase.ApplyRuntimeEffects(); },
                v => v.ToString(CultureInfo.InvariantCulture));
            AddIntSlider(ui, state, contentParent, font, fontMat,
                T("Разрешение теней (0=Низк .. 3=Ультра)", "Max Shadow Resolution"),
                0, 3,
                () => OptimizerConfig.MaxShadowResolution,
                v => { OptimizerConfig.MaxShadowResolution = v; SettingsDatabase.ApplyRuntimeEffects(); },
                FormatShadowRes);
            AddFloatSlider(ui, state, contentParent, font, fontMat,
                T("Дальность теней (м)", "Max Shadow Distance"),
                20f, 150f, true,
                () => OptimizerConfig.MaxShadowDistance,
                v => { OptimizerConfig.MaxShadowDistance = v; SettingsDatabase.ApplyRuntimeEffects(); },
                v => Mathf.RoundToInt(v) + " m");
            AddIntSlider(ui, state, contentParent, font, fontMat,
                T("Сглаживание MSAA (0=Выкл, 1=2x, 2=4x, 3=8x)", "Max MSAA"),
                0, 3,
                () => AaToStep(OptimizerConfig.MaxAntiAliasing),
                s => { OptimizerConfig.MaxAntiAliasing = StepToAa(s); SettingsDatabase.ApplyRuntimeEffects(); },
                s => { int aa = StepToAa(s); return aa == 0 ? T("Выкл", "Off") : aa + "x"; });
            AddIntSlider(ui, state, contentParent, font, fontMat,
                T("Пиксельных источников света", "Max Pixel Light Count"),
                0, 4,
                () => OptimizerConfig.MaxPixelLightCount,
                v => { OptimizerConfig.MaxPixelLightCount = v; SettingsDatabase.ApplyRuntimeEffects(); },
                v => v.ToString(CultureInfo.InvariantCulture));
            AddFloatSlider(ui, state, contentParent, font, fontMat,
                T("Множитель дальности LOD моделей", "Max LOD Bias"),
                0.5f, 2.0f, false,
                () => OptimizerConfig.MaxLodBias,
                v => { OptimizerConfig.MaxLodBias = (float)Math.Round(v, 2); SettingsDatabase.ApplyRuntimeEffects(); },
                v => v.ToString("0.00", CultureInfo.InvariantCulture));
            AddIntSlider(ui, state, contentParent, font, fontMat,
                T("Лимит шейдеров мира (3=Без лимита, 0=400, 1=700, 2=900)", "Global Shader LOD Cap"),
                0, 3,
                () => ShaderLodToStep(OptimizerConfig.MaxGlobalShaderLod),
                s => { OptimizerConfig.MaxGlobalShaderLod = StepToShaderLod(s); SettingsDatabase.ApplyRuntimeEffects(); },
                s => { int l = StepToShaderLod(s); return l < 0 ? T("Макс", "Auto") : l.ToString(CultureInfo.InvariantCulture); });

            AddSectionHeader(contentParent, font, fontMat, T("Постобработка SSAO", "SSAO Post-Processing"));
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Оптимизатор SSAO (ускорение в 4x + без утечек)", "Fast SSAO Optimizer"),
                () => OptimizerConfig.EnableSsaoOptimizer,
                v => OptimizerConfig.EnableSsaoOptimizer = v);
            AddIntSlider(ui, state, contentParent, font, fontMat,
                T("Делитель разрешения SSAO (1=Натив, 2=Половина)", "SSAO Downsampling"),
                1, 4,
                () => OptimizerConfig.SsaoDownsampling,
                v => OptimizerConfig.SsaoDownsampling = v,
                v => v + "x");
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Уменьшать разрешение блюра SSAO", "SSAO Blur Downsampling"),
                () => OptimizerConfig.SsaoBlurDownsampling,
                v => OptimizerConfig.SsaoBlurDownsampling = v);
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Быстрый Gaussian блюр SSAO", "Fast SSAO Gaussian Blur"),
                () => OptimizerConfig.SsaoUseGaussianBlur,
                v => OptimizerConfig.SsaoUseGaussianBlur = v);

            AddSectionHeader(contentParent, font, fontMat, T("Трава, Мир и Камера", "Grass, World & Camera"));
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Оптимизатор густой травы", "Grass Optimizer"),
                () => OptimizerConfig.EnableGrassOptimizer,
                v => { OptimizerConfig.EnableGrassOptimizer = v; GrassOptimizerPatch.ReapplyAllScenes(); });
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Отключить тени от травы", "Disable Grass Shadows"),
                () => OptimizerConfig.DisableGrassShadows,
                v => { OptimizerConfig.DisableGrassShadows = v; GrassOptimizerPatch.ReapplyAllScenes(); });
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Отключить тяжёлый ветер в шейдере травы", "Disable Heavy Grass Wind Shader"),
                () => OptimizerConfig.DisableGrassWindShader,
                v => { OptimizerConfig.DisableGrassWindShader = v; GrassOptimizerPatch.ReapplyAllScenes(); });
            AddFloatSlider(ui, state, contentParent, font, fontMat,
                T("Сдвиг LOD травы", "Grass LOD Multiplier"),
                1.0f, 2.5f, false,
                () => OptimizerConfig.GrassLodMultiplier,
                v => { OptimizerConfig.GrassLodMultiplier = (float)Math.Round(v, 2); GrassOptimizerPatch.ReapplyAllScenes(); },
                v => v.ToString("0.00", CultureInfo.InvariantCulture) + "x");
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Послойный куллинг камеры (дальние акторы)", "Camera Layer Culling"),
                () => OptimizerConfig.EnableCameraLayerCull,
                v => { OptimizerConfig.EnableCameraLayerCull = v; CameraCullDistancePatch.MarkDirty(); });
            AddFloatSlider(ui, state, contentParent, font, fontMat,
                T("Дальность мелких акторов (м)", "Actor Layer Cull Distance"),
                40f, 200f, true,
                () => OptimizerConfig.LayerActorCullDistance,
                v => { OptimizerConfig.LayerActorCullDistance = v; CameraCullDistancePatch.MarkDirty(); },
                v => Mathf.RoundToInt(v) + " m");
            AddFloatSlider(ui, state, contentParent, font, fontMat,
                T("Макс. дальность прорисовки мира (м)", "Max Camera Far Clip"),
                200f, 1000f, true,
                () => OptimizerConfig.MaxFarClipPlane,
                v => { OptimizerConfig.MaxFarClipPlane = v; CameraCullDistancePatch.MarkDirty(); },
                v => Mathf.RoundToInt(v) + " m");
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Убрать тени у плортов, еды и цыплят", "Cull Small Actor Shadows"),
                () => OptimizerConfig.EnableSmallObjectShadowCull,
                v => OptimizerConfig.EnableSmallObjectShadowCull = v);

            AddSectionHeader(contentParent, font, fontMat, T("Слаймы: Рендер, Физика и ИИ", "Slimes: Render, Physics & AI"));
            AddToggle(ui, state, contentParent, font, fontMat,
                T("LOD скиннинга и теней слаймов", "Slime Render LOD"),
                () => OptimizerConfig.EnableSlimeRenderLod,
                v => OptimizerConfig.EnableSlimeRenderLod = v);
            AddFloatSlider(ui, state, contentParent, font, fontMat,
                T("1-костный скиннинг слаймов с (м)", "Slime 1-Bone Skin Distance"),
                8f, 40f, true,
                () => OptimizerConfig.SlimeSkinLodDistance,
                v => OptimizerConfig.SlimeSkinLodDistance = v,
                v => Mathf.RoundToInt(v) + " m");
            AddFloatSlider(ui, state, contentParent, font, fontMat,
                T("Убирать тени слаймов с (м)", "Slime Shadow Cull Distance"),
                10f, 60f, true,
                () => OptimizerConfig.SlimeShadowCullDistance,
                v => OptimizerConfig.SlimeShadowCullDistance = v,
                v => Mathf.RoundToInt(v) + " m");
            AddToggle(ui, state, contentParent, font, fontMat,
                T("LOD желе-физики костей (RubberBone)", "RubberBone Jiggle LOD"),
                () => OptimizerConfig.EnableRubberBoneLod,
                v => OptimizerConfig.EnableRubberBoneLod = v);
            AddFloatSlider(ui, state, contentParent, font, fontMat,
                T("Замедлять желе-физику с (м)", "RubberBone LOD Distance"),
                4f, 25f, true,
                () => OptimizerConfig.RubberLodDistance,
                v => OptimizerConfig.RubberLodDistance = v,
                v => Mathf.RoundToInt(v) + " m");
            AddFloatSlider(ui, state, contentParent, font, fontMat,
                T("Усыплять желе-физику с (м)", "RubberBone Sleep Distance"),
                10f, 50f, true,
                () => OptimizerConfig.RubberDisableDistance,
                v => OptimizerConfig.RubberDisableDistance = v,
                v => Mathf.RoundToInt(v) + " m");
            AddToggle(ui, state, contentParent, font, fontMat,
                T("LOD ИИ и рейкастов дальних слаймов", "Slime AI Distance LOD"),
                () => OptimizerConfig.EnableSlimeAiLod,
                v => OptimizerConfig.EnableSlimeAiLod = v);
            AddFloatSlider(ui, state, contentParent, font, fontMat,
                T("Дистанция замедления ИИ (м)", "Far AI Distance"),
                12f, 50f, true,
                () => OptimizerConfig.FarAiDistance,
                v => OptimizerConfig.FarAiDistance = v,
                v => Mathf.RoundToInt(v) + " m");
            AddToggle(ui, state, contentParent, font, fontMat,
                T("LOD анимации лиц и эмоций слаймов", "Slime Face & Emotions LOD"),
                () => OptimizerConfig.EnableSlimeFaceEmotionLod,
                v => OptimizerConfig.EnableSlimeFaceEmotionLod = v);

            AddSectionHeader(contentParent, font, fontMat, T("Движок, Память и Статистика (GC)", "Engine, Memory & Stats"));
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Быстрый поиск еды без GetComponent", "Fast Food Search"),
                () => OptimizerConfig.EnableFastFoodSearch,
                v => OptimizerConfig.EnableFastFoodSearch = v);
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Кэш массивов материалов лица слайма", "Expression Material[] Cache"),
                () => OptimizerConfig.EnableExpressionCache,
                v => OptimizerConfig.EnableExpressionCache = v);
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Пул списков сборщика плортов", "PlortCollector List Pool"),
                () => OptimizerConfig.EnablePlortCollectorPool,
                v => OptimizerConfig.EnablePlortCollectorPool = v);
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Быстрый подсчёт в CellDirector", "CellDirector Fast Count"),
                () => OptimizerConfig.EnableCellDirectorFastCount,
                v => OptimizerConfig.EnableCellDirectorFastCount = v);
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Пул коллайдеров TrackCollisions", "TrackCollisions List Pool"),
                () => OptimizerConfig.EnableTrackCollisionsPool,
                v => OptimizerConfig.EnableTrackCollisionsPool = v);
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Кэш Camera.main для билбордов", "Billboard Camera Cache"),
                () => OptimizerConfig.EnableBillboardCameraCache,
                v => OptimizerConfig.EnableBillboardCameraCache = v);
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Кэш компонентов в DisableEffects", "Effects Component Cache"),
                () => OptimizerConfig.EnableEffectsComponentCache,
                v => OptimizerConfig.EnableEffectsComponentCache = v);
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Кэш делегатов рейкаста Plexer", "Plexer Delegate Cache"),
                () => OptimizerConfig.EnablePlexerDelegateCache,
                v => OptimizerConfig.EnablePlexerDelegateCache = v);
            AddToggle(ui, state, contentParent, font, fontMat,
                T("Сбор статистики (slimeopt stats)", "Collect Stats Counters"),
                () => OptimizerConfig.EnableStatsCollection,
                v => OptimizerConfig.EnableStatsCollection = v);

            AddResetStatsRow(ui, state, contentParent, font, fontMat);

            state.InfoText = CreateText(contentParent, "InfoLabel", font, fontMat, 14, FontStyles.Normal,
                new Color(0.8f, 0.88f, 0.96f, 0.9f), TextAlignmentOptions.TopLeft);
            state.InfoText.enableWordWrapping = true;
            LayoutElement infoLe = state.InfoText.gameObject.AddComponent<LayoutElement>();
            infoLe.minHeight = 46f;

            state.TabObject = tabGo;
            state.PanelObject = panelGo;
            RefreshState(state);
        }

        private static Transform CreateScrollContainer(Transform panelRoot)
        {
            GameObject scrollGo = new GameObject("SodiumScrollRect", typeof(RectTransform), typeof(Image), typeof(ScrollRect));
            scrollGo.transform.SetParent(panelRoot, false);
            RectTransform scrollRt = scrollGo.GetComponent<RectTransform>();
            scrollRt.anchorMin = Vector2.zero;
            scrollRt.anchorMax = Vector2.one;
            scrollRt.offsetMin = new Vector2(10f, 10f);
            scrollRt.offsetMax = new Vector2(-10f, -10f);

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

            ScrollRect sr = scrollGo.GetComponent<ScrollRect>();
            sr.viewport = viewportRt;
            sr.content = contentRt;
            sr.horizontal = false;
            sr.vertical = true;
            sr.movementType = ScrollRect.MovementType.Clamped;
            sr.scrollSensitivity = 36f;
            sr.verticalScrollbar = sb;
            sr.verticalScrollbarVisibility = ScrollRect.ScrollbarVisibility.Permanent;

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
                RefreshState(state);
            });
            CreateActionButton(ui, row.transform, font, fontMat, T("Баланс", "Balanced"), new Color(0.22f, 0.52f, 0.78f), () =>
            {
                SettingsDatabase.ApplyPreset(OptimizerPreset.Balanced);
                RefreshState(state);
            });
            CreateActionButton(ui, row.transform, font, fontMat, T("Качество", "High Quality"), new Color(0.68f, 0.50f, 0.20f), () =>
            {
                SettingsDatabase.ApplyPreset(OptimizerPreset.HighQuality);
                RefreshState(state);
            });
            CreateActionButton(ui, row.transform, font, fontMat, T("Ваниль (Выкл.)", "Vanilla (Off)"), new Color(0.62f, 0.28f, 0.28f), () =>
            {
                SettingsDatabase.ApplyPreset(OptimizerPreset.VanillaOff);
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

            CreateActionButton(ui, row.transform, font, fontMat, T("Сбросить настройки по умолчанию", "Reset To Defaults"), new Color(0.28f, 0.45f, 0.66f), () =>
            {
                SettingsDatabase.ApplyPreset(OptimizerPreset.Balanced);
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
                vtLe.minWidth = 70f;
                vtLe.preferredWidth = 75f;
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
