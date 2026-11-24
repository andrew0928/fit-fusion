using Dynastream.Fit;
using FitDateTime = Dynastream.Fit.DateTime;
using FitFile = Dynastream.Fit.File;

if (args.Length is < 1 or > 2 || args.Length == 2 && args[1] != "--scenarios")
{
    Console.Error.WriteLine("Usage: FitFixtures NEW_DIRECTORY [--scenarios]");
    return 2;
}
Directory.CreateDirectory(args[0]);
if (args.Length == 2)
{
    WriteRide("rider-gps.fit", 267, 2205, [110, 111, 255, 114, 114, 115], 6, true, sensors: true);
    WriteRide("cirqa-sensors.fit", 1, 5019, [109, 110, 113, 113, 114, 115], 2, true, sensors: true);
    WriteRide("cirqa-later.fit", 1, 5019, [121, 122], 2, true, 0x40000010);
    WriteRide("cirqa-later-alt.fit", 1, 5019, [124, 125], 3, true, 0x40000010);
    return 0;
}
WriteRide("rider.fit", 267, 2205, [110, 111, 255, 114, 114, 115], 6, true);
WriteRide("rider-later.fit", 267, 2205, [120, 121], 3, true, 0x40000010);
WriteRide("cirqa.fit", 1, 5019, [109, 110, 113, 113, 114, 115], 2, false);
return 0;

void WriteRide(string name, ushort maker, ushort product, byte[] heartRates, ushort total, bool cadence, uint start = 0x40000000, bool sensors = false)
{
    using var stream = new FileStream(Path.Combine(args[0], name), FileMode.CreateNew);
    var encoder = new Encode(ProtocolVersion.V20);
    encoder.Open(stream);
    var file = new FileIdMesg();
    file.SetType(FitFile.Activity);
    file.SetManufacturer(maker);
    file.SetProduct(product);
    encoder.Write(file);
    for (int i = 0; i < heartRates.Length; i++)
    {
        var record = new RecordMesg();
        record.SetTimestamp(new FitDateTime(start + (uint)i));
        record.SetHeartRate(heartRates[i]);
        if (cadence) record.SetCadence(i == 2 ? byte.MaxValue : (byte)0);
        if (sensors)
        {
            record.SetPositionLat(100 + i);
            record.SetPositionLong(i == 2 ? int.MaxValue : 200 + i);
            record.SetTemperature(22);
        }
        encoder.Write(record);
    }
    var session = new SessionMesg();
    session.SetSport(Sport.Cycling);
    session.SetStartTime(new FitDateTime(start));
    session.SetTimestamp(new FitDateTime(start + (uint)heartRates.Length - 1));
    session.SetTotalElapsedTime(heartRates.Length);
    session.SetTotalCalories(total);
    session.SetFieldValue(196, (ushort)1);
    if (sensors) session.SetAvgTemperature(22);
    encoder.Write(session);
    encoder.Close();
}
