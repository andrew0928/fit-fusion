# FIT 檢查器 CLI 規格

檢查器回答「指定的解碼器從這一份 FIT 讀到了什麼」。它唯讀開啟單一檔案，盤點實際編碼的訊息、欄位與取樣情況。不同實作各自讀取**相同位元組**，比對工具才有條件驗證 SDK 的讀取能力。例如 `record.heart_rate` 是逐點串流，`session.total_calories` 是整趟摘要，兩者都要盤點。跨裝置判斷心率品質與選源由合併器負責。[合併器規格](merger.md)

## 語言與解碼器

| 語言 | 現況 | 要用同一套案例驗證的對象 |
|---|---|---|
| Python | 已有 `inspect`、`raw` 與 wire 欄位重新編碼。目前實檔對照使用自有 `wire` 解碼器。 | `inspect` 也可指定 `--decoder fitdecode`。報告分開記錄兩種後端，`raw` 由 wire 提供。 |
| C# | 已有 .NET `WireReader` 、獨立統計與欄位重新編碼實作。 | 現行基準為自有讀取器。Garmin 官方 C# SDK 若納入矩陣，另列後端。 |
| Rust | `inspect` 預設由 RustyFIT 0.11.2 解碼樣本，wire 提供實際編碼的欄位定義與寬度。 | `inspect FILE --decoder wire` 可獨立盤點，`raw` 提供逐框資料。往返命令執行 RustyFIT Decoder 與 Encoder。 |
| Node.js | 獨立的 Node.js wire 後端提供盤點、逐框 JSONL 與逐欄重新編碼。 | 另以 `sdk-check` 實際執行 @garmin/fitsdk 21.222.0 Decoder、Encoder，列出每個 member 的原生欄位比對與匯出差異。 |

這四種語言是首批目標。日後新增語言或同語言的其他 SDK，沿用同一份輸入、輸出契約與案例。每個後端要能獨立讀檔。若某項輸出由另一個 wire reader 補足，驗收紀錄標出實際提供者。[兩個既有實作的決策](../decisions/2026-11-21-csharp-inspection-control.md) · [SDK 試驗紀錄](../research/2026-11-23-rust-fit-cli.md)

## 命令契約

Rust 檢查器由 clap 解析參數。`inspect --decoder wire ride.fit` 與將 `--decoder` 放在檔名後方的寫法等價。`inspect ride.fit --help` 顯示 inspect 的 help。未知旗標回傳結束碼 2、將用法診斷送至 stderr，並在讀取檔案前結束。要傳入以連字號開頭的檔名，可使用位置參數分隔符號，例如 `inspect -- -ride.fit`。

以下入口以參數表示。各語言可使用自己的啟動命令。Python、C# 的可執行範例見[操作手冊](../inspectors.md)。

| 操作 | 輸入、輸出與判定 |
|---|---|
| `--version`、`-V` | 四種實作都在總覽與子命令 help 首行顯示 CLI 版本。`--version` 首行使用相同識別，下方列出編譯器或執行環境、目標或平台，以及 FIT 引擎。查詢版本可離線執行。 |
| `inspect FILE.fit` | 檢查 FIT 完整性，成功時 stdout 恰有一份 JSON。頂層的 `file` 標示來源，`members[]` 按檔案順序各自列出協定版本、profile 版本及 `messages[]`。檔內串接的多個 member 各自統計。 |
| `raw FILE.fit` | stdout 每行一個 JSON frame，保留檔案順序、byte offset 與原始 `hex`。依 frame 類型列出 header、欄位定義、資料值及 CRC。依序還原 `hex` 所得 SHA-256 必須等於輸入。 |
| `--help`、`--skill [topic]` | 四種檢查器都有總覽與子命令 help。單獨使用 `--skill` 或指定 `index` 可讀取任務索引，從中取得該語言的完整呼叫方式。共用主題為 `inspect`、`raw`、`roundtrip`，Node.js 另有 `sdk-check`。各主題說明輸入、輸出、結束碼與失敗處置。離線即可查閱。 |
| `roundtrip FILE.fit --out NEW.fit` | 解碼後按欄位重新序列化，驗 CRC 並讀回。只發布新的檔案。成功輸出 JSON，含 backend、路徑、member／訊息數、兩份 SHA-256、`byte_equal` 與 `semantic_equal`。語義差異、無效來源或目的已存在時以非零結束。 |
| 失敗處理 | 讀檔／解碼失敗回傳非零，stderr 提供原因。`inspect` 的 stdout 應為空。`raw` 的呼叫端依結束碼決定能否採用整批輸出，失敗前可能已收到部分 frame。所有路徑均保持輸入檔原樣。 |

每個 message 至少列全域編號、名稱、筆數與欄位清單。每個欄位列編號、型別、編碼長度、可取得的單位、developer index、未知標記與有效樣本數。已定義但全程缺值的欄位仍列出。原生欄位和 developer 欄位即使同號也分開計數，developer index 0 是有效識別值。

欄位涵蓋率為「含該欄位有效值的訊息筆數 ÷ 同類訊息總筆數」。無時間戳的訊息也計入分母。時間統計使用可還原的 UTC 時間戳，列出首末、跨度、正間隔的最小／中位／最大值、重複與倒退次數。典型頻率為正間隔中位數的倒數，無法計算時為 `null`。例如心率涵蓋率 99.54% 只描述成功讀到值的比例，判斷訊號停滯還需要其他裝置的時間序列。

## 跨 SDK 如何驗收

1. 固定輸入的 SHA-256。每次記錄實作語言、解碼器或 SDK 名稱與版本、自有程式版本及使用的 profile。輸入 FIT header 的 profile 版本也保留，兩種版本分別記錄。
2. 逐框比對順序、offset、原始位元組、全域訊息號、欄位定義及原始值，再比對統計 JSON。JSON 物件鍵序及空白可不同。frame、member、陣列元素的順序和整數精度須保留。比較不同 profile 時，名稱／單位差異與原始欄位編號／數值差異應分別列出。
3. 按「輸入 × 解碼器 × 能力」記錄結果。符合已知預期值且跨實作相符才標通過。輸出相異時指出第一個差異的檔案、frame／欄位或 JSON 路徑。SDK 未暴露原始 frame 時可獨立驗收 `inspect`，在 `raw` 註明能力缺口，完整逐框驗收仍待通過。

原有雙後端比對工具固定使用 Python wire 與 C# WireReader。兩者共用名稱／單位對照表。其一致結果能驗證獨立解碼及統計，該表的欄位語義則需另外查證。比對成功時回傳 0 並寫出 `comparison.json`，其中逐檔列出 SHA-256、比對筆數及通過項目。解碼或比對失敗時以非零結束並提供診斷。具版本的多後端矩陣是後續驗收目標。[現行比對工具](../../src/tools/compare_inspectors.py) · [檢查器驗收情境與案例](testcases/inspector.md)

四語言驗收另外核對 Rust 與 Node.js 的完整 raw 與 inspect，再以另一個後端解碼每份重編碼 FIT。比對項目及可重現命令見[往返實測](../inspectors.md#四種檢查器的-fit-往返驗證)。
