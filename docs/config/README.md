# FIT Fusion 個人設定

從 0.2.3 起，三支 `fit-query-*` 共用選填的 `~/.fit-fusion/config.yaml`。`fit-merge` 與 `fit-inspector` 直接用命令列參數執行。[英文註解範例](config.example.yaml) · [完整 YAML schema](query-config-v1.schema.yaml) · [欄位預設、覆寫順序與驗收條件](../spec/cli-common.md)

### 從 0.2.x 升級到五支獨立工具

升級至公開版可執行 `brew update && brew upgrade andrew0928/tools/fit-fusion`。使用開發版或升級後的工具執行 `fit-query-files config init`，即可建立全註解的英文範本。按需啟用 `version: 1`、區段及設定值。多次 init、重裝與升級均保留個人檔。直接從 GitHub 下載的 binary 使用同一條 init 命令。

0.2.x 的舊 `fit-fusion` 仍可讀它的 JSON 設定，0.3.0 移除此執行檔。將常用 FIT 目錄、Apple Health ZIP 或 Garmin dotenv 路徑填入新 YAML 的工具區段。以前用 `fit-fusion merge --date` 完成的流程，現在先用來源對應的 `fit-query-*` 查詢、匯出當天的 FIT，再由 `fit-merge` 明確挑選附檔整條覆寫的欄位。以兩個已選出的活動為例：

```sh
fit-query-files list --date 2026-11-21
fit-query-garmin list --date 2026-11-21
fit-query-files export --id "$BIKE_ACTIVITY_ID" --out ride/bike
fit-query-garmin export --id "$GARMIN_ACTIVITY_ID" --out ride/band
```

從兩次匯出的 `exports[].path` 取得主檔與附檔路徑，分別代入 `$BIKE_FIT`、`$GARMIN_FIT`。以下只取代整趟熱量，心率沿用主檔。要改用整條 Garmin 心率可再加 `--stream record.heart_rate`。

```sh
fit-merge --primary "$BIKE_FIT" --overlay "$GARMIN_FIT" --stream session.total_calories --out ride/merged.fit
fit-inspector inspect ride/merged.fit
```

檢查後，在 Strava 網頁的活動上傳頁選擇 `ride/merged.fit`。若需確認 CLI 選用哪個目錄或時區，可查看執行時的 stderr 或 `fit-query-files --skill config`。

## 0.2.x 舊 fit-fusion 的 JSON 設定（歷史規格）

2026-11-23 已確認此格式作為 M2 的實作依據。[決策紀錄](../decisions/2026-11-23-date-driven-config.md)

設定一次資料來源與串流順位後，日常操作只需給騎乘出發日期。預設輸出合併 FIT 與報告供手動上傳。第二條另存可追溯原值的分析檔。

```sh
fit-fusion merge --date 2026-11-21
fit-fusion merge --date 2026-11-21 --parquet --json
```

預設檔案為 `~/.config/fit-fusion/config.json`。執行 `fit-fusion config init` 可建立權限 0600 的範本，已有設定會保留原檔。填好車錶目錄及手環來源即可使用範本的心率與熱量規則。`fit-fusion config check --json` 可離線檢查設定，結果的 `streams` 只列明確指定的規則。`--config <path>` 可指定另一份設定。[完整 JSON 範例](config.example.json) · [JSON Schema](config-v1.schema.json)

## 檔案結構

| 欄位 | 用途 |
|---|---|
| `version` | 整數 `1`，表示設定檔契約版本。 |
| `timezone` | 解讀命令列日期的 IANA 時區。範例為 `Asia/Taipei`，跨裝置配對一律用 UTC。 |
| `output_dir` | 每趟合併 FIT 與報告的根目錄。 |
| `sources.<id>` | 一個以上的具名來源實例。ID 供合併規則引用；`kind` 決定取檔方式，本機 FIT 的 `device` 可選填以篩選型號。 |
| `merge.primary_source` | 從哪個來源找出當日騎乘，並以該份活動建立主時間軸。預設使用 `bike_meter`，可收錄先後使用的多台車錶。 |
| `merge.min_overlap_ratio` | 配對時雙方各自須達到的重疊比例。範例為 `0.90`。 |
| `merge.streams` | 選填。只列需要覆寫或從其他來源追加的同名 FIT 欄位，值為依順位排列的來源 ID。省略整段或填 `{}` 都會完整沿用主檔。 |
| `merge.field_ids` | 選填。將內附 FIT Profile 尚未收錄的 `record.*` 名稱對應到原生欄號。 |
| `destinations.<id>` | 零個以上的具名目的地實例。`kind` 指定發佈連線器。僅做本機匯出可省略整段，也可填 `{}`。 |

路徑接受絕對路徑、`~/` 與相對於**設定檔所在目錄**的路徑。來源目錄、下載快取與輸出目錄各自獨立。設定檔為 JSON，重複鍵、未知鍵或未支援的版本都回報設定錯誤。

## 如何增加設備或目的地

來源 ID 按用途命名。新版範本使用 `bike_meter`、`health_band` 與 `apple_watch`。來源 ID 決定合併順位，`kind` 決定如何取檔，活動清單和合併報告的 `device` 則識別實際錄製該檔案的設備。已有 v1 設定可繼續使用原 ID；手動改名時一併更新 `merge.primary_source` 和每個 `merge.streams` 引用。

本機車錶可共用一個專屬目錄。省略 `device` 後，Rider 460、Rider 15 Neo 及後續車錶都可使用同一組順位。若同一目錄混放車錶與手環，可用以下數字篩選器區分來源。範例採用已驗證的 Rider 15 Neo 代碼，並示範三個目的地。

```json
{
  "sources": {
    "old_bike": {"kind": "fit_directory", "device": {"manufacturer": 267, "product": 2004}, "directory": "~/Rides/inbox/neo15"}
  },
  "destinations": {
    "strava": {"kind": "strava", "client_id": null},
    "bike_log": {"kind": "bryton_activity", "options": {}},
    "connect_log": {"kind": "garmin_connect", "options": {}}
  }
}
```

- **新設備。** `device` 可用已註冊別名，或填入數字物件 `{"manufacturer": 267, "product": 2004}`。兩個數字來自 Rider 15 Neo 的 `file_id`。從新車錶原檔的 `file_id` 讀取兩個數字後即可套用同一個 CLI。Garmin Connect 只接受 Garmin 製造商識別。
- **新連線器。** 新來源或目的地共用 `{"kind":"…","options":{…}}` 形式；`options` 可省略，預設為空物件。基礎 Schema 能驗證外層結構。loader 依方向查找已編入的來源或目的地 factory，再由該 factory 離線驗證自己的選項，包括必要鍵、未知鍵與型別。未知種類、未知設備或不相容的設備皆回報含設定位置的 `ConfigError`。例如註冊 Garmin 下載連線器只提供讀取能力，要將 Garmin 列為目的地另須註冊發佈連線器。
- **既有 v1 相容。** 舊 `destinations.strava` 可省略 `kind`，此時按 Strava 的既有憑證格式解讀。其他目的地 ID 須填 `kind`；自訂 ID 搭配 `kind: strava` 也適用相同的嚴格憑證欄位檢查。既有目的地設定可讀；本機 FIT 供使用者手動上傳 Strava。

可直接執行的 [本機 FIT 設定](local-fit.example.json) 與 [驗證步驟](../../README.md#驗證本機-fit-目錄來源) 使用同一目錄內的合成 Bryton／Garmin 活動。

## 每個來源如何取得資料

| 範例來源 | 必要設定 | 日期模式的用途 |
|---|---|---|
| `sources.bike_meter` | `kind: fit_directory`、`directory`，`device` 選填 | 遞迴讀取目錄內的 `.fit`（含大寫副檔名）。從 FIT 內的設備、運動與時間辨識主活動，改檔名不影響結果。 |
| `sources.health_band` | `kind: garmin_connect`、`device: {"manufacturer":1,"product":5019}`、`cache_dir`、`auth` | 由內建的 Garmin Connect Personal API 連線器依時段列舉活動，按活動 ID 下載原始 FIT 並快取。已取得且通過雜湊驗證的同一版本可重用。 |
| `sources.apple_watch` | `kind: apple_health_export`、`archive` | 登記 Apple 健康匯出 ZIP 供分析。日常日期合併不讀此檔，亦可省略此來源。 |

**Garmin 也可完全離線。** 已有兩台設備的 FIT 時，只要將範例中 `sources.health_band` 的內容替換如下，心率及熱量的順位、日期命令均可沿用。專案現有兩個來源目錄分別是 `data/sources/bryton/fit/` 及 `data/sources/garmin/`。

```json
{"kind": "fit_directory", "directory": "~/Rides/inbox/health_band"}
```

`sources.health_band.auth` 可放 `personal_api_key`、`access_token`、`expires_at`。可用 bearer 直接查詢，屆期由個人 key 換發。Strava 段可放 `client_id`、`client_secret`、`athlete_id`、`access_token`、`refresh_token`、`expires_at`。兩平台的到期時間均為 Unix 秒數（UTC），識別碼以字串保存。範本中的 `null` 或省略表示尚未設定，執行需要該連線的操作時才檢查授權。更新憑證時對同一個設定檔路徑加鎖並原子回存，檔案權限 0600。報告以來源 ID 與內容雜湊追溯資料，敏感欄位不寫入報告。[認證與換發](../milestones/m2-cli.md#本機設定與資料來源)

若已有主記錄，執行 `fit-fusion --config rides.json merge PRIMARY.fit --auto-match --out merged-ride`。CLI 逐一向 `merge.streams` 引用的次要來源尋找同趟活動。每個來源的 `kind` 決定從本機 FIT 目錄或 Garmin Connect 取檔。

### 從 FIT 取得設備名稱

本機 `activities list --source bike_meter --json` 的每筆 `activities[].device`、合併報告的 `inputs.<來源 ID>.device` 都包含 `manufacturer`、`product`、`name` 和 `name_source`。依下列順序取第一個可用名稱。

| `name_source` | 如何取得 `name` |
|---|---|
| `file_id.product_name` | FIT 內的產品名稱文字。 |
| `device_info.product_name` | 限建立檔案的設備（`device_index=0`），如有製造商及型號須與 `file_id` 相符。 |
| `model_lookup` | 用原檔的兩個代碼查已驗證的型號。例如 `267/2004` 為 Bryton Rider 15 Neo，`267/2205` 為 Bryton Rider 460，`1/5019` 為 Garmin CIRQA。 |
| `fit_ids` | 未收錄型號顯示 `FIT <manufacturer>/<product>`。 |

目前合併器要求原始 FIT 提供有效的製造商與產品代碼，缺碼時回報 `invalid_data`。Garmin API 列表先回傳 `device: null`，開啟下載的原始 FIT 後才確認設備。Apple Health ZIP 維持分析用途。手動給兩個 FIT 時，CLI 依型號篩選器判別角色；兩種排列都成立時，請以各角色專屬目錄執行日期模式，或為其中一個角色設定型號篩選器。[Garmin FIT 檔案識別](https://developer.garmin.com/fit/file-types) · [Activity 裝置資訊](https://developer.garmin.com/fit/file-types/activity)

## 每個串流如何取值

日期合併以 `primary_source` 的 FIT 為底。若只想把整趟總熱量換成 Garmin 的值，整個 `streams` 可以只寫 `{"session.total_calories": ["health_band"]}`。GPS、速度、踏頻、心率及主檔日後新增的功率都會沿用。

目前 `config init` 產生的 `merge.streams` 如下。第一條啟用車錶心率與手環備援，後兩條分別指定 Garmin CIRQA 的兩個原生熱量欄位。

```json
{
  "record.heart_rate": ["bike_meter", "health_band"],
  "session.total_calories": ["health_band"],
  "session.metabolic_calories": ["health_band"]
}
```

- 一般 `record.*` 在主時間軸的每一點依順位選第一個有效同名欄位。只有一個來源就保留其缺口。兩個 GPS 欄位須成對設定、順位相同且從同一筆樣本取值。踏頻 0 rpm 有效。`speed`、`enhanced_speed` 是兩個可獨立選取的欄位。M2 的設定語法只支援同名選值。
- `record.heart_rate` 使用整趟品質判定。預設用 Bryton，短缺口才補 Garmin。Bryton 品質不佳時，整趟使用經檢查的第二順位，該來源的缺口留空。缺少可用備援時標示待判讀。版本 1 的停滯判準以 Rider 460 對 CIRQA 校驗，適用的設備與順序列在語意檢查中。[心率品質決策](../decisions/2026-11-23-heart-rate-quality.md)
- `session.*` 以可比較的整趟活動為選值單位。預設選 CIRQA 的 `total_calories` 與 `metabolic_calories`。選用總熱量時照規格另存該趟 Garmin 的原始總量與代謝量。活動熱量計算在報告與分析輸出。若已要求整趟熱量而 Garmin 缺檔或時段不可比，該趟回報待判讀，保留診斷。[熱量欄位決策](../decisions/2026-11-23-ride-calorie-source.md)
- 未列在 `merge.streams` 的串流、摘要與 developer 欄位從主檔沿用。移除 `record.temperature` 的覆寫後，就會保留主檔的溫度與溫度摘要。若跨來源改變某串流，對應且無法成立的既有摘要會移除。

### 新感測器與新欄位

功率計配對到車錶後，只要該車錶是主來源，便會自動保留原生 `record.power` 的瓦數。主時間軸每一秒若要由第二台補缺，在現有 `merge.streams` 加入 `"record.power": ["bike_meter", "health_band"]`。保留 0 W；所有來源都缺時保留缺值，無插值。用 `--config FILE merge --date YYYY-MM-DD` 可由日期自動配對，手動雙檔若要套用自己的順位也應明確指定 `--config FILE`。

內附 FIT Profile 的原生欄位可直接用名稱，例如 `record.left_right_balance` 或陣列 `record.left_power_phase`。若設備寫入 Profile 尚未收錄的原生欄，可加入 `"field_ids": {"record.extra_sensor": 240}` 到 `merge`，再在 `streams` 指定其順位。欄號應由來源規格或檢查器確認；別名不得覆寫已知欄位或共用同一個號碼。陣列依整筆樣本選源，保留所有元素。跨檔選源使用原生欄位；主檔的 developer 欄位直接保留。全新 API 或新格式的解碼仍需要對應的連線器。[可執行的雙設備範例](stream-fit.example.json) · [v1 行為變更](../decisions/2026-11-24-config-primary-inheritance.md)

## 只給日期時的處理契約

1. **列出當日主活動。** 將 `--date YYYY-MM-DD` 解為設定時區的日曆日。從 `merge.primary_source` 找出騎乘開始時間落在「當日零時到次日零時，含起不含迄」的所有活動。來源 FIT 以 session 起點及經過時間取得完整活動區間，跨午夜歸出發日。相同來源檔的 SHA-256 只處理一次。
2. **找出需要的來源。** 從已選 streams 收集來源 ID，對每趟主活動分別查詢。例如六個欄位用到 Garmin 也只需配對一次。依完整活動區間列舉可重疊的騎乘，含提早開錄和跨日候選。若 API 以開始日查詢，對長度為 `D`、門檻 `r` 的主活動，可從主起點前 `D × (1/r − 1)` 查至主終點，轉成 API 日曆日期並取完分頁。下載後再依原檔設備及 UTC 時段判定。
3. **逐趟配對。** 對主區間 A、候選區間 B 計算交集秒數 I，要求 `I / duration(A) >= r` 且 `I / duration(B) >= r`，兩區間須有正長度。每個所需來源有唯一合格候選才自動採用。相同活動 ID 或相同原檔先去重。多個不同候選符合，或同一候選同時符合兩趟主活動時，標出有歧義的配對，列回來源、ID、起訖與兩個比率。可用既有的指定雙檔或 Garmin ID 入口選定該趟。
4. **合併與重跑。** 依主活動開始時間、來源識別穩定排序，每趟各自執行選值與品質檢查。輸出依 `<output_dir>/<出發日>/<主檔 SHA-256>/<修訂 ID>/` 分開。修訂 ID 由實際輸入雜湊、有效選源及品質規則版本和輸出選項產生，排除本機路徑及 token。相同輸入重跑會重用已驗證產物。補到新來源、變更順位則產生另一修訂。
5. **回報整天結果。** `--json` 回傳 `date`、`timezone`、`rides[]` 及整批狀態。當日有兩趟就分別處理兩趟。成功的一趟可正常輸出，缺候選則依該串流的缺值規則處理，有歧義則該趟暫停並回報 `ambiguous_match`。部分完成時回傳 `partial` 與逐趟原因；批次結束碼取未完成項目中最大的既有錯誤碼。當日確實沒有主活動回傳 `no_activities`、空陣列、結束碼 0。來源無法讀取或 API 查詢失敗須回報錯誤。騎士可從成功項目的 `artifacts.fit` 取得 FIT，自行上傳 Strava。[M2 結束碼與發佈契約](../milestones/m2-cli.md)

## 驗證邊界

JSON Schema 驗證鍵名、資料型別、來源區段、可選 FIT 欄位與非空且不重複的順位。載入器另須執行以下語意檢查，錯誤要指出設定檔內的位置，例如 `/merge/streams/record.heart_rate/1`。

- 時區須存在。主來源及所有順位項目須指向已宣告的來源。主來源須能列舉並開啟單一騎乘 FIT；Apple 匯出只能用於分析。來源依 FIT 製造商與產品組合確認設備，Rider 460 為 `(267, 2205)`，CIRQA 為 `(1, 5019)`。
- GPS 兩欄順位須相同。心率品質規則接受一個首選及最多一個備援，預設為 Rider 460 與 CIRQA；整趟熱量使用單一設定來源，預設仍為 CIRQA，兩個整趟熱量欄若皆啟用須取自同一份 session。來源 ID 改名並同步修改引用仍有效。
- 檢查展開後的實際路徑，避免把輸出或快取當成主活動反覆讀入。缺少 token、設備未掛載、API 限流是執行時各自的狀態。靜態 `config check` 不讀取私人健康檔、不發網路請求。

以現有索引的六筆 Bryton 與六筆 Garmin 時段交叉比對，`0.90` 門檻下六趟各有唯一候選。10/14、10/28 兩趟均在次日結束；11/21 的雙方較低重疊比為 96.97%。M2-3 已驗同日兩趟、歧義候選、目錄遺失、跨午夜及重跑；六趟實際 Connect 輸出與本機雙檔逐位元組相同。[六趟驗證資料說明](../research/2026-11-23-heart-rate-quality.md)
