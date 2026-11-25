# M5 開發計畫：0.3.0 首次正式發佈

**目標**　在公開的 `andrew0928/homebrew-tools` 發佈五支 macOS Apple Silicon CLI 的 0.3.0 正式套件。五支工具為 `fit-merge`、`fit-inspector`、`fit-query-files`、`fit-query-apple`、`fit-query-garmin`。Homebrew 套件名稱沿用 `andrew0928/tools/fit-fusion`。

**驗收基準**　0.2.2 是 Homebrew 測試版。M4-7 的乾淨環境首次安裝已通過 review，M5-4 從已安裝的 0.2.2 驗證升級。[R14 案例與紀錄](../spec/testcases/m4-cli.md#r14review) · [發佈決策](../decisions/2026-11-25-first-stable-release.md)

| 任務 | 狀態 | 工作 | 完成條件 |
|---|---|---|---|
| M5-1 | 已確認，待辦 | 發佈來源與公開內容 | 將 Formula、打包所需來源及去識別化範例納入版控。檢查執行檔、內嵌 help／skill、範例、附件及 Release 說明，排除憑證、私人活動及健康資料。核對第三方授權與再分發條件。匿名下載候選附件，確認 SHA-256 與 Formula 和校驗檔一致。 |
| M5-2 | 待確認費用方案 | GitHub Actions 建置 | 提案是在私有開發庫的標準 macOS arm64 runner 依 tag 與 Cargo.lock 建置五支 CLI，跑既有單元及 CLI 驗收，記錄來源 commit、產物與結果，再交付公開 tap。方案依帳號可用額度確認。[GitHub 計費](https://docs.github.com/en/billing/concepts/product-billing/github-actions) |
| M5-3 | 本版略過 | Apple Developer ID 簽章與公證 | 使用者目前沒有 Apple Developer 帳號，已決定 0.3.0 略過簽章、公證及對應的瀏覽器下載啟動驗收。 |
| M5-4 | 已確認，待辦 | Homebrew 跨版本升級 | 在測試用 tap 分支，從已安裝的 0.2.2 執行 `brew update`、`brew upgrade andrew0928/tools/fit-fusion` 升至候選版。核對五支工具，通過 `brew test`，以合成 FIT 合併並用安裝版檢查器讀回。 |
| M5-5 | 已確認，待辦 | 共通設定與舊 CLI 退場 | 定義並驗收 `~/.fit-fusion/` 與 Garmin `--env-file` 的取用規則，保留 `fit-merge` 免設定檔即可執行。交付安裝、升級及退場指引，以合成檔走完查詢、匯出、合併與檢查，並說明如何手動上傳 Strava。 |

**完成里程碑的條件**　候選版演練保留來源 commit、附件 SHA-256 與驗收結果。正式發佈時核對 `v0.3.0` 來源 tag、公開 GitHub Release 附件與 tap 的 Formula；匿名下載取得相符的 0.3.0 套件。
