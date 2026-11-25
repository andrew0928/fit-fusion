# M5-5 查詢工具的設定檔命名與分檔

## 狀態

`superseded`。2026-11-25 提出依執行檔分檔的方案。2026-11-26 使用者選定[單一 YAML 的共用與工具區段](2026-11-26-single-yaml-config.md)。下文保留原提案。

## 背景

[M5-5](../milestones/m5-release.md) 已指定 `~/.fit-fusion/`，並要求 `fit-merge` 能免設定檔執行。三個查詢來源分別使用本機 FIT 目錄、Apple Health ZIP 和 Garmin 憑證。

## 決策

提案採用 `~/.fit-fusion/<cli-name>.yaml`。有個人預設值的工具只讀同名檔，命令列明確指定的值優先。先為三支查詢工具定義選填檔案。Garmin YAML 可指定 dotenv 路徑，金鑰沿用既有的 [dotenv 契約](2026-11-25-garmin-dotenv.md)。

## 影響

使用者可從命令名稱推得檔名；單一來源的設定錯誤只影響該工具。若三支查詢工具都要覆寫預設時區，分檔方案可能需要重複設定。

## 替代方案

先前提出單一 `~/.fit-fusion/config.json`，內含共用時區和各來源區段。其優點是時區只需設定一次。XDG 另有 `$XDG_CONFIG_HOME`（預設 `~/.config`）的目錄慣例，若採用會改變 M5-5 目前指定的位置。[XDG 規範](https://specifications.freedesktop.org/basedir/latest/)

## 後續工作

待確認是否需要一次變更所有查詢工具的時區。定案後驗證缺少設定檔、命令列覆寫、單一設定檔損壞時的行為，以及既有 Garmin `--env-file` 指令與離線工具的相容性。
