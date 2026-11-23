# Rust、Node.js 與 C# 的 FIT CLI 選型與讀寫實測

2026-11-23。本次比較後，使用者選定 Rust、`rustyfit` 與 macOS。Garmin 官方 C# SDK 可作交叉對照。FIT 檔案相容性取決於輸出的協定結構、活動訊息及欄位。Garmin 公開協定，第三方程式可以直接讀寫。Garmin 的官方語言清單未列 Rust，Rust 方案要自行追蹤 FIT Profile 更新、函式庫維護及裝置相容性。[FIT 協定](https://developer.garmin.com/fit/protocol/) · [官方 SDK 語言清單](https://developer.garmin.com/fit/get-the-sdk/) · [技術選型決策](../decisions/2026-11-21-production-fit-sdk.md)

## 開發語言影響哪些工作

下載檔案、OAuth、上傳、查詢活動、等待處理結果與寫入 Parquet 都可以在 Rust CLI 內完成。各平台的個人 API、權限與同步方向仍依平台而定。Apple Watch 內建運動可由 iPhone 上已授權的 App 匯成檔案，再交給 Mac 的 CLI。專案特有的工作是按 Bryton 時間軸與來源優先順序選值、處理缺漏、保存來源的額外欄位並產生報告。這些規則需由應用程式實作。[平台能力比較](2026-11-23-activity-platform-automation.md) · [Apple 漸進匯出](2026-11-23-apple-health-incremental-export.md) · [Apache Parquet 的 Rust 函式庫](https://docs.rs/parquet/latest/parquet/) · [專案規格](../specification.md)

## 同一份 FIT 的讀寫結果

輸入採專案既有 `tests/test_inspect.py` 的 `recording()`。檔案長度 111 bytes，SHA-256 為 `49f6c735f1d2f762da81a1b2423d139edcbd425ab422c6ded385b94d2b744211`。它包含 6 筆資料訊息，3 筆 `record` 都有未知原生欄位 240，另有未知訊息 60000 的欄位 42。標準心率欄位 3 的值依序是 120、255（無效值）、121，並存的 developer field 也是欄位 3，使用 developer index 0，值皆為 80。這是測試 FIT 編碼的合成資料。[測試資料產生器](../../tests/test_inspect.py)

各工具先進行解碼與重新編碼，全程不改值，再用專案獨立的 Python `wire_frames` 驗證 CRC、逐筆比較全域訊息號碼、欄位順序、編號、型別、長度、developer index 與原始值。完整檔案的雜湊另行比較。兩個通過逐欄比對的 SDK 均寫出 113 bytes，輸入為 12-byte header，兩者輸出均為 14-byte header。[獨立讀回器](../../fit_fusion/wire.py)

| 實測工具與讀寫路徑 | 本次結果 | 對 fit-fusion 的意義 |
|---|---|---|
| Rust 社群 `rustyfit 0.11.2`，使用協定層的 `Decoder::decode` 與 `Encoder::encode` | 6 筆訊息逐欄相同，包括未知欄位、無效值與同號 developer field。另驗證指定改值及新增 developer field。 | 純 Rust 已有可行路徑。此版要求 Rust 1.93，本機原有 1.90，本次以暫存的 1.93.1 編譯。[RustyFIT](https://github.com/muktihari/rustyfit) · [版本](https://crates.io/crates/rustyfit/0.11.2) |
| Rust 社群 `fit-sdk-rust 0.2.1`，使用 README 的 typed decoder → `Encoder::encode` | CRC 通過，但 3 筆 `record` 均失去欄位 240。訊息 60000 仍在，欄位 42 從 1 byte 變為 4 bytes。 | 此版本的 typed encoder 對已知訊息會略過 profile 中找不到的標準欄位，對未知訊息則可能從值推導型別。採用此途徑須另做原始欄位保存。[Crate 與編碼範例](https://docs.rs/crate/fit-sdk-rust/0.2.1) |
| C# 官方 `Garmin.FIT.Sdk 21.222.0`，將 `Decode.MesgEvent` 的通用 `Mesg` 依序交給 `Encode.Write`（協定 V2） | 6 筆訊息逐欄相同。專案原有的獨立 C# 檢查器與這次使用的官方 NuGet 套件是兩種實作。 | 可作 Rust 輸出的對照。若實騎資料揭露 Rust 寫回限制，也可把 FIT 讀寫交給 C# 小工具，或全案採 C#。[官方 C# SDK](https://github.com/garmin/fit-csharp-sdk) · [官方 developer data 範例](https://developer.garmin.com/fit/cookbook/developer-data/) |
| Node.js 官方 `@garmin/fitsdk 21.222.0`，使用 `mesgListener` → `Encoder.onMesg`，並註冊 developer field | 開啟 `includeUnknownData` 後，讀取端能看到 240 與 60000。寫訊息 60000 時回報 `Could not write Message`。另做對照刻意略過 60000 後可寫出，但 240 仍消失，developer field 有保留。 | 適合已知欄位的活動編碼及 API 整合。若要保存這批私有資料，直接將解碼物件交給 encoder 會有缺口。[官方 JavaScript SDK](https://github.com/garmin/fit-javascript-sdk) |

另做兩個獨立的 RustyFIT 改寫試驗。其一，把中間那筆標準心率的無效值 255 改為 130，讀回後只有該值改變，既有 developer 心率仍為 80。其二，保留原活動欄位，新增一組 `developer_data_id` 與 `field_description`，再把測試用 `watch_hr` 寫入 developer index 1、欄位 3。讀回三個新值 101、102、103，並確認同號的原生心率和 index 0 的欄位都在。測試數字來自合成樣本。

其他 Rust 選擇中，`fitparser` 提供解碼與資料查詢，寫出活動需搭配另一個 encoder。Rust 也能透過 FFI 呼叫 Garmin 官方 C SDK，代價是要建置與封裝原生函式庫、維護綁定，再驗證未知欄位。這兩條路均未在本次試驗中採用。[fitparser](https://docs.rs/fitparser/latest/fitparser/) · [Garmin C SDK](https://github.com/garmin/fit-c-sdk)

## Garmin 的 CLI 能處理到哪裡

| 工具 | 能完成的工作 | 專案中的邊界 |
|---|---|---|
| `FitCSVTool.jar`（Java） | `-b` 將 FIT 轉為 CSV，`-c` 將工具格式的 CSV 轉回 FIT。`-i` 先檢查輸入完整性，可篩選定義或資料訊息、顯示無效值。 | 本次用 21.222.0 執行 `-i -b` 再 `-c`，編碼端明確回報略過未知欄位和訊息。獨立讀回確認 240 與 60000 消失（6 筆變 5 筆）。適合檢視、除錯和已知欄位編輯。CSV 往返不符合本案的未知資料保存要求。[參數表](https://developer.garmin.com/fit/fitcsvtool/commandline) · [測試版本](https://github.com/garmin/fit-sdk-tools/releases/tag/21.222.0) |
| `ActivityRepairTool.jar`（Java） | 修復常見的 FIT Activity 檔結構問題，救回損壞點之前可解讀的資料。 | 檔案結構正常但心率錯誤或缺漏時，工具無從決定該用哪一台裝置的讀值。[官方工具說明](https://github.com/garmin/fit-sdk-tools) |
| `FitGen` | 依設定產生或精簡 SDK 的 profile 程式碼，可加入已知自訂訊息。 | 適合有廠商欄位定義時產生程式碼。未知欄位的物理意義仍需來源佐證。[官方 FitGen 說明](https://github.com/garmin/fit-c-sdk#generating-the-c-sdk-with-fitgen) |
| `gc-personal`（Garmin Connect 個人 API 的 Python CLI） | 用個人 key 列舉活動（`activities`）、讀取明細（`activity <ID>`）、下載活動檔 ZIP（`activity-download <ID>`）。 | 處理 Garmin 帳號的讀取。合併與上傳需另外實作。本案的發佈目標 Strava 有自己的上傳 API。[官方 CLI](https://github.com/garmin/connect-personal-api-python) · [Strava 上傳](https://developers.strava.com/docs/uploads/) |

## 實騎檔案的相容性驗收

本次 checkout 沒有個人活動 FIT。驗收時，以 11 月 1、8、21 日的六份 Bryton／Garmin 原始 FIT 及含 Apple developer fields 的修復檔重做讀寫。重點包括 Bryton 私有訊息 65282、65286、Garmin 未知欄位 135、136、143，以及陣列、無效值、壓縮時間戳、取樣順序和來源歸屬。用原有 frame 檢查器比較未改動內容，再讀回替換的心率與新增欄位。輸入檔唯讀，產物另存。Strava 匯入後能查回哪些序列需另行驗證。[實騎欄位與需求](../specification.md) · [三趟騎乘](../2026-11-three-device-comparison.md)
