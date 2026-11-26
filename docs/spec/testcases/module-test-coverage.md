# M2 Rust 模組測試與決策列對照

> 歷史規格。重跑本文的舊 CLI 與案例，請使用 [v0.2.3 原始碼](https://github.com/andrew0928/fit-fusion/tree/v0.2.3)。

M2-3 交付時有 **25 個 Rust 測試函式**。A、B、C、D 共 **76 個規格列**，一個函式可用多組資料驗證相鄰門檻。C 表的 10 個共通列分別在本機與模擬 Connect 執行。CLI 端對端測試另計。[完整條件與預期結果](module-decision-tables.md)

```sh
cargo test --offline --locked --lib --manifest-path src/rust/merger/Cargo.toml
```

## A　Config（24 列）

| 測試函式 | 決策列 | 可驗證的結果 |
|---|---|---|
| `sources_and_field_selection`、`a_config_loads_named_instances_and_priorities_offline` | A1、A1c、A1d、A2a、A2b、A3、A5a、A5b | 來源 ID 集合、主來源、啟用欄位與精確順位。Garmin 切換讀取器、改名後的設備、引用舊 ID 的錯誤位置。 |
| `multiple_devices_and_new_source_kinds`、`a_config_loads_named_instances_and_priorities_offline` | A1b、A1e、A1f、A6a、A6e | 新設備與新種類可分別加入；原來的主來源和順位保持一致；指定合成舊車錶的踏頻備援後只改變該欄。 |
| `target_registration_is_independent_of_readers`、`a_config_loads_named_instances_and_priorities_offline` | A4a–f、A6f、A6g | 空目的地、預設 Strava、兩個獨立 Strava、擴充種類、讀取與發佈方向分開，以及未知選項定位。 |
| `invalid_config_is_diagnosed_offline` | A6b–d | 版本、座標順位、Apple 心率備援被拒絕；另驗時區、路徑重疊與重複鍵。 |

測試經由設定檔載入，檢查來源讀取與平台請求計數為零。M2-3 的目的地 factory 僅驗證設定，Strava 發佈 transport 與互動測試已隨 M2-4 廢除。[Config 測試](https://github.com/andrew0928/fit-fusion/blob/v0.2.3/src/rust/merger/src/config/tests.rs)

## B　Query（5 列）

| 測試函式 | 決策列 | 可驗證的結果 |
|---|---|---|
| `b1_b2_b3_calendar_query_and_dual_threshold` | B1a、B1b、B2、B3a、B3b | 台北本地日含起不含迄；提早開錄且跨午夜可列入候選；0.90／0.95 取自設定。另驗紐約進入夏令時間的一天只有 23 小時。 |

## C　Data source（17 列）

| 測試函式 | 決策列 | 可驗證的結果 |
|---|---|---|
| `discovers_unique_cycling_files_by_content_in_time_order` | C1a–c、C4a | 本機零／一／多趟、兩筆 ID、設備、運動與起迄。 |
| `distinguishes_missing_stream_invalid_sample_and_zero_without_reopening` | C4c、C6a–c | 本機原檔雜湊，整欄無踏頻、單點無效及 0 rpm。 |
| `recovers_renames_and_never_opens_replacement_as_requested_activity`、`missing_or_corrupt_source_is_not_a_successful_partial_list` | C3a、C4b、C4d、C7 | 無法列舉、未知 ID、改名後重新列舉開啟、內容與來源身分。 |
| `c1_c2_connect_complete_pagination_and_identity` | C1a–c、C2 | Connect 取完第二頁、重複 ID 去重、零／一／多趟與來源身分。 |
| `c3_errors_preserve_auth_rate_limit_and_failed_second_page`、`http_responses_distinguish_revocation_rate_limit_and_outage` | C3b–d | 換發遭拒、429／Retry-After、第二頁失敗時整次查詢失敗，以及 HTTP 狀態到 CLI 錯誤的對應。 |
| `c4_c5_c6_cached_original_and_stream_reads` | C4a–d、C5、C6a–c | Connect 的 FIT、設備、來源 ID、UTC、缺值與零值。開啟後讀兩欄，額外下載及整檔解碼均為零。 |

另有三個跨模組回歸，分別是 `merge_accepts_already_open_recordings`、`revoked_bearer_replaced_once_and_config_update_preserves_other_sections`、`m10_date_merge_renews_token_deduplicates_ids_and_matches_dual_file`。[本機測試](https://github.com/andrew0928/fit-fusion/blob/v0.2.3/src/rust/merger/src/data_source/tests.rs) · [Connect 測試](https://github.com/andrew0928/fit-fusion/blob/v0.2.3/src/rust/merger/src/data_source/garmin/tests.rs)

## D　Policy（30 列）

| 測試函式 | 決策列 | 可驗證的結果 |
|---|---|---|
| `h1_h2_h3_preferred_fill_and_unfilled_seconds` | H1–H3 | 同秒首選、單點替補、雙方缺值與原時間軸。 |
| `h4_h5_h6_confirmed_and_unconfirmed_plateaus` | H4–H6 | 疑似平台、確認停滯後整趟切換、切換後缺值。 |
| `h7a_h7b_missing_or_unhealthy_backup` | H7a–b | 缺少備援與備援自身停滯的不同診斷。 |
| `q1_q2_missing_duration_and_fraction_boundaries` | Q1a–c、Q2a–c | 599／600 秒、全程無心率、120 秒與 5% 的交界。 |
| `q3_q4_plateau_evidence_and_pause_boundaries` | Q3a–f、Q4 | 60 秒平台、覆蓋率、心率變動、連續分歧及暫停邊界。 |
| `q5_whole_and_affected_coverage_are_independent` | Q5a–c | 全程與異常段覆蓋門檻各自生效。 |
| `f1_f2_f3_session_calories_missing_and_invalid_difference` | F1、F2、F3a–b | CIRQA 總量、兩個原值、缺總量中止及無法計算的衍生活動熱量。 |
| `f4_f5_inherit_temperature_and_select_coordinates_in_pairs` | F4、F5 | 移除溫度覆寫時，保留主來源溫度與摘要，缺半組 GPS 時兩欄同時缺值。 |

[Policy 測試](https://github.com/andrew0928/fit-fusion/blob/v0.2.3/src/rust/merger/src/policy/tests.rs) · [端對端案例](merger.md#目前可重跑的驗收)
