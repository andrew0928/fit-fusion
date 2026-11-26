# M5 發佈計畫

**0.2.x 完成功能與驗收，v0.2.4 預發佈提前移除舊 `fit-fusion`，0.3.0 為首次正式發佈目標。** v0.2.3 已公開交付共通 YAML，並通過從 v0.2.2 升級的公開驗收。後續修正持續使用 0.2.x 的 patch 版號。[預發佈決策](../decisions/2026-11-26-v024-prerelease.md) · [里程碑總覽](../CONTEXT.md#roadmap)

| 任務 | 狀態（2026-11-26） | 驗收結果與剩餘工作 |
|---|---|---|
| M5-1 公開主 repo | 完成 | 使用者核准內容公開，已驗證匿名下載。[複核依據](../decisions/2026-11-26-public-source-and-release.md#活動檔公開範圍) |
| M5-2 自動建置 binary | 已取消 | 維護者手動建置、發佈主 repo Release，GitHub Actions 自動更新 tap 的 Formula。 |
| M5-3 Developer ID 簽章與公證 | 本版略過 | 依既有決定維持 Apple Silicon 本機執行檢查。 |
| M5-4 Homebrew 跨版本升級 | 完成，公開 0.2.2 → 0.2.3 驗收通過 | Release 自動更新兩個 Formula。已用公開 tap 在原有安裝執行升級、套件測試與 FIT 讀回，使用者 YAML 完整保留。 |
| M5-5 共通設定與操作指引 | 完成，使用者已通過 0.2.3 review | 三支 query 共用 YAML。已實測 `config init` 重跑、已編輯設定保留與 CLI 覆寫。2026-11-26 使用者確認設定操作及終端配色。[設定與升級指引](../config/README.md) · [介面決策](../decisions/2026-11-26-config-init-and-effective-arguments.md) |

## 0.2.4 預發佈

五支現行工具、檢查器 bridge、共通 YAML 範本及授權文件構成 Apple Silicon 套件。舊合併器的 Rust crate 與專屬測試已移除，12 份合成 FIT 的內容雜湊與 v0.2.3 相同。原始碼測試通過 20 項 Rust、33 項 CLI 整合及 11 項檢查器測試，涵蓋四語言 round-trip 與六趟實騎合併。另從獨立目錄解開套件，驗證五支 CLI 和 bridge，匯出兩份 FIT 後覆寫並讀回六筆心率。GitHub 穩定版與 Homebrew 公開 Formula 維持 v0.2.3。[預發佈決策](../decisions/2026-11-26-v024-prerelease.md)

| 發佈後驗收 | 結果 |
|---|---|
| 公開附件與原始碼 | 匿名下載 `v0.2.4` 的 binary、校驗檔及 GitHub 原始碼封存。binary SHA-256 為 `7dc00581841fd5516c65f8e98dc967be8c512842691923ba47add727be636cb6`，與本機驗收檔逐位元組相同。原始碼中的五份 Rust manifest 版本均為 0.2.4，已確認舊 crate 與舊 CLI 專屬測試均已移除。[GitHub pre-release](https://github.com/andrew0928/fit-fusion/releases/tag/v0.2.4) |
| 升級與讀回 | 在已安裝 0.2.3 的隔離 Homebrew，使用指向公開 pre-release 附件的本機測試 Formula，成功執行 `brew upgrade`、`brew test`。用已安裝的 CLI 再執行查詢、匯出及合併。獨立解碼的心率為 109、110、113、113、114、115 bpm。既有 YAML 內容、權限、inode 及修改時間均相同。 |
| 穩定通路 | GitHub 的 latest 與一般 Homebrew 安裝均維持 0.2.3。預發佈事件如期略過公開 tap 同步，tap 的 main 仍為 `d8b1a15`。[Actions 紀錄](https://github.com/andrew0928/fit-fusion/actions/runs/45766971626) |

清理 tap 時已刪除舊的 `fit-fusion-v0.2.2` Git 標籤。指定版本 Formula 仍可直接從主 repo 匿名下載；其 0.2.2 附件的 SHA-256 與 Formula 相符。[0.2.2 Formula](https://github.com/andrew0928/homebrew-tools/blob/d8b1a154b7cfa07544cd0ac8c1519c40c5278c48/Formula/fit-fusion%400.2.2.rb)

## 0.2.3 公開發佈與升級驗收

2026-11-26 發佈 `v0.2.3`，標籤指向 M5-5 提交 `a8f0f6f`。GitHub 的 latest Release 已指向此版。匿名下載的 macOS arm64 壓縮檔與校驗檔均與本機驗收版本逐位元組一致。壓縮檔 SHA-256 為 `0ebab57982a5dcf652c6396436937f61ef7972d682804c496338bc860d2d2814`。[GitHub Release](https://github.com/andrew0928/fit-fusion/releases/tag/v0.2.3)

| 驗收 | 觀察結果 |
|---|---|
| 公開 tap 同步 | GitHub Actions 由此次 Release 觸發並成功，tap 提交 `d8b1a15` 同時更新預設 Formula 及 `fit-fusion@0.2.3`。兩者的下載網址與 SHA-256 均已核對。[Actions 紀錄](https://github.com/andrew0928/fit-fusion/actions/runs/45762518616) · [tap 提交](https://github.com/andrew0928/homebrew-tools/commit/d8b1a154b7cfa07544cd0ac8c1519c40c5278c48) |
| 使用者既有安裝 | 在 Apple Silicon 的 `/opt/homebrew` 執行 `brew update` 發現 0.2.2 可升級，再執行 `brew upgrade andrew0928/tools/fit-fusion` 與 `brew test andrew0928/tools/fit-fusion`，兩者均通過。五支已連結工具均回報 0.2.3，執行檔與匿名下載的公開附件逐位元組一致。 |
| 使用者既有設定 | 升級前後的 `~/.fit-fusion/config.yaml` 內容、權限、inode 與修改時間一致。 |
| 安裝版端到端驗證 | 以獨立測試設定查詢及匯出兩份合成 FIT，確認匯出與原檔相同。覆寫心率後，以已安裝的檢查器與獨立解碼器讀回 109、110、113、113、114、115 bpm。 |

## 0.2.3 候選套件升級實測

- **起點**　Apple Silicon 的隔離 Homebrew 前綴，從公開 tap `9533579` 安裝正式 0.2.2。下載檔 SHA-256 為 `f9987dac5906e522d98eb843dd610012035758ce5aff356c8ba1ae70ef85d94a`。該版本的 `brew test` 通過。
- **更新**　在同一前綴與同一個 `andrew0928/tools/fit-fusion` 名稱下，用測試 tap 提供候選 Formula。它由正式產生器產生，測試下載網址指向本機附件。Homebrew 偵測 0.2.3 可升級，五支已連結的 CLI 均回報 0.2.3，候選版的 `brew test` 通過。
- **個人設定**　升級前先建立一份含自訂註解、UTC 及相對 FIT 目錄的 YAML。升級後用三支 query 分別執行 `config init`，都回報 `already_exists`。檔案的 SHA-256、0600 權限、inode 與 mtime 均一致。明確傳入與 YAML 同值的 `--timezone UTC` 時，來源由 `config` 變為 `cli`。
- **資料流程**　從 YAML 指定的位置查詢並匯出兩份合成 FIT，副本與來源 SHA-256 相同。再用安裝版 `fit-merge` 覆寫心率，以 `fit-inspector` 檢查，並獨立解碼讀回 109、110、113、113、114、115 bpm。
- **迴歸測試**　六個 Rust 套件合計 48 項測試通過。使用含 DuckDB 的 Python 測試環境執行 CLI 測試套件，52 項全數通過。完整輸出保存於同版候選附件目錄的 `rust-tests.log` 與 `cli-tests-all.log`。
- **候選附件**　`fit-fusion-v0.2.3-macos-arm64.tar.gz` 的 SHA-256 為 `0ebab57982a5dcf652c6396436937f61ef7972d682804c496338bc860d2d2814`。通用壓縮檔保留 `fit-fusion 0.1.0` 與 `fit-inspector-bridge`。Formula 安裝五支獨立 CLI、全註解範本與第三方授權文件。本機附件、校驗檔、安裝日誌及 `upgrade-result.json` 位於 `runtime/releases/v0.2.3/`。

## 接下來的版本

1. **0.2.x 完成其餘驗收**　選定專案授權並完成第三方再分發檢查。發現問題即在後續 patch 修正並複驗。維護者在主 repo 發佈一次，tap 只存 Formula。[發佈位置決策](../decisions/2026-11-26-public-source-and-release.md)
2. **0.3.0 首次正式發佈**　沿用 0.2.4 預發佈驗證過的五支工具、個人 YAML 及套件內容。既有的 Homebrew 安裝名稱 `andrew0928/tools/fit-fusion` 繼續使用。

此前的 0.2.2 乾淨安裝已通過 R14；歷史四版及預設 Formula 也已通過 `brew test`。2026-11-26 使用現有 Release 手動觸發同步，首次寫入 tap，第二次未新增提交。[R14 驗收](../spec/testcases/m4-cli.md#r14review) · [首次同步](https://github.com/andrew0928/fit-fusion/actions/runs/45747888794) · [重跑紀錄](https://github.com/andrew0928/fit-fusion/actions/runs/45747963685)
