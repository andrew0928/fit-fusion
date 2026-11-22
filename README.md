# FIT Fusion

Inspect a FIT recording before choosing how to align its sensor streams. The inspector reads the source file and writes a JSON report to standard output.

需求、裝置辨識與資訊流盤點見[處理規格](docs/specification.md)。三台設備的樣本、來源選擇及驗證結果見[11 月 21 日騎乘處理紀錄](docs/2026-11-21-ride.md)。

## Run the inspector

From this directory, with Python 3.9 or later, run

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -r requirements.txt  # optional, enables the fitdecode backend
python -m fit_fusion inspect path/to/ride.fit
```

To save the report, redirect stdout to a separate file, for example `python -m fit_fusion inspect ride.fit > ride-report.json`. Decode errors go to stderr and produce a nonzero exit status. The reader checks FIT CRCs before the CLI prints the report.

## Read the report

Each entry in `members` describes one FIT member in file order. Most recordings contain one. Within a member, start with the `record` message (global number 20) to see how the activity timeline compares with individual sensors.

- **Inventory.** Each message has its global number, name and count. Fields include their encoded number, byte sizes, base type and units. A non-null `developer_data_index`, including zero, identifies a developer field. Unrecognized fields retain their numbers and carry `unknown: true`. A defined field remains visible even when it has no valid samples.
- **Coverage.** A field's `coverage_pct` is the percentage of all messages of its type that carry a valid value. Zero is a valid reading when the FIT base type permits it. An array with any valid element counts as one populated message. Coverage includes messages without timestamps.
- **Timing.** Message and field summaries give the earliest and latest UTC timestamps and their span. Intervals are measured between successive timestamped observations in file order. Minimum, median and maximum use positive intervals. Equal and backward steps have separate counts. A single observation has a zero span and an undetermined rate (`null`).
- **Sampling.** `typical_hz` is the reciprocal of the median positive interval. For example, a heart-rate value in every other 1 Hz record gives 50% coverage and 0.5 Hz when the timestamps are evenly spaced. Gaps and pauses remain visible in the maximum interval. These figures describe message-level sampling. Arrays can contain several sensor readings per message.
- **Timestamp scope.** Timing uses the standard FIT timestamp in whole seconds, including timestamps reconstructed from compressed headers. A missing, invalid or device-relative timestamp contributes no UTC observation. The field inventory covers encoded definitions. Derived components and compressed-header timestamps do not add field samples.

## Python FIT libraries considered

| Library | Relevance to inspection |
| --- | --- |
| [fitdecode](https://fitdecode.readthedocs.io/en/latest/reference/records.html) | Selected for its frame iterator. Definition frames expose native and developer fields, including numeric identifiers. Header and CRC frames delimit chained members. |
| [Garmin FIT Python SDK](https://github.com/garmin/fit-python-sdk) | The official SDK provides profile-aware decoding, developer field support and encoding. It is a candidate for the later FIT export milestone. |
| [fitparse](https://github.com/dtcooper/python-fitparse) | Provides a message and field API for activity data. Its maintainer suggests fitdecode as an alternative. |

Run `python -m unittest discover -s tests -v` in the activated environment. The integration tests construct small FIT members locally so the test suite requires no personal activity data.

## 離線檢查原始內容

`python3 -m fit_fusion raw ride.fit > ride.wire.jsonl` 使用 Python 標準函式庫，可直接驗證 FIT 的 header 與檔尾 CRC 並逐筆輸出。命令成功時回傳 0；若回傳非零，請捨棄該次輸出。

每行代表一個 frame。`definition.fields` 的順序對應同一 `local` 編號後續資料列的 `values`，直到新的定義取代它。資料值保留 FIT 的儲存單位及無效值。未知型別和 developer field 的 payload 使用十六進位。依序串接每行的 `hex` 即可重建來源檔。`inspect` 預設優先使用已安裝的 fitdecode；缺少套件時自動使用離線解碼器。可用 `--decoder fitdecode` 或 `--decoder wire` 指定來源。

## Milestone 1-1：C# 對照組

在專案根目錄使用 .NET 10 SDK。兩個 CLI 都直接讀來源檔，報告輸出至 stdout。第一次使用先建置，再執行指令，以便將輸出重新導向到檔案。

```sh
dotnet build csharp/FitFusion
dotnet run --no-build --project csharp/FitFusion -- inspect data/2026-1121-bryant-460.fit
dotnet run --no-build --project csharp/FitFusion -- raw data/2026-1121-garmin-cirqa.fit
```

以一條命令重跑兩份實檔的 Python／C# 驗證：

```sh
python3 scripts/compare_inspectors.py data/*.fit
```

腳本會重新建置 C#，執行兩種語言的 CLI，依序比較 JSONL 各 frame 的所有欄位、還原檔案的 SHA-256，最後比較完整的 `inspect` JSON。失敗時回傳非零並顯示差異位置。結果包含每個來源的 C# 原始匯出、兩份具名統計與[驗證結果](reports/milestone-1-1/comparison.json)。

C# 的 FIT frame 解析、CRC 與統計均由 .NET 實作。Python 對照固定使用 `--decoder wire`，兩邊共用 `fit_fusion/profile.tsv` 的全域訊息編號、欄位名稱和單位，因此名稱對照表本身不在交叉驗證範圍。表內資料來自 Garmin 公開的 FIT Profile 21.221.0，開發者欄位的名稱和型別則從各輸入檔取得。未在對照表定義的欄位用編號保留。[Garmin Profile](https://github.com/garmin/fit-csharp-sdk/blob/main/Dynastream/Fit/Profile.cs) · [Garmin 訊息編號](https://github.com/garmin/fit-csharp-sdk/blob/main/Dynastream/Fit/Profile/Types/MesgNum.cs)
