# Apple 健康如何漸進匯出騎乘資料

查核日期為 2026-11-23。目的為每趟騎乘後讓 Mac 自動取得新增的 Apple Watch 資料，避免每次重做超過 100 MB 的全量匯出。以下費用採查核當日的台灣 App Store 標價，尚待以個人裝置驗證到檔時間及取樣完整度。

fit-fusion 需要 Watch Series 9 每趟騎乘的時間、逐點心率及路線。現有全量匯出已足以作為三趟歷史樣本的比對基準。同一個健康資料庫也含 Garmin Connect 寫入的騎乘，驗證時要辨認記錄來源。[三趟樣本與取樣範圍](rides/2026-11-three-device-comparison.md) · [四平台比較](2026-11-23-activity-platform-automation.md)

## HealthFit 逐趟匯出 FIT

由 iPhone 上的使用者授權 HealthFit 讀取「體能訓練」、「體能訓練路線」、心率及需要的騎乘指標，選 FIT 並開啟運動自動同步。作者明列 Google Drive 為自動匯出目的地，授權範圍為 `drive.file`。Mac 端的 CLI 可直接讀取同步資料夾中的新檔案，無須申請 HealthFit API key 或 Apple Developer Program 會員。若希望沿用 iCloud Drive，GPXManagement 的 Mac 匯入說明也記載 HealthFit 可自動放入每趟 FIT，實際資料夾以 iPhone 上設定為準。[HealthFit 作者說明](https://altifondo.com/) · [資料與權限](https://altifondo.com/privacy-google/healthfit-privacy.html) · [iCloud Drive 整合說明](https://gpxmanagement.net/aide/apple-sante/)

查核當日台灣 App Store 售價為 NT$220，另列 Supporter 內購。作者將背景同步列為 App 功能，公開頁面未逐項列出各同步目的地的付費門檻。僅憑商店標價，尚無法確認目前版本每個目的地都包含在基本購買內。[台灣 App Store](https://apps.apple.com/tw/app/healthfit/id1202650514)

## 背景匯出的時間與範圍

每趟會取得由 HealthKit 資料轉製的運動檔。睡眠、全天心率等資料須另選匯出範圍。iPhone 鎖定時，背景 App 可能無法讀取健康資料。同一活動的路線也可能比運動紀錄晚寫入或後續更新，驗收應檢查最終到檔時間及晚到樣本。[Apple 健康資料保護](https://developer.apple.com/documentation/healthkit/protecting-user-privacy) · [路線更新時序](https://developer.apple.com/documentation/healthkit/reading-route-data)

## 不同資料需求與個人使用門檻

| 方案 | 匯出方式 | 個人使用門檻 |
|---|---|---|
| iPhone 內建捷徑 | 以 Watch 騎乘結束或每日定時觸發「尋找健康樣本」，按日期、來源取心率，存成 JSON 或 CSV 至 iCloud Drive。可每天重抓近期一小段時間，Mac 依來源、時間與數值去重，補到較晚同步的樣本。此流程涵蓋所選指標，完整騎乘 FIT 與 GPS 路線另需工具。 | 免費，首次允許捷徑讀取指定健康資料及寫入檔案。[Apple 觸發條件](https://support.apple.com/guide/shortcuts/apd932ff833f/ios) · [健康樣本篩選](https://matthewcassinelli.com/actions/find-health-samples/) · [心率匯出範例](https://matthewcassinelli.com/shortcuts/export-heart-rate-data/) |
| Health Auto Export | 需要 HTTP 時，REST API 自動化可選 `Since Last Sync`，由 iPhone 推送給自己的接收端。若選 iCloud Drive，則依日等期間分檔。騎乘 JSON 可選心率及路線；健康指標可另外選擇。 | 背景自動化屬 Premium，台灣列有 NT$790 終身買斷。使用者無須開發者帳號。[自動化設定](https://help.healthyapps.dev/en/health-auto-export/automations/) · [iCloud 的分檔及路線設定](https://help.healthyapps.dev/en/health-auto-export/automations/icloud-drive/) · [台灣價格](https://apps.apple.com/tw/app/health-auto-export-json-csv/id1115567069) |
| 自寫 iPhone HealthKit App | 用 `HKObserverQuery` 取得變更通知，按資料類型保存 `HKAnchoredObjectQuery` 的 anchor，下次只讀新增及刪除；路線獨立追蹤。輸出成功後才更新 anchor。 | Apple 列出免費帳號可用 HealthKit。Mac 上的 Xcode 可用免費 Personal Team 部署到自己的 iPhone，但安裝描述檔 7 天到期，須重建並重裝。[查詢機制](https://developer.apple.com/documentation/healthkit/hkanchoredobjectquery) · [能力表](https://developer.apple.com/help/account/reference/supported-capabilities-ios/) · [免費部署限制](https://developer.apple.com/help/account/basics/about-your-developer-account/) |

## 如何驗證

先匯出 11 月 21 日已知騎乘，比對 Watch 來源、時間戳、心率和 GPS。在既有 Bryton 時間軸範圍內，Apple 樣本有心率 2,829 個有效秒、GPS 13,562 個有效秒。再用下一趟新騎乘驗證自動到檔時間、同趟重送及是否補齊路線。檢查器以唯讀開啟匯入檔，避免影響同步資料夾及原始紀錄。[基準與比較方式](rides/2026-11-three-device-comparison.md)

工具選擇與採用狀態見[Apple Watch 漸進式匯出提案](../decisions/2026-11-23-apple-health-incremental-export.md)。
