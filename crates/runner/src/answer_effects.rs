use anybot_protocol::{CardAnswer, CardButton};
use anybot_trust::{AlwaysEntry, SwitchInfo};

use crate::approvals::PendingCard;
use crate::error::TurnError;
use crate::runner::TurnRunner;
use crate::trust_settings::ComputerUseChange;

pub(crate) fn button_of(answer: CardAnswer) -> CardButton {
    match answer {
        CardAnswer::AllowOnce => CardButton::AllowOnce,
        CardAnswer::AllowConversation => CardButton::AllowConversation,
        CardAnswer::AlwaysAllow => CardButton::AlwaysAllow,
        CardAnswer::Deny => CardButton::Deny,
        CardAnswer::DirectMode => CardButton::DirectMode,
    }
}

pub(crate) fn ensure_offered(card: &PendingCard, answer: CardAnswer) -> Result<(), TurnError> {
    if card.buttons.contains(&button_of(answer)) {
        return Ok(());
    }
    Err(TurnError::AnswerNotOffered {
        digest: card.digest.clone(),
    })
}

pub(crate) fn apply(
    runner: &TurnRunner,
    card: &PendingCard,
    answer: CardAnswer,
) -> Result<(), TurnError> {
    let settings = &*runner.trust;
    let bot = &runner.bot.id;
    match answer {
        CardAnswer::AllowOnce | CardAnswer::Deny => Ok(()),
        CardAnswer::AllowConversation => {
            if card.ticket.switch_info().is_some() {
                return Ok(());
            }
            let key = card.ticket.action_key();
            settings.allow_in_conversation(bot, runner.conversation(), &key)?;
            Ok(())
        }
        CardAnswer::DirectMode => {
            settings.set_direct_mode(true)?;
            Ok(())
        }
        CardAnswer::AlwaysAllow => match card.ticket.switch_info() {
            Some(info) => tick_switch(runner, card, info),
            None => write_always(runner, card),
        },
    }
}

fn tick_switch(
    runner: &TurnRunner,
    card: &PendingCard,
    info: &SwitchInfo,
) -> Result<(), TurnError> {
    let settings = &*runner.trust;
    let bot = &runner.bot.id;
    if info.needs_bot_tick() {
        settings.set_computer_use(bot, &card.computer, ComputerUseChange::AllowBot)?;
    }
    if info.needs_local_allow() {
        settings.set_computer_use(bot, &card.computer, ComputerUseChange::RunHereAlways)?;
    }
    Ok(())
}

fn write_always(runner: &TurnRunner, card: &PendingCard) -> Result<(), TurnError> {
    let settings = &*runner.trust;
    for action in &card.actions {
        settings.add_always(AlwaysEntry {
            action: action.clone(),
            added_at: runner.clock.now_ms(),
            from_bot: runner.bot.id,
        })?;
    }
    if let Some(rule) = card.ticket.ask_rule_label() {
        settings.remove_ask_rule(&rule)?;
    }
    Ok(())
}
