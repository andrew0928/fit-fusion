# 整趟騎乘的熱量消耗優先使用 Garmin

## 狀態

accepted。2026-11-23 使用者指定「熱量消耗以 garmin 為優先」，並要求標入既有的設備優先順序對照表。[三裝置對照表](../specification.md)

同日討論 M2 時，使用者確認用 CIRQA 的數值發佈，並指定除覆寫 `total_calories` 外，也要保留 Garmin 的兩個原始數值。

## 背景

11 月 1 日冷水坑騎乘，Bryton 原始 FIT 和 Strava 同步至 Apple 健康的熱量都是 6,186 kcal。Garmin 同趟的總熱量為 1,897 kcal，活動熱量為 1,514 kcal。另有車友報告其他 Bryton 型號偏高，Rider 460 的具體成因仍待釐清。[六趟實檔與公開資料](../research/2026-11-23-strava-ride-calories.md)

## 決策

已配對同趟騎乘，且熱量定義和時段可比較時，整趟熱量摘要以 Garmin CIRQA 為第一來源。保留總熱量與活動熱量兩個值。本批六份 Garmin FIT 可讀取 `session.total_calories` 與 `session.metabolic_calories`，兩者相減皆與 Connect 寫入 Apple 健康的活動熱量相符。

發佈時由同趟 CIRQA 的 `session.total_calories` 覆寫合併 FIT 的同名標準欄位，並在合併 FIT 內另外保留 CIRQA 兩個原始欄位的值。Garmin 對單次記錄的 **Activity Calories** 定義為該段的活動消耗加靜息消耗。以下以「活動熱量」指 **Active Calories**，兩個英文詞在此有不同含義。[Garmin 熱量用語](https://support.garmin.com/en-CA/?faq=lkl4cwCLlK7ox362uGQEV7) · [Garmin FIT Profile](https://github.com/garmin/fit-python-sdk/blob/main/garmin_fit_sdk/profile.py)

下表是 M2 的欄位映射提案。冷水坑兩份原始 FIT 已用本專案讀取器核對。標準 session 訊息編號為 18。

| 合併 FIT 的 session 欄位 | 冷水坑 kcal | 來源與用途 |
|---|---:|---|
| 標準 `total_calories`（欄位 11） | **1,897** | 發佈給 Strava。Bryton 與 Garmin 原檔都有同名同號的欄位，原值分別為 6,186、1,897。 |
| 標準 `metabolic_calories`（欄位 196） | 383 | 選同趟 Garmin 同名欄位。本趟 Bryton 沒有欄位 196。Strava 的 FIT 表未列此欄。 |
| CIRQA 來源副本的 `total_calories` | 1,897 | 保留 Garmin 原始名稱、欄位 11 及數值，供單憑合併 FIT 查回。 |
| CIRQA 來源副本的 `metabolic_calories` | 383 | 保留 Garmin 原始名稱、欄位 196 及數值。 |

M2 僅挑選同名欄位。為另外保留原值，兩個來源副本使用 FIT Fusion 專為 CIRQA 來源配置的 application ID，在 session 的 developer fields 中沿用原欄名、原欄號及 kcal 單位。寫入時配置未占用的 developer index 並附欄位定義。讀回以 application ID、message 與欄號辨識，所以同名的標準欄位和來源副本可各自取回。寫入有效原值，來源缺失時標缺。活動熱量 1,514 kcal 由 1,897 − 383 得出，只在報告與分析輸出另列，並保存 Bryton 原值。只有總量與代謝量皆有效且代謝量不超過總量才計算差值。[Garmin developer fields](https://developer.garmin.com/fit/cookbook/developer-data/)

## 影響

Bryton 提供主時間軸，其熱量原值保留在來源與報告。Apple Watch 提供本批唯一有起訖時間的活動及基礎熱量增量，應依原區間保存。Garmin 的整趟數值無法還原這些區間的分布。[騎乘資訊流來源決策](2026-11-22-bryton-primary-streams.md)

## 替代方案

沿用 Bryton 摘要會在冷水坑個案帶入高值；按固定倍率縮小也會影響 10 月 28 日原本就與兩支手錶接近的數字。Apple Watch 的運動摘要可供同趟交叉核對。

## 後續工作

用獨立 FIT 讀取器分別讀回冷水坑的標準欄位及 CIRQA 來源副本，兩組都應為 1,897、383。報告中的活動熱量應為 1,514，Bryton 原值為 6,186。Strava 官方列出會讀取標準 `session.total_calories`，來源副本在本機 FIT 驗收。另用新的授權騎乘讀回 Strava 活動明細的 `calories`。Garmin 總量缺值或時段不可比時的備援來源尚待決定，M2 提案先標為待判讀。[Strava FIT 匯入表](https://developers.strava.com/docs/uploads/) · [M2 驗收](../milestones/m2-cli.md)
