# M4 CLI 規格：從雙檔覆寫開始

**已確認的範圍。** 0.2.x 開發獨立工具，0.3.0 起舊 `fit-fusion` 退場。合併與查詢分開，資料搜尋、來源種類、憑證都留在各自的 query CLI。可搬移共用程式或採用相同套件；正式執行檔不呼叫、載入或要求安裝舊工具。[開發計畫](../milestones/m4-cli.md) · [驗收條件](testcases/m4-cli.md)

## fit-merge：一次覆寫兩份 FIT

**0.2.0 的第一個開發任務。** 使用者已選好同一趟的兩份本機檔案，以及要從附加檔整條覆寫的欄位名稱。所有操作資訊均來自命令列；工具不讀取個人 config，也不需要來源 ID、帳號、API 或網路。此工具採用 clap，提供離線 help 與 skill。[所有 CLI 的共同契約](cli-common.md)

```sh
fit-merge --primary bike.fit --overlay band.fit \
  --stream record.heart_rate --stream session.total_calories --out ride.fit

fit-merge --primary ride.fit --overlay power-meter.fit \
  --stream record.power --out ride-with-power.fit
```

| 參數 | 次數 | 語意 |
|---|---|---|
| `--primary FILE` | 一次 | 輸出的基礎 FIT，提供時間軸、原始訊息與未指定欄位。 |
| `--overlay FILE` | 一次 | 本次所有指定欄位的唯一資料來源。路徑直接決定角色，與廠牌無關。 |
| `--stream NAME` | 一次以上，可重複旗標 | 每個值是一個同名 FIT 欄位，例如 `record.heart_rate`、`record.power` 或單一 session 的 `session.total_calories`。 |
| `--out FILE` | 一次 | 新的合併 FIT 路徑，來源唯讀，輸出可再作下一次的 primary。 |

### 覆寫語意

1. **整條覆寫。** 對每筆主檔 record，以 FIT 的 UTC 時間戳精確對齊附檔。附檔在該時間有有效值就寫入，該筆缺欄、值為 FIT invalid 或該時間完全無樣本就輸出缺值。即使主檔該點有值也不回填。0 W、0 rpm 等有效的零保留。沿用主檔時間軸，僅存在於附檔的時間點不新增 record。
2. **欄位選擇。** 主檔原本沒有、附檔有的欄位可明列並加入。未指定的原生欄位、developer 欄位、訊息及摘要沿用主檔，因此只指定逐點心率時，原有的 session／lap 心率摘要也沿用主檔。本版不推測感測器、不插值、不偵測停滯後自動切換來源；同名欄位優先支援 FIT Profile 中可辨識的原生欄位。
3. **整趟摘要。** 支援明列 `session.total_calories`、`session.metabolic_calories`。主檔與附檔須各有唯一可對應的同趟 session，一次只改點名的欄位。可明列兩者來保留手環的兩個熱量值。此模式沒有設備優先表。
4. **整檔缺欄與局部缺樣。** 已確認附檔完全沒有要求的欄位時，整次回報該欄名並保留既有檔案；欄位存在但值全部是 invalid 時，所選欄輸出缺值。兩檔時間完全不重疊、供覆寫的同秒樣本無法唯一判定、或摘要有多個候選時，回報原因。
5. **安全另存與串接。** 第三個輸入、遺漏必要參數、重複來源角色或輸出指向任一輸入皆回報錯誤。成功時交付可由獨立檢查器讀回的 FIT；失敗保留兩份來源與既有輸出。要用第三台裝置時重跑一遍，以前一步輸出為主檔、第三份為附檔。多次點名同一個欄位，最後一次成功操作決定該欄的值與缺口。

### 取樣分析交給 fit-inspector

`fit-inspector inspect FILE.fit` 已提供每種訊息的 `message_count`，以及實際欄位的 `sample_count` 和 `coverage_pct`。0.2.2 的 M4-5 另提供「哪些欄完全沒有、總共幾筆、有效幾筆、缺幾筆、缺多少百分比」。使用 `fit-inspector inspect band.fit --stream record.heart_rate --stream session.total_calories` 點名一或多個預期欄位。未指定時盤點實際欄位。輸出沿用 inspect 的 JSON；agent 用 `--skill inspect` 查閱。[現行盤點格式](inspector.md) · [Rust 計數實作](../../src/rust/inspector/src/inventory.rs)

| 欄位 | 計算方式（每個 FIT member、每種訊息分開） |
|---|---|
| 總筆數 | 該類型實際的訊息筆數。例如心率及功率用 record 筆數，整趟熱量用 session 筆數。 |
| 有效取樣數 | 含該欄有效值的訊息筆數；FIT invalid 或該筆缺欄都算缺樣，有效零算一筆。 |
| 缺樣數、缺樣率 | 總筆數減有效取樣數；再除以總筆數乘 100%。總筆數為零時，缺樣數為 0、比例為 `null`。 |
| 欄位狀態 | 區別「整檔未記錄這個欄位」與「欄位存在但無有效值」。兩者在有 6 筆 record 時均為有效 0、缺 6、100%。 |

以現有合成檔 `src/rust/merger/tests/fixtures/rider.fit` 為例，v0.1.0 實際讀到 6 筆 record、5 筆有效心率。按此定義，M4 報告應顯示缺 1 筆、缺樣率約 16.67%。這是「已記錄的各筆中缺欄」的度量；兩筆 record 的時間間隔另由既有 timing 呈現。分析只供使用者決定要不要覆寫，結果不會觸發合併器切換來源。

## 資料來源 CLI

三支 query 使用一致的 list、read、export 操作，各自處理來源參數。建議本機直接指定目錄或 ZIP；Garmin 授權由 `fit-query-garmin` 自行取得或讀取。0.2.2 的三支工具均可獨立執行。匯出的 FIT 路徑可交給支援 FIT 的工具。

```sh
fit-query-files --dir inbox/bike list --date 2026-11-21 --json
fit-query-files --dir inbox/bike export --date 2026-11-21 --out selected/bike --json
fit-query-garmin read --id ACTIVITY_ID --json
fit-query-garmin export --id ACTIVITY_ID --out selected/band --json
fit-query-apple --archive apple-health-export.zip export --date 2026-11-21 --out analysis/apple --json
```

| 操作 | 輸入與選擇規則 | 輸出與副作用 |
|---|---|---|
| `list` | 日期 `--date` 或期間 `--from/--until`；依查詢時區將出發日解為半開區間。優先支援騎乘，可依記錄設備／來源篩選；跨午夜歸起始日。 | 清單至少含來源 ID、來源內穩定的活動 ID、起訖、設備（未知為空）。files 另提供對應的本機 FIT 路徑。零筆為成功的空清單，與來源讀取失敗分開回報。 |
| `read --id` | 用 list 回傳的來源及活動 ID 選定一趟。 | 回傳該活動 metadata、可讀串流及其時間範圍；若只需 FIT 逐框細節，可將匯出檔交給 inspector。 |
| `export --id` 或 `export <查詢條件>` | 匯出單趟或所有符合項目至 `--out DIR`。 | JSON 逐筆回傳來源、活動 ID、檔案路徑、SHA-256、原始／轉製及結果。重跑可重用已核對的檔案；遇同名異內容保護既有檔並回報衝突。批次回報成功與失敗項目。 |

- **files** 按 FIT 內容辨識活動及設備，保留原始位元組。檔名變更應能找到同一份內容，目錄混放裝置時可用既有數字設備篩選。
- **garmin** 以 Connect 活動 ID 查詢或下載，沿用分頁、授權換發及原檔快取。核對 SHA-256 的對象是下載的原始 FIT；若 API 回傳封裝檔，先識別對應 FIT 成員。正式帳戶測試與模擬限流／失效測試分別記錄。
- **apple** 先支援本機完整匯出 ZIP 內的 Workout XML、關聯 GPX 及同一來源和時段的健康樣本。用來源／設備區別同時段的 Apple Watch 與 Connect 重複活動。穩定 ID 優先使用來源可提供的識別，否則以可重現的 Workout 身分推導並檢測碰撞。同一活動補到路線後，以新路徑另存修訂並保留舊檔。FIT 標示為轉製，保留原始取樣時間與已知單位，缺路線或缺心率時標出缺項，不補造數值。11/21 的轉製 FIT 已由 RustyFIT、Python 與 Node wire 讀回。[既有三趟 XML／GPX 樣本](../research/rides/2026-11-three-device-comparison.md)

### 查詢結果的共通欄位與輸出格式

三支工具的 `activities[]`、`activity`、`exports[]` 都含兩個可空字串。ID 仍用於選取；多筆活動可以同名。[決策紀錄](../decisions/2026-11-25-query-display-and-inspector-streams.md)

| 來源 | `activity_name`（活動標題） | `device_name`（記錄設備的可讀名稱） |
|---|---|---|
| 本機 FIT | 一般活動 FIT 缺少活動標題時為 `null`，原檔名另見 `path`。 | 依序用 FIT 產品名稱、creator 產品名稱、廠商與產品 ID 的型號表。只有未知代碼時為 `null`，代碼仍在 `device`。 |
| Garmin Connect | 取清單的 `activityName`。查詢過的標題可存在指定快取，供後續按 ID 及離線操作使用。只有 FIT 且未取得標題時為 `null`。 | 優先用本機原檔的設備識別，否則取摘要中可用的設備名稱。一般清單不額外下載 FIT，選用數字 `--device` 時才為篩選下載。 |
| Apple Health（M4-4） | 採 Workout 明確提供的標題。只有類型或寫入來源時為 `null`。 | 採 Workout 內可辨識的設備型號及對照表。11/21 的 `Watch7,1` 對應 Apple Watch Series 9，同時段的 Connect Workout 缺設備資料即為 `null`。`--source-name` 按寫入來源篩選，例如 `AndrewWatch`。 |

預設為縮排 JSON；`--json` 為相同內容的單行 JSON。`--text` 以查詢時區顯示時間；清單顯示標題、設備與可直接拿去 read/export 的 ID，read 顯示串流，export 列出每筆狀態與目的路徑。未知名稱顯示 `—`。空清單、部分成功與錯誤沿用各工具既有的結束碼。兩個格式旗標互斥，用法錯誤回傳 2。

### 本機 FIT 目錄

`--dir DIR` 是唯一必要的來源參數。遞迴搜尋副檔名為 `.fit` 的一般檔案（含大寫），略過符號連結。首版支援單一 cycling session。日期預設以 `Asia/Taipei` 解讀，可用 `--timezone ZONE` 指定 IANA 時區。`--from` 與 `--until` 成對使用。可加 `--device MANUFACTURER/PRODUCT` 篩選錄製設備。以下路徑指向本機實騎資料。

```sh
rides=data/sources/bryton/fit
fit-query-files --dir "$rides" list --date 2026-10-28 --json
fit-query-files --dir "$rides" list --date 2026-10-29 --json
id=$(fit-query-files --dir "$rides" list --date 2026-11-21 --json | jq -er '.activities[0].id')
fit-query-files --dir "$rides" read --id "$id" --json
fit-query-files --dir "$rides" export --from 2026-11-01 --until 2026-12-01 --out runtime/selected/2026-11 --json
```

| 回傳欄位 | 使用方式 |
|---|---|
| `activities[]` | 按出發時間、ID 排序。每筆含 `source_id`（目錄絕對路徑）、`id`（原始 FIT 的 SHA-256）、`path`、`sport`、UTC 起訖及設備。無設備識別時 `device` 為 `null`。同目錄樹內位元組相同的檔案只列一次，採路徑排序最前者。 |
| `activity`、`streams[]` | `read --id` 回傳活動及各 record 欄位的有效樣本數、最早／最晚有效樣本時間。時間戳用於對齊；串流另帶 FIT 欄號與 developer index，保留未知欄。無有效取樣時間時為 `null`。 |
| `exports[]` | 每筆含 `source_id`、`id`、`sha256`（原檔雜湊）、目的 `path`、`format: original` 和 `status`。沿用輸入檔名，以暫存檔完整寫入後建立新檔。`exported` 表新建、`reused` 表相同副本、`conflict` 表目的名稱已占用且無法重用、`error` 表讀寫失敗。位於來源樹內的目的子目錄會在該次匯出掃描中略過。 |
| `status`、`errors[]` | 回傳 0 表示查詢完成（含空清單）。命令用法錯誤回傳 2。來源或匯出失敗回傳 3，列出路徑及原因；同批仍有成功項目時標為 `partial`。 |

### Garmin Connect 個人活動

`fit-query-garmin` 使用 Garmin Connect Personal API。活動 ID 是 Connect 的正十進位整數，在查詢結果以字串回傳。來源 ID 為 `garmin_connect`。目前操作使用 `--env-file .env`。預設讀工作目錄的 .env。明確指定的檔案若無法使用，會回報授權錯誤。[憑證決策](../decisions/2026-11-25-garmin-dotenv.md)

| 操作 | 輸出與副作用 |
|---|---|
| `list --date DAY` 或 `--from DAY --until DAY` | 逐頁取得騎乘、按活動 ID 去重，再依查詢時區的出發日篩選。預設台北時區、迄日不含當天；省略日期可查全部。一般清單有經雜湊核對的本機原檔時可帶出設備，無資料時為空。選用 `--device 1/5019` 時，會按需下載原始 FIT 核對錄製設備。 |
| `read --id ID` | 從 FIT 讀回騎乘的起訖、設備、串流、有效取樣數及時間。回傳 `sha256`、`path` 與 `cache_hit`。支援單一騎乘 session 的摘要。 |
| `export --id ID --out DIR` 或按日期／期間／設備匯出 | 直接 FIT 或 ZIP 中唯一的 FIT 會保留原始位元組。每筆使用 `ID.fit`，同內容回報 `reused`、同名異內容回報 `conflict`。批次報告保留成功與失敗 ID。發生平台授權、限流或服務錯誤後，餘項標示 `not_attempted`。 |
| `--cache-dir DIR`、`read/export --id ID --offline` | 預設快取為 `~/Library/Caches/fit-query-garmin`。按 ID 與 SHA-256 儲存，寫入以每 ID 的鎖與暫存檔保護。離線命中可在無憑證時使用，雜湊失敗或未下載的 ID 回報 `cache_miss`。不同帳號可指定不同的快取目錄。 |

結束碼 0 表成功（含零筆）；2 表語法或日期組合錯誤；3 表活動、快取或輸出問題；4 表應檢查 .env 與金鑰；5 表限流，`retry_after` 保留服務端給的等待資訊；6 表服務或回應異常。短效 bearer 過期時重換；若 Garmin 拒絕現有 bearer，至多換發並重試一次。個人 key 到期後需至 Garmin Connect 重新核發。[Garmin 官方說明](https://support.garmin.com/en-US/?faq=HfTwLfw1Gn2A7WS1GdAMH8)

### Apple Health 完整匯出 ZIP

`fit-query-apple --archive FILE` 以 XML 根元素 HealthData 識別活動資料，支援在地化 XML 檔名。list 依出發日及寫入來源選取自行車 Workout。read/export 以同來源、相容設備與完整落在運動內的區間挑選健康樣本，另讀取該 Workout 引用的 GPX。ZIP 維持唯讀；每次查詢仍須掃描完整主 XML。

- 轉製 FIT 在 file_id.product_name 記錄 Apple Health conversion、寫入來源及可辨識型號。逐筆保留心率、踏頻、功率、速度與 GPX 座標和高度。秒以下時間、距離增量和樣本區間長度使用具名 developer 欄位。以 Workout 明示的距離、活動熱量與時長填入摘要。
- `coverage` 列出路線點數、缺少的心率／路線及遺失的 GPX 參照。相同來源名稱也可記錄多趟活動。活動 ID 優先取來源與 external UUID 的雜湊，否則取來源、硬體、運動類型及出發時間。碰撞時回報歧義。
- 匯出檔名為 `<workout-id>-<FIT SHA-256>.fit`。相同內容重用，後續 ZIP 補入資料後另存新版本。衝突檔保留。一般錯誤回傳 3，參數錯誤回傳 2，成功及空清單回傳 0。

## 執行與建置邊界

```mermaid
flowchart LR
    Q["三支 fit-query 工具"] -->|逐趟 FIT| D["本機檔案"]
    D -->|指定主檔與附檔| M["fit-merge"]
    M --> O["新的 FIT"]
    O -->|下一次的主檔| M
    D --> I["fit-inspector"]
    O --> I
```

圖中的箭頭代表使用者傳遞檔案。每支工具可單獨使用。可將原程式內需要的 FIT 讀寫函式搬入獨立 Rust 模組或函式庫，並重用 Cargo 套件。各新工具自帶 `--help`、`--version`、離線 `--skill index` 與使用範例；不載入 `fit-fusion` 可執行檔、其設定或其執行時服務。
