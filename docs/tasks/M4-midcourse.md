# M4 中途修正：測試卡死

你跑 `cargo nextest run -p test-harness` 時，有兩個假時鐘的測試卡了 8 分鐘以上（CPU 0%），我把它們收掉了，也停了你這一輪。工作目錄裡的修改都還在，接著做，但先處理下面兩件。

卡住的測試：
- `fake_clock::tests::test_fake_clock_wakes_sleepers_in_deadline_order`
- `fake_clock::tests::test_fake_clock_timeout_fires_after_advance`

## 1. 修假時鐘

- 這兩個測試裡的等待永遠沒醒，代表 `advance` 沒有叫醒到期的等待，或叫醒的方式有死結。找出原因修好，不是改測試讓它過。
- 測試裡等假時鐘的東西（等某個等待醒來、等某個任務結束），都要包一層**真時間**的期限，例如幾秒。到期就判定失敗並說清楚在等什麼。這是 `AGENTS.md` 第三節「測試裡等外面的東西都要有期限」，非同步的等待也一樣。
- 這個「真時間的期限」只准在測試和 `tests/harness` 用，產品程式碼還是只能透過 `Clock`。

## 2. 測試卡死要被自動收掉

- 加 `.config/nextest.toml`：預設設定檔用 `slow-timeout` 加 `terminate-after`，讓卡住的測試在幾分鐘內被收掉並判定失敗。例如每 30 秒提醒一次，4 次後收掉。
- 本來就比較慢的測試（PostgreSQL、端對端）用 `[[profile.default.overrides]]` 給較長的時間，不要整體放寬。
- iron-laws 加一道檢查：`.config/nextest.toml` 存在，而且預設設定檔有 `terminate-after`。附「拿掉會被擋」的測試。
- 你自己跑測試的指令，也要給你的執行工具設逾時（你的 bash 工具有 `timeout` 參數），不要讓一個指令無限期等。

## 其他照舊

- `docs/tasks/M4-test-foundation.md` 照原本的計畫繼續做：A 段 commit 一次，B 段 commit 一次，最後一起回報。
- 回報多加一段「中途修正」：假時鐘為什麼卡住、怎麼修的。
