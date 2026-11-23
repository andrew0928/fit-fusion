# FIT Fusion 需求與處理規格

本文維護騎乘資料及逐欄選源規則。兩類 CLI 的輸入輸出與驗收案例各有入口。[檢查器規格](spec/inspector.md) · [檢查器案例](spec/testcases/inspector.md) · [合併器規格](spec/merger.md) · [合併器案例](spec/testcases/merger.md)

## 要解決的問題

2026-11-23 使用者確認以下兩項需求。日常騎乘時會同時開啟 Bryton、Apple Watch 與 Garmin，Strava 上的活動主要由 Bryton 上傳。

1. **合併同一趟騎乘的多個數據串流。** 依各裝置實際記錄的起訖時間與重疊區間辨識同趟騎乘，以 Bryton 為主時間軸，整合心率、GPS、速度、踏頻等互補序列。允許各機提早或延後開錄、停止及跨午夜。驗收時應能從合併結果查回每個值採用的裝置、原始時間與缺值區段，並識別健康 App 中其他平台同步寫入的副本。[裝置與資訊流決策](decisions/2026-11-22-device-and-stream-selection.md)
2. **找出並修正騎乘卡路里異常。** 同趟活動在 Strava 和手錶上可能相差數倍。配對時核對騎乘時段與 kcal 單位，發佈直接取 CIRQA 的整趟 `session.total_calories`。合併 FIT 另保留 Garmin 總量與代謝量兩個原值，報告列出 Bryton 修正前的數值、採用理由及衍生的活動熱量。以獨立讀取器核對輸出。缺少可比 Garmin 摘要時的備援及異常門檻仍待驗證。[熱量來源決策](decisions/2026-11-23-ride-calorie-source.md) · [熱量差異調查](research/2026-11-23-strava-ride-calories.md)

## 既有驗證紀錄

截至 2026-11-22，已完成 Python 與 C# 的 FIT 檢查，以及 11 月 21 日騎乘的一次性修復。以下的「已確認」指使用者已決定的產品需求，「已驗證」指已在指定樣本完成的操作。原始檔保存在本機 `data/`。當時的匯出報告使用 `reports/`，目前比對工具寫入 `runtime/reports/`。本規格與[騎乘處理紀錄](research/rides/2026-11-21-ride.md)可獨立閱讀。

11 月 1 日的兩份 FIT 涵蓋 18,361 個共同記錄秒，車表心率僅缺 34 秒，當日感測器來源尚未確認。[11 月 1 日雙機比較](research/rides/2026-11-01-ride.md)

11 月 8 日另有兩份已比對的 FIT。該次車表配對胸帶，可用於查閱一般配對情境下兩台心率相符程度及缺值時段。[11 月 8 日 Bryton 與 CIRQA 比較](research/rides/2026-11-08-ride.md)

三日的 Apple 健康原始匯出均已辨識到 Watch 運動、關聯 GPX 與健康樣本。三機同秒心率、GPS、速度、高度與同取樣區間距離見[三趟騎乘的三裝置比較](research/rides/2026-11-three-device-comparison.md)。

## 使用情境與交付需求

騎士在一次騎乘中同時使用 Bryton Rider 460、Garmin CIRQA 與 Apple Watch Series 9。希望每次運動後一次交付同趟檔案，由 CLI 辨識裝置，按預先定義的對照表為每種數值選取來源，合成同一趟活動。iPhone 16 Pro 是本次 Apple 健康資料的匯出介面。[裝置與選值決策](decisions/2026-11-22-device-and-stream-selection.md)

- **主活動與時間軸。** 採 Bryton 活動及其記錄時間，各設備樣本依時間對齊。整趟與分圈的平均值、最大值、總熱量等摘要另存。整趟熱量依第 2 項需求優先選用同趟 Garmin 資料。[冷水坑熱量差異實測](research/2026-11-23-strava-ride-calories.md)
- **選值與追溯。** Bryton 有提供的騎乘資訊流以 Bryton 為主。心率先評估整趟品質。品質通過時優先取車表的有效讀數（正常配對時來自胸帶），小缺口用 Garmin CIRQA 補齊；品質不佳時選用同趟通過檢查的 Garmin 本機心率。保留各來源的原值、採樣頻率及裝置與欄位出處。資訊流涵蓋率、時間缺口及同時段差異可供判讀。使用者可裁定無法分出首選的項目，本次距離即由使用者選定。[主來源決策](decisions/2026-11-22-bryton-primary-streams.md) · [距離決策](decisions/2026-11-22-primary-distance-stream.md)
- **原始資料保存。** 來源檔唯讀，產物另存。未知訊息與欄位保留原始編號、定義及資料。FIT 廠商自訂訊息與 FIT 2.0 developer fields 分別辨識，額外資訊流可使用 developer fields。
- **預期交付。** `merged.fit` 供 Strava 匯入，`merged.parquet` 供完整資料分析，另附選值與驗證報告。這些是正式融合的輸出需求。目前已驗證的產物是檢查報告及本次個案 FIT。[原始需求](CONTEXT.md)

日常操作只給出發日期，由設定指定的主來源列舉當日騎乘，再依各串流引用的來源配對。M2 範例從 Bryton 本機 FIT 目錄列舉，CIRQA 可從 Garmin Connect 或本機 FIT 目錄取得，Apple 健康匯出供分析。後續可研究直接從 Bryton Active 與 Garmin Connect 兩端同步。[設定檔 v1 與逐趟配對](config/README.md) · [個人雲端取檔可行性](decisions/2026-11-22-cloud-activity-import.md)

## 盤點表一　如何從輸入辨識記錄裝置

FIT 的 `file_id.manufacturer` 使用 Garmin 配發的製造商編號，`file_id.product` 是該製造商編定的產品號。查型號要取兩者的組合。辨識出車表後，外接感測器仍需另外的來源證據。騎士確認 11 月 21 日未帶胸帶，當天車表使用 Garmin 心率廣播。[裝置辨識決策](decisions/2026-11-22-device-and-stream-selection.md)

| 記錄裝置 | 本次輸入 | 檔內可辨識的依據 | 證據能支持的結論 |
|---|---|---|---|
| Bryton Rider 460 | `2026-1121-bryant-460.fit` | `file_id` 的製造商／產品為 `(267, 2205)` | 可辨識 Bryton 及產品號。本次 Rider 460 的型號名稱由使用者確認。 |
| Garmin CIRQA | `2026-1121-garmin-cirqa.fit` | `file_id` 的製造商／產品為 `(1, 5019)` | 此組合有 Garmin 官方 CIRQA 型號對照。 |
| Apple Watch Series 9 | iPhone「健康」匯出的運動 XML、樣本與 GPX | Workout 的裝置資料、硬體代碼 `Watch7,1` 與來源 `AndrewWatch`。路線依該次 WorkoutRoute 的 FileReference 取得。 | 可在含多個寫入來源的匯出檔中選出這支手錶記錄的運動；同帳號另有 Garmin Connect 寫入的活動。 |

## 設備優先順序與合併後的數據清單

下表列出合併後需要的騎乘數據。逐點欄位以 Bryton 主活動的時間軸對齊，整趟摘要則依同一騎乘及相同熱量定義選取。原始 FIT 訊息、未知欄位和各機原始取樣另按資料保存要求處理。[裝置與選值決策](decisions/2026-11-22-device-and-stream-selection.md)

| 合併後的數據 | 第一順位 | 第二順位 | 選值單位與限制 |
|---|---|---|---|
| 心率（bpm） | Bryton `record.heart_rate`，通常為配對的胸帶 | **Garmin CIRQA** `record.heart_rate` | 先判整趟品質。一般缺口逐點補；長缺、頻繁缺值或持續寫舊值時，整趟心率改用可用的 Garmin。 |
| GPS 位置（經緯度） | Bryton | 待定 | 每筆經緯度須成對且來自同一裝置；現有提案以 Apple 為缺點時的候選。 |
| 騎乘速度 | Bryton | 待定 | 三機都有逐點速度。 |
| 高度 | Bryton（現行順位） | 待定 | 三機都有高度；冷水坑的絕對海拔選源仍在評估。 |
| 累積騎乘距離 | Bryton | 待定 | Garmin 也有累積里程。跨機替補前須能對齊里程起點；Apple 提供區間增量。 |
| 踏頻（rpm） | Bryton | 本批無其他來源 | 0 rpm 是有效樣本。 |
| 溫度（°C） | Bryton | 本批無其他來源 | 使用沿途的逐點溫度。 |
| 整趟總熱量（kcal，含基礎代謝） | Garmin CIRQA | 待定 | 使用 `session.total_calories`。Apple 可對照；Bryton 原值留供追溯。 |
| 整趟活動熱量（kcal） | Garmin CIRQA | 待定 | 本批取總熱量減 `session.metabolic_calories`，已與 Connect 核對。Apple 運動摘要亦有活動熱量。 |

**取值規則。** 心率先套用下述品質判定；其他欄位與品質通過的心率，在兩台都有有效值時取第一順位。首選整條資訊流缺失，就在主活動時段內逐點使用第二順位的有效樣本；只有局部缺漏，就只補對應時間點。欄位不存在與 FIT 無效值都算缺漏。兩順位都缺、只有單一來源或第二順位尚待決定時，沒有可用樣本的位置保留缺值。有值處記錄來源裝置與原始取樣時間，報告可分別計數首選、替補與缺漏。整趟熱量的選值單位是一趟，可各自追溯總熱量與活動熱量。[定位與高度提案](decisions/2026-11-22-position-and-elevation-sources.md) · [距離決策](decisions/2026-11-22-primary-distance-stream.md) · [熱量決策](decisions/2026-11-23-ride-calorie-source.md)

**心率品質與驗收。** 全程無心率、連續缺值達 10 分鐘，或缺值累計達 2 分鐘且占記錄時段至少 5%，均觸發整趟換源。對有值的資料，若 Bryton 連續至少 60 秒寫相同心率，而 Garmin 同段有足夠的變化且與該值持續分歧，也觸發整趟換源。正式選值只使用 Bryton 和 Garmin。Apple Watch 現行需整批匯入，心率留供分析。Garmin 於主活動及異常秒的有效覆蓋都須達 95%，並通過自身品質檢查，否則標示待判讀。[完整判準](decisions/2026-11-23-heart-rate-quality.md)

六趟實檔回放中，10/09、10/14、11/01 保留 Bryton，只補短缺口；10/28、11/08、11/21 改用整趟 Garmin。11/08 預期輸出 17,190 個 Garmin 樣本；11/21 預期輸出 14,149 個 Garmin 樣本，即使當天 Bryton 只有 65 秒明確缺值，另有兩段持續寫舊值已足以觸發。[逐趟數據與平台時間點](research/2026-11-23-heart-rate-quality.md)

### Strava FIT 逐欄選源表

本表逐項對照 Strava《Uploading to Strava》截至 2026-11-23 公開的 FIT 匯入表。Strava 將它稱為所用欄位的概覽。表內列名表示官方提到會讀取該欄，平台呈現與 API 可讀回的欄位另行驗證。每一筆騎乘軌跡點須帶時間。M2 只選同名、同語義的來源欄位。Apple Watch 全部留作分析，表內特別標示可用來對照的數值。[Strava 官方 FIT 欄位表](https://developers.strava.com/docs/uploads/) · [Garmin FIT Profile](../src/python/fit_fusion/profile.tsv)

「沿用」指主檔已有有效欄位時原樣保留。「—」表示本版沒有指定替補。「略過」表示不為這趟自行車活動建立該欄位。六趟 Bryton 主檔均無 lap 訊息，Garmin 的自動分圈另存原始資料。新樣本有主檔分圈時才沿用該圈。[設備與資訊流選擇](decisions/2026-11-22-device-and-stream-selection.md)

| Strava 列出的 FIT 欄位 | M2 第一來源 | 第二來源 | 選值條件或其他裝置用途 |
|---|---|---|---|
| `file_id.manufacturer` | Bryton | — | 沿用主活動的製造商。 |
| `file_id.product` | Bryton | — | 與製造商成對。 |
| `file_id.time_created` | Bryton | — | 主檔建立時間。 |
| `activity.timestamp` | Bryton | — | 沿用活動時間。 |
| `activity.local_timestamp` | Bryton | — | 沿用裝置本地時間。跨機配對用 UTC。 |
| `record.timestamp` | Bryton | — | 主時間軸，其他機器的額外秒點只供分析。 |
| `record.position_lat` | Bryton | — | 同秒經緯度成對。Apple Watch、Garmin 可核對跳點。 |
| `record.position_long` | Bryton | — | 與緯度取自同一原始樣本。 |
| `record.distance` | Bryton | — | 累積里程。Garmin 起點待對齊，Apple Watch 提供區間增量。 |
| `record.speed` | Bryton | — | 本批車表的速度欄。Apple Watch 速度留供對照。 |
| `record.enhanced_speed` | Bryton 如有 | — | 本批車表無此欄。Garmin 同名欄位留在來源，未指定為替補。 |
| `record.altitude` | Bryton | — | 現行高度順位。Apple Watch 與 Garmin 高度留供校正分析。 |
| `record.enhanced_altitude` | Bryton 如有 | — | 本批車表無此欄。Garmin 同名高度留供比較。 |
| `record.heart_rate` | Bryton | Garmin CIRQA | 小缺口用 Garmin 同秒值。整趟判定品質不佳則全部取合格的 Garmin。Apple Watch 只供分析。 |
| `record.cadence` | Bryton | — | 0 rpm 有效。 |
| `record.temperature` | Bryton | — | 逐點溫度。 |
| `record.power` | — | — | 本批三台均無有效功率。 |
| `session.sport` | Bryton | — | 活動類型，主流程限騎乘。 |
| `session.sub_sport` | Bryton | — | 主檔運動子類型。 |
| `session.start_time` | Bryton | — | 騎乘開始。 |
| `session.timestamp` | Bryton | — | session 時間戳。 |
| `session.total_elapsed_time` | Bryton | — | 含暫停的經過時間。 |
| `session.total_timer_time` | Bryton | — | 裝置計時器累計時間。 |
| `session.total_distance` | Bryton | — | 主活動總里程。 |
| `session.total_ascent` | Bryton | — | 沿用主活動爬升摘要。 |
| `session.total_descent` | Bryton | — | 沿用主活動下降摘要。 |
| **`session.total_calories`** | **Garmin CIRQA** | — | **選 Garmin 原檔同名欄位 11。** 冷水坑寫 1,897 kcal。Bryton 原值 6,186，Apple Watch 留供分析。 |
| `session.avg_speed` | Bryton | — | 沿用整趟均速。 |
| `session.max_speed` | Bryton | — | 沿用整趟最大速度。 |
| `session.avg_heart_rate` | 依選定的逐點心率 | — | 選 Bryton 或 Garmin 的同名摘要，須涵蓋相同有效樣本。混合來源或時段不符時標缺。 |
| `session.avg_cadence` | Bryton | — | 沿用有效踏頻摘要。 |
| `session.avg_temperature` | Bryton | — | 沿用平均溫度。 |
| `session.num_laps` | Bryton | — | 核對主檔的圈數及實際 lap 訊息，差異列入報告。 |
| `session.total_work` | Bryton 如有 | — | 騎乘作功，單位 J。本批無有效值，與 kcal 分開。 |
| `session.total_cycles` | Bryton | — | 沿用有效的騎乘循環數。 |
| `session.pool_length` | 略過 | — | 游泳。 |
| `session.total_strides` | 略過 | — | 步數，騎乘略過。 |
| `session.avg_step_length` | 略過 | — | 步幅，騎乘略過。 |
| `lap.start_time` | Bryton 如有 | — | 原圈開始。 |
| `lap.timestamp` | Bryton 如有 | — | 原圈結束時間戳。 |
| `lap.total_elapsed_time` | Bryton 如有 | — | 原圈經過時間。 |
| `lap.total_timer_time` | Bryton 如有 | — | 原圈計時。 |
| `lap.total_distance` | Bryton 如有 | — | 原圈距離。 |
| `lap.total_ascent` | Bryton 如有 | — | 原圈爬升。 |
| `event.timestamp` | Bryton | — | 暫停與恢復依主檔事件。 |
| `event.type` | Bryton 原事件 | — | Strava 表用語為 `type`。保留原始 FIT 的 `event`、`event_type` 編碼。 |
| `event.timer_trigger` | Bryton 原事件 | — | 有對應計時器事件時保留其原生 data 子欄位。 |
| `hr.timestamp` | 略過 | — | 本批來源使用 `record.heart_rate`。 |
| `hr.event_timestamp` | 略過 | — | 本批不用逐搏事件。 |
| `hr.filtered_bpm` | 略過 | — | 不另建一套和選定心率相競的 HR 訊息。 |
| `length.event_type` | 略過 | — | 泳池趟次。 |
| `length.length_type` | 略過 | — | 泳池趟次。 |
| `length.swim_stroke` | 略過 | — | 泳姿。 |
| `length.start_time` | 略過 | — | 泳池趟次。 |
| `length.total_elapsed_time` | 略過 | — | 泳池趟次。 |
| `length.total_timer_time` | 略過 | — | 泳池趟次。 |
| `length.total_strokes` | 略過 | — | 泳池划水次數。 |
| `split.split_type` | 略過 | — | 騎乘首版不建立 split。 |
| `split.start_time` | 略過 | — | 同上。 |
| `split.total_elapsed_time` | 略過 | — | 同上。 |
| `set.set_type` | 略過 | — | 重訓組次。 |
| `set.start_time` | 略過 | — | 重訓組次。 |
| `set.duration` | 略過 | — | 重訓組次。 |
| `set.repetitions` | 略過 | — | 重訓組次。 |
| `set.weight` | 略過 | — | 重訓組次。 |
| `set.category` | 略過 | — | 重訓組次。 |
| `set.category_subtype` | 略過 | — | 重訓組次。 |

**熱量的額外保存。** Strava 表列的發佈欄位只有 `session.total_calories` 涉及本案的整趟熱量。合併 FIT 另保留 Garmin 的兩個同名原值，並在來源層識別 CIRQA。標準 `session.metabolic_calories` 及來源副本可由本機讀取器核對。冷水坑的活動熱量 1,514 kcal 是報告中的衍生值。[原欄位及保存方式](decisions/2026-11-23-ride-calorie-source.md)

**未來的 rename。** 後續版本要能明確指定「來源裝置＋訊息＋欄位 → 目的平台＋訊息＋欄位」，並驗證目標單位、資料型別、作用範圍及同一目標的衝突。使用者舉的 `Garmin session.metabolic_calories → Strava session.total_calories` 在冷水坑會發佈 383 kcal，代表代謝量。這條是映射能力的驗收例，M2 的實際發佈選同名總熱量 1,897 kcal。

### 清單外的樣本如何保存

| 已盤點的數據 | 本批可用來源 | 用途 |
|---|---|---|
| 行進方向、水平與垂直定位精度 | Apple Watch 路線點 | 保存方向與精度供分析、核對路線。 |
| 活動熱量與基礎熱量的區間增量 | Apple Watch 健康樣本 | 保留各區間起訖與原值，供核對整趟熱量。 |
| 體力消耗強度、運動分鐘、日照時間 | Apple Watch 健康樣本 | 保存原始樣本，目前只確認為同手錶、同時段。 |
| 騎乘功率 | 本批三台均無可辨識的有效樣本 | 此批無輸出值。 |
| Garmin 私有欄位 135、136、143 及其他未知訊息 | 各來源 FIT | 依原始編號、定義與數值保存並納入盤點。 |

2026-11-23 使用者確認正式合併範圍為騎乘數據與整趟熱量，Apple 明細留作分析。上表各項保留在來源檔與完整分析資料。11 月 21 日修復檔曾額外寫入行進方向、區間熱量與體力消耗強度，該次寫入範圍見[處理紀錄](research/rides/2026-11-21-ride.md)。

### 欄位與樣本的解讀

- FIT 騎乘時間序列取自 `record`。心率使用 `heart_rate`，座標取成對的 `position_lat`、`position_long`，距離取累積的 `distance`。本次 Bryton 使用 `speed`、`altitude`、`cadence`、`temperature`。Garmin 的速度與高度有有效值的欄位為 `enhanced_speed`、`enhanced_altitude`。有效的零值也算樣本，例如本次 Bryton 有 7,542 筆 0 rpm。
- Apple 的點資料依 GPX 路線點時間讀取，心率取健康紀錄的 `HeartRate`。距離與熱量是區間增量，保留起訖；比較距離時，以兩台 FIT 在共同時段的累積距離增量比較。Apple 的逐點和區間欄位、取樣頻率與實際涵蓋見[本次樣本清單](research/rides/2026-11-21-ride.md)。
- 這批檔案可用 UTC 整秒直接對齊。跨裝置比較覆蓋率時，分母是 Bryton 的 14,149 個記錄秒，計算同秒有原始有效值的秒數。單獨檢查一份檔案時，則以該檔該訊息類型的筆數為分母。例如 Garmin 全檔有效座標有 13,515 筆，落在 Bryton 主時間軸的有 13,289 筆。
- 使用者選擇 Bryton 距離以對齊主時間軸。共同區間的距離增量，Bryton 為 38,111.41 公尺，Garmin 為 38,247.87 公尺，相差 136.46 公尺。GPS 的同秒有效數為 Bryton 13,572、Apple 13,562、Garmin 13,289。本次沒有標準里程或參考軌跡可裁定絕對誤差。

## 里程碑與未解決事項

2026-11-22 已確認決策時程：制定融合 CLI 開發規格時，一併決定水平定位與絕對海拔的來源順序、跳點判定與回退條件。三趟盤點及冷水坑的五點高程比對供屆時評估。[冷水坑實測與待決提案](decisions/2026-11-22-position-and-elevation-sources.md)

1. **Milestone 1，已驗證。** Python 提供 `raw` 與 `inspect`。可檢查 FIT 訊息、欄位、時間範圍、取樣間隔、頻率、涵蓋率、未知欄位與 developer fields。原始匯出含 frame 位元組供還原。Python 的 fitdecode 是選用後端，本次雙語比對採離線解碼器。
2. **Milestone 1-1，已驗證。** .NET 10 的 C# CLI 獨立執行相同兩個命令。兩邊共用 FIT profile 名稱與單位對照。兩份真實 FIT 的逐框與統計比對已完成，驗證數字見[處理紀錄](research/rides/2026-11-21-ride.md)。[里程碑決策](decisions/2026-11-21-csharp-inspection-control.md)
3. **正式 CLI 技術選型已確認。** 2026-11-23 使用者選定 Rust、RustyFIT（Cargo 套件 `rustyfit`）與 macOS。合成檔案的讀寫已通過逐欄比對。實騎原檔的未知欄位保存與 Strava 匯入仍待驗證。[技術選型決策](decisions/2026-11-21-production-fit-sdk.md)
4. **融合規則與機讀格式。** 心率先依整趟品質決定選源模式，其餘依逐欄順位補缺。已確認的設定檔 v1 明列來源與順位，跨裝置時鐘偏移、插值、未指定的備援裝置及未知裝置辨識仍待驗證。[設定檔 v1](config/README.md)
5. **未解決：私有欄位與來源歸屬。** Garmin `record` 的 135、136、143 保留原值。136 在本檔每筆等於標準心率欄位 3；先前討論的腕式心率（136）及 Body Battery（143）屬非官方線索，143 在樣本中由 57 降至 30。三者均缺已確認的官方語義，135 的用途未定。Garmin 訊息 233 共 14,604 筆且無獨立時間戳，22、288 只有零星記錄。Bryton 的 65282、65286 亦保留。Apple 的體力消耗強度、運動分鐘與日照只確認同手錶同時段，缺逐筆運動歸屬。
6. **M2 正式需求。** 交付可離線合併並輸出 FIT、可自動發佈到 Strava 的 macOS Rust CLI。用 `--help` 與 `--skill <topic>` 說明操作。CLI、Parquet 交付、實檔未知欄位寫回及 Strava 實際匯入仍待實作驗證。M3 收錄憑證安全保管、正式發佈與其他整合。[M2 流程、驗收與 M3 待辦](milestones/m2-cli.md)
