# 正式 CLI 採用 Rust、RustyFIT 與 macOS

## 狀態

accepted。2026-11-23 使用者在上一輪 RustyFIT 讀寫比較後，明確指定開發語言為 Rust、套件為「rustfit」、平台為 macOS。依該輪實測對象，套件在 Cargo 中記為 `rustyfit`，上游專案名為 RustyFIT。[上游專案](https://github.com/muktihari/rustyfit) · [選型比較與實測](../research/2026-11-23-rust-fit-cli.md)

## 背景

正式版需要讀取不同廠牌的活動，依時間整合感測資料，輸出可匯入 Strava 的 FIT，並保留可供分析的完整資料。現有兩份樣本中，Bryton 含廠商自訂訊息 65282、65286。這類訊息與 FIT 2.0 developer fields 使用不同的編碼機制，應分別驗證。[FIT Protocol](https://developer.garmin.com/fit/protocol/)

## 決策

正式 CLI 以 Rust 開發，在 macOS 執行。FIT 的解碼與寫出採用社群維護的 `rustyfit`。已驗證的開發基準為 `rustyfit 0.11.2` 搭配 Rust 1.93.1。該版在合成檔上能寫回未知欄位、單獨替換一筆標準心率，並新增與既有欄位同號、索引不同的 developer field。[讀寫實測與限制](../research/2026-11-23-rust-fit-cli.md)

## 影響

正式版可用歷史里程碑的 Python、C# 獨立檢查器逐欄核對輸出。採用 RustyFIT 後，Bryton 和 Garmin 真實檔案的寫回保真、Strava 的匯入結果仍待實測。

## 替代方案

- 2026-11-21 初案優先評估 Garmin 官方 C# SDK，因為它同時支援 activity 編碼和 developer fields。官方 C# SDK 可供相容性對照，與 1-1 里程碑的獨立檢查器各自實作。[官方 C# SDK](https://github.com/garmin/fit-csharp-sdk) · [Developer Data 範例](https://developer.garmin.com/fit/cookbook/developer-data/)
- 2026-11-22 使用者提出 Rust > C# > JavaScript > Python 的偏好，當時優先評估 `fit-sdk-rust`。11 月 23 日實測該套件 0.2.1 的 typed encoder 會漏寫欄位 240，遂提出並採用 RustyFIT。其他考慮過的途徑包括僅支援解碼的 `fitparser` 及透過 FFI 整合 Garmin C SDK。[候選套件比較](../research/2026-11-23-rust-fit-cli.md)
- 11 月 22 日比較中對 Python 的描述已於 11 月 23 日更正。Garmin 官方 Python SDK 21.222.0 提供 `Encoder`。[官方 Python SDK](https://github.com/garmin/fit-python-sdk#encoder)

## 後續工作

1. 對三趟的六份 Bryton／Garmin 原始 FIT 與一份含 Apple developer fields 的修復檔做讀入、寫回、再讀入試驗，逐筆比對時間、值、欄位編號、型別及來源，列出差異。
2. 用另含 developer fields 的 fixture 驗證定義、索引及資料值，尤其是與原生欄位編號相同的情形。
3. 驗證 Strava 匯入正式輸出的結果，以及分析輸出能否追溯 FIT 未知欄位，將結果記入相容性報告。
