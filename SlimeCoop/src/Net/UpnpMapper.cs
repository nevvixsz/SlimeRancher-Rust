using System;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;

namespace SlimeCoop.Net
{
    internal static class UpnpMapper
    {
        public static volatile string StatusText = "Не запускался";
        public static volatile string ExternalIp = string.Empty;
        private static string _controlUrl;
        private static string _serviceType;
        private static int _mappedPort;

        public static void OpenPortAsync(int port)
        {
            StatusText = "Поиск роутера (UPnP)...";
            ExternalIp = string.Empty;
            _mappedPort = port;

            ThreadPool.QueueUserWorkItem(_ =>
            {
                try
                {
                    if (!DiscoverGateway(out string ctrlUrl, out string srvType))
                    {
                        StatusText = "UPnP роутер не найден (используй Radmin VPN / LAN / прямой IP)";
                        return;
                    }

                    _controlUrl = ctrlUrl;
                    _serviceType = srvType;

                    string localIp = GetLocalIpv4ToGateway();
                    if (string.IsNullOrEmpty(localIp))
                    {
                        StatusText = "Не определён локальный IPv4";
                        return;
                    }

                    string extIp = QueryExternalIp(ctrlUrl, srvType);
                    bool mapped = AddPortMapping(ctrlUrl, srvType, localIp, port, "UDP", "SlimeCoop P2P");
                    if (mapped)
                    {
                        ExternalIp = extIp ?? string.Empty;
                        StatusText = string.IsNullOrEmpty(ExternalIp)
                            ? ("UPnP порт " + port + " открыт")
                            : ("UPnP OK -> " + ExternalIp + ":" + port);
                    }
                    else
                    {
                        StatusText = "Роутер отклонил UPnP (используй Radmin VPN или пробрось UDP " + port + ")";
                    }
                }
                catch (Exception ex)
                {
                    StatusText = "UPnP ошибка: " + ex.Message;
                }
            });
        }

        public static void ClosePortAsync()
        {
            string ctrl = _controlUrl;
            string srv = _serviceType;
            int port = _mappedPort;
            _controlUrl = null;
            if (string.IsNullOrEmpty(ctrl) || port <= 0)
            {
                return;
            }

            ThreadPool.QueueUserWorkItem(_ =>
            {
                try
                {
                    string body =
                        "<u:DeletePortMapping xmlns:u=\"" + srv + "\">" +
                        "<NewRemoteHost></NewRemoteHost>" +
                        "<NewExternalPort>" + port + "</NewExternalPort>" +
                        "<NewProtocol>UDP</NewProtocol>" +
                        "</u:DeletePortMapping>";
                    SendSoap(ctrl, srv, "DeletePortMapping", body);
                }
                catch
                {
                }
            });
        }

        private static bool DiscoverGateway(out string controlUrl, out string serviceType)
        {
            controlUrl = null;
            serviceType = null;

            string req =
                "M-SEARCH * HTTP/1.1\r\n" +
                "HOST: 239.255.255.250:1900\r\n" +
                "ST: urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\n" +
                "MAN: \"ssdp:discover\"\r\n" +
                "MX: 2\r\n\r\n";

            byte[] reqBytes = Encoding.ASCII.GetBytes(req);
            using (UdpClient udp = new UdpClient())
            {
                udp.Client.ReceiveTimeout = 2200;
                IPEndPoint mcast = new IPEndPoint(IPAddress.Parse("239.255.255.250"), 1900);
                udp.Send(reqBytes, reqBytes.Length, mcast);
                udp.Send(reqBytes, reqBytes.Length, mcast);

                DateTime deadline = DateTime.UtcNow.AddMilliseconds(2300);
                while (DateTime.UtcNow < deadline)
                {
                    try
                    {
                        IPEndPoint from = null;
                        byte[] resp = udp.Receive(ref from);
                        string text = Encoding.ASCII.GetString(resp);
                        string loc = ExtractHeader(text, "LOCATION:");
                        if (!string.IsNullOrEmpty(loc) && TryParseDeviceXml(loc, out controlUrl, out serviceType))
                        {
                            return true;
                        }
                    }
                    catch
                    {
                        break;
                    }
                }
            }

            return false;
        }

        private static string ExtractHeader(string text, string headerName)
        {
            string[] lines = text.Split(new[] { "\r\n", "\n" }, StringSplitOptions.RemoveEmptyEntries);
            for (int i = 0; i < lines.Length; i++)
            {
                if (lines[i].StartsWith(headerName, StringComparison.OrdinalIgnoreCase))
                {
                    return lines[i].Substring(headerName.Length).Trim();
                }
            }
            return null;
        }

        private static bool TryParseDeviceXml(string locationUrl, out string controlUrl, out string serviceType)
        {
            controlUrl = null;
            serviceType = null;

            HttpWebRequest req = (HttpWebRequest)WebRequest.Create(locationUrl);
            req.Timeout = 2500;
            using (HttpWebResponse resp = (HttpWebResponse)req.GetResponse())
            using (StreamReader sr = new StreamReader(resp.GetResponseStream()))
            {
                string xml = sr.ReadToEnd();
                string[] candidates =
                {
                    "urn:schemas-upnp-org:service:WANIPConnection:1",
                    "urn:schemas-upnp-org:service:WANIPConnection:2",
                    "urn:schemas-upnp-org:service:WANPPPConnection:1"
                };

                for (int c = 0; c < candidates.Length; c++)
                {
                    int idx = xml.IndexOf(candidates[c], StringComparison.OrdinalIgnoreCase);
                    if (idx < 0) continue;

                    int endService = xml.IndexOf("</service>", idx, StringComparison.OrdinalIgnoreCase);
                    if (endService < 0) endService = xml.Length;
                    string slice = xml.Substring(idx, endService - idx);

                    string ctrl = ExtractTag(slice, "controlURL");
                    if (!string.IsNullOrEmpty(ctrl))
                    {
                        Uri baseUri = new Uri(locationUrl);
                        Uri full = new Uri(baseUri, ctrl);
                        controlUrl = full.ToString();
                        serviceType = candidates[c];
                        return true;
                    }
                }
            }

            return false;
        }

        private static string ExtractTag(string xml, string tag)
        {
            string open = "<" + tag + ">";
            string close = "</" + tag + ">";
            int a = xml.IndexOf(open, StringComparison.OrdinalIgnoreCase);
            if (a < 0) return null;
            a += open.Length;
            int b = xml.IndexOf(close, a, StringComparison.OrdinalIgnoreCase);
            if (b < 0) return null;
            return xml.Substring(a, b - a).Trim();
        }

        private static bool AddPortMapping(string controlUrl, string serviceType, string internalIp, int port, string proto, string desc)
        {
            string body =
                "<u:AddPortMapping xmlns:u=\"" + serviceType + "\">" +
                "<NewRemoteHost></NewRemoteHost>" +
                "<NewExternalPort>" + port + "</NewExternalPort>" +
                "<NewProtocol>" + proto + "</NewProtocol>" +
                "<NewInternalPort>" + port + "</NewInternalPort>" +
                "<NewInternalClient>" + internalIp + "</NewInternalClient>" +
                "<NewEnabled>1</NewEnabled>" +
                "<NewPortMappingDescription>" + desc + "</NewPortMappingDescription>" +
                "<NewLeaseDuration>0</NewLeaseDuration>" +
                "</u:AddPortMapping>";
            string res = SendSoap(controlUrl, serviceType, "AddPortMapping", body);
            return res != null;
        }

        private static string QueryExternalIp(string controlUrl, string serviceType)
        {
            try
            {
                string body = "<u:GetExternalIPAddress xmlns:u=\"" + serviceType + "\"></u:GetExternalIPAddress>";
                string res = SendSoap(controlUrl, serviceType, "GetExternalIPAddress", body);
                return res != null ? ExtractTag(res, "NewExternalIPAddress") : null;
            }
            catch
            {
                return null;
            }
        }

        private static string SendSoap(string controlUrl, string serviceType, string action, string innerBody)
        {
            string envelope =
                "<?xml version=\"1.0\"?>" +
                "<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\">" +
                "<s:Body>" + innerBody + "</s:Body></s:Envelope>";

            byte[] bytes = Encoding.UTF8.GetBytes(envelope);
            HttpWebRequest req = (HttpWebRequest)WebRequest.Create(controlUrl);
            req.Method = "POST";
            req.Timeout = 2500;
            req.ContentType = "text/xml; charset=\"utf-8\"";
            req.Headers.Add("SOAPACTION", "\"" + serviceType + "#" + action + "\"");
            req.ContentLength = bytes.Length;

            using (Stream st = req.GetRequestStream())
            {
                st.Write(bytes, 0, bytes.Length);
            }

            using (HttpWebResponse resp = (HttpWebResponse)req.GetResponse())
            using (StreamReader sr = new StreamReader(resp.GetResponseStream()))
            {
                return sr.ReadToEnd();
            }
        }

        private static string GetLocalIpv4ToGateway()
        {
            try
            {
                using (Socket s = new Socket(AddressFamily.InterNetwork, SocketType.Dgram, 0))
                {
                    s.Connect("8.8.8.8", 65530);
                    IPEndPoint ep = s.LocalEndPoint as IPEndPoint;
                    return ep != null ? ep.Address.ToString() : null;
                }
            }
            catch
            {
                return null;
            }
        }
    }
}
