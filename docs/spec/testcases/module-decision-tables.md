# M2 模組介面單元測試決策表

**實作基準。** 2026-11-24 騎士確認 A＝Config、B＝Query、C＝Data source、D＝Policy 的整體結構，授權依現有列進入開發。每列可用 `Row` 指定保留、修改、拆分或刪除，例如「A5b 修改 Result 2」。字母後綴是上一版情境拆出的子列；D 表保留 H（心率選源）、Q（品質門檻）、F（其他欄位）的編號。[已確認的 C#／Rust 介面](../config-and-data-source.md)

`Condition` 是呼叫前可安排的輸入或環境；`Result` 是呼叫後可斷言的回傳值、來源、診斷或副作用。以下採 Rust 方法名。來源 ID 使用設定範例中的 `bryton`、`garmin`、`apple_watch`。`—` 表示該欄無額外條件或斷言；`缺值` 是有意驗證的資料狀態。

目前可執行的測試函式、已驗證的 Result 與待補列，見[測試覆蓋與排程](module-test-coverage.md)。

## 執行方式

**A–D 全部採 Rust 單元測試。** 由 `cargo test` 執行獨立的 Rust 測試碼，直接呼叫正式 CLI 共用的模組。測試碼可放在各模組的 `#[cfg(test)] mod tests`，透過與 CLI 相同的介面呼叫。每列使用固定輸入，斷言回傳值及必要的互動次數。測試過程不啟動 merger 或 inspector 程序。本機 FIT、設定、選源策略與 Connect 模擬案例均已可由 cargo test 重跑。[重現命令](../../../README.md#驗證本機-fit-目錄來源)

- **A · Config** 使用小型 JSON、測試用的來源與目的地 factory。只有這張表展開設備、連線器與實例數量的組合。
- **B · Query** 直接傳日期、時區和 UTC 區間，核對查詢端點與門檻。
- **C · Data source** 直接呼叫來源實作。以測試目錄或記憶體檔案、可控制的 Connect 回應與下載計數安排讀取情境。需要驗證解碼時，讀取預先由 C# 及 Garmin FIT SDK 產生的小型 FIT，例如 C6a–c 分辨整欄缺失、單點無效與有效的 0 rpm。
- **D · Policy** 在記憶體建立已配對的記錄，把樣本、缺值、暫停與摘要直接交給 `select`。例如 Q1a／Q1b 只需改變連續缺值秒數，無須為每個門檻另造 FIT。

需要新 FIT 時，以 **C# + Garmin 官方 FIT SDK** 依案例明列的設備、UTC 時戳、逐點值與 session 摘要編碼。保留可重建的輸入定義、SDK 版本與對應的預期值。測試可重用已產生的小檔，執行 Rust 單元測試時無須啟動 .NET。涉及整支命令、產出 FIT 或發佈的代表流程列在[合併器端對端驗收](merger.md#端對端驗收的代表流程)。

## A．Config test

**測試對象**為 `Settings::load` 與來源、欄位、目的地的列舉方法。以 v1 範例為基準，每列只覆寫指定項目。來源及目的地的數量由各列設定決定。擴充列注入離線測試用的設備與連線器註冊項目；「已註冊」指對應方向的 factory 可建立該實例。共同斷言為私人活動檔讀取 0 次、平台請求 0 次、上傳 0 次；載入時可讀取設定檔。[設定檔 v1](../../config/README.md#如何增加設備或目的地)

- **操作**列出要呼叫的方法；**Condition 1** 指定設定的變動；**Condition 2** 指定查詢引數或執行環境。
- **Result 1** 指定應取得的物件、集合、序列或錯誤；**Result 2** 指定排序、欄位啟用狀態或錯誤位置。大括號表示集合，方括號表示有序序列。

| Row | 操作 | Condition 1 · 設定 | Condition 2 · 引數或環境 | Result 1 · 回傳 | Result 2 · 額外斷言 |
|---|---|---|---|---|---|
| A1 | `load → sources, primary_source` | 採用範例的三個來源 | 車表未掛載、token 空白 | 來源集合等於設定鍵集合 `{bryton, garmin, apple_watch}` | 主來源為 `bryton`；集合列舉順序任意 |
| A1b | `load → sources, primary_source, sources_for` | 加入第四個來源 `old_bike`，kind 為 `fit_directory`、device 為 `bryton_neo15` | 測試已註冊 Neo15，僅新增來源段 | 來源集合新增 `old_bike`；兩台 Bryton 依不同 ID 列舉 | 主來源仍為 `bryton`，心率仍為 `[bryton, garmin]`，總熱量仍為 `[garmin]` |
| A1c | `load → sources, sources_for` | 僅設定 Rider 460 主來源，啟用欄位僅為 Bryton 踏頻 | 省略 Garmin 與 Apple | `{bryton}` | `sources_for(record.cadence)` 為 `[bryton]` |
| A1d | `load → sources, sources_for` | 保留來源 ID 與設備，將 Garmin 取檔方式由 Connect 改為本機目錄 | 各載入一份有效設定 | 兩次都列出同一組來源 ID | 兩次的心率與總熱量順位相同 |
| A1e | `load → sources, primary_source, sources_for` | 新增 `archive2: {"kind":"test_archive","options":{}}` | 已註冊該測試來源，接受空選項 | 來源集合新增 `archive2` | 主來源為 `bryton`，各啟用欄位的來源順位與新增前逐欄相同 |
| A1f | `load → sources_for` | 在 A1b 的設定中，將踏頻順位改為 `[bryton, old_bike]` | 測試註冊的 Neo15 支援踏頻 | `sources_for(record.cadence)` 依序回傳 `bryton`、`old_bike` | 心率仍為 `[bryton, garmin]`，總熱量仍為 `[garmin]` |
| A2a | `selected_fields, sources_for` | 預設順位 | 查 `record.heart_rate` | `[bryton, garmin]` | 心率在啟用清單內 |
| A2b | `selected_fields, sources_for` | 預設順位 | 查 `session.total_calories` | `[garmin]` | 總熱量在啟用清單內 |
| A3 | `selected_fields, sources_for` | 移除 `record.temperature` | 查溫度 | `[]` | 啟用清單省略溫度 |
| A4a | `targets` | 省略整段 `destinations` | — | 空集合 | — |
| A4b | `targets` | 設定 Strava | token 空白 | `{strava}` | 授權流程啟動 0 次 |
| A4c | `load → targets` | 設定 Strava、`bike_log`（Bryton Activity）、`connect_log`（Garmin Connect） | 三種 target 均已註冊，沒有 token | 依各 target 的 ID 取得 `{strava, bike_log, connect_log}` | 回傳三個獨立實例，授權流程啟動 0 次 |
| A4d | `load → targets` | 只設定 `bike_log`，kind 為 `bryton_activity` | 已註冊該測試 target | `{bike_log}` | 無需 Strava 設定 |
| A4e | `load → targets` | 沿用原 v1 的 `destinations.strava`，省略 `kind` | 提供原有形狀的 Strava 欄位 | `{strava}` | 使用 Strava factory |
| A4f | `load → targets` | 自訂 ID `training` 和 `backup`，兩者均明示 `kind: strava` | 各用獨立設定段 | `{training, backup}` | 同一 factory 可建立兩個可分別尋址的目的地 |
| A5a | `load → sources_for` | 來源 ID 改為 `cirqa`，同步改全部引用 | 查心率 | `[bryton, cirqa]` | 第二順位的設備仍是 CIRQA |
| A5b | `load` | 同 A5a，僅保留心率第二順位的舊 ID | — | `ConfigError` | 指向 `/merge/streams/record.heart_rate/1` |
| A6a | `load` | 新增 `archive2: {"kind":"test_archive","options":{}}` | Schema 合法，未註冊該來源 | `ConfigError` | 指向 `/sources/archive2/kind` |
| A6b | `load` | 版本為不支援的 `2` | — | `ConfigError` | 指向 `/version` |
| A6c | `load` | 經度順位為 `[bryton]`，緯度為 `[bryton, garmin]` | — | `ConfigError` | 可定位經緯度選源規則 |
| A6d | `load` | 心率順位為 `[bryton, apple_watch]` | Apple 來源已宣告 | `ConfigError` | 可定位心率規則中的 Apple 順位 |
| A6e | `load` | 同 A1b 的 `old_bike` | 未註冊 Neo15 型號 | `ConfigError` | 指向 `/sources/old_bike/device` |
| A6f | `load` | `destinations.connect_log.kind` 為 `garmin_connect` | 已註冊 Garmin 讀取 factory，發佈方向未註冊 | `ConfigError` | 指向 `/destinations/connect_log/kind` |
| A6g | `load` | 已註冊的測試 target 收到 `options.typo` | 該 factory 的選項契約只接受空物件 | `ConfigError` | 可定位該目的地的未知選項 |

## B．Query test

**測試對象**為 `Settings::primary_query` 與 `candidate_query` 產生的 `ActivityQuery`。B 表只檢查查詢物件的語意，可用表列時間探測其邊界。實際執行列舉放在 C 表；原檔重疊率核實與候選唯一性屬合併器驗收。[日期配對契約](../../config/README.md#只給日期時的處理契約)

- **操作**區分主活動查詢與候選查詢；**Condition 1** 指定時區與門檻；**Condition 2** 是傳給建構方法的出發日或主活動 UTC 區間。
- **Result 1** 指定查詢使用的時間條件；**Result 2** 指定 UTC 端點、門檻或應可納入搜尋的範例。`[起, 迄)` 含起點、排除終點；表中日期皆為 2026 年。

| Row | 操作 | Condition 1 · 時區／門檻 | Condition 2 · 日期或主區間 | Result 1 · 查詢條件 | Result 2 · 邊界或門檻 |
|---|---|---|---|---|---|
| B1a | `primary_query` | 台北 | 出發日 10/14 | 起點落在 UTC `[10/13 16:00, 10/14 16:00)` | 恰於 10/13 16:00 UTC 出發納入 |
| B1b | `primary_query` | 台北 | 出發日 10/14 | 同 B1a | 恰於 10/14 16:00 UTC 出發歸次日 |
| B2 | `candidate_query` | 台北／0.90 | UTC `[10/14 15:50, 10/14 16:30)`，台北跨午夜 | 與完整主區間重疊的騎乘 | UTC 15:48 已開錄、16:30 結束的候選在查詢範圍內 |
| B3a | `candidate_query` | 台北／0.90 | 同 B2 | 重疊查詢 | 帶入雙向門檻 0.90 |
| B3b | `candidate_query` | 台北／0.95 | 同 B2 | 重疊查詢 | 帶入雙向門檻 0.95 |

## C．Data source test

**測試對象**為 `DataSource` 與它開啟的 `ActivityRecording`。共通列各用本機 FIT 與模擬 Connect 執行。模擬 Connect 提供可控制的分頁、錯誤與下載計數。

- **操作**是受測方法；**Condition 1** 指定來源實作及初始狀態；**Condition 2** 安排該來源提供的活動、樣本或故障。
- **Result 1** 是呼叫結果或可區分的錯誤類別；**Result 2** 指定資料身分、完整性或可觀察的 I/O 次數。錯誤類別名稱用來表達語意，Rust 變體的拼字留給實作。

| Row | 操作 | Condition 1 · 來源／狀態 | Condition 2 · 資料或事件 | Result 1 · 回傳或錯誤 | Result 2 · 身分、完整性或 I/O |
|---|---|---|---|---|---|
| C1a | `list_activities` | 共通；查詢完整成功 | 符合 0 趟 | 成功，空清單 | — |
| C1b | `list_activities` | 共通；查詢完整成功 | 只有活動 X 符合 | 清單含 X 一筆 | 可取得來源 ID 和 X 的活動 ID |
| C1c | `list_activities` | 共通；查詢完整成功 | 活動 X、Y 均符合 | 清單含 X、Y 各一筆 | 活動可分別識別 |
| C2 | `list_activities` | Connect；共兩頁 | 唯一符合的 X 在第二頁 | 回傳 X 一筆 | 第二頁有被讀取 |
| C3a | `list_activities` | 本機目錄 | 來源目錄不可讀 | 來源讀取錯誤 | 可區分 C1a |
| C3b | `list_activities` | Connect；token 已過期 | 換發亦因憑證失效遭拒 | 授權錯誤 | 可區分限流 |
| C3c | `list_activities` | Connect；憑證有效 | 查詢收到 429 | 限流錯誤 | 可區分授權失效 |
| C3d | `list_activities` | Connect；第一頁已取得 X | 第二頁持續讀取失敗 | 查詢失敗 | 第一頁的 X 不會作為完整成功清單回傳 |
| C4a | `metadata(X)` | 共通 | X 存在 | 回傳 X 的 metadata | 設備、運動種類、UTC 起迄與測試資料一致 |
| C4b | `metadata(X)` | 共通 | 本來源沒有 X | 活動 ID 不存在 | 可識別此次查詢的 X |
| C4c | `open_activity(X)` | 共通 | X 存在 | 回傳唯讀的 X 記錄 | 可由原始內容核對來源雜湊 |
| C4d | `open_activity(X)` | 共通 | 本來源沒有 X | 活動 ID 不存在 | 可識別此次開啟的 X |
| C5 | `open_activity → read_stream → read_summary` | Connect；冷快取 | 同趟先讀心率再讀 session 總熱量 | 兩次讀值均與同一份 FIT 相符 | 第二次讀值增加的原檔下載、整檔解碼均為 0 次 |
| C6a | `read_stream` | 共通；記錄已開啟 | 整份記錄沒有踏頻欄位 | 整欄缺失 | 可區分 C6b |
| C6b | `read_stream` | 共通；記錄已開啟 | 踏頻欄位存在，指定秒為無效碼 | 串流存在，該秒缺值 | 可區分 C6c |
| C6c | `read_stream` | 共通；記錄已開啟 | 指定秒的踏頻為有效零值 | 該秒為 0 rpm | 保留時間與有效狀態 |
| C7 | `list_activities → open_activity` | 本機來源 | FIT 改檔名後重新列舉，以當次取得的活動 ID 開啟 | 原檔內容雜湊、取樣與摘要與改名前一致 | 可追溯到同一個來源 ID |

## D．Policy test

**測試對象**為 `settings.merge_policy().select(&matched_ride)`。每列輸入已配對、可對齊的合成記錄；表中提到 11/01 的數值可直接寫進測試資料。逐點選值需可查回來源與時間；整趟換源或待判讀需可查回原因。[心率品質門檻](../../decisions/2026-11-23-heart-rate-quality.md) · [熱量來源規則](../../decisions/2026-11-23-ride-calorie-source.md)

- **規則**指出要驗證的選值行為；**Condition 1** 指定主時間軸、Bryton 值或欄位設定；**Condition 2** 指定 Garmin 樣本、品質或整趟摘要。
- **Result 1** 是整趟模式或欄位結果；**Result 2** 指定要核對的來源、缺值、數量或診斷。`QualityReview` 表示該趟需判讀。測試直接觀察策略回傳，發佈行為另由 CLI 驗收。

Q 列各自隔離一個門檻，其餘品質檢查均通過。Q1a/b 的主時間軸為 12,100 秒，使 599／600 秒缺值都低於全程 5%；Q2 的缺秒分散。Q3a–f 的平台長度、Garmin 同段有效覆蓋、Garmin 的 P90−P10、兩機連續相差至少 5 bpm 的秒數，基準分別為 **60 秒、95%（57/60）、5 bpm、20 秒**。每列覆寫的條件以外，其餘取基準或達標；Q3a 的 59 秒窗採 100% 覆蓋。Q5 的兩個覆蓋欄位依序為「全程、異常秒聯集」。

| Row | 規則 | Condition 1 · 主來源／設定 | Condition 2 · 次來源／對照 | Result 1 · 模式或值 | Result 2 · 來源、缺值或診斷 |
|---|---|---|---|---|---|
| H1 | 心率逐點選值 | 品質通過，該秒 130 bpm | 同秒 135 bpm | 該秒 130 | 來源 Bryton |
| H2 | 心率短缺 | 品質通過，只缺一秒 | 同秒 135 bpm | 該秒 135 | 該秒 Garmin，前後有效點仍取 Bryton |
| H3 | 心率短缺 | 品質通過，只缺一秒 | 同秒亦缺 | 該秒缺值 | 主時間軸保留該秒 |
| H4 | 疑似舊值 | 90 秒相同 bpm，其他時段正常 | 同段心率與 Bryton 接近，未達分歧門檻 | 保留 Bryton | 報告標記疑似平台 |
| H5 | 整趟切換 | 後段已確認停滯；停滯前某秒 Bryton 有值 | 品質通過，該秒也有值 | 整趟取 Garmin | 停滯前該秒也取 Garmin |
| H6 | 切換後遇缺值 | 已觸發切換；觀察秒 Bryton 有值 | 整趟合格，但觀察秒缺值 | 整趟取 Garmin | 該秒留空 |
| H7a | 備援缺檔 | 已判不佳 | Garmin 缺檔 | `QualityReview` | 原因可辨識為缺少備援 |
| H7b | 備援自身品質 | 已判不佳 | Garmin 有連續 600 秒相同 bpm，兩項覆蓋均達標 | `QualityReview` | 記錄 Garmin 自身品質原因 |
| Q1a | 連續缺值 | 連缺 599 秒 | Garmin 同秒皆有效 | 逐點補缺 | 補 599 點，其餘有效點取 Bryton |
| Q1b | 連續缺值 | 連缺 600 秒 | Garmin 合格 | 整趟取 Garmin | 觸發連續缺值 |
| Q1c | 全程缺值 | 60 秒活動，全程無有效心率 | Garmin 合格 | 整趟取 Garmin | 觸發全程缺值 |
| Q2a | 累積缺值 | 119/2,380 秒＝5% | Garmin 合格 | 逐點補缺 | 累計秒數未達 120 |
| Q2b | 累積缺值 | 120/2,401 秒，小於 5% | Garmin 合格 | 逐點補缺 | 比例未達 5% |
| Q2c | 累積缺值 | 120/2,400 秒＝5% | Garmin 合格 | 整趟取 Garmin | 累計秒數與比例均達門檻 |
| Q3a | 停滯長度 | 平台 59 秒 | 其餘佐證達標 | 保留 Bryton | 未達 60 秒 |
| Q3b | 停滯全部達標 | 平台 60 秒 | 覆蓋 57/60、P90−P10 為 5、連續分歧 20 秒 | 整趟取 Garmin | 觸發有對照佐證的停滯 |
| Q3c | 停滯同段覆蓋 | 平台 60 秒 | 同段有效 56/60 秒 | 保留 Bryton | 平台待佐證，覆蓋未達 95% |
| Q3d | 停滯同段變化 | 平台 60 秒 | P90−P10 為 4 bpm | 保留 Bryton | 平台待佐證，變化未達 5 bpm |
| Q3e | 停滯連續分歧 | 平台 60 秒 | 相差至少 5 bpm 只連續 19 秒 | 保留 Bryton | 平台待佐證，分歧時間未達 20 秒 |
| Q3f | 停滯分歧幅度 | 平台 60 秒 | 有連續 20 秒相差 4 bpm，全段沒有連續 20 秒相差至少 5 bpm | 保留 Bryton | 平台待佐證，未達分歧幅度與時長組合 |
| Q4 | 暫停邊界 | 30 秒同值、已知暫停、再 30 秒同值 | 兩段均有變化佐證 | 保留 Bryton | 兩段各自計時；對照無暫停的 Q3b |
| Q5a | Garmin 覆蓋 | Bryton 已判不佳 | 全程 95%、異常段 94% | `QualityReview` | 異常段覆蓋不足 |
| Q5b | Garmin 覆蓋 | Bryton 已判不佳 | 全程 94%、異常段 95% | `QualityReview` | 全程覆蓋不足 |
| Q5c | Garmin 覆蓋 | Bryton 已判不佳 | 全程 95%、異常段 95%，自身品質通過 | 整趟取 Garmin | 兩項門檻均含等號 |
| F1 | 總熱量 | 可比 session，Bryton 為 6,186 kcal | 總量 1,897、代謝 383 kcal | 選 Garmin 總量 1,897 | Garmin 兩個原值、衍生 1,514 與 Bryton 原值均可追溯 |
| F2 | 必要總熱量 | 已啟用總熱量，Bryton 有值 | 同趟 Garmin 總量缺失 | `QualityReview` | 記錄必要總量缺失 |
| F3a | 衍生活動熱量 | 已啟用總熱量與代謝量 | 總量 100，代謝缺值 | 總量 100，衍生值缺 | Garmin 代謝欄標缺 |
| F3b | 衍生活動熱量 | 同 F3a | 總量 100，代謝 101 | 衍生值缺 | 可查回原值 100、101 |
| F4 | 選取欄位 | 設定移除溫度，原檔有溫度 | — | 選值結果省略溫度及其摘要 | 同一份原始記錄仍可讀到溫度 |
| F5 | 座標成對 | GPS 只選 Bryton，該秒有緯度、缺經度 | — | 該秒座標對缺值 | 保留主時間點 |
