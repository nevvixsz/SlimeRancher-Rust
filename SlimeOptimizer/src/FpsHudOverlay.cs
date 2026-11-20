using System;
using System.Text;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace SlimeOptimizer
{
    internal sealed class FpsHudOverlay : MonoBehaviour
    {
        private static FpsHudOverlay _instance;

        public static int CurrentFps { get; private set; }
        public static int MinFps { get; private set; }
        public static float AvgFrameMs { get; private set; }
        public static bool IsOptionsMenuOpen { get; set; }

        private Canvas _canvas;
        private TextMeshProUGUI _text;
        private float _accumTime;
        private int _accumFrames;
        private float _worstFrameMs;
        private float _minResetTimer;
        private readonly StringBuilder _sb = new StringBuilder(64);

        public static void EnsureCreated()
        {
            if (_instance != null)
            {
                return;
            }

            GameObject go = new GameObject("SlimeOptimizerFpsHud");
            DontDestroyOnLoad(go);
            _instance = go.AddComponent<FpsHudOverlay>();
        }

        private void Awake()
        {
            _canvas = gameObject.AddComponent<Canvas>();
            _canvas.renderMode = RenderMode.ScreenSpaceOverlay;
            _canvas.sortingOrder = 32000;

            CanvasScaler scaler = gameObject.AddComponent<CanvasScaler>();
            scaler.uiScaleMode = CanvasScaler.ScaleMode.ScaleWithScreenSize;
            scaler.referenceResolution = new Vector2(1920f, 1080f);
            scaler.matchWidthOrHeight = 0.5f;

            GameObject bgGo = new GameObject("Badge", typeof(RectTransform), typeof(Image));
            bgGo.transform.SetParent(transform, false);
            RectTransform bgRt = bgGo.GetComponent<RectTransform>();
            bgRt.anchorMin = new Vector2(0.5f, 1f);
            bgRt.anchorMax = new Vector2(0.5f, 1f);
            bgRt.pivot = new Vector2(0.5f, 1f);
            bgRt.anchoredPosition = new Vector2(0f, -6f);
            bgRt.sizeDelta = new Vector2(270f, 28f);

            Image bgImg = bgGo.GetComponent<Image>();
            bgImg.color = new Color(0.04f, 0.06f, 0.10f, 0.62f);
            bgImg.raycastTarget = false;

            GameObject txtGo = new GameObject("FpsText", typeof(RectTransform), typeof(TextMeshProUGUI));
            txtGo.transform.SetParent(bgGo.transform, false);
            RectTransform txtRt = txtGo.GetComponent<RectTransform>();
            txtRt.anchorMin = Vector2.zero;
            txtRt.anchorMax = Vector2.one;
            txtRt.offsetMin = new Vector2(8f, 2f);
            txtRt.offsetMax = new Vector2(-8f, -2f);

            _text = txtGo.GetComponent<TextMeshProUGUI>();
            _text.fontSize = 15f;
            _text.fontStyle = FontStyles.Bold;
            _text.alignment = TextAlignmentOptions.Center;
            _text.raycastTarget = false;
            _text.text = "FPS: ...";

            _canvas.enabled = OptimizerConfig.ShowFpsHud;
        }

        private void Update()
        {
            bool show = OptimizerConfig.ShowFpsHud;
            if (_canvas != null && _canvas.enabled != show)
            {
                _canvas.enabled = show;
            }

            if (!show && !IsOptionsMenuOpen)
            {
                return;
            }

            float dt = Time.unscaledDeltaTime;
            if (dt <= 0f || dt > 1f)
            {
                return;
            }

            _accumTime += dt;
            _accumFrames++;
            float dtMs = dt * 1000f;
            if (dtMs > _worstFrameMs)
            {
                _worstFrameMs = dtMs;
            }

            _minResetTimer += dt;
            if (_minResetTimer >= 4.0f)
            {
                _minResetTimer = 0f;
                _worstFrameMs = dtMs;
            }

            if (_accumTime >= 0.25f)
            {
                CurrentFps = Mathf.RoundToInt(_accumFrames / _accumTime);
                AvgFrameMs = (_accumTime / _accumFrames) * 1000f;
                MinFps = _worstFrameMs > 0.1f ? Mathf.RoundToInt(1000f / _worstFrameMs) : CurrentFps;

                if (show && _text != null)
                {
                    if (CurrentFps >= 120)
                    {
                        _text.color = new Color(0.45f, 0.98f, 0.55f);
                    }
                    else if (CurrentFps >= 60)
                    {
                        _text.color = new Color(1.0f, 0.88f, 0.35f);
                    }
                    else
                    {
                        _text.color = new Color(1.0f, 0.42f, 0.42f);
                    }

                    _sb.Length = 0;
                    _sb.Append("FPS: ").Append(CurrentFps)
                       .Append("  |  min ").Append(MinFps)
                       .Append("  |  ").Append(AvgFrameMs.ToString("0.0")).Append(" ms");
                    _text.text = _sb.ToString();
                }

                _accumTime = 0f;
                _accumFrames = 0;
            }
        }
    }
}
