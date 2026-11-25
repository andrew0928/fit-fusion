using System.Diagnostics;
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

if (args is not ["--worker", _])
{
    var bridge = new ProcessStartInfo(Path.Combine(AppContext.BaseDirectory, "fit-inspector-bridge")) { UseShellExecute = false };
    foreach (var item in new[] { "csharp", identity, "dotnet", assembly.Location, "--worker", "--" }.Concat(args))
        bridge.ArgumentList.Add(item);
    using var process = Process.Start(bridge)!;
    process.WaitForExit();
    return process.ExitCode;
}
using var document = JsonDocument.Parse(args[1]);
var request = document.RootElement;
string Get(string key) => request.GetProperty(key).GetString()!;
var command = Get("action");
if (command == "version")
{
    Console.WriteLine(identity);
    Console.WriteLine($"Runtime   {RuntimeInformation.FrameworkDescription}");
    Console.WriteLine($"Platform  {RuntimeInformation.RuntimeIdentifier}");
    Console.WriteLine($"Target    {assembly.GetCustomAttribute<TargetFrameworkAttribute>()!.FrameworkName}");
    Console.WriteLine("FIT       C# WireReader");
    return 0;
}
if (command == "skill")
{
    Console.Write(Guide(Get("topic") + ".md"));
    return 0;
}
var file = Get("file");
try
{
    var data = File.ReadAllBytes(file);
    if (command == "roundtrip")
    {
        Console.WriteLine(JsonSerializer.Serialize(RoundTrip.Export(data, Get("out"))));
        return 0;
    }
    var reader = new WireReader(data);
    if (command == "raw")
        reader.Read(frame => Console.WriteLine(JsonSerializer.Serialize(frame.Raw())));
    else
    {
        var inventory = new Inventory();
        reader.Read(inventory.Observe);
        var report = new { file = file, members = inventory.Members };
        Console.WriteLine(JsonSerializer.Serialize(report, new JsonSerializerOptions { WriteIndented = true }));
    }
    return 0;
}
catch (Exception ex) when (ex is IOException or InvalidDataException or UnauthorizedAccessException)
{
    Console.Error.WriteLine($"FitFusion: {file}: {ex.Message}");
    return 1;
}
