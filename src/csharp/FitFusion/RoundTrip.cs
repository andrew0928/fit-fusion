using System.Buffers.Binary;
using System.Security.Cryptography;
using System.Text;

static class RoundTrip
{
    static void U16(Stream stream, ushort value, bool big = false)
    {
        Span<byte> b = stackalloc byte[2];
        if (big) BinaryPrimitives.WriteUInt16BigEndian(b, value);
        else BinaryPrimitives.WriteUInt16LittleEndian(b, value);
        stream.Write(b);
    }

    static byte[] EncodeValue(object value, byte? type, bool big, ReadOnlySpan<byte> original)
    {
        int k = (type ?? 255) & 31;
        if (k == 7)
        {
            // The wire reader presents the first string. Keep binary padding and subsequent strings.
            int n = original.IndexOf((byte)0);
            if (n < 0) n = original.Length;
            byte[] first = Encoding.UTF8.GetBytes((string)value);
            if (!first.AsSpan().SequenceEqual(original[..n]))
                return original.ToArray(); // The reader represents malformed UTF-8 with replacement characters.
            return [.. first, .. original[n..]];
        }
        if (value is string opaque && (k >= WireValues.Names.Length || opaque is not ("nan" or "inf" or "-inf")))
            return Convert.FromHexString(opaque);
        using var bytes = new MemoryStream();
        foreach (var item in value as object[] ?? [value])
        {
            byte[] scalar;
            if (item is string special && special == "nan")
            {
                int width = k == 8 ? 4 : 8;
                bytes.Write(original.Slice((int)bytes.Length, width)); // Preserve NaN payload bits.
                continue;
            }
            scalar = k switch {
                0 or 2 or 10 or 13 => [(byte)item], 1 => [unchecked((byte)(sbyte)item)],
                3 => BitConverter.GetBytes((short)item), 4 or 11 => BitConverter.GetBytes((ushort)item),
                5 => BitConverter.GetBytes((int)item), 6 or 12 => BitConverter.GetBytes((uint)item),
                8 => BitConverter.GetBytes((float)FloatValue(item)), 9 => BitConverter.GetBytes(FloatValue(item)),
                14 => BitConverter.GetBytes((long)item), 15 or 16 => BitConverter.GetBytes((ulong)item),
                _ => throw new InvalidDataException($"Unsupported base type {k}")
            };
            if (big == BitConverter.IsLittleEndian) Array.Reverse(scalar);
            bytes.Write(scalar);
        }
        return bytes.ToArray();
    }

    static double FloatValue(object value) => value switch {
        "inf" => double.PositiveInfinity, "-inf" => double.NegativeInfinity, _ => (double)value
    };

    public static object Export(byte[] source, string destination)
    {
        using var output = new MemoryStream();
        using var body = new MemoryStream();
        var types = new Dictionary<(byte,byte),byte>();
        Frame? header = null;
        int members = 0, messages = 0;
        var expected = new Queue<Frame>();
        new WireReader(source).Read(frame =>
        {
            // Retain layout and payload slices for read-back without holding boxed scalar arrays.
            expected.Enqueue(frame with { Values = [] });
            switch (frame.Kind)
            {
                case "header":
                    header = frame;
                    types.Clear();
                    body.SetLength(0);
                    members++;
                    break;
                case "definition":
                    var layout = frame.Definition!;
                    int count = layout.Fields.Count(f => f.@base.HasValue);
                    int dev = layout.Fields.Count - count;
                    body.WriteByte((byte)(0x40 | frame.Local | (dev > 0 ? 0x20 : 0)));
                    body.WriteByte(0);
                    body.WriteByte((byte)(layout.Big ? 1 : 0));
                    U16(body, (ushort)layout.Number, layout.Big);
                    body.WriteByte((byte)count);
                    foreach (var f in layout.Fields.Take(count)) body.Write([f.id, f.size, f.@base!.Value]);
                    if (dev > 0)
                    {
                        body.WriteByte((byte)dev);
                        foreach (var f in layout.Fields.Skip(count)) body.Write([f.id, f.size, f.developer_index!.Value]);
                    }
                    break;
                case "data":
                    WriteMessage(body, frame, types);
                    messages++;
                    break;
                case "crc":
                    var original = header!.Bytes.Span;
                    var h = new byte[Math.Max(original.Length, 14)];
                    h[0] = (byte)h.Length;
                    h[1] = (byte)header.Protocol;
                    BinaryPrimitives.WriteUInt16LittleEndian(h.AsSpan(2), (ushort)header.Profile);
                    BinaryPrimitives.WriteUInt32LittleEndian(h.AsSpan(4), checked((uint)body.Length));
                    ".FIT"u8.CopyTo(h.AsSpan(8));
                    if (h.Length > 14) original[12..^2].CopyTo(h.AsSpan(12));
                    BinaryPrimitives.WriteUInt16LittleEndian(h.AsSpan(h.Length-2), WireReader.Crc(h.AsSpan(0,h.Length-2)));
                    output.Write(h);
                    var payload = body.GetBuffer().AsSpan(0, (int)body.Length);
                    output.Write(payload);
                    U16(output, WireReader.Crc(payload));
                    break;
            }
        });
        byte[] encoded = output.ToArray();
        new WireReader(encoded).Read(actual => {
            if (!expected.TryDequeue(out var original) || !SameContent(original, actual))
                throw new InvalidDataException($"Read-back differs at output byte {actual.Offset}");
        });
        if (expected.Count != 0) throw new InvalidDataException("Read-back frame count changed");
        var temp = Path.Combine(Path.GetDirectoryName(Path.GetFullPath(destination))!, Path.GetRandomFileName());
        var options = new FileStreamOptions { Mode = FileMode.CreateNew, Access = FileAccess.Write };
        if (!OperatingSystem.IsWindows()) options.UnixCreateMode = UnixFileMode.UserRead | UnixFileMode.UserWrite;
        using (var file = new FileStream(temp, options))
        {
            try { file.Write(encoded); file.Flush(); File.Move(temp, destination, overwrite:false); }
            finally { File.Delete(temp); }
        }
        return new { backend = "C# WireReader", output = destination, members, messages, semantic_equal = true,
            byte_equal = source.AsSpan().SequenceEqual(encoded), input_sha256 = Convert.ToHexStringLower(SHA256.HashData(source)),
            output_sha256 = Convert.ToHexStringLower(SHA256.HashData(encoded)) };
    }

    static bool SameContent(Frame a, Frame b)
    {
        if (a.Kind != b.Kind) return false;
        return a.Kind switch {
            "header" => a.Protocol == b.Protocol && a.Profile == b.Profile,
            "crc" => true, // WireReader validated both CRCs.
            "definition" => a.Local == b.Local && a.Definition!.Number == b.Definition!.Number &&
                a.Definition.Big == b.Definition.Big && a.Definition.Fields.SequenceEqual(b.Definition.Fields),
            "data" => a.Local == b.Local && a.TimeOffset == b.TimeOffset &&
                a.Definition!.Number == b.Definition!.Number && a.Bytes.Span[1..].SequenceEqual(b.Bytes.Span[1..]),
            _ => false
        };
    }

    static void WriteMessage(Stream body, Frame frame, Dictionary<(byte,byte),byte> types)
    {
        var layout = frame.Definition!;
        body.WriteByte((byte)(frame.TimeOffset is int t ? 128 | frame.Local << 5 | t : frame.Local));
        var native = new Dictionary<int,object>();
        int pos = 1;
        for (int i = 0; i < layout.Fields.Count; i++)
        {
            var f = layout.Fields[i];
            var original = frame.Bytes.Span.Slice(pos,f.size);
            pos += f.size;
            byte? type = f.@base;
            object value = frame.Values[i];
            if (f.developer_index is byte index)
            {
                type = types.TryGetValue((index,f.id),out var b) ? b : null;
                value = WireValues.Decode(original,type,layout.Big);
            }
            else if (layout.Number == 206) native[f.id] = value;
            var encoded = EncodeValue(value,type,layout.Big,original);
            if (!original.SequenceEqual(encoded))
                throw new InvalidDataException($"Field {f.id} changed at byte {frame.Offset}");
            body.Write(encoded);
        }
        if (layout.Number == 206 && native.GetValueOrDefault(0) is byte idx &&
            native.GetValueOrDefault(1) is byte id && native.GetValueOrDefault(2) is byte baseType)
            types[(idx,id)] = baseType;
    }
}
