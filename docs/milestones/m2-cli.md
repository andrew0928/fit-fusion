# M2 設計　從同趟騎乘產出 FIT 並發佈到 Strava

M2 交付一支在 macOS 本機執行的 Rust CLI。騎士指定日期，由 CLI 到已設定的主來源找出當日騎乘、配對 CIRQA，再按逐串流順位合併。可取得驗證過的 FIT 或發佈到 Strava。Agent 能從執行檔取得操作指引，依輸出和結束碼判斷下一步。里程碑範圍、熱量來源與設定檔 v1 已確認。以下列出目標介面及實作計畫。[設定檔 v1](../config/README.md) · [里程碑決策](../decisions/2026-11-23-cli-milestones.md) · [合併規格](../specification.md)

## 騎完一趟如何使用

以下是目標介面。日常只給出發日期，CLI 依設定檔決定去哪裡找記錄與輸出。也可直接指定兩個原檔離線重播，或單獨指定某個主檔與 Garmin 來源。

```sh
fit-fusion --help
fit-fusion --skill index
fit-fusion --skill merge

fit-fusion merge --date 2026-11-21
fit-fusion merge --date 2026-11-21 --publish strava --json
fit-fusion merge ride-bryton.fit ride-garmin.fit --out ./rides/local --json
fit-fusion merge ride-bryton.fit --garmin auto --out ./rides/published --publish strava --json

# 先前已完成本機合併，現在才要發佈或接續查詢
fit-fusion publish strava --from ./rides/local --json
```

| 入口 | 行為與結果 |
|---|---|
| `--help`、`<command> --help` | 人可直接查用途、參數、實例、產物與是否會連網或上傳。 |
| `--skill <topic>` | 內嵌版本相符的 Markdown。以 `index` 列舉 `config`、`merge`、`publish`。每個主題提供前置條件、可複製命令、結果欄位、常見錯誤、下一步及重試的副作用。未知主題列出可用主題並回傳 2。Help 和 skill 在沒有設定、憑證、網路時都可用。 |
| `config init`、`config check`、`auth strava` | 初始化寫出含預設順位與空白憑證的設定，已存在時保留原檔。check 離線檢查 Schema 與來源引用。首次 Strava 授權由騎士在瀏覽器同意，CLI 以 loopback callback 取回並驗證結果。 |
| `merge --date YYYY-MM-DD` | 依設定的時區、主來源、串流順位與輸出位置逐趟處理。可加 `--publish strava` 或 `--json`。日期模式與手動指定輸入互斥。[日期與多趟結果契約](../config/README.md#只給日期時的處理契約) |
| `merge <FIT> [<FIT>] --out <dir>` | 雙檔依內容辨識設備，順序不限。僅有 Bryton 時可指定 `--garmin auto` 或 `--garmin-id <id>` 下載原檔。三種輸入方式擇一。預設只寫本機，加 `--publish strava` 才執行發佈。 |
| `publish strava --from <dir>` | 讀取合併報告、驗證 FIT 雜湊，沿用既有發佈紀錄。可發佈離線產物、恢復等待中的上傳，或取回已建立的活動 ID。 |

**給 agent 的結果契約。** 作業命令接受 `--json`，stdout 恰有一份 JSON，進度與診斷送 stderr。至少含 `schema_version`、`status`、`code`、`artifacts`、`next_action`。發佈結果另含字串型別的 `upload_id`、`activity_id`，未知為 `null`。憑證永不出現在輸出。下例表示本機 FIT 已完成，重跑同一個發佈命令即可續查。

```json
{"schema_version":1,"status":"pending","code":"strava_processing","artifacts":{"fit":"rides/local/merged.fit","report":"rides/local/merge-report.json"},"upload_id":"12345","activity_id":null,"next_action":"fit-fusion publish strava --from ./rides/local --json"}
```

| 結束碼 | Agent 的處理方式 |
|---|---|
| 0 | 已完成所要求的動作。本機合併回傳 `exported`，成功發佈回傳 `published` 及活動 ID。 |
| 2、3、4 | 2 表示參數或設定錯誤。3 表示輸入無法配對、資料須判讀、產物驗證失敗或平台判重，依穩定的 `code` 分辨。4 表示需更新憑證或重新授權。 |
| 5、6 | 5 表示處理中或限流，依 `next_action` 和回傳的等待時間續跑。6 表示平台失敗或送出結果未知，依 `code` 區分可否重試。 |

## 本機設定與資料來源

預設位置為 `~/.config/fit-fusion/config.json`，全域 `--config <path>` 可切換。`sources` 分段定義檔案或 API 與授權，`merge` 明列主來源、配對門檻及各串流的來源 ID 順位。`destinations.<id>` 分別保存各目的地設定，M2 以 Strava 發佈作為驗收。無設定時，手動雙檔入口使用同一份預設選源規則。[完整設定範例](../config/config.example.json) · [格式與檢查](../config/README.md)

- **Garmin。** 區分在 Connect 建立的 Personal API Key 與換得的 bearer access token。有效 bearer 可直接使用。需要換發時用同檔的 key 取得新 token 並更新到期時間。若 key 也到期，指引騎士至 Connect 更新。以本案既有的個人 key 啟用時填入 `personal_api_key`。[Garmin Personal API FAQ](https://developer.garmin.com/gc-developer-program/personal/personal-faq/)
- **Strava。** 騎士在設定內填自己的 app ID、secret，首次同意 `activity:write` 才能上傳及查詢上傳狀態。可選擇授權讀取以驗證平台顯示值，私密活動需要 `activity:read_all`。換發後在同一把設定檔鎖內更新 access token、最新 refresh token 與到期時間。設定目錄權限 0700，檔案及暫存檔 0600，以同目錄原子取代回存。範例、狀態、合併報告均不含認證資料。[Strava OAuth](https://developers.strava.com/docs/authentication/) · [Uploads API](https://developers.strava.com/docs/reference/#api-Uploads)
- **取得兩份原檔。** Bryton 首版接受本機 FIT，例如自車表複製。Garmin 的線上路徑查詢涵蓋騎乘起訖的日期，跨午夜包含兩天及 API 時區邊界，按候選 ID 下載原始 FIT，依 ID 與內容雜湊另存本機。來源先後到齊均可再執行。Apple Watch 留供分析。[四平台取檔調查](../research/2026-11-23-activity-platform-automation.md)

## 預設選源規則

每個可選欄位在 `merge.streams` 明列來源 ID 陣列。例如 `record.heart_rate` 的預設順序是 Bryton、CIRQA，先判整趟心率品質。`session.total_calories` 預設只有 CIRQA，按整趟選值。報告保存實際生效的選源設定與規則版本。完整的 10 種訊息、67 個公開欄位見[逐欄選源表](../specification.md#strava-fit-逐欄選源表)。

| 步驟 | 決定與可追溯的結果 |
|---|---|
| 配對同趟 | 先驗證 FIT CRC、設備與騎乘類型。以兩者實際記錄的 UTC 起訖求交集。依設定檔的 `merge.min_overlap_ratio` 判斷，預設交集須各占兩份記錄至少 90%，且僅有一筆合格。六趟索引中較長一份的重疊率為 93.8%–98.6%。無候選或多筆候選回報 ID、時段與比率，讓呼叫端指定 ID 消除歧義。手動指定仍檢查時段；局部記錄列為待判讀。 |
| 對齊與選值 | 以 Bryton 記錄為輸出時間軸，本批採 UTC 同秒、不插值。逐項套用既有順位，第二順位待定時保留缺值。經緯度成對，0 rpm 有效，保留暫停邊界。GPS、速度、高度、累積距離、踏頻、溫度沿用 Bryton。首版遇疑似定位跳點或未校正高度時在報告註記，維持現行順位。保留原始取樣供日後驗證定位與高度提案。[選值表](../specification.md#設備優先順序與合併後的數據清單) · [冷水坑對照](../decisions/2026-11-22-position-and-elevation-sources.md) |
| 心率 | 先依已確認的整趟品質判準評估缺值、累積缺口及有 Garmin 佐證的舊值停滯。一般情形僅用 Garmin 補缺，失敗則整趟改用通過檢查的 Garmin。雙方品質不足或時鐘無法對齊時回報 `quality_review`、輸出診斷，停止自動發佈。報告列出觸發區段、覆蓋率與選用樣本數。[完整判準與邊界](../decisions/2026-11-23-heart-rate-quality.md) |
| 整趟熱量 | 對可比時段直接採 CIRQA 原始 `session.total_calories`。合併 FIT 另保留 Garmin 的兩個原始數值與來源。冷水坑的標準發佈值為 1,897 kcal，Garmin 原始總量 1,897、原始代謝 383。衍生活動熱量 1,514 與 Bryton 原值 6,186 留在報告。多 session 無法一一對應、時段不可比或 Garmin 總量無效時，回報待判讀。[欄位映射與缺值行為](../decisions/2026-11-23-ride-calorie-source.md) |

M2 依逐欄表從同名 FIT 欄位擇值。session／lap 心率摘要要與選定心率涵蓋相同有效樣本，分圈熱量要能對應同一圈。缺少合格的同名摘要時標缺並記錄原因。保留 Bryton 主檔其他訊息、私有／developer 欄位的定義與值。CIRQA 熱量來源副本採獨立的 developer 命名空間，保留原欄名。完整來源取樣由原檔與分析產物保存。[Strava FIT 逐欄選源表](../specification.md#strava-fit-逐欄選源表) · [FIT 讀寫實測](../research/2026-11-23-rust-fit-cli.md)

## 產物與發佈生命週期

- **本機完成。** 來源以唯讀開啟。用暫存目錄寫出 `merged.fit`、`merge-report.json`，通過 CRC 與讀回驗證後才交付。報告含輸入雜湊、設備、原始時段、規則版本、各欄位來源與缺口區段、熱量修正、未知欄位盤點及輸出雜湊。逐筆選值保留來源檔、frame／欄位及取樣時間供回查。輸出不得覆蓋來源。既有目錄若屬相同輸入、版本與選項，驗證後重用；內容不同時要求新位置。
- **完整分析。** `merge --parquet` 加產 `merged.parquet`，保存本次所有輸入 FIT 的原始 frame（含定義、位元組、順序及無時間戳的訊息）、原始取樣與最終選值的對照。首版日常發佈無須先產生 Parquet。以重建來源 SHA-256 和查回停滯區間的兩台原始心率驗收。[原始交付要求](../CONTEXT.md)
- **送出與續查。** 發佈前驗證合併報告、以「授權的運動員 ID ＋ FIT SHA-256」鎖定本機發佈紀錄，跨目錄共用。先記錄送出意圖，再以 multipart POST 上傳 FIT，收到 `upload_id` 即保存並輪詢。取得 `activity_id` 才回報 `published`。處理中、限流或中斷均保留本機 FIT。有上傳 ID 就續查；送件後連線中斷且無 ID 時回報 `submission_unknown`，由騎士核對 Strava 活動後處置，避免重送。平台判重回報 `duplicate`，保留既有活動。日常若要讓合併檔成為 Strava 上的版本，該趟應由 FIT Fusion 先發佈，避免 Bryton 自動同步搶先建立活動。[Strava 上傳與判重](https://developers.strava.com/docs/uploads/)

## 實作順序與完成條件

下表安排單一 Rust 合併器的開發順序。逐一執行時，依案例內的輸入、操作、結束碼及預期值驗收。[合併器規格](../spec/merger.md) · [合併器驗收案例](../spec/testcases/merger.md)

| 順序 | 可交付的結果 | 驗收依據 |
|---|---|---|
| 1 | AX 入口與一趟離線合併 | 在乾淨 macOS 環境從原始碼建置 Rust／RustyFIT CLI。清空設定、斷網仍可用 help 與 skill 完成一趟雙檔合併。執行文件內範例，解析 stdout 的 JSON、結束碼與產物。 |
| 2 | 六趟實檔選值與 FIT 保存 | 10/09、10/14、11/01 只補短缺口；10/28、11/08、11/21 改用整趟 Garmin。11/21 須產出 14,149 個 Garmin 心率樣本。獨立 Python 或 C# 讀回冷水坑的標準熱量與 Garmin 保留值，逐欄比較未修改訊息；另用含同號開發者欄位的樣本驗證沒有碰撞。六趟來源 SHA-256 前後一致。[六趟回放](../research/2026-11-23-heart-rate-quality.md) |
| 3 | 只給日期完成取檔與配對 | 用六趟已有對照檔的騎乘，核對日期模式、Garmin API 與雙檔離線結果。測同日零／多趟、跨午夜、改檔名、候選不足／歧義、部分成功重跑與權杖到期。已知 ID 能解歧義。 |
| 4 | Strava 自動發佈 | 模擬處理中、成功、判重、429、送出後斷線及兩個程序同時重跑，核對 POST 次數與恢復結果。另用一趟獲授權且尚未上傳的騎乘實測，記下 upload ID、activity ID，讀回平台熱量並與 FIT 比對。模擬結果與實際匯入分別記錄。[Strava 上傳回應](https://developers.strava.com/docs/reference/#api-Uploads) |
| 5 | 分析交付 | 加 `--parquet` 後能還原兩份原檔並追查每個選值。驗證主流程不需 Apple 全量匯出。 |

第 1–3 項已完成。本機實測六個出發日，日期模式從正式 Garmin Personal API 配對及取回 CIRQA FIT，六份輸出均與手動雙檔逐位元組相同。第 4 項接續開發 Strava 授權、上傳及恢復查詢，第 5 項為 Parquet。[模組測試與 76 個決策列](../spec/testcases/module-test-coverage.md) · [日期模式驗收](../spec/testcases/merger.md#目前可重跑的驗收)


## M3 需求與研究待辦

| 項目 | 待研究或驗證的問題 |
|---|---|
| macOS 安全保管 | 用 Keychain 儲存與輪替個人 token／secret。驗證從 M2 設定遷移、CLI 更新後的存取權與鎖定時的回報。另評估 Secure Enclave 適合保護的金鑰與支援機型。[Apple Keychain](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains) |
| 正式發佈 | 比較 GitHub Releases 搭配 Homebrew 自有 tap 的方案與進入 homebrew/core 的條件。盤點授權、Apple Developer ID 簽章、公證、架構與更新驗證，以乾淨 Mac 安裝及升級為 PoC。[GitHub Releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases) · [Homebrew tap](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap) · [Apple Developer ID](https://developer.apple.com/developer-id/) |
| 其他整合 | Bryton Active 雲端原檔、Apple 逐趟匯入、其他發佈目的地，分別驗證個人授權、同趟去重及原檔保存，再決定優先順序。[平台調查](../research/2026-11-23-activity-platform-automation.md) |
| 未來版本的 rename | 使用者已要求支援不同來源欄與目的欄的明確映射。後續設計比對單位、時間範圍與語義，並定義覆寫衝突及原值追溯。排期於 M2 之後。[欄位映射驗收例](../specification.md#strava-fit-逐欄選源表) |
