# 以具名實例擴充設定中的設備與發佈目的地

## 狀態

accepted。2026-11-24 騎士確認來源可增加其他車表，目的地可增加其他平台，並指定由 A 表驗證擴充能力。

2026-11-24 已調整未列欄位與新型號的處理，以[主檔保留與設定擴充決策](2026-11-24-config-primary-inheritance.md)為準。

## 背景

舊設定可自訂來源 ID，但 Schema 將本機 FIT 的型號限於兩種，目的地只接受 `strava`。納入另一台 Bryton 車表與新增一種平台上傳方式，需要兩種不同的擴充點。

## 決策

`sources` 與 `destinations` 各自按實例 ID 宣告。相同取檔方式可搭配不同設備，相同連線器可建立多個實例。讀取與發佈各自註冊 factory。Schema 驗證設定形狀，載入器依註冊資訊確認設備與選項。既有 `destinations.strava` 可繼續省略 `kind`。[設定檔範例](../config/README.md#如何增加設備或目的地)

## 影響

僅新增來源會讓它可供列舉。要讓它參與某個欄位的選值，仍須明列該欄順位。新增目的地供列舉後，由呼叫者指定本次發佈對象。

## 替代方案

逐一在 Schema 列舉型號與目的地，會讓每個新型號和平台都需要修改共同格式。以實例、設備、連線器分別識別，可重用既有取檔實作與逐欄選源規則。

## 後續工作

A 表以測試註冊項目驗證 Neo15、Bryton Activity 與 Garmin Connect 目的地。正式納入 Neo15 時核實原始 FIT 的設備識別，新增平台發佈能力時另驗證授權及上傳。[Config 決策表](../spec/testcases/module-decision-tables.md#aconfig-test)
