# 開源主 repo，統一由主 repo 發佈 binary

## 狀態

`accepted`。2026-11-26 使用者確認在 0.3.0 發佈前開源 FIT Fusion，binary 由同一個 repo 的 GitHub Release 提供，Homebrew tap 僅維護 Formula。這項決定取代[0.2.2 測試分發](2026-11-25-homebrew-cli.md)與[原 M5 計畫](2026-11-25-first-stable-release.md)中的附件存放方式。

## 背景

0.2.2 測試時主 repo 為私有，公開 tap 另行提供匿名下載的套件。使用者希望每次發版只上傳一次。

## 決策

原始碼、版本 tag、GitHub Release、macOS arm64 壓縮檔與校驗檔統一在 `andrew0928/fit-fusion`。維護者在主 repo 上傳附件和校驗檔後發佈 Release。主 repo 的 GitHub Actions 讀取已公開的正式版本，產生最新版 `fit-fusion.rb` 與各版 `fit-fusion@X.Y.Z.rb`，將變動推至 `andrew0928/homebrew-tools`。每份 Formula 固定下載網址、版本與 SHA-256，安裝時可重現。

v0.2.2 曾分別提供開發者套件與五支工具的 Homebrew 套件。遷移時將 Homebrew 套件原樣附加於主 repo 同一個 Release，沿用既有安裝檔的 SHA-256。v0.1.0、v0.2.0 與 v0.2.1 直接使用各自的原始 Release 套件，Formula 只安裝該版已提供的獨立 CLI。[主 repo 發行版本](https://github.com/andrew0928/fit-fusion/releases) · [各版 Formula](https://github.com/andrew0928/homebrew-tools/tree/main/Formula)

### 指名安裝預發佈版

2026-11-26 使用者選定 v0.2.5 作為 GitHub 預發佈版，並要求能以 `fit-fusion@0.2.5` 指名安裝。預設 Formula 仍取最高的正式版，此次為 0.2.3。要將預發佈加入 tap，維護者在 GitHub Release 說明內加上一行 `<!-- homebrew: versioned-only -->`。同步流程為該版產生 keg-only Formula，日後重跑時也會保留。v0.2.4 沿用原先的 GitHub 下載通路。

### 活動檔公開範圍

2026-11-26 使用者確認，活動資料的判準是一般 `git clone` 能否直接取得真實的活動 FIT、健康 XML、GPX 或封裝在檔案裡的原始軌跡。可取得的分支與標籤歷史也在範圍內。合成測試檔、冷水坑研究的五個抽樣座標與地圖連結、Garmin 活動 ID，以及文件中指向本機 `data/` 等目錄的路徑文字，均可公開。間接線索免另行複核。

以 `cbbbfe2` 及 `v0.1.0` 至 `v0.2.2` 的可達歷史驗收，共 17 筆提交。12 份 FIT 均由 [C# 測試檔產生器](../../src/csharp/FitFixtures/Program.cs) 逐位元組重建一致，XML 與 GPX 檔案數均為 0。真實活動來源保存在 Git 忽略的本機目錄。

## 影響

2026-11-26 使用者核准公開內容並完成 M5-1，主 repo 已改為 public。原 M5-2 取消，建置及發佈 Release 仍採手動。同日使用者進一步指定從主 repo 發佈一次即可更新 Homebrew，據此啟用跨 repo 的 Formula 同步。M5-4 將從 0.2.2 驗證升級，安裝名稱沿用 `brew install andrew0928/tools/fit-fusion`。

## 替代方案

沿用 0.2.2 的分發方式需要在 tap 維護另一份 binary 與附件連結。

## 後續工作

0.3.0 正式發佈前，選定專案開源授權、核對第三方再分發條件，並從 0.2.2 完成升級驗收。[M5 驗收條件](../milestones/m5-release.md)
