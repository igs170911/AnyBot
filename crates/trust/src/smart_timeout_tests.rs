use anybot_protocol::{ComputerOwner, RuleLayer};

use crate::card_buttons::AskOrigin;
use crate::smart_timeout::{
    SmartCase, SmartChoice, ToolRisk, smart_choice, smart_seconds, tool_risk,
};

fn case(origin: &AskOrigin, risk: ToolRisk) -> SmartCase<'_> {
    SmartCase {
        origin,
        risk,
        has_classes: false,
        is_switch: false,
        mdm_locked: false,
    }
}

#[test]
fn smart_seconds_clamp() {
    let got: Vec<u32> = [0, 15, 3600, 3700].map(smart_seconds).to_vec();
    assert_eq!(got, vec![30, 30, 3600, 3600]);
}

#[test]
fn smart_seconds_floor_to_the_grid() {
    assert_eq!(smart_seconds(45), 30);
    assert_eq!(smart_seconds(300), 300);
    assert_eq!(smart_seconds(-5), 30);
}

#[test]
fn smart_choice_covers_every_class() {
    let default = AskOrigin::Default;
    assert_eq!(
        smart_choice(&case(&default, ToolRisk::LowRisk)),
        SmartChoice::AutoAllow
    );
    assert_eq!(
        smart_choice(&case(&default, ToolRisk::BotWork)),
        SmartChoice::AutoAllow
    );
    assert_eq!(
        smart_choice(&case(&default, ToolRisk::Sensitive)),
        SmartChoice::AutoDeny
    );
    assert_eq!(
        smart_choice(&case(&default, ToolRisk::Unknown)),
        SmartChoice::Never
    );
}

#[test]
fn smart_choice_never_answers_cards_only_a_person_may_answer() {
    let origins = [
        AskOrigin::EveryTime,
        AskOrigin::UserAskRule {
            rule: "shell(git push*)".to_owned(),
        },
        AskOrigin::UserHook,
        AskOrigin::AdminOrFloor {
            layer: RuleLayer::Floor,
        },
        AskOrigin::Untrusted,
    ];
    for origin in &origins {
        assert_eq!(
            smart_choice(&case(origin, ToolRisk::LowRisk)),
            SmartChoice::Never
        );
    }
    let default = AskOrigin::Default;
    let six = SmartCase {
        has_classes: true,
        ..case(&default, ToolRisk::BotWork)
    };
    let switch = SmartCase {
        is_switch: true,
        ..case(&default, ToolRisk::BotWork)
    };
    let locked = SmartCase {
        mdm_locked: true,
        ..case(&default, ToolRisk::LowRisk)
    };
    assert_eq!(smart_choice(&six), SmartChoice::Never);
    assert_eq!(smart_choice(&switch), SmartChoice::Never);
    assert_eq!(smart_choice(&locked), SmartChoice::Never);
}

#[test]
fn tool_risk_trusts_files_only_on_the_bot_computer() {
    assert_eq!(
        tool_risk("read_file", ComputerOwner::User),
        ToolRisk::LowRisk
    );
    assert_eq!(tool_risk("shell", ComputerOwner::User), ToolRisk::BotWork);
    assert_eq!(
        tool_risk("edit_file", ComputerOwner::Bot),
        ToolRisk::BotWork
    );
    assert_eq!(
        tool_risk("edit_file", ComputerOwner::User),
        ToolRisk::Unknown
    );
    assert_eq!(tool_risk("click", ComputerOwner::User), ToolRisk::Unknown);
}
