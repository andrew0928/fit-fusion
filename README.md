# FIT Fusion

FIT Fusion 開發兩類 CLI。檢查器用多個獨立解碼器盤點同一份 FIT。合併器把同趟多裝置記錄中的騎乘數據整合成一份 FIT，處理心率停滯與熱量差異。[需求與里程碑](docs/CONTEXT.md)

| CLI | 交付範圍與進度 | 規格 | 驗收情境與案例 |
|---|---|---|---|
| 檢查器 | Python、C#、Rust 已提供盤點、逐框檢查及 FIT 往返匯出。Node.js 待開發。 | [檢查器規格](docs/spec/inspector.md) | [盤點、逐框還原與跨 SDK 比對](docs/spec/testcases/inspector.md) |
| 合併器 | macOS Rust CLI 可按日期從本機 FIT 或 Garmin Connect 配對、依設定選源並輸出 FIT 與報告，可加 `--parquet` 追查逐點選值與還原原檔。騎士手動上傳 FIT 至 Strava。 | [合併器規格](docs/spec/merger.md) | [選源、保存與發佈](docs/spec/testcases/merger.md) |

## 專案目錄

| 位置 | 內容 |
|---|---|
| [docs/](docs/) | [處理規格](docs/specification.md)、[決策索引](docs/decisions/README.md)、[研究](docs/research/)與[實騎分析](docs/research/rides/)。 |
| [src/python/](src/python/) | Python FIT 檢查器、FIT Profile 對照表、依賴與測試。 |
| [src/csharp/](src/csharp/) | 獨立的 .NET FIT 檢查器。 |
| [src/rust/inspector/](src/rust/inspector/) | Rust 檢查器與 RustyFIT 讀寫驗證。 |
| [src/rust/merger/](src/rust/merger/) | Rust 日期與雙檔合併 CLI。 |
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

需要分析檔時，在同一條合併命令加 `--parquet`，手動模式請指定新的輸出目錄。日期模式會自動保留各次修訂。Strava 手動上傳請選 `merged.fit`。[分析檔欄位與查詢](docs/spec/merger.md#parquet-分析檔-v2)

可用兩台未註冊的合成車錶驗證新串流。從根目錄執行 `runtime/build/rust/release/fit-fusion --config docs/config/stream-fit.example.json merge --date 2024-01-09 --parquet --json`，即可讀回功率、左右平衡及踩踏功率相位。[新設備與感測器](docs/config/README.md#新感測器與新欄位)

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

`activities list` 回傳來源 ID、活動 ID 與起迄的 Unix UTC 秒數。本機活動 ID 為內容 SHA-256，Connect 使用 Garmin 活動 ID。

## 按出發日期合併

```sh
runtime/build/rust/release/fit-fusion --config docs/config/local-fit.example.json merge --date 2024-01-09 --json
```

範例目錄刻意放了兩趟 Bryton 與一趟 Garmin。第一趟產出 FIT，第二趟缺少必要的 CIRQA 熱量，整日回傳 `partial` 及結束碼 3。可從 `rides[0].artifacts.fit` 取得第一趟產物；補齊第二趟資料後直接重跑。下列命令只選範例中資料齊全的第一趟，可取得結束碼 0。

```sh
runtime/build/rust/release/fit-fusion --config docs/config/local-fit.example.json merge \
  src/rust/merger/tests/fixtures/rider.fit --garmin auto --out runtime/acceptance/m2-3-demo --json
```

日常可在 `~/.config/fit-fusion/config.json` 設定 Bryton 目錄、Garmin 的 `cache_dir` 及 `auth.personal_api_key`，之後執行 `fit-fusion merge --date 2026-11-21 --json`。有兩筆候選時，檢視該趟的 `details.candidates`，用 `merge PRIMARY.fit --garmin-id ID --out NEW_DIR` 指定。日期修訂以輸入內容與有效選源規則識別。兩個來源已有 FIT 時，可把 Garmin 改成 `fit_directory` 完全離線執行。[設定範本](docs/config/config.example.json) · [日期配對契約](docs/config/README.md#只給日期時的處理契約)
