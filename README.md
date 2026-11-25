# FIT Fusion

FIT Fusion 開發兩類 CLI。檢查器用多個獨立解碼器盤點同一份 FIT。合併器把同趟多裝置記錄中的騎乘數據整合成一份 FIT，處理心率停滯與熱量差異。[需求與里程碑](docs/CONTEXT.md)

| CLI | 交付範圍與進度 | 規格 | 驗收情境與案例 |
|---|---|---|---|
| 檢查器 | Python、C#、Rust 與 Node.js 可盤點、逐框檢查及往返匯出。Node.js 另列 Garmin 官方 SDK 的讀寫結果。 | [檢查器規格](docs/spec/inspector.md) | [盤點、逐框還原與跨 SDK 比對](docs/spec/testcases/inspector.md) |
| 合併器 | macOS Rust CLI 可按日期從本機 FIT 或 Garmin Connect 配對、依設定選源並輸出 FIT 與報告，可加 `--parquet` 追查逐點選值與還原原檔。騎士手動上傳 FIT 至 Strava。 | [合併器規格](docs/spec/merger.md) | [選源、保存與發佈](docs/spec/testcases/merger.md) |

## 下載 macOS Apple Silicon 版

從 [GitHub Release v0.2.1](https://github.com/andrew0928/fit-fusion/releases/tag/v0.2.1) 下載 `fit-fusion-v0.2.1-macos-arm64.tar.gz` 和 `SHA256SUMS`，在下載目錄執行：

```sh
shasum -a 256 -c SHA256SUMS
tar -xzf fit-fusion-v0.2.1-macos-arm64.tar.gz
cd fit-fusion-v0.2.1-macos-arm64
./fit-query-garmin --skill index
./fit-query-files --version
./fit-merge --help
./fit-fusion --version
./fit-fusion --help
./fit-inspector --help
```

首次使用可執行 `./fit-fusion config init`，填入車錶目錄及手環來源。範本沿用車錶所有串流，只列心率備援與兩個手環熱量欄位。接著執行 `./fit-fusion merge --date YYYY-MM-DD --json`。[設定說明](docs/config/README.md)

## 專案目錄

| 位置 | 內容 |
|---|---|
| [docs/](docs/) | [處理規格](docs/specification.md)、[決策索引](docs/decisions/README.md)、[研究](docs/research/)與[實騎分析](docs/research/rides/)。 |
| [src/python/](src/python/) | Python FIT 檢查器、FIT Profile 對照表、依賴與測試。 |
| [src/csharp/](src/csharp/) | 獨立的 .NET FIT 檢查器。 |
| [src/node/inspector/](src/node/inspector/) | Node.js FIT 檢查器與 Garmin JavaScript SDK 驗證。 |
| [src/rust/inspector/](src/rust/inspector/) | Rust 檢查器與 RustyFIT 讀寫驗證。 |
| [src/rust/merger/](src/rust/merger/) | Rust 日期與雙檔合併 CLI。 |
| [src/rust/query-files/](src/rust/query-files/) | 獨立的本機 FIT 活動查詢與原檔匯出 CLI。 |
| [src/rust/query-garmin/](src/rust/query-garmin/) | Garmin Connect 查詢、原生 FIT 匯出與快取。 |
| [src/tools/](src/tools/) | 跨語言逐框比對與原檔雜湊驗證工具。 |
| `data/` | 本機唯讀輸入。原始 FIT、Apple 健康匯出與騎乘索引由 Git 忽略。 |
| `runtime/` | 本機產物。建置快取在 `build/`，驗證結果在 `reports/`，開發用 Python 虛擬環境可放在 `venv/`。 |

## fit-merge 0.2.0

從專案根目錄建置後，直接指定主檔、附檔、要整條覆寫的欄位與新輸出檔。GitHub v0.2.0 的發佈內容見上方下載說明。[M4 覆寫規格](docs/spec/m4-cli.md) · [CLI 的 help 與 skill 契約](docs/spec/cli-common.md)

```sh
cargo build --locked --release --manifest-path src/rust/pair-merge/Cargo.toml
runtime/build/rust/release/fit-merge --help
runtime/build/rust/release/fit-merge --skill index
runtime/build/rust/release/fit-merge --primary src/rust/merger/tests/fixtures/rider.fit \
  --overlay src/rust/merger/tests/fixtures/cirqa.fit --stream record.heart_rate \
  --stream session.total_calories --out runtime/ride.fit
```

## fit-query-files

從專案根目錄建置後，可用隨 repo 提供的 Garmin SDK 樣本走完查詢和匯出。下例清單有三趟，設備篩選後會匯出兩份 Bryton FIT。再次執行最後一行，可在回傳的 `exports[].status` 看到 `reused`。[本機查詢規格與實騎範例](docs/spec/m4-cli.md#本機-fit-目錄)

```sh
cargo build --locked --release --manifest-path src/rust/query-files/Cargo.toml
cli=./runtime/build/rust/release/fit-query-files
rides=src/rust/merger/tests/fixtures
"$cli" --skill index
"$cli" --dir "$rides" list --date 2024-01-09 --json
id=$("$cli" --dir "$rides" list --device 267/2205 --json | jq -er '.activities[0].id')
"$cli" --dir "$rides" read --id "$id" --json
"$cli" --dir "$rides" export --date 2024-01-09 --device 267/2205 --out runtime/selected/bryton --json
```

0.2.1 可加 `--text` 顯示人讀版活動、設備、串流及匯出結果。預設維持縮排 JSON，`--json` 是單行 JSON。

## fit-query-garmin 0.2.1

從 repo 根目錄建置。現行本機流程以 `--env-file .env` 讀取 Garmin 個人 API 金鑰，可在不開啟登入鑰匙圈的情況下完成。既有快取可離線按 Garmin 活動 ID 重用。[命令規格](docs/spec/m4-cli.md#garmin-connect-個人活動)

```sh
cargo build --release --locked --manifest-path src/rust/query-garmin/Cargo.toml
cli=./runtime/build/rust/release/fit-query-garmin
"$cli" --skill index
"$cli" --env-file .env list --date 2026-11-21 --text
"$cli" --env-file .env --cache-dir data/cache/garmin-query export --id 25149578721 --out runtime/selected/garmin --json
"$cli" --cache-dir data/cache/garmin-query read --id 25149578721 --offline --json
```

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
npm ci --prefix src/node/inspector
python3 src/tools/verify_roundtrip.py  # 四種語言各自重編碼合成樣本，印出報告位置
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
runtime/build/rust/release/fit-fusion --config docs/config/local-fit.example.json activities list --source bike_meter --json
runtime/build/rust/release/fit-fusion --config docs/config/local-fit.example.json activities list --source health_band --json
cargo test --locked --manifest-path src/rust/merger/Cargo.toml
python3 -m unittest discover -s src/tools/tests -p test_merger.py -v
```

單打 `fit-fusion activities` 會顯示查詢語法與範例。用 `config check` 取得來源 ID，再以 `activities list --source ID` 列出該來源的騎乘。結果包含活動 ID 與起迄的 Unix UTC 秒數。本機活動 ID 為內容 SHA-256，Connect 使用 Garmin 活動 ID。

## 按出發日期合併

```sh
runtime/build/rust/release/fit-fusion --config docs/config/local-fit.example.json merge --date 2024-01-09 --json
```

範例目錄刻意放了兩趟 Bryton 與一趟 Garmin。第一趟產出 FIT，第二趟缺少必要的 CIRQA 熱量，整日回傳 `partial` 及結束碼 3。可從 `rides[0].artifacts.fit` 取得第一趟產物；補齊第二趟資料後直接重跑。下列命令只選範例中資料齊全的第一趟，可取得結束碼 0。

```sh
runtime/build/rust/release/fit-fusion --config docs/config/local-fit.example.json merge \
  src/rust/merger/tests/fixtures/rider.fit --auto-match --out runtime/acceptance/m2-3-demo --json
```

`--auto-match` 會向 `merge.streams` 引用的各個次要來源尋找同趟活動。上例的 `health_band` 是本機 FIT 目錄。

日常可在 `~/.config/fit-fusion/config.json` 設定 Bryton 目錄、Garmin 的 `cache_dir` 及 `auth.personal_api_key`，之後執行 `fit-fusion merge --date 2026-11-21 --json`。有兩筆 Garmin 候選時，檢視該趟的 `details.candidates`，用 `merge PRIMARY.fit --garmin-id ID --out NEW_DIR` 指定。日期修訂以輸入內容與有效選源規則識別。兩個來源已有 FIT 時，可把 Garmin 改成 `fit_directory` 完全離線執行。[設定範本](docs/config/config.example.json) · [日期配對契約](docs/config/README.md#只給日期時的處理契約)
