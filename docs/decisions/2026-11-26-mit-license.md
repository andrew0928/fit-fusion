# FIT Fusion 原創程式碼與使用文件採 MIT

## 狀態

`accepted`。2026-11-26，維護者以個人技術分享為目的，確認採用 MIT License。

## 背景

FIT Fusion 透過 GitHub 公開原始碼及 macOS 執行檔，也經由 Homebrew 安裝。維護者希望他人能自由研究、修改、散布與商用。

## 決策

以根目錄的 [LICENSE](../../LICENSE) 授權專案原創程式碼與隨附使用文件，著作權聲明為 `Copyright (c) 2026 Andrew Wu`。第三方 SDK、函式庫及其資料沿用各自授權。[MIT 原文](https://opensource.org/license/mit)

## 影響

v0.2.4 預發佈後的新版本在 macOS 壓縮檔內附上同一份 `LICENSE`。Homebrew 將它安裝在套件的共用資料目錄，與第三方聲明並列。v0.2.4 及更早版本的附件與 SHA-256 沿用原發佈紀錄。

## 替代方案

曾考慮含明文專利授權的 Apache-2.0。維護者選擇條款較短的 MIT。

## 後續工作

首次正式發佈前逐項核對第三方元件的再分發條件與隨附聲明。[M5 發佈計畫](../milestones/m5-release.md)
