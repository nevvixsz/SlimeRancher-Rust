using System;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine;

namespace SlimeCoop.Protocol
{
    [StructLayout(LayoutKind.Explicit)]
    internal struct FloatIntUnion
    {
        [FieldOffset(0)] public float FloatVal;
        [FieldOffset(0)] public int IntVal;
    }

    [StructLayout(LayoutKind.Explicit)]
    internal struct DoubleLongUnion
    {
        [FieldOffset(0)] public double DoubleVal;
        [FieldOffset(0)] public long LongVal;
    }

    internal enum PacketType : byte
    {
        // Transport & Discovery
        LanDiscoveryReq = 1,
        LanDiscoveryResp = 2,
        ReliableAck = 3,
        Ping = 4,
        Pong = 5,

        // Auth & Session
        Hello = 10,
        AuthChallenge = 11,
        JoinRequest = 12,
        JoinAccept = 13,
        JoinReject = 14,
        Disconnect = 15,
        PlayerList = 16,
        ChatMessage = 17,

        // Players (Unreliable high-freq)
        PlayerTransform = 20,

        // World Snapshot & Shared Ranch
        WorldSnapshotChunk = 30,
        WorldSnapshotEnd = 31,
        TimeWeatherSync = 32,
        RequestSnapshot = 33,

        // Ranch Plots
        PlotBuild = 40,
        PlotUpgrade = 41,
        PlotPlant = 42,
        SiloSlotSync = 43,
        PlotCollectorActivate = 44,
        ExchangeStateSync = 45,
        ExchangeAction = 46,
        ExchangeOpenChat = 47,
        ExchangeCrateBreak = 48,
        ExchangeDirectCoin = 49,

        // World Interactables
        DoorState = 50,
        GordoEatSync = 51,
        OasisSync = 52,
        PodSync = 53,
        GadgetPlace = 54,
        GadgetDestroy = 55,

        // Actors & Vacuum
        ActorSpawn = 60,
        ActorDestroy = 61,
        ActorBatchUpdate = 62,
        ClientVacCapture = 63,
        ClientExpelRequest = 64,
        MarketDepositNotice = 65
    }

    internal sealed class PacketWriter
    {
        private byte[] _buf;
        private int _pos;

        public PacketWriter(int capacity = 1400)
        {
            _buf = new byte[capacity];
        }

        public byte[] Buffer => _buf;
        public int Length => _pos;

        public void Reset()
        {
            _pos = 0;
        }

        private void Ensure(int extra)
        {
            if (_pos + extra <= _buf.Length) return;
            int newCap = Math.Max(_buf.Length * 2, _pos + extra + 256);
            byte[] next = new byte[newCap];
            System.Buffer.BlockCopy(_buf, 0, next, 0, _pos);
            _buf = next;
        }

        public void WriteByte(byte v)
        {
            Ensure(1);
            _buf[_pos++] = v;
        }

        public void WriteBool(bool v)
        {
            WriteByte(v ? (byte)1 : (byte)0);
        }

        public void WriteUShort(ushort v)
        {
            Ensure(2);
            _buf[_pos++] = (byte)(v & 0xFF);
            _buf[_pos++] = (byte)((v >> 8) & 0xFF);
        }

        public void WriteShort(short v)
        {
            WriteUShort((ushort)v);
        }

        public void WriteInt(int v)
        {
            Ensure(4);
            _buf[_pos++] = (byte)(v & 0xFF);
            _buf[_pos++] = (byte)((v >> 8) & 0xFF);
            _buf[_pos++] = (byte)((v >> 16) & 0xFF);
            _buf[_pos++] = (byte)((v >> 24) & 0xFF);
        }

        public void WriteLong(long v)
        {
            Ensure(8);
            ulong u = (ulong)v;
            for (int i = 0; i < 8; i++)
            {
                _buf[_pos++] = (byte)((u >> (i * 8)) & 0xFF);
            }
        }

        public void WriteFloat(float v)
        {
            FloatIntUnion u = default(FloatIntUnion);
            u.FloatVal = v;
            WriteInt(u.IntVal);
        }

        public void WriteDouble(double v)
        {
            DoubleLongUnion u = default(DoubleLongUnion);
            u.DoubleVal = v;
            WriteLong(u.LongVal);
        }

        public void WriteVector3(Vector3 v)
        {
            WriteFloat(v.x);
            WriteFloat(v.y);
            WriteFloat(v.z);
        }

        public void WriteQuaternion(Quaternion q)
        {
            WriteFloat(q.x);
            WriteFloat(q.y);
            WriteFloat(q.z);
            WriteFloat(q.w);
        }

        public void WriteString(string s)
        {
            if (string.IsNullOrEmpty(s))
            {
                WriteUShort(0);
                return;
            }
            if (s.Length > 20000) s = s.Substring(0, 20000);
            int byteCount = Encoding.UTF8.GetByteCount(s);
            Ensure(2 + byteCount);
            WriteUShort((ushort)byteCount);
            Encoding.UTF8.GetBytes(s, 0, s.Length, _buf, _pos);
            _pos += byteCount;
        }

        public void WriteBytes(byte[] src, int offset, int count)
        {
            Ensure(2 + count);
            WriteUShort((ushort)count);
            System.Buffer.BlockCopy(src, offset, _buf, _pos, count);
            _pos += count;
        }

        public byte[] ToArray()
        {
            byte[] copy = new byte[_pos];
            System.Buffer.BlockCopy(_buf, 0, copy, 0, _pos);
            return copy;
        }
    }

    internal struct PacketReader
    {
        private readonly byte[] _buf;
        private readonly int _len;
        private int _pos;

        public PacketReader(byte[] buf, int offset, int length)
        {
            _buf = buf;
            _pos = offset;
            _len = offset + length;
        }

        public int Remaining => _len - _pos;

        public byte ReadByte()
        {
            if (_pos >= _len) return 0;
            return _buf[_pos++];
        }

        public bool ReadBool()
        {
            return ReadByte() != 0;
        }

        public ushort ReadUShort()
        {
            if (_pos + 2 > _len) return 0;
            ushort v = (ushort)(_buf[_pos] | (_buf[_pos + 1] << 8));
            _pos += 2;
            return v;
        }

        public short ReadShort()
        {
            return (short)ReadUShort();
        }

        public int ReadInt()
        {
            if (_pos + 4 > _len) return 0;
            int v = _buf[_pos] | (_buf[_pos + 1] << 8) | (_buf[_pos + 2] << 16) | (_buf[_pos + 3] << 24);
            _pos += 4;
            return v;
        }

        public long ReadLong()
        {
            if (_pos + 8 > _len) return 0;
            ulong u = 0;
            for (int i = 0; i < 8; i++)
            {
                u |= ((ulong)_buf[_pos + i]) << (i * 8);
            }
            _pos += 8;
            return (long)u;
        }

        public float ReadFloat()
        {
            FloatIntUnion u = default(FloatIntUnion);
            u.IntVal = ReadInt();
            return u.FloatVal;
        }

        public double ReadDouble()
        {
            DoubleLongUnion u = default(DoubleLongUnion);
            u.LongVal = ReadLong();
            return u.DoubleVal;
        }

        public Vector3 ReadVector3()
        {
            return new Vector3(ReadFloat(), ReadFloat(), ReadFloat());
        }

        public Quaternion ReadQuaternion()
        {
            return new Quaternion(ReadFloat(), ReadFloat(), ReadFloat(), ReadFloat());
        }

        public string ReadString()
        {
            ushort len = ReadUShort();
            if (len == 0 || _pos + len > _len)
            {
                if (_pos + len > _len) _pos = _len;
                return string.Empty;
            }
            string s = Encoding.UTF8.GetString(_buf, _pos, len);
            _pos += len;
            return s;
        }

        public byte[] ReadBytes()
        {
            ushort len = ReadUShort();
            if (len == 0 || _pos + len > _len)
            {
                if (_pos + len > _len) _pos = _len;
                return new byte[0];
            }
            byte[] arr = new byte[len];
            Buffer.BlockCopy(_buf, _pos, arr, 0, len);
            _pos += len;
            return arr;
        }
    }
}
