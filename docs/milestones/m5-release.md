# M5 開發計畫：0.3.0 首次正式發佈

**目標**　從已公開的 `andrew0928/fit-fusion` 發佈 0.3.0。每版在主 repo 建立一次 GitHub Release，提供 `fit-merge`、`fit-inspector` 與三支 `fit-query-*` 的 macOS Apple Silicon 套件及 `SHA256SUMS`。發佈後自動更新 `andrew0928/homebrew-tools` 的最新版與各版本 Formula，下載位置指向主 repo。安裝名稱沿用 `andrew0928/tools/fit-fusion`。[發佈位置決策](../decisions/2026-11-26-public-source-and-release.md)

**目前進度（2026-11-26）**　最新發佈版本為 0.2.2，M4-7 的乾淨環境首次安裝已通過 R14。下一個開發任務是 **M5-5，實作三支查詢工具共用的 YAML 載入器**。0.3.0 候選套件備妥後，再執行 M5-4 跨版本升級驗收。[里程碑總覽](../CONTEXT.md#roadmap) · [R14 案例與紀錄](../spec/testcases/m4-cli.md#r14review)

| 任務 | 狀態 | 工作 | 完成條件 |
|---|---|---|---|
| M5-1 | 完成 | 主 repo 內容公開核准 | 2026-11-26 使用者確認目前 repo 的所有資訊均可公開，指定本項結案。已檢查的 Git 歷史含 12 份合成 FIT，無真實活動 FIT、Health XML 或 GPX。[複核範圍與依據](../decisions/2026-11-26-public-source-and-release.md#活動檔公開範圍) |
| M5-2 | 已取消 | 自動建置及發佈 binary | 維護者手動建置並發佈主 repo Release。發佈後的 Formula 同步已另行交付。 |
| M5-3 | 本版略過 | Apple Developer ID 簽章與公證 | 使用者目前沒有 Apple Developer 帳號，已決定 0.3.0 略過簽章、公證及對應的瀏覽器下載啟動驗收。 |
| M5-4 | 已確認，待辦 | Homebrew 跨版本升級 | 從已安裝的 0.2.2 在測試用 tap 分支升級。Formula 的 `url` 指向主 repo 的候選附件，`version` 與 `sha256` 對應該附件。執行 `brew update`、`brew upgrade andrew0928/tools/fit-fusion` 與 `brew test`，核對五支工具，並用安裝版合併與讀回合成 FIT。 |
| M5-5 | 規格完成，下一項開發 | 共通設定與舊 CLI 退場 | 三支查詢工具支援選填的 `~/.fit-fusion/config.yaml`。`fit-merge` 與 `fit-inspector` 繼續直接用參數執行。交付安裝、升級及舊 CLI 退場指引。[設定契約與 CLI 驗收情境](../spec/cli-common.md) |

## 已驗證的發佈流程

主 repo 提供 v0.1.0、v0.2.0、v0.2.1、v0.2.2 的匿名下載，tap 的舊 Release 已移除。在 Apple Silicon 上，預設 Formula 及四份指定版本的 Formula 均通過安裝與 `brew test`。用現有版本手動觸發同步兩次，第一次更新 tap，第二次核對後未產生新提交。首次由 `release.published` 觸發的驗證留到下一個正式版本。[主 repo Releases](https://github.com/andrew0928/fit-fusion/releases) · [首次同步](https://github.com/andrew0928/fit-fusion/actions/runs/45747888794) · [重跑紀錄](https://github.com/andrew0928/fit-fusion/actions/runs/45747963685)

## 接下來的執行順序

1. **M5-5，共用設定載入。** YAML 格式、英文註解範例與完整 schema 已備妥，8 個 schema 案例通過。接下來在 Rust 實作讀取與驗證，逐欄套用「明確 CLI 參數 → YAML → 內建預設」。0.3.0 的新工具只支援 YAML，0.2.x 的 JSON 契約留作歷史。[設定範例](../config/config.example.yaml) · [完整 schema](../config/query-config-v1.schema.yaml)
2. **M5-5，工具整合與退場驗收。** 將載入器接到 files、apple、garmin，驗證不同工作目錄、缺檔、錯誤 YAML、Garmin dotenv 與離線快取。用合成資料走完查詢、匯出、合併與檢查，更新安裝指引及舊 CLI 對照，說明手動上傳 Strava 的最後一步。[CLI 驗收情境](../spec/cli-common.md)
3. **M5-4，候選版升級。** 在測試用 tap 從已安裝的 0.2.2 升級到 0.3.0 候選套件，執行上表的 `brew test` 與 FIT 合併、讀回驗收，留下版本與校驗紀錄。
4. **發佈 v0.3.0。** 定案專案授權並核對第三方再分發條件後，依下方流程發佈，確認 Release 事件成功更新 tap，並從 Formula 的 URL 匿名核對套件 SHA-256。

## 建置與發佈

**0.3.0 發佈流程**　原 M5-2 的自動建置已取消。發佈前確認專案授權與第三方再分發條件。依版本 tag 與 Cargo.lock 建置、測試五支 CLI，在主 repo 的 Release 草稿上傳 macOS arm64 套件及 `SHA256SUMS`，再發佈 Release。GitHub Actions 隨後更新 tap 中 Formula 的 `version`、`url` 與 `sha256`。發佈者可在主 repo 的 Actions 查看結果或重跑同步。

以 `v0.3.0` 為例，兩個附件須命名為 `fit-fusion-v0.3.0-macos-arm64.tar.gz` 與 `SHA256SUMS`。壓縮檔在同一層放置五支可執行 CLI、`THIRD-PARTY-NOTICES.txt`、`RUST-LIBRARY-COPYRIGHT.html`。執行 `shasum -a 256 fit-fusion-v0.3.0-macos-arm64.tar.gz > SHA256SUMS` 可產生校驗檔。同步涵蓋正式的 `vX.Y.Z`，最新版取最大的語意版本。附件缺漏時補齊附件，再從 Actions 手動執行同步。[工作流程](../../.github/workflows/sync-homebrew.yml) · [Formula 產生器](../../scripts/sync-fit-fusion.py)

**完成里程碑的條件**　使用者能匿名取得主 repo 的 `v0.3.0` 原始碼與 Release binary。tap 對該版只有 Formula 更新。從 Formula 的 URL 匿名下載套件，確認 SHA-256 與主 repo 的 `SHA256SUMS` 相符。保留來源 commit 及升級驗收紀錄。
