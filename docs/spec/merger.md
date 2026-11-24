# 騎乘合併器 CLI 規格

合併器只維護**一套 Rust 實作**，M2 在 macOS 使用 RustyFIT。它從同一次騎乘的多份記錄選取資料，在設定的主來源時間軸上產生一份可發佈的 FIT 與來源報告。Python、C#、Rust、Node.js 檢查器可獨立讀回這個輸出。現行 config v1 預設沿用主檔，`merge.streams` 只覆寫指定串流。設備可用 FIT 製造商與型號配置。[設定範例](../config/stream-fit.example.json)[技術決策](../decisions/2026-11-21-production-fit-sdk.md) · [檢查器規格](inspector.md)

設定如何決定主來源、逐欄順位與整趟選值，以及連線器如何開啟一趟記錄，見[設定與活動來源介面（C# 與 Rust 對照）](config-and-data-source.md)。

## 輸入與操作

| 目標入口 | 行為 |
|---|---|
| `fit-fusion merge --date YYYY-MM-DD [--parquet] [--json]` | 讀取 v1 設定，依設定時區找出在當日開始的主來源騎乘，逐趟配對需要的來源。跨午夜的一趟歸出發日。同日多趟分別輸出。 |
| `fit-fusion merge <FIT> <FIT> --out <dir>` | 可離線處理兩個已設定設備的原檔，預設為 Bryton 與 CIRQA。依檔內設備與實際時段識別，輸入順序可對調。若只有 Bryton，可用 `--garmin auto` 或 `--garmin-id <id>` 取回對應原檔。 |
| M2-4 延後的 `--publish strava`、`fit-fusion publish strava --from <dir>` | 前者在合併並驗證後發佈，後者使用已完成的合併產物。預設執行 merge 只交付本機檔案。 |
| `--help`、`--skill <topic>` | help 提供人用說明。skill 提供 agent 可重現的前置條件、命令、輸出、結束碼與重試方式。無帳號或網路也能查指引。 |

來源的檔案位置、Garmin 個人 key／access token、Strava 授權及來源順位依已確認的 config v1。兩個本機 FIT 可使用內建預設離線處理。所有合併入口接受 `--parquet`。目標命令與授權步驟見[M2 命令契約](../milestones/m2-cli.md)及[設定檔 v1](../config/README.md)。

## 合併與交付

1. **配對活動。** 取得每份來源的設備、運動種類、UTC 起訖與 SHA-256。預設兩份記錄的交集時長各占該份記錄至少 90%，且同一來源只有一個合格候選時才自動配對。有歧義時回報候選、起訖與比率。仍可處理當天其他有唯一配對的騎乘。
2. **挑選串流。** 按設定的同名欄位及來源順位處理，保留原始採樣。預設心率以 Bryton 為先，短缺口由同秒 Garmin 補，長缺或有對照佐證的停滯則整趟換成合格的 Garmin。Garmin 自身缺值時留空。GPS 經緯度成對、踏頻零值有效。Apple Watch 留供分析。[逐欄規則](../specification.md#strava-fit-逐欄選源表) · [心率品質判準](../decisions/2026-11-23-heart-rate-quality.md)
3. **選擇摘要。** 比對同趟可比較時段，標準 `session.total_calories` 採 CIRQA 同名原值。合併 FIT 另保留 CIRQA 原始總量與代謝量。報告列出 Bryton 原值、修正依據及衍生的活動熱量。M2 只選同名欄位。[熱量規格](../decisions/2026-11-23-ride-calorie-source.md)
4. **驗證並交付。** 原檔唯讀，另存 `merged.fit` 與 `merge-report.json`。交付前核對 CRC、讀回值及雜湊。報告可追溯每個選值的來源、原始欄位／時間，以及品質判定、缺口與未辨識欄位。`--parquet` 另產完整分析資料，包含原始 frame 與逐點選值的對照。
5. **發佈與重跑。** 本版由騎士手動上傳 FIT。下列自動發佈行為保留為 M2-4 契約。 只有取得 Strava `activity_id` 才回報已發佈。保存 `upload_id` 以便續查。遇重複活動或送出結果未知時，保留本機結果及診斷。對相同輸入與有效設定重跑應重用已驗證產物，並依運動員及 FIT 雜湊重用發佈紀錄。

## 可觀察的結果

`--json` 讓 stdout 輸出一份機讀結果，診斷送 stderr。日期批次含 `date`、`timezone`、`rides[]` 與整批狀態。結束碼 0 表示所要求的工作完成，也適用於查無活動。2 表示用法或設定錯誤。3 表示資料、品質、產物驗證或判重問題。4 表示授權問題。5 表示處理中或限流。6 表示平台失敗或送出結果未知。批次部分完成時按逐趟結果呈現。[日期及批次契約](../config/README.md#只給日期時的處理契約)

目前已實作雙檔離線入口、內建預設順位、心率品質檢查、熱量與來源副本、讀回驗證及安全重跑。用 RustyFIT 解碼與驗證，以主 FIT 的欄位定義和原始 payload 保留未修改欄位。六趟產物已用 Python 檢查器逐秒比對。M2-3 已支援日期探索、逐串流設定、本機 FIT 與 Garmin Personal API、逐趟配對、歧義處理與修訂重用。M2-5 支援 Parquet 分析匯出，M2-4 自動發佈已延後。[合併器驗收情境與案例](testcases/merger.md)

## Parquet 分析檔 v2

所有 `merge` 入口加上 `--parquet` 會將 `merged.parquet`、`merged.fit`、`merge-report.json` 一起交付。日期模式從 `rides[].artifacts.parquet` 取得路徑。手動模式從 `artifacts.parquet` 取得路徑，變更格式時需另給新的 `--out`。日期模式將輸入雜湊、有效選源、規則版本及分析格式版本納入修訂識別。若同一修訂的檔案缺漏或遭改寫，回報 `output_conflict` 並保留現場。

同一張表依 `row_kind` 查詢。時間欄 `timestamp_fit_s` 使用從 1989-12-31 00:00:00 UTC 起算的 FIT 秒數。原始取樣保留各來源的時刻，選值使用主檔時刻；摘要可透過來源與 frame 位置查回該設備的實際時間。

| `row_kind` | 每列的內容 | 如何追溯 |
|---|---|---|
| `frame` | 每個輸入檔的標頭、定義、資料或 CRC；`frame_kind` 分別為 `header`、`definition`、`data`、`crc`。 | 按 `source_id` 分組，依 `frame_offset` 遞增串接二進位 `frame_bytes`，即可逐位元組重建該來源，包括無時間戳與未知訊息。 |
| `sample` | 原始 data frame 的每個欄位。整數在 `value_int`，其餘值在 `value_json`，包括陣列、浮點、字串與未解讀的十六進位資料。保留 FIT 原始編碼值與缺值代碼。 | `source_id`、`frame_offset`、`message_id`、`field_id`、`developer_index` 定位一個欄位。原生欄的 developer index 為 NULL，已知原生欄另有 `field_name`。 |
| `selected` | 各覆寫串流在每個主時間點的最終值、直接沿用主檔的原生與 developer record 欄位，以及所選心率、熱量的 session／lap 摘要。無合格值時，數值、來源及位置皆為 NULL；有效的 0 會保留。 | 以來源、frame 位置、訊息、欄號及 developer index 連回 `sample`。判斷來源時讀具名的 `source_id`。 |
| `derived` | `session.active_calories` 為可推導的總熱量減代謝熱量，單位 kcal，無法計算時為 NULL。 | 有值時指向被選用的來源 session，可查回同一 frame 的欄位 11、196。 |

Parquet 檔案中繼資料的 `fit_fusion` 鍵是 JSON，包含 `schema_version`、每份輸入的 SHA-256、主來源、選源設定、規則版本、心率品質判定與合併 FIT 的 SHA-256。分析檔本身的 SHA-256 在報告的 `parquet.sha256`。手動雙檔時兩份均收錄；若未參與選源的第二台設備 無法對應唯一設定實例，以 `@input-secondary` 識別。日期模式只收錄本趟實際配對的來源。

以下 DuckDB 查詢在 11/21 的分析檔上逐秒列出第一段停滯的兩台原值及最終心率。可將檔名換成 CLI 回傳的完整路徑。

```sql
SELECT timestamp_fit_s,
       max(value_int) FILTER (WHERE row_kind = 'sample' AND source_id = 'bryton') AS bryton_bpm,
       max(value_int) FILTER (WHERE row_kind = 'sample' AND source_id = 'garmin') AS garmin_bpm,
       max(value_int) FILTER (WHERE row_kind = 'selected') AS merged_bpm,
       max(source_id) FILTER (WHERE row_kind = 'selected') AS selected_source
FROM read_parquet('merged.parquet')
WHERE field_name = 'record.heart_rate' AND timestamp_fit_s BETWEEN 1164179763 AND 1164179830
GROUP BY timestamp_fit_s
ORDER BY timestamp_fit_s;
```
