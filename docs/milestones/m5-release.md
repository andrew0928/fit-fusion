# M5 發佈計畫

**0.2.x 完成功能與驗收，0.3.0 只移除舊 `fit-fusion`。** 0.2.3 是第一個交付共通 YAML 並驗證跨版升級的候選套件。後續修正持續使用 0.2.x 的 patch 版號。[版本範圍決策](../decisions/2026-11-26-v023-upgrade-and-retirement.md) · [里程碑總覽](../CONTEXT.md#roadmap)

| 任務 | 狀態（2026-11-26） | 驗收結果與剩餘工作 |
|---|---|---|
| M5-1 公開主 repo | 完成 | 使用者核准內容公開，已驗證匿名下載。[複核依據](../decisions/2026-11-26-public-source-and-release.md#活動檔公開範圍) |
| M5-2 自動建置 binary | 已取消 | 維護者手動建置、發佈主 repo Release，GitHub Actions 自動更新 tap 的 Formula。 |
| M5-3 Developer ID 簽章與公證 | 本版略過 | 依既有決定維持 Apple Silicon 本機執行檢查。 |
| M5-4 Homebrew 跨版本升級 | 0.2.2 → 0.2.3 本機候選驗收通過 | 已實際執行 `brew update`、`brew upgrade`、`brew test`。0.2.3 公開發佈後再核對 Release 事件、公開 tap 與匿名下載。 |
| M5-5 共通設定與操作指引 | 完成，使用者已通過 0.2.3 review | 三支 query 共用 YAML。已實測 `config init` 重跑、已編輯設定保留與 CLI 覆寫。2026-11-26 使用者確認設定操作及終端配色。[設定與升級指引](../config/README.md) · [介面決策](../decisions/2026-11-26-config-init-and-effective-arguments.md) |

## 0.2.3 升級實測

- **起點**　Apple Silicon 的隔離 Homebrew 前綴，從公開 tap `9533579` 安裝正式 0.2.2。下載檔 SHA-256 為 `f9987dac5906e522d98eb843dd610012035758ce5aff356c8ba1ae70ef85d94a`。該版本的 `brew test` 通過。
- **更新**　在同一前綴與同一個 `andrew0928/tools/fit-fusion` 名稱下，用測試 tap 提供候選 Formula。它由正式產生器產生，測試下載網址指向本機附件。Homebrew 偵測 0.2.3 可升級，五支已連結的 CLI 均回報 0.2.3，候選版的 `brew test` 通過。
- **個人設定**　升級前先建立一份含自訂註解、UTC 及相對 FIT 目錄的 YAML。升級後用三支 query 分別執行 `config init`，都回報 `already_exists`。檔案的 SHA-256、0600 權限、inode 與 mtime 均一致。明確傳入與 YAML 同值的 `--timezone UTC` 時，來源由 `config` 變為 `cli`。
- **資料流程**　從 YAML 指定的位置查詢並匯出兩份合成 FIT，副本與來源 SHA-256 相同。再用安裝版 `fit-merge` 覆寫心率，以 `fit-inspector` 檢查，並獨立解碼讀回 109、110、113、113、114、115 bpm。
- **迴歸測試**　六個 Rust 套件合計 48 項測試通過。使用含 DuckDB 的 Python 測試環境執行 CLI 測試套件，52 項全數通過。完整輸出保存於同版候選附件目錄的 `rust-tests.log` 與 `cli-tests-all.log`。
- **候選附件**　`fit-fusion-v0.2.3-macos-arm64.tar.gz` 的 SHA-256 為 `0ebab57982a5dcf652c6396436937f61ef7972d682804c496338bc860d2d2814`。通用壓縮檔保留 `fit-fusion 0.1.0` 與 `fit-inspector-bridge`。Formula 安裝五支獨立 CLI、全註解範本與第三方授權文件。本機附件、校驗檔、安裝日誌及 `upgrade-result.json` 位於 `runtime/releases/v0.2.3/`。

## 接下來的版本

1. **0.2.3 發佈**　在主 repo 的 `v0.2.3` Release 上傳候選附件與同批 `SHA256SUMS`。核對 `release.published` 的工作流程執行結果、最新版與指定版本 Formula、匿名下載 SHA-256。[工作流程](../../.github/workflows/sync-homebrew.yml) · [Formula 產生器](../../scripts/sync-fit-fusion.py)
2. **0.2.x 完成其餘驗收**　選定專案授權並完成第三方再分發檢查。發現問題即在後續 patch 修正並複驗。維護者在主 repo 發佈一次，tap 只存 Formula。[發佈位置決策](../decisions/2026-11-26-public-source-and-release.md)
3. **0.3.0 移除舊 CLI**　從現行工具與通用發行附件移除 `fit-fusion`，以已驗收的 0.2.x 為基準核對五支工具與個人 YAML。既有的 Homebrew 安裝名稱 `andrew0928/tools/fit-fusion` 繼續使用。

此前的 0.2.2 乾淨安裝已通過 R14；歷史四版及預設 Formula 也已通過 `brew test`。2026-11-26 使用現有 Release 手動觸發同步，首次寫入 tap，第二次未新增提交。[R14 驗收](../spec/testcases/m4-cli.md#r14review) · [首次同步](https://github.com/andrew0928/fit-fusion/actions/runs/45747888794) · [重跑紀錄](https://github.com/andrew0928/fit-fusion/actions/runs/45747963685)
