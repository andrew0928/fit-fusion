# Strava 免費帳號、使用者授權與合作資格

查核日期為 2026-11-24。FIT Fusion 現行需求是替騎士把本機設備 FIT 合併，再以該騎士身分發佈。免費帳號可直接在網頁上傳合併檔，每批最多 15 檔，每檔上限 25 MB。先前手動匯入的 DEMO 已能在 Strava 顯示騎乘、心率、踏頻與 73 kcal。[官方網頁上傳規格](https://support.strava.com/en-us/articles/15402173-how-do-i-bulk-upload-activities-to-strava) · [專案手動驗收](../spec/testcases/merger.md)

| 階段 | 誰可以授權 | 開發者資格 |
|---|---|---|
| Standard，初始 | 只有 app 建立者 | 建立個人 app 需要有效的 Strava 訂閱。 |
| Standard，自助升級 | 上限 10 名已連結運動員，各自完成 OAuth | 開發者在 API Settings Dashboard 升級，可做小規模測試。 |
| Standard，擴大服務 | 超過 10 名，核准容量可到 9,999 | 目前容量用滿再送審，附產品資料、Connect with Strava 按鈕及所有顯示 Strava 資料的畫面，遵守 API 與品牌規範。實際容量依審核結果。 |
| Extended Access | 一般面向服務 10,000 名以上使用者的應用 | Strava 逐案核准，現行規定免開發者訂閱，並具備申請 Partner APIs 的資格。平台保留個別豁免裁量。 |

分級取自 API Policy §3.3 與開發者文件。公開說明明列 Standard 開發者訂閱門檻，條款另保留對特定終端使用者要求訂閱的權利。使用者給 OAuth 同意並不會變更 app 的容量、層級或訂閱資格。[API Policy](https://www.strava.com/legal/api_policy) · [官方分級 FAQ](https://communityhub.strava.com/developers-knowledge-base-14/strava-api-faq-12906) · [註冊與升級](https://developers.strava.com/docs/getting-started/) · [申請審核與所需畫面](https://developers.strava.com/docs/rate-limits/)

## 套用到 FIT Fusion

- 上傳的公開端點是 `POST /uploads`。若將來提供多人服務，每位騎士可授予 `activity:write`，由服務使用該騎士自己的 token 上傳。若還要從 API 核對私密活動的卡路里，才需 `activity:read_all`。OAuth 流程由服務保管 app secret，每人各自授權、可撤銷。[上傳](https://developers.strava.com/docs/uploads/) · [OAuth scopes](https://developers.strava.com/docs/authentication/)
- 審核時可描述明確的使用者功能：用自己的 Bryton、Garmin 原檔修補心率與熱量，然後發佈到自己的 Strava 帳號。服務須處理撤銷、刪除與隱私政策。若設計為向其他服務或 agent 轉供 Strava 資料或 API，會涉及 §5.3 與 §5.16 等使用限制。[API Policy](https://www.strava.com/legal/api_policy) · [撤銷事件](https://developers.strava.com/docs/webhooks/)
- 現行專案是個人使用。依已公開的條件，尚無理由預期可直接取得 Extended Access。以將來的服務構想申請是否獲准，只有 Strava 能確認。可以向 `developers@strava.com` 說明目前規模、預期人數與產品功能，詢問個案資格。[官方申請窗口](https://partners.strava.com/undefined/developers-faqs)若僅需為合併檔上傳，Standard 的公開端點已足夠。
- 穿戴設備的既有同步不受這次分級更新影響，送出的是裝置記錄。Garmin 說明它不會將從第三方網站收到的活動再轉送給另一個第三方網站。這些文件未證實能將本專案的合併 FIT 自動原樣發佈。[Strava 公告](https://communityhub.strava.com/insider-journal-9/an-update-to-our-developer-program-13428) · [Garmin 同步說明](https://support.garmin.com/en-HK/?faq=8L3Z4gEmTk6sXV7ido1Uz6)

調查當日先將 M2-4 暫緩，2026-11-25 已廢除。交付方式為 CLI 輸出 FIT，騎士在網頁上傳。[原決策](../decisions/2026-11-24-manual-strava-export.md) · [現行範圍](../decisions/2026-11-25-cli-split.md)
