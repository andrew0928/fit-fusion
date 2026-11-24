# 檢查器驗收情境與案例

每個案例對同一份 FIT、每個具名解碼器各跑一次。先對照下表的已知預期值，再比較各語言輸出。現況欄只涵蓋列出的實作。Rust、C#、Python 與 Node.js 可重跑往返案例。[檢查器契約與實作矩陣](../inspector.md)

| 案例 | 輸入與操作 | 通過條件 | 現有覆蓋 |
|---|---|---|---|
| I01　盤點稀疏串流 | 對合成樣本 `recording()` 執行 `inspect`。三個連續秒的原生心率為 120、FIT 無效碼 255、121。 | 3 筆 record，心率有效 2 筆，覆蓋率為 2/3（約 66.67%），心率典型頻率 0.5 Hz。record 時間軸為 1 Hz。 | 四語言合成樣本。 |
| I02　原生、自訂及未知欄位 | 同一樣本執行 `raw` 與 `inspect`。檔內另有 developer index 0 的同號欄位 3、原生未知欄位 240、未知訊息 60000／欄位 42。 | 原生與 developer 心率分列。開發者心率 3 筆皆為 80，原生 240 的零值有效。未知項仍可依編號及原始值取回。按順序串接 frame 的 `hex`，還原全部 111 bytes 及下列 SHA-256。 | 四語言逐框比對。 |
| I03　串接、時間壓縮與位元組序 | 串接兩個 `recording()`。另用現有 big-endian 陣列及壓縮時間戳回繞樣本。 | 前者產生兩個內容相同的 member。後者還原 3 個帶時間戳 record、跨度 2 秒，未知陣列僅 1 筆含有效元素。 | 四語言協定及往返測試。 |
| I04　損壞的來源 | 分別提供空檔、刪掉最後一個 byte 的樣本、錯誤 CRC 的樣本。執行 `inspect`、`raw` 並比對執行前後雜湊。 | 各命令非零結束，診斷含輸入檔。失敗的 `inspect` 沒有 JSON，失敗的 `raw` 整批捨棄，來源位元組保持一致。 | 四語言錯誤路徑測試。 |
| I05　真實騎乘的獨立解碼 | 對 11/21 的 Bryton 檔與 CIRQA 檔各自做跨解碼器比較。 | Bryton 的 14,172 個 frame、CIRQA 的 29,346 個 frame 分別相符，兩個雜湊各自還原。Bryton 的 `record.heart_rate` 在 14,149 筆 record 中有 14,084 個有效樣本，約 99.54%、1 Hz。 | 四語言的 wire 讀取器有實檔結果。 |
| I06　多 SDK 能力與差異 | 四種語言對 I02 各跑一次。另以測試替身讓一個後端漏報 240，另一個只支援 `inspect`。 | 結果按語言、後端、版本、能力分列。遺漏 240 的比較失敗並定位訊息／欄位。只支援盤點的後端，其 `raw` 標明未支援。已通過的後端仍可追溯到各自的報告。 | 四語言比對並測試 RustyFIT 遇到無描述的 developer field 時回報具體欄位。Node.js 的官方 SDK 可讀取控制樣本的有效原生值，遇到其未知訊息時無法完整重編碼。 |
| I07　數值精度 | 合成一個 `uint64` 欄位，連續存放 9007199254740992 與 9007199254740993，再跑 `raw`。 | 逐值可區分兩個相鄰整數，原始 bytes 可還原。比對器能檢出任一筆差一的情況。特別驗證 Node.js 輸出到 JSON 的路徑。 | 四語言逐值比對，Node.js 以 BigInt 保留整數精度。 |
| I08　指令探索 | 從四種檢查器的 `--skill index` 逐項取指引，以合成 FIT 執行每頁範例，另測各子命令 help 與錯誤主題。 | 索引可直接選擇任務。範例能以對應語言啟動、取得盤點與逐框資料、驗證往返收據。Node.js 另驗 SDK 比對結果。未知主題回傳 2，stderr 列可用主題。 | 四語言 CLI 自動化驗收。 |
| I09　由欄位重新編碼 | 對兩個串接 member（含同號 developer 心率欄位、未知欄及 64 位元整數）與壓縮時間／大端序樣本執行四套 `roundtrip`。 | 所有輸出通過 CRC。另一個語言逐筆讀回每個來源時間戳、欄位身分及值。原始 12-byte header 樣本重新編成 14-byte header，`byte_equal=false`、`semantic_equal=true`。 | 四語言 CLI 整合測試。 |
| I10　失敗保護 | 指定毀損的後續 member、同一路徑、硬連結、符號連結或已存在的輸出。另提供含兩個樣本的非標準純量心率欄位給 RustyFIT。 | 拒絕覆寫，原檔及既有目的保持原樣。SDK 解讀遺失第二個值時匯出失敗，未產生目的檔。 | 四語言拒寫與 RustyFIT 失真回歸測試。 |

## 重現現有基準

從專案根目錄執行。合成案例使用版本控制內的產生器，無需個人的騎乘資料。單份 `recording()` 的 SHA-256 為 `49f6c735f1d2f762da81a1b2423d139edcbd425ab422c6ded385b94d2b744211`。[樣本與統計測試](../../../src/python/tests/test_inspect.py) · [逐框測試](../../../src/python/tests/test_wire.py) · [C# 對照測試](../../../src/python/tests/test_csharp_cli.py)

```sh
PYTHONPATH=src/python python3 -m unittest discover -s src/python/tests -v
```

要將 I01、I02、I07 的相同位元組交給其他語言，可從根目錄產生兩份無個資的樣本。

```sh
PYTHONPATH=src/python:src/python/tests python3 - <<'PY'
import struct
from pathlib import Path
from test_inspect import fit_member, message_definition, recording

out = Path("runtime/acceptance/inspector")
out.mkdir(parents=True, exist_ok=True)
(out / "control.fit").write_bytes(recording())
layout = message_definition(0, 60000, [(42, 8, 0x8F)])
values = struct.pack("<BQBQ", 0, 2**53, 0, 2**53 + 1)
(out / "uint64.fit").write_bytes(fit_member(layout + values))
PY
```

有 11/21 私人樣本時可執行 I05。於 `runtime/reports/milestone-1-1/comparison.json` 逐檔核對 `raw_equal`、`inspect_equal`、`reconstructed_sha256_equal` 均為 true。比對器也保存各自的欄位報告，可再查心率的樣本數。[比對工具](../../../src/tools/compare_inspectors.py)

```sh
python3 src/tools/compare_inspectors.py \
  data/sources/bryton/fit/261121141151.fit \
  data/sources/garmin/25149578721/25149578721_ACTIVITY.fit
```

執行 `python3 -m unittest discover -s src/tools/tests -v` 驗證四套 CLI。執行 `python3 src/tools/verify_roundtrip.py` 另存合成樣本、重編碼 FIT 及逐項報告。[讀取報告與實檔示例](../../inspectors.md#四種檢查器的-fit-往返驗證)
