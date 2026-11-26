# FIT Fusion

FIT Fusion 開發兩類 CLI。檢查器用多個獨立解碼器盤點同一份 FIT。合併器把同趟多裝置記錄中的騎乘數據整合成一份 FIT，可按欄位覆寫心率、熱量等數據。[需求與里程碑](docs/CONTEXT.md)

| CLI | 交付範圍與進度 | 規格 | 驗收情境與案例 |
|---|---|---|---|
| 檢查器 | Python、C#、Rust 與 Node.js 可盤點、逐框檢查及往返匯出。Node.js 另列 Garmin 官方 SDK 的讀寫結果。 | [檢查器規格](docs/spec/inspector.md) | [盤點、逐框還原與跨 SDK 比對](docs/spec/testcases/inspector.md) |
| 合併器 | `fit-merge` 以主檔為基準，從另一份 FIT 整條覆寫指定的同名欄位，輸出可手動上傳的 FIT。 | [雙檔覆寫規格](docs/spec/m4-cli.md) | [合併與查詢驗收](docs/spec/testcases/m4-cli.md) |

## 下載 macOS Apple Silicon 版

Homebrew 的 **0.2.x 套件供測試使用**。提供 `fit-merge`、`fit-inspector` 與三支 `fit-query-*`，可匿名安裝。0.2.x 持續完成功能與升級驗收。**0.2.4 預發佈**已移除舊 `fit-fusion` 的原始碼及執行檔。Homebrew 穩定版為 0.2.3。各版附件集中於主 repo，tap 提供最新版及指定版本的 Formula。[全部 Release](https://github.com/andrew0928/fit-fusion/releases) · [Homebrew tap](https://github.com/andrew0928/homebrew-tools) · [M5 發佈計畫](docs/milestones/m5-release.md)

```sh
brew install andrew0928/tools/fit-fusion
fit-inspector --skill index
```

需要重現歷史版本時，可指定完整版本，例如 `brew install andrew0928/tools/fit-fusion@0.2.1`。歷史版提供該版已發佈的工具。

從 0.2.3 起可用 `fit-query-files config init` 建立個人 YAML。三支查詢工具共用此檔，五支獨立 CLI 的執行前摘要會列出有效參數與來源。[初始化、升級與舊 CLI 退場指引](docs/config/README.md)

### 試用 0.2.4 預發佈版

從 [v0.2.4 pre-release](https://github.com/andrew0928/fit-fusion/releases/tag/v0.2.4) 下載 `fit-fusion-v0.2.4-macos-arm64.tar.gz` 和 `SHA256SUMS`，在同一目錄執行：

```sh
shasum -a 256 -c SHA256SUMS
tar -xzf fit-fusion-v0.2.4-macos-arm64.tar.gz
cd fit-fusion-v0.2.4-macos-arm64
./fit-merge --version
./fit-query-files --skill config
```

壓縮檔內有五支 CLI、供多語言檢查器使用的 `fit-inspector-bridge`、設定範本與授權文件。需要重現舊合併器與 JSON 設定時，可使用 [v0.2.3 的原始碼與套件](https://github.com/andrew0928/fit-fusion/releases/tag/v0.2.3)。

## 專案目錄

| 位置 | 內容 |
|---|---|
| [docs/](docs/) | [處理規格](docs/specification.md)、[決策索引](docs/decisions/README.md)、[研究](docs/research/)與[實騎分析](docs/research/rides/)。 |
| [src/python/](src/python/) | Python FIT 檢查器、FIT Profile 對照表、依賴與測試。 |
| [src/csharp/](src/csharp/) | 獨立的 .NET FIT 檢查器。 |
| [src/node/inspector/](src/node/inspector/) | Node.js FIT 檢查器與 Garmin JavaScript SDK 驗證。 |
| [src/rust/inspector/](src/rust/inspector/) | Rust 檢查器與 RustyFIT 讀寫驗證。 |
| [src/rust/pair-merge/](src/rust/pair-merge/) | 獨立的雙檔串流覆寫 CLI。 |
| [src/rust/query-files/](src/rust/query-files/) | 獨立的本機 FIT 活動查詢與原檔匯出 CLI。 |
| [src/rust/query-apple/](src/rust/query-apple/) | 本機 Apple Health ZIP 查詢與轉製 FIT。 |
| [src/rust/query-garmin/](src/rust/query-garmin/) | Garmin Connect 查詢、原生 FIT 匯出與快取。 |
| [src/tools/](src/tools/) | 跨語言逐框比對與原檔雜湊驗證工具。 |
| `data/` | 本機唯讀輸入。原始 FIT、Apple 健康匯出與騎乘索引由 Git 忽略。 |
| `runtime/` | 本機產物。建置快取在 `build/`，驗證結果在 `reports/`，開發用 Python 虛擬環境可放在 `venv/`。 |

## fit-merge

從專案根目錄建置後，直接指定主檔、附檔、要整條覆寫的欄位與新輸出檔。[M4 覆寫規格](docs/spec/m4-cli.md) · [CLI 的 help 與 skill 契約](docs/spec/cli-common.md)

```sh
cargo build --locked --release --manifest-path src/rust/pair-merge/Cargo.toml
runtime/build/rust/release/fit-merge --help
runtime/build/rust/release/fit-merge --skill index
runtime/build/rust/release/fit-merge --primary src/tools/tests/fixtures/activities/rider.fit \
  --overlay src/tools/tests/fixtures/activities/cirqa.fit --stream record.heart_rate \
  --stream session.total_calories --out runtime/ride.fit
```

## fit-query-files

從專案根目錄建置後，可用隨 repo 提供的 Garmin SDK 樣本走完查詢和匯出。下例清單有三趟，設備篩選後會匯出兩份 Bryton FIT。再次執行最後一行，可在回傳的 `exports[].status` 看到 `reused`。[本機查詢規格與實騎範例](docs/spec/m4-cli.md#本機-fit-目錄)

```sh
cargo build --locked --release --manifest-path src/rust/query-files/Cargo.toml
cli=./runtime/build/rust/release/fit-query-files
rides=src/tools/tests/fixtures/activities
"$cli" --skill index
"$cli" --dir "$rides" list --date 2024-01-09 --json
id=$("$cli" --dir "$rides" list --device 267/2205 --json | jq -er '.activities[0].id')
"$cli" --dir "$rides" read --id "$id" --json
"$cli" --dir "$rides" export --date 2024-01-09 --device 267/2205 --out runtime/selected/bryton --json
```

0.2.1 可加 `--text` 顯示人讀版活動、設備、串流及匯出結果。預設維持縮排 JSON，`--json` 是單行 JSON。

## fit-query-garmin 0.2.1

從 repo 根目錄建置。現行本機流程以 `--env-file .env` 讀取 Garmin 個人 API 金鑰，預設讀取工作目錄的 .env。既有快取可離線按 Garmin 活動 ID 重用。[命令規格](docs/spec/m4-cli.md#garmin-connect-個人活動)

```sh
cargo build --release --locked --manifest-path src/rust/query-garmin/Cargo.toml
cli=./runtime/build/rust/release/fit-query-garmin
"$cli" --skill index
"$cli" --env-file .env list --date 2026-11-21 --text
"$cli" --env-file .env --cache-dir data/cache/garmin-query export --id 25149578721 --out runtime/selected/garmin --json
"$cli" --cache-dir data/cache/garmin-query read --id 25149578721 --offline --json
```

## Apple Health 查詢與取樣檢查（0.2.2）

```sh
cargo build --release --locked --manifest-path src/rust/query-apple/Cargo.toml
cargo build --release --locked --manifest-path src/rust/inspector/Cargo.toml
bin=runtime/build/rust/release
archive=data/sources/apple-health/apple-health-export.zip
"$bin/fit-query-apple" --archive "$archive" list --date 2026-11-21 --text
"$bin/fit-query-apple" --archive "$archive" export --date 2026-11-21 --source-name AndrewWatch --out data/exports/apple-watch --json
"$bin/fit-inspector" inspect src/tools/tests/fixtures/activities/rider.fit --stream record.heart_rate --stream record.power
```

## 從專案根目錄執行

Python 3.9 以上可用標準函式庫執行離線檢查。Python 和 Node.js 的入口會從 repo 的 release 建置目錄或 PATH 找到共用的 `fit-inspector-bridge`。macOS arm64 壓縮檔已附上這個執行檔。C# 檢查器使用 .NET 10 SDK，建置時會以 Cargo 編譯並複製 bridge。將下列 `path/to/ride.fit` 換成自己的檔案。

```sh
cargo build --release --locked --manifest-path src/rust/inspector/Cargo.toml
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

## 作者與聯絡方式

在本版原始碼建置的任一 CLI 執行 `--skill author`（例如 `fit-merge --skill author`），即可離線查看 Andrew Wu 的聯絡方式、專案 GitHub 網址與工具介紹文章的刊登狀態。[作者頁](src/python/fit_fusion/guides/author.md)

## 授權

FIT Fusion 的原創程式碼與隨附使用文件採用 MIT License。第三方 SDK、函式庫及其資料沿用各自的授權。[MIT 授權全文](LICENSE)
