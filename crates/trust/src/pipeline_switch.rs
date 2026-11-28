use anybot_protocol::{
    CardButton, CardKind, ComputerCaps, GateOutcome, GateRecord, GateStep, JudgementSource,
    ReasonPayload, RuleLayer,
};

use crate::action::GateAction;
use crate::card_data::build_card_for_computer;
use crate::card_view::{CardView, CardViewError};
use crate::computer_use::SwitchReason;
use crate::message_codes_flow::{
    CODE_CARD_SWITCH, CODE_CARD_SWITCH_ACTION, CODE_CARD_SWITCH_ALWAYS_BOT,
    CODE_CARD_SWITCH_ALWAYS_LOCAL, CODE_GATE_SWITCH_DECLINED_BOT, CODE_GATE_SWITCH_DECLINED_LOCAL,
};
use crate::pipeline::ActionRequest;
use crate::pipeline_ask::{AskTicket, CallContext, prepared_action};
use crate::pipeline_run::OutcomeText;
use crate::runner::GatedComputer;

pub const SWITCH_RULE: &str = "computer-use-switch";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchInfo {
    pub bot: String,
    pub reasons: Vec<SwitchReason>,
    pub direct_button: bool,
}

impl SwitchInfo {
    #[must_use]
    pub fn needs_bot_tick(&self) -> bool {
        self.reasons.contains(&SwitchReason::BotNotAllowed)
    }

    #[must_use]
    pub fn needs_local_allow(&self) -> bool {
        self.reasons.contains(&SwitchReason::AskEveryTime)
    }
}

#[must_use]
pub fn switch_ticket(
    request: &ActionRequest,
    ctx: &CallContext,
    computer: &dyn GatedComputer,
    info: SwitchInfo,
) -> AskTicket {
    let action = prepared_action(request, computer);
    let gate = GateRecord::new(
        GateStep::ComputerUse,
        RuleLayer::User,
        SWITCH_RULE.to_owned(),
        JudgementSource::Rule,
        GateOutcome::Ask,
        false,
    );
    AskTicket::new(
        GateOutcome::Ask,
        gate,
        action,
        &computer.declared_caps(),
        request,
        computer.name(),
        ctx,
    )
    .with_switch(info)
}

fn payload(code: &str, params: &[(&str, &str)]) -> ReasonPayload {
    ReasonPayload {
        code: code.to_owned(),
        params: params
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
    }
}

pub(crate) fn switch_card_view(
    action: &GateAction,
    caps: &ComputerCaps,
    computer: &str,
    info: &SwitchInfo,
) -> Result<CardView, CardViewError> {
    let data = build_card_for_computer(action, caps, computer);
    let Some(primary) = data.actions.first().cloned() else {
        return Err(CardViewError::MissingAction);
    };
    let mut buttons = vec![
        CardButton::AllowOnce,
        CardButton::AllowConversation,
        CardButton::AlwaysAllow,
        CardButton::Deny,
    ];
    if info.direct_button {
        buttons.push(CardButton::DirectMode);
    }
    let always_label = if info.needs_bot_tick() {
        payload(
            CODE_CARD_SWITCH_ALWAYS_BOT,
            &[("bot", &info.bot), ("computer", computer)],
        )
    } else {
        payload(CODE_CARD_SWITCH_ALWAYS_LOCAL, &[("computer", computer)])
    };
    Ok(CardView {
        action: primary.clone(),
        actions: data.actions,
        kind: CardKind::Switch,
        buttons,
        reason: payload(
            CODE_CARD_SWITCH,
            &[("bot", &info.bot), ("computer", computer)],
        ),
        always_label: Some(always_label),
        notes: vec![payload(
            CODE_CARD_SWITCH_ACTION,
            &[("computer", computer), ("action", &primary)],
        )],
        classes: data.classes,
    })
}

#[must_use]
pub(crate) fn switch_decline_text(info: &SwitchInfo, computer: &str) -> OutcomeText {
    if info.needs_bot_tick() {
        return OutcomeText::of(
            CODE_GATE_SWITCH_DECLINED_BOT,
            vec![
                ("bot".to_owned(), info.bot.clone()),
                ("computer".to_owned(), computer.to_owned()),
            ],
        );
    }
    OutcomeText::of(
        CODE_GATE_SWITCH_DECLINED_LOCAL,
        vec![("computer".to_owned(), computer.to_owned())],
    )
}
