# 正式版 FIT 處理函式庫選型

## 狀態

proposed。2026-11-21 使用者提出 C# 與 Rust 兩個候選，並指定以 FIT 處理能力及相容性為優先。當時建議優先考慮 C#。2026-11-22 使用者補充語言偏好為 Rust > C# > JavaScript > Python，最終選型留待開發規格階段。

## 背景

正式版需要讀取不同廠牌的活動，依時間整合感測資料，輸出可匯入 Strava 的 FIT，並保留可供分析的完整資料。現有兩份樣本中，Bryton 含廠商自訂訊息 65282、65286。這類訊息與 FIT 2.0 developer fields 使用不同的編碼機制，應分別驗證。[FIT Protocol](https://developer.garmin.com/fit/protocol/)

## 決策

建議優先評估 Garmin 官方 C# FIT SDK。它提供 activity 編碼及 FIT 2.0 developer fields 的實作範例，能以同一套工具處理輸入與輸出。是否能保存這兩份裝置檔的未知欄位，仍須以實測確認。[官方 C# SDK](https://github.com/garmin/fit-csharp-sdk) · [Developer Data 範例](https://developer.garmin.com/fit/cookbook/developer-data/)

2026-11-22 補充建議：以官方 C# SDK 作相容性對照，優先驗證第三方純 Rust 的 `fit-sdk-rust`。Garmin 官方提供能讀寫 FIT 的 C#、JavaScript 與 C 等 SDK；Python 官方版僅支援解碼，官方語言清單沒有 Rust。Rust 可透過 FFI 呼叫官方 C SDK，需自行維護綁定及驗證實際可保存的欄位。純 Rust 方案若能通過三趟共六份原始 FIT 及一份含 Apple developer fields 的修復檔驗證，便符合語言偏好；否則優先考慮 C#。目前專案的 C# 檢查器是獨立實作。[Garmin 官方語言與能力表](https://developer.garmin.com/fit/get-the-sdk) · [Rust crate 文件](https://docs.rs/fit-sdk-rust/latest/fit/)

## 影響

選型影響正式版的解碼器、FIT 寫出器與相容性測試。第一里程碑使用 Python 檢視真實樣本，其原始匯出可供後續比對。

## 替代方案

Rust 的 `fitparser` 支援解碼；若採用 Rust，可評估提供解碼與編碼的 `fit-sdk-rust`，以同一批資料比較未知訊息、陣列、無效值與 developer fields 的保存結果。[fitparser](https://docs.rs/fitparser/latest/fitparser/) · [fit-sdk-rust](https://docs.rs/fit-sdk-rust/latest/fit/)

## 後續工作

1. 對兩份真實樣本做讀入、寫回、再讀入試驗，逐筆比對時間、值、欄位編號、型別及來源，列出差異。
2. 用另含 developer fields 的 fixture 驗證定義、索引及資料值，尤其是與原生欄位編號相同的情形。
3. 另驗證匯出活動的 Strava 匯入結果，以及分析輸出能否追溯 FIT 未知欄位，再記錄選型結論。
