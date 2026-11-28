use anybot_protocol::{
    CapState, CardButton, CardKind, ComputerCaps, ComputerOwner, ComputerPlatform,
};

use crate::action::{ActionSubject, GateAction};
use crate::card_view::{CardViewError, card_view};
use crate::gate_flow::{GateConfig, judge};
use crate::message_codes::CODE_CARD_NOTE_NO_SANDBOX_PRIVATE;

#[test]
fn card_without_action_is_error() {
    let caps = ComputerCaps::declared(ComputerPlatform::Linux, ComputerOwner::Bot);
    let action = GateAction {
        tool: "caps".to_owned(),
        subject: ActionSubject::Capabilities,
        extra_paths: Vec::new(),
        workdir: "/workspace".to_owned(),
    };
    let verdict = judge(&action, &caps, &GateConfig::plain(), "bot-1");
    let err = card_view(&verdict, &action, &caps, "bot-1").unwrap_err();
    assert_eq!(err, CardViewError::MissingAction);
}

#[test]
fn card_view_renders_no_sandbox_and_admin_cards() {
    let mut caps = ComputerCaps::declared(ComputerPlatform::Mac, ComputerOwner::User);
    caps.command_sandbox = CapState::Unsupported;
    let action = GateAction::command("shell", "ls", "/workspace", Vec::new());
    let verdict = judge(&action, &caps, &GateConfig::plain(), "Work Mac");
    let view = card_view(&verdict, &action, &caps, "Work Mac").unwrap();
    assert_eq!(view.kind, CardKind::EveryTime);
    assert_eq!(view.buttons, vec![CardButton::AllowOnce, CardButton::Deny]);
    assert!(
        view.notes
            .iter()
            .any(|n| n.code == CODE_CARD_NOTE_NO_SANDBOX_PRIVATE)
    );
    assert!(view.always_label.is_none());
}
