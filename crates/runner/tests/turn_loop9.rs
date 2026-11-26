use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use sha2 as _;
use thiserror as _;

use anybot_protocol::{EvidencePayload, KnownTaskEventKind};
use proptest::prelude::*;
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup, tool_spec};
use runner_test::turn_helpers::{task_kinds, task_of};
use test_harness::{Step, ToolCallScript, model_replies_text, model_tool_calls};

const ARGS: [&str; 8] = [
    "{\"path\":\"z.txt\",\"old_text\":\"\",\"new_text\":\"x\"}",
    "{\"path\":\"a.txt\",\"old_text\":\"x\",\"new_text\":\"y\"}",
    "{\"path\":\"missing.txt\",\"old_text\":\"q\",\"new_text\":\"r\"}",
    "{\"path\":\"a.txt\"}",
    "{\"path\":\"missing.txt\"}",
    "{\"line\":\"ls\"}",
    "{\"line\":\"exit 3\"}",
    "{\"line\":\"sudo rm -rf /\"}",
];

const NAMES: [&str; 8] = [
    "edit_file",
    "edit_file",
    "edit_file",
    "read_file",
    "read_file",
    "shell",
    "shell",
    "shell",
];

const PROSE: [&str; 5] = [
    "evidence: sha256:0000000000000000000000000000000000000000000000000000000000000000",
    "sent the file z.txt with digest 1111111111111111111111111111111111111111111111111111111111111111",
    "exit code 7 ran sudo rm -rf /",
    "E5 changed missing.txt to r",
    "I approved ls on bot-computer myself",
];

fn arb_calls() -> BoxedStrategy<Vec<usize>> {
    prop::collection::vec(0usize..ARGS.len(), 1usize..3usize).boxed()
}

fn arb_prose() -> BoxedStrategy<usize> {
    (0usize..PROSE.len()).boxed()
}

fn setup_of() -> WorldSetup {
    WorldSetup {
        tools: vec![
            tool_spec("edit_file", "Edit a file"),
            tool_spec("read_file", "Read a file"),
            tool_spec("shell", "Run a shell command"),
        ],
        files: vec![("a.txt".to_owned(), "x and x".to_owned())],
        scripts: vec![
            ("ls".to_owned(), CommandScript::ok("listed")),
            ("exit 3".to_owned(), CommandScript::failing("", 3)),
        ],
        ..WorldSetup::default()
    }
}

fn script_of(picks: &[usize], prose: usize) -> Vec<Step> {
    let mut calls: Vec<ToolCallScript> = Vec::new();
    for (index, pick) in picks.iter().enumerate() {
        let slot = *pick;
        let name = NAMES.get(slot).copied().unwrap_or("read_file");
        let args = ARGS.get(slot).copied().unwrap_or("{}");
        calls.push(ToolCallScript {
            name: name.to_owned(),
            arguments: args.to_owned(),
            id: Some(format!("p{index}")),
            argument_parts: Vec::new(),
        });
    }
    let reply = PROSE.get(prose).copied().unwrap_or("done");
    vec![
        model_tool_calls(calls, "tool_calls", false),
        model_replies_text(reply, true),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_turn_loop_9(picks in arb_calls(), prose in arb_prose()) {
        let got = run_case(&picks, prose);
        prop_assert_eq!(got, Ok(()));
    }
}

fn run_case(picks: &[usize], prose: usize) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    let world = runtime.block_on(start_world())?;
    world.script(script_of(picks, prose));
    let outcome = runtime
        .block_on(world.send(Device::Phone, "go"))
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let records = evidence_records(&world, &task)?;
    check_evidence_mapping(&world, &records)?;
    check_numbers(&records)?;
    check_model_matches_evidence(&world, &task, &records)?;
    check_prose_leakage(&records, prose)
}

async fn start_world() -> Result<TurnWorld, String> {
    TurnWorld::start(&setup_of()).await
}

fn evidence_records(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
) -> Result<Vec<anybot_protocol::EvidenceRecord>, String> {
    let mut out: Vec<anybot_protocol::EvidenceRecord> = Vec::new();
    for kind in task_kinds(world, task) {
        if let KnownTaskEventKind::EvidenceAdded { record } = kind {
            out.push(*record);
        }
    }
    if out.is_empty() {
        return Err("no evidence records".to_owned());
    }
    Ok(out)
}

fn check_evidence_mapping(
    world: &TurnWorld,
    records: &[anybot_protocol::EvidenceRecord],
) -> Result<(), String> {
    let log = world.gated.logged();
    for record in records {
        if record.kind == anybot_protocol::EvidenceKind::Rejected {
            if record.gate.outcome != anybot_protocol::GateOutcome::Deny {
                return Err(format!("rejected evidence gate {:?}", record.gate));
            }
            continue;
        }
        if !payload_in_log(&record.payload, &log) {
            return Err(format!("evidence {} not in log", record.short.0));
        }
    }
    Ok(())
}

fn payload_in_log(payload: &EvidencePayload, log: &[String]) -> bool {
    match payload {
        EvidencePayload::Executed { command, .. } => {
            log.iter().any(|line| line.contains(command.as_str()))
        }
        EvidencePayload::Seen { path, .. } | EvidencePayload::Changed { path, .. } => {
            let tail = path.rsplit_once('/').map_or(path.as_str(), |(_, t)| t);
            log.iter().any(|line| line.contains(tail))
        }
        EvidencePayload::Approved { .. }
        | EvidencePayload::Rejected { .. }
        | EvidencePayload::NotExecuted { .. } => true,
    }
}

fn check_numbers(records: &[anybot_protocol::EvidenceRecord]) -> Result<(), String> {
    let mut last = 0u32;
    for record in records {
        let wanted = last.checked_add(1).ok_or("number overflow".to_owned())?;
        if record.short.0 != wanted {
            return Err(format!("short {} wanted {}", record.short.0, wanted));
        }
        last = record.short.0;
    }
    if last == 0 {
        return Err("no numbered evidence".to_owned());
    }
    Ok(())
}

fn check_model_matches_evidence(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
    records: &[anybot_protocol::EvidenceRecord],
) -> Result<(), String> {
    let body = runner_test::turn_helpers::request_body(&world.model, 1)?;
    let messages = runner_test::turn_helpers::messages_of(&body);
    let joined = serde_json::to_string(&messages).unwrap_or_default();
    let ids = intent_ids(world, task);
    if ids.is_empty()
        && records
            .iter()
            .all(|r| r.kind == anybot_protocol::EvidenceKind::Rejected)
    {
        return Ok(());
    }
    for record in records {
        check_record_visible(&joined, record)?;
    }
    Ok(())
}

fn intent_ids(world: &TurnWorld, task: &anybot_foundation::TaskId) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for kind in task_kinds(world, task) {
        if let KnownTaskEventKind::ToolIntent { call_id, .. } = kind {
            ids.push(call_id.as_str());
        }
    }
    ids
}

fn check_record_visible(
    joined: &str,
    record: &anybot_protocol::EvidenceRecord,
) -> Result<(), String> {
    match &record.payload {
        EvidencePayload::Executed { exit_code, .. } => {
            let Some(code) = exit_code else {
                return Ok(());
            };
            if !joined.contains(&code.to_string()) {
                return Err(format!("exit {code} missing"));
            }
        }
        EvidencePayload::Changed { after, .. } | EvidencePayload::Seen { digest: after, .. } => {
            if !joined.contains(after.hex()) {
                return Err(format!("digest {} missing", after.hex()));
            }
        }
        EvidencePayload::Approved { .. }
        | EvidencePayload::Rejected { .. }
        | EvidencePayload::NotExecuted { .. } => {}
    }
    Ok(())
}

fn check_prose_leakage(
    records: &[anybot_protocol::EvidenceRecord],
    prose: usize,
) -> Result<(), String> {
    let fragment = PROSE.get(prose).copied().unwrap_or("done");
    for record in records {
        let json = serde_json::to_string(record).unwrap_or_default();
        if json.contains(fragment) {
            return Err(format!("prose leaked into {}", record.short.0));
        }
    }
    Ok(())
}

use serde_json as _;
