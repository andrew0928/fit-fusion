# M2 模組介面　設定與活動來源

**規格狀態：已確認（accepted）。** 2026-11-24 騎士確認 `config` 與 `data_source` 的模組邊界、C# 與 Rust 對照的方法簽名及驗收條件，作為 M2 實作依據。[介面決策](../decisions/2026-11-24-config-and-data-source.md)

**換取檔方式**時，同一個 Garmin 來源可從本機目錄改由 Connect 取得，選值規則繼續適用。**換選值做法**時，同一批已開啟的原檔可以重跑策略，取得新結果與理由。本文以 C# 表達呼叫端的意圖，Rust 是 M2 的實作語言。[config v1 檔案格式](../config/README.md)

## 從一趟騎乘看邊界

| 呼叫者想做什麼 | 由誰回答 | 呼叫完成後可依賴什麼 |
|---|---|---|
| 只給日期，決定先找誰、還要找哪些來源 | `config` 的 `Settings` | 可執行的來源物件、已選欄位與各欄順位。設定列有 Apple Watch 時亦可離線查看。 |
| 找出活動、讀 metadata、取得原始紀錄 | `data_source` 的 `DataSource` | 此來源的活動識別、UTC 時段與唯讀資料。目錄掃描、分頁、換發憑證和快取留在連線器內。 |
| 決定同趟各欄的最終值 | 由 `Settings` 提供的 `MergePolicy` | 一次評估整趟，交付選值與來源證據，或帶診斷的待判讀結果。 |
| 配對候選、寫 FIT／報告、應要求發佈 | 既有合併器 CLI | CLI 協調上述操作，將同一次選值交給輸出器。只有明確要求發佈才呼叫 target。 |

以下兩段是同一趟**已完成配對**後的呼叫。11/01 可讀到逐點替補的心率，11/21 可讀到整趟選自 Garmin 的心率。呼叫端不需要知道在哪一秒觸發切換。

```csharp
ISettings settings = ConfigLoader.LoadSettings(configPath);
SelectedRide selected = settings.GetMergePolicy().Select(matchedRide);
var heartRate = selected.GetHeartRate();
```

```rust
let settings = Settings::load(config_path)?;
let selected = settings.merge_policy().select(&matched_ride)?;
let heart_rate = selected.heart_rate();
```

## config　從設定取得行為

原草圖的 `IEnumerator<IDataSource> GetSources()` 表達「讓我逐一使用來源」。C# 公開 `IEnumerable<T>` 供每次 `foreach` 取得自己的列舉器。Rust 回傳借用來源的 `Iterator`。原草圖的 `T GetData(...)` 分成「依策略選完整趟」與「向選值結果讀取」兩步。這使 11/21 前段已出現的心率也能採用整趟判定。

```csharp
public interface ISettings
{
    IEnumerable<IDataSource> GetSources();
    IDataSource GetPrimarySource();
    IEnumerable<FieldKey> GetSelectedFields();
    IEnumerable<IDataSource> GetSourcesFor(FieldKey field);
    IEnumerable<IDataTarget> GetTargets();
    ActivityQuery GetPrimaryQuery(DateOnly departureDate);
    ActivityQuery GetCandidateQuery(UtcRange primaryRide);
    IMergePolicy GetMergePolicy();
}

public interface IMergePolicy
{
    SelectedRide Select(MatchedRide ride);
}
```

Rust 用具私有狀態的 `Settings` 與 `MergePolicy` 提供相同操作。以下是公開方法簽名，省略方法本體；`DataSource` 與 `DataTarget` 是可替換實作的 trait。`&dyn DataSource` 表示呼叫者在 `Settings` 的生命週期內借用該來源。

```rust
impl Settings {
    pub fn load(config_path: &Path) -> Result<Self, ConfigError>;
    pub fn sources(&self) -> impl Iterator<Item = &dyn DataSource>;
    pub fn primary_source(&self) -> &dyn DataSource;
    pub fn selected_fields(&self) -> impl Iterator<Item = FieldKey>;
    pub fn sources_for(&self, field: FieldKey) -> impl Iterator<Item = &dyn DataSource>;
    pub fn targets(&self) -> impl Iterator<Item = &dyn DataTarget>;
    pub fn primary_query(&self, departure_date: LocalDate) -> ActivityQuery;
    pub fn candidate_query(&self, primary_ride: UtcRange) -> ActivityQuery;
    pub fn merge_policy(&self) -> &MergePolicy;
}

impl MergePolicy {
    pub fn select(&self, ride: &MatchedRide) -> Result<SelectedRide, QualityReview>;
}
```

- **載入。** C# 的 `ConfigLoader.LoadSettings(string configPath)` 對應 Rust 的 `Settings::load(&Path)`。兩者讀取設定、解析相對路徑、驗證版本、Schema、設備及來源引用，建立可延後取檔的來源物件。`config check` 走同一路徑，無需網路、有效 token 或私人活動檔。設定錯誤指向設定檔中的位置。
- **順位。** `sources()` 列舉已設定來源，列舉順序沒有選值含義。`primary_source()` 回傳發現活動及建立時間軸的主設備（預設 Bryton）。`selected_fields()` 列舉顯式覆寫的逐點欄位與整趟摘要；`sources_for(field)` 才有優先順序。預設心率為 Bryton → CIRQA、總熱量為 CIRQA。未覆寫的 record 欄位回傳主來源。CLI 只對啟用欄位所需的來源取檔，Apple Watch 留供分析。[逐欄選源表](../specification.md#strava-fit-逐欄選源表)
- **日期與配對。** `primary_query(date)` 依設定時區取得出發日的 UTC 半開區間，查詢起點落入該區間的騎乘。`candidate_query(primary_ride)` 查詢可與主活動重疊的騎乘，帶入雙向重疊門檻（預設 90%），容許提早開錄與跨日。來源負責找齊可能合格的候選。合併器以原檔時段核實雙向比例，跨當日各主活動檢查唯一性，再建立 `MatchedRide`。手動雙檔入口略過列舉，仍驗證設備與時段，並使用相同的選值策略。[日期及歧義契約](../config/README.md#只給日期時的處理契約)
- **選值。** `MatchedRide` 包含主時間軸、暫停邊界與按來源 ID 配對的唯讀紀錄。`select` 在本機對同一組輸入及有效規則產生一致結果。`SelectedRide` 可依欄位讀取有效值、缺值及來源，另提供品質判定、觸發區段與規則版本；每個選值可追到來源檔、原始欄位和取樣時間。`heart_rate()` 是讀取心率串流的例子。備援品質不足等情況回傳 `QualityReview`，保留診斷並停止該趟自動發佈。
- **目的地。** `GetTargets()`／`targets()` 依設定列舉零個以上的目的地，與 `GetSources()`／`sources()` 一樣按實例建立集合。C# 以 `IDataTarget.Id`、Rust 以 `DataTarget::id(&self) -> &TargetId` 取得設定中的目的地 ID。CLI 可據此選取指定的 target；同時列出多個目的地時仍只發佈到明確指定的一個。M2 的實際發佈驗收目標為 Strava。[發佈與重跑](merger.md#合併與交付)

## data_source　從一個來源取得一趟記錄

同一來源有三種操作。查候選回傳輕量摘要，可供顯示或縮小範圍；有了活動 ID 可直接讀 metadata；要用實際數據時才開啟該趟。下列 C# 簽名以具型別的例外回報操作失敗，Rust 以 `Result` 表達同一組結果。

```csharp
public interface IDataSource
{
    SourceId Id { get; }
    IReadOnlyList<ActivitySummary> ListActivities(ActivityQuery query);
    ActivityMetadata GetMetadata(ActivityId id);
    ActivityRecording OpenActivity(ActivityId id);
}
```

```rust
pub trait DataSource {
    fn id(&self) -> &SourceId;
    fn list_activities(&self, query: &ActivityQuery) -> Result<Vec<ActivitySummary>, SourceError>;
    fn metadata(&self, id: &ActivityId) -> Result<ActivityMetadata, SourceError>;
    fn open_activity(&self, id: &ActivityId) -> Result<ActivityRecording, SourceError>;
}
```

以指定某一趟 Garmin 活動為例，兩種語言都只開啟一次，接著便可在同一份記錄讀心率與總熱量。`StreamKey` 表示逐點欄位，`SummaryKey` 帶有 session 或 lap 範圍，兩者在選源設定中統稱 `FieldKey`。

```csharp
var recording = garmin.OpenActivity(activityId);
var samples = recording.ReadStream(StreamKey.HeartRate);
var calories = recording.ReadSummary(SummaryKey.SessionTotalCalories);
```

```rust
let recording = garmin.open_activity(&activity_id)?;
let samples = recording.read_stream(StreamKey::HeartRate);
let calories = recording.read_summary(SummaryKey::SessionTotalCalories);
```

| 契約 | 呼叫端可觀察的行為 |
|---|---|
| 身分 | `SourceId` 是設定中可自訂的來源 ID。`ActivityId` 在該來源內有效，兩者合用才能識別活動。本機 FIT 可用內容雜湊辨識，Connect 用平台活動 ID。開啟後以原始內容雜湊追溯檔案，支援改名、快取及重跑。 |
| 列舉與 metadata | `ActivitySummary` 至少能識別來源及活動，並提供可用的時段資訊。本機摘要帶實際錄製設備及名稱依據，Connect 摘要的設備要到開啟原檔後才確認。`ActivityMetadata` 提供配對所需的設備、運動種類與 UTC 起訖，線上候選最終以原檔驗證。查詢由連線器處理必要分頁。完整查詢成功且無活動時回傳空清單；目錄不可讀、憑證失效、限流或中途分頁失敗均回傳可區分的錯誤。指定 ID 查無活動則回報該 ID 不存在。 |
| 讀取串流與摘要 | 在已開啟的記錄上，以 C# `recording.ReadStream(key)`／Rust `recording.read_stream(key)` 讀取原始時間序列，以 `ReadSummary(key)`／`read_summary(key)` 讀取同名 session 或 lap 摘要及其時段。能分辨欄位缺失、單點缺值及有效零值。讀同趟的心率與熱量共用一份已開啟的記錄，無須重複下載或解碼整檔。 |
| 原始資料 | `ActivityRecording` 提供唯讀原始資料及其來源識別。FIT 輸入可查回原始 frame、未知欄位與 developer 定義，讓輸出器保存未修改欄位。Apple 健康分析來源保留 Workout 與關聯檔的來源關係。所有合併產物另存。 |

## 如何擴充

**新增設備或取檔方式。** 同一種 `fit_directory` 可建立多個具名實例。專屬目錄可省略型號篩選器。例如 Bryton Rider 15 Neo 和 Rider 460 都可屬於 `bike_meter`；列舉本機活動或開啟原檔後，可取得該份 FIT 的設備代碼、顯示名稱及名稱依據。新增取檔方式時實作 `IDataSource`／`DataSource`，把 factory 依 `kind` 編入來源註冊表。loader 以來源 ID 建立實例，factory 驗證並解讀該種類的選項。現有 Garmin 來源在本機目錄和 Connect 間切換時可沿用同一個 ID 與逐欄順位。

**新增發佈服務。** 實作 `IDataTarget`／`DataTarget` 並將 factory 編入獨立的目的地註冊表。兩個方向可以使用相同的 `kind` 名稱，各自驗證讀取與發佈能力。基礎 Schema 提供新連線器的通用設定結構，未註冊的種類由 loader 指出位置。以 A 表的離線測試註冊合成車錶、Bryton Activity 和 Garmin Connect 目的地，可驗證擴充性；實際連線器各自排入後續整合。[JSON 設定範例與相容性](../config/README.md#如何增加設備或目的地) · [A 表](testcases/module-decision-tables.md#aconfig-test)

**調整選值做法。** 合法的來源順位由 config 決定；整趟品質演算法由 `MergePolicy` 封裝。改心率停滯判準時，修改並測試該規則、更新報告中的規則版本；來源連線器仍提供同一份原始紀錄。每次處理都從唯讀輸入產生新結果，讓兩版策略可用同一趟活動對照。[心率品質決策](../decisions/2026-11-23-heart-rate-quality.md)

## 驗收邊界

| 情境 | 預期結果 |
|---|---|
| 車表未掛載、尚未提供憑證時載入有效設定 | 可列舉來源、目的地及逐欄順位。活動檔讀取、API 請求及上傳次數均為零。 |
| 同一個 Garmin 來源 ID 分別使用本機 FIT 與 Connect 測試連線器，兩者提供相同原始內容 | 使用相同配對與選值流程。各欄值、來源順位及品質判定一致；同趟的心率與熱量只取回一次原檔。 |
| 11/01 冷水坑與 11/21 使用同一個 `select` 呼叫 | 11/01 保留 18,327 個 Bryton 心率，只補 34 秒；11/21 的 14,149 個輸出心率全取 CIRQA。兩趟的 FIT 與報告均反映各自的判定。冷水坑總熱量為 1,897 kcal，Garmin 兩個原值 1,897、383 可讀回。 |
| 移除設定中的溫度，再用同一份來源紀錄選值 | 產物沿用主檔溫度及相關摘要。 |
| 跨午夜、兩個候選同時合格、備援心率不足 | 依序驗證出發日歸屬、回報歧義及 `quality_review`。遇到歧義或品質待判讀，該趟的上傳次數為零。 |

首版已接通 config check、依設定列舉本機 FIT 及手動雙檔合併。目錄來源以 SHA-256 識別活動，略過符號連結，檢查每份 FIT 的 CRC 並篩選設備及騎乘 session。目錄損壞或讀取失敗會附路徑回報。目錄列舉重建記憶體索引，按 ID 開啟時核對原始內容。已開啟的 recording 共用一次 RustyFIT 解碼，保留原始 bytes。雙檔入口可直接開啟指定檔案，沿用相同 recording 與內建策略。日期模式及 API 讀取列為後續實作。[本機來源驗證](../../README.md#驗證本機-fit-目錄來源) · [M01–M14 合併器驗收案例](testcases/merger.md)
