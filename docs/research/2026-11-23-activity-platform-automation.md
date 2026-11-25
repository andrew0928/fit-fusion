# Apple 健康、Garmin Connect、Bryton Active 與 Strava 的活動自動化

查核日期為 2026-11-23。以下根據平台文件與工具作者說明整理。個人帳號的下載、上傳與判重行為尚待實測。

Garmin Connect 的官方個人 API 提供 FIT 下載與活動查詢，Strava 的官方 API 提供 FIT 上傳與活動查詢。Bryton Active 的雲端取檔有社群工具可參考。Apple 健康可經 iPhone 上的 HealthKit 讀寫運動，Apple Watch 內建「體能訓練」需轉製才有 FIT 檔。

## 三項必要功能

範圍是個人帳號授權後，以程式處理自己的已完成活動。「下載」要求取得裝置錄製的 FIT，並能核對原始位元組與未知欄位。「上傳」指提交 `merged.fit` 以建立已完成的騎乘。「查詢／讀取」涵蓋列舉活動、取得單筆明細及可用的感測器序列。

| 平台 | 下載原生 `.fit` | 上傳合併的 `.fit` | 查詢／讀取活動 |
|---|---|---|---|
| Apple 健康（Watch 內建「體能訓練」） | 無原生 FIT 可取。健康 App 匯出 XML，本專案三趟樣本另含 GPX。HealthFit 可將運動轉製成 FIT。[Apple 匯出](https://support.apple.com/zh-tw/guide/iphone/iph5ede58c3d/27/ios/27) · [HealthFit 檔案支援](https://altifondo.com/privacy-google/healthfit-privacy.html) | 需經 iPhone App 解讀 FIT，將運動、心率、距離及 GPS 路線寫入 HealthKit。HealthFit 支援開檔匯入，RunGap 提供付費背景自動分享，執行時間由 iOS 排程。FIT 私有欄位仍需另存。[寫入路線](https://developer.apple.com/documentation/healthkit/creating-a-workout-route) · [RunGap 自動分享](https://rungap.zendesk.com/hc/en-us/articles/115004348594-Using-Auto-Sharing-with-RunGap) | iPhone App 可透過官方 HealthKit 查詢運動、樣本與關聯路線，也能接收背景通知。[背景通知](https://developer.apple.com/documentation/healthkit/executing-observer-queries) · [讀取路線](https://developer.apple.com/documentation/healthkit/reading-route-data) |
| Garmin Connect | 官方 Personal API 可依活動 ID 下載 ZIP 內的 FIT。與 CIRQA 裝置原檔的一致性待驗。[官方 CLI](https://github.com/garmin/connect-personal-api-python) | 官方個人方案僅提供 Pull，官網可手動匯入 FIT。另行登入 Connect 後，可用社群套件 `python-garminconnect` 的 `upload_activity()` 自動上傳。[官方方案](https://developer.garmin.com/gc-developer-program/overview/) · [手動匯入](https://support.garmin.com/en-CA/?faq=Ht3ZP52Kju075uKvqTqu99) · [社群套件](https://github.com/cyberjunky/python-garminconnect) | 官方 CLI 的 `gc-personal activities` 可依日期列舉，`gc-personal activity <ID>` 讀取單筆，`gc-personal activity-download <ID>` 可取檔解析逐點資料。[官方 CLI](https://github.com/garmin/connect-personal-api-python) |
| Bryton Active（Rider 460） | 車表可自動同步到 Active。社群工具 intervalssync 記載可從雲端下載原始 FIT；可由 USB 複製車表檔案作核對基準。[取檔流程](https://github.com/jorge-huxley/intervalssync/blob/master/docs/AGENT.md) · [Rider 460 手冊](https://global.brytonsport.com/pages/support_downloadum_r460) | 官網可手動上傳 FIT。未查到供個人自動回寫已完成活動的公開 API。intervalssync 回傳 Bryton 的資料是預定課表。[Rider 460 手冊](https://global.brytonsport.com/pages/support_downloadum_r460) · [intervalssync 支援方向](https://github.com/jorge-huxley/intervalssync) | Active 可查看活動。程式可透過 intervalssync 所用的非官方介面列舉，按 ID 取 FIT 後解析明細與逐點資料。[社群工具流程](https://github.com/jorge-huxley/intervalssync/blob/master/docs/AGENT.md) |
| Strava | 官網活動頁可用 **Export Original** 取回當初上傳的格式，來源為 FIT 才得到 FIT。公開 API 未列原檔下載端點。社群工具 strava-backup 記載可透過登入網站取檔，目前可用性待驗。[官方匯出](https://support.strava.com/en-us/articles/15401919-how-do-i-export-my-strava-data) · [社群工具](https://github.com/pR0Ps/strava-backup) | 個人自建 app 的 Standard Tier 需要開發者訂閱，免費帳號可網頁上傳。[帳號資格](2026-11-24-strava-developer-access.md) 官方 `POST /uploads` 接受 FIT，需 `activity:write`。以 `GET /uploads/{uploadId}` 查詢處理結果，取得活動 ID 或錯誤。[Uploads API](https://developers.strava.com/docs/uploads/) | 官方 API 可列舉自己的活動、按 ID 讀明細，並取得該活動可用的時間、GPS、心率等 streams。需 `activity:read`，包括「只有自己」的活動則用 `activity:read_all`。[API 參考](https://developers.strava.com/docs/reference/) · [授權範圍](https://developers.strava.com/docs/authentication/) |

## 個人帳號如何取得來源檔

- Garmin Connect Personal API 支援個人、自助申請、pull 模式及 FIT。從 Connect Web 建立 Personal API Key，CLI 以 key 換短期 access token，依活動清單及 activity ID 下載檔案。token 最長有效 24 小時且受 key 到期日限制，到期用 key 重換。key 可選 30、90、180 天或一年。定期輪詢可在 Mac 完成。[建立個人 key](https://connect.garmin.com/app/settings/personalApiToken) · [Personal API FAQ](https://developer.garmin.com/gc-developer-program/personal/personal-faq/) · [個人 API 端點](https://developer.garmin.com/gc-developer-program/personal/swagger-docs) · [交付模式](https://developer.garmin.com/gc-developer-program/overview/)
- Bryton 官方確認 Rider 460 可同步到 Bryton Active 及指定平台。這次查閱的公開文件沒有提供個人開發者建立 token、列舉活動並下載原始 FIT 的流程。社群專案 intervalssync 的作者記載能取得 Bryton 原始 FIT，使用 Bryton 帳號、密碼及相應區域登入。這是非官方整合，憑證型態與維護風險需另行決定。[Rider 460 官方同步說明](https://global.brytonsport.com/products/rider-460) · [社群專案](https://github.com/jorge-huxley/intervalssync)

Apple 的逐趟 FIT 匯出、資料權限、費用及免年費方案，見[Apple 健康漸進匯出調查](2026-11-23-apple-health-incremental-export.md)。

## macOS 如何保管個人 CLI 的憑證

個人 Mac 上的 Rust CLI 可將秘密存入使用者的本機登入鑰匙圈（login keychain），以平台與帳號識別 generic password 項目，讓 macOS 控制讀取權限。Rust 的 `security-framework` 已提供建立、更新、查詢及刪除介面。此處評估的是一般命令列程式能使用的檔案式登入鑰匙圈，個人本機使用無須付 Apple Developer 年費。iCloud 同步與跨 App 的 keychain access group 有各自的簽署和佈建條件。[Apple TN3137](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains) · [Rust 介面](https://docs.rs/security-framework/latest/security_framework/passwords/index.html)

| 帳號 | 建議存入登入鑰匙圈 | CLI 自動執行時 |
|---|---|---|
| Strava 個人 OAuth App | App 的 `client_secret`、該運動員最新的 `refresh_token`。若要跨程序重用短效 `access_token`，可將它與到期時間及 refresh token 一起保存在同一筆使用者憑證。 | 首次用瀏覽器授權。Access token 有效 6 小時。每次換發後保存回應中的 refresh token，一旦輪替，舊值立即失效。手動與排程同時執行時，同帳號的換發與回存應依序完成。[Strava OAuth](https://developers.strava.com/docs/authentication/) |
| Garmin Connect Personal API | 使用者建立的 Personal API Key。短效 bearer token 可只放程序記憶體，需要跨次重用時再放鑰匙圈。 | 用 key 向 Garmin 換發 token。Token 最長 24 小時，且受 key 到期時間限制。這個方案沒有 refresh token；key 本身到期後由本人在 Connect 建立新 key。[Garmin Personal API FAQ](https://developer.garmin.com/gc-developer-program/personal/personal-faq/) |

若沿用 Garmin 官方 `gc-personal`，它會把 bearer token 明文存入 `~/.config/garmin/connect-personal-api-bearer-token.json`，現版無法關閉該快取。若要讓權杖只出現在鑰匙圈或記憶體中，Rust CLI 可直接呼叫官方換發及活動端點。[Garmin 官方 CLI 快取說明](https://github.com/garmin/connect-personal-api-python#bearer-token-cache)

背景排程可在同一個登入使用者的 LaunchAgent 中執行，首次存取若出現提示，可對指定的 CLI 授權。若鑰匙圈已鎖定或程式更新後需要再次授權，背景工作應回報需要解鎖或重新授權，避免把秘密寫入設定檔、命令列參數或日誌。此方案尚未用本人的鑰匙圈或帳號做端到端驗證。[Apple 存取控制](https://support.apple.com/en-gb/guide/mac-help/kychn002/26/mac/26) · [Apple TN3137](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains)

## 同趟活動已存在時

- **Apple 健康。** RunGap 會跳過起始時間完全相同的運動。其他匯入程式須核對活動來源及是否重複計算。[RunGap 的 Apple Health 行為](https://rungap.zendesk.com/hc/en-us/articles/222528287-Using-Apple-Health-with-RunGap)
- **Garmin Connect。** 第三方平台曾收到過濾掉未公開欄位的 FIT，Personal API 下載的檔案需獨立比對。RunGap 辨識為既有活動時只更新名稱與類型，合併後的心率應讀回逐點資料確認。[第三方整合者收到的 Garmin 答覆](https://forums.garmin.com/developer/fit-sdk/f/discussion/432872/discrepancies-between-fit-files-from-garmin-connect-and-api-api-downgraded/2018354?pifragment-1304=2) · [RunGap 的 Garmin 行為](https://rungap.zendesk.com/hc/en-us/articles/222528187-Using-Garmin-Connect-with-RunGap)
- **Bryton Active。** 手冊未說明原騎乘已同步時，網站會接受、略過或另建合併檔，也未承諾保留 developer fields。須用樣本驗證。[Rider 460 手冊](https://global.brytonsport.com/pages/support_downloadum_r460)
- **Strava。** 自建 API App 需有 Strava 訂閱，新 App 預設可授權自己的帳號。若裝置已將同趟騎乘同步到 Strava，再上傳合併檔可能收到 duplicate。成功時以回傳的活動 ID 讀取心率等 streams，核對修復區段。平台 streams 只涵蓋其支援的欄位。[開發者資格](https://developers.strava.com/docs/getting-started/) · [上傳狀態與重複錯誤](https://developers.strava.com/docs/uploads/) · [活動 streams](https://developers.strava.com/docs/reference/#api-Streams-getActivityStreams)

## Rider 460 騎後免接線取檔（2026-11-25 調查）

Rider 460 手冊第 9 頁說明，將車錶與 iPhone 的 Bryton Active 配對，並開啟「Activity Auto Sync」後，新的騎乘紀錄會經 BLE 同步。Mac 可在活動進入 Bryton Active 雲端後下載。手冊將電腦直接讀取活動目錄的操作列在 USB 一節。本次未找到 Rider 460 讓 Mac 經藍牙直接列舉、下載 FIT 的公開協定或已驗證工具，也未找到此機型以家中 Wi-Fi 自行上傳的操作說明。[Rider 460 官方手冊](https://global.brytonsport.com/pages/support_downloadum_r460)

核對 intervalssync v0.9.5 原始碼，標準 CLI 的 Bryton 同步固定開啟 intervals.icu 上傳。`--download-dir` 和 `--keep-files` 控制本機副本。底層 Bryton 模組則可用自己的 Bryton 帳號登入、列舉活動與按 ID 取得 FIT。將 `download_fit=True`、`upload_intervals=False`、`upload_dropbox=False`、`intervals_api_key=None` 傳入 `SyncConfig`，即可走純下載分支。這是第三方套件的內部介面，Bryton 雲端存取依賴非官方介面。[CLI 參數與組態](https://github.com/jorge-huxley/intervalssync/blob/v0.9.5/src/intervalssync/cli/main.py) · [Bryton 核心流程](https://github.com/jorge-huxley/intervalssync/blob/v0.9.5/src/intervalssync/bryton/core.py) · [FIT 下載實作](https://github.com/jorge-huxley/intervalssync/blob/v0.9.5/src/intervalssync/bryton/api.py)

可行性驗證可先取一筆已有 USB 基準檔的活動，比對 SHA-256。下一趟保留 iPhone 鎖屏及平常的 App 狀態，記下車錶存檔、雲端出現活動、Mac 收到檔案的時間。Mac 可用 LaunchAgent 定期查詢，以活動 ID 跳過已完整下載的檔案，再將目錄交給現有的 `fit-query-files`。上游純下載分支預設取最近五筆，並不依本機檔案去重。這些結果來自手冊及原始碼，個人帳號、檔案一致性與背景同步仍待實測。

## 待實測項目

1. 取得同趟 Garmin 與 Bryton 的雲端 FIT，解開 ZIP 後分別與已保存的裝置來源檔比對 SHA-256。若有差異，再逐框核對取樣時間、欄位值、私有訊息與 developer fields。
2. 使用 11 月 21 日騎乘的修復檔驗證回寫。保留原活動，記錄平台回傳的活動 ID 或 HealthKit UUID，讀回修復區段的心率與 GPS，核對實際可讀欄位。再次送檔時觀察是新增、更新、略過或回報重複。[騎乘修復紀錄](rides/2026-11-21-ride.md)
3. 查詢新活動時，驗證能排除剛發佈的合併活動，避免同一趟被再次當成待融合的來源。

專案的來源偏好與取檔提案見[個人雲端活動取檔決策](../decisions/2026-11-22-cloud-activity-import.md)。
