# FIT Fusion 檢查器使用說明

本手冊說明現有 Python、C# 與 Rust 檢查器的執行方式。四種語言共用的輸出契約、SDK 能力與案例另有規格。[檢查器規格](spec/inspector.md) · [驗收情境與案例](spec/testcases/inspector.md)

需求、裝置辨識與資訊流盤點見[處理規格](specification.md)。三台設備的樣本、來源選擇及驗證結果見[11 月 21 日騎乘處理紀錄](research/rides/2026-11-21-ride.md)。

## Development CLI prerequisites (M4-5)

Build the clap frontend from the repository root before running the Python or Node source launchers:

```sh
cargo build --release --locked --manifest-path src/rust/inspector/Cargo.toml
runtime/build/rust/release/fit-inspector inspect src/tools/tests/fixtures/activities/rider.fit --stream record.heart_rate --stream record.power
```

The launchers use the sibling build or a `fit-inspector-bridge` on PATH. The C# build copies the frontend alongside FitFusion.dll. Each language still decodes the original bytes itself. Each member's `sampling` contains coverage and missing counts from that decoder's inventory.

## Run the inspector

From the repository root, with Python 3.9 or later, run

```sh
export PYTHONPATH="$PWD/src/python${PYTHONPATH:+:$PYTHONPATH}"
python3 -m venv runtime/venv
. runtime/venv/bin/activate
python -m pip install -r src/python/requirements.txt  # optional, enables the fitdecode backend
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

Run `python -m unittest discover -s src/python/tests -v` in the activated environment. The integration tests construct small FIT members locally so the test suite requires no personal activity data.

## 離線檢查原始內容

`python3 -m fit_fusion raw ride.fit > ride.wire.jsonl` 使用 Python 標準函式庫，可直接驗證 FIT 的 header 與檔尾 CRC 並逐筆輸出。命令成功時回傳 0；若回傳非零，請捨棄該次輸出。

每行代表一個 frame。`definition.fields` 的順序對應同一 `local` 編號後續資料列的 `values`，直到新的定義取代它。資料值保留 FIT 的儲存單位及無效值。未知型別和 developer field 的 payload 使用十六進位。依序串接每行的 `hex` 即可重建來源檔。`inspect` 預設優先使用已安裝的 fitdecode；缺少套件時自動使用離線解碼器。可用 `--decoder fitdecode` 或 `--decoder wire` 指定來源。

## Milestone 1-1：C# 對照組

在專案根目錄使用 .NET 10 SDK。兩個 CLI 都直接讀來源檔，報告輸出至 stdout。第一次使用先建置，再執行指令，以便將輸出重新導向到檔案。

```sh
dotnet build src/csharp/FitFusion
dotnet run --no-build --project src/csharp/FitFusion -- inspect data/sources/bryton/fit/261121141151.fit
dotnet run --no-build --project src/csharp/FitFusion -- raw data/sources/garmin/25149578721/25149578721_ACTIVITY.fit
```

以一條命令重跑兩份實檔的 Python／C# 驗證：

```sh
python3 src/tools/compare_inspectors.py \
  data/sources/bryton/fit/261121141151.fit \
  data/sources/garmin/25149578721/25149578721_ACTIVITY.fit
```

腳本會重新建置 C#，執行兩種語言的 CLI，依序比較 JSONL 各 frame 的所有欄位、還原檔案的 SHA-256，最後比較完整的 `inspect` JSON。失敗時回傳非零並顯示差異位置。結果包含每個來源的 C# 原始匯出、兩份具名統計與驗證摘要 `runtime/reports/milestone-1-1/comparison.json`。

C# 的 FIT frame 解析、CRC 與統計均由 .NET 實作。Python 對照固定使用 `--decoder wire`，兩邊共用 `src/python/fit_fusion/profile.tsv` 的全域訊息編號、欄位名稱和單位，因此名稱對照表本身不在交叉驗證範圍。表內資料來自 Garmin 公開的 FIT Profile 21.221.0，開發者欄位的名稱和型別則從各輸入檔取得。未在對照表定義的欄位用編號保留。[Garmin Profile](https://github.com/garmin/fit-csharp-sdk/blob/main/Dynastream/Fit/Profile.cs) · [Garmin 訊息編號](https://github.com/garmin/fit-csharp-sdk/blob/main/Dynastream/Fit/Profile/Types/MesgNum.cs)

## 四種檢查器的 FIT 往返驗證

從專案根目錄執行。需要 Python 3.9+、.NET 10 SDK、Node.js 22.12+ 與 rustup。專案指定 Rust 1.93.1，Cargo.lock 固定 RustyFIT 0.11.2。初次建置需要下載工具鏈與套件，建置後的檢查器可離線使用。

```sh
npm ci --prefix src/node/inspector

# 產生不含個資的控制樣本，執行四套檢查器並保存四個新 FIT
python3 src/tools/verify_roundtrip.py

# 用自己的 11/21 騎乘驗證，每次建立新結果目錄
python3 src/tools/verify_roundtrip.py \
  data/sources/bryton/fit/261121141151.fit \
  data/sources/garmin/25149578721/25149578721_ACTIVITY.fit

# 直接操作 Rust 檢查器
cargo build --locked --release --manifest-path src/rust/inspector/Cargo.toml
runtime/build/rust/release/fit-inspector --help
runtime/build/rust/release/fit-inspector --skill roundtrip
runtime/build/rust/release/fit-inspector inspect data/sources/bryton/fit/261121141151.fit
runtime/build/rust/release/fit-inspector roundtrip data/sources/bryton/fit/261121141151.fit --out runtime/bryton-reencoded.fit
```

Rust 的 `inspect` 預設由 RustyFIT 讀取樣本、時間戳與有效值，從 wire 定義取得原始欄位寬度。若 SDK 無法解出未附描述的 developer field，錯誤會列出訊息及欄位編號。用 `inspect FILE.fit --decoder wire` 可查看編碼端留下的欄位。

獨立執行 Python 可用 `PYTHONPATH=src/python python3 -m fit_fusion roundtrip INPUT.fit --out NEW.fit`。C# 可用 `dotnet run --project src/csharp/FitFusion -- roundtrip INPUT.fit --out NEW.fit`。輸出目錄須存在，既有檔案和指向來源的硬連結、符號連結都受到保護。

驗收工具印出本次 `runtime/reports/inspector-roundtrip/run-*/comparison.json` 的完整路徑。每個輸入各有一個子目錄，內含 `python.fit`、`csharp.fit`、`node.fit` 與 `rust.fit`，可交給任何讀取器驗證。

| 報告欄位 | 如何判讀 |
|---|---|
| `raw_equal`、`inspect_equal` | 四套 wire 讀取器逐框一致。RustyFIT 與四套 wire 的串流統計均已比對。 |
| `exports[].backend`、`verified_by` | 前者是匯出端，後者是另外開檔驗證的語言。Python、C#、Node.js 各由自己的 wire 欄位模型寫出，Rust 執行 RustyFIT Decoder → Encoder。 |
| `semantic_equal` | 驗過 CRC，逐 member、逐筆比對訊息、時間戳、原生／developer 欄位、型別及值。比較時還原大小端與壓縮時間，字串允許尾端 NUL 填充不同。 |
| `byte_equal`、兩份 SHA-256 | 表示整份檔案的位元組是否一致。定義重排、header 或填充改變都可能讓它為 false。 |
| Rust 的 `changed_string_widths` | 字串欄位值的編碼長度變動筆數。一個訊息可以包含多個字串欄位。 |

先前三語言驗收的六趟 Bryton／CIRQA 共 12 原檔對照了 256,129 個 frame。36 個匯出均通過另一個語言讀回。這批原檔上，Python、C# 輸出雜湊各自與輸入相同，RustyFIT 的 12 份輸出雜湊均改變。每份 CIRQA 檔有 14 個字串欄位值改變儲存長度，Bryton 各為 0。SDK 若把非標準陣列當純量並遺失值，Rust 檢查器會回報差異並拒絕交付檔案。[邊界與失敗案例](spec/testcases/inspector.md)

## Node.js 與 Garmin JavaScript SDK

先執行 `npm ci --prefix src/node/inspector`。下列命令以同一份來源分別驗證原始 FIT 與 Garmin SDK 的解讀。SDK 測試在記憶體重編碼，報告保留首筆差異或例外。需要可完整保存設備欄位的新檔時，使用 `roundtrip`。

```sh
node src/node/inspector/cli.mjs --skill inspect
node src/node/inspector/cli.mjs inspect ride.fit
node src/node/inspector/cli.mjs raw ride.fit
node src/node/inspector/cli.mjs sdk-check ride.fit
node src/node/inspector/cli.mjs roundtrip ride.fit --out reconstructed.fit
python3 -m unittest discover -s src/tools/tests -p test_roundtrip.py -v
```

11/21 的四語言比對中，Bryton 共 14,172 個 frame，CIRQA 共 29,346 個。兩檔的逐框與統計報告均相符；Python 獨立讀回 Node.js 重編碼的兩個 FIT，輸出 SHA-256 各與原檔相同。

`comparison.json` 的 `node_sdk.members[]` 另記錄官方 SDK 21.222.0 的能力。Bryton 解出 14,160 筆訊息，125,968 個帶有效值的原生欄位觀察均相符。CIRQA 解出 29,321 筆訊息，156,980 個觀察均相符。這項讀取比對排除 developer 欄位，陣列計為一次觀察，並對齊 FIT 無效碼與 JS 缺值。官方 Encoder 在兩個實檔都遇到無法寫出的訊息；合成的標準心率 record 則可成功往返。完整逐框及 developer 欄位的驗收使用 JS wire 後端。
