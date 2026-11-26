# M5-5 個人設定初始化與執行前參數來源

## 狀態

`accepted`。2026-11-26 使用者確認初始化及參數來源方案，並指定用三欄條列呈現，鍵以紅色、值以白色、來源以灰色顯示。適用 0.3.0 發行的五支獨立 CLI，個人 YAML 由三支查詢工具共用。[已確認的設定契約](2026-11-26-single-yaml-config.md)

2026-11-26 後續決策將共通設定與跨版本升級提前至 0.2.3 驗收；所有功能驗收在 0.2.x 完成。2026-11-26 使用者確認「M5-5 review OK」，設定初始化、參數來源顯示與操作指引通過驗收。[版本範圍修訂](2026-11-26-v023-upgrade-and-retirement.md)

## 背景

使用者要從可以直接編輯的範本開始，自行決定啟用哪些欄位。人和 agent 也需要在工具讀取活動或寫檔前，看見這次會使用哪些值。現有查詢結果可透過 stdout 交給其他程式，檢查器的 `raw` 更會逐行輸出 JSON。[共同 CLI 規格](../spec/cli-common.md)

## 決策

### 1. 建立個人設定

三支查詢工具各提供同一個 `config init` 子命令，執行任一支一次即可。直接下載執行檔的使用者也可執行 `fit-query-files config init`。Homebrew 安裝時隨套件提供同一份唯讀範本，在安裝說明提示下列一行用法。Homebrew 的 post-install 在隔離的 HOME 中執行，交由使用者啟動 CLI 才能建立該帳號的設定。[Homebrew Formula 實作](https://docs.brew.sh/rubydoc/Formula.html#run_post_install-instance_method)

```sh
brew install andrew0928/tools/fit-fusion && fit-query-files config init
```

| 情境 | 行為 |
|---|---|
| 首次 init | 建立 `~/.fit-fusion/config.yaml`，回報 `created` 與路徑。新目錄權限 0700、新檔 0600。隨 binary 內嵌與套件安裝的範本取自同一份英文文件。 |
| 重跑、兩支工具同時 init | 以不可覆蓋的方式發布寫好的完整檔案。已存在的一般檔回報 `already_exists`、結束碼 0，保留檔案內容、權限與時間。目的地是目錄、符號連結或無法寫入時，回報路徑與錯誤。 |
| 安裝新版、重裝 | Homebrew 更新套件內的範本。個人檔由使用者維護。直接下載版採相同約定。 |

範本中的**每一個鍵與值都在英文註解內**，包含 `version`、父區段及所有工具選項。以下為內容示意，使用者啟用某個設定時也一併取消上層區段及 `version: 1` 的註解。

```yaml
# Uncomment version and the sections you use.
# version: 1
# common:
#   timezone: Asia/Taipei
# tools:
#   fit-query-files:
#     dir: ~/Cycling/Bryton
#   fit-query-apple:
#     archive: ~/Downloads/apple-health-export.zip
#   fit-query-garmin:
#     env_file: garmin.env
#     cache_dir: ~/Library/Caches/fit-query-garmin
```

這需要補充一條載入規則。只有空白或註解的檔案視為「尚未設定」，效果與缺檔相同。有啟用欄位時仍依既有 schema 驗證，必須明寫整數 `version: 1`。明寫 `null`、空物件 `{}`、錯誤版本或損壞的 YAML 都是設定錯誤。這讓初始化後原有的命令列用法立即可用，且可指出漏填版本的設定。[現有 schema](../config/query-config-v1.schema.yaml)

### 2. Help、skill 與執行前摘要

三支查詢工具的頂層 help 說明個人檔位置、init 命令及「命令列 → YAML → 內建值」的逐欄順位。資料子命令 help 在各選項標示可用的 YAML 鍵與省略行為。五支工具的 skill 索引說明如何讀取執行前摘要。查詢工具新增 `--skill config`，示範初始化、單次覆寫、離線讀取及保存 stdout 的方式。合併器與檢查器的 help／skill 說明參數來自命令列或工具內建值。上述說明入口和 `--version` 均可離線、獨立於設定檔執行。

| 旗標 | 可設定的 YAML 鍵 | 兩者皆省略 |
|---|---|---|
| 三支查詢工具 `--timezone` | `common.timezone` | `Asia/Taipei` |
| files `--dir` | `tools.fit-query-files.dir` | 資料命令回報缺少 FIT 目錄 |
| apple `--archive` | `tools.fit-query-apple.archive` | 資料命令回報缺少匯出 ZIP |
| garmin `--env-file` | `tools.fit-query-garmin.env_file` | 線上操作讀取工作目錄的 `.env` |
| garmin `--cache-dir` | `tools.fit-query-garmin.cache_dir` | `~/Library/Caches/fit-query-garmin` |

操作、位置參數、篩選條件、輸出檔與格式按各自的 clap 契約處理。例如 `--date`、`--id`、`--out` 只從命令列指定。工具先解析並驗證這次的有效參數，再將一份摘要完整寫至 **stderr**，然後才開啟活動來源、連線或寫入輸出。預設即顯示，包含非互動執行。stdout 仍可直接交給 `jq` 或重新導向。

以下是使用者已在 YAML 啟用 `dir: rides` 的例子。YAML 的相對路徑從設定檔所在目錄解析。這個呼叫的日期來自 CLI，時區與輸出格式採內建值。

```sh
fit-query-files list --date 2026-11-21 > activities.json
```

```text
- date      2026-11-21                       cli
- dir       /Users/example/.fit-fusion/rides  config
- output    indented-json                    cli (default)
- timezone  Asia/Taipei                      cli (default)
```

每個生效參數各佔一行，順序為鍵、值、來源。stderr 是支援顏色的終端時依序使用紅、白、灰；重新導向、非空的 `NO_COLOR` 或 `TERM=dumb` 使用純文字。內建值標為 `cli (default)`。摘要列出該操作所有實際生效的值，包含位置參數、明確旗標、內建值及最後解析的路徑。未選的篩選條件省略。缺必要值時在 stderr 指出對應旗標、可設定的 YAML 鍵與檔案位置並以 2 結束。查詢工具顯式使用 `--json` 時，stderr 的摘要改為一行 JSON，包含 `command`、`config`、`parameters`，每個參數包含 `value`、`source`（`cli` 或 `config`）與 `detail`（旗標、位置參數、YAML 鍵或 `default`）。stdio 由 shell 分開擷取。Garmin 只顯示憑證檔路徑與選取來源；離線讀快取時標明免用憑證。token、dotenv 內容與授權標頭永不進入摘要或錯誤。

## 影響

三支查詢工具共用同一份解析結果供「顯示」與「執行」使用。必須保留 clap 的命令列來源，才能把「明確指定、恰等於預設值」正確標為 `cli`。現有 CLI 的 `--help`、`--skill`、`--version` 維持原輸出契約，資料命令與管理命令 `config init` 分別以執行前摘要、初始化結果回報狀態。

## 替代方案

曾考慮在第一次 `list` 或 `read` 時自動建立個人設定。建議保留明確的 `config init`，讓唯讀查詢能在空白或唯讀的個人目錄下操作。另一個選擇是讓範本的 `version: 1` 直接生效；全註解範本更符合本次需求。

## 後續工作

本機驗收涵蓋以下情境。Homebrew 跨版升級另由 M5-4 驗收。

| 驗收 | 預期結果 |
|---|---|
| 空目錄從任一 query 初始化，另一支重跑；編輯後升級與重裝 | 三支共用同一路徑。初始檔沒有啟用值，原有檔案逐位元組保留。 |
| 全註解範本、只有 `config.json`、啟用欄位卻缺版本、無效 YAML | 前兩者走既有參數或內建值，後兩者指出設定錯誤。 |
| YAML 設為 UTC，命令列明寫 `--timezone Asia/Taipei` | 實際查詢與摘要均使用 Asia/Taipei，來源為 CLI。移除旗標後兩者均使用 YAML 的 UTC。 |
| 相對路徑、必要值遺漏、Garmin 已驗證快取的離線讀取 | 路徑依來源正確展開；必要值遺漏於活動 I/O 前結束；離線讀取免開啟 dotenv。 |
| 查詢接 `jq`、合併、檢查器 `raw`、所有工具的 help／skill／version | stdout 的資料可獨立解析；執行前摘要出現在 stderr；說明入口即使 YAML 損壞也可用。 |
