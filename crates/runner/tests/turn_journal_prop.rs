use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_tools as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use anybot_foundation::BotId;
use anybot_runner::{JournalRecord, Record, RunnerParts, TurnRunner, UserMessage};
use anybot_store_client::{Store, bot_stream};
use anybot_trust::GatedComputer;
use proptest::prelude::*;
use runner_test::FakeComputer;
use runner_test::fake_builder::FakeComputerBuilder;
use runner_test::fake_scripts::CommandScript;
use runner_test::mem_raw::MemoryRaw;
use runner_test::mem_watch::{MemoryWatch, share_watch};
use runner_test::turn::WorldSetup;
use runner_test::turn_setup::{bot_profile, build_session, computer_of, share_clock, turn_model};
use runner_test::turn_tool_helpers::all_specs;
use test_harness::{
    FakeClock, FakeModel, Step, ToolCallScript, model_replies_text, model_start_with_format,
    model_tool_calls,
};

mod journal_probe;
use journal_probe::{CrashProbe, DropAfterPolls};

struct SharedHarness {
    runtime: tokio::runtime::Runtime,
    model: Arc<FakeModel>,
    port: u16,
}

fn shared_harness() -> Result<&'static SharedHarness, String> {
    static HARNESS: OnceLock<SharedHarness> = OnceLock::new();
    if let Some(existing) = HARNESS.get() {
        return Ok(existing);
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    let (model, port) =
        runtime.block_on(model_start_with_format(test_harness::ModelFormat::OpenAi))?;
    if HARNESS
        .set(SharedHarness {
            runtime,
            model,
            port,
        })
        .is_err()
    {
        return HARNESS.get().ok_or_else(|| "harness init".to_owned());
    }
    HARNESS.get().ok_or_else(|| "harness missing".to_owned())
}

fn script_of(steps_spec: &[Vec<usize>]) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut counter = 0usize;
    for batch in steps_spec {
        let mut calls = Vec::new();
        for &idx in batch {
            counter = counter.saturating_add(1);
            let (name, args) = match idx {
                0 => ("read_file", "{\"path\":\"a.txt\"}"),
                1 => (
                    "edit_file",
                    "{\"path\":\"a.txt\",\"old_text\":\"\",\"new_text\":\"z\"}",
                ),
                _ => ("shell", "{\"line\":\"ls\"}"),
            };
            calls.push(ToolCallScript {
                name: name.to_owned(),
                arguments: args.to_owned(),
                id: Some(format!("q{counter}")),
                argument_parts: Vec::new(),
            });
        }
        steps.push(model_tool_calls(calls, "tool_calls", false));
    }
    steps.push(model_replies_text("done", false));
    steps
}

async fn run_and_verify(
    dir: &Path,
    setup: &WorldSetup,
    clock: &Arc<FakeClock>,
    bot: BotId,
    fake: &Arc<FakeComputer>,
    port: u16,
    drop_poll: usize,
) -> Result<(), String> {
    let stream = bot_stream(&bot.as_str());
    let opened = Store::open(dir, 1).await.map_err(|e| e.to_string())?;
    let store = opened.store;
    let record: Arc<dyn Record> = Arc::new(JournalRecord::new(store.clone()));
    let probe = Arc::new(CrashProbe {
        inner: Arc::clone(fake),
        store: store.clone(),
        stream: stream.clone(),
        handle: tokio::runtime::Handle::current(),
        exec_count: AtomicUsize::new(0),
        missing_intent: AtomicBool::new(false),
    });
    let watch = Arc::new(MemoryWatch::new());
    let raw: Arc<dyn anybot_runner::RawStore> = Arc::new(MemoryRaw::new());
    let clock_dyn = share_clock(clock);
    let runner = Arc::new(TurnRunner::new(RunnerParts {
        gated: Arc::clone(&probe) as Arc<dyn GatedComputer>,
        record,
        raw,
        watch: share_watch(&watch),
        session: build_session(clock)?,
        clock: Arc::clone(&clock_dyn),
        bot: bot_profile(setup, bot),
        model: turn_model(setup, port),
        tools: setup.tools.clone(),
        lang: None,
        computer: computer_of(setup),
    }));
    let device = anybot_foundation::DeviceId::generate(clock_dyn.as_ref());
    let fut = runner.on_user_message(UserMessage {
        device,
        text: "go".to_owned(),
    });
    let _stopped = DropAfterPolls {
        inner: Box::pin(fut),
        remaining: drop_poll,
    }
    .await;
    drop(runner);
    store.close().await;
    verify_after_reopen(dir, &stream, fake, &probe).await
}

async fn verify_after_reopen(
    dir: &Path,
    stream: &str,
    fake: &FakeComputer,
    probe: &CrashProbe,
) -> Result<(), String> {
    let reopened = Store::open(dir, 2).await.map_err(|e| e.to_string())?;
    let rows = reopened
        .store
        .journal_stream_rows(stream)
        .await
        .map_err(|e| e.to_string())?;
    let executed = fake
        .logged()
        .iter()
        .filter(|e| *e != "capabilities" && !e.starts_with("run_command done"))
        .count();
    let intents = rows.iter().filter(|r| r.kind == "tool_intent").count();
    if probe.missing_intent.load(Ordering::SeqCst) {
        return Err("execution observed before tool_intent persisted".to_owned());
    }
    if intents < executed {
        return Err(format!("intents {intents} < executed {executed}"));
    }
    let open_rows = reopened
        .store
        .journal_open()
        .await
        .map_err(|e| e.to_string())?;
    let has_start = rows.iter().any(|r| r.kind == "turn_started");
    let has_end = rows.iter().any(|r| r.kind == "turn_ended");
    if has_start && !has_end && open_rows.is_empty() {
        return Err("open turn missing from journal_open after crash".to_owned());
    }
    reopened.store.close().await;
    Ok(())
}

fn run_crash_case(steps_spec: &[Vec<usize>], drop_poll: usize) -> Result<(), String> {
    let h = shared_harness()?;
    let dir = test_harness::test_dir("turn_journal_prop_case")?;
    let setup = WorldSetup {
        tools: all_specs(),
        files: vec![("a.txt".to_owned(), "x".to_owned())],
        scripts: vec![("ls".to_owned(), CommandScript::ok("listed"))],
        ..WorldSetup::default()
    };
    h.model.script_set(script_of(steps_spec));
    let clock = FakeClock::new(1_700_000_000_000);
    let bot = BotId::generate(share_clock(&clock).as_ref());
    let fake = Arc::new(
        FakeComputerBuilder::default()
            .named("bot-computer")
            .sandbox(true)
            .gate(setup.gate.clone())
            .clock(Arc::clone(&clock))
            .bot_machine()
            .file("a.txt", "x")
            .script("ls", CommandScript::ok("listed"))
            .build(),
    );
    h.runtime.block_on(run_and_verify(
        &dir, &setup, &clock, bot, &fake, h.port, drop_poll,
    ))
}

fn arb_steps() -> BoxedStrategy<Vec<Vec<usize>>> {
    prop::collection::vec(prop::collection::vec(0usize..3usize, 1usize..3usize), 1..3).boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn t_task_journal_4(steps in arb_steps(), drop_poll in 0usize..40usize) {
        let got = run_crash_case(&steps, drop_poll);
        prop_assert_eq!(got, Ok(()));
    }
}
