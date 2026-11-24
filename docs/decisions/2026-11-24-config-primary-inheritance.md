# config v1 預設保留主 FIT，依名稱選取串流並以設定辨識新設備

## 狀態

accepted。2026-11-24 使用者要求預設完整保留 primary source，串流與新設備由 config 調配，並確認「直接修改 v1，所有現有設定改用新規則」。本決策取代[原設定契約](2026-11-23-date-driven-config.md)中省略欄位即移除的行為，以及[具名設備擴充決策](2026-11-24-config-extensions.md)中新增型號須先註冊的要求。

## 背景

原 v1 的列舉已包含標準功率，卻會在未設定時移除車錶記錄的瓦數。使用者添購感測器或換用另一台可輸出 FIT 的車錶時，希望用原本的 CLI 處理。

## 決策

- 主來源提供時間軸及整份基礎 FIT。`merge.streams` 只列需要覆寫或追加的欄位及來源順位。使用者確認要縮減設定後，v1 允許省略整段，效果與填 `{}` 相同，日期合併會使用同一個修訂 ID。
- `config init` 範本列三條規則：心率以車錶優先、手環備援，兩個原生熱量欄位 `session.total_calories` 與 `session.metabolic_calories` 均取自手環。GPS、踏頻、功率和其他主檔串流會自動沿用。已有設定中的明確順位仍有效。
- 按內附 FIT Profile 將標準 `record.*` 名稱對應原生欄號，按輸入檔的欄位定義保留型別、陣列及位元順序。更新的原生欄或私有原生欄可在 `merge.field_ids` 設定名稱及欄號。來源應對同一欄號使用相同語意與單位。主檔的 developer 欄位連同定義保留；跨設備選取 developer 欄仍需額外的應用程式及欄位身分設計。
- 現有設備別名可沿用。對新型號直接在具名來源的 `device` 指定 FIT `file_id.manufacturer` 與 `file_id.product`。現有本機 FIT 或 Garmin Connect 連線器負責取檔。新的雲端協定仍由連線器實作。

## 影響

同一份 v1 設定中未列出的溫度、功率、心率或熱量可能重新出現在輸出。缺值與有效的 0 分開處理。規則版本已更新，日期合併另建修訂，既有輸出保留。手動合併若指定已有不同結果的目錄，仍回報衝突。分析檔 v2 的 `selected` 也涵蓋主檔直接沿用的 record 感測器欄。Strava 的 FIT 支援以平台公布的欄位為準。[官方 FIT 欄位表](https://developers.strava.com/docs/uploads/)

## 替代方案

曾提出使用 config v2 分隔新舊行為。使用者選擇直接套用至 v1。

## 後續工作

以合成的兩個新型號驗證按日期列舉、手動交換輸入、逐點功率補缺、左右平衡、陣列與空覆寫。Neo15 實際的製造商與型號編號仍需由原檔查證。[可執行範例](../config/stream-fit.example.json) · [CLI 驗收](../spec/testcases/merger.md)

2026-11-24 已取得 Bryton Rider 15 Neo 原檔，確認型號為 `267/2004`。後續新增以用途命名來源與目錄型號篩選選填的決策。[來源角色與設備識別](2026-11-24-source-roles.md)
