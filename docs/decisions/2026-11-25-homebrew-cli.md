# Homebrew 安裝五支 Apple Silicon CLI

## 狀態

`accepted`。2026-11-25 使用者指定 Brew 僅安裝雙檔合併器、檢查器與各查詢工具，平台限定 macOS Apple Silicon，並確認建立獨立的公開 Homebrew tap。使用者選定名稱 `tools`，供個人工具與 demo 共用。

## 背景

v0.2.2 的 macOS arm64 壓縮檔同時包含獨立工具、舊合併器與多語言檢查器的輔助程式。

## 決策

Homebrew 套件沿用專案名 `fit-fusion`。安裝清單為 `fit-merge`、`fit-inspector`、`fit-query-files`、`fit-query-apple`、`fit-query-garmin`。Formula 指向特定版本的 Release 附件並固定 SHA-256。[套件定義](../../Formula/fit-fusion.rb)

## 影響

一次安裝與升級整組 CLI。套件版本取自 Release，各執行檔的 `--version` 顯示其元件版本。首次安裝測試時，公開 tap 保存 Formula、README 及版本附件，當時的開發庫為私有。公開套件使用通用的 Garmin 活動 ID 與 Apple Watch 來源範例。[0.2.2 套件現址](https://github.com/andrew0928/fit-fusion/releases/tag/v0.2.2)

## 替代方案

將整個壓縮檔安裝到 `bin` 會多安裝舊版 `fit-fusion` 與 `fit-inspector-bridge`。逐一列名可以固定每版安裝範圍。

## 後續工作

2026-11-26 已將 0.2.2 的 Homebrew 套件原樣移至主 repo 的同版 Release，並決定在主 repo 發佈後自動更新 tap。[主 repo 發佈決策](2026-11-26-public-source-and-release.md)

本機已在隔離的 Homebrew prefix 使用 v0.2.2 下載附件的快取通過 `brew install` 與 `brew test`。實際安裝的五支 CLI 均能執行，另以兩份合成 FIT 完成心率覆寫並由安裝版檢查器讀回 6 筆有效樣本。

公開版另用匿名 HTTP GET 讀回壓縮檔及校驗檔，下載內容符合 Formula 的 SHA-256。從全新 tap 註冊直接執行 `brew install andrew0928/tools/fit-fusion` 後，再以系統安裝的兩支 CLI 合併並讀回 6 筆心率。使用者已完成乾淨環境 R14 review。0.2.x 的 Homebrew 套件供測試，跨版本升級列入 0.3.0 正式發佈前清單。[R14 驗收紀錄](../spec/testcases/m4-cli.md#r14review) · [發佈定位決策](2026-11-25-first-stable-release.md)
