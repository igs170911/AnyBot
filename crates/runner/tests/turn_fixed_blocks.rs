use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_protocol as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use anybot_messages::{Catalog, Lang, Params};
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{chat_requests, request_body, system_content};
use test_harness::model_replies_text;

fn fmt(lang: Lang, code: &str, params: &Params) -> Result<String, String> {
    Catalog::new()
        .format_str(lang, code, params)
        .map_err(|e| format!("{e:?}"))
}

fn identity_text(lang: Lang) -> Result<String, String> {
    let params = Params::new()
        .with_text("name", "Ada")
        .with_text("timezone", "Asia/Taipei");
    fmt(lang, "turn-fixed-identity", &params)
}

fn instructions_text(lang: Lang, instructions: &str) -> Result<String, String> {
    let params = Params::new().with_text("instructions", instructions);
    fmt(lang, "turn-fixed-instructions", &params)
}

fn computer_text(lang: Lang) -> Result<String, String> {
    fmt(lang, "turn-fixed-computer-bot-machine", &Params::new())
}

fn safety_text(lang: Lang) -> Result<String, String> {
    fmt(lang, "turn-fixed-safety", &Params::new())
}

fn task_evidence_text(lang: Lang) -> Result<String, String> {
    fmt(lang, "turn-fixed-task-evidence", &Params::new())
}

fn expected_fixed_parts(lang: Lang, instructions: &str) -> Result<Vec<String>, String> {
    let mut parts = vec![identity_text(lang)?];
    if !instructions.is_empty() {
        parts.push(instructions_text(lang, instructions)?);
    }
    parts.push(computer_text(lang)?);
    parts.push(safety_text(lang)?);
    parts.push(task_evidence_text(lang)?);
    parts.push("read_file: Read a file".to_owned());
    Ok(parts)
}

fn lang_of(lang: Option<Lang>) -> Lang {
    match lang {
        Some(value) => value,
        None => Lang::En,
    }
}

fn check_increasing(system: &str, parts: &[String]) -> Result<(), String> {
    let mut last: Option<usize> = None;
    for part in parts {
        let Some(at) = system.find(part) else {
            return Err(format!("missing part: {part}"));
        };
        if let Some(prev) = last
            && at <= prev
        {
            return Err("parts out of order".to_owned());
        }
        last = Some(at);
    }
    Ok(())
}

async fn send_hi(world: &TurnWorld) -> Result<(), String> {
    world.script(vec![model_replies_text("ok", false)]);
    world
        .send(Device::Phone, "hi")
        .await
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

#[tokio::test]
async fn turn_fixed_blocks_follow_order_and_language() -> Result<(), String> {
    for lang in [Some(Lang::ZhTw), None] {
        let setup = WorldSetup {
            lang,
            ..WorldSetup::default()
        };
        let world = TurnWorld::start(&setup).await?;
        world.script(vec![model_replies_text("hi", false)]);
        let outcome = world.send(Device::Phone, "hello").await;
        let outcome = outcome.map_err(|e| format!("{e:?}"))?;
        drop(outcome);
        let body = request_body(&world.model, 0)?;
        let system = system_content(&body)?;
        let parts = expected_fixed_parts(lang_of(lang), "Be brief.")?;
        check_increasing(&system, &parts)?;
        if lang == Some(Lang::ZhTw) {
            let english = identity_text(Lang::En)?;
            assert!(!system.contains(&english));
        }
        let again = world.send(Device::Phone, "again").await;
        drop(again.map_err(|e| format!("{e:?}"))?);
        assert_eq!(chat_requests(&world.model).len(), 2);
        let body2 = request_body(&world.model, 1)?;
        assert_eq!(system_content(&body)?, system_content(&body2)?);
    }
    Ok(())
}

#[tokio::test]
async fn turn_fixed_blocks_skip_empty_instructions() -> Result<(), String> {
    let setup = WorldSetup {
        instructions: String::new(),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_replies_text("hi", false)]);
    let outcome = world.send(Device::Phone, "hello").await;
    let outcome = outcome.map_err(|e| format!("{e:?}"))?;
    drop(outcome);
    let body = request_body(&world.model, 0)?;
    let system = system_content(&body)?;
    let skipped = instructions_text(Lang::En, "")?;
    assert!(!system.contains(&skipped));
    let parts = expected_fixed_parts(Lang::En, "")?;
    check_increasing(&system, &parts)
}

#[tokio::test]
async fn t_context_fixed_2() -> Result<(), String> {
    let mut first_systems = Vec::new();
    for lang in [Some(Lang::ZhTw), None] {
        let setup = WorldSetup {
            lang,
            ..WorldSetup::default()
        };
        let world = TurnWorld::start(&setup).await?;
        send_hi(&world).await?;
        let system1 = system_content(&request_body(&world.model, 0)?)?;
        let parts = expected_fixed_parts(lang_of(lang), "Be brief.")?;
        let expected_prefix = parts.join("\n");
        assert!(
            system1.starts_with(&expected_prefix),
            "system prefix mismatch for {lang:?}: {system1}"
        );
        check_increasing(&system1, &parts)?;
        if lang == Some(Lang::ZhTw) {
            assert!(!system1.contains(&safety_text(Lang::En)?));
            assert!(!system1.contains(&task_evidence_text(Lang::En)?));
        }
        assert!(system1.contains("Asia/Taipei"));
        assert!(!system1.contains("1700000"));
        send_hi(&world).await?;
        let system2 = system_content(&request_body(&world.model, 1)?)?;
        assert_eq!(system1.as_bytes(), system2.as_bytes());
        first_systems.push(system1);
    }
    assert_ne!(first_systems.first(), first_systems.get(1));
    Ok(())
}

#[tokio::test]
async fn fixed_blocks_keep_order_without_pinned() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = TurnWorld::start(&setup).await?;
    send_hi(&world).await?;
    let system = system_content(&request_body(&world.model, 0)?)?;
    let parts = expected_fixed_parts(Lang::En, "Be brief.")?;
    check_increasing(&system, &parts)?;
    let expected_prefix = parts.join("\n");
    let tail = system
        .strip_prefix(&expected_prefix)
        .ok_or_else(|| format!("missing prefix in: {system}"))?;
    assert!(tail.is_empty() || tail == "\npinned snapshot v");
    Ok(())
}
