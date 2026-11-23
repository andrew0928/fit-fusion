# 騎乘合併器 CLI 規格

合併器只維護**一套 Rust 實作**，M2 在 macOS 使用 RustyFIT。它從同一次騎乘的多份記錄選取資料，在 Bryton 時間軸上產生一份可發佈的 FIT 與來源報告。Python、C#、Rust、Node.js 檢查器可獨立讀回這個輸出。各語言的驗收結果分開記錄。[技術決策](../decisions/2026-11-21-production-fit-sdk.md) · [檢查器規格](inspector.md)

## 輸入與操作

| 目標入口 | 行為 |
|---|---|
| `fit-fusion merge --date YYYY-MM-DD [--json]` | 讀取 v1 設定，依設定時區找出在當日開始的主來源騎乘，逐趟配對需要的來源。跨午夜的一趟歸出發日。同日多趟分別輸出。 |
| `fit-fusion merge <FIT> <FIT> --out <dir>` | 可離線處理 Bryton 與 CIRQA 原檔。依檔內設備與實際時段識別，輸入順序可對調。若只有 Bryton，可用 `--garmin auto` 或 `--garmin-id <id>` 取回對應原檔。 |
| `--publish strava`、`fit-fusion publish strava --from <dir>` | 前者在合併並驗證後發佈，後者使用已完成的合併產物。預設執行 merge 只交付本機檔案。 |
| `--help`、`--skill <topic>` | help 提供人用說明。skill 提供 agent 可重現的前置條件、命令、輸出、結束碼與重試方式。無帳號或網路也能查指引。 |

來源的檔案位置、Garmin 個人 key／access token、Strava 授權及來源順位依已確認的 config v1。兩個本機 FIT 可使用內建預設離線處理。目標命令與授權步驟見[M2 命令契約](../milestones/m2-cli.md)及[設定檔 v1](../config/README.md)。

## 合併與交付

1. **配對活動。** 取得每份來源的設備、運動種類、UTC 起訖與 SHA-256。預設兩份記錄的交集時長各占該份記錄至少 90%，且同一來源只有一個合格候選時才自動配對。有歧義時回報候選、起訖與比率。仍可處理當天其他有唯一配對的騎乘。
2. **挑選串流。** 按設定的同名欄位及來源順位處理，保留原始採樣。預設心率以 Bryton 為先，短缺口由同秒 Garmin 補，長缺或有對照佐證的停滯則整趟換成合格的 Garmin。Garmin 自身缺值時留空。GPS 經緯度成對、踏頻零值有效。Apple Watch 留供分析。[逐欄規則](../specification.md#strava-fit-逐欄選源表) · [心率品質判準](../decisions/2026-11-23-heart-rate-quality.md)
3. **選擇摘要。** 比對同趟可比較時段，標準 `session.total_calories` 採 CIRQA 同名原值。合併 FIT 另保留 CIRQA 原始總量與代謝量。報告列出 Bryton 原值、修正依據及衍生的活動熱量。M2 只選同名欄位。[熱量規格](../decisions/2026-11-23-ride-calorie-source.md)
4. **驗證並交付。** 原檔唯讀，另存 `merged.fit` 與 `merge-report.json`。交付前核對 CRC、讀回值及雜湊。報告可追溯每個選值的來源、原始欄位／時間，以及品質判定、缺口與未辨識欄位。`--parquet` 另產完整分析資料，包含原始 frame 與逐點選值的對照。
5. **發佈與重跑。** 只有取得 Strava `activity_id` 才回報已發佈。保存 `upload_id` 以便續查。遇重複活動或送出結果未知時，保留本機結果及診斷。對相同輸入與有效設定重跑應重用已驗證產物，並依運動員及 FIT 雜湊重用發佈紀錄。

## 可觀察的結果

`--json` 讓 stdout 輸出一份機讀結果，診斷送 stderr。日期批次含 `date`、`timezone`、`rides[]` 與整批狀態。結束碼 0 表示所要求的工作完成，也適用於查無活動。2 表示用法或設定錯誤。3 表示資料、品質、產物驗證或判重問題。4 表示授權問題。5 表示處理中或限流。6 表示平台失敗或送出結果未知。批次部分完成時按逐趟結果呈現。[日期及批次契約](../config/README.md#只給日期時的處理契約)

目前已實作雙檔離線入口、內建預設順位、心率品質檢查、熱量與來源副本、讀回驗證及安全重跑。用 RustyFIT 解碼與驗證，以主 FIT 的欄位定義和原始 payload 保留未修改欄位。六趟產物已用 Python 檢查器逐秒比對。自訂 config、日期探索、平台 API 與 Parquet 屬後續交付。[合併器驗收情境與案例](testcases/merger.md)
