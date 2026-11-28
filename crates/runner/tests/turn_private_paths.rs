use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_protocol as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use anybot_protocol::{
    ComputerOwner, ComputerPlatform, EvidenceKind, EvidencePayload, GateOutcome, KnownTaskEventKind,
};
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{task_kinds, task_of};
use runner_test::turn_tool_helpers::{all_specs, call, one_tool_step};

fn private_setup() -> WorldSetup {
    WorldSetup {
        owner: ComputerOwner::User,
        platform: ComputerPlatform::Mac,
        computer_name: "Work Mac".to_owned(),
        workdir: "/home/alice/work".to_owned(),
        home_dir: "/home/alice".to_owned(),
        anybot_dir: "/home/alice/.local/share/AnyBot".to_owned(),
        case_insensitive: true,
        tools: all_specs(),
        links: vec![("link".to_owned(), "~/.ssh".to_owned())],
        unresolvable: vec!["broken/secret.txt".to_owned()],
        files: vec![
            (
                "/home/alice/.ssh/config".to_owned(),
                "SSH_SECRET".to_owned(),
            ),
            (
                "/home/alice/.aws/config".to_owned(),
                "AWS_SECRET".to_owned(),
            ),
            ("/home/alice/work/.ENV".to_owned(), "ENV_SECRET".to_owned()),
            (
                "/home/alice/work/broken/secret.txt".to_owned(),
                "BROKEN_SECRET".to_owned(),
            ),
        ],
        ..WorldSetup::default()
    }
}

fn read_args(path: &str) -> String {
    serde_json::json!({ "path": path }).to_string()
}

fn edit_args(path: &str) -> String {
    serde_json::json!({
        "path": path,
        "old_text": "",
        "new_text": "overwrite"
    })
    .to_string()
}

async fn run_and_assert_private_denied(
    world: &TurnWorld,
    tool: &str,
    args: String,
    expected_path: &str,
    call_id: &str,
) -> Result<(), String> {
    world.script(one_tool_step(vec![call(tool, &args, call_id)]));
    let outcome = world.send(Device::Phone, "check").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let kinds = task_kinds(world, &task);
    let started = kinds
        .iter()
        .any(|k| matches!(k, KnownTaskEventKind::ToolStarted { .. }));
    if started {
        return Err(format!("tool started for private path {expected_path}"));
    }
    assert_private_gate_and_evidence(&kinds, expected_path)
}

fn assert_private_gate_and_evidence(
    kinds: &[KnownTaskEventKind],
    expected_path: &str,
) -> Result<(), String> {
    let denied = kinds.iter().any(|k| {
        matches!(
            k,
            KnownTaskEventKind::GateResult { gate, .. } if gate.outcome == GateOutcome::Deny
        )
    });
    if !denied {
        return Err(format!("missing GateOutcome::Deny for {expected_path}"));
    }
    for kind in kinds {
        let KnownTaskEventKind::EvidenceAdded { record } = kind else {
            continue;
        };
        if record.kind == EvidenceKind::Rejected
            && let EvidencePayload::Rejected { reason } = &record.payload
        {
            if reason.code != "gate-private-file" {
                return Err(format!("wrong code {} for {expected_path}", reason.code));
            }
            let got_path = reason
                .params
                .iter()
                .find(|(k, _)| k == "path")
                .map(|(_, v)| v.as_str());
            if got_path != Some(expected_path) {
                return Err(format!("wrong path {got_path:?} expected {expected_path}"));
            }
            return Ok(());
        }
    }
    Err(format!("missing rejected evidence for {expected_path}"))
}

#[tokio::test]
async fn t_sandbox_private_2() -> Result<(), String> {
    let world = TurnWorld::start(&private_setup()).await?;
    for (idx, path) in ["link/config", "../.aws/config", ".env"].iter().enumerate() {
        run_and_assert_private_denied(
            &world,
            "read_file",
            read_args(path),
            path,
            &format!("priv_{idx}"),
        )
        .await?;
    }
    for req in world.chat_requests() {
        let body = serde_json::to_string(&req.body).map_err(|e| format!("{e:?}"))?;
        for secret in ["SSH_SECRET", "AWS_SECRET", "ENV_SECRET"] {
            if body.contains(secret) {
                return Err(format!("leaked secret {secret}"));
            }
        }
    }
    Ok(())
}

fn spec_private_targets() -> [&'static str; 30] {
    [
        "/home/alice/.local/share/AnyBot/keys.db",
        "/home/alice/backup/vault.bin",
        "/home/alice/backup/vault.key",
        "/home/alice/work/.git/HEAD",
        "/home/alice/.ssh/id_rsa",
        "/home/alice/.gnupg/secring.gpg",
        "/home/alice/Library/Keychains/login.keychain-db",
        "/home/alice/.aws/credentials",
        "/home/alice/.config/gcloud/creds.json",
        "/home/alice/.kube/config",
        "/home/alice/.docker/config.json",
        "/home/alice/.anybot/cfg.json",
        "/home/alice/.local/share/keyrings/login.keyring",
        "/home/alice/Library/Group Containers/2BUA8C4S2C.com.1password/data",
        ".env",
        ".env.local",
        "cert.pem",
        "server.key",
        "id_rsa",
        "id_rsa.pub",
        "id_ed25519",
        "id_ecdsa",
        "id_dsa",
        ".netrc",
        "credentials",
        "credentials.json",
        "credentials.toml",
        "application_default_credentials.json",
        ".npmrc",
        ".pypirc",
    ]
}

#[tokio::test]
async fn private_paths_cover_spec_list() -> Result<(), String> {
    let world = TurnWorld::start(&private_setup()).await?;
    let mut all_paths: Vec<&'static str> = spec_private_targets().to_vec();
    all_paths.push(".zsh_history");
    all_paths.push(".bash_history");
    for (idx, path) in all_paths.iter().enumerate() {
        run_and_assert_private_denied(
            &world,
            "read_file",
            read_args(path),
            path,
            &format!("read_{idx}"),
        )
        .await?;
        run_and_assert_private_denied(
            &world,
            "edit_file",
            edit_args(path),
            path,
            &format!("edit_{idx}"),
        )
        .await?;
    }
    Ok(())
}

#[tokio::test]
async fn private_path_unresolved_is_denied() -> Result<(), String> {
    let world = TurnWorld::start(&private_setup()).await?;
    run_and_assert_private_denied(
        &world,
        "read_file",
        read_args("broken/secret.txt"),
        "broken/secret.txt",
        "unres_1",
    )
    .await?;
    for req in world.chat_requests() {
        let body = serde_json::to_string(&req.body).map_err(|e| format!("{e:?}"))?;
        if body.contains("BROKEN_SECRET") {
            return Err("leaked unresolvable file secret".to_owned());
        }
    }
    Ok(())
}
