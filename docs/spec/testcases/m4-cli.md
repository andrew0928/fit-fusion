# M4 獨立 CLI 驗收表

以下是 0.2.x 的開發驗收條件。雙檔覆寫先用 Rust 單元測試驗證每項欄位規則，再用 C#／Garmin SDK 合成的 FIT 跑實際 CLI，以獨立檢查器核對輸出。測試環境可只裝受測 CLI，使用空的個人設定目錄，全程不啟動舊合併器。[命令與取樣定義](../m4-cli.md) · [任務順序](../../milestones/m4-cli.md)

## 第一個交付：fit-merge（M4-1）

| 案例 | Condition　輸入與操作 | Result　可觀察的結果 |
|---|---|---|
| R01　獨立執行 | 無 config、無平台憑證、無網路、PATH 只有必要工具；兩份 FIT 可有相同設備型號。給 primary、overlay、一個 stream、out。另在預設設定位置放入無效 JSON 再跑。 | 兩次都成功，結果可獨立讀回且相同。角色完全由參數決定。help 顯示版本，skill 可離線查閱。 |
| R02　整條覆寫含缺樣 | 主檔在 t0…t3 都有心率 110；附檔 t0=140、t1=invalid、t2 沒有 record、t3=145。只選心率。 | 逐點為 `[140, 缺, 缺, 145]`。t1、t2 的主檔心率不會回流；其他串流、摘要及主時間軸維持主檔值。 |
| R03　不同採樣率與有效零 | 點名主檔原本沒有的功率。附檔 t0=0、t2=180、另有主時間軸之外的一筆。 | 主檔 t0…t3 對應功率為 `[0, 缺, 180, 缺]`，輸出仍是四筆 record。 |
| R04　整趟熱量 | 兩檔各有一個同趟 session，總量、代謝量各不相同。先只選 `session.total_calories`，再同時選兩個欄位。 | 第一份只有總量改採附檔值，另一份兩欄均為附檔值。來源唯讀，未選取的私有／developer 欄位可獨立讀回。 |
| R05　欄位缺席或全部失效 | 附檔完全沒有點名的欄，或有該欄定義且所選期間全部為 FIT invalid。 | 前者報出欄名且不產生部分結果；後者輸出全缺的所選欄。其他欄位不受影響。 |
| R06　多次執行 | 第一次由 B 覆寫 A 的心率為中間檔；第二次以中間檔為主、C 為附檔，僅覆寫功率；再另測第二次也點名心率。 | 只改功率時保留第一次的心率和缺口；第二次也改心率時，該欄完全取自 C。每一步恰好兩份輸入，輸出均能獨立讀回。 |
| R07　輸入錯誤 | 少一份輸入、多給第三份、未選 stream、目標指向來源，或兩檔完全不重疊。 | 非零結束，診斷指出參數、路徑或時段問題。來源與既有輸出逐位元組保持原樣。 |

## 查詢、分析與退場

| 案例／任務 | Condition　輸入與操作 | Result　可觀察的結果 |
|---|---|---|
| R08／M4-2 | 本機 FIT 含跨午夜、多趟、改名與混合設備，依日期 list、read、批次 export；另測空日、遺失目錄及同名衝突。 | 清單與出發日及設備相符，空日與讀取失敗可區別；輸出是原 FIT 的位元組副本。相同檔可重用，異內容保留既有檔。 |
| R09／M4-3 | Garmin 活動橫跨分頁；按 ID 匯出，再以既有快取重跑，模擬 token 到期與限流。 | 活動完整且去重，匯出為原生 FIT，換發、等待或重新授權各有可辨識回應。只允許 dotenv 提供金鑰。 |
| R10／M4-4 | ZIP 中 Watch 與 Connect 同時段，Watch 心率稀疏且首次缺路線。後續 ZIP 才補上同趟 GPX。 | 匯出所選的 Watch Workout，FIT 標明轉製、保留原取樣及缺項；資料變更另存修訂，相同 ZIP 重跑可重用。 |
| R11／M4-5 | 6 筆 record 中，指定欄位只有 4 筆有效（含有效零）、1 筆 invalid、1 筆未帶此欄。時間間隔不規則。 | 報總筆數 6、有效 4、缺 2、缺樣率約 33.33%。時間間隔另列，分析前後來源 FIT 雜湊相同。 |
| R12／M4-5 | 分別點名未記錄的心率、已記錄但整段 invalid 的心率，以及完全無 record 的活動。 | 前兩者在 6 筆 record 時都是 0 有效、缺 6、100%，各自標明欄位狀態。無 record 時總數、有效及缺數皆 0，百分比為 `null`。 |
| R13／M4-6（已取消） | 後續顯式欄位映射，含不相容單位、型別及目標衝突。 | 映射來源與生效值可查；衝突有診斷。保留原案例供歷史追溯。 |
| R14／M4-7（review 通過） | 在未安裝舊 CLI 的乾淨 macOS 安裝五支工具，各自執行 help、skill 與主要操作。 | 下載校驗與版本可核對，各工具可獨立完成任務。2026-11-25 使用者確認通過。[驗收紀錄](#r14review) |
| R15／M4-8（已取消） | 以候選平台的一筆活動研究雲端原檔或其他上傳目的地。 | 記錄授權、原檔或收據與去重能力，再決定是否實作。Strava 維持手動上傳。 |

M4-1 已通過 R01–R07 的本機驗收，另以六趟實騎的附檔心率與總熱量逐欄核對輸出。相關測試為 [Rust 核心測試](../../../src/rust/pair-merge/src/merge.rs) 與 [CLI 端對端測試](../../../src/tools/tests/test_pair_merge.py)。R10 起依對應任務驗收。各入口另須通過 [clap／help／skill 共通案例](../cli-common.md)。

### R08–R10 查詢輸出共通驗收

以同一查詢分別省略格式旗標、指定 `--json`、指定 `--text`。前兩次解析後的 JSON 相等；文字版能依 ID 區分同名活動，讀到設備、串流或逐筆匯出結果。兩旗標同時給定須回傳 2。另驗空清單、未知名稱與來源錯誤的結束碼。

- R08：同一已知設備的 FIT 改檔名後，活動 ID 與 `device_name` 相同。無活動標題時 `activity_name` 為 null，仍可從路徑辨識檔案。
- R09：清單有 Connect 標題，無設備資訊與快取時名稱為 null；已有對應原檔後列出查表設備。在同一快取內離線讀回觀察過的活動標題。清單過程不額外下載 FIT。
- R10：同一 ZIP 的 Watch 與 Connect 在同時段各有一筆；來源、標題、設備名稱分開檢驗。兩筆即使標題相同或缺標題也可各別選取。Watch 型號可查表，缺型號的 Connect Workout 回傳 null。按寫入來源篩選後只匯出相符活動。

### M4-2 本機驗收 R08

- [Rust 單元測試](../../../src/rust/query-files/src/tests.rs) 以既有 C#／Garmin SDK 樣本核對查詢時區及範圍、混合設備、改名去重、有效取樣區間、來源檔被替換及並行匯出。
- [CLI 端對端測試](../../../src/tools/tests/test_query_files.py) 在隔離目錄執行 help／skill、按日期與設備列出、按 ID 讀取及匯出，驗證同名衝突時仍可完成其餘檔案及安全重跑。
- 本機實騎覆蓋 10/28 跨午夜歸屬，以及 11/01、11/08、11/21 批次匯出。三份輸出均與來源 FIT 逐位元組相同，第二次均回報 `reused`。未提供本機實騎資料時，測試框架會標示該案例略過。

### M4-3 Garmin 驗收 R09

- [Rust 單元測試](../../../src/rust/query-garmin/src/tests.rs) 以可預期的頁面、HTTP 狀態及既有 Garmin SDK FIT 核對跨午夜、跨頁去重、重複頁、第二頁失敗、bearer 過期／拒絕、429 的等待資訊、離線命中、雜湊失效與歧義 ZIP。
- [獨立 CLI 端對端測試](../../../src/tools/tests/test_query_garmin.py) 核對 help／skill、無效參數、明確指定的憑證檔、離線讀回、重跑及同名衝突。缺少 .env 時即使程序已有同名變數也須回報授權錯誤。
- 正式帳號以 `--env-file .env` 查到 2026-11-21 的 `25149578721`。新下載 FIT 與既存 CIRQA 原檔逐位元組相同；離線讀回 9 個串流，按 ID／日期再匯出均回報重用。10/28 跨午夜的活動只歸在出發日。正式金鑰、原始 FIT 與驗收產物留在本機。

### M4-5 檢查器 R11、R12

四個語言入口以同一個串接兩個 member 的 FIT 驗證有效零、invalid、單筆缺欄、整檔未定義欄位及零分母。成功或失敗均保存輸入。另執行各後端既有 help、skill、wire、SDK 與 roundtrip 驗收。[跨語言測試](../../../src/tools/tests/test_roundtrip.py)

### M4-4 Apple Health R10

[Apple CLI 驗收](../../../src/tools/tests/test_query_apple.py) 產生含 Watch、Connect 同時段騎乘及跑步的 ZIP，驗證跨日、來源及硬體過濾、轉製標記、秒以下樣本、缺 GPX、補入 GPX 產生修訂、重用與衝突時保護既有檔案。獨立 Python wire reader 核對實際 FIT 值。另以本機 11/21 的完整 ZIP 分出兩筆活動，Watch FIT 有 13,821 個路線點、3,069 筆心率與 11,259 筆距離增量，重跑回報 reused。

<a id="r14review"></a>
### M4-7 乾淨環境驗收 R14

**通過。** 2026-11-25 使用者回報已完成乾淨環境驗收，確認 M4-7 R14 通過 review。驗收基準為 0.2.2 的 Apple Silicon Homebrew 測試套件。發佈端的匿名下載、SHA-256 及合成 FIT 讀回另有[分發驗證紀錄](../../decisions/2026-11-25-homebrew-cli.md)。

前置條件為已安裝 Homebrew 的 Apple Silicon Mac，PATH 中沒有舊 `fit-fusion` 執行檔。重現這次驗收時使用指向 0.2.2 的 Formula。下表列出操作、預期結果及各項紀錄的依據。

| 情境 | 操作 | 預期結果 | 核對依據 |
|---|---|---|---|
| 匿名取得套件 | 從公開 tap 的 0.2.2 Release 下載壓縮檔及校驗檔，比對 SHA-256。 | 免登入可下載，檔案雜湊與校驗檔、Formula 一致。 | 已記錄匿名下載與校驗。 |
| 乾淨環境首次安裝 | 執行 `brew install andrew0928/tools/fit-fusion`，核對套件版本及安裝目錄。 | 套件版本為 0.2.2，bin 目錄恰有 merge、inspector 與三支 query 工具。各工具可在沒有舊 CLI 的環境啟動。 | 使用者 R14 review 通過；Formula 測試另核對五支執行檔清單。 |
| 人與 agent 找到用法 | 對五支 CLI 各執行 `--help`、`--skill index`、`--version`。 | 可讀到該工具的命令、可查閱的 skill 主題與元件版本。 | R14 共通介面。Formula 的自動測試涵蓋五支工具的 `--help`。 |
| 查詢本機 FIT | 以空目錄執行 `fit-query-files --dir rides list --json`。 | 正常結束，`status` 為 `ok`，`activities` 為空陣列。 | 安裝版 `brew test` 的案例。 |
| 查詢 Apple 與 Garmin 活動 | Apple 準備含已知騎乘的 Health 匯出 ZIP，以 `fit-query-apple --archive FILE.zip list --date YYYY-MM-DD` 查詢騎乘日。Garmin 準備有效的本機 dotenv，以 `fit-query-garmin --env-file .env list --date YYYY-MM-DD` 查詢。 | 各工具可獨立列出對應來源的活動及 ID。 | 主要操作的參考範例，來源功能實測分列於 R10、R09。 |
| 合併後用檢查器讀回 | 以 `fit-merge` 指定同趟兩份合成 FIT，覆寫 `record.heart_rate` 並另存。執行 `fit-inspector inspect merged.fit --stream record.heart_rate`。 | 合併檔可獨立讀回，這組樣本有 **6 筆有效心率**。 | 0.2.2 已用 Homebrew 安裝版走完並記錄筆數。 |

R14 驗證首次安裝。從 0.2.2 升級至候選版由 [M5-4](../../milestones/m5-release.md) 驗收。
