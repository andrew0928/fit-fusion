# 查詢工具共用活動與設備名稱、文字輸出；檢查器依欄位盤點缺樣

## 狀態

accepted。2026-11-25 使用者確認三支查詢工具都加入 `activity_name`、`device_name` 與 `--text`，維持預設 JSON，並確認 M4-5 檢查器介面。

## 背景

活動 ID 適合後續命令精確選取。人從同一天多台設備的活動中選一筆時，還需要來源提供的標題與可辨識的記錄設備。同一份 Apple Health ZIP 可包含 Watch 與 Connect 各自寫入的 Workout。

## 決策

- 三支 `fit-query-*` 的活動清單、單筆明細與匯出項目分別提供 `activity_name`、`device_name`。前者取來源的活動標題，後者取記錄設備的產品名稱或查表結果。缺少可辨識資訊以 JSON `null` 表示。ID、來源與設備識別欄位各自保留。
- 預設輸出縮排 JSON，`--json` 輸出緊湊 JSON，`--text` 提供便於閱讀的報告。兩個旗標互斥。輸出格式共用查詢結果及結束碼。
- `fit-inspector inspect FILE [--stream NAME]…` 允許重複點名欄位；省略時盤點實際欄位。依 FIT member 與訊息類型計數，分開報告完全缺欄、全數 invalid、部分缺樣和零分母。輸出 JSON，說明收在 `--skill inspect`。

## 影響

Garmin 輕量列表直接採 Connect 的活動名稱；已有本機原始 FIT 時也可辨識設備。FIT 檔名、Apple 的寫入來源名稱及運動類型都各有獨立語義。缺樣率描述訊息帶有有效值的比例，無法單獨辨識持續寫入的舊心率。

## 替代方案

以文字作預設會改變現有 agent 流程；要求使用者從多層設備資料自行取名稱也會增加跨來源操作成本。

## 後續工作

以 R08、R09、R10 驗證名稱、來源、缺值及輸出格式。M4-5 以 R11、R12 驗證取樣規則與唯讀性。[命令規格](../spec/m4-cli.md) · [驗收表](../spec/testcases/m4-cli.md)
