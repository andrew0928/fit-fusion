# 決策文件規範

每個重要決策使用一份 `YYYY-MM-DD-topic.md`。內容依序包含「狀態、背景、決策、影響、替代方案、後續工作」。記下取捨理由、受影響的程式或資料及驗證方式；已確認的事項註明日期和依據。

| 狀態 | 使用時機 |
|---|---|
| `proposed` | 已提出，等待確認或驗證。 |
| `accepted` | 已確認採用，作為後續實作依據。 |
| `superseded` | 已由後續決策取代，在原文件連到取代者。 |

新增或更名時同步更新下表，每份決策一列。依檔名日期由新到舊排列，同日保留既有相對順序。

## 決策索引

| 日期 | 決策 | 狀態 |
|---|---|---|
| 2026-11-25 | [M4 依用途拆分 CLI，移轉 M3 並廢除 M2-4](2026-11-25-cli-split.md)（合併入口與時程由同日後續決策取代） | accepted |
| 2026-11-25 | [0.2.x 獨立工具、首項雙檔覆寫與 0.3.0 舊 CLI 退場](2026-11-25-independent-pair-merge.md) | accepted |
| 2026-11-25 | [所有 CLI 的 clap、help 與 skill 共同契約](2026-11-25-common-cli.md) | accepted |
| 2026-11-25 | [本機 FIT 依出發日期查詢，按原始內容匯出](2026-11-25-local-fit-query.md) | accepted |
| 2026-11-25 | [Garmin 獨立查詢使用個人 API 金鑰，保留本機原始 FIT](2026-11-25-garmin-query-credentials.md) | superseded |
| 2026-11-25 | [三支查詢的名稱與文字輸出、檢查器的取樣介面](2026-11-25-query-display-and-inspector-streams.md) | accepted |
| 2026-11-25 | [v0.2.1 發佈與 M4 開發順序](2026-11-25-v021-roadmap.md) | accepted |
| 2026-11-25 | [Garmin 僅讀 dotenv，0.3.0 規劃共通設定](2026-11-25-garmin-dotenv.md) | accepted |
| 2026-11-24 | [M2 設定與活動來源的模組邊界、介面簽名與驗收條件](2026-11-24-config-and-data-source.md) | accepted |
| 2026-11-24 | [以具名實例擴充設定中的設備與發佈目的地](2026-11-24-config-extensions.md) | accepted |
| 2026-11-24 | [Rust 模組以單元測試驗證，主要 CLI 流程以獨立檢查器驗收](2026-11-24-module-test-strategy.md) | accepted |
| 2026-11-24 | [先實作本機 FIT 目錄，Garmin API 與 Apple Health ZIP 分型保留](2026-11-24-local-fit-source.md) | accepted |
| 2026-11-24 | [本版手動上傳 Strava，曾將 M2-4 暫緩](2026-11-24-manual-strava-export.md) | superseded |
| 2026-11-24 | [config v1 沿用主 FIT，streams 選填，只列覆寫或追加欄位](2026-11-24-config-primary-inheritance.md) | accepted |
| 2026-11-24 | [用用途命名來源，從 FIT 識別實際設備](2026-11-24-source-roles.md) | accepted |
| 2026-11-24 | [以 M15、M18 代表舊設備驗收，透過 GitHub Release 發佈 v0.1.0](2026-11-24-v0.1.0-release.md) | accepted |
| 2026-11-24 | [兩支 Rust CLI 以 clap 宣告及解析命令列](2026-11-24-rust-clap.md) | accepted |
| 2026-11-23 | [Apple Watch 騎乘資料的漸進式匯出](2026-11-23-apple-health-incremental-export.md) | proposed |
| 2026-11-23 | [整趟騎乘的熱量消耗優先使用 Garmin](2026-11-23-ride-calorie-source.md)（發佈 CIRQA 總熱量並另存兩個原值） | accepted |
| 2026-11-23 | [心率先判整趟品質，再決定逐點補缺或整趟改用 Garmin](2026-11-23-heart-rate-quality.md) | accepted |
| 2026-11-23 | [M2 交付可供 agent 操作的 macOS 融合 CLI，M3 處理安全保管與發佈](2026-11-23-cli-milestones.md)（M2 已交付，M3 排程由 [M4](2026-11-25-cli-split.md) 取代） | accepted |
| 2026-11-23 | [個人設定檔分開定義來源與逐串流順位，依日期從主來源找活動](2026-11-23-date-driven-config.md)（M2-3 六趟 API 與離線配對已驗收，11/24 確認單檔自動配對旗標 `--auto-match`） | accepted |
| 2026-11-23 | [專案以 docs、src、data 與 runtime 分開保存文件、程式及本機資料](2026-11-23-project-layout.md) | accepted |
| 2026-11-23 | [多語言 FIT 檢查器與單一 Rust 合併器分別驗收](2026-11-23-inspector-and-merger.md) | accepted |
| 2026-11-23 | [三種檢查器以解碼後的欄位重新編碼 FIT](2026-11-23-inspector-roundtrip.md)（三語言驗收後啟動 M2，11/24 補齊 Node.js 對照） | accepted |
| 2026-11-22 | [騎乘累積距離採用 Bryton 主來源](2026-11-22-primary-distance-stream.md) | accepted |
| 2026-11-22 | [騎乘資訊流優先使用 Bryton，心率缺漏由 Garmin 替補](2026-11-22-bryton-primary-streams.md)（11/23 確認心率第二順位） | accepted |
| 2026-11-22 | [CLI 依時間配對同趟記錄，按合併清單與兩級順位選取來源](2026-11-22-device-and-stream-selection.md)（11/23 確認 M2 同名選值與後續 rename） | accepted |
| 2026-11-22 | [11 月 21 日騎乘以 CIRQA 本機心率修復廣播中斷](2026-11-22-nov21-heart-rate-repair.md) | accepted |
| 2026-11-22 | [以冷水坑實測評估路線與海拔來源](2026-11-22-position-and-elevation-sources.md) | proposed |
| 2026-11-22 | [四平台活動查詢、原生 FIT 下載與合併 FIT 發佈](2026-11-22-cloud-activity-import.md) | proposed |
| 2026-11-21 | [正式 CLI 採用 Rust、RustyFIT 與 macOS](2026-11-21-production-fit-sdk.md)（11/23 確認，[讀寫實測](../research/2026-11-23-rust-fit-cli.md)） | accepted |
| 2026-11-21 | [Milestone 1-1 使用獨立 C# CLI 驗證同一份 FIT](2026-11-21-csharp-inspection-control.md) | accepted |
