# Garmin 使用 dotenv，0.3.0 規劃共通設定

## 狀態
accepted。2026-11-25 使用者在 v0.2.1 發佈前確認移除 M4-3 的 macOS Keychain 支援。

## 背景
M4-3 已用 .env 完成 Garmin 正式帳戶驗收。

## 決策
金鑰只取自工作目錄的 .env 或 --env-file 指定的檔案。Bearer 僅在執行時換發及使用。離線讀取原檔沿用快取。

## 影響
移除 Keychain 查詢、寫入及 auth 子命令。`--skill auth` 保留為離線的憑證說明。

## 替代方案
原本的程序環境變數與鑰匙圈方案由本決策取代。[原決策](2026-11-25-garmin-query-credentials.md)

## 後續工作
0.3.0 再設計及實作位於 `~/.fit-fusion/` 的共通設定檔。
