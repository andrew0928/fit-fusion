using Dynastream.Fit;
using FitDateTime = Dynastream.Fit.DateTime;
using FitFile = Dynastream.Fit.File;

if (args.Length is < 1 or > 2 || args.Length == 2 && args[1] is not ("--scenarios" or "--streams" or "--identities"))
{
    Console.Error.WriteLine("Usage: FitFixtures NEW_DIRECTORY [--scenarios | --streams | --identities]");
    return 2;
}
Directory.CreateDirectory(args[0]);
if (args.Length == 2 && args[1] == "--identities")
{
    WriteRide("file-name.fit", 267, 2004, [120, 121], 2, true, fileName: "Recorder A", creatorName: "Recorder B");
    WriteRide("creator-name.fit", 267, 2004, [120, 121], 2, true, creatorName: "Recorder B");
    WriteRide("sensor-only.fit", 267, 30001, [120, 121], 2, true, accessory: true);
    return 0;
}
if (args.Length == 2 && args[1] == "--streams")
{
    // Synthetic product numbers: neither model is registered in the CLI.
    WriteRide("bike.fit", 267, 30001, [110, 111, 255, 114, 114, 115], 6, true, power: true);
    WriteRide("backup.fit", 1, 30002, [109, 110, 113, 113, 114, 115], 2, true, power: true);
    return 0;
}
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

void WriteRide(string name, ushort maker, ushort product, byte[] heartRates, ushort total, bool cadence, uint start = 0x40000000, bool sensors = false, bool power = false, string fileName = null, string creatorName = null, bool accessory = false)
{
    using var stream = new FileStream(Path.Combine(args[0], name), FileMode.CreateNew);
    var encoder = new Encode(ProtocolVersion.V20);
    encoder.Open(stream);
    var file = new FileIdMesg();
    file.SetType(FitFile.Activity);
    file.SetManufacturer(maker);
    file.SetProduct(product);
    if (fileName != null) file.SetProductName(fileName);
    encoder.Write(file);
    if (creatorName != null || accessory)
    {
        var info = new DeviceInfoMesg();
        info.SetDeviceIndex(accessory ? (byte)1 : (byte)0);
        info.SetManufacturer(maker);
        info.SetProduct(product);
        info.SetProductName(accessory ? "Cadence sensor" : creatorName);
        encoder.Write(info);
    }
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
        if (power)
        {
            ushort?[] watts = maker == 267 ? [0, 120, null, 250, 180, null] : [99, 99, 170, 99, 99, null];
            if (watts[i] is ushort sample) record.SetPower(sample);
            record.SetLeftRightBalance((byte)(maker == 267 ? 45 : 52));
            if (maker == 1)
            {
                record.SetLeftPowerPhase(0, 45f);
                record.SetLeftPowerPhase(1, 90f);
            }
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
