# Garmin 獨立查詢使用個人 API 金鑰，保留本機原始 FIT

## 狀態

`accepted`。2026-11-25 使用者要求先發佈 v0.2.0，再開發 M4-3，並確認目前直接用專案 `.env` 的 Garmin 金鑰。個人登入鑰匙圈的密碼與解鎖問題另列待辦。

## 背景

騎士已有 Garmin Connect Personal API key 與先前下載的原始 FIT，可用同一活動 ID 交叉驗收。查詢工具應能單獨執行。[Garmin 個人 API 說明](https://support.garmin.com/en-US/?faq=HfTwLfw1Gn2A7WS1GdAMH8)

## 決策

目前使用 `fit-query-garmin --env-file .env` 讀取 `GARMIN_PERSONAL_ACCESS_TOKEN`；該變數存放可換取短效 bearer 的 personal API key。CLI 僅解析指定變數，不執行檔案中的 shell。Bearer 在此次執行的記憶體內換發。選用 `auth import` 才寫入 macOS Keychain。活動以 Garmin ID 識別，原始 FIT 以 SHA-256 核對及快取，輸出依 ID 命名。

## 影響

查詢日期以指定時區的出發日為準。依活動 ID 讀取或匯出時，可在離線模式重用已核對的快取。回報及產物保留活動識別、來源與原始檔雜湊。Keychain 的選用機制與個人密碼問題不影響 `.env` 流程。

## 替代方案

一般密鑰字串可存入 macOS Keychain generic password。Secure Enclave 的應用程式介面提供本機產生的私鑰運算，Garmin 預先核發的 API key 仍需由客戶端取用以交換 bearer。本版以 generic password 做選用的保存方式。[Apple Keychain](https://developer.apple.com/documentation/security/adding-a-password-to-the-keychain) · [Apple Secure Enclave](https://developer.apple.com/documentation/security/protecting-keys-with-the-secure-enclave)

## 後續工作

個人登入鑰匙圈解鎖仍待騎士排除。鎖定、解鎖、匯入、輪替及讀回已用獨立的臨時鑰匙圈與合成金鑰驗證。日常驗收可持續使用 `--env-file .env`。[M4-3 驗收案例](../spec/testcases/m4-cli.md#m4-3-garmin-驗收-r09)
