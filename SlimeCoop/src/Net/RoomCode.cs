using System;
using System.Collections.Generic;
using System.Net;
using System.Net.NetworkInformation;
using System.Net.Sockets;
using System.Text;

namespace SlimeCoop.Net
{
    internal struct NetworkEndpointEntry
    {
        public string InterfaceName;
        public string Ip;
        public int Port;
        public string RoomCode;
        public bool IsVpn;
        public bool IsVirtualHypervisor;
    }

    internal static class RoomCode
    {
        // Standard Crockford Base32 (32 strictly unique chars: 0-9 + 22 letters, no I/L/O/U)
        private const string Alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        private static readonly int[] ReverseLookup = BuildReverse();

        private static int[] BuildReverse()
        {
            int[] map = new int[128];
            for (int i = 0; i < map.Length; i++) map[i] = -1;
            for (int i = 0; i < Alphabet.Length; i++)
            {
                char c = Alphabet[i];
                if (c < 128) map[c] = i;
            }
            // Friendly human-typing aliases
            map['O'] = map['0'];
            map['I'] = map['1'];
            map['L'] = map['1'];
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

        public static List<NetworkEndpointEntry> GetNetworkEndpoints(int port)
        {
            List<NetworkEndpointEntry> vpnList = new List<NetworkEndpointEntry>();
            List<NetworkEndpointEntry> lanList = new List<NetworkEndpointEntry>();
            List<NetworkEndpointEntry> virtualList = new List<NetworkEndpointEntry>();
            HashSet<string> seenIps = new HashSet<string>();

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
                    bool hasGateway = props.GatewayAddresses != null && props.GatewayAddresses.Count > 0;
                    bool isHypervisor = nameLower.Contains("vethernet") || nameLower.Contains("hyper-v") ||
                                        nameLower.Contains("wsl") || nameLower.Contains("docker") ||
                                        nameLower.Contains("vmware") || nameLower.Contains("virtualbox") ||
                                        nameLower.Contains("vbox") || nameLower.Contains("loopback") ||
                                        nameLower.Contains("bluetooth") || nameLower.Contains("npcap") ||
                                        nameLower.Contains("km-test");

                    bool isVpn = !isHypervisor && (
                                 nameLower.Contains("radmin") || nameLower.Contains("hamachi") ||
                                 nameLower.Contains("zerotier") || nameLower.Contains("tailscale") ||
                                 nameLower.Contains("wireguard") || nameLower.Contains("amnezia") ||
                                 nameLower.Contains("openvpn") || nameLower.Contains("vpn") ||
                                 nameLower.Contains("wintun") || nameLower.Contains("tap") ||
                                 nameLower.Contains("tun"));

                    foreach (UnicastIPAddressInformation uni in props.UnicastAddresses)
                    {
                        if (uni.Address.AddressFamily != AddressFamily.InterNetwork) continue;
                        byte[] bytes = uni.Address.GetAddressBytes();
                        string s = uni.Address.ToString();
                        if (s.StartsWith("169.254.") || !seenIps.Add(s)) continue;

                        bool isPrivate172 = bytes[0] == 172 && bytes[1] >= 16 && bytes[1] <= 31;
                        bool entryHypervisor = isHypervisor || (isPrivate172 && !hasGateway && !isVpn);
                        bool entryVpn = !entryHypervisor && (isVpn || bytes[0] == 26 || bytes[0] == 25 || bytes[0] == 10 || (bytes[0] == 100 && bytes[1] >= 64 && bytes[1] <= 127));

                        NetworkEndpointEntry entry = new NetworkEndpointEntry
                        {
                            InterfaceName = string.IsNullOrEmpty(nic.Name) ? nic.Description : nic.Name,
                            Ip = s,
                            Port = port,
                            RoomCode = Encode(s, port),
                            IsVpn = entryVpn,
                            IsVirtualHypervisor = entryHypervisor
                        };

                        if (entryHypervisor) virtualList.Add(entry);
                        else if (entryVpn) vpnList.Add(entry);
                        else if (hasGateway) lanList.Insert(0, entry);
                        else lanList.Add(entry);
                    }
                }
            }
            catch
            {
            }

            List<NetworkEndpointEntry> combined = new List<NetworkEndpointEntry>(vpnList.Count + lanList.Count + virtualList.Count);
            combined.AddRange(vpnList);
            combined.AddRange(lanList);
            combined.AddRange(virtualList);
            if (combined.Count == 0)
            {
                combined.Add(new NetworkEndpointEntry
                {
                    InterfaceName = "Localhost",
                    Ip = "127.0.0.1",
                    Port = port,
                    RoomCode = Encode("127.0.0.1", port)
                });
            }
            return combined;
        }

        public static List<IPAddress> GetSubnetBroadcastAddresses()
        {
            List<IPAddress> list = new List<IPAddress>();
            HashSet<string> seen = new HashSet<string>();

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

                    foreach (UnicastIPAddressInformation uni in props.UnicastAddresses)
                    {
                        if (uni.Address.AddressFamily != AddressFamily.InterNetwork) continue;
                        byte[] ipBytes = uni.Address.GetAddressBytes();
                        if (ipBytes[0] == 169 && ipBytes[1] == 254) continue;

                        byte[] maskBytes = uni.IPv4Mask != null ? uni.IPv4Mask.GetAddressBytes() : null;
                        if (maskBytes == null || maskBytes.Length != 4)
                        {
                            maskBytes = new byte[] { 255, 255, 255, 0 };
                        }

                        // Skip /32 point-to-point masks (nothing to broadcast to via mask)
                        if (maskBytes[0] == 255 && maskBytes[1] == 255 && maskBytes[2] == 255 && maskBytes[3] == 255)
                        {
                            continue;
                        }

                        byte[] bcast = new byte[4];
                        for (int b = 0; b < 4; b++)
                        {
                            bcast[b] = (byte)(ipBytes[b] | (~maskBytes[b]));
                        }

                        IPAddress bcastIp = new IPAddress(bcast);
                        string s = bcastIp.ToString();
                        if (seen.Add(s))
                        {
                            list.Add(bcastIp);
                        }
                    }
                }
            }
            catch
            {
            }

            return list;
        }
    }
}
