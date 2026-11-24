using System.Reflection;
using System.Runtime.Versioning;
using System.Runtime.InteropServices;
using System.Text.Json;

var assembly = typeof(WireReader).Assembly;
var version = assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()!.InformationalVersion;
var identity = $"fit-inspector {version} (C#)";

string Guide(string file)
{
    using var stream = typeof(WireReader).Assembly.GetManifestResourceStream($"guides.{file}")
        ?? throw new InvalidOperationException($"Missing embedded guide {file}");
    using var reader = new StreamReader(stream);
    return reader.ReadToEnd()
        .Replace("{cli}", "dotnet FitFusion.dll")
        .Replace("{identity}", identity)
        .Replace("{launch_note}", "Run from the directory containing the built C# FitFusion.dll.")
        .Replace("{decoder_args}", "")
        .Replace("{decoder_note}", "This inspector uses the C# WireReader.")
        .Replace("{sdk_row}", "").Replace("{sdk_topic}", "");
}

if (args is ["--version" or "-V"])
{
    Console.WriteLine(identity);
    Console.WriteLine($"Runtime   {RuntimeInformation.FrameworkDescription}");
    Console.WriteLine($"Platform  {RuntimeInformation.RuntimeIdentifier}");
    Console.WriteLine($"Target    {assembly.GetCustomAttribute<TargetFrameworkAttribute>()!.FrameworkName}");
    Console.WriteLine("FIT       C# WireReader");
    return 0;
}
if (args is ["--help" or "-h"])
{
    Console.Write(Guide("help.txt"));
    return 0;
}
if (args is ["raw" or "inspect" or "roundtrip", "--help" or "-h"])
{
    Console.Write(Guide(args[0] + ".txt"));
    return 0;
}
if (args is ["--skill", ..])
{
    if (args is ["--skill"] or ["--skill", "index" or "raw" or "inspect" or "roundtrip"])
    {
        Console.Write(Guide((args.Length == 1 ? "index" : args[1]) + ".md"));
        return 0;
    }
    Console.Error.WriteLine("Use --skill index for available topics (inspect, raw, roundtrip).");
    return 2;
}
if (args is not (["raw" or "inspect", _] or ["roundtrip", _, "--out", _]))
{
    Console.Error.Write(Guide("help.txt"));
    return 2;
}
try
{
    var data = File.ReadAllBytes(args[1]);
    if (args[0] == "roundtrip")
    {
        Console.WriteLine(JsonSerializer.Serialize(RoundTrip.Export(data, args[3])));
        return 0;
    }
    var reader = new WireReader(data);
    if (args[0] == "raw")
        reader.Read(frame => Console.WriteLine(JsonSerializer.Serialize(frame.Raw())));
    else
    {
        var inventory = new Inventory();
        reader.Read(inventory.Observe);
        var report = new { file = args[1], members = inventory.Members };
        Console.WriteLine(JsonSerializer.Serialize(report, new JsonSerializerOptions { WriteIndented = true }));
    }
    return 0;
}
catch (Exception ex) when (ex is IOException or InvalidDataException or UnauthorizedAccessException)
{
    Console.Error.WriteLine($"FitFusion: {args[1]}: {ex.Message}");
    return 1;
}
