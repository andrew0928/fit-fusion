# M2 Rust 模組測試與決策列對照

2026-11-24 以 `cargo test --offline --locked --lib --manifest-path src/rust/merger/Cargo.toml` 重跑，9 個函式全數通過。A、B、C、D 分別有 24、5、17、30 列。下表以測試內實際執行的操作與斷言為準，同一函式可涉及多列。

| Rust 測試函式 | 對照列 | 已驗證的結果與缺口 |
|---|---|---|
| `sources_and_field_selection` | A1、A1c、A1d、A2a、A2b、A3、A5a、A5b | 已驗預設主來源、心率與熱量順位、改取檔方式後逐欄順位、移除溫度、來源改名與舊引用的錯誤位置。A1c 還要驗單一踏頻的順位，A1d 要比較兩次來源 ID 集合，A2a／b 要直接查啟用清單，A5a 要核對改名後的設備。A1 的來源集合目前還額外限定了列舉順序。 |
| `multiple_devices_and_new_source_kinds` | A1b、A1e、A1f、A6a、A6e | 已驗未知種類與設備的錯誤位置，也驗 Neo15 踏頻備援及心率、熱量順位。A1b 只加設備時僅斷言來源數；A1e 還要在只加新種類的條件下，核對新增 ID、主來源與全部原有順位。 |
| `target_registration_is_independent_of_readers` | A4a、A4c、A4d、A4e、A4f、A6f、A6g | 已驗零目的地、三個具名目的地及讀取／發佈分開註冊。A4d 只查第一個 ID，A4e／f 在同一設定只核對數量，A6g 只定位到 options。A4b「只設定 Strava 且 token 空白」尚無獨立斷言；A4b／c 的授權啟動次數也待驗。 |
| `invalid_config_is_diagnosed_offline` | A6b、A6c、A6d | 已驗版本、GPS 順位不一致、Apple 心率備援的錯誤位置。另驗未知時區、輸入輸出目錄重疊與重複 JSON 鍵。 |
| `discovers_unique_cycling_files_by_content_in_time_order` | C1a、C1b、C1c、C4a 的本機版本 | 已驗 0／1／2 趟、來源與活動身分、設備、運動和 UTC 時段。C1c 另宜明列第二趟的預期 ID。這裡傳入已建好的 UTC 查詢，B 表的日期轉換與查詢建構仍待測。 |
| `distinguishes_missing_stream_invalid_sample_and_zero_without_reopening` | C4c、C6a、C6b、C6c 的本機版本 | 依內容雜湊開啟並核對原始 bytes；移走來源檔後仍能讀踏頻與摘要。已區分整欄缺、單點無效及有效零值。C5 還需要 Connect 下載與解碼計數。 |
| `recovers_renames_and_never_opens_replacement_as_requested_activity` | C4b、C4d 的本機版本及 C7 | 已驗改名後可用舊 ID 開啟、重新列舉得到相同 ID、替換內容後回報 `activity_not_found`。C4b／d 還要斷言錯誤內的 ID，C7 還要按該列順序用新列舉的 ID 開啟，核對來源 ID、樣本與摘要。 |
| `missing_or_corrupt_source_is_not_a_successful_partial_list` | C3a | 目錄不存在時驗得 `source_io`。另驗同一目錄有完整與毀損 FIT 時回報失敗。 |
| `merge_accepts_already_open_recordings` | 表外的跨模組回歸 | 驗證開啟兩份 recording 後可刪除輸入檔再合併，報告保留兩個來源 ID 及一筆 Garmin 心率替補。D 表要求直接驗證策略回傳與原因。 |

A 表的共同條件另行追蹤。現有四個函式直接呼叫 `Settings::parse`；公開的 `Settings::load` 已由 CLI 回歸走過。模組測試仍須直接驗證載入設定時活動檔讀取、平台請求及上傳各為零，也要對錯誤案例同時斷言錯誤類別及位置。[Config 測試碼](../../../src/rust/merger/src/config/tests.rs) · [本機來源測試碼](../../../src/rust/merger/src/data_source/tests.rs)

## 剩餘測試的執行順序

以下按可驗收的 M2 交付點排程。測試失敗時應能從測試名稱或斷言訊息辨識 Row ID，同一組資料可供多列使用。

1. **M2-3 下一個工作項目，先補現有模組。** 補 A 表共同條件及上述缺少的 Result，再補 C1c、C4b／d、C7 的本機斷言。D 表先處理 H1–H6、H7b、全部 Q 列與 F1、F2、F3a、F3b，共 27 列。這批涵蓋相鄰門檻、整趟切換、Garmin 熱量原值與缺值診斷，需要時把已在報告中產生的資訊提供給策略介面驗證。這些可用記憶體資料執行。
2. **M2-3 本機日期查詢實作時。** 先寫 B1a、B1b、B2、B3a、B3b，以台北跨日及 0.90／0.95 驗證查詢建構，再接本機來源配對。日期流程接上自訂選源時完成 D 表餘列，包括 H7a 缺備援檔、F4 移除溫度、F5 成對座標。在本機日期合併驗收前對齊 D 表全部 30 列。
3. **M2-3 Garmin Connect 讀取器實作時。** 用模擬回應完成 C2、C3b、C3c、C3d、C5，並將 C1a–c、C4a–d、C6a–c 共 10 個共通情境對 Connect 再跑一次。驗收包含第二頁失敗、token 換發失敗、429，以及讀同一活動兩個欄位的下載／解碼次數。
