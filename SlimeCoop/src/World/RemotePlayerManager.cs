using System.Collections.Generic;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace SlimeCoop.World
{
    internal sealed class RemotePlayerVisual
    {
        public int PlayerId;
        public string Name;
        public int ColorIndex;
        public GameObject Root;
        public Transform HeadTransform;
        public Transform VacTransform;
        public GameObject JetpackGlow;
        public Transform LabelCanvasTransform;
        public TMP_Text NameText;

        public Vector3 TargetPos;
        public float TargetYaw;
        public float TargetPitch;
        public bool IsJetpacking;
        public bool IsVacActive;
        public ZoneDirector.Zone CurrentZone = ZoneDirector.Zone.RANCH;
        public float LastPacketTime;
    }

    internal static class RemotePlayerManager
    {
        private static readonly Color[] PlayerColors =
        {
            new Color(1.0f, 0.42f, 0.68f), // Pink Slime
            new Color(0.32f, 0.78f, 1.0f), // Puddle/Rock Blue
            new Color(0.50f, 0.96f, 0.56f), // Rad Lime
            new Color(1.0f, 0.78f, 0.25f), // Honey Gold
            new Color(0.76f, 0.48f, 1.0f), // Quantum Violet
            new Color(0.35f, 0.95f, 0.85f), // Mosaic Mint
            new Color(1.0f, 0.52f, 0.34f), // Boom Coral
            new Color(0.95f, 0.95f, 0.98f)  // Crystal White
        };

        private static readonly Dictionary<int, RemotePlayerVisual> Visuals =
            new Dictionary<int, RemotePlayerVisual>();

        private static TMP_FontAsset _cachedFont;
        private static Material _cachedFontMat;
        private static Shader _standardShader;
        private static Camera _cachedCam;

        public static void ClearAll()
        {
            foreach (RemotePlayerVisual v in Visuals.Values)
            {
                if (v.Root != null)
                {
                    Object.Destroy(v.Root);
                }
            }
            Visuals.Clear();
        }

        public static void RemovePlayer(int playerId)
        {
            if (Visuals.TryGetValue(playerId, out RemotePlayerVisual v))
            {
                if (v.Root != null)
                {
                    Object.Destroy(v.Root);
                }
                Visuals.Remove(playerId);
            }
        }

        public static void UpdateTransform(int playerId, string name, int colorIdx, Vector3 pos, float yaw, float pitch, bool jetpack, bool vacActive)
        {
            if (SRSingleton<SceneContext>.Instance == null) return;

            if (!Visuals.TryGetValue(playerId, out RemotePlayerVisual v) || v.Root == null)
            {
                v = CreateVisual(playerId, name, colorIdx, pos);
                Visuals[playerId] = v;
            }

            if (!string.IsNullOrEmpty(name) && v.Name != name)
            {
                v.Name = name;
                if (v.NameText != null) v.NameText.text = name;
            }

            v.TargetPos = pos;
            v.TargetYaw = yaw;
            v.TargetPitch = pitch;
            v.IsJetpacking = jetpack;
            v.IsVacActive = vacActive;
            v.LastPacketTime = Time.unscaledTime;
        }

        public static bool TryGetPlayerPosition(int playerId, out Vector3 pos)
        {
            if (Visuals.TryGetValue(playerId, out RemotePlayerVisual v) && v.Root != null)
            {
                pos = v.Root.transform.position;
                return true;
            }
            pos = Vector3.zero;
            return false;
        }

        public static void TickInterpolate()
        {
            if (Visuals.Count == 0) return;

            if (_cachedCam == null || !_cachedCam.isActiveAndEnabled)
            {
                _cachedCam = Camera.main;
            }
            Camera cam = _cachedCam;
            float lerpT = Mathf.Clamp01(Time.unscaledDeltaTime * 16f);

            foreach (RemotePlayerVisual v in Visuals.Values)
            {
                if (v.Root == null) continue;

                v.Root.transform.position = Vector3.Lerp(v.Root.transform.position, v.TargetPos, lerpT);
                Quaternion bodyRot = Quaternion.Euler(0f, v.TargetYaw, 0f);
                v.Root.transform.rotation = Quaternion.Slerp(v.Root.transform.rotation, bodyRot, lerpT);

                if (v.HeadTransform != null)
                {
                    v.HeadTransform.localRotation = Quaternion.Euler(Mathf.Clamp(v.TargetPitch, -75f, 75f), 0f, 0f);
                }
                if (v.VacTransform != null)
                {
                    float vacShake = v.IsVacActive ? Mathf.Sin(Time.unscaledTime * 32f) * 0.03f : 0f;
                    v.VacTransform.localPosition = new Vector3(0.42f + vacShake, 0.92f, 0.35f);
                }
                if (v.JetpackGlow != null && v.JetpackGlow.activeSelf != v.IsJetpacking)
                {
                    v.JetpackGlow.SetActive(v.IsJetpacking);
                }
                if (v.LabelCanvasTransform != null && cam != null)
                {
                    v.LabelCanvasTransform.rotation = cam.transform.rotation;
                }
            }
        }

        private static RemotePlayerVisual CreateVisual(int playerId, string name, int colorIdx, Vector3 pos)
        {
            Color accent = PlayerColors[Mathf.Abs(colorIdx) % PlayerColors.Length];
            if (_standardShader == null)
            {
                _standardShader = Shader.Find("Standard") ?? Shader.Find("Unlit/Color") ?? Shader.Find("Sprites/Default");
            }

            GameObject root = new GameObject("SlimeCoop_Player_" + playerId);
            Object.DontDestroyOnLoad(root);
            root.transform.position = pos;

            // Body capsule (no collider so it doesn't interfere with player/slime physics)
            GameObject body = CreatePart("Body", PrimitiveType.Capsule, root.transform,
                new Vector3(0f, 0.85f, 0f), new Vector3(0.68f, 0.62f, 0.68f), accent);

            // Backpack
            CreatePart("Backpack", PrimitiveType.Cube, root.transform,
                new Vector3(0f, 0.98f, -0.34f), new Vector3(0.46f, 0.56f, 0.24f), new Color(0.22f, 0.26f, 0.34f));

            // Jetpack glow
            GameObject jetGlow = CreatePart("JetpackGlow", PrimitiveType.Sphere, root.transform,
                new Vector3(0f, 0.58f, -0.38f), new Vector3(0.32f, 0.42f, 0.32f), new Color(1.0f, 0.65f, 0.15f));
            jetGlow.SetActive(false);

            // Head
            GameObject head = CreatePart("Head", PrimitiveType.Sphere, root.transform,
                new Vector3(0f, 1.56f, 0f), new Vector3(0.62f, 0.58f, 0.62f), Color.Lerp(accent, Color.white, 0.2f));

            // Cute slime eyes on head
            CreatePart("EyeL", PrimitiveType.Sphere, head.transform,
                new Vector3(-0.20f, 0.08f, 0.44f), new Vector3(0.16f, 0.20f, 0.12f), new Color(0.08f, 0.08f, 0.12f));
            CreatePart("EyeR", PrimitiveType.Sphere, head.transform,
                new Vector3(0.20f, 0.08f, 0.44f), new Vector3(0.16f, 0.20f, 0.12f), new Color(0.08f, 0.08f, 0.12f));

            // Vacpack gun
            GameObject vac = CreatePart("VacPack", PrimitiveType.Cylinder, root.transform,
                new Vector3(0.42f, 0.92f, 0.35f), new Vector3(0.20f, 0.45f, 0.20f), new Color(0.20f, 0.24f, 0.30f));
            vac.transform.localRotation = Quaternion.Euler(90f, 0f, 0f);

            // World-space name tag
            TMP_Text nameText = null;
            GameObject labelGo = new GameObject("NameTag");
            labelGo.transform.SetParent(root.transform, false);
            labelGo.transform.localPosition = new Vector3(0f, 2.25f, 0f);
            labelGo.transform.localScale = new Vector3(0.014f, 0.014f, 0.014f);

            Canvas canvas = labelGo.AddComponent<Canvas>();
            canvas.renderMode = RenderMode.WorldSpace;
            RectTransform crt = labelGo.GetComponent<RectTransform>();
            crt.sizeDelta = new Vector2(260f, 52f);

            GameObject bgGo = new GameObject("Bg");
            bgGo.transform.SetParent(labelGo.transform, false);
            Image bg = bgGo.AddComponent<Image>();
            bg.color = new Color(0.03f, 0.05f, 0.09f, 0.70f);
            RectTransform bgRt = bgGo.GetComponent<RectTransform>();
            bgRt.anchorMin = Vector2.zero;
            bgRt.anchorMax = Vector2.one;
            bgRt.offsetMin = Vector2.zero;
            bgRt.offsetMax = Vector2.zero;

            GameObject txtGo = new GameObject("Text");
            txtGo.transform.SetParent(labelGo.transform, false);
            TextMeshProUGUI tmp = txtGo.AddComponent<TextMeshProUGUI>();
            ResolveFont();
            if (_cachedFont != null) tmp.font = _cachedFont;
            if (_cachedFontMat != null) tmp.fontSharedMaterial = _cachedFontMat;
            tmp.text = string.IsNullOrEmpty(name) ? ("Player " + playerId) : name;
            tmp.fontSize = 26f;
            tmp.alignment = TextAlignmentOptions.Center;
            tmp.color = Color.Lerp(accent, Color.white, 0.45f);
            tmp.enableWordWrapping = false;
            RectTransform trt = txtGo.GetComponent<RectTransform>();
            trt.anchorMin = Vector2.zero;
            trt.anchorMax = Vector2.one;
            trt.offsetMin = Vector2.zero;
            trt.offsetMax = Vector2.zero;
            nameText = tmp;

            return new RemotePlayerVisual
            {
                PlayerId = playerId,
                Name = name,
                ColorIndex = colorIdx,
                Root = root,
                HeadTransform = head.transform,
                VacTransform = vac.transform,
                JetpackGlow = jetGlow,
                LabelCanvasTransform = labelGo.transform,
                NameText = nameText,
                TargetPos = pos,
                LastPacketTime = Time.unscaledTime
            };
        }

        private static GameObject CreatePart(string name, PrimitiveType type, Transform parent, Vector3 localPos, Vector3 localScale, Color color)
        {
            GameObject go = GameObject.CreatePrimitive(type);
            go.name = name;
            Collider col = go.GetComponent<Collider>();
            if (col != null) Object.Destroy(col);
            go.transform.SetParent(parent, false);
            go.transform.localPosition = localPos;
            go.transform.localScale = localScale;
            Renderer r = go.GetComponent<Renderer>();
            if (r != null && _standardShader != null)
            {
                Material mat = new Material(_standardShader);
                mat.color = color;
                r.sharedMaterial = mat;
            }
            return go;
        }

        private static void ResolveFont()
        {
            if (_cachedFont != null) return;
            TMP_Text[] existing = Resources.FindObjectsOfTypeAll<TMP_Text>();
            for (int i = 0; i < existing.Length; i++)
            {
                if (existing[i] != null && existing[i].font != null)
                {
                    _cachedFont = existing[i].font;
                    _cachedFontMat = existing[i].fontSharedMaterial;
                    break;
                }
            }
        }
    }
}
