# FIT Fusion 需求與處理規格

## 要解決的問題

2026-11-23 使用者確認以下兩項需求。日常騎乘時會同時開啟 Bryton、Apple Watch 與 Garmin，Strava 上的活動主要由 Bryton 上傳。

1. **合併同一趟騎乘的多個數據串流。** 依各裝置實際記錄的起訖時間與重疊區間辨識同趟騎乘，以 Bryton 為主時間軸，整合心率、GPS、速度、踏頻等互補序列。允許各機提早或延後開錄、停止及跨午夜。驗收時應能從合併結果查回每個值採用的裝置、原始時間與缺值區段，並識別健康 App 中其他平台同步寫入的副本。[裝置與資訊流決策](decisions/2026-11-22-device-and-stream-selection.md)
2. **找出並修正騎乘卡路里異常。** 同趟活動在 Strava 和手錶上可能相差數倍。整趟熱量消耗以 Garmin CIRQA 為第一來源，分別保存活動熱量與含基礎代謝的總熱量。比對時核對騎乘時段與 kcal 單位。交付的 `merged.fit` 應能採用經確認的熱量，報告須可追溯修正前後的數值、來源及理由，並能用獨立讀取器核對輸出。發佈採哪種熱量、缺少可比 Garmin 摘要時的備援、異常門檻與既有活動的處置方式仍待決定。[熱量來源決策](decisions/2026-11-23-ride-calorie-source.md) · [熱量差異調查](research/2026-11-23-strava-ride-calories.md)

## 既有驗證紀錄

截至 2026-11-22，已完成 Python 與 C# 的 FIT 檢查，以及 11 月 21 日騎乘的一次性修復。以下的「已確認」指使用者已決定的產品需求，「已驗證」指已在指定樣本完成的操作。原始檔與匯出報告分別留在本機 `data/`、`reports/`。本規格與[騎乘處理紀錄](2026-11-21-ride.md)可獨立閱讀。

11 月 1 日的兩份 FIT 涵蓋 18,361 個共同記錄秒，車表心率僅缺 34 秒，當日感測器來源尚未確認。[11 月 1 日雙機比較](2026-11-01-ride.md)

11 月 8 日另有兩份已比對的 FIT。該次車表配對胸帶，可用於查閱一般配對情境下兩台心率相符程度及缺值時段。[11 月 8 日 Bryton 與 CIRQA 比較](2026-11-08-ride.md)

三日的 Apple 健康原始匯出均已辨識到 Watch 運動、關聯 GPX 與健康樣本。三機同秒心率、GPS、速度、高度與同取樣區間距離見[三趟騎乘的三裝置比較](2026-11-three-device-comparison.md)。

## 使用情境與交付需求

騎士在一次騎乘中同時使用 Bryton Rider 460、Garmin CIRQA 與 Apple Watch Series 9。希望每次運動後一次交付同趟檔案，由 CLI 辨識裝置，按預先定義的對照表為每種數值選取來源，合成同一趟活動。iPhone 16 Pro 是本次 Apple 健康資料的匯出介面。[裝置與選值決策](decisions/2026-11-22-device-and-stream-selection.md)

- **主活動與時間軸。** 採 Bryton 活動及其記錄時間，各設備樣本依時間對齊。整趟與分圈的平均值、最大值、總熱量等摘要另存。整趟熱量依第 2 項需求優先選用同趟 Garmin 資料。[冷水坑熱量差異實測](research/2026-11-23-strava-ride-calories.md)
- **選值與追溯。** Bryton 有提供的騎乘資訊流以 Bryton 為主。心率優先取車表的有效讀數（正常配對時來自胸帶），缺漏處由 Garmin CIRQA 補上。保留各來源的原值、採樣頻率及裝置與欄位出處。資訊流涵蓋率、時間缺口及同時段差異可供判讀。使用者可裁定無法分出首選的項目，本次距離即由使用者選定。[主來源決策](decisions/2026-11-22-bryton-primary-streams.md) · [距離決策](decisions/2026-11-22-primary-distance-stream.md)
- **原始資料保存。** 來源檔唯讀，產物另存。未知訊息與欄位保留原始編號、定義及資料。FIT 廠商自訂訊息與 FIT 2.0 developer fields 分別辨識，額外資訊流可使用 developer fields。
- **預期交付。** `merged.fit` 供 Strava 匯入，`merged.parquet` 供完整資料分析，另附選值與驗證報告。這些是正式融合的輸出需求。目前已驗證的產物是檢查報告及本次個案 FIT。[原始需求](../CONTEXT.md)

取得資料時優先使用 Bryton Active 與 Garmin Connect，希望兩端完成同步後，由 Mac 以本機憑證自動抓取各自的活動檔。Apple 健康全量匯出列第三順位，可另行補入。各資訊流的主來源仍依選值規格決定。[個人雲端取檔可行性](decisions/2026-11-22-cloud-activity-import.md)

## 盤點表一　如何從輸入辨識記錄裝置

FIT 的 `file_id.manufacturer` 使用 Garmin 配發的製造商編號，`file_id.product` 是該製造商編定的產品號。查型號要取兩者的組合。辨識出車表後，外接感測器仍需另外的來源證據。本次胸帶故障及改接廣播都是騎士提供的事實。[裝置辨識決策](decisions/2026-11-22-device-and-stream-selection.md)

| 記錄裝置 | 本次輸入 | 檔內可辨識的依據 | 證據能支持的結論 |
|---|---|---|---|
| Bryton Rider 460 | `2026-1121-bryant-460.fit` | `file_id` 的製造商／產品為 `(267, 2205)` | 可辨識 Bryton 及產品號。本次 Rider 460 的型號名稱由使用者確認。 |
| Garmin CIRQA | `2026-1121-garmin-cirqa.fit` | `file_id` 的製造商／產品為 `(1, 5019)` | 此組合有 Garmin 官方 CIRQA 型號對照。 |
| Apple Watch Series 9 | iPhone「健康」匯出的運動 XML、樣本與 GPX | Workout 的裝置資料、硬體代碼 `Watch7,1` 與來源 `AndrewWatch`。路線依該次 WorkoutRoute 的 FileReference 取得。 | 可在含多個寫入來源的匯出檔中選出這支手錶記錄的運動；同帳號另有 Garmin Connect 寫入的活動。 |

## 設備優先順序與合併後的數據清單

下表列出合併後需要的騎乘數據。逐點欄位以 Bryton 主活動的時間軸對齊，整趟摘要則依同一騎乘及相同熱量定義選取。原始 FIT 訊息、未知欄位和各機原始取樣另按資料保存要求處理。[裝置與選值決策](decisions/2026-11-22-device-and-stream-selection.md)

| 合併後的數據 | 第一順位 | 第二順位 | 選值單位與限制 |
|---|---|---|---|
| 心率（bpm） | Bryton `record.heart_rate`，通常為配對的胸帶 | **Garmin CIRQA** `record.heart_rate` | 每個對齊時間點取一筆，Bryton 恢復有效心率即切回。 |
| GPS 位置（經緯度） | Bryton | 待定 | 每筆經緯度須成對且來自同一裝置；現有提案以 Apple 為缺點時的候選。 |
| 騎乘速度 | Bryton | 待定 | 三機都有逐點速度。 |
| 高度 | Bryton（現行順位） | 待定 | 三機都有高度；冷水坑的絕對海拔選源仍在評估。 |
| 累積騎乘距離 | Bryton | 待定 | Garmin 也有累積里程。跨機替補前須能對齊里程起點；Apple 提供區間增量。 |
| 踏頻（rpm） | Bryton | 本批無其他來源 | 0 rpm 是有效樣本。 |
| 溫度（°C） | Bryton | 本批無其他來源 | 使用沿途的逐點溫度。 |
| 整趟總熱量（kcal，含基礎代謝） | Garmin CIRQA | 待定 | 使用 `session.total_calories`。Apple 可對照；Bryton 原值留供追溯。 |
| 整趟活動熱量（kcal） | Garmin CIRQA | 待定 | 本批取總熱量減 `session.metabolic_calories`，已與 Connect 核對。Apple 運動摘要亦有活動熱量。 |

**取值規則。** 兩台都有有效值時取第一順位。首選整條資訊流缺失，就在主活動時段內逐點使用第二順位的有效樣本；只有局部缺漏，就只補對應時間點。欄位不存在與 FIT 無效值都算缺漏。兩順位都缺、只有單一來源或第二順位尚待決定時，沒有可用樣本的位置保留缺值。有值處記錄來源裝置與原始取樣時間，報告可分別計數首選、替補與缺漏。整趟熱量的選值單位是一趟，可各自追溯總熱量與活動熱量。[定位與高度提案](decisions/2026-11-22-position-and-elevation-sources.md) · [距離決策](decisions/2026-11-22-primary-distance-stream.md) · [熱量決策](decisions/2026-11-23-ride-calorie-source.md)

**心率驗收。** 以 11 月 8 日原檔的 17,190 個 Bryton 記錄秒為例，應保留 3,906 筆 Bryton 有效心率，另取 13,284 筆 Garmin 同秒原始值。10 月 28 日 Bryton 全趟無有效心率，9,632 個記錄秒均有 Garmin 對應值。11 月 21 日若套用一般規則，應保留 Bryton 14,084 筆、用 Garmin 補 65 筆；歷史修復檔依騎士當天的指定，採用 CIRQA 整段廣播來源。[11 月 8 日實檔](2026-11-08-ride.md) · [11 月 21 日單次處理](decisions/2026-11-22-nov21-heart-rate-repair.md)

### 清單外的樣本如何保存

| 已盤點的數據 | 本批可用來源 | 用途 |
|---|---|---|
| 行進方向、水平與垂直定位精度 | Apple Watch 路線點 | 保存方向與精度供分析、核對路線。 |
| 活動熱量與基礎熱量的區間增量 | Apple Watch 健康樣本 | 保留各區間起訖與原值，供核對整趟熱量。 |
| 體力消耗強度、運動分鐘、日照時間 | Apple Watch 健康樣本 | 保存原始樣本，目前只確認為同手錶、同時段。 |
| 騎乘功率 | 本批三台均無可辨識的有效樣本 | 此批無輸出值。 |
| Garmin 私有欄位 135、136、143 及其他未知訊息 | 各來源 FIT | 依原始編號、定義與數值保存並納入盤點。 |

這些項目保留在來源檔與完整分析資料。11 月 21 日修復檔曾額外寫入行進方向、區間熱量與體力消耗強度，該次寫入範圍見[處理紀錄](2026-11-21-ride.md)。

### 欄位與樣本的解讀

- FIT 騎乘時間序列取自 `record`。心率使用 `heart_rate`，座標取成對的 `position_lat`、`position_long`，距離取累積的 `distance`。本次 Bryton 使用 `speed`、`altitude`、`cadence`、`temperature`。Garmin 的速度與高度有有效值的欄位為 `enhanced_speed`、`enhanced_altitude`。有效的零值也算樣本，例如本次 Bryton 有 7,542 筆 0 rpm。
- Apple 的點資料依 GPX 路線點時間讀取，心率取健康紀錄的 `HeartRate`。距離與熱量是區間增量，保留起訖；比較距離時，以兩台 FIT 在共同時段的累積距離增量比較。Apple 的逐點和區間欄位、取樣頻率與實際涵蓋見[本次樣本清單](2026-11-21-ride.md)。
- 這批檔案可用 UTC 整秒直接對齊。跨裝置比較覆蓋率時，分母是 Bryton 的 14,149 個記錄秒，計算同秒有原始有效值的秒數。單獨檢查一份檔案時，則以該檔該訊息類型的筆數為分母。例如 Garmin 全檔有效座標有 13,515 筆，落在 Bryton 主時間軸的有 13,289 筆。
- 使用者選擇 Bryton 距離以對齊主時間軸。共同區間的距離增量，Bryton 為 38,111.41 公尺，Garmin 為 38,247.87 公尺，相差 136.46 公尺。GPS 的同秒有效數為 Bryton 13,572、Apple 13,562、Garmin 13,289。本次沒有標準里程或參考軌跡可裁定絕對誤差。

## 里程碑與未解決事項

2026-11-22 已確認決策時程：制定融合 CLI 開發規格時，一併決定水平定位與絕對海拔的來源順序、跳點判定與回退條件。三趟盤點及冷水坑的五點高程比對供屆時評估。[冷水坑實測與待決提案](decisions/2026-11-22-position-and-elevation-sources.md)

1. **Milestone 1，已驗證。** Python 提供 `raw` 與 `inspect`。可檢查 FIT 訊息、欄位、時間範圍、取樣間隔、頻率、涵蓋率、未知欄位與 developer fields。原始匯出含 frame 位元組供還原。Python 的 fitdecode 是選用後端，本次雙語比對採離線解碼器。
2. **Milestone 1-1，已驗證。** .NET 10 的 C# CLI 獨立執行相同兩個命令。兩邊共用 FIT profile 名稱與單位對照。兩份真實 FIT 的逐框與統計比對已完成，驗證數字見[處理紀錄](2026-11-21-ride.md)。[里程碑決策](decisions/2026-11-21-csharp-inspection-control.md)
3. **正式 CLI 技術選型已確認。** 2026-11-23 使用者選定 Rust、RustyFIT（Cargo 套件 `rustyfit`）與 macOS。合成檔案的讀寫已通過逐欄比對。實騎原檔的未知欄位保存與 Strava 匯入仍待驗證。[技術選型決策](decisions/2026-11-21-production-fit-sdk.md)
4. **其餘待定的融合規則。** 清單內的欄位先用第一順位，整段或局部缺漏用已指定的第二順位。其餘備援裝置、跨裝置時間偏移與容許時間差、插值政策、機讀對照格式及未知裝置辨識仍待定義。
5. **未解決：私有欄位與來源歸屬。** Garmin `record` 的 135、136、143 保留原值。136 在本檔每筆等於標準心率欄位 3；先前討論的腕式心率（136）及 Body Battery（143）屬非官方線索，143 在樣本中由 57 降至 30。三者均缺已確認的官方語義，135 的用途未定。Garmin 訊息 233 共 14,604 筆且無獨立時間戳，22、288 只有零星記錄。Bryton 的 65282、65286 亦保留。Apple 的體力消耗強度、運動分鐘與日照只確認同手錶同時段，缺逐筆運動歸屬。
6. **未解決：正式輸出與平台相容性。** 通用融合 CLI、Parquet 交付與正式 SDK 的未知欄位寫回均待驗證。已討論 Strava 可用具 `activity:write` 權限的 token 上傳 FIT，非同步查詢會回傳活動 ID 或錯誤，既有騎乘可能被判重複。本次僅有本機讀回紀錄，修復檔實際匯入及 Apple 自訂欄位在 Strava 的呈現仍未驗證。自動上傳也尚未列為正式需求。
