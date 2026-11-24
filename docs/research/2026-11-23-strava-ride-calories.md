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

## 2026-11-24　冷水坑的 kJ 與 kcal 是否混用

騎士觀察到冷水坑 Strava 頁面的作功數字接近 Garmin 的熱量。這裡有兩個可以分開檢驗的問題。

1. **Strava 的機械作功 kJ 為什麼會接近 Garmin 的 kcal？** Bryton 官方以 150 W 騎兩小時為例，踏板輸出 1,080 kJ，估計人體消耗約 1,080 kcal。同一份能量的換算為 1 kcal = 4.184 kJ，但人體轉為踏板作功的效率約為 20–25%。若以 24% 示範，1,080 ÷（4.184 × 0.24）約為 1,076 kcal。這說明不同物理量可有相近讀數。官方也說明只靠心率或速度估算時誤差較大。[Bryton 能量消耗說明](https://www.brytonsport.com.tw/blogs/article/bryton-energy-expenditure)
2. **Rider 460 是否把以 kJ 算出的代謝能量直接填進 FIT 的 kcal 欄？** 冷水坑 Bryton 原檔的標準 `session.total_calories` 是 6,186 kcal。假設這個原始數字其實是代謝能量 kJ，除以 4.184 得到 1,478.5 kcal，比 CIRQA 活動熱量 1,514 低 2.35%，比 CIRQA 總熱量 1,897 低 22.1%。Garmin 同趟的代謝欄位為 383 kcal，1,514 是由總量扣除此值而來。Garmin 也是估算來源，不能充當單位標定基準。

以騎乘索引重算，Bryton 原值除以同趟 Garmin **活動熱量**的比值如下。固定單位換算預期倍率接近 4.184。

| 日期 | 10/09 | 10/14 | 10/28 | 11/01 | 11/08 | 11/21 |
|---|---:|---:|---:|---:|---:|---:|
| Bryton / Garmin 活動熱量 | 5.42 | 3.94 | 1.04 | 4.09 | 1.54 | 4.12 |

三趟接近該倍率，也有一趟接近 1。這批資料不足以採用固定修正因子，仍可保留「特定條件下估算或寫出有誤」作為待驗假設。

**冷水坑原檔核對。** Bryton 的 `session.total_work` 標準單位為 J，原值為 `0xFFFFFFFF`（uint32 無效碼）。CIRQA 沒有這個 session 欄位。兩份 FIT 的 `record.power` 有效筆數皆為 0。Apple 健康中的 Strava 副本只列活動熱量 6,186 kcal 與騎乘距離。冷水坑 Strava 頁面顯示的確切作功值仍待核對。Strava API 將 `kilojoules` 定義為騎乘作功、`calories` 定義為消耗熱量，`device_watts` 可辨別量測與估算；平台在無功率計時可用體重、車重、速度與高程估算功率。[Strava 活動欄位](https://developers.strava.com/docs/reference/#api-models-DetailedActivity) · [Strava 功率估算](https://support.strava.com/en-us/articles/15401944-how-to-get-power-for-your-rides) · [Strava 熱量來源](https://support.strava.com/en-us/articles/15402159-how-does-strava-calculate-calories)

**第一手案例。** 本次查閱可找到 Bryton 數值偏高、偏低及車友直接質疑單位混用的案例，未找到 Bryton 官方確認 Rider 460 有此單位缺陷的公告。

| 來源 | 車友實測或提問 | 參考價值 |
|---|---|---|
| [Mobile01，Rider 420 與 Garmin 500](https://www.mobile01.com/topicdetail.php?f=318&t=6662900) | 相同路線，Bryton 2,980 kcal，Garmin 1,605 kcal。 | 車友觀察到 Bryton 高出約 1.86 倍。原文未統一活動／總熱量口徑。 |
| [Reddit，Rider 320 搭配 4iiii 功率計](https://www.reddit.com/r/Strava/comments/mw15w9/calories_way_lower_than_total_work/) | 作者上傳 Strava 後看到一趟作功逾 200 kJ、熱量約 100 kcal，另一趟爬坡為 900 kJ、400 kcal。 | 直接比較同一活動的作功與熱量，數值差異方向與冷水坑不同。 |
| [TrainerRoad，直接討論 Strava 是否將 joules 當成 calories](https://www.trainerroad.com/forum/t/strava-tr-calculated-calories-from-power-meter-data-incorrect-joules-given-as-calories/87500) | 原作者因作功 kJ 與顯示 kcal 接近而懷疑高估約 4.2 倍，回覆指出踏板作功與人體消耗之間還有機械效率。 | 重現相同的單位疑問，討論對象是功率計與平台。 |

騎士已確認當天沒有功率計。Strava 顯示的作功數字可用來辨認畫面所指的物理量，但不能作為實測功率來校準熱量。現有合併規則採同趟 CIRQA 的 `session.total_calories`。[整趟熱量來源決策](../decisions/2026-11-23-ride-calorie-source.md)

## 2026-11-24　用同趟手錶紀錄與騎乘強度檢驗熱量

**同趟、同口徑的比較。** 以兩份原生 FIT 的 `session.total_calories` 對照 Apple Watch 自行記錄的 Workout，將其活動熱量與基礎熱量相加。六趟共同時段占 Bryton 記錄的 96.95% 至 100%，下表使用各裝置完整摘要。三個來源皆為裝置估值，與 Apple Watch 的差距衡量裝置間的一致性。

| 日期 | Apple Watch 總熱量（kcal） | Garmin 總熱量（kcal） | Bryton 總熱量（kcal） |
|---|---:|---:|---:|
| 10/09 | 724 | 844 | 2,994 |
| 10/14 | 1,024 | 1,024 | 3,207 |
| 10/28 | 998 | 994 | 816 |
| 11/01 冷水坑 | 1,584 | 1,897 | 6,186 |
| 11/08 | 1,582 | 1,820 | 2,220 |
| 11/21 | 1,316 | 1,272 | 4,000 |

六趟中 Garmin 每趟都較接近 Apple Watch。用未取整的 Apple Workout 數值計算，每趟絕對差的中位數是 Garmin 82 kcal、Bryton 2,227 kcal。冷水坑 Garmin 比 Apple Watch 高約 20%，Bryton 約為 Apple Watch 的 3.9 倍。若要比較運動額外消耗，則用 Garmin 活動熱量 1,514 對 Apple 活動熱量 1,189 kcal。六趟數值可用本機 `data/index/rides.csv` 的原檔路徑重算。

**體重與時間的交叉檢查。** 成人活動量表以 1 MET 約等於每公斤體重每小時 1 kcal。以各裝置的總熱量，除以騎士體重及各自全程的小時數，得到冷水坑的隱含平均強度。Apple Watch 約 4.7 MET、Garmin 約 5.6 MET、Bryton 約 18.7 MET。量表給自選中等速度騎乘 7 MET、劇烈上坡登山車 14 MET、登山車競賽 16 MET。這些是族群參考值。冷水坑全程約五小時，包含上坡、下坡與停留，Bryton 數字要求全時段平均強度高過上述競賽參考值。[MET 定義](https://pacompendium.com/) · [2024 成人騎車活動量表](https://pacompendium.com/bicycling/)

Bryton 原檔距離 48.31 km，經過 18,367 秒。逐秒速度大於 1 m/s 的記錄有 14,164 筆，約 3 小時 56 分。若給這段時間 7 MET，其他時間以 1 MET 示範，全程合計約 28.7 kcal/kg，接近 Garmin 的數量級。速度門檻只供這項估算使用，其餘時間含低速及缺值。各段強度均屬假設，無法從這項試算得出個人熱量的精確值。

**車重與爬升的交叉檢查。** 當天 Bryton 與 CIRQA 的 `session.total_ascent` 分別為 854 與 1,031 m，Apple Watch Workout 的 `HKElevationAscended` 為 923.84 m。用人車合重乘以重力加速度與累積爬升（`m × g × h`），可估出克服重力的機械能，再依效率估算此部分對應的代謝能量。兩項實測研究分別報告上坡平均總效率 18.7% 與跨騎姿、地形平均 22.4%。以騎士提供的人車合重及 Apple 的爬升量示範，光是提升高度約對應 725–868 kcal，整趟另有滾動與空氣阻力等成本。此項試算說明爬升的能量規模，無法單獨驗證整趟總值，也無法判定 Bryton 的運算錯在何處。[Nimmerichter 等人的戶外爬坡研究](https://pubmed.ncbi.nlm.nih.gov/25611890/) · [Millet 等人的平路與上坡實測](https://pubmed.ncbi.nlm.nih.gov/12370567/)

## 2026-11-24　Apple Watch 與 CIRQA 的熱量能驗證到什麼程度

**研究結論與適用範圍。** 六趟騎乘與分圈紀錄顯示，兩台裝置的活動熱量差距隨騎乘和路段而變。目前缺少騎士本人的獨立代謝量測，無法判定冷水坑哪台更準，也缺乏套用固定修正倍率的依據。騎士於 2026-11-24 確認這些結果供分析使用，合併沿用已定案的逐欄順位。整趟熱量採同趟 CIRQA 的原始 `session.total_calories`，Apple Watch 熱量供交叉核對。[資料流順位表](../specification.md#strava-fit-逐欄選源表) · [熱量來源決策](../decisions/2026-11-23-ride-calorie-source.md)

**六趟的活動熱量。** 由本機 `data/index/rides.csv` 找出 Apple Watch 自己記錄的 Workout，取其活動熱量。Garmin 取原始 FIT 的總熱量減去代謝欄位。以 Apple 為分母，按 10/09、10/14、10/28、11/01、11/08、11/21 排列，Garmin 相對差依序為 +26.4%、+0.5%、+0.7%、+27.3%、+21.0%、−0.2%。三趟相差不到 1%，另三趟 Garmin 高出 21–27%。這是裝置間的差距，各裝置依自身完整記錄時段計算。

**冷水坑的差距從哪裡來。** Garmin 的活動熱量比 Apple 高 324.8 kcal（1,514 對 1,189.2），靜息部分反而低 12.1 kcal（383 對 395.1）。兩支裝置記錄時長相差約 73 秒。在三台皆有心率的 3,663 個同秒樣本中，Apple 與 CIRQA 心率的平均絕對差為 1.13 bpm。心率相近仍可得到不同熱量，現有資料無法確定裝置內部造成差距的機制。前述 MET 試算也無法區分兩者。沿用相同移動時間及其餘時間 1 MET 的假設，把移動時段取為 5.8 或 7 MET，整趟總熱量分別約為 1,559 或 1,866 kcal，依序接近 Apple 的 1,584 與 Garmin 的 1,897 kcal。[同秒心率分析](rides/2026-11-three-device-comparison.md#三台同秒讀數的差異) · [成人騎乘 MET 參考表](https://pacompendium.com/bicycling/)

**同一趟內也會變換方向。** 10/09 的 Garmin FIT 有五個分圈。對照每圈的活動熱量與 Apple Watch 同一時段的活動熱量樣本，第一圈 Garmin 為 147 kcal、Apple 約 92 kcal，第二圈則是 Garmin 94 kcal、Apple 約 113 kcal。Apple 各圈可分別以「完整落入圈內的樣本加總」及「所有與該圈重疊的樣本加總」作為邊界，前兩圈分別為 92.23–92.48 與 113.33–113.52 kcal。邊界處理不影響差距方向。冷水坑的 Garmin FIT 只有一個整趟分圈，缺少可辨識的逐點熱量，無法用相同方法定位路段。

**外部研究與個人驗證。** 一項固定式自行車間歇研究以 COSMED K5 間接熱量測定為參考，Apple Watch Series 8 與 Garmin Forerunner 955 在該測試的平均偏差（裝置減參考）分別為 +21.6 與 +68.6 kcal。型號、受試者與運動時間皆不同，該結果不能用來校正個人的 Series 9 或 CIRQA。Series 9 本身也有間接熱量測定研究，採用的是 10 分鐘跑步機測試。這次查詢未找到以呼吸氣體為參考、同時比較 Series 9 與 CIRQA 騎乘熱量的研究。[自行車研究與測試方法](https://doi.org/10.1371/journal.pone.0353261) · [Series 9 跑步機研究](https://pubmed.ncbi.nlm.nih.gov/39728259/)

若要驗證騎士自己的兩支裝置，可在運動科學實驗室同時佩戴兩者，以固定式自行車和呼吸氣體分析同步記錄平常的輕鬆及中等強度。先測靜息基準，兩支裝置用相同個人資料、同步起訖及暫停設定。整段氣體量測所得消耗與裝置總熱量相比，扣除等時長靜息消耗後再與活動熱量相比，各自列出相差幾 kcal 和百分比。能在不同天重測，更能看出偏差是否穩定。較低門檻的做法是借有量測功率、已校準的訓練台，在相同實測作功下重複比較兩支裝置；個人踩踏效率仍是未知數，只能用此法檢查重複性與消耗的合理範圍。[踏頻、地形對實測效率的影響](https://pubmed.ncbi.nlm.nih.gov/25611890/)

舊車錶的同路線檔與 Rider 460 的同機對照，見 [Rider 15 Neo 冷水坑跨年比對](2026-11-24-rider-15-neo.md)。
