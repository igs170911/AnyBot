# M7／M7.5 交接（監督者用）

最後更新：2026-11-28 12:20（UTC+8）。接手的人先讀這份，再讀 `~/.claude/skills/anybot-dev/SKILL.md` 和 `docs/tasks/M7-plan-*.md`。

## 一、人和規則

- 使用者回覆一律用繁體中文，白話、簡短。
- 監督者（Claude）不寫產品程式，寫程式交給 Task 子代理；監督者做派工、合併、重產生成檔、驗證、推送。
- 主資料夾 `~/Documents/AnyBotNext` 永遠停在 `dev`，不要切分支。整合在獨立 worktree 做。
- 推送只用 `~/.claude/skills/anybot-dev/bin/scan_push.sh ~/Documents/AnyBotNext dev`。不用 `gh` 推、不強制推、不改寫歷史。
- 每次派工前複製 `target/agent-prompts/common.md` 到該工作區的 `target/agent-common.md`（若 `target/` 被清掉要重建，內容在各 `ws/m7-*` 工作區的 `target/agent-common.md` 也有一份）。
- 這一輪的規矩：agent 只寫程式和自己套件的檢查，不跑 `make check`、`make test`、雲端檢查；完整驗證由監督者在合併後一次做。
- 不准裝規則清單以外的工具；不用 Docker。

## 二、使用者已經做的決定

- 規劃文件裡約 110 個問題全部照「建議」答案走（各 `M7-plan-*.md` 的 D 節）。
- `/goal`、`/plan` 放 M9，M7 不做。
- 逐字顯示：執行端打 `POST /v1/streams/typing`，伺服器在現有 SSE 加 `typing` 提示，不存紀錄（M7j 做）。
- 形式驗證：在 `tests/` 用 Rust 手寫窮舉，不引進 TLA+ 等工具。
- 端對端測試裡「另一台裝置的動作」可以用照公開 API 收送的替身（預期沒寫那一台自己的畫面時），回報要寫明。
- 同時最多 5 個寫程式的 agent。
- 單一作者套件 `jieba-rs 0.11.0`、`tiktoken-rs 0.12.1` 已同意採用。
- `tokenizers` 因 RUSTSEC-2024-0436 被擋：不做 B10 的 `TokenizerJson`，不改 `deny.toml`。
- `.gitleaksignore` 列 6 筆歷史上的假值（已推 `9362fcb`）。
- 舊決定：Bot 種類用 `bot:<id>` 的 workaround；SLOW 門檻 5 秒、只在請求進行中；`crates/sync` 的測試搬到 `tests/sync-test`。

## 三、目前進度（第一波）

`dev` = `9362fcb`（已推）。五條 ws 分支都從 `ee6845a` 切出，沒有 CI 修正（`a90997b`）和 `.gitleaksignore`，合併時會帶進來。

| 線 | 分支／工作區 | 完成 | Agent ID（可用 resume 追問） |
|---|---|---|---|
| 回合與任務 | `ws/m7-turn`（`AnyBotNext-wt/m7-turn`） | T0–T6、T8（8 commit） | `15693ef5-50ea-4d47-8dfd-9c04ec69751f` |
| 信任卡片 | `ws/m7-trust` | 全部做完：S14、S1、S2、S7、S11、S13、S8、S3、S4、S9、S5、S6、S10；S12 只做了 `T-claims-check-6`（其餘要等回合線的叫醒和 `TurnInitiator::Check`） | `a89e01a2-f127-41b4-ad9d-aa912b629bd8` |
| 工具與交付 | `ws/m7-tools` | F1、F2、F3、F7、F4、F8、F6（7 commit） | `b81d1990-a8b0-4ad5-bd00-8c59c2e34438` |
| 大腦與上下文 | `ws/m7-brain` | B1、B2、B3、B13、B4、B10（只有 o200k）、B14、B15 | `18ea9248-c1fe-4845-b047-09db97863018` |
| 手機畫面 | `ws/m7-phone` | U1、U3、U4 | `ae1b6e74-91b1-4499-a3d9-4a8053a50c43` |

M7j 伺服器整合規劃已完成並合進 `dev`：`docs/tasks/M7-plan-server-integration.md`（23 片 J1–J23，伺服器遷移 0400–0449；問題全部照建議，其中 Q1 `model-reach` 放 J18、Q2 兩列解讀、Q7 跨區分類、Q25 客戶端遷移用 0060–0069，要在最後回報給使用者）。
第二波已派出（2026-11-28 13:10）：`ws/m7-jpush`（J3 Web Push→J2 伺服器半段，agent `6995339f-7196-4c81-a358-861ab6504932`）、`ws/m7-jguard`（J19→J20，agent `b65af779-b6e5-4b3a-96ff-8b3ccfdca9b5`）。J1（狀態入口＋👍）、J5（卡片鑰匙）等第一波整合進 `dev` 後再開。

第一波整合：agent `d1fd70ab-59bd-4ff6-ab63-0e4eed9ee1ce` 正在 `integ/m7-w1` 合併四條分支、重產生成檔、跑快的檢查（2026-11-28 12:30 派出）。它回報後，監督者審最難的衝突（`action.rs`、`approvals.rs`、`runner.rs`、`step.rs`），跑完整驗證再推。

整合分支：`integ/m7-w1`（`AnyBotNext-wt/integ-m7`），目前只合了 `ws/m7-turn`（`fb4ecf0`）。

## 四、下一步（照這個順序）

1. 信任線做完後，派一個整合 agent 在 `AnyBotNext-wt/integ-m7` 依序合 `ws/m7-tools`、`ws/m7-trust`、`ws/m7-brain`、`ws/m7-phone`。
   - 已知衝突：工具線和回合線在 `crates/runner/src/{action,action_events,approvals,context,context_input,lib,runner,step}.rs`、`crates/runner/tests/turn_{approval,descriptive}.rs`、`tests/runner-test/src/{lib,mem_store,turn}.rs` 衝突很多，兩邊的改動都要保留；`web/ui/src/generated/locales.ts` 等生成檔重跑 `make locale-refresh`。
   - `crates/runner/src/context_denied.rs` 的衝突：用 `dev` 的版本（測試模組叫 `context_denied_tests`）。
   - 合完要重產：`make sqlx-prepare`、`make locale-refresh`、`api/openapi.yaml`、`api/asyncapi.yaml`、`api/docs/*.html`、`web/api-client/src/generated/schema.ts`、`make spec-baseline`，以及 `api/breaking-changes.log`（要一行 `日期 | sha256=<committed 基準檔的雜湊> | 做了什麼 | 為什麼`，雜湊要照 api-compat 工具要求）。
   - 手機線的 `StatusResponse & { admin_account? }` 暫時型別，重產 schema 後要拿掉。
   - 客戶端遷移號碼：turn 用 `0200`，brain 預留 `0040`，要查有沒有撞號。
2. 整合完成後本機只跑快的：`make check-fast`，每個動過的套件 `make test-pkg P=<pkg>`；完整的 `make check` 交給雲端。需要全套測試時用 `make test`（會等跨工作區測試鎖）。長時間指令一律用背景 Shell 加 `notify_on_output`，不要在前景等。
3. 整合分支全綠後：在主資料夾 `git merge --no-ff integ/m7-w1`，`scan_push.sh` 推 `dev`，看雲端 CI（`gh run list -R igs170911/AnyBot`），失敗就看 `gh run view <id> --log-failed`。
4. 第二波切片（從新的 `dev` 切新分支，每條 ≤ 5 個並行）：
   - 回合線：T7、T9、T10、T11、T12、T13、T14、T15、T16、T17、T18、T19、T20、T21（T12 伺服器收一句話最先，手機線 U5 要它）。
   - 信任線：若第一輪沒做完的 S3–S6、S9、S12。
   - 工具線：F5（要信任 S14 已合）、F9、F10、F11、F12。
   - 大腦線：B5–B9、B11、B12（需要回合線和工具線先合），M7.5 的 B16–B29（記憶伺服器、搜尋、頁面）。
   - 手機線：U2（要 T13、T14）、U5–U22。
   - M7j 伺服器整合：即時通道 `typing`、電腦在線狀態給手機讀、交付確認、註冊頁管理者名稱（手機線已做了 `admin_account` 的伺服器部分）、Web Push（放 `channels` 的 `web_push` 模組）。**還沒有規劃文件**，要先派規劃 agent 寫 `M7-plan-server-integration.md`。
5. 收尾：手機線 U22 掃全畫面、完整 e2e、雲端全綠，才算 M7 完成；M7.5 另外看 B13–B29。

## 五、未解決、要留意的事

- `crates/sync/src/client_settings.rs:19` 有 `unwrap_or(0)` 的 `offset_ms` 佔位值（送出時由 `setting_hold::freeze_offset` 取代），審查可能會被問。
- 暫時的 stub 要收：
  - 工具線：`crates/tools/src/shell.rs` 的 `find_error_line`（和 context 重複）、`crates/tools/src/list.rs` 的私密名單、`tests/runner-test/src/fake_search.rs`（和 `crates/computers/src/search_rules.rs` 重複）、`crates/runner/src/context_input.rs` 的 `effective_view_limit`（shell／read_result 放寬到 21,000 字，這是產品行為，要跟使用者確認）。
  - 回合線：`crates/runner/src/journal_mask.rs` 的 `SecretMasker` 要換成信任線的 `mask_secrets`；`fallback.rs` 的 `ModelReach`；`card_wait.rs` 的 `TaskWatch`；`edit_file` 的分級要改成 `Irreversible`。
  - 信任線：S8 的送出判斷（`is_send_tool`）等工具線 F10 的 `EvidencePayload::Sent`。
  - 回合線與信任線都有 `mask_secrets`／金鑰規則重複（`anybot_domain::memory_secret_rules`、foundation 的遮罩），要統一。
- `private_paths.rs` 多了 4 個不在規格的名單（`/etc/shadow`、`/etc/sudoers`、`/.securebookmarkswrite`、`cookies`），先保留，等使用者確認。
- `spec/07-repo-layout.md` §3 要補新套件 `crates/terms` 的依賴方向；`spec/` 唯讀，要使用者自己改。
- 手機線的小問題：登入 `label` 用固定英文 `web browser`；兩步驟驗證畫面先放「之後再做」；舊版伺服器偵測沒做。
- 新依賴要在全套檢查看：`cargo deny`、`cargo machete`、`make depcheck`（brain 加了 `crates/terms`、tiktoken、jieba；phone 的 testenv 加了 `sqlx`）；Windows/macOS 編譯（`jieba-rs` 的 `zstd-sys` C 編譯）。
- 生成檔只由監督者重產，agent 不碰。
- 磁碟：每個工作區冷編譯約 4–6G；剩下不到 20G 就先清已合併工作區的 `target/`。只用 `git worktree remove` 清已合併的工作區。
- 殺程式只用 PID，不用 `pkill`／`killall`。

## 六、信任線留給使用者的問題（整合後一次問）

- 在使用者電腦上讀檔（例如 `~/Downloads`）該算一般卡還是「每次都問」？`machine_rules.rs` 的 `user_file_rule` 目前全判每次問，所以智慧模式自答不了，`T-autonomy-smart-2` 只有描述性測試。
- `T-autonomy-smart-8`、`T-claims-history-1`、`T-claims-wake-3/4`、`T-safety-reject-1` 等被回合線的叫醒、接續（T7）、`TurnInitiator::Check` 擋住，等回合線第二波。
- 設定登錄：`yolo`、`direct_mode`、`smart_seconds` 都登成「同步、不分項目」，規格沒明寫 `direct_mode`，請確認。
- 新訊息代碼繁中措辭（`turn-yolo-on/off/status`、`gate-smart-auto-allowed`）請使用者定。
- yolo 被管理員鎖住時，指令沒有回「打不開」的訊息（`T-autonomy-yolo-8` 未做）。
- 卡片由呼叫端（`outcome.pending`）持有：`release_for_yolo`、`answer_expired_cards` 要由呼叫端（執行端主程式）叫；到期開新一輪也要 T7。
- 智慧模式的 `tool_risk` 只認 `read_file`、`shell`、Bot 電腦上的 `write_file`／`edit_file`；`AutoDeny` 沒有工具會走到。
- `report_due` 回 `Result`，跟藍圖簽名不同；「要你注意」的「拒絕」也含自動拒絕；trust 直接用 `url` 套件。
