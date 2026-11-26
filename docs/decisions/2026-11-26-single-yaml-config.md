# M5-5 使用單一 YAML，區分共用與工具設定

## 狀態

`accepted`。2026-11-26 使用者確認使用單一 YAML，並要求明確定義共用區段及個別工具區段。本決策取代[依工具分檔提案](2026-11-25-query-config-layout.md)。

2026-11-26 後續決策將共通設定與跨版本升級提前至 0.2.3 驗收；所有功能驗收在 0.2.x 完成。[版本範圍修訂](2026-11-26-v023-upgrade-and-retirement.md)

## 背景

三支查詢工具都依出發日查詢騎乘，各自使用不同的輸入來源。M5-5 已選定個人設定目錄 `~/.fit-fusion/`。

## 決策

設定檔為 `~/.fit-fusion/config.yaml`。在 `common` 宣告共用值，在 `tools.<完整執行檔名>` 宣告工具專屬值。各欄位、預設與覆寫規則見[共同 CLI 規格](../spec/cli-common.md)。

使用者進一步確認，這項決定適用於 0.3.0 的新工具。三支查詢工具只從 `config.yaml` 讀取個人設定；`config.json` 不列入設定搜尋或回退順序。`fit-merge` 與 `fit-inspector` 以命令列參數執行。0.2.x 舊 `fit-fusion` 的 JSON 契約保留為歷史規格。英文註解的[範例](../config/config.example.yaml)與[格式定義](../config/query-config-v1.schema.yaml)均以 YAML 保存。

## 影響

時區只需設定一次，來源路徑分別由各 query 讀取。Garmin 憑證沿用 dotenv，YAML 保存其路徑。合併與檢查工具可直接以顯式參數執行。

## 替代方案

歷次比較包含每支工具一份 YAML 與整套工具共用一份 JSON。使用者選定集中維護，並以 YAML 撰寫。

## 後續工作

M5-5 開發版已完成設定載入、命令列覆寫與離線流程的本機驗收。[初始化與執行前參數來源](2026-11-26-config-init-and-effective-arguments.md)
