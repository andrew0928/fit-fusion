# Bryton 騎乘上傳 Strava 後的熱量差異

2026-11-23。使用者表示平常以 Bryton 的記錄上傳 Strava。實檔確認，11 月 1 日冷水坑的 Bryton FIT 已記錄 6,186 kcal，Strava 寫回 Apple 健康的同趟活動也是 6,186 kcal。高值可追溯到上傳前的車表檔案。造成車表估算偏高的設定或算法仍待確認。

## Bryton 官方說明與車友回報

| 來源 | 公開資料提到什麼 | 對 Rider 460 的判讀 |
|---|---|---|
| Bryton 台灣「車錶教室」及 Rider 460 手冊 | 官方說明以功率計的實測作功來估算熱量，僅有心率或速度時，誤差會顯著增加。460 手冊將熱量（kcal）與累積機械作功（kJ）列為不同欄位。 | 這是估算限制。本次查閱的官方公開資料未找到將 Rider 460 熱量高估列為已知韌體問題或指明修正版本的公告。[Bryton 能量消耗說明](https://www.brytonsport.com.tw/blogs/article/bryton-energy-expenditure) · [Rider 460 手冊，第 42 頁](https://cdn.shopify.com/s/files/1/0714/6687/7176/files/Rider460_UM_EN.pdf?v=1725258596#page=42) |
| Mobile01 車友，2022-09-15 | 原作者回報同一路線使用 Bryton Rider 420 得到 2,980 kcal，Garmin 500 為 1,605 kcal，約相差 1.86 倍。 | 是不同裝置間出現明顯差距的第一手比較。原文提供的兩個數字未區分活動熱量與總熱量。[〈請教 Bryton 碼表卡路里問題〉](https://www.mobile01.com/topicdetail.php?f=318&t=6662900) |
| Reddit r/cycling 車友，2021 年 | 同趟 MTB 騎乘，Rider 420 有連接心率與踏頻感測器，記錄 2,000 kcal；Garmin vívoactive 3 Music 顯示 1,550 kcal。作者也詢問兩台是否採用相同的熱量定義。 | 同樣觀察到 Bryton 較高，但兩台的記錄啟動方式不同，也沒有獨立量測消耗。[車友原文](https://www.reddit.com/r/cycling/comments/lz0rlv/bryton_calories_calculation_actives_or_totals/) |

這兩份具體數字來自 Rider 420。本案對 Rider 460 的直接證據是下列六份車表 FIT，均未辨識到有效的逐點功率。本批資料可以定位高值出現在哪一端，尚無法辨識車表當時採用了哪個公式或證實某個韌體缺陷。

## 六趟實檔的比對

逐筆比對 Bryton Rider 460 與 Garmin CIRQA 原始 FIT 的 `session.total_calories`，再以騎乘時間配對 Apple Watch 的 Workout，以及 Connect、Strava 在 Apple 健康中各自寫入的 Workout。三裝置共同時段涵蓋各趟 Bryton 記錄時長的 96.95% 至 100%（四捨五入）。下表採各裝置完整活動摘要。Strava 欄取自 Apple 健康匯出中由 Strava 寫入的活動熱量，空白表示這份匯出沒有同趟 Strava 副本。

| 騎乘日期 | Bryton FIT 摘要（kcal） | Strava 寫回健康（kcal） | Garmin 總熱量（kcal） | Garmin 活動熱量（kcal） | Apple Watch 活動熱量（kcal） |
|---|---:|---:|---:|---:|---:|
| 10/09 | 2,994 | 2,994 | 844 | 552 | 436.835 |
| 10/14 | 3,207 | 3,207 | 1,024 | 814 | 809.649 |
| 10/28 | 816 | | 994 | 785 | 779.599 |
| 11/01 冷水坑 | 6,186 | 6,186 | 1,897 | 1,514 | 1,189.2 |
| 11/08 | 2,220 | | 1,820 | 1,442 | 1,191.75 |
| 11/21 | 4,000 | | 1,272 | 972 | 973.88 |

冷水坑的 Garmin FIT 同時提供總熱量 1,897 與 `metabolic_calories` 383；相減得到 1,514 kcal，與 Connect 寫入 Apple 健康的活動熱量吻合。六趟皆可用同一方法核對。冷水坑 Apple Watch 的運動摘要為活動熱量 1,189.2 加基礎熱量 395.088，共 1,584.288 kcal。這些數值說明「約 1,500」可能指不同的熱量定義。11 月 21 日 Bryton 的 4,000 與兩支手錶的活動熱量也明顯分歧，但 10 月 28 日的三個值很接近，這批資料不支持對 Bryton 一律除以四。

每趟的原始路徑、來源、精確起訖與重疊秒數收錄於本機 `data/index/rides.csv`，FIT 摘要與分圈欄位在 `data/index/fit-fields.json`。Strava 數字取自同步副本。使用者於 M2 設計確認直接發佈 CIRQA 的原始總熱量，另保留 Garmin 兩個原值。[已確認的熱量來源決策](../decisions/2026-11-23-ride-calorie-source.md)

## Strava 從哪裡取得熱量

冷水坑雙機原檔的 session 原生欄位 11 都是 `total_calories`，Bryton 為 6,186、Garmin 為 1,897。Garmin 原檔另有欄位 196 `metabolic_calories`，值為 383，Bryton 沒有此欄。Garmin 稱單次運動的整段消耗為 Activity Calories，含活動與靜息消耗。扣除代謝的 1,514 kcal 是 Active Calories。Strava 的公開 FIT 表列出 `session.total_calories`，未列 `session.metabolic_calories` 或本案的來源副本，亦未指定供應商的運算公式。這支持發佈 Garmin 同名欄位並在本機分別保存原值與衍生值。[Garmin 熱量名詞](https://support.garmin.com/en-CA/?faq=lkl4cwCLlK7ox362uGQEV7) · [Strava 官方欄位表](https://developers.strava.com/docs/uploads/) · [開發規格逐欄對照](../specification.md#strava-fit-逐欄選源表)

Strava 說明，若上傳來源提供熱量，平台採用來源提供的數字。針對 FIT，Strava 的欄位表明列會讀取 `session.total_calories`。因此第一個要核對的是實際上傳檔的騎乘 session 中有沒有有效的總熱量，其單位為 kcal。分圈的 `lap.total_calories` 可獨立交叉核對。整趟與所有分圈描述同一份消耗，兩者相加會重複計算。[Strava 熱量說明](https://support.strava.com/en-us/articles/15402159-how-does-strava-calculate-calories) · [Strava FIT 匯入欄位](https://developers.strava.com/docs/uploads/) · [專案 FIT Profile 欄位表](../../src/python/fit_fusion/profile.tsv)

若上傳來源沒有提供熱量，Strava 對騎乘依功率與人體效率係數估算。有功率計時能採用量測功率；缺少功率時，估算會用到騎士體重、車重、速度與高程變化。對已採用第三方熱量的活動，修改 Strava 的體重或活動海拔不會改變該熱量。Rider 460 手冊把 Calories 定義為總消耗，另列 Kilojoules 為累積機械作功。手冊該欄位說明沒有給出熱量算法，無法從四倍差距推定是單位轉換錯誤。[Strava 熱量說明](https://support.strava.com/en-us/articles/15402159-how-does-strava-calculate-calories) · [Strava 估算功率](https://support.strava.com/en-us/articles/15401944-how-to-get-power-for-your-rides) · [Bryton Rider 460 手冊，第 42 頁](https://cdn.shopify.com/s/files/1/0714/6687/7176/files/Rider460_UM_EN.pdf?v=1725258596#page=42)

## 用一趟騎乘定位差異

優先準備當時上傳的 Bryton FIT，以及同一筆 Strava 活動頁上的確切熱量與單位。若手邊沒有當時的檔案，可在該筆活動的網頁選單選 **Export Original**，取回上傳的原格式。從車表複製的 FIT 另存，若兩份都有，可比對是否一致。若要選修正值，再補同趟 Garmin FIT 或 Apple Watch 運動摘要，附起訖時間，以及「活動熱量」或「總熱量」標籤即可。[Strava 原檔匯出](https://support.strava.com/en-us/articles/15401919-how-do-i-export-my-strava-data)

| 實際上傳的 FIT | 可以判斷什麼 | 如何修正或進一步核對 |
|---|---|---|
| 單一騎乘的 `session.total_calories` 約 6,000，Strava 也約 6,000 | 高值在上傳之前就存在。若車表原檔也是同值，可追溯到車表輸出。 | 核對車表個人資料、體重單位、該次記錄時間及感測器異常。在同時間、同熱量定義下選定參考來源，只在另存的 `merged.fit` 寫入經確認的數值。 |
| 該 session 缺少有效熱量 | 符合 Strava 對騎乘啟用估算的條件。 | 查 Strava 的 `device_watts`、平均功率、累積作功、體重、車重及爬升。若採用其他裝置的熱量，可在合併檔明確提供該趟摘要，再讀回 Strava 的結果。 |
| 該 session 約 1,500，Strava 卻約 6,000 | 兩端不符，僅據此無法確定算法。 | 先確認活動 ID、上傳原檔、kcal 單位與 session 數。逐一比對分段和分圈，檢查網站數字是否為 Calories，並與 API 回傳的 `calories` 交叉核對。 |

Strava 活動明細的 `calories` 單位為 kcal。`kilojoules` 表示騎乘機械作功，`device_watts` 標示平台所用功率是否來自功率計。平台沒有公開每筆活動的熱量來源旗標。Apple 的活動熱量與基礎熱量則各自有區間，可供融合時保留。[Strava 活動明細](https://developers.strava.com/docs/reference/#api-models-DetailedActivity) · [三趟裝置比較](rides/2026-11-three-device-comparison.md)

## 合併檔與既有 Strava 活動

後續實作用 11 月 1 日驗證原始總量 1,897、代謝量 383 及報告中的差值 1,514，用 10 月 28 日驗證沒有套用固定修正倍率。發佈前核對騎乘時段，選用 Garmin 同名總熱量。報告與 Parquet 應保存 Bryton 原值、採用值、單位、來源和時間範圍。若檔案帶分圈熱量，須有對應各圈的資料才能採用，任何無法對齊的圈都應在報告標示。

公開的 Strava 活動更新 API 沒有可直接覆寫 `calories` 的參數，上傳同趟修正檔也可能被判為重複。這次可先保留既有活動，只用原檔判讀；後續新騎乘若選擇發佈合併檔，應在原裝置自動同步至 Strava 之前決定發佈哪個版本，並以匯入後的活動 ID 讀回熱量。[Strava 活動更新](https://developers.strava.com/docs/reference/#api-models-UpdatableActivity) · [Strava 上傳狀態](https://developers.strava.com/docs/uploads/)

合成室內騎乘已完成 Strava 網頁手動匯入。Bryton 輸入為 410 kcal，Rust 合併檔的標準總熱量為 CIRQA 的 73 kcal，Strava 活動概觀顯示 73 kcal。[匯入驗收紀錄](../spec/testcases/merger.md#strava-網頁手動匯入實測)
