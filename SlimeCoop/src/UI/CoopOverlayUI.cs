using System;
using System.Collections.Generic;
using MonomiPark.SlimeRancher.Regions;
using SlimeCoop.Net;
using SlimeCoop.World;
using UnityEngine;

namespace SlimeCoop.UI
{
    internal static class CoopOverlayUI
    {
        private static bool _isOpen;
        private static int _tab; // 0 = Join, 1 = Host, 2 = Session/Chat
        private static string _joinInput = "127.0.0.1:7777";
        private static string _joinPassword = string.Empty;
        private static string _hostPortStr = "7777";
        private static string _chatInput = string.Empty;
        private static Vector2 _chatScroll;
        private static Vector2 _lanScroll;

        private static bool _wasCursorForced;
        private static bool _capturedCursorState;
        private static bool _stylesInit;

        private static GUIStyle _winStyle;
        private static GUIStyle _titleStyle;
        private static GUIStyle _labelStyle;
        private static GUIStyle _mutedStyle;
        private static GUIStyle _btnStyle;
        private static GUIStyle _btnActiveStyle;
        private static GUIStyle _boxStyle;
        private static Texture2D _darkBgTex;
        private static Texture2D _panelTex;
        private static Texture2D _accentTex;

        private static readonly string[] ColorNames =
        {
            "Розовый", "Голубой", "Лайм", "Медовый",
            "Квантовый", "Мятный", "Коралл", "Кристалл"
        };

        public static bool IsOpen => _isOpen;

        public static void Toggle()
        {
            SetOpen(!_isOpen);
        }

        public static void SetOpen(bool open)
        {
            if (_isOpen == open) return;
            _isOpen = open;

            if (open && !CoopSession.IsActive)
            {
                CoopSession.RefreshLanDiscovery();
            }
            if (!open)
            {
                ClientProfileStore.SaveLocalPrefs();
            }

            SyncCursorLock(open);
        }

        private static void SyncCursorLock(bool open)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.Player == null) return;

            vp_FPInput fp = ctx.Player.GetComponentInChildren<vp_FPInput>();
            if (fp == null) return;

            if (open)
            {
                if (!_capturedCursorState)
                {
                    _wasCursorForced = fp.MouseCursorForced;
                    _capturedCursorState = true;
                }
                fp.MouseCursorForced = true;
                fp.AllowGameplayInput = false;
                vp_Utility.LockCursor = false;
                Cursor.lockState = CursorLockMode.None;
                Cursor.visible = true;
            }
            else if (_capturedCursorState)
            {
                fp.MouseCursorForced = _wasCursorForced;
                fp.AllowGameplayInput = true;
                vp_Utility.LockCursor = !_wasCursorForced;
                _capturedCursorState = false;
            }
        }

        private static void EnsureStyles()
        {
            if (_stylesInit) return;
            _stylesInit = true;

            _darkBgTex = MakeTex(new Color(0.08f, 0.10f, 0.16f, 0.94f));
            _panelTex = MakeTex(new Color(0.13f, 0.17f, 0.26f, 0.90f));
            _accentTex = MakeTex(new Color(0.22f, 0.56f, 0.86f, 0.95f));

            _winStyle = new GUIStyle(GUI.skin.box);
            _winStyle.normal.background = _darkBgTex;
            _winStyle.padding = new RectOffset(14, 14, 12, 12);

            _boxStyle = new GUIStyle(GUI.skin.box);
            _boxStyle.normal.background = _panelTex;
            _boxStyle.padding = new RectOffset(10, 10, 8, 8);

            _titleStyle = new GUIStyle(GUI.skin.label);
            _titleStyle.fontSize = 16;
            _titleStyle.fontStyle = FontStyle.Bold;
            _titleStyle.normal.textColor = new Color(0.75f, 0.94f, 1.0f);

            _labelStyle = new GUIStyle(GUI.skin.label);
            _labelStyle.fontSize = 13;
            _labelStyle.normal.textColor = new Color(0.92f, 0.95f, 0.98f);
            _labelStyle.wordWrap = true;

            _mutedStyle = new GUIStyle(GUI.skin.label);
            _mutedStyle.fontSize = 12;
            _mutedStyle.normal.textColor = new Color(0.68f, 0.76f, 0.86f);
            _mutedStyle.wordWrap = true;

            _btnStyle = new GUIStyle(GUI.skin.button);
            _btnStyle.fontSize = 13;
            _btnStyle.fontStyle = FontStyle.Bold;
            _btnStyle.normal.textColor = Color.white;

            _btnActiveStyle = new GUIStyle(_btnStyle);
            _btnActiveStyle.normal.background = _accentTex;
            _btnActiveStyle.hover.background = _accentTex;
        }

        private static Texture2D MakeTex(Color c)
        {
            Texture2D t = new Texture2D(1, 1);
            t.SetPixel(0, 0, c);
            t.Apply();
            return t;
        }

        public static void Draw()
        {
            if (!_isOpen) return;

            EnsureStyles();

            // Keep cursor unlocked while open
            if (SRSingleton<SceneContext>.Instance != null && Cursor.lockState != CursorLockMode.None)
            {
                SyncCursorLock(true);
            }

            float w = Mathf.Min(640f, Screen.width - 40f);
            float h = Mathf.Min(540f, Screen.height - 60f);
            Rect rect = new Rect((Screen.width - w) * 0.5f, (Screen.height - h) * 0.5f, w, h);

            GUILayout.BeginArea(rect, _winStyle);

            // Header
            GUILayout.BeginHorizontal();
            GUILayout.Label("SlimeCoop v1.0 — P2P Мультиплеер", _titleStyle);
            GUILayout.FlexibleSpace();
            if (GUILayout.Button("X", _btnStyle, GUILayout.Width(32f), GUILayout.Height(24f)))
            {
                SetOpen(false);
            }
            GUILayout.EndHorizontal();

            GUILayout.Label("Статус: " + CoopSession.StatusMessage, _mutedStyle);
            GUILayout.Space(6f);

            // Tabs
            GUILayout.BeginHorizontal();
            if (GUILayout.Button("Подключиться", _tab == 0 ? _btnActiveStyle : _btnStyle, GUILayout.Height(28f))) _tab = 0;
            if (GUILayout.Button("Создать сервер", _tab == 1 ? _btnActiveStyle : _btnStyle, GUILayout.Height(28f))) _tab = 1;
            if (GUILayout.Button("Игроки & Чат", _tab == 2 ? _btnActiveStyle : _btnStyle, GUILayout.Height(28f))) _tab = 2;
            GUILayout.EndHorizontal();

            GUILayout.Space(8f);

            if (_tab == 0) DrawJoinTab();
            else if (_tab == 1) DrawHostTab();
            else DrawSessionTab();

            GUILayout.EndArea();
        }

        private static void DrawProfileHeader()
        {
            GUILayout.BeginVertical(_boxStyle);
            GUILayout.BeginHorizontal();
            GUILayout.Label("Твой ник:", _labelStyle, GUILayout.Width(90f));
            CoopSession.LocalNick = GUILayout.TextField(CoopSession.LocalNick, 20, GUILayout.Width(190f));
            GUILayout.Space(12f);
            GUILayout.Label("Цвет:", _labelStyle, GUILayout.Width(45f));
            if (GUILayout.Button(ColorNames[Mathf.Abs(CoopSession.LocalColorIndex) % ColorNames.Length], _btnStyle, GUILayout.Width(110f)))
            {
                CoopSession.LocalColorIndex = (CoopSession.LocalColorIndex + 1) % ColorNames.Length;
                ClientProfileStore.SaveLocalPrefs();
            }
            GUILayout.EndHorizontal();
            GUILayout.Label("Общее ранчо, загоны, двери, Гордо. Апгрейды, Слаймопедия и достижения — личные у каждого.", _mutedStyle);
            GUILayout.EndVertical();
            GUILayout.Space(6f);
        }

        private static void DrawJoinTab()
        {
            DrawProfileHeader();

            GUILayout.BeginVertical(_boxStyle);
            GUILayout.Label("Подключение по Коду комнаты или IP:порт:", _labelStyle);
            GUILayout.BeginHorizontal();
            _joinInput = GUILayout.TextField(_joinInput, 64);
            if (GUILayout.Button("Вставить", _btnStyle, GUILayout.Width(90f)))
            {
                string clip = GUIUtility.systemCopyBuffer;
                if (!string.IsNullOrEmpty(clip)) _joinInput = clip.Trim();
            }
            GUILayout.EndHorizontal();

            GUILayout.Space(4f);
            GUILayout.BeginHorizontal();
            GUILayout.Label("Пароль (если нужен):", _labelStyle, GUILayout.Width(160f));
            _joinPassword = GUILayout.PasswordField(_joinPassword, '*', 32);
            GUILayout.EndHorizontal();

            GUILayout.Space(6f);
            GUILayout.BeginHorizontal();
            if (!CoopSession.IsActive)
            {
                if (GUILayout.Button("Подключиться", _btnActiveStyle, GUILayout.Height(30f)))
                {
                    try
                    {
                        CoopSession.ConnectTo(_joinInput, _joinPassword);
                    }
                    catch (Exception ex)
                    {
                        CoopSession.StatusMessage = "Ошибка: " + ex.Message;
                        ModMain.LogConsole?.LogError("SlimeCoop ConnectTo error: " + ex);
                    }
                }
            }
            else
            {
                if (GUILayout.Button("Отключиться", _btnStyle, GUILayout.Height(30f)))
                {
                    CoopSession.Disconnect();
                }
            }
            GUILayout.EndHorizontal();
            GUILayout.EndVertical();

            GUILayout.Space(6f);
            GUILayout.BeginVertical(_boxStyle);
            GUILayout.BeginHorizontal();
            GUILayout.Label("Серверы в локальной сети / Radmin VPN:", _labelStyle);
            GUILayout.FlexibleSpace();
            if (GUILayout.Button("Обновить", _btnStyle, GUILayout.Width(95f)))
            {
                CoopSession.RefreshLanDiscovery();
            }
            GUILayout.EndHorizontal();

            _lanScroll = GUILayout.BeginScrollView(_lanScroll, GUILayout.Height(140f));
            if (CoopSession.DiscoveredServers.Count == 0)
            {
                GUILayout.Label("Серверы не найдены. Нажми 'Обновить' или введи код комнаты / IP выше.", _mutedStyle);
            }
            else
            {
                for (int i = 0; i < CoopSession.DiscoveredServers.Count; i++)
                {
                    LanServerEntry srv = CoopSession.DiscoveredServers[i];
                    GUILayout.BeginHorizontal();
                    string info = string.Format("{0}  [{1}/{2}]  {3}  ({4})",
                        srv.RoomName, srv.Players, srv.MaxPlayers, srv.HasPassword ? "[Пароль]" : "[Открыт]", srv.EndPoint);
                    GUILayout.Label(info, _labelStyle);
                    if (GUILayout.Button("Войти", _btnActiveStyle, GUILayout.Width(80f)))
                    {
                        _joinInput = srv.EndPoint.ToString();
                        CoopSession.ConnectTo(_joinInput, _joinPassword);
                    }
                    GUILayout.EndHorizontal();
                }
            }
            GUILayout.EndScrollView();
            GUILayout.EndVertical();
        }

        private static void DrawHostTab()
        {
            DrawProfileHeader();

            GUILayout.BeginVertical(_boxStyle);
            GUILayout.BeginHorizontal();
            GUILayout.Label("Название:", _labelStyle, GUILayout.Width(90f));
            CoopSession.RoomName = GUILayout.TextField(CoopSession.RoomName, 32);
            GUILayout.EndHorizontal();

            GUILayout.BeginHorizontal();
            GUILayout.Label("UDP Порт:", _labelStyle, GUILayout.Width(90f));
            _hostPortStr = GUILayout.TextField(_hostPortStr, 6, GUILayout.Width(80f));
            GUILayout.Space(12f);
            GUILayout.Label("Макс. игроков:", _labelStyle, GUILayout.Width(105f));
            if (GUILayout.Button("-", _btnStyle, GUILayout.Width(28f))) CoopSession.MaxPlayers = Mathf.Max(2, CoopSession.MaxPlayers - 1);
            GUILayout.Label(CoopSession.MaxPlayers.ToString(), _labelStyle, GUILayout.Width(22f));
            if (GUILayout.Button("+", _btnStyle, GUILayout.Width(28f))) CoopSession.MaxPlayers = Mathf.Min(8, CoopSession.MaxPlayers + 1);
            GUILayout.EndHorizontal();

            GUILayout.BeginHorizontal();
            GUILayout.Label("Пароль:", _labelStyle, GUILayout.Width(90f));
            CoopSession.HostPassword = GUILayout.PasswordField(CoopSession.HostPassword, '*', 32);
            GUILayout.EndHorizontal();

            CoopSession.UseUpnp = GUILayout.Toggle(CoopSession.UseUpnp, " Авто-проброс порта на роутере (UPnP)");
            CoopSession.ShareMarketIncome = GUILayout.Toggle(CoopSession.ShareMarketIncome, " Начислять выручку с плорт-маркета всем игрокам (свои деньги у каждого)");

            GUILayout.Space(6f);
            if (!CoopSession.IsHost)
            {
                if (GUILayout.Button("Запустить сервер", _btnActiveStyle, GUILayout.Height(32f)))
                {
                    try
                    {
                        int port = 7777;
                        if (!int.TryParse(_hostPortStr, out port) || port < 1024 || port > 65535)
                        {
                            port = 7777;
                        }
                        if (CoopSession.StartHost(port, CoopSession.HostPassword, CoopSession.RoomName, CoopSession.MaxPlayers, CoopSession.UseUpnp))
                        {
                            _tab = 1;
                        }
                    }
                    catch (Exception ex)
                    {
                        CoopSession.StatusMessage = "Ошибка: " + ex.Message;
                        ModMain.LogConsole?.LogError("SlimeCoop StartHost error: " + ex);
                    }
                }
            }
            else
            {
                if (GUILayout.Button("Остановить сервер", _btnStyle, GUILayout.Height(32f)))
                {
                    CoopSession.Disconnect();
                }
            }
            GUILayout.EndVertical();

            if (CoopSession.IsHost)
            {
                GUILayout.Space(6f);
                GUILayout.BeginVertical(_boxStyle);
                string primaryCode = CoopSession.GetPrimaryRoomCode();
                GUILayout.Label("Главный Код комнаты:  " + primaryCode, _titleStyle);
                GUILayout.BeginHorizontal();
                if (GUILayout.Button("Скопировать Код комнаты", _btnActiveStyle, GUILayout.Height(28f)))
                {
                    GUIUtility.systemCopyBuffer = primaryCode;
                    CoopSession.AddChat("Система", "Код комнаты скопирован в буфер: " + primaryCode, true);
                }
                GUILayout.EndHorizontal();

                if (CoopSession.HostRoomCodes.Count > 1)
                {
                    GUILayout.Label("Локальные/VPN коды: " + string.Join("  |  ", CoopSession.HostRoomCodes.ToArray()), _mutedStyle);
                }
                GUILayout.Label("UPnP статус: " + UpnpMapper.StatusText, _mutedStyle);
                GUILayout.EndVertical();
            }
        }

        private static void DrawSessionTab()
        {
            GUILayout.BeginVertical(_boxStyle);
            GUILayout.Label("Игроки в сессии:", _titleStyle);
            if (!CoopSession.IsActive)
            {
                GUILayout.Label("Вы не в кооперативной игре.", _mutedStyle);
            }
            else
            {
                GUILayout.Label(string.Format("• {0} (Вы) — {1}", CoopSession.LocalNick, CoopSession.IsHost ? "Хост" : "Клиент"), _labelStyle);
                for (int i = 0; i < CoopSession.ConnectedPeers.Count; i++)
                {
                    CoopPeerInfo p = CoopSession.ConnectedPeers[i];
                    if (p.PlayerId == CoopSession.LocalPlayerId) continue;

                    GUILayout.BeginHorizontal();
                    GUILayout.Label(string.Format("• {0}  [{1}]", p.Name, p.CurrentZone), _labelStyle);
                    GUILayout.FlexibleSpace();
                    if (GUILayout.Button("ТП к игроку", _btnStyle, GUILayout.Width(105f)))
                    {
                        TeleportToRemotePlayer(p.PlayerId);
                    }
                    GUILayout.EndHorizontal();
                }
            }
            GUILayout.EndVertical();

            GUILayout.Space(6f);
            GUILayout.BeginVertical(_boxStyle);
            GUILayout.Label("Чат:", _labelStyle);
            _chatScroll = GUILayout.BeginScrollView(_chatScroll, GUILayout.Height(180f));
            for (int i = 0; i < CoopSession.ChatLog.Count; i++)
            {
                CoopChatEntry msg = CoopSession.ChatLog[i];
                GUILayout.Label(msg.IsSystem ? ("* " + msg.Text) : (msg.Sender + ": " + msg.Text),
                    msg.IsSystem ? _mutedStyle : _labelStyle);
            }
            GUILayout.EndScrollView();

            GUILayout.BeginHorizontal();
            _chatInput = GUILayout.TextField(_chatInput, 180);
            if (GUILayout.Button("Отправить", _btnActiveStyle, GUILayout.Width(95f)) ||
                (Event.current.type == EventType.KeyDown && Event.current.keyCode == KeyCode.Return && !string.IsNullOrEmpty(_chatInput)))
            {
                CoopSession.SendChatMessage(_chatInput);
                _chatInput = string.Empty;
            }
            GUILayout.EndHorizontal();
            GUILayout.EndVertical();
        }

        internal static void TeleportToRemotePlayer(int playerId)
        {
            SceneContext ctx = SRSingleton<SceneContext>.Instance;
            if (ctx == null || ctx.Player == null) return;
            if (!RemotePlayerManager.TryGetPlayerPosition(playerId, out Vector3 pos)) return;

            TeleportablePlayer tp = ctx.Player.GetComponent<TeleportablePlayer>();
            RegionMember rm = ctx.Player.GetComponent<RegionMember>();
            RegionRegistry.RegionSetId setId = rm != null ? rm.setId : RegionRegistry.RegionSetId.HOME;

            if (tp != null)
            {
                tp.TeleportTo(pos + Vector3.up * 1.5f, setId, null, false, true);
            }
            else
            {
                ctx.Player.transform.position = pos + Vector3.up * 1.5f;
            }
        }
    }
}
