use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_protocol as _;
use anybot_runner as _;
use anybot_store_client as _;
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

fn code_text(lang: Lang, code: &str) -> Result<String, String> {
    fmt(lang, code, &Params::new())
}

fn fixed_prompt(lang: Lang, instructions: &str) -> Result<Vec<String>, String> {
    let mut parts = vec![identity_text(lang)?];
    if !instructions.is_empty() {
        parts.push(instructions_text(lang, instructions)?);
    }
    parts.push(computer_text(lang)?);
    for code in [
        "turn-fixed-no-retry-after-reject",
        "turn-fixed-untrusted-not-consent",
        "turn-fixed-draft-only",
        "turn-fixed-task-evidence",
    ] {
        parts.push(code_text(lang, code)?);
    }
    Ok(parts)
}

fn tool_lines(lang: Lang) -> Result<Vec<String>, String> {
    let specs = [
        ("edit_file", "tool-desc-edit-file"),
        ("read_file", "tool-desc-read-file"),
        ("shell", "tool-desc-shell"),
        ("list_dir", "tool-desc-list-dir"),
        ("search", "tool-desc-search"),
        ("read_result", "tool-desc-read-result"),
        ("ask_person", "tool-desc-ask-person"),
    ];
    let mut lines = Vec::new();
    for (name, code) in specs {
        lines.push(format!("{name}: {}", code_text(lang, code)?));
    }
    Ok(lines)
}

fn expected_parts(lang: Lang, instructions: &str) -> Result<Vec<String>, String> {
    let mut parts = fixed_prompt(lang, instructions)?;
    parts.extend(tool_lines(lang)?);
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

async fn send_text(world: &TurnWorld, text: &str) -> Result<(), String> {
    world.script(vec![model_replies_text("ok", false)]);
    world
        .send(Device::Phone, text)
        .await
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

fn assert_no_english(system: &str) -> Result<(), String> {
    let identity = identity_text(Lang::En)?;
    if system.contains(&identity) {
        return Err("english identity leaked".to_owned());
    }
    for code in [
        "turn-fixed-no-retry-after-reject",
        "turn-fixed-untrusted-not-consent",
        "turn-fixed-draft-only",
        "turn-fixed-task-evidence",
    ] {
        let english = code_text(Lang::En, code)?;
        if system.contains(&english) {
            return Err(format!("english fixed block leaked: {code}"));
        }
    }
    Ok(())
}

#[tokio::test]
async fn turn_fixed_blocks_follow_order_and_language() -> Result<(), String> {
    for lang in [Some(Lang::ZhTw), None] {
        let setup = WorldSetup {
            lang,
            ..WorldSetup::default()
        };
        let world = TurnWorld::start(&setup).await?;
        send_text(&world, "hello").await?;
        let body = request_body(&world.model, 0)?;
        let system = system_content(&body)?;
        let parts = expected_parts(lang_of(lang), "Be brief.")?;
        check_increasing(&system, &parts)?;
        if lang == Some(Lang::ZhTw) {
            assert_no_english(&system)?;
        }
        send_text(&world, "again").await?;
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
    send_text(&world, "hello").await?;
    let body = request_body(&world.model, 0)?;
    let system = system_content(&body)?;
    let skipped = instructions_text(Lang::En, "")?;
    assert!(!system.contains(&skipped));
    let parts = expected_parts(Lang::En, "")?;
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
        send_text(&world, "hi").await?;
        let system1 = system_content(&request_body(&world.model, 0)?)?;
        let parts = expected_parts(lang_of(lang), "Be brief.")?;
        let expected_prefix = parts.join("\n");
        let Some(tail) = system1.strip_prefix(&expected_prefix) else {
            return Err(format!("system prefix mismatch for {lang:?}: {system1}"));
        };
        assert_eq!(tail, "\npinned snapshot v");
        if lang == Some(Lang::ZhTw) {
            assert_no_english(&system1)?;
        }
        assert!(system1.contains("Asia/Taipei"));
        assert!(!system1.contains("1700000"));
        send_text(&world, "hi").await?;
        let system2 = system_content(&request_body(&world.model, 1)?)?;
        assert_eq!(system1.as_bytes(), system2.as_bytes());
        first_systems.push(system1);
    }
    assert_ne!(first_systems.first(), first_systems.get(1));
    Ok(())
}
