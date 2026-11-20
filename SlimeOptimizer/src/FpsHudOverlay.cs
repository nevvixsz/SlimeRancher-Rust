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
        private bool _lastShow = true;
        private float _accumTime;
        private int _accumFrames;
        private float _worstFrameMs;
        private float _minResetTimer;

        private int _lastShownFps = -1;
        private int _lastShownMin = -1;
        private int _lastShownTenths = -1;
        private int _lastColorTier = -1;
        private readonly char[] _charBuf = new char[64];

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

            _lastShow = OptimizerConfig.ShowFpsHud;
            _canvas.enabled = _lastShow;
        }

        private void FixedUpdate()
        {
            CameraCache.EnsureCurrentFrame();
        }

        private void Update()
        {
            bool show = OptimizerConfig.ShowFpsHud;
            if (show != _lastShow)
            {
                _lastShow = show;
                if (_canvas != null)
                {
                    _canvas.enabled = show;
                }
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

            if (_accumTime >= 0.5f)
            {
                int fps = Mathf.RoundToInt(_accumFrames / _accumTime);
                float avgMs = (_accumTime / _accumFrames) * 1000f;
                int minFps = _worstFrameMs > 0.1f ? Mathf.RoundToInt(1000f / _worstFrameMs) : fps;

                CurrentFps = fps;
                AvgFrameMs = avgMs;
                MinFps = minFps;

                if (show && _text != null)
                {
                    int tenths = Mathf.RoundToInt(avgMs * 10f);
                    if (fps != _lastShownFps || minFps != _lastShownMin || tenths != _lastShownTenths)
                    {
                        _lastShownFps = fps;
                        _lastShownMin = minFps;
                        _lastShownTenths = tenths;

                        int colorTier = fps >= 120 ? 2 : (fps >= 60 ? 1 : 0);
                        if (colorTier != _lastColorTier)
                        {
                            _lastColorTier = colorTier;
                            _text.color = colorTier == 2
                                ? new Color(0.45f, 0.98f, 0.55f)
                                : (colorTier == 1 ? new Color(1.0f, 0.88f, 0.35f) : new Color(1.0f, 0.42f, 0.42f));
                        }

                        int len = FormatHudChars(fps, minFps, tenths);
                        _text.SetCharArray(_charBuf, 0, len);
                    }
                }

                _accumTime = 0f;
                _accumFrames = 0;
            }
        }

        private int FormatHudChars(int fps, int minFps, int avgTenths)
        {
            int pos = 0;
            pos = WriteLiteral(pos, "FPS: ");
            pos = WriteInt(pos, fps);
            pos = WriteLiteral(pos, "  |  min ");
            pos = WriteInt(pos, minFps);
            pos = WriteLiteral(pos, "  |  ");
            pos = WriteInt(pos, avgTenths / 10);
            _charBuf[pos++] = '.';
            _charBuf[pos++] = (char)('0' + (avgTenths % 10));
            pos = WriteLiteral(pos, " ms");
            return pos;
        }

        private int WriteLiteral(int pos, string s)
        {
            for (int i = 0; i < s.Length; i++)
            {
                _charBuf[pos++] = s[i];
            }
            return pos;
        }

        private int WriteInt(int pos, int val)
        {
            if (val <= 0)
            {
                _charBuf[pos++] = '0';
                return pos;
            }

            int start = pos;
            while (val > 0)
            {
                _charBuf[pos++] = (char)('0' + (val % 10));
                val /= 10;
            }

            int end = pos - 1;
            while (start < end)
            {
                char tmp = _charBuf[start];
                _charBuf[start] = _charBuf[end];
                _charBuf[end] = tmp;
                start++;
                end--;
            }

            return pos;
        }
    }
}
