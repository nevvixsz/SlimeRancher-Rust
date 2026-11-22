using System;
using System.Collections.Generic;
using System.Reflection;
using System.Text;
using TMPro;
using UnityEngine;
using UnityEngine.UI;
using Object = UnityEngine.Object;

namespace SlimeCoop.World
{
    internal sealed class RemotePlayerVisual
    {
        public int PlayerId;
        public string Name;
        public int ColorIndex;
        public GameObject Root;
        public Transform RootTransform;
        public Transform HeadTransform;
        public Transform VacTransform;
        public GameObject JetpackGlow;
        public GameObject VacGlow;
        public Transform LabelCanvasTransform;
        public TMP_Text NameText;
        public bool IsRealBeatrixModel;

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
        private static Transform _cachedCamTransform;
        private static GameObject _cachedBeatrixPrefab;
        private static bool _beatrixSearchLogged;
        private static bool _beatrixDumpLogged;

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
            _cachedCam = null;
            _cachedCamTransform = null;
            _cachedBeatrixPrefab = null;
            _beatrixSearchLogged = false;
            _beatrixDumpLogged = false;
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
            if (Visuals.TryGetValue(playerId, out RemotePlayerVisual v) && v.RootTransform != null)
            {
                pos = v.RootTransform.position;
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
                _cachedCamTransform = _cachedCam != null ? _cachedCam.transform : null;
            }
            Transform camTr = _cachedCamTransform;
            float lerpT = Mathf.Clamp01(Time.unscaledDeltaTime * 16f);

            foreach (RemotePlayerVisual v in Visuals.Values)
            {
                Transform rootTr = v.RootTransform;
                if (rootTr == null) continue;

                rootTr.position = Vector3.Lerp(rootTr.position, v.TargetPos, lerpT);
                Quaternion bodyRot = Quaternion.Euler(0f, v.TargetYaw, 0f);
                rootTr.rotation = Quaternion.Slerp(rootTr.rotation, bodyRot, lerpT);

                if (!v.IsRealBeatrixModel)
                {
                    if (v.HeadTransform != null)
                    {
                        v.HeadTransform.localRotation = Quaternion.Euler(Mathf.Clamp(v.TargetPitch, -75f, 75f), 0f, 0f);
                    }
                    if (v.VacTransform != null)
                    {
                        float vacShake = v.IsVacActive ? Mathf.Sin(Time.unscaledTime * 32f) * 0.03f : 0f;
                        v.VacTransform.localPosition = new Vector3(0.42f + vacShake, 0.92f, 0.35f);
                    }
                }

                if (v.JetpackGlow != null && v.JetpackGlow.activeSelf != v.IsJetpacking)
                {
                    v.JetpackGlow.SetActive(v.IsJetpacking);
                }
                if (v.VacGlow != null && v.VacGlow.activeSelf != v.IsVacActive)
                {
                    v.VacGlow.SetActive(v.IsVacActive);
                }
                if (v.LabelCanvasTransform != null && camTr != null)
                {
                    v.LabelCanvasTransform.rotation = camTr.rotation;
                }
            }
        }

        private static GameObject ResolveBeatrixPrefab()
        {
            if (_cachedBeatrixPrefab != null) return _cachedBeatrixPrefab;

            try
            {
                RanchHouseUI[] houseUis = Resources.FindObjectsOfTypeAll<RanchHouseUI>();
                for (int i = 0; i < houseUis.Length; i++)
                {
                    if (houseUis[i] != null && houseUis[i].beatrixPrefab != null)
                    {
                        _cachedBeatrixPrefab = houseUis[i].beatrixPrefab;
                        break;
                    }
                }

                if (_cachedBeatrixPrefab == null)
                {
                    UIActivator[] activators = Resources.FindObjectsOfTypeAll<UIActivator>();
                    for (int i = 0; i < activators.Length; i++)
                    {
                        UIActivator act = activators[i];
                        if (act == null || act.uiPrefab == null) continue;
                        RanchHouseUI ui = act.uiPrefab.GetComponent<RanchHouseUI>();
                        if (ui != null && ui.beatrixPrefab != null)
                        {
                            _cachedBeatrixPrefab = ui.beatrixPrefab;
                            break;
                        }
                    }
                }
            }
            catch (Exception ex)
            {
                CoopLog.Warn("ResolveBeatrixPrefab failed: " + ex.Message);
            }

            if (!_beatrixSearchLogged)
            {
                _beatrixSearchLogged = true;
                CoopLog.Info("ResolveBeatrixPrefab result: " + (_cachedBeatrixPrefab != null ? _cachedBeatrixPrefab.name : "not found (using fallback visual)"));
            }

            return _cachedBeatrixPrefab;
        }

        private static Shader ResolveSafeShader()
        {
            if (_standardShader != null && _standardShader.isSupported && _standardShader.name != "Hidden/InternalErrorShader")
            {
                return _standardShader;
            }

            string[] candidates =
            {
                "Diffuse",
                "Legacy Shaders/Diffuse",
                "Standard",
                "Transparent/Diffuse",
                "Unlit/Color",
                "Sprites/Default"
            };
            for (int i = 0; i < candidates.Length; i++)
            {
                Shader s = Shader.Find(candidates[i]);
                if (s != null && s.isSupported && s.name != "Hidden/InternalErrorShader")
                {
                    _standardShader = s;
                    CoopLog.Info("RemotePlayerManager ResolveSafeShader -> " + s.name);
                    return _standardShader;
                }
            }

            return _standardShader;
        }

        private static RemotePlayerVisual CreateVisual(int playerId, string name, int colorIdx, Vector3 pos)
        {
            Color accent = PlayerColors[Mathf.Abs(colorIdx) % PlayerColors.Length];
            ResolveSafeShader();

            GameObject root = new GameObject("SlimeCoop_Player_" + playerId);
            Object.DontDestroyOnLoad(root);
            root.transform.position = pos;

            Transform headTransform = null;
            Transform vacTransform = null;
            bool isRealBeatrix = false;
            float labelY = 2.15f;

            GameObject beatrixPrefab = ResolveBeatrixPrefab();
            if (beatrixPrefab != null)
            {
                isRealBeatrix = TryBuildBeatrixVisual(beatrixPrefab, root.transform, out labelY);
            }

            if (!isRealBeatrix)
            {
                BuildFallbackVisual(root.transform, accent, out headTransform, out vacTransform);
                labelY = 2.25f;
            }

            // Jetpack glow (visible when flying)
            Vector3 jetPos = isRealBeatrix ? new Vector3(0f, 0.72f, -0.32f) : new Vector3(0f, 0.58f, -0.36f);
            GameObject jetGlow = CreatePart("JetpackGlow", PrimitiveType.Sphere, root.transform,
                jetPos, new Vector3(0.30f, 0.38f, 0.30f), new Color(1.0f, 0.65f, 0.15f));
            jetGlow.SetActive(false);

            // Vacpack active glow
            Vector3 vacPos = isRealBeatrix ? new Vector3(0.28f, 0.96f, 0.64f) : new Vector3(0.36f, 0.96f, 0.66f);
            GameObject vacGlow = CreatePart("VacGlow", PrimitiveType.Sphere, root.transform,
                vacPos, new Vector3(0.20f, 0.20f, 0.20f), new Color(0.35f, 0.88f, 1.0f));
            vacGlow.SetActive(false);

            // World-space name tag
            TMP_Text nameText = null;
            GameObject labelGo = new GameObject("NameTag");
            labelGo.transform.SetParent(root.transform, false);
            labelGo.transform.localPosition = new Vector3(0f, labelY, 0f);
            labelGo.transform.localScale = new Vector3(0.014f, 0.014f, 0.014f);

            Canvas canvas = labelGo.AddComponent<Canvas>();
            canvas.renderMode = RenderMode.WorldSpace;
            RectTransform crt = labelGo.GetComponent<RectTransform>();
            crt.sizeDelta = new Vector2(260f, 52f);

            GameObject bgGo = new GameObject("Bg");
            bgGo.transform.SetParent(labelGo.transform, false);
            Image bg = bgGo.AddComponent<Image>();
            bg.color = new Color(0.03f, 0.05f, 0.09f, 0.72f);
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
                RootTransform = root.transform,
                HeadTransform = headTransform,
                VacTransform = vacTransform,
                JetpackGlow = jetGlow,
                VacGlow = vacGlow,
                LabelCanvasTransform = labelGo.transform,
                NameText = nameText,
                IsRealBeatrixModel = isRealBeatrix,
                TargetPos = pos,
                LastPacketTime = Time.unscaledTime
            };
        }

        private static bool TryBuildBeatrixVisual(GameObject beatrixPrefab, Transform parentRoot, out float labelY)
        {
            labelY = 2.10f;
            GameObject pivot = new GameObject("BeatrixPivot");
            pivot.transform.SetParent(parentRoot, false);
            pivot.transform.localPosition = Vector3.zero;
            pivot.transform.localRotation = Quaternion.identity;
            pivot.transform.localScale = Vector3.one;

            GameObject clone = null;
            try
            {
                clone = Object.Instantiate(beatrixPrefab, pivot.transform, false);
                clone.SetActive(true);

                // Determine portrait camera facing direction before removing cameras
                Camera portraitCam = clone.GetComponentInChildren<Camera>(true);
                Vector3 camPos = Vector3.zero;
                Vector3 camForward = Vector3.forward;
                bool hasPortraitCam = portraitCam != null;
                if (hasPortraitCam)
                {
                    camPos = portraitCam.transform.position;
                    camForward = portraitCam.transform.forward;
                }

                StringBuilder dump = !_beatrixDumpLogged ? new StringBuilder() : null;
                if (dump != null)
                {
                    dump.Append("BeatrixPrefab dump '").Append(beatrixPrefab.name).Append("': ");
                }

                // Disable & strip non-visual/physics/camera/distortion components immediately
                Camera[] cams = clone.GetComponentsInChildren<Camera>(true);
                for (int i = 0; i < cams.Length; i++)
                {
                    if (cams[i] != null)
                    {
                        cams[i].enabled = false;
                        Object.Destroy(cams[i]);
                    }
                }

                Light[] lights = clone.GetComponentsInChildren<Light>(true);
                for (int i = 0; i < lights.Length; i++)
                {
                    if (lights[i] != null)
                    {
                        lights[i].enabled = false;
                        Object.Destroy(lights[i]);
                    }
                }

                Canvas[] canvases = clone.GetComponentsInChildren<Canvas>(true);
                for (int i = 0; i < canvases.Length; i++)
                {
                    if (canvases[i] != null)
                    {
                        canvases[i].enabled = false;
                        Object.Destroy(canvases[i]);
                    }
                }

                Collider[] cols = clone.GetComponentsInChildren<Collider>(true);
                for (int i = 0; i < cols.Length; i++)
                {
                    if (cols[i] != null)
                    {
                        cols[i].enabled = false;
                        Object.Destroy(cols[i]);
                    }
                }

                Rigidbody[] rbs = clone.GetComponentsInChildren<Rigidbody>(true);
                for (int i = 0; i < rbs.Length; i++)
                {
                    if (rbs[i] != null)
                    {
                        rbs[i].isKinematic = true;
                        Object.Destroy(rbs[i]);
                    }
                }

                MonoBehaviour[] behaviours = clone.GetComponentsInChildren<MonoBehaviour>(true);
                for (int i = 0; i < behaviours.Length; i++)
                {
                    MonoBehaviour mb = behaviours[i];
                    if (mb == null) continue;
                    string mbName = mb.GetType().Name;
                    if (dump != null) dump.Append("[MB:").Append(mbName).Append(" on ").Append(mb.gameObject.name).Append("] ");

                    // Keep Recolorizer & UpdateMaterialUnscaledTime so Beatrix/vac materials get proper colors!
                    if (mbName == "Recolorizer" || mbName == "UpdateMaterialUnscaledTime")
                    {
                        continue;
                    }

                    mb.enabled = false;
                    Object.Destroy(mb);
                }

                Component[] allComponents = clone.GetComponentsInChildren<Component>(true);
                for (int i = 0; i < allComponents.Length; i++)
                {
                    Component c = allComponents[i];
                    if (c == null) continue;
                    string typeName = c.GetType().Name;

                    if (typeName == "AudioListener" || typeName == "Cloth" || typeName == "TrailRenderer" || typeName == "LineRenderer" || typeName == "ParticleSystem")
                    {
                        DisableComponentIfPossible(c);
                        Object.Destroy(c);
                    }
                    else if (typeName == "Animator")
                    {
                        Type t = c.GetType();
                        PropertyInfo pRoot = t.GetProperty("applyRootMotion");
                        if (pRoot != null) pRoot.SetValue(c, false, null);
                        PropertyInfo pUpdate = t.GetProperty("updateMode");
                        if (pUpdate != null && pUpdate.PropertyType.IsEnum)
                        {
                            pUpdate.SetValue(c, Enum.ToObject(pUpdate.PropertyType, 0), null);
                        }
                        PropertyInfo pCull = t.GetProperty("cullingMode");
                        if (pCull != null && pCull.PropertyType.IsEnum)
                        {
                            pCull.SetValue(c, Enum.ToObject(pCull.PropertyType, 0), null);
                        }
                    }
                }

                // Move all transforms to Default layer (0) so main camera renders Beatrix instead of hidden layer 17
                Transform[] allTransforms = clone.GetComponentsInChildren<Transform>(true);
                for (int i = 0; i < allTransforms.Length; i++)
                {
                    if (allTransforms[i] != null)
                    {
                        allTransforms[i].gameObject.layer = 0;
                    }
                }

                // Inspect & fix materials/renderers and compute world bounds
                RanchDirector ranchDir = SRSingleton<SceneContext>.Instance != null
                    ? SRSingleton<SceneContext>.Instance.RanchDirector
                    : null;
                Shader fallbackShader = ResolveSafeShader();

                Renderer[] renderers = clone.GetComponentsInChildren<Renderer>(true);
                Bounds totalBounds = new Bounds();
                bool hasBounds = false;
                for (int i = 0; i < renderers.Length; i++)
                {
                    Renderer r = renderers[i];
                    if (r == null) continue;
                    string rTypeName = r.GetType().Name;
                    if (rTypeName == "ParticleSystemRenderer" || rTypeName == "TrailRenderer" || rTypeName == "LineRenderer")
                    {
                        r.enabled = false;
                        continue;
                    }
                    if (!r.enabled || !r.gameObject.activeInHierarchy) continue;

                    SkinnedMeshRenderer smr = r as SkinnedMeshRenderer;
                    if (smr != null)
                    {
                        smr.updateWhenOffscreen = true;
                    }

                    // Ensure materials are valid and recolored if applicable
                    Material[] mats = r.sharedMaterials;
                    bool matChanged = false;
                    for (int m = 0; m < mats.Length; m++)
                    {
                        Material mat = mats[m];
                        if (mat == null) continue;

                        if (dump != null)
                        {
                            string shName = mat.shader != null ? mat.shader.name : "NULL";
                            bool sup = mat.shader != null && mat.shader.isSupported;
                            dump.Append("[R:").Append(r.gameObject.name).Append("(").Append(rTypeName).Append(") mat=").Append(mat.name)
                                .Append(" sh=").Append(shName).Append(" sup=").Append(sup).Append("] ");
                        }

                        if (ranchDir != null)
                        {
                            Material recolored = ranchDir.GetRecolorMaterial(mat, ZoneDirector.Zone.RANCH);
                            if (recolored != null && recolored != mat)
                            {
                                mats[m] = recolored;
                                mat = recolored;
                                matChanged = true;
                            }
                        }

                        if (mat.shader == null || !mat.shader.isSupported || mat.shader.name == "Hidden/InternalErrorShader")
                        {
                            if (fallbackShader != null)
                            {
                                Material repaired = new Material(mat);
                                repaired.shader = fallbackShader;
                                mats[m] = repaired;
                                matChanged = true;
                            }
                        }
                    }
                    if (matChanged)
                    {
                        r.sharedMaterials = mats;
                    }

                    Bounds b = r.bounds;
                    if (b.size.sqrMagnitude < 0.0001f && smr != null && smr.sharedMesh != null)
                    {
                        Bounds localB = smr.sharedMesh.bounds;
                        Vector3 worldCenter = r.transform.TransformPoint(localB.center);
                        Vector3 worldSize = Vector3.Scale(localB.size, r.transform.lossyScale);
                        b = new Bounds(worldCenter, new Vector3(Mathf.Abs(worldSize.x), Mathf.Abs(worldSize.y), Mathf.Abs(worldSize.z)));
                    }
                    if (b.size.sqrMagnitude < 0.0001f) continue;

                    if (!hasBounds)
                    {
                        totalBounds = b;
                        hasBounds = true;
                    }
                    else
                    {
                        totalBounds.Encapsulate(b);
                    }
                }

                if (!hasBounds)
                {
                    if (dump != null && !_beatrixDumpLogged)
                    {
                        _beatrixDumpLogged = true;
                        CoopLog.Warn(dump.ToString() + " -> NO BOUNDS");
                    }
                    Object.Destroy(pivot);
                    return false;
                }

                // Align facing direction to +Z of parentRoot
                if (hasPortraitCam)
                {
                    Vector3 toCam = camPos - totalBounds.center;
                    toCam.y = 0f;
                    if (toCam.sqrMagnitude < 0.001f)
                    {
                        toCam = -camForward;
                        toCam.y = 0f;
                    }
                    if (toCam.sqrMagnitude > 0.001f)
                    {
                        float faceYaw = Quaternion.LookRotation(toCam.normalized).eulerAngles.y;
                        pivot.transform.rotation = Quaternion.Euler(0f, parentRoot.eulerAngles.y - faceYaw, 0f);
                    }
                }

                // Re-measure bounds after rotation and normalize height to ~1.68m standing on y=0
                hasBounds = false;
                for (int i = 0; i < renderers.Length; i++)
                {
                    Renderer r = renderers[i];
                    if (r == null || !r.enabled || !r.gameObject.activeInHierarchy) continue;
                    string rTypeName = r.GetType().Name;
                    if (rTypeName == "ParticleSystemRenderer" || rTypeName == "TrailRenderer" || rTypeName == "LineRenderer") continue;

                    Bounds b = r.bounds;
                    SkinnedMeshRenderer smr = r as SkinnedMeshRenderer;
                    if (b.size.sqrMagnitude < 0.0001f && smr != null && smr.sharedMesh != null)
                    {
                        Bounds localB = smr.sharedMesh.bounds;
                        Vector3 worldCenter = r.transform.TransformPoint(localB.center);
                        Vector3 worldSize = Vector3.Scale(localB.size, r.transform.lossyScale);
                        b = new Bounds(worldCenter, new Vector3(Mathf.Abs(worldSize.x), Mathf.Abs(worldSize.y), Mathf.Abs(worldSize.z)));
                    }
                    if (b.size.sqrMagnitude < 0.0001f) continue;

                    if (!hasBounds)
                    {
                        totalBounds = b;
                        hasBounds = true;
                    }
                    else
                    {
                        totalBounds.Encapsulate(b);
                    }
                }

                float scale = 1f;
                if (hasBounds && totalBounds.size.y > 0.05f)
                {
                    float targetHeight = 1.68f;
                    scale = Mathf.Clamp(targetHeight / totalBounds.size.y, 0.25f, 4.0f);
                    pivot.transform.localScale = new Vector3(scale, scale, scale);

                    Vector3 feetLocal = parentRoot.InverseTransformPoint(
                        new Vector3(totalBounds.center.x, totalBounds.min.y, totalBounds.center.z)) * scale;
                    pivot.transform.localPosition = -feetLocal;
                    labelY = targetHeight + 0.36f;
                }

                if (dump != null && !_beatrixDumpLogged)
                {
                    _beatrixDumpLogged = true;
                    dump.Append("hasCam=").Append(hasPortraitCam)
                        .Append(" boundsSize=").Append(totalBounds.size.ToString("F2"))
                        .Append(" scale=").Append(scale.ToString("F2"));
                    CoopLog.Info(dump.ToString());
                }

                return true;
            }
            catch (Exception ex)
            {
                CoopLog.Warn("TryBuildBeatrixVisual exception: " + ex.Message);
                if (pivot != null) Object.Destroy(pivot);
                return false;
            }
        }

        private static void DisableComponentIfPossible(Component c)
        {
            if (c == null) return;
            Behaviour b = c as Behaviour;
            if (b != null)
            {
                b.enabled = false;
                return;
            }
            Renderer r = c as Renderer;
            if (r != null)
            {
                r.enabled = false;
                return;
            }
            PropertyInfo pEnabled = c.GetType().GetProperty("enabled");
            if (pEnabled != null && pEnabled.CanWrite && pEnabled.PropertyType == typeof(bool))
            {
                pEnabled.SetValue(c, false, null);
            }
        }

        private static void BuildFallbackVisual(Transform parentRoot, Color accent, out Transform headTransform, out Transform vacTransform)
        {
            // Body capsule (no collider so it doesn't interfere with player/slime physics)
            CreatePart("Body", PrimitiveType.Capsule, parentRoot,
                new Vector3(0f, 0.85f, 0f), new Vector3(0.68f, 0.62f, 0.68f), accent);

            // Backpack
            CreatePart("Backpack", PrimitiveType.Cube, parentRoot,
                new Vector3(0f, 0.98f, -0.34f), new Vector3(0.46f, 0.56f, 0.24f), new Color(0.22f, 0.26f, 0.34f));

            // Head
            GameObject head = CreatePart("Head", PrimitiveType.Sphere, parentRoot,
                new Vector3(0f, 1.56f, 0f), new Vector3(0.62f, 0.58f, 0.62f), Color.Lerp(accent, Color.white, 0.2f));
            headTransform = head.transform;

            // Cute slime eyes on head
            CreatePart("EyeL", PrimitiveType.Sphere, head.transform,
                new Vector3(-0.20f, 0.08f, 0.44f), new Vector3(0.16f, 0.20f, 0.12f), new Color(0.08f, 0.08f, 0.12f));
            CreatePart("EyeR", PrimitiveType.Sphere, head.transform,
                new Vector3(0.20f, 0.08f, 0.44f), new Vector3(0.16f, 0.20f, 0.12f), new Color(0.08f, 0.08f, 0.12f));

            // Vacpack gun
            GameObject vac = CreatePart("VacPack", PrimitiveType.Cylinder, parentRoot,
                new Vector3(0.42f, 0.92f, 0.35f), new Vector3(0.20f, 0.45f, 0.20f), new Color(0.20f, 0.24f, 0.30f));
            vac.transform.localRotation = Quaternion.Euler(90f, 0f, 0f);
            vacTransform = vac.transform;
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
            Shader shader = ResolveSafeShader();
            if (r != null && shader != null)
            {
                Material mat = new Material(shader);
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
