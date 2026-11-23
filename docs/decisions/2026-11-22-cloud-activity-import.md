# 四平台活動查詢、原生 FIT 下載與合併 FIT 發佈

## 狀態

proposed。2026-11-22 使用者確認偏好兩台裝置各自同步到廠商雲端後，由自己的 Mac 自動下載，必要憑證可在本機準備。Apple 手動全量匯出列第三順位。正式取檔方案留待制定開發規格時決定。

2026-11-23 使用者指定比較 Apple 健康、Garmin Connect、Bryton Active 與 Strava，必要功能為下載 FIT、上傳合併 FIT、查詢及讀取活動。以下依平台與工具作者公開文件整理，個人帳號的端到端行為尚待實測。

## 背景

融合需要逐筆活動檔。Bryton 與 Garmin 皆包含尚未辨識的訊息或欄位，下載活動摘要無法滿足保存及交叉驗證需求。

## 決策提案

- Garmin Connect Personal API 支援個人、自助申請、pull 模式及 FIT。從 Connect Web 建立 Personal API Key，CLI 以 key 換短期 access token，依活動清單及 activity ID 下載檔案。token 最長有效 24 小時且受 key 到期日限制，到期用 key 重換。key 可選 30、90、180 天或一年。定期輪詢可在 Mac 完成。[建立個人 key](https://connect.garmin.com/app/settings/personalApiToken) · [Personal API FAQ](https://developer.garmin.com/gc-developer-program/personal/personal-faq/) · [個人 API 端點](https://developer.garmin.com/gc-developer-program/personal/swagger-docs) · [交付模式](https://developer.garmin.com/gc-developer-program/overview/)
- Bryton 官方確認 Rider 460 可同步到 Bryton Active 及指定平台。這次查閱的公開文件沒有提供個人開發者建立 token、列舉活動並下載原始 FIT 的流程。社群專案 intervalssync 的作者記載能取得 Bryton 原始 FIT，使用 Bryton 帳號、密碼及相應區域登入。這是非官方整合，憑證型態與維護風險需另行決定。[Rider 460 官方同步說明](https://global.brytonsport.com/products/rider-460) · [社群專案](https://github.com/jorge-huxley/intervalssync)
- Apple 健康匯出作補充。日常自動取檔應允許 Bryton 或 Garmin 先到，依來源活動 ID 去重，另一份到齊後再依時間配對。Strava 與 Garmin 的憑證可由 macOS 登入鑰匙圈保管，兩者的換發與快取方式見[個人 CLI 憑證調查](../research/2026-11-23-activity-platform-automation.md#macos-如何保管個人-cli-的憑證)。

四個平台的功能比較、個人授權、重複活動處理與驗證條件，見[四平台活動自動化調查](../research/2026-11-23-activity-platform-automation.md)。Apple 逐趟匯出的工具與費用，見[Apple 健康漸進匯出調查](../research/2026-11-23-apple-health-incremental-export.md)。

## 影響

雲端取檔與 FIT 解碼器可獨立選型。取得方便程度決定日常可用的輸入，逐欄位來源優先順序仍依實測與待定規格決定。

## 替代方案

若希望只使用官方支援的帳戶介面，可先實作 Garmin 輪詢，Bryton 沿用本機 FIT 匯入。第三方平台轉存的檔案仍需與車表來源比對，尤其要核對私有欄位。

## 後續工作

本次為文件查核。後續以個人帳戶實測下載，與已保存的同趟來源 FIT 比對 SHA-256；若容器或檔案位元組不同，再逐框核對欄位、取樣與私有訊息。Bryton 是否提供可申請的官方個人介面、是否採用非官方登入，均待確認。

回寫可用 11 月 21 日修復案例驗收。保留原活動，記下各平台的活動 ID 或 HealthKit UUID，核對匯入筆數、修復區段的逐點心率、GPS 及可讀回的 developer fields，再重送一次觀察判重。若平台只能顯示部分欄位，記錄可驗證範圍；取檔流程須能排除已發佈的合併活動。
