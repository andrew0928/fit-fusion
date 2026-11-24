# FIT Fusion 個人設定檔 v1

2026-11-23 已確認此格式作為 M2 的實作依據。[決策紀錄](../decisions/2026-11-23-date-driven-config.md)

設定一次資料來源與串流順位後，日常操作只需給騎乘出發日期。M2-3 可按日期輸出合併 FIT 與報告；下列第二條為 M2-4 的發佈介面。

```sh
fit-fusion merge --date 2026-11-21
fit-fusion merge --date 2026-11-21 --publish strava --json
```

預設檔案為 `~/.config/fit-fusion/config.json`。執行 `fit-fusion config init` 可建立權限 0600、含完整預設順位的範本；已有設定會保留原檔，再調整來源目錄與憑證。`fit-fusion config check` 可在離線狀態檢查結構與來源引用。`--config <path>` 可指定另一份設定。[完整 JSON 範例](config.example.json) · [JSON Schema](config-v1.schema.json)

## 檔案結構

| 欄位 | 用途 |
|---|---|
| `version` | 整數 `1`，表示設定檔契約版本。 |
| `timezone` | 解讀命令列日期的 IANA 時區。範例為 `Asia/Taipei`，跨裝置配對一律用 UTC。 |
| `output_dir` | 每趟合併 FIT 與報告的根目錄。 |
| `sources.<id>` | 一個以上的具名來源實例。ID 供合併規則引用；`kind` 決定取檔方式，設備來源的 `device` 決定辨識哪種設備。 |
| `merge.primary_source` | 從哪個來源找出當日騎乘，並以該份活動建立主時間軸。本版的騎乘規格使用 Rider 460。 |
| `merge.min_overlap_ratio` | 配對時雙方各自須達到的重疊比例。範例為 `0.90`。 |
| `merge.streams` | 要寫入合併 FIT 的感測串流與熱量摘要。鍵是同名 FIT 欄位，值是由優先到備援的來源 ID 陣列。 |
| `destinations.<id>` | 零個以上的具名目的地實例。`kind` 指定發佈連線器。僅做本機匯出可省略整段，也可填 `{}`。 |

路徑接受絕對路徑、`~/` 與相對於**設定檔所在目錄**的路徑。來源目錄、下載快取與輸出目錄各自獨立。設定檔為 JSON，重複鍵、未知鍵或未支援的版本都回報設定錯誤。

## 如何增加設備或目的地

來源與目的地各有獨立的 ID 空間。ID 可自行命名，例如 `old_bike`；同一種 `kind` 可建立多個實例。下列 JSON 是設定片段：在既有來源集合加入一台 Bryton Neo15，並設定三個獨立目的地。Neo15 及兩個額外發佈服務以測試註冊項目示範，平台授權與上傳能力另行驗證。

```json
{
  "sources": {
    "old_bike": {"kind": "fit_directory", "device": "bryton_neo15", "directory": "~/Rides/inbox/neo15"}
  },
  "destinations": {
    "strava": {"kind": "strava", "client_id": null},
    "bike_log": {"kind": "bryton_activity", "options": {}},
    "connect_log": {"kind": "garmin_connect", "options": {}}
  }
}
```

- **新設備。** `fit_directory` 可重用於已註冊的新型號。Schema 檢查 `device` 的識別字格式，loader 依設備註冊資訊檢查取檔方式是否相容。Neo15 的實際 FIT 製造商與產品編號待原檔確認。加入來源供列舉後，只有明確配置到主來源或串流順位才參與合併，並須符合該規則的設備條件。
- **新連線器。** 新來源或目的地共用 `{"kind":"…","options":{…}}` 形式；`options` 可省略，預設為空物件。基礎 Schema 能驗證外層結構。loader 依方向查找已編入的來源或目的地 factory，再由該 factory 離線驗證自己的選項，包括必要鍵、未知鍵與型別。未知種類、未知設備或不相容的設備皆回報含設定位置的 `ConfigError`。例如註冊 Garmin 下載連線器只提供讀取能力，要將 Garmin 列為目的地另須註冊發佈連線器。
- **既有 v1 相容。** 舊 `destinations.strava` 可省略 `kind`，此時按 Strava 的既有憑證格式解讀。其他目的地 ID 須填 `kind`；自訂 ID 搭配 `kind: strava` 也適用相同的嚴格憑證欄位檢查。發佈以明確選取的目的地 ID 為單位。

可直接執行的 [本機 FIT 設定](local-fit.example.json) 與 [驗證步驟](../../README.md#驗證本機-fit-目錄來源) 使用同一目錄內的合成 Bryton／Garmin 活動。

## 每個來源如何取得資料

| 範例來源 | 必要設定 | 日期模式的用途 |
|---|---|---|
| `sources.bryton` | `kind: fit_directory`、`directory`、`device: bryton_rider_460` | 遞迴讀取目錄內的 `.fit`（含大寫副檔名）。從 FIT 內的設備、運動與時間辨識主活動，改檔名不影響結果。 |
| `sources.garmin` | `kind: garmin_connect`、`device: garmin_cirqa`、`cache_dir`、`auth` | 由內建的 Garmin Connect Personal API 連線器依時段列舉活動，按活動 ID 下載原始 FIT 並快取。已取得且通過雜湊驗證的同一版本可重用。 |
| `sources.apple_watch` | `kind: apple_health_export`、`archive` | 登記 Apple 健康匯出 ZIP 供分析。日常日期合併不讀此檔，亦可省略此來源。 |

**Garmin 也可完全離線。** 已有兩台設備的 FIT 時，只要將範例中 `sources.garmin` 的內容替換如下，心率及熱量的順位、日期命令均可沿用。專案現有兩個來源目錄分別是 `data/sources/bryton/fit/` 及 `data/sources/garmin/`。

```json
{"kind": "fit_directory", "device": "garmin_cirqa", "directory": "~/Rides/inbox/garmin"}
```

`sources.garmin.auth` 可放 `personal_api_key`、`access_token`、`expires_at`。可用 bearer 直接查詢，屆期由個人 key 換發。Strava 段可放 `client_id`、`client_secret`、`athlete_id`、`access_token`、`refresh_token`、`expires_at`。兩平台的到期時間均為 Unix 秒數（UTC），識別碼以字串保存。範本中的 `null` 或省略表示尚未設定，執行需要該連線的操作時才檢查授權。更新憑證時對同一個設定檔路徑加鎖並原子回存，檔案權限 0600。報告以來源 ID 與內容雜湊追溯資料，敏感欄位不寫入報告。[認證與換發](../milestones/m2-cli.md#本機設定與資料來源)

## 每個串流如何取值

以下是完整範例中三條規則的節錄。順序由左至右，每個來源只需宣告一次。

```json
{
  "record.heart_rate": ["bryton", "garmin"],
  "record.cadence": ["bryton"],
  "session.total_calories": ["garmin"]
}
```

- 一般 `record.*` 在主時間軸的每一點依順位選第一個有效同名欄位。只有一個來源就保留其缺口。兩個 GPS 欄位須成對設定、順位相同且從同一筆樣本取值。踏頻 0 rpm 有效。`speed`、`enhanced_speed` 是兩個可獨立選取的欄位。M2 的設定語法只支援同名選值。
- `record.heart_rate` 使用整趟品質判定。預設用 Bryton，短缺口才補 Garmin。Bryton 品質不佳時，整趟使用經檢查的第二順位，該來源的缺口留空。缺少可用備援時標示待判讀。版本 1 的停滯判準以 Rider 460 對 CIRQA 校驗，適用的設備與順序列在語意檢查中。[心率品質決策](../decisions/2026-11-23-heart-rate-quality.md)
- `session.*` 以可比較的整趟活動為選值單位。預設選 CIRQA 的 `total_calories` 與 `metabolic_calories`。選用總熱量時照規格另存該趟 Garmin 的原始總量與代謝量。活動熱量計算在報告與分析輸出。若已要求整趟熱量而 Garmin 缺檔或時段不可比，該趟回報待判讀，保留診斷。[熱量欄位決策](../decisions/2026-11-23-ride-calorie-source.md)
- 清單列出哪些可選欄位，就產生哪些。刪除例如 `record.temperature`，正式溫度串流與相應溫度摘要便不輸出，來源原值仍可追溯。FIT 的 `file_id`、時間軸、事件等結構欄位由主來源提供。其他活動／分圈摘要沿用逐欄規格，受選值影響的摘要須能對上同一批樣本。原始未知欄位及來源副本依保存規格處理。這樣設定檔只需列騎士要挑選的數據。[Strava 逐欄選源表](../specification.md#strava-fit-逐欄選源表)

## 只給日期時的處理契約

1. **列出當日主活動。** 將 `--date YYYY-MM-DD` 解為設定時區的日曆日。從 `merge.primary_source` 找出騎乘開始時間落在「當日零時到次日零時，含起不含迄」的所有活動。來源 FIT 以 session 起點及經過時間取得完整活動區間，跨午夜歸出發日。相同來源檔的 SHA-256 只處理一次。
2. **找出需要的來源。** 從已選 streams 收集來源 ID，對每趟主活動分別查詢。例如六個欄位用到 Garmin 也只需配對一次。依完整活動區間列舉可重疊的騎乘，含提早開錄和跨日候選。若 API 以開始日查詢，對長度為 `D`、門檻 `r` 的主活動，可從主起點前 `D × (1/r − 1)` 查至主終點，轉成 API 日曆日期並取完分頁。下載後再依原檔設備及 UTC 時段判定。
3. **逐趟配對。** 對主區間 A、候選區間 B 計算交集秒數 I，要求 `I / duration(A) >= r` 且 `I / duration(B) >= r`，兩區間須有正長度。每個所需來源有唯一合格候選才自動採用。相同活動 ID 或相同原檔先去重。多個不同候選符合，或同一候選同時符合兩趟主活動時，標出有歧義的配對，列回來源、ID、起訖與兩個比率。可用既有的指定雙檔或 Garmin ID 入口選定該趟。
4. **合併與重跑。** 依主活動開始時間、來源識別穩定排序，每趟各自執行選值與品質檢查。輸出依 `<output_dir>/<出發日>/<主檔 SHA-256>/<修訂 ID>/` 分開。修訂 ID 由實際輸入雜湊、有效選源及品質規則版本和輸出選項產生，排除本機路徑及 token。相同輸入重跑會重用已驗證產物。補到新來源、變更順位則產生另一修訂。
5. **回報整天結果。** `--json` 回傳 `date`、`timezone`、`rides[]` 及整批狀態。當日有兩趟就分別處理兩趟。成功的一趟可正常輸出，缺候選則依該串流的缺值規則處理，有歧義則該趟暫停並回報 `ambiguous_match`。部分完成時回傳 `partial` 與逐趟原因；批次結束碼取未完成項目中最大的既有錯誤碼。當日確實沒有主活動回傳 `no_activities`、空陣列、結束碼 0。來源無法讀取或 API 查詢失敗須回報錯誤。僅 `--publish strava` 會發佈已驗證完成的趟次，重跑沿用發佈紀錄。[M2 結束碼與發佈契約](../milestones/m2-cli.md)

## 驗證邊界

JSON Schema 驗證鍵名、資料型別、來源區段、可選 FIT 欄位與非空且不重複的順位。載入器另須執行以下語意檢查，錯誤要指出設定檔內的位置，例如 `/merge/streams/record.heart_rate/1`。

- 時區須存在。主來源及所有順位項目須指向已宣告的來源。主來源須能列舉 Rider 460 騎乘；Apple 匯出只能用於分析。來源依 FIT 製造商與產品組合確認設備，Rider 460 為 `(267, 2205)`，CIRQA 為 `(1, 5019)`。
- GPS 兩欄順位須相同。預設心率規則支援 Rider 460 首選與可選的 CIRQA 次選；已選整趟熱量時沿用已確認的 CIRQA 來源，兩個整趟熱量欄若皆啟用須取自同一份 session。來源 ID 改名並同步修改引用仍有效。
- 檢查展開後的實際路徑，避免把輸出或快取當成主活動反覆讀入。缺少 token、設備未掛載、API 限流是執行時各自的狀態。靜態 `config check` 不讀取私人健康檔、不發網路請求。

以現有索引的六筆 Bryton 與六筆 Garmin 時段交叉比對，`0.90` 門檻下六趟各有唯一候選。10/14、10/28 兩趟均在次日結束；11/21 的雙方較低重疊比為 96.97%。M2-3 已驗同日兩趟、歧義候選、目錄遺失、跨午夜及重跑；六趟實際 Connect 輸出與本機雙檔逐位元組相同。[六趟驗證資料說明](../research/2026-11-23-heart-rate-quality.md)
