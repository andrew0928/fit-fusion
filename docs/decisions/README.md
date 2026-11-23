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
| 2026-11-23 | [Apple Watch 騎乘資料的漸進式匯出](2026-11-23-apple-health-incremental-export.md) | proposed |
| 2026-11-23 | [整趟騎乘的熱量消耗優先使用 Garmin](2026-11-23-ride-calorie-source.md) | accepted |
| 2026-11-22 | [騎乘累積距離採用 Bryton 主來源](2026-11-22-primary-distance-stream.md) | accepted |
| 2026-11-22 | [騎乘資訊流優先使用 Bryton，心率缺漏由 Garmin 替補](2026-11-22-bryton-primary-streams.md)（11/23 確認心率第二順位） | accepted |
| 2026-11-22 | [CLI 依時間配對同趟記錄，按合併清單與兩級順位選取來源](2026-11-22-device-and-stream-selection.md)（11/23 補充） | accepted |
| 2026-11-22 | [11 月 21 日騎乘以 CIRQA 本機心率修復廣播中斷](2026-11-22-nov21-heart-rate-repair.md) | accepted |
| 2026-11-22 | [以冷水坑實測評估路線與海拔來源](2026-11-22-position-and-elevation-sources.md) | proposed |
| 2026-11-22 | [四平台活動查詢、原生 FIT 下載與合併 FIT 發佈](2026-11-22-cloud-activity-import.md) | proposed |
| 2026-11-21 | [正式 CLI 採用 Rust、RustyFIT 與 macOS](2026-11-21-production-fit-sdk.md)（11/23 確認，[讀寫實測](../research/2026-11-23-rust-fit-cli.md)） | accepted |
| 2026-11-21 | [Milestone 1-1 使用獨立 C# CLI 驗證同一份 FIT](2026-11-21-csharp-inspection-control.md) | accepted |
