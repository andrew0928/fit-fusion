using System.Text.Json;

const string usage = "Usage: FitFusion <raw|inspect> FILE.fit\n       FitFusion roundtrip FILE.fit --out NEW.fit";
if (args is ["--help"] or ["-h"] or ["raw" or "inspect" or "roundtrip", "--help"])
{
    Console.WriteLine(usage);
    return 0;
}
if (args is not (["raw" or "inspect", _] or ["roundtrip", _, "--out", _]))
{
    Console.Error.WriteLine(usage);
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
