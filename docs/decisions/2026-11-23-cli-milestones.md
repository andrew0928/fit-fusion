# M2 交付可供 agent 操作的 macOS 融合 CLI，M3 處理安全保管與發佈

## 狀態

2026-11-25 起，M3 與 M2-4 的排程已由新決策取代。其餘 M2 已交付內容仍適用。[變更範圍](2026-11-25-cli-split.md)

accepted（里程碑範圍）。2026-11-23 使用者指定 M2 優先打通融合、FIT 輸出與 Strava 發佈，並將安全保管及正式發佈通路列入 M3。具體命令、設定結構與驗收切分見 [M2 設計提案](../milestones/m2-cli.md)。

2026-11-24 [本版發佈方式決策](2026-11-24-manual-strava-export.md)調整 Strava 的交付時程，原設計留存供後續開發。

2026-11-24 使用者確認人用 help 直接顯示 CLI 版本，`--version` 提供進階資訊。合併器與四種檢查器均適用。

## 背景

M1 與 M1-1 已能用兩個獨立讀取器檢查原檔。騎士每趟同時記錄 Bryton 與 Garmin，希望一次交付活動，由 agent 依已知的來源與資訊流順位完成合併及發佈。

## 決策

- M2 使用 Rust、RustyFIT 與 macOS。提供給人閱讀的 `--help` 與給 agent 操作的 `--skill <topic>`，優先完成可執行的日常流程。
- 個人 Garmin 存取資料及 Strava app ID、secret、token 集中在本機設定。M2 可以使用檔案。按既有選源規則挑選同名欄位、輸出合併 FIT，並支援上傳 Strava。未來版本要能指定跨欄位的 rename。[技術選型](2026-11-21-production-fit-sdk.md) · [騎乘欄位與順位](../specification.md#strava-fit-逐欄選源表)
- M3 記錄 macOS 優先的 Keychain／安全硬體研究、正式合規的發佈通路與其他整合。正式發佈方案待研究後選定。

## 影響

Rust 的 CLI 版本取自各 crate 的 `Cargo.toml`，Cargo 編譯時嵌入。Node.js 檢查器讀取 `package.json`，Python 讀取套件的 `__version__`，C# 讀取由專案 `Version` 產生的組件資訊。Rust 編譯器與目標在建置時記錄，Python、Node.js 與 .NET 的執行環境在呼叫時查詢。[合併器規格](../spec/merger.md) · [檢查器規格](../spec/inspector.md)

使用者可先只取得 FIT，完成本機驗證後再發佈。Agent 指引須能說明輸入、產物、授權、是否已建立遠端活動及重試方式。原檔唯讀、完整分析與來源追溯仍屬交付要求。

## 替代方案

先建立發佈通路或跨平台憑證管理，會推遲首次用自己的騎乘驗證端到端結果。M2 先用 macOS 上從原始碼建置的 CLI。

## 後續工作

依設計提案用六趟成對實檔驗證融合，另以一次授權的新騎乘核對 Strava 活動 ID 與匯入值。M3 各項先列研究問題與驗證方法。
