# FIT Fusion

FIT Fusion 開發兩類 CLI。檢查器用多個獨立解碼器盤點同一份 FIT。合併器把同趟多裝置記錄中的騎乘數據整合成一份 FIT，處理心率停滯與熱量差異。[需求與里程碑](docs/CONTEXT.md)

| CLI | 交付範圍與進度 | 規格 | 驗收情境與案例 |
|---|---|---|---|
| 檢查器 | Python、C#、Rust 已提供盤點、逐框檢查及 FIT 往返匯出。Node.js 待開發。 | [檢查器規格](docs/spec/inspector.md) | [盤點、逐框還原與跨 SDK 比對](docs/spec/testcases/inspector.md) |
| 合併器 | macOS Rust CLI 可檢查 config、列舉本機 FIT 目錄、合併指定的兩個 FIT 並輸出報告。日期配對及 Strava 自動發佈列於 M2。 | [合併器規格](docs/spec/merger.md) | [選源、保存與發佈](docs/spec/testcases/merger.md) |

## 專案目錄

| 位置 | 內容 |
|---|---|
| [docs/](docs/) | [處理規格](docs/specification.md)、[決策索引](docs/decisions/README.md)、[研究](docs/research/)與[實騎分析](docs/research/rides/)。 |
| [src/python/](src/python/) | Python FIT 檢查器、FIT Profile 對照表、依賴與測試。 |
| [src/csharp/](src/csharp/) | 獨立的 .NET FIT 檢查器。 |
| [src/rust/inspector/](src/rust/inspector/) | Rust 檢查器與 RustyFIT 讀寫驗證。 |
| [src/rust/merger/](src/rust/merger/) | Rust 雙檔合併 CLI。 |
| [src/tools/](src/tools/) | 跨語言逐框比對與原檔雜湊驗證工具。 |
| `data/` | 本機唯讀輸入。原始 FIT、Apple 健康匯出與騎乘索引由 Git 忽略。 |
| `runtime/` | 本機產物。建置快取在 `build/`，驗證結果在 `reports/`，開發用 Python 虛擬環境可放在 `venv/`。 |

## 從專案根目錄執行

Python 3.9 以上可用標準函式庫執行離線檢查。C# 檢查器使用 .NET 10 SDK。將下列 `path/to/ride.fit` 換成自己的檔案。

```sh
PYTHONPATH=src/python python3 -m fit_fusion inspect --decoder wire path/to/ride.fit
dotnet build src/csharp/FitFusion
dotnet run --no-build --project src/csharp/FitFusion -- inspect path/to/ride.fit
```

執行測試，或以同趟兩個原檔重跑獨立驗證：

```sh
PYTHONPATH=src/python python3 -m unittest discover -s src/python/tests -v
python3 src/tools/compare_inspectors.py path/to/bryton.fit path/to/garmin.fit
python3 src/tools/verify_roundtrip.py  # 三種語言各自重編碼合成樣本，印出報告位置
```

比對工具將結果寫入 `runtime/reports/milestone-1-1/`。舊版文件裡的 `reports/` 指當時執行環境的產物目錄。[檢查器安裝、原始資料與報告格式](docs/inspectors.md)

## 本機雙檔合併

從根目錄建置後可離線執行。兩份原檔可對調，目的指定新的目錄。相同結果再次執行會驗證並重用產物。

```sh
cargo build --locked --release --manifest-path src/rust/merger/Cargo.toml
runtime/build/rust/release/fit-fusion --skill merge
runtime/build/rust/release/fit-fusion merge data/sources/bryton/fit/261121141151.fit \
  data/sources/garmin/25149578721/25149578721_ACTIVITY.fit --out runtime/acceptance/2026-11-21 --json
```

## 驗證本機 FIT 目錄來源

從根目錄執行。範例指向三個由 C# 與 Garmin FIT SDK 21.222.0 產生的小檔，可離線重現。兩個來源可指向同一目錄，各自依 FIT 裡的設備識別篩選。預期 Bryton 列出兩趟，Garmin 一趟。[範例設定](docs/config/local-fit.example.json)

```sh
cargo build --locked --release --manifest-path src/rust/merger/Cargo.toml
runtime/build/rust/release/fit-fusion --config docs/config/local-fit.example.json config check --json
runtime/build/rust/release/fit-fusion --config docs/config/local-fit.example.json activities list --source bryton --json
runtime/build/rust/release/fit-fusion --config docs/config/local-fit.example.json activities list --source garmin --json
cargo test --locked --manifest-path src/rust/merger/Cargo.toml
python3 -m unittest discover -s src/tools/tests -p test_merger.py -v
```

`activities list` 回傳來源 ID、活動內容 SHA-256、起迄的 Unix UTC 秒數。將範例的 `directory` 換為自己的 FIT 目錄即可列舉實騎。手動雙檔 `merge` 使用內建選源順位。Garmin API 與 Apple Health ZIP 可在設定中宣告，目前呼叫這兩種讀取器會回報 `source_unavailable`。
