# Apple Watch 騎乘資料的漸進式匯出

## 狀態

proposed。2026-11-23 使用者確認需要漸進、自動地取得自己的 Apple 健康資料，每次全量匯出已超過 100 MB，並希望降低開發者帳號及年費門檻。工具選擇及資料完整度仍待實機驗證。

## 背景

fit-fusion 需要 Watch Series 9 每趟騎乘的時間、逐點心率及路線。現有全量匯出已足以作為三趟歷史樣本的比對基準。同一個健康資料庫也含 Garmin Connect 寫入的騎乘，驗證時要辨認記錄來源。[三趟樣本與取樣範圍](../research/rides/2026-11-three-device-comparison.md) · [四平台比較](2026-11-22-cloud-activity-import.md)

## 決策提案

優先驗證 HealthFit 逐趟輸出 FIT，讓 Mac 從同步資料夾讀取新活動。採用條件是能讀回 Watch 來源、取樣時間、心率及 GPS，並確認選用目的地的費用。[Apple 健康漸進匯出調查](../research/2026-11-23-apple-health-incremental-export.md)

## 影響

Mac 可逐趟接收運動檔，iPhone 鎖定與路線延後同步可能影響到檔時間及資料完整度。

## 替代方案

內建捷徑適合零成本擷取指定指標，Health Auto Export 可按期間或上次同步時間輸出，自寫 HealthKit App 則可控制新增與刪除樣本的處理。[各方案的個人使用門檻](../research/2026-11-23-apple-health-incremental-export.md)

## 後續工作

以 11 月 21 日歷史騎乘核對取樣，再用下一趟驗證自動到檔、重複活動與晚到的路線。[驗證基準](../research/rides/2026-11-three-device-comparison.md)
