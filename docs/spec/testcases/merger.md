# 合併器驗收情境與案例

所有案例驗收同一套 macOS Rust CLI。下表列出整個 M2 的產品驗收。有實騎資料的列可用既有回放作為預期值。檔內數值以獨立檢查器的 `raw` 驗證，涵蓋率和取樣統計以 `inspect` 驗證。每趟都要保留來源雜湊及合併報告。[合併器規格](../merger.md) · [檢查器規格](../inspector.md)

| 案例 | 輸入與操作 | 通過條件 |
|---|---|---|
| M01　離線雙檔與指引 | 清除測試環境的帳號設定、停用網路。以 `--help`、`--skill index`、`--skill merge` 找到操作方式，將 11/21 雙檔交給 `merge`，再對調輸入順序。 | 兩次均辨識同一車表與手環，回傳 0。產出 `merged.fit`、`merge-report.json`，選值及來源一致，無上傳。兩份原始 FIT 雜湊保持一致。 |
| M02　短缺口 | 合併 11/01 冷水坑騎乘。Bryton 主時間軸有 18,361 個記錄秒，心率缺 34 秒。 | 18,327 個有效車表心率逐值保留，只在 34 個缺秒取同秒 CIRQA 值。報告中的來源計數相符。 |
| M03　有值卻停滯 | 合併 11/21 騎乘。Bryton 只有 65 個明確缺秒，另有 68、86 秒兩段受 Garmin 佐證的停滯，以及一段未達分歧門檻的 90 秒平台。 | 報告列出兩段觸發區間。全部 14,149 個輸出心率樣本取自 CIRQA，逐秒相符。90 秒平台本身不列為觸發原因。主時間軸及其他串流仍按自己的順位。 |
| M04　備援也不合格 | 合成 1,000 個連續記錄秒，兩邊時鐘對齊。Bryton 連續缺 600 秒，Garmin 在該段只提供 500 秒。 | 觸發整趟檢查，Garmin 在異常段覆蓋率僅 83.33%。回傳 3 及 `quality_review`，報告保留區段與兩台覆蓋率。本機結果標記為待判讀。 |
| M05　總熱量與來源副本 | 對冷水坑合併 FIT 讀取 session 標準欄、developer 定義與報告。 | 標準 `total_calories` 為 CIRQA 的 1,897 kcal，`metabolic_calories` 為 383。可用來源命名空間獨立讀回同名原值 1,897、383。報告列 Bryton 原值 6,186、衍生活動熱量 1,514。 |
| M06　列表與有效零值 | 合成一個含配對經緯度、0 rpm 與溫度的主檔。刻意讓其中一點缺經度，省略 `record.temperature`，明確指定經緯度皆取自主來源。 | 輸出保留 0 rpm，該點無可用座標對，溫度串流與溫度摘要保留主檔原值。來源的溫度仍可追溯。 |
| M07　日期、跨午夜與空日 | 在只有六趟實檔的本機目錄以台北時區分別執行 `--date 2026-10-14`、`2026-10-15`。另將一天的測試資料安排為上午、下午各一趟。 | 10/14 回傳一趟，輸出含跨午夜的後半段。10/15 回傳 0、`no_activities`、`rides: []`。同日兩趟都有唯一配對時，分開輸出兩組產物。 |
| M08　候選歧義 | 建立同日兩趟主活動。第一趟只配到一筆 Garmin。第二趟配到兩個內容不同、各自達雙向 90% 門檻的候選。 | 第一趟交付。第二趟回報 `ambiguous_match`，附兩筆 ID、起訖及雙方比率。批次為 `partial`、結束碼 3。 |
| M09　來源保存與重跑 | 合併含未知訊息及同號 developer field 的樣本並加 `--parquet`。以相同輸入與設定重跑，再新增一條溫度的選源規則重跑日期模式。 | CRC 正確，未改動的主檔欄位與原值相符。以 Parquet 內的原始 frame 依序重建兩份來源 SHA-256。相同輸入重用產物，變更有效選源產生新修訂，可追查每個選值。 |
| M10　設定與 Garmin 取檔 | `config check` 用含不存在來源 ID 的設定。修正後以模擬 Garmin 回傳分頁、同一活動 ID 與已到期 token。另以同趟本機雙檔對照。 | 無效引用回傳 2 並指出設定位置。有效 key 可換發並取得唯一原檔，來源僅計一次，合併值與離線相符。需重新授權時回傳 4，輸出與報告均無秘密。 |
| M11（已廢除）　Strava 成功與重試 | 模擬授權帳號收到上傳後先回 `upload_id`、稍後才有 `activity_id`。在處理中或 429 時中斷，接著重跑同一份已驗證產物。 | 暫時回傳 5 及續跑指引。已取得上傳 ID 時只續查。最後回傳 0、`published` 與活動 ID。對同一帳號及 FIT 的成功送件共一次。 |
| M12（已廢除）　重複及送件結果未知 | 分別模擬 Bryton 已先同步而被平台判重，以及 POST 送出後斷線且未取得上傳 ID。 | 第一種回傳 3、`duplicate`，既有活動保留。第二種回傳 6、`submission_unknown`，重跑時不盲目再 POST。兩種均可取得原本的本機產物。 |
| M13（已廢除）　實際發佈驗收 | 使用經騎士授權、尚未上傳的新騎乘，以 `--publish strava` 送件。以具讀取權限的帳號讀回該活動。 | 保存 upload ID、activity ID，確認遠端活動屬於該趟，讀回的 `calories` 與輸出 FIT 的標準總熱量相符。此案例獨立於模擬測試記錄。 |
| M14　無個資的最小成功案例 | 建立兩個各 6 秒的 cycling session，同時開始、無暫停、每秒一筆。設備識別分別為 Rider 460 `(267, 2205)`、CIRQA `(1, 5019)`。Bryton 心率依序為 `[110, 111, 255, 114, 114, 115]`，Garmin 為 `[109, 110, 113, 113, 114, 115]`。Bryton 總熱量 6 kcal，Garmin 總量 2、代謝量 1 kcal。用預設順位離線合併。 | 結束碼 0。以獨立檢查器讀回逐點心率 `[110, 111, 113, 114, 114, 115]`、標準總熱量 2 kcal。報告列心率首選 5 筆、替補 1 筆、缺值 0 筆，以及活動熱量 1 kcal。 |
| M15　Bryton Rider 15 Neo 單機實檔 | 唯讀取得一份 Rider 15 Neo 原生 FIT，從 `file_id` 讀設備識別，建立獨立的本機來源。以空白 streams 規則按出發日期合併。 | `activities list` 只列該原檔 SHA-256；合併 FIT 與輸入逐位元組一致，重跑重用同一份產物。 |
| M16　其他廠牌單機實檔 | 每個實際提供的廠牌／型號至少選一份，重跑 M15。由已安裝的同一支 Rust CLI 讀取新設定。 | 每型號獨立記錄結果，涵蓋原檔內實際存在的串流、自訂及未知欄位。格式或必要 metadata 不相容時保存錯誤及原檔雜湊。 |
| M17　同趟跨裝置選值 | 以騎士確認同趟的兩份 FIT，挑有差異或缺值的一個原生串流（踏頻、功率、溫度）。以測試設定指定順位。 | 依日期唯一配對。獨立檢查器逐個主檔時間戳重算選值，對照輸出、來源區段與其餘 record 欄位。第二來源須至少實際改變一筆輸出，兩端都缺則保留缺值。 |
| M18　不同趟的負對照 | 在兩個來源各放一份可讀、已知不同趟且未達時段重疊門檻的 FIT，讓 CLI 按日期尋找次來源。 | 兩檔均可被各自的來源列舉，合併報告只引用主檔 SHA-256，輸出逐位元組等於主檔。 |

六趟心率回歸預期為 10/09 補 78 秒、10/14 補 1 秒、11/01 補 34 秒。10/28、11/08、11/21 整趟改用 Garmin，分別取 9,632、17,190、14,149 個樣本。來源與時段由本機 `data/index/rides.csv` 對應，回放依據另有記錄。[心率回放與停滯區段](../../research/2026-11-23-heart-rate-quality.md) · [熱量實檔對照](../../research/2026-11-23-strava-ride-calories.md)

## 端對端驗收的代表流程

端對端測試啟動真正的 Rust CLI，準備設定與輸入檔，觀察結束碼、JSON、產物與對外請求。下表涵蓋原規劃的三條流程，涉及 Strava transport 的驗收隨 M2-4 廢除。逐一測試門檻、錯誤分類與來源排列的工作由 [A–D 模組單元測試](module-decision-tables.md) 負責。

| 流程 | 測試資料及操作 | 從 CLI 外部驗證 |
|---|---|---|
| 依日期找到活動並輸出 FIT（M07、M14） | 用 C# + Garmin FIT SDK 產生 M14 的六秒雙檔，置於兩個本機來源目錄。以設定時區的出發日呼叫 `merge --date … --json`。 | 回傳 0、僅一趟。獨立 inspector CLI 用 `raw` 讀回心率 `[110, 111, 113, 114, 114, 115]`、總熱量 2 kcal、代謝量 1 kcal，用 `inspect` 核對 6 筆 record。報告列心率來源 5＋1 筆，兩份輸入雜湊與執行前一致。 |
| 品質不足（M04） | 交給本機合併器處理 M04 的雙檔。 | 回傳 3 與 `quality_review`，指出 Garmin 在異常段僅覆蓋 500/600 秒。 |
| 已廢除的歷史案例（M11） | 使用已驗證的合併產物。測試端點先接受 FIT 並回傳 upload ID，再模擬暫停與重跑，最後回傳 activity ID。 | 收到的 FIT 雜湊與已驗證產物一致。重跑沿用 upload ID，最終回傳 0、`published` 與同一活動 ID。端點只收到一次上傳。 |

每份新合成 FIT 的預期欄位值來自事先寫定的案例。讀回可用現有 C# `WireReader` 或 Python wire inspector，兩者均獨立於 Rust 合併器。C# 的官方 Garmin SDK 用於造檔，現有 C# inspector 有自己的解碼器。M13 原規劃的 API 驗收已結案，手動網頁匯入有獨立實測紀錄。[檢查器後端](../inspector.md#語言與解碼器)

## 11/21 離線驗收步驟

以 M01、M03 為同一趟的兩種觀察。從根目錄執行以下目標命令，在全新的輸出目錄保存結果。使用 Python 或 C# 檢查器讀回 FIT，確認 `record` 數、心率涵蓋及原始逐點值，再核對合併報告中的來源與觸發區段。

```sh
fit-fusion merge data/sources/bryton/fit/261121141151.fit \
  data/sources/garmin/25149578721/25149578721_ACTIVITY.fit \
  --out runtime/acceptance/2026-11-21 --json
PYTHONPATH=src/python python3 -m fit_fusion inspect --decoder wire \
  runtime/acceptance/2026-11-21/merged.fit
```

## 目前可重跑的驗收

Rust 命令列驗收涵蓋全域選項換序、群組 help 與檔名後的 help。另將 0–3 個 FIT 輸入與日期、自動配對、Garmin ID、輸出目錄四個條件展開為 64 種組合，使用不存在的設定檔區分兩種結果。有效的命令進入設定載入，回傳 `invalid_config`。無效的命令在解析階段回傳 `usage`。兩者都有可解析的 JSON，且不建立產物。[自動化驗收](../../../src/tools/tests/test_merger.py)

M01 的指引驗收從執行檔的 `--skill index` 發現各主題，擷取頁面中的設定與命令，使用同一對測試 FIT 執行四種合併方式。驗收比對輸出 FIT、報告雜湊與各模式的產物路徑，並檢查空日、當日部分成功及重跑沿用產物。[指引驗收案例](../../../src/tools/tests/test_merger.py)

單檔 `--auto-match` 另以手環主檔配對 Bryton 車錶目錄，並配置一個名為 `garmin`、未列入串流順位的來源。驗收要求 `matched_sources` 只列車錶，產生的 FIT 與手動提供雙檔逐位元組相同。舊語法、重複旗標、額外參數或與其他模式混用時，應回傳結束碼 2 及 `usage`。

從根目錄執行 `python3 -m unittest discover -s src/tools/tests -p test_merger.py -v`。合成測試包含 M14 的逐秒讀回、M04 的備援失敗、developer 欄位同號及目的目錄防覆寫。這台 Mac 有六趟私人樣本時也會執行實檔測試，逐秒檢查 M01–M03 的選源、M05 的標準與 developer 熱量，並比較未修改的串流與私有訊息。缺少私人索引時 unittest 會明列跳過該測試。[自動化驗收](../../../src/tools/tests/test_merger.py)

原有 Python 雙檔回歸持續驗證歷史輸出。新增測試用 [C# 產生器](../../../src/csharp/FitFixtures/Program.cs) 與 Garmin.FIT.Sdk 21.222.0 固定設備、六秒心率、踏頻及熱量。測試執行 config check、本機目錄列舉、指定雙檔合併，再以獨立 Python inspector CLI 的 raw 讀回 M14 的心率與熱量。日期入口另有 M06–M08、M09 修訂部分與 M10 驗收。M14 直接以日期模式合併，再透過 Python inspector CLI 讀回。Strava 自動發佈的驗收隨 M2-4 廢除。若要重建測試檔，可從根目錄執行 `dotnet run --project src/csharp/FitFixtures -- <新的空目錄>`，再比對兩邊的檔案雜湊。

### M2-3 日期與 Garmin 實測

六趟出發日（10/09、10/14、10/28、11/01、11/08、11/21）分別透過正式 Garmin Personal API 取回原始 FIT，以相同設定與既有雙檔比對。六份合併檔逐位元組一致。另以本機目錄重跑這六趟，10/14 跨午夜的完整輸出與雙檔相同，10/15 回傳空清單。正式 API 驗證使用私人金鑰，日常自動化測試使用可控制的 Garmin 回應。

合成測試使用 C# + Garmin SDK 產生的 FIT，驗證同日兩趟、缺備援後補齊、改名重跑、兩筆候選、兩個主檔共用候選、手動指定 ID 及改變 config 後另存修訂。可用 `dotnet run --project src/csharp/FitFixtures -- <新的空目錄> --scenarios` 重建日期配對情境檔。M09 的 Parquet 驗收見下節。

### M2-5 分析匯出（M09）

六個出發日各自以兩個本機 FIT 來源執行日期合併。當初 M2-5 驗收時，啟用分析輸出與 M2-3 的 FIT 逐位元組相同。改用主檔保留規則後重新逐欄核對這六趟，差異均為重新保留主檔三個值為 FIT 缺值碼的 session 欄位（平均、最大、標準化功率，各為 65535）。每份 FIT 增加 15 bytes，原有心率與熱量相同。用 DuckDB 讀回各來源的 frame，驗證區段連續並逐位元組重建 12 份原檔，同時核對 SHA-256。對每個非空選值，依來源、frame 位置、訊息、原生欄號比回相同的原始值，六趟均無差異。

| 資料與操作 | 獨立讀回結果 |
|---|---|
| 11/21 在兩段停滯逐秒查原值與選值 | 68 秒中 Bryton 固定 102 bpm，Garmin 為 102–112。另一段 86 秒 Bryton 固定 148，Garmin 為 138–148。整趟 14,149 筆選值均來自 Garmin。 |
| 11/01 冷水坑查 session 熱量 | Bryton 原始 6,186 kcal。Garmin 原始總量 1,897、代謝量 383。選值可回連到兩個原生欄位，衍生活動熱量為 1,514。 |
| 同一輸入重跑，再變更輸出格式或有效選源 | 未改設定時回傳 `reused: true`。日期模式切換格式或移除溫度會建立不同修訂，上次產物可繼續讀取。篡改或移除既有 Parquet 回傳 `output_conflict`。 |
| 含未知原生欄及同號 developer 欄的合成檔 | 保留欄號、developer index 與原始位元組。手動指定但未參與選值的第二份 FIT 亦可重建。 |

從根目錄用裝有 DuckDB 的 Python 執行 `python -m unittest discover -s src/tools/tests -p test_merger.py -v`。獨立讀回測試使用 Python 套件 `duckdb`（此版驗收使用 1.4.5）；Rust CLI 自行產生 Parquet。缺少私人樣本或 DuckDB 時，測試會明列略過的案例。[分析檔結構與查詢](../merger.md#parquet-分析檔-v2) · [自動化驗收](../../../src/tools/tests/test_merger.py)

### config v1 擴充驗收

C# 與 Garmin FIT SDK 產生兩台未註冊的合成車錶。用 [v1 設定](../../config/stream-fit.example.json) 依日期或交換雙檔順序合併，以 Python 讀回。省略 `merge.streams` 時，輸出與主檔逐位元組一致。改填 `{}` 重跑會重用同一修訂。配置功率順位後，六秒依序為 0、120、170、250、180 W、缺值；170 W 由第二台補入。左右平衡與陣列型踩踏功率相位可直接按 Profile 欄名從第二台選取。執行 `dotnet run --project src/csharp/FitFixtures -- <新目錄> --streams` 可重建兩個樣本。

## Strava 網頁手動匯入實測

使用者手動上傳 Rust 合併器輸出的合成室內騎乘 FIT，並提供活動概觀截圖。測試活動開始於 2026-11-22 08:37（台北）。受測檔案的 SHA-256 為 `f2e534ae3a9d792487038487d8475cd480e56213d7fc2b30cfec8c0443031109`。

| 核對項目 | 合併 FIT 的預期值 | Strava 活動概觀 |
|---|---|---|
| 總熱量 | Bryton 輸入 410 kcal，合併後的 `session.total_calories` 取 CIRQA 的 73 kcal | 73 kcal |
| 距離與時間 | 3.00 公里、10 分鐘 | 3.00 公里，移動與經過時間皆為 10:00 |
| 心率 | 平均 130、最高 140 bpm | 平均 130、最高 140 bpm |
| 踏頻 | 80 rpm | 平均與最高皆為 80 rpm |
| 記錄裝置 | 主檔識別為 Bryton Rider 460 | Bryton Rider 460 |

此案例通過網頁匯入與顯示值驗收。M13 的 CLI 自動上傳、取得 upload ID 與透過 API 讀回活動仍待驗證。

## 舊車錶與跨廠牌實檔驗收（M15–M18）

1. **收件與盤點。** 原始 FIT 或 ZIP 放在 Git 忽略的 `data/sources/legacy/`，各型號分目錄存放，ZIP 保留原件，取出的 FIT 另存。記下原檔 SHA-256、實際廠牌與型號、騎乘日期／時區、已知同趟關係。設備代碼從 FIT 讀取。舊檔只有單機記錄即可做 M15 或 M16。
2. **檢查器。** 對代表實檔執行 `python3 src/tools/verify_roundtrip.py FILE.fit`，沿用 M1 四語言工具比較逐框資料、欄位統計與重編碼結果；Garmin JS SDK 的能力差異單獨記錄。
3. **合併器。** 使用原檔的獨立副本，執行 Rust CLI 的 `config check`、`activities list`、`merge --date`，再以 Python inspector CLI 讀回。M17 的順位只作用於該案例。原始活動時間、正式心率及熱量規則均沿用現況。每次執行另存一份 `runtime/reports/device-e2e/run-*/verification.json`，回報狀態、輸入雜湊及成功產物，失敗含原因。[執行器](../../../src/tools/verify_device_e2e.py)

從專案根目錄執行。日期由原檔 session 起點與時區取得，時區預設為 `Asia/Taipei`，可用 `--timezone` 調整。

```sh
python3 src/tools/verify_device_e2e.py neo15.fit
python3 src/tools/verify_device_e2e.py other-brand.fit
python3 src/tools/verify_device_e2e.py neo15.fit --secondary same-ride.fit --stream cadence
python3 src/tools/verify_device_e2e.py neo15.fit --unrelated different-ride.fit
```

M17 先驗證主來源優先、次來源補缺。如果兩台所選串流都完整且有不同值，可加 `--prefer secondary` 檢查整欄改取次來源。報告列各來源樣本數；若兩份資料無從觀察這個欄位的替換，需改用其他串流或樣本。

**實檔驗收結果**

| 案例 | 結果 |
|---|---|
| M15 單機保留 | Bryton Rider 15 Neo 共 19 檔，11 份成功輸出且逐位元組相同。5 份 CRC 錯誤、1 份截斷、2 份時間戳重複。 |
| M16 其他廠牌 | **略過**。騎士確認沒有其他廠牌 FIT，舊車錶單機以 M15 為代表。 |
| M17 同趟補值 | **略過實機雙檔案例**。騎士確認沒有同期舊設備雙檔。另行保留 2023-07-08 的 Rider 15 Neo 與 C# Garmin SDK 合成功率對照，其 15,915 筆中正確補 3 筆（0、180、220 W）。 |
| M18 不同趟排除 | 2023-07-04 Rider 15 Neo 與 2026-11-21 CIRQA 各自可列舉，按日期只輸出主檔，位元組相同。 |

空覆寫另以 11/21 Rider 460 的 14,149 筆 record 驗過。雙合成車錶的控制組涵蓋功率逐秒選值、反轉順位及重跑。[舊車錶實檔、SDK 與熱量對照](../../research/2026-11-24-rider-15-neo.md)

設備名稱測試可用 `dotnet run --project src/csharp/FitFixtures -- <新目錄> --identities` 重建。三個 FIT 分別驗證檔頭名稱優先、採用建立者名稱，以及忽略外接感測器名稱。另以同一個無型號篩選的 `bike_meter` 目錄列舉 11 份 Rider 15 Neo、6 份 Rider 460，17 趟皆能依原檔辨識型號。空覆寫輸出雜湊逐趟一致。
