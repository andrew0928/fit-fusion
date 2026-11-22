using System.Buffers.Binary;
using System.Text;
using System.Text.Json.Serialization;

record FieldDef(byte id, byte size,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] byte? @base,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] byte? developer_index);
record Layout(int Number, bool Big, List<FieldDef> Fields);

record Frame(int Offset, ReadOnlyMemory<byte> Bytes, string Kind)
{
    public int Protocol, Profile, BodySize, Local;
    public int? HeaderCrc, TimeOffset;
    public Layout? Definition;
    public object[] Values = [];

    public object Raw()
    {
        var result = new Dictionary<string, object?> {
            ["kind"] = Kind, ["offset"] = Offset, ["hex"] = Convert.ToHexStringLower(Bytes.Span)
        };
        if (Kind == "header")
        {
            result["protocol"] = Protocol;
            result["profile"] = Profile;
            result["body_size"] = BodySize;
            result["header_crc"] = HeaderCrc;
        }
        else if (Kind == "crc")
            result["crc"] = BinaryPrimitives.ReadUInt16LittleEndian(Bytes.Span);
        else
        {
            result["local"] = Local;
            result["global_id"] = Definition!.Number;
            if (Kind == "definition")
            {
                result["endian"] = Definition.Big ? ">" : "<";
                result["fields"] = Definition.Fields;
            }
            else
            {
                result["values"] = Values;
                if (TimeOffset.HasValue) result["timestamp_offset"] = TimeOffset;
            }
        }
        return result;
    }
}

class WireReader(byte[] bytes)
{
    int position, limit = bytes.Length;

    ReadOnlyMemory<byte> Take(int count)
    {
        if (count < 0 || count > limit - position)
            throw new InvalidDataException($"Truncated FIT frame at byte {position}");
        var data = bytes.AsMemory(position, count);
        position += count;
        return data;
    }
    byte Next() => Take(1).Span[0];
    Frame Finished(int start, string kind) => new(start, bytes.AsMemory(start, position-start), kind);

    static readonly ushort[] CrcTable = [
        0, 0xcc01, 0xd801, 0x1400, 0xf001, 0x3c00, 0x2800, 0xe401,
        0xa001, 0x6c00, 0x7800, 0xb401, 0x5000, 0x9c01, 0x8801, 0x4400];
    static ushort Crc(ReadOnlySpan<byte> data)
    {
        int crc = 0;
        foreach (var b in data)
        {
            crc = (crc >> 4) ^ CrcTable[(crc ^ b) & 15];
            crc = (crc >> 4) ^ CrcTable[(crc ^ (b >> 4)) & 15];
        }
        return (ushort)crc;
    }

    public void Read(Action<Frame> emit)
    {
        if (bytes.Length == 0) throw new InvalidDataException("Empty FIT file");
        while (position < bytes.Length)
        {
            int start = position;
            var header = Take(12).Span;
            int length = header[0];
            if (!header[8..].SequenceEqual(".FIT"u8) || (length != 12 && length < 14))
                throw new InvalidDataException($"Invalid FIT header at byte {start}");
            var extension = Take(length - 12);
            ushort? headerCrc = extension.IsEmpty ? null : BinaryPrimitives.ReadUInt16LittleEndian(extension.Span[^2..]);
            if (headerCrc is > 0 && Crc(bytes.AsSpan(start, length-2)) != headerCrc)
                throw new InvalidDataException($"Header CRC mismatch at byte {start}");
            uint size = BinaryPrimitives.ReadUInt32LittleEndian(header[4..]);
            if ((long)position + size + 2 > bytes.Length) throw new InvalidDataException("Truncated FIT member");
            limit = position + (int)size;
            if (Crc(bytes.AsSpan(start, limit-start)) != BinaryPrimitives.ReadUInt16LittleEndian(bytes.AsSpan(limit)))
                throw new InvalidDataException($"Member CRC mismatch at byte {limit}");
            emit(Finished(start, "header") with { Protocol = header[1], Profile = BinaryPrimitives.ReadUInt16LittleEndian(header[2..]),
                BodySize = (int)size, HeaderCrc = headerCrc });
            var locals = new Layout?[16];
            while (position < limit) emit(ReadRecord(locals));
            limit = bytes.Length;
            int footer = position;
            Take(2);
            emit(Finished(footer, "crc"));
        }
    }

    Frame ReadRecord(Layout?[] locals)
    {
        int start = position, h = Next();
        bool compressed = (h & 128) != 0;
        int local = compressed ? (h >> 5) & 3 : h & 15;
        if (!compressed && (h & 64) != 0)
        {
            Next(); // reserved
            int arch = Next();
            if (arch > 1) throw new InvalidDataException($"Invalid architecture at byte {start}");
            bool big = arch == 1;
            var number = WireValues.U16(Take(2).Span, big);
            var fields = new List<FieldDef>();
            int count = Next();
            for (int i = 0; i < count; i++) fields.Add(new(Next(), Next(), Next(), null));
            if ((h & 32) != 0)
            {
                count = Next();
                for (int i = 0; i < count; i++) fields.Add(new(Next(), Next(), null, Next()));
            }
            locals[local] = new(number, big, fields);
            return Finished(start, "definition") with { Local = local, Definition = locals[local] };
        }
        var layout = locals[local] ?? throw new InvalidDataException($"Undefined local message {local} at byte {start}");
        var values = new object[layout.Fields.Count];
        for (int i = 0; i < values.Length; i++)
        {
            var f = layout.Fields[i];
            values[i] = WireValues.Decode(Take(f.size).Span, f.@base, layout.Big);
        }
        return Finished(start, "data") with { Local = local, Definition = layout, Values = values,
            TimeOffset = compressed ? h & 31 : null };
    }
}

static class WireValues
{
    static readonly int[] Width = [1,1,1,2,2,4,4,1,4,8,1,2,4,1,8,8,8];
    public static readonly string[] Names = ["enum","sint8","uint8","sint16","uint16","sint32","uint32",
        "string","float32","float64","uint8z","uint16z","uint32z","byte","sint64","uint64","uint64z"];
    public static ushort U16(ReadOnlySpan<byte> b, bool big) =>
        big ? BinaryPrimitives.ReadUInt16BigEndian(b) : BinaryPrimitives.ReadUInt16LittleEndian(b);
    static uint U32(ReadOnlySpan<byte> b, bool big) =>
        big ? BinaryPrimitives.ReadUInt32BigEndian(b) : BinaryPrimitives.ReadUInt32LittleEndian(b);
    static ulong U64(ReadOnlySpan<byte> b, bool big) =>
        big ? BinaryPrimitives.ReadUInt64BigEndian(b) : BinaryPrimitives.ReadUInt64LittleEndian(b);
    static object Float(double value) => double.IsFinite(value) ? value :
        double.IsNaN(value) ? "nan" : value > 0 ? "inf" : "-inf";

    public static object Decode(ReadOnlySpan<byte> raw, byte? type, bool big)
    {
        int k = (type ?? 255) & 31;
        if (k >= Width.Length || raw.Length % Width[k] != 0) return Convert.ToHexStringLower(raw);
        if (k == 7)
        {
            int end = raw.IndexOf((byte)0);
            return Encoding.UTF8.GetString(end < 0 ? raw : raw[..end]);
        }
        int width = Width[k], count = raw.Length / width;
        if (count == 1) return Scalar(raw, k, big);
        var values = new object[count];
        for (int i = 0; i < count; i++) values[i] = Scalar(raw.Slice(i*width, width), k, big);
        return values;
    }

    static object Scalar(ReadOnlySpan<byte> b, int k, bool big) => k switch {
        0 or 2 or 10 or 13 => b[0], 1 => unchecked((sbyte)b[0]),
        3 => unchecked((short)U16(b,big)), 4 or 11 => U16(b,big),
        5 => unchecked((int)U32(b,big)), 6 or 12 => U32(b,big),
        8 => Float(BitConverter.UInt32BitsToSingle(U32(b,big))),
        9 => Float(BitConverter.UInt64BitsToDouble(U64(b,big))),
        14 => unchecked((long)U64(b,big)), 15 or 16 => U64(b,big),
        _ => throw new InvalidDataException($"Invalid FIT base type {k}")
    };

    public static bool Valid(object value, int k)
    {
        if (value is object[] a) return a.Any(v => Valid(v,k));
        return k switch {
            0 or 2 or 13 => !Equals(value, byte.MaxValue), 1 => !Equals(value, sbyte.MaxValue),
            3 => !Equals(value, short.MaxValue), 4 => !Equals(value, ushort.MaxValue),
            5 => !Equals(value, int.MaxValue), 6 => !Equals(value, uint.MaxValue),
            7 => value is string s && s.Length > 0, 8 or 9 => !Equals(value,"nan"),
            10 or 11 or 12 or 16 => value is string || Convert.ToDecimal(value) != 0,
            14 => !Equals(value, long.MaxValue), 15 => !Equals(value, ulong.MaxValue),
            _ => true
        };
    }
}
