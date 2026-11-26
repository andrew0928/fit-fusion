# M5 開發計畫：0.3.0 首次正式發佈

**目標**　從已公開的 `andrew0928/fit-fusion` 發佈 0.3.0。每版在主 repo 建立一次 GitHub Release，提供 `fit-merge`、`fit-inspector` 與三支 `fit-query-*` 的 macOS Apple Silicon 套件及 `SHA256SUMS`。發佈後自動更新 `andrew0928/homebrew-tools` 的最新版與各版本 Formula，下載位置指向主 repo。安裝名稱沿用 `andrew0928/tools/fit-fusion`。[發佈位置決策](../decisions/2026-11-26-public-source-and-release.md)

**驗收基準**　0.2.2 是 Homebrew 測試版。M4-7 的乾淨環境首次安裝已通過 review，M5-4 從已安裝的 0.2.2 驗證升級。[R14 案例與紀錄](../spec/testcases/m4-cli.md#r14review) · [發佈決策](../decisions/2026-11-25-first-stable-release.md)

| 任務 | 狀態 | 工作 | 完成條件 |
|---|---|---|---|
| M5-1 | 完成 | 主 repo 內容公開核准 | 2026-11-26 使用者確認目前 repo 的所有資訊均可公開，指定本項結案。已檢查的 Git 歷史含 12 份合成 FIT，無真實活動 FIT、Health XML 或 GPX。[複核範圍與依據](../decisions/2026-11-26-public-source-and-release.md#活動檔公開範圍) |
| M5-3 | 本版略過 | Apple Developer ID 簽章與公證 | 使用者目前沒有 Apple Developer 帳號，已決定 0.3.0 略過簽章、公證及對應的瀏覽器下載啟動驗收。 |
| M5-4 | 已確認，待辦 | Homebrew 跨版本升級 | 從已安裝的 0.2.2 在測試用 tap 分支升級。Formula 的 `url` 指向主 repo 的候選附件，`version` 與 `sha256` 對應該附件。執行 `brew update`、`brew upgrade andrew0928/tools/fit-fusion` 與 `brew test`，核對五支工具，並用安裝版合併與讀回合成 FIT。 |
| M5-5 | 設定格式已確認，待實作與驗收 | 共通設定與舊 CLI 退場 | 實作並驗收 [`~/.fit-fusion/config.yaml` 的共用與工具區段](../spec/cli-common.md)及 Garmin `--env-file` 規則，保留 `fit-merge` 免設定檔即可執行。交付安裝、升級及退場指引，以合成檔走完查詢、匯出、合併與檢查，並說明如何手動上傳 Strava。 |

**0.3.0 發佈流程**　原 M5-2 的自動建置已取消。發佈前確認專案授權與第三方再分發條件。依版本 tag 與 Cargo.lock 建置、測試五支 CLI，在主 repo 的 Release 草稿上傳 macOS arm64 套件及 `SHA256SUMS`，再發佈 Release。GitHub Actions 隨後更新 tap 中 Formula 的 `version`、`url` 與 `sha256`。發佈者可在主 repo 的 Actions 查看結果或重跑同步。

以 `v0.3.0` 為例，兩個附件須命名為 `fit-fusion-v0.3.0-macos-arm64.tar.gz` 與 `SHA256SUMS`。壓縮檔在同一層放置五支可執行 CLI、`THIRD-PARTY-NOTICES.txt`、`RUST-LIBRARY-COPYRIGHT.html`。執行 `shasum -a 256 fit-fusion-v0.3.0-macos-arm64.tar.gz > SHA256SUMS` 可產生校驗檔。同步涵蓋正式的 `vX.Y.Z`，最新版取最大的語意版本。附件缺漏時補齊附件，再從 Actions 手動執行同步。[工作流程](../../.github/workflows/sync-homebrew.yml) · [Formula 產生器](../../scripts/sync-fit-fusion.py)

**完成里程碑的條件**　使用者能匿名取得主 repo 的 `v0.3.0` 原始碼與 Release binary。tap 對該版只有 Formula 更新。從 Formula 的 URL 匿名下載套件，確認 SHA-256 與主 repo 的 `SHA256SUMS` 相符。保留來源 commit 及升級驗收紀錄。
