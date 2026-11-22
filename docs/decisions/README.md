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
| 2026-11-22 | [騎乘累積距離採用 Bryton 主來源](2026-11-22-primary-distance-stream.md) | accepted |
| 2026-11-22 | [Bryton 有提供的騎乘資訊流優先使用 Bryton](2026-11-22-bryton-primary-streams.md) | accepted |
| 2026-11-22 | [CLI 依預先定義的裝置與資訊流對照表選取來源](2026-11-22-device-and-stream-selection.md) | accepted |
| 2026-11-22 | [11 月 21 日騎乘以 CIRQA 本機心率修復廣播中斷](2026-11-22-nov21-heart-rate-repair.md) | accepted |
| 2026-11-22 | [以冷水坑實測評估路線與海拔來源](2026-11-22-position-and-elevation-sources.md) | proposed |
| 2026-11-22 | [個人雲端活動取檔方式](2026-11-22-cloud-activity-import.md) | proposed |
| 2026-11-21 | [正式版 FIT 處理函式庫選型](2026-11-21-production-fit-sdk.md) | proposed |
| 2026-11-21 | [Milestone 1-1 使用獨立 C# CLI 驗證同一份 FIT](2026-11-21-csharp-inspection-control.md) | accepted |
