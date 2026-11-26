use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use anybot_protocol::{KnownTaskEventKind, TaskEvent, TaskEventKind};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{request_body, task_of};
use runner_test::turn_tool_helpers::{all_specs, call, edit_call};
use test_harness::{Step, ToolCallScript, model_replies_text, model_tool_calls};

const EDIT_SLOTS: [usize; 3] = [1, 3, 5];
const TOTAL_STEPS: usize = 20;

fn ls_call(index: usize) -> ToolCallScript {
    call("shell", "{\"line\":\"ls\"}", &format!("x{index}"))
}

fn build_setup() -> WorldSetup {
    WorldSetup {
        tools: all_specs(),
        files: vec![
            ("a.txt".to_owned(), "one".to_owned()),
            ("b.txt".to_owned(), "two".to_owned()),
            ("c.txt".to_owned(), "three".to_owned()),
        ],
        scripts: vec![
            ("ls".to_owned(), CommandScript::ok("listed")),
            ("boom".to_owned(), CommandScript::failing("bad", 1)),
        ],
        ..WorldSetup::default()
    }
}

fn edit_slot(step: usize) -> Option<usize> {
    EDIT_SLOTS.iter().position(|found| *found == step)
}

fn script_build() -> Vec<Step> {
    let mut out: Vec<Step> = Vec::new();
    for step in 0..60usize {
        out.push(model_tool_calls(vec![ls_call(step)], "tool_calls", false));
    }
    out.push(model_replies_text("done", false));
    out
}

fn script_state() -> Vec<Step> {
    let mut out: Vec<Step> = Vec::new();
    for step in 1..=TOTAL_STEPS {
        let calls = calls_of(step);
        out.push(model_tool_calls(calls, "tool_calls", false));
    }
    out.push(model_replies_text("done", false));
    out
}

fn calls_of(step: usize) -> Vec<ToolCallScript> {
    if step == 7 {
        return vec![call("shell", "{\"line\":\"boom\"}", "s7")];
    }
    if let Some(found) = edit_slot(step) {
        let wanted = ["a.txt", "b.txt", "c.txt"];
        let Some(path) = wanted.get(found) else {
            return vec![ls_call(step)];
        };
        let args = format!("{{\"path\":\"{path}\",\"old_text\":\"\",\"new_text\":\"v{step}\"}}");
        return vec![edit_call(&args, &format!("s{step}"))];
    }
    vec![ls_call(step)]
}

#[tokio::test]
async fn t_context_build_4() -> Result<(), String> {
    let world = TurnWorld::start(&build_setup()).await?;
    world.script(script_build());
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let wanted = world.chat_requests().len();
    if wanted != 61 {
        return Err(format!("requests {wanted}"));
    }
    let events = world.task_events(task);
    let kinds = known_kinds(&events);
    let mut built = 0usize;
    for (index, kind) in kinds.iter().enumerate() {
        let KnownTaskEventKind::ModelAsked { .. } = kind else {
            continue;
        };
        let Some(prev) = index.checked_sub(1) else {
            return Err("asked at head".to_owned());
        };
        let Some(KnownTaskEventKind::ContextBuilt { manifest_id, .. }) = kinds.get(prev) else {
            return Err("asked without build".to_owned());
        };
        if manifest_id.is_empty() {
            return Err("empty manifest".to_owned());
        }
        built = built.saturating_add(1);
    }
    if built != wanted {
        return Err(format!("built {built} requests {wanted}"));
    }
    Ok(())
}

fn known_kinds(events: &[TaskEvent]) -> Vec<KnownTaskEventKind> {
    events
        .iter()
        .filter_map(|event| match &event.kind {
            TaskEventKind::Known(kind) => Some(kind.clone()),
            TaskEventKind::Raw(_) => None,
        })
        .collect()
}

#[tokio::test]
async fn t_context_state_2() -> Result<(), String> {
    let world = TurnWorld::start(&build_setup()).await?;
    world.script(script_state());
    let outcome = world.send(Device::Phone, "go").await;
    let _task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let wanted = world.chat_requests().len();
    if wanted != 21 {
        return Err(format!("requests {wanted}"));
    }
    let body = request_body(&world.model, 20)?;
    let text = joined_messages(&body);
    check_state_text(&text, &world)
}

fn joined_messages(body: &serde_json::Value) -> String {
    let items = runner_test::turn_helpers::messages_of(body);
    let mut out = String::new();
    for item in items {
        if let Some(found) = item.get("content").and_then(|v| v.as_str()) {
            out.push_str(found);
            out.push('\n');
        }
    }
    out
}

fn check_state_text(text: &str, world: &TurnWorld) -> Result<(), String> {
    let found = text.matches(anybot_context::CODE_TURN_PROGRAM_NOTE).count();
    if found != 1 {
        return Err(format!("notes {found}"));
    }
    for path in ["a.txt", "b.txt", "c.txt"] {
        let Some(content) = world.tree_content(path) else {
            return Err(format!("no file {path}"));
        };
        if !text.contains(&digest_hex(&content)) {
            return Err(format!("digest {path} missing"));
        }
    }
    if !text.contains("exit 1") {
        return Err("exit 1 missing".to_owned());
    }
    if !text.contains("step ") {
        return Err("steps missing".to_owned());
    }
    Ok(())
}

fn digest_hex(content: &str) -> String {
    anybot_foundation::ContentDigest::sha256(content.as_bytes())
        .hex()
        .to_owned()
}
