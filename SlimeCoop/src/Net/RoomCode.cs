using System;
using System.Collections.Generic;
using System.Net;
using System.Net.NetworkInformation;
using System.Net.Sockets;
using System.Text;

namespace SlimeCoop.Net
{
    internal static class RoomCode
    {
        // Crockford-like readable 32-char alphabet (no 0/O, 1/I/L)
        private const string Alphabet = "ABCDEFGHJKMNPQRSTUVWXYZ23456789C";
        private static readonly int[] ReverseLookup = BuildReverse();

        private static int[] BuildReverse()
        {
            int[] map = new int[128];
            for (int i = 0; i < map.Length; i++) map[i] = -1;
            for (int i = 0; i < 32; i++)
            {
                char c = Alphabet[i];
                if (c < 128) map[c] = i;
            }
            // Friendly aliases
            map['0'] = map['O'] >= 0 ? map['O'] : 0;
            map['1'] = map['I'] >= 0 ? map['I'] : 0;
            return map;
        }

        public static string Encode(string ipv4, int port)
        {
            IPAddress ip;
            if (!IPAddress.TryParse(ipv4, out ip) || ip.AddressFamily != AddressFamily.InterNetwork)
            {
                return string.Empty;
            }

            byte[] ipBytes = ip.GetAddressBytes();
            ulong val = ((ulong)ipBytes[0] << 40) |
                        ((ulong)ipBytes[1] << 32) |
                        ((ulong)ipBytes[2] << 24) |
                        ((ulong)ipBytes[3] << 16) |
                        ((ulong)(ushort)port);

            // 48 bits -> 10 base32 chars (50 bits)
            char[] chars = new char[10];
            for (int i = 9; i >= 0; i--)
            {
                chars[i] = Alphabet[(int)(val & 31UL)];
                val >>= 5;
            }

            return string.Format("SLIME-{0}{1}{2}{3}-{4}{5}{6}{7}-{8}{9}",
                chars[0], chars[1], chars[2], chars[3],
                chars[4], chars[5], chars[6], chars[7],
                chars[8], chars[9]);
        }

        public static bool TryDecode(string code, out IPEndPoint endPoint)
        {
            endPoint = null;
            if (string.IsNullOrEmpty(code))
            {
                return false;
            }

            string s = code.Trim().ToUpperInvariant();
            if (s.StartsWith("SLIME-"))
            {
                s = s.Substring(6);
            }

            StringBuilder clean = new StringBuilder(10);
            for (int i = 0; i < s.Length; i++)
            {
                char c = s[i];
                if (c == '-' || c == ' ') continue;
                clean.Append(c);
            }

            if (clean.Length != 10)
            {
                return false;
            }

            ulong val = 0;
            for (int i = 0; i < 10; i++)
            {
                char c = clean[i];
                if (c >= 128 || ReverseLookup[c] < 0)
                {
                    return false;
                }
                val = (val << 5) | (uint)ReverseLookup[c];
            }

            byte[] ipBytes = new byte[4];
            ipBytes[0] = (byte)((val >> 40) & 0xFF);
            ipBytes[1] = (byte)((val >> 32) & 0xFF);
            ipBytes[2] = (byte)((val >> 24) & 0xFF);
            ipBytes[3] = (byte)((val >> 16) & 0xFF);
            int port = (int)(val & 0xFFFF);

            if (port <= 0) port = 7777;
            endPoint = new IPEndPoint(new IPAddress(ipBytes), port);
            return true;
        }

        public static bool TryParseEndpoint(string input, int defaultPort, out IPEndPoint endPoint)
        {
            endPoint = null;
            if (string.IsNullOrEmpty(input))
            {
                return false;
            }

            string trimmed = input.Trim();
            if (trimmed.StartsWith("SLIME-", StringComparison.OrdinalIgnoreCase))
            {
                return TryDecode(trimmed, out endPoint);
            }

            int port = defaultPort;
            string host = trimmed;
            int colon = trimmed.LastIndexOf(':');
            if (colon > 0 && colon < trimmed.Length - 1)
            {
                if (int.TryParse(trimmed.Substring(colon + 1), out int parsedPort) && parsedPort > 0 && parsedPort <= 65535)
                {
                    port = parsedPort;
                    host = trimmed.Substring(0, colon);
                }
            }

            if (string.Equals(host, "localhost", StringComparison.OrdinalIgnoreCase))
            {
                endPoint = new IPEndPoint(IPAddress.Loopback, port);
                return true;
            }

            IPAddress ip;
            if (IPAddress.TryParse(host, out ip) && ip.AddressFamily == AddressFamily.InterNetwork)
            {
                endPoint = new IPEndPoint(ip, port);
                return true;
            }

            try
            {
                IPAddress[] addrs = Dns.GetHostAddresses(host);
                for (int i = 0; i < addrs.Length; i++)
                {
                    if (addrs[i].AddressFamily == AddressFamily.InterNetwork)
                    {
                        endPoint = new IPEndPoint(addrs[i], port);
                        return true;
                    }
                }
            }
            catch
            {
            }

            return false;
        }

        public static List<string> GetLocalAndVpnAddresses()
        {
            List<string> preferredVpn = new List<string>();
            List<string> lan = new List<string>();

            try
            {
                NetworkInterface[] nics = NetworkInterface.GetAllNetworkInterfaces();
                for (int i = 0; i < nics.Length; i++)
                {
                    NetworkInterface nic = nics[i];
                    if (nic.OperationalStatus != OperationalStatus.Up) continue;
                    if (nic.NetworkInterfaceType == NetworkInterfaceType.Loopback) continue;

                    IPInterfaceProperties props = nic.GetIPProperties();
                    if (props == null) continue;

                    string nameLower = (nic.Name + " " + nic.Description).ToLowerInvariant();
                    bool isVpn = nameLower.Contains("radmin") || nameLower.Contains("hamachi") ||
                                 nameLower.Contains("zerotier") || nameLower.Contains("tailscale") ||
                                 nameLower.Contains("wireguard") || nameLower.Contains("tap") ||
                                 nameLower.Contains("tun");

                    foreach (UnicastIPAddressInformation uni in props.UnicastAddresses)
                    {
                        if (uni.Address.AddressFamily != AddressFamily.InterNetwork) continue;
                        string s = uni.Address.ToString();
                        if (s.StartsWith("169.254.")) continue;

                        if (isVpn || s.StartsWith("26.") || s.StartsWith("25."))
                        {
                            if (!preferredVpn.Contains(s)) preferredVpn.Add(s);
                        }
                        else
                        {
                            if (!lan.Contains(s)) lan.Add(s);
                        }
                    }
                }
            }
            catch
            {
            }

            List<string> combined = new List<string>(preferredVpn.Count + lan.Count);
            combined.AddRange(preferredVpn);
            combined.AddRange(lan);
            if (combined.Count == 0)
            {
                combined.Add("127.0.0.1");
            }
            return combined;
        }
    }
}
