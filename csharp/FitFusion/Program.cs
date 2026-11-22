using System.Text.Json;

const string usage = "Usage: FitFusion <raw|inspect> FILE.fit";
if (args is ["--help"] or ["-h"] or ["raw" or "inspect", "--help"])
{
    Console.WriteLine(usage);
    return 0;
}
if (args.Length != 2 || args[0] is not ("raw" or "inspect"))
{
    Console.Error.WriteLine(usage);
    return 2;
}
try
{
    var reader = new WireReader(File.ReadAllBytes(args[1]));
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
