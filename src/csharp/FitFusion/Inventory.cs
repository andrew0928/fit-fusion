using System.Reflection;

record Label(string Name, string? Units);
record DevLabel(string? Name, string? Units, byte Type);
record FieldKey(int Number, byte? Index, string Name, string? Units, string TypeName, bool Unknown);
class FieldStats(FieldKey key)
{
    public long Count;
    public readonly SortedSet<int> Sizes = [];
    public readonly Timing Timing = new();
    public object Report(long rows) => new {
        number = key.Number, name = key.Name, units = key.Units, base_type = key.TypeName,
        unknown = key.Unknown, developer_data_index = key.Index, encoded_sizes_bytes = Sizes,
        sample_count = Count, coverage_pct = rows == 0 ? (double?)null : 100.0 * Count / rows,
        timing = Timing.Report()
    };
}

class Timing
{
    static readonly DateTime Epoch = new(1989,12,31,0,0,0,DateTimeKind.Utc);
    long? first, last, previous;
    long count, duplicates, backwards;
    readonly SortedDictionary<long,long> intervals = [];
    public void Add(long? timestamp)
    {
        if (timestamp is not long t) return;
        count++;
        first = Math.Min(first ?? t, t);
        last = Math.Max(last ?? t, t);
        if (previous is long prev)
        {
            long delta = t - prev;
            if (delta == 0) duplicates++;
            else if (delta < 0) backwards++;
            else intervals[delta] = intervals.GetValueOrDefault(delta) + 1;
        }
        previous = t;
    }

    long At(long index)
    {
        foreach (var (delta, frequency) in intervals)
        {
            if (index < frequency) return delta;
            index -= frequency;
        }
        throw new InvalidOperationException("Empty interval histogram");
    }
    static string? Utc(long? seconds) => seconds is long s
        ? Epoch.AddSeconds(s).ToString("yyyy-MM-dd'T'HH:mm:ss'Z'", System.Globalization.CultureInfo.InvariantCulture) : null;
    public object Report()
    {
        long n = intervals.Values.Sum();
        double? median = n == 0 ? null : (At((n-1)/2) + At(n/2)) / 2.0;
        return new { timestamped_count = count, start_utc = Utc(first), end_utc = Utc(last),
            span_s = last-first, min_interval_s = n == 0 ? (long?)null : intervals.First().Key,
            median_interval_s = median, max_interval_s = n == 0 ? (long?)null : intervals.Last().Key,
            typical_hz = 1.0/median, duplicate_intervals = duplicates, backward_intervals = backwards };
    }
}

class MessageStats
{
    public long Rows;
    public readonly Timing Timing = new();
    public readonly Dictionary<FieldKey,FieldStats> Fields = [];
}

class Inventory
{
    public readonly List<object> Members = [];
    readonly Dictionary<int,string> names = [];
    readonly Dictionary<(int,int),Label> labels = [];
    readonly SortedDictionary<int,MessageStats> messages = [];
    readonly Dictionary<(byte,byte),DevLabel> developers = [];
    // Bind fields at definition time, including repeated local definition changes.
    readonly Dictionary<int,(FieldStats Stats, byte? Type)[]> bindings = [];
    Frame? header;
    long? clock;

    public Inventory()
    {
        using var stream = Assembly.GetExecutingAssembly().GetManifestResourceStream("profile.tsv")!;
        using var reader = new StreamReader(stream);
        while (reader.ReadLine() is string line)
        {
            if (line.Length == 0 || line.StartsWith('#')) continue;
            var parts = line.Split('\t');
            int message = int.Parse(parts[0]);
            if (parts[2] == "") names.Add(message,parts[1]);
            else labels.Add((message,int.Parse(parts[2])), new(parts[3],parts[4] == "" ? null : parts[4]));
        }
    }

    public void Observe(Frame frame)
    {
        switch (frame.Kind)
        {
            case "header":
                header = frame;
                messages.Clear();
                developers.Clear();
                bindings.Clear();
                clock = null;
                break;
            case "definition": Register(frame); break;
            case "data": ObserveData(frame); break;
            case "crc":
                Members.Add(new {
                    protocol_version = $"{header!.Protocol >> 4}.{header.Protocol & 15}",
                    profile_version = $"{header.Profile / 100}.{header.Profile % 100:D2}",
                    messages = messages.Select(pair => new {
                        number = pair.Key, name = names.GetValueOrDefault(pair.Key, $"unknown_{pair.Key}"),
                        unknown = !names.ContainsKey(pair.Key), message_count = pair.Value.Rows,
                        timing = pair.Value.Timing.Report(),
                        fields = pair.Value.Fields.Values.Select(f => f.Report(pair.Value.Rows)).ToArray()
                    }).ToArray()
                });
                break;
        }
    }

    void Register(Frame frame)
    {
        var layout = frame.Definition!;
        if (!messages.TryGetValue(layout.Number, out var msg)) messages.Add(layout.Number, msg = new());
        var fields = new List<(FieldStats,byte?)>();
        foreach (var f in layout.Fields)
        {
            byte? type = f.@base;
            string? name, units;
            if (f.developer_index is byte index)
            {
                var dev = developers.GetValueOrDefault((index,f.id));
                name = dev?.Name;
                units = dev?.Units;
                type = dev?.Type;
            }
            else
            {
                var label = labels.GetValueOrDefault((layout.Number, f.id));
                name = label?.Name;
                units = label?.Units;
            }
            int k = (type ?? 13) & 31;
            var typeName = k < WireValues.Names.Length ? WireValues.Names[k] : $"unknown_{type}";
            var key = new FieldKey(f.id, f.developer_index, name ?? $"unknown_{f.id}", units, typeName, name == null);
            if (!msg.Fields.TryGetValue(key,out var stats)) msg.Fields.Add(key,stats = new(key));
            stats.Sizes.Add(f.size);
            fields.Add((stats,type));
        }
        bindings[frame.Local] = fields.ToArray();
    }

    void ObserveData(Frame frame)
    {
        var layout = frame.Definition!;
        var msg = messages[layout.Number];
        long? timestamp = null;
        for (int i = 0; i < layout.Fields.Count; i++)
        {
            if (layout.Fields[i] is { id: 253, developer_index: null } &&
                frame.Values[i] is byte or sbyte or short or ushort or int or uint or long or ulong)
            {
                decimal t = Convert.ToDecimal(frame.Values[i]);
                if (t >= 0 && t < uint.MaxValue) timestamp = (long)t;
            }
        }
        if (frame.TimeOffset is int offset && clock is long anchor)
            timestamp = anchor + ((offset - (anchor & 31) + 32) % 32);
        if (timestamp.HasValue) clock = timestamp;
        if (timestamp < 0x10000000 || timestamp >= uint.MaxValue) timestamp = null;
        msg.Rows++;
        msg.Timing.Add(timestamp);
        var populated = new HashSet<FieldStats>();
        for (int i = 0; i < layout.Fields.Count; i++)
        {
            var (stats,type) = bindings[frame.Local][i];
            var value = frame.Values[i];
            if (layout.Fields[i].developer_index.HasValue)
                value = WireValues.Decode(Convert.FromHexString((string)value),type ?? 13,layout.Big);
            if (WireValues.Valid(value,(type ?? 13) & 31) && populated.Add(stats))
            {
                stats.Count++;
                stats.Timing.Add(timestamp);
            }
        }
        if (layout.Number == 206)
        {
            object? Field(int id)
            {
                int i = layout.Fields.FindIndex(f => f.id == id && f.developer_index == null);
                return i >= 0 ? frame.Values[i] : null;
            }
            if (Field(0) is byte index && Field(1) is byte number && Field(2) is byte type)
            {
                string? name = Field(3) as string, units = Field(8) as string;
                developers[(index,number)] = new(string.IsNullOrEmpty(name) ? null : name,
                    string.IsNullOrEmpty(units) ? null : units, type);
            }
        }
    }
}
