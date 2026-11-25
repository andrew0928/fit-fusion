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

`fit-inspector inspect FILE.fit` 已提供每種訊息的 `message_count`，以及實際欄位的 `sample_count` 和 `coverage_pct`。M4-5 擬補上「哪些欄完全沒有、總共幾筆、有效幾筆、缺幾筆、缺多少百分比」。可提案用 `fit-inspector inspect band.fit --stream record.heart_rate` 點名預期欄位；未指定時盤點實際欄位。[現行盤點格式](inspector.md) · [Rust 計數實作](../../src/rust/inspector/src/inventory.rs)

| 欄位 | 計算方式（每個 FIT member、每種訊息分開） |
|---|---|
| 總筆數 | 該類型實際的訊息筆數。例如心率及功率用 record 筆數，整趟熱量用 session 筆數。 |
| 有效取樣數 | 含該欄有效值的訊息筆數；FIT invalid 或該筆缺欄都算缺樣，有效零算一筆。 |
| 缺樣數、缺樣率 | 總筆數減有效取樣數；再除以總筆數乘 100%。總筆數為零時，缺樣數為 0、比例為 `null`。 |
| 欄位狀態 | 區別「整檔未記錄這個欄位」與「欄位存在但無有效值」。兩者在有 6 筆 record 時均為有效 0、缺 6、100%。 |

以現有合成檔 `src/rust/merger/tests/fixtures/rider.fit` 為例，v0.1.0 實際讀到 6 筆 record、5 筆有效心率。按此定義，M4 報告應顯示缺 1 筆、缺樣率約 16.67%。這是「已記錄的各筆中缺欄」的度量；兩筆 record 的時間間隔另由既有 timing 呈現。分析只供使用者決定要不要覆寫，結果不會觸發合併器切換來源。

## 資料來源 CLI

三支 query 使用一致的 list、read、export 操作，各自處理來源參數。建議本機直接指定目錄或 ZIP；Garmin 授權由 `fit-query-garmin` 自行取得或讀取。files 的命令可在本機執行，Garmin 與 Apple 命令保留為後續規格。匯出的 FIT 路徑可交給支援 FIT 的工具。

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
- **apple** 先支援本機完整匯出 ZIP 內的 Workout XML、關聯 GPX 及同一來源和時段的健康樣本。用來源／設備區別同時段的 Apple Watch 與 Connect 重複活動。穩定 ID 優先使用來源可提供的識別，否則以可重現的 Workout 身分推導並檢測碰撞。同一活動補到路線後，以新路徑另存修訂並保留舊檔。FIT 標示為轉製，保留原始取樣時間與已知單位，缺路線或缺心率時標出缺項，不補造數值。PoC 須確認實際 FIT metadata 能由現有讀取器接受。[既有三趟 XML／GPX 樣本](../research/rides/2026-11-three-device-comparison.md)

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
