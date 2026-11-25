# 所有 CLI 的共同命令列契約

**已確認，2026-11-25。** 專案所有面向使用者或 agent 的 CLI 都使用 Rust 的 **clap** 解析命令列，並交付 `--help` 與 `--skill`。此要求涵蓋合併、查詢、檢查工具與各語言的對外入口。資料格式、授權與輸出內容仍由各工具的規格定義。[決策紀錄](../decisions/2026-11-25-common-cli.md)

| 操作 | 使用者可以依賴的行為 |
|---|---|
| 解析命令列 | 在 clap 宣告子命令、選項、必填、重複與互斥關係，從同一份命令樹產生 help。先驗證語法，再開啟設定、活動檔或網路。未知參數與語法錯誤回傳結束碼 2。 |
| `--help`、`-h` | 離線顯示工具名稱、版本、用途、用法、參數及可照做的範例，回傳 0。有子命令時，`<subcommand> --help` 顯示該子命令的用法。 |
| `--skill`、`--skill index` | 兩種拼法回傳相同的離線主題索引和結束碼 0。索引列出該工具實際支援的主題及呼叫方式。 |
| `--skill <topic>` | 回傳該任務給 agent 的指引，包括可執行命令、必要輸入、輸出、副作用、失敗處理。未知主題回傳 2 並提示可用主題。取用指引不需資料檔、個人設定或授權。 |
| `--version`、`-V` | 顯示版本及既有規格要求的建置或執行資訊，便於把報告與執行版本配對。 |

## 共同驗收

每個可交付的執行檔均須在乾淨目錄測試頂層 help、各子命令 help、兩種索引、每個已列主題、未知主題及錯誤旗標。依指引在隔離的輸入／輸出目錄執行至少一條正常流程，核對結束碼及產物。clap 的採用另從實際對外入口與建置依賴核對；只通過文字輸出的比對仍需核對解析器。

### 三支 fit-query 的輸出

`fit-query-files`、`fit-query-garmin`、`fit-query-apple` 的資料命令預設印出縮排 JSON。`--json` 和 `--text` 分別指定單行 JSON 與人讀版，兩者互斥。顯示格式不改變活動的選取、匯出內容或結束碼。[名稱與輸出契約](m4-cli.md#查詢結果的共通欄位與輸出格式)

## M5-5 個人設定（0.3.0 規格，待實作）

個人設定集中在一份選用的 `~/.fit-fusion/config.yaml`。根節點是 YAML mapping。`version` 為格式版本（整數 `1`），`common` 保存跨工具共用值，`tools` 下以完整執行檔名稱作為區段名稱。`common`、`tools`、各工具區段及其欄位均可省略。

```yaml
version: 1
common:
  timezone: Asia/Taipei
tools:
  fit-query-files:
    dir: ~/Cycling/Bryton
  fit-query-apple:
    archive: ~/Downloads/輸出.zip
  fit-query-garmin:
    env_file: garmin.env
    cache_dir: ~/Library/Caches/fit-query-garmin
```

| 欄位（值為字串） | 讀取者與用途 | 命令列覆寫 | 省略時 |
|---|---|---|---|
| `common.timezone` | 三支 query 共用的 IANA 時區，用於解讀騎乘出發日 | `--timezone` | `Asia/Taipei` |
| `tools.fit-query-files.dir` | 本機 FIT 查詢的輸入目錄 | `--dir` | 資料命令須指定 `--dir` |
| `tools.fit-query-apple.archive` | Apple Health 查詢的 ZIP 檔 | `--archive` | 資料命令須指定 `--archive` |
| `tools.fit-query-garmin.env_file` | Garmin 線上操作讀取的 dotenv 路徑，金鑰名稱為 `GARMIN_PERSONAL_ACCESS_TOKEN` | `--env-file` | 工作目錄的 `.env` |
| `tools.fit-query-garmin.cache_dir` | Garmin 原始 FIT 快取目錄 | `--cache-dir` | `~/Library/Caches/fit-query-garmin` |

每個欄位只有表列的一個 YAML 位置。查詢時按欄位選用「明確命令列參數 → 對應 YAML 欄位 → 既有預設」。YAML 路徑支援絕對路徑及 `~/`，相對路徑以 `config.yaml` 所在目錄解析。命令列相對路徑以工作目錄解析。已選中的輸入或憑證檔有誤時，回報其路徑及原因，避免改讀另一個來源或帳號。

`fit-merge` 與 `fit-inspector` 在此版沒有個人設定欄位，直接使用命令列參數。五支工具的 `--help`、`--skill`、`--version` 均可獨立於 YAML 執行。舊版 `fit-fusion` 的 v1 JSON 與選源規則沿用其[原規格](../config/README.md)。

### M5-5 設定驗收

- 在空的個人目錄中，既有的顯式參數仍可走完查詢、匯出、合併與檢查。即使 YAML 語法錯誤，免設定的兩支工具及五支工具的說明入口仍可使用。
- 使用同一份 YAML 從兩個工作目錄查詢，三支 query 的日期解讀一致。對單次呼叫指定 `--timezone` 或來源路徑時，只覆寫該次呼叫的對應欄位。
- 每支 query 只使用共用區段與自己的區段。其他來源未掛載或 ZIP 路徑失效時，仍能查詢本來源。讀取設定的命令遇到 YAML 語法錯誤或不支援的格式版本時，應指出設定檔路徑。
- Garmin 分別驗證顯式 `--env-file`、YAML 內的相對路徑，以及兩者均省略時的工作目錄 `.env`。顯式憑證檔缺少必要鍵時，回報鍵名並隱去值。按 ID 離線讀取已驗證快取時免讀憑證。

## 目前採用情形

| 對外入口 | clap | help／skill | 歸屬 |
|---|---|---|---|
| Rust `fit-inspector`、舊 `fit-fusion` | 已採用 | 已有索引與主題 | 0.1.0 已交付，舊合併器自 0.3.0 起退場。 |
| `fit-merge` | 已採用 | `index`、`merge` | M4-1。 |
| `fit-query-files` | 已採用 | `index`、`list`、`read`、`export` | M4-2。 |
| `fit-query-garmin` | 已採用 | `index`、`list`、`read`、`export`、`auth` | M4-3。 |
| `fit-query-apple` | 已採用 | `index`、`list`、`read`、`export` | M4-4 於 0.2.2 通過 R10。 |
| Python、C#、Node.js 檢查器 CLI | 已採用；三支入口透過 fit-inspector-bridge 的 clap 命令樹解析 | 已有索引與主題 | M4-5 完成。各後端獨立解碼，Rust 只依後端盤點結果計算缺樣。 |

各工具可重用命令列程式碼或套件。新執行檔獨立提供操作指引，執行時不呼叫舊的 `fit-fusion`。
