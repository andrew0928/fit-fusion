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
| 2026-11-23 | [整趟騎乘的熱量消耗優先使用 Garmin](2026-11-23-ride-calorie-source.md)（發佈 CIRQA 總熱量並另存兩個原值） | accepted |
| 2026-11-23 | [心率先判整趟品質，再決定逐點補缺或整趟改用 Garmin](2026-11-23-heart-rate-quality.md) | accepted |
| 2026-11-23 | [M2 交付可供 agent 操作的 macOS 融合 CLI，M3 處理安全保管與發佈](2026-11-23-cli-milestones.md) | accepted |
| 2026-11-23 | [個人設定檔分開定義來源與逐串流順位，依日期從主來源找活動](2026-11-23-date-driven-config.md) | accepted |
| 2026-11-23 | [專案以 docs、src、data 與 runtime 分開保存文件、程式及本機資料](2026-11-23-project-layout.md) | accepted |
| 2026-11-23 | [多語言 FIT 檢查器與單一 Rust 合併器分別驗收](2026-11-23-inspector-and-merger.md) | accepted |
| 2026-11-23 | [三種檢查器以解碼後的欄位重新編碼 FIT](2026-11-23-inspector-roundtrip.md)（已驗收並啟動 M2 離線合併） | accepted |
| 2026-11-22 | [騎乘累積距離採用 Bryton 主來源](2026-11-22-primary-distance-stream.md) | accepted |
| 2026-11-22 | [騎乘資訊流優先使用 Bryton，心率缺漏由 Garmin 替補](2026-11-22-bryton-primary-streams.md)（11/23 確認心率第二順位） | accepted |
| 2026-11-22 | [CLI 依時間配對同趟記錄，按合併清單與兩級順位選取來源](2026-11-22-device-and-stream-selection.md)（11/23 確認 M2 同名選值與後續 rename） | accepted |
| 2026-11-22 | [11 月 21 日騎乘以 CIRQA 本機心率修復廣播中斷](2026-11-22-nov21-heart-rate-repair.md) | accepted |
| 2026-11-22 | [以冷水坑實測評估路線與海拔來源](2026-11-22-position-and-elevation-sources.md) | proposed |
| 2026-11-22 | [四平台活動查詢、原生 FIT 下載與合併 FIT 發佈](2026-11-22-cloud-activity-import.md) | proposed |
| 2026-11-21 | [正式 CLI 採用 Rust、RustyFIT 與 macOS](2026-11-21-production-fit-sdk.md)（11/23 確認，[讀寫實測](../research/2026-11-23-rust-fit-cli.md)） | accepted |
| 2026-11-21 | [Milestone 1-1 使用獨立 C# CLI 驗證同一份 FIT](2026-11-21-csharp-inspection-control.md) | accepted |
