use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_protocol as _;
use anybot_runner as _;
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
        let parts: Vec<String> = vec![
            identity_text(lang_of(lang))?,
            instructions_text(lang_of(lang), "Be brief.")?,
            computer_text(lang_of(lang))?,
            "read_file: Read a file".to_owned(),
        ];
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
    check_increasing(
        &system,
        &[identity_text(Lang::En)?, computer_text(Lang::En)?],
    )
}

use anybot_context as _;
use anybot_domain as _;
use sha2 as _;
