use anybot_messages::{Lang, Params};
use anybot_protocol::{
    CardKind, ComputerOwner, ComputerPlatform, EvidenceKind, EvidencePayload, KnownTaskEventKind,
    ReasonPayload,
};
use anybot_trust::{ComputerUse, GateConfig, ModeFlags, RunHere};

use crate::fake_scripts::CommandScript;
use crate::turn::{Device, TurnWorld, WorldSetup};
use crate::turn_helpers::fmt_text;
use crate::turn_tool_helpers::{all_specs, call, one_tool_step};

pub const MAC: &str = "Work Mac";

pub fn mac_setup(name: &str) -> WorldSetup {
    WorldSetup {
        lang: Some(Lang::ZhTw),
        owner: ComputerOwner::User,
        platform: ComputerPlatform::Mac,
        computer_name: name.to_owned(),
        bot_name: "Bot1".to_owned(),
        workdir: "/home/alice/work".to_owned(),
        home_dir: "/home/alice".to_owned(),
        tools: all_specs(),
        files: vec![
            ("/home/alice/Documents/a.txt".to_owned(), "doc".to_owned()),
            ("/home/alice/Downloads/b.txt".to_owned(), "dl".to_owned()),
            ("/home/alice/work/a.md".to_owned(), "old".to_owned()),
            ("/home/alice/work/n.txt".to_owned(), "n".to_owned()),
        ],
        scripts: vec![
            ("ls".to_owned(), CommandScript::ok("a.md")),
            ("make test".to_owned(), CommandScript::ok("ok")),
        ],
        ..WorldSetup::default()
    }
}

pub fn unticked() -> ComputerUse {
    ComputerUse {
        usable_by_bot: false,
        run_here: RunHere::AlwaysAllow,
        this_machine: true,
    }
}

pub fn ask_every_time() -> ComputerUse {
    ComputerUse {
        usable_by_bot: true,
        run_here: RunHere::AskEveryTime,
        this_machine: true,
    }
}

pub fn never() -> ComputerUse {
    ComputerUse {
        usable_by_bot: true,
        run_here: RunHere::Never,
        this_machine: true,
    }
}

pub fn cards_of(kinds: &[KnownTaskEventKind]) -> Vec<KnownTaskEventKind> {
    kinds
        .iter()
        .filter(|k| matches!(k, KnownTaskEventKind::CardShown { .. }))
        .cloned()
        .collect()
}

pub fn rejected_reason(kinds: &[KnownTaskEventKind]) -> Option<ReasonPayload> {
    kinds.iter().find_map(|k| {
        let KnownTaskEventKind::EvidenceAdded { record } = k else {
            return None;
        };
        if record.kind != EvidenceKind::Rejected {
            return None;
        }
        if let EvidencePayload::Rejected { reason } = &record.payload {
            Some(reason.clone())
        } else {
            None
        }
    })
}

pub fn param<'a>(reason: &'a ReasonPayload, key: &str) -> Option<&'a str> {
    reason
        .params
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

pub async fn first_turn(
    world: &TurnWorld,
    tool: &str,
    args: &str,
) -> Result<anybot_runner::TurnOutcome, String> {
    world.script(one_tool_step(vec![call(tool, args, "c1")]));
    world
        .send(Device::Phone, "go")
        .await
        .map_err(|e| format!("{e:?}"))
}

pub fn modes(yolo: bool, direct: bool) -> GateConfig {
    GateConfig {
        modes: ModeFlags {
            ask_every_time: false,
            yolo,
            direct_mode: direct,
            mdm_locks_direct: false,
        },
        ..GateConfig::plain()
    }
}

pub fn card_reason(
    cards: &[KnownTaskEventKind],
) -> Result<(ReasonPayload, Option<ReasonPayload>, CardKind), String> {
    let Some(KnownTaskEventKind::CardShown {
        reason,
        always_label,
        kind,
        ..
    }) = cards.first()
    else {
        return Err("missing CardShown".to_owned());
    };
    Ok((reason.clone(), always_label.clone(), *kind))
}

pub fn render(payload: &ReasonPayload) -> Result<String, String> {
    let mut params = Params::new();
    for (key, value) in &payload.params {
        params = params.with_text(key, value);
    }
    fmt_text(Lang::ZhTw, &payload.code, &params)
}
