# 整趟騎乘的熱量消耗優先使用 Garmin

## 狀態

accepted。2026-11-23 使用者指定「熱量消耗以 garmin 為優先」，並要求標入既有的設備優先順序對照表。[三裝置對照表](../specification.md)

## 背景

11 月 1 日冷水坑騎乘，Bryton 原始 FIT 和 Strava 同步至 Apple 健康的熱量都是 6,186 kcal。Garmin 同趟的總熱量為 1,897 kcal，活動熱量為 1,514 kcal。另有車友報告其他 Bryton 型號偏高，Rider 460 的具體成因仍待釐清。[六趟實檔與公開資料](../research/2026-11-23-strava-ride-calories.md)

## 決策

已配對同趟騎乘，且熱量定義和時段可比較時，整趟熱量摘要以 Garmin CIRQA 為第一來源。保留總熱量與活動熱量兩個值。本批六份 Garmin FIT 可讀取 `session.total_calories` 與 `session.metabolic_calories`，兩者相減皆與 Connect 寫入 Apple 健康的活動熱量相符。

## 影響

Bryton 提供主時間軸，其熱量原值保留在來源與報告。Apple Watch 提供本批唯一有起訖時間的活動及基礎熱量增量，應依原區間保存。Garmin 的整趟數值無法還原這些區間的分布。[騎乘資訊流來源決策](2026-11-22-bryton-primary-streams.md)

## 替代方案

沿用 Bryton 摘要會在冷水坑個案帶入高值；按固定倍率縮小也會影響 10 月 28 日原本就與兩支手錶接近的數字。Apple Watch 的運動摘要可供同趟交叉核對。

## 後續工作

輸出規格仍須決定 Strava 發佈欄位採活動或總熱量，以及 Garmin 缺值或時段無法比對時的備援。實作時以冷水坑驗證兩種熱量的取值，並在合併報告保留採用來源、原值和修正值。
