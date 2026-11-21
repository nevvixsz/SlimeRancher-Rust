using SlimeCoop.UI;
using SRML.Console;

namespace SlimeCoop.Commands
{
    internal sealed class CoopCommand : ConsoleCommand
    {
        private readonly string _id;

        public CoopCommand(string id = "coop")
        {
            _id = id;
        }

        public override string ID => _id;
        public override string Usage => _id + " <ui | host [port] [pass] | join <code|ip:port> [pass] | leave | say <msg> | tp <nick>>";
        public override string Description => "Управление P2P кооперативом SlimeCoop (хост, подключение, чат, телепорт к другу).";

        public override bool Execute(string[] args)
        {
            if (args == null || args.Length == 0)
            {
                CoopOverlayUI.Toggle();
                return true;
            }

            string sub = args[0].Trim().ToLowerInvariant();
            if (sub == "ui" || sub == "окно" || sub == "menu")
            {
                CoopOverlayUI.Toggle();
                return true;
            }

            if (sub == "host" || sub == "хост")
            {
                int port = 7777;
                if (args.Length >= 2) int.TryParse(args[1], out port);
                string pass = args.Length >= 3 ? args[2] : string.Empty;
                bool ok = CoopSession.StartHost(port, pass, CoopSession.RoomName, CoopSession.MaxPlayers, true);
                ModMain.LogConsole?.Log(ok
                    ? ("Сервер запущен! Код: " + CoopSession.GetPrimaryRoomCode())
                    : CoopSession.StatusMessage);
                return true;
            }

            if (sub == "join" || sub == "войти")
            {
                if (args.Length < 2)
                {
                    ModMain.LogConsole?.LogError("Укажите код комнаты или IP:порт.");
                    return true;
                }
                string pass = args.Length >= 3 ? args[2] : string.Empty;
                CoopSession.ConnectTo(args[1], pass);
                ModMain.LogConsole?.Log(CoopSession.StatusMessage);
                return true;
            }

            if (sub == "leave" || sub == "выход")
            {
                CoopSession.Disconnect();
                ModMain.LogConsole?.Log("Отключено от кооператива.");
                return true;
            }

            if (sub == "say" || sub == "чат")
            {
                if (args.Length >= 2)
                {
                    CoopSession.SendChatMessage(string.Join(" ", args, 1, args.Length - 1));
                }
                return true;
            }

            if (sub == "tp" || sub == "тп")
            {
                if (args.Length >= 2)
                {
                    string q = args[1].ToLowerInvariant();
                    for (int i = 0; i < CoopSession.ConnectedPeers.Count; i++)
                    {
                        CoopPeerInfo p = CoopSession.ConnectedPeers[i];
                        if (p.PlayerId != CoopSession.LocalPlayerId &&
                            !string.IsNullOrEmpty(p.Name) &&
                            p.Name.ToLowerInvariant().Contains(q))
                        {
                            CoopOverlayUI.TeleportToRemotePlayer(p.PlayerId);
                            ModMain.LogConsole?.Log("Телепорт к игроку " + p.Name);
                            return true;
                        }
                    }
                }
                ModMain.LogConsole?.LogError("Игрок не найден.");
                return true;
            }

            ModMain.LogConsole?.Log("Использование: " + Usage);
            return true;
        }
    }
}
