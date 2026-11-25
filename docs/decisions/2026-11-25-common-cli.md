# 所有 CLI 都以 clap 解析並提供 help 與 skill

## 狀態

`accepted`。2026-11-25 使用者確認所有 CLI 一律使用 clap 並支援 `--help`、`--skill`。此決策擴大[兩支 Rust CLI 的既有決策](2026-11-24-rust-clap.md)適用範圍。

## 背景

0.2.x 將依任務分出多支可獨立執行的工具。使用者和 agent 需要從每個執行檔取得相同形式的操作入口。

## 決策

命令與選項由 clap 解析。各工具離線提供版本化 help、skill 索引與可操作的主題指引。新 CLI 及既有其他語言的對外入口都適用。[共通規格與驗收](../spec/cli-common.md)

## 影響

新工具從第一個開發任務就驗收人與 agent 的入口。舊版非 Rust 檢查器提供 help／skill，現有 parser 的遷移列入 M4-5。各語言保持獨立的資料解碼實作。

## 替代方案

原先由各語言自行選擇或實作 parser。使用者指定統一採用 clap。

## 後續工作

每支 CLI 交付時執行共同案例，並更新採用狀態。[M4 工作順序](../milestones/m4-cli.md)
