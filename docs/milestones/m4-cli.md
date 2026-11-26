# M4 開發計畫：獨立的 FIT 命令列工具

**M4 從 0.2.0 開始，第一個開發任務是 `fit-merge`。** 0.2.x 以完成四個新 CLI 為主；舊 `fit-fusion` 自 v0.2.4 預發佈退場。新工具可有重疊功能，一律獨立安裝及執行。版本方向、雙檔覆寫與 Apple 轉製 FIT 已確認；所有 CLI 須使用 clap 並通過共通的 help／skill 驗收。[共同規格](../spec/cli-common.md)[範圍決策](../decisions/2026-11-25-independent-pair-merge.md) · [CLI 規格](../spec/m4-cli.md) · [驗收表](../spec/testcases/m4-cli.md)

## 版本與里程碑

| 版本 | 狀態 | 交付與去向 |
|---|---|---|
| 0.1.0（M1、M2） | 已發佈 | 多語言檢查器、舊合併器、Garmin 日期配對與 Parquet；舊設備以 M15、M18 驗收。M2-4 已廢除，使用者手動上傳 Strava。 |
| M3 | 已廢除 | 安全保管、發佈、整合與 rename 分派至下方工作項目。 |
| 0.2.0（M4 起點） | 已發佈 | `fit-merge`、`fit-query-files`、既有 `fit-fusion` 與 `fit-inspector` 的 macOS arm64 套件及 SHA-256。[GitHub Release v0.2.0](https://github.com/andrew0928/fit-fusion/releases/tag/v0.2.0) |
| 0.2.1 | M4-3 review 通過，發佈版 | 新增 `fit-query-garmin`，並更新兩支查詢工具的人讀版輸出。正式帳號使用 `.env` 查詢與下載成功。依確認順序先做 M4-5，再做 M4-4。 |
| 0.2.2 | 已發佈，Homebrew 供測試 | M4-5 四語言檢查器提供逐串流缺樣統計，M4-4 提供 Apple Health ZIP 查詢與 FIT 匯出。R10、R11、R12 通過。使用者已完成 Homebrew 乾淨環境驗收，R14 review 通過。[GitHub Release v0.2.2](https://github.com/andrew0928/fit-fusion/releases/tag/v0.2.2) |
| 0.2.3（M5） | 已發佈，M5-4、M5-5 完成 | 共通 YAML 與參數來源顯示通過使用者 review；從公開 tap 進行 0.2.2 → 0.2.3 升級驗收通過。[M5](m5-release.md) |
| 0.2.4（M5） | 預發佈 | 移除舊 CLI 原始碼與執行檔，保留五支獨立工具。 |
| 0.3.0（M5） | 待 0.2.x 驗收完成 | 首次正式發佈。 |

## M4 執行順序與完成條件

以下編號取代首稿。M4-1 直接使用本機測試檔開工，功能不依賴後續的查詢或分析任務。

| 編號 | 工作 | 可驗收的交付 |
|---|---|---|
| **M4-1** | **0.2.0 首項：`fit-merge`** | 給主 FIT、附加 FIT、待覆寫的同名欄位和輸出路徑，即可離線合併。一次恰好兩份輸入，選定串流完全採用附檔，在附檔缺樣時保留缺值。以多次呼叫串接第三份檔。 |
| M4-2 | `fit-query-files` 與查詢介面（已實作，R08 通過） | 掃描指定 FIT 目錄，依條件 list、read 或 export 至指定目錄。匯出副本與原 FIT 的 SHA-256 相同，重跑可重用，發生衝突可定位。 |
| M4-3 | `fit-query-garmin`（已實作，R09 通過） | 查詢 Garmin Connect 或按 ID 下載原始 FIT。測跨日、分頁、授權到期及快取。Garmin 憑證由此來源負責。 |
| M4-5 | `fit-inspector` 取樣分析 | 0.2.2 提供 `inspect FILE [--stream NAME]…`，R11、R12 通過。唯讀列出各串流的有效樣本數、缺樣數、缺樣百分比與分母；可點名檔內完全沒有的串流。使用者可先檢查附檔，再決定是否覆寫。Python、C#、Node.js 入口亦已遷移至共用 clap 前端。 |
| M4-4 | `fit-query-apple`（0.2.2，R10 通過） | 讀本機 Apple Health ZIP，查詢 Workout、取樣與關聯路線；每趟匯出一份**標明來源的轉製 FIT**。以同時段 Watch 與 Connect 活動驗證來源辨識、缺項與重跑。納入三支 query 共通的活動／設備名稱及文字輸出。 |
| M4-6 | rename（已取消） | 設計顯式來源欄 → 目標欄的參數、單位／型別與衝突驗證。首版 `fit-merge` 只接受同名覆寫。 |
| M4-7 | 0.2.2 Homebrew 測試分發（完成，R14 通過） | 已驗證匿名下載與首次安裝。2026-11-25 使用者確認乾淨環境 R14 review 通過。正式分發、升級與舊 CLI 退場移交 M5。[R14 案例](../spec/testcases/m4-cli.md#r14review) · [M5 任務](m5-release.md) |
| M4-8 | 其他平台研究（已取消） | 分別驗證 Bryton 雲端取檔與其他目的地之授權、原始檔取得或上傳收據及去重。Strava 的交付流程到本機 FIT 為止。 |

## 原 M3 工作移轉

| 原需求 | 承接項目 | 範圍 |
|---|---|---|
| Garmin 憑證 | M4-3、M5-5 | v0.2.1 僅用 .env，M5-5 定義 ~/.fit-fusion/ 下的共通設定檔。 |
| 正式發佈管道 | M4-7、M5 | 0.2.2 公開附件、SHA-256、Homebrew 首次安裝與乾淨環境 R14 已驗收。後續升級與首次正式發佈由 [M5](m5-release.md) 追蹤。 |
| Apple 漸進匯入 | M4-4、M4-2 | 逐趟匯出 ZIP 中新增或變更的活動；若已有 HealthFit 同步的 FIT，交由 files 查詢。手機端持續到檔另行評估。 |
| 其他來源與目的地 | M4-8（已取消） | Bryton Active 雲端原檔與其他平台寫入分開研究。 |
| 跨欄位映射 | M4-6（已取消） | 合併維持同名欄位覆寫。 |

## 程式碼共用邊界

可從 0.1.0 搬出 FIT 解碼、欄位識別、時間對齊與寫檔程式，或直接使用相同 Rust 套件。獨立工具的產品契約由各自的用途決定；執行時只需自身執行檔、輸入與所需的來源授權。新工具的測試以預先寫定的 FIT 值和獨立檢查器為準，不安排與舊 `fit-fusion` 的組合流程或輸出等價測試。
