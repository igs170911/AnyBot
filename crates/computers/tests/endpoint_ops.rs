use std::path::PathBuf;

use anybot_computers::BotComputer;
use anybot_foundation::{SystemClock, TaskId, TurnId};
use anybot_protocol::{
    BeforeDigest, ComputerOwner, ComputerPlatform, EvidenceKind, EvidencePayload, GateOutcome,
};
use anybot_trust::{
    ActionContext, ActionRequest, ActionSubject, GateConfig, PathView, RunStep, run_action,
};
use runner_test::TestView;
use tokio as _;

fn root_of(label: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("endpoint-ops")
        .join(label)
}

fn temp_root(label: &str) -> Result<PathBuf, String> {
    let base = root_of(label);
    drop(std::fs::remove_dir_all(&base));
    std::fs::create_dir_all(&base).map_err(|e| format!("cannot create the root: {e}"))?;
    Ok(base)
}

fn computer(label: &str) -> Result<BotComputer, String> {
    let root = temp_root(label)?;
    let text = root.to_str().ok_or("the root path is not text")?;
    BotComputer::new(label, ComputerPlatform::Linux, ComputerOwner::Bot, text)
        .map_err(|e| format!("cannot start the computer: {}", e.reason))
}

fn context(tool: &str, workdir: &str) -> ActionContext {
    let clock = SystemClock;
    ActionContext {
        task: TaskId::generate(&clock),
        turn: TurnId::generate(&clock),
        evidence_no: 7,
        computer: tool.to_owned(),
        workdir: workdir.to_owned(),
        tool: tool.to_owned(),
        classes: Vec::new(),
    }
}

fn request(tool: &str, workdir: &str, subject: ActionSubject) -> ActionRequest {
    ActionRequest {
        context: context(tool, workdir),
        subject,
    }
}

fn dir_of(label: &str) -> String {
    root_of(label).display().to_string()
}

fn run(computer: &BotComputer, req: &ActionRequest) -> RunStep {
    run_action(req, computer, &GateConfig::plain(), &TestView, &SystemClock)
}

fn write_subject(path: &str, content: &str) -> ActionSubject {
    ActionSubject::WriteFile {
        path: PathView::raw(path),
        content: content.to_owned(),
    }
}

fn changed_of(step: &RunStep) -> Result<(BeforeDigest, anybot_foundation::ContentDigest), String> {
    let RunStep::Executed(run) = step else {
        return Err("the run must execute".to_owned());
    };
    assert_eq!(run.gate.outcome, GateOutcome::Allow);
    let last = run
        .evidence
        .last()
        .ok_or("evidence records missing".to_owned())?;
    let EvidencePayload::Changed { before, after, .. } = &last.payload else {
        return Err("changed payload expected".to_owned());
    };
    Ok((before.clone(), after.clone()))
}

#[test]
fn t_computer_ops_2() -> Result<(), String> {
    let c = computer("ops2")?;
    let first = run(
        &c,
        &request(
            "edit_file",
            &dir_of("ops2"),
            write_subject("notes.md", "one\n"),
        ),
    );
    let stored = std::fs::read_to_string(root_of("ops2").join("notes.md"))
        .map_err(|e| format!("cannot read notes: {e}"))?;
    assert_eq!(stored, "one\n");
    let (before_first, after_first) = changed_of(&first)?;
    assert_eq!(before_first, BeforeDigest::Absent);
    let second = run(
        &c,
        &request(
            "edit_file",
            &dir_of("ops2"),
            write_subject("notes.md", "two\n"),
        ),
    );
    let (before_second, _) = changed_of(&second)?;
    assert_eq!(before_second, BeforeDigest::Present(after_first));
    assert_eq!(
        std::fs::read_to_string(root_of("ops2").join("notes.md"))
            .map_err(|e| format!("cannot read notes: {e}"))?,
        "two\n",
    );
    Ok(())
}

#[test]
fn t_computer_ops_3() -> Result<(), String> {
    let c = computer("ops3")?;
    let deep = run(
        &c,
        &request(
            "edit_file",
            &dir_of("ops3"),
            write_subject("deep/notes.md", "text\n"),
        ),
    );
    if !root_of("ops3").join("deep").is_dir() {
        return Err(format!("the folder is missing after {deep:?}"));
    }
    let read = run(
        &c,
        &request(
            "read_file",
            &dir_of("ops3"),
            ActionSubject::ReadFile {
                path: PathView::raw("deep/notes.md"),
            },
        ),
    );
    let RunStep::Executed(read) = read else {
        return Err("read must execute".to_owned());
    };
    let first_seen = read.evidence.first().ok_or("evidence missing".to_owned())?;
    assert_eq!(first_seen.kind, EvidenceKind::Seen);
    let missing = run(
        &c,
        &request(
            "read_file",
            &dir_of("ops3"),
            ActionSubject::ReadFile {
                path: PathView::raw("missing.md"),
            },
        ),
    );
    let RunStep::Executed(missing) = missing else {
        return Err("a failed read still completes".to_owned());
    };
    assert_eq!(missing.summary.status.code, "turn-tool-failed");
    Ok(())
}

#[test]
fn t_computer_ops_4() -> Result<(), String> {
    let c = computer("ops4")?;
    let done = run(
        &c,
        &request(
            "shell",
            &dir_of("ops4"),
            ActionSubject::Command {
                line: "true".to_owned(),
                timeout_ms: 5000,
            },
        ),
    );
    let RunStep::Executed(done) = done else {
        return Err("the command must execute".to_owned());
    };
    let executed = done
        .evidence
        .last()
        .ok_or("the executed record is missing".to_owned())?;
    let EvidencePayload::Executed { exit_code, .. } = &executed.payload else {
        return Err("executed payload expected".to_owned());
    };
    assert_eq!(*exit_code, Some(0));
    let slow = run(
        &c,
        &request(
            "shell",
            &dir_of("ops4"),
            ActionSubject::Command {
                line: "sleep 5".to_owned(),
                timeout_ms: 200,
            },
        ),
    );
    let RunStep::Executed(slow) = slow else {
        return Err("the timeout run completes".to_owned());
    };
    let executed = slow
        .evidence
        .last()
        .ok_or("the executed record is missing".to_owned())?;
    let EvidencePayload::Executed { exit_code, .. } = &executed.payload else {
        return Err("executed payload expected".to_owned());
    };
    assert_eq!(*exit_code, Some(124));
    Ok(())
}

#[test]
fn t_computer_ops_5() -> Result<(), String> {
    let c = computer("ops5")?;
    let outside = run(
        &c,
        &request(
            "edit_file",
            &dir_of("ops5"),
            write_subject("/tmp/m7c-outside.md", "x\n"),
        ),
    );
    assert_eq!(outside.into_run().gate.outcome, GateOutcome::Deny);
    let escape = run(
        &c,
        &request(
            "edit_file",
            &dir_of("ops5"),
            write_subject("../escape.md", "x\n"),
        ),
    );
    assert_eq!(escape.into_run().gate.outcome, GateOutcome::Deny);
    let link = root_of("ops5").join("link.md");
    let target = root_of("ops5").join("target.md");
    std::fs::write(&target, "inside\n").map_err(|e| format!("cannot write the target: {e}"))?;
    std::os::unix::fs::symlink(&target, &link).map_err(|e| format!("cannot link: {e}"))?;
    let checked = run(
        &c,
        &request(
            "edit_file",
            &dir_of("ops5"),
            ActionSubject::PathCheck {
                path: PathView::raw("link.md"),
            },
        ),
    );
    let RunStep::Executed(checked) = checked else {
        return Err("path check executes".to_owned());
    };
    let seen = checked
        .evidence
        .first()
        .ok_or("evidence missing".to_owned())?;
    assert_eq!(seen.kind, EvidenceKind::Seen);
    Ok(())
}
