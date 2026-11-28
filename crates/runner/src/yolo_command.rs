use anybot_foundation::TaskId;
use anybot_messages::{Catalog, MessageCode, Params};
use anybot_protocol::{FinishReason, KnownTaskEventKind, TaskOpener, TurnEndReason, TurnInitiator};

use crate::error::TurnError;
use crate::runner::{KindCount, TurnOutcome, TurnRunner, UserMessage};
use crate::trust_settings::YoloState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YoloArg {
    On,
    Off,
    Status,
}

const COMMANDS: [&str; 2] = ["/always-approve", "/yolo"];
const ON_WORDS: [&str; 5] = ["", "on", "\u{958b}", "\u{6253}\u{958b}", "\u{958b}\u{555f}"];
const OFF_WORDS: [&str; 7] = [
    "off",
    "stop",
    "\u{95dc}",
    "\u{95dc}\u{9589}",
    "\u{95dc}\u{6389}",
    "\u{53d6}\u{6d88}",
    "\u{505c}\u{6b62}",
];

#[must_use]
pub fn parse_yolo_arg(text: &str) -> YoloArg {
    let word = text.trim().to_lowercase();
    if ON_WORDS.contains(&word.as_str()) {
        return YoloArg::On;
    }
    if OFF_WORDS.contains(&word.as_str()) {
        return YoloArg::Off;
    }
    YoloArg::Status
}

#[must_use]
pub fn parse_command(text: &str) -> Option<YoloArg> {
    let trimmed = text.trim();
    let (head, rest) = match trimmed.split_once(char::is_whitespace) {
        Some((head, rest)) => (head, rest),
        None => (trimmed, ""),
    };
    if COMMANDS.contains(&head) {
        return Some(parse_yolo_arg(rest));
    }
    None
}

pub(crate) async fn run(
    runner: &TurnRunner,
    message: &UserMessage,
    arg: YoloArg,
) -> Result<TurnOutcome, TurnError> {
    let task = TaskId::generate(&*runner.clock);
    runner.append_user_said(&task, message).await?;
    runner
        .append_task_event(
            &task,
            KnownTaskEventKind::TaskCreated {
                goal: message.text.clone(),
                opened_by: TaskOpener::You,
                bot: runner.bot.id,
                parent: None,
            },
        )
        .await?;
    let turn = runner
        .count_task_kind(&task, KindCount::TurnStarted)
        .await?
        .saturating_add(1);
    let initiator = TurnInitiator::You {
        device: message.device,
    };
    runner
        .append_task_event_in_turn(&task, Some(turn), runner.turn_started_kind(turn, initiator))
        .await?;
    let reply = apply(runner, arg)?;
    let params = if reply.code == MessageCode::TurnYoloStatus {
        Params::new().with_text("state", reply.state)
    } else {
        Params::new()
    };
    let text = Catalog::new().format(runner.lang, &reply.code, &params)?;
    runner
        .append_task_event(
            &task,
            KnownTaskEventKind::ModelAnswer {
                text,
                thinking: None,
                tool_requests: Vec::new(),
                finish: FinishReason::Finished,
                native: None,
            },
        )
        .await?;
    let ctx = runner.approval_context(task, turn);
    runner
        .append_turn_end(&ctx, TurnEndReason::Answered, true)
        .await?;
    Ok(TurnOutcome {
        task,
        turn,
        reason: TurnEndReason::Answered,
        pending: Vec::new(),
        unread_steers: Vec::new(),
    })
}

struct Reply {
    code: MessageCode,
    state: &'static str,
}

fn apply(runner: &TurnRunner, arg: YoloArg) -> Result<Reply, TurnError> {
    let current = runner.trust.yolo()?;
    match arg {
        YoloArg::Status => Ok(Reply {
            code: MessageCode::TurnYoloStatus,
            state: if current == YoloState::Off {
                "off"
            } else {
                "on"
            },
        }),
        YoloArg::On => {
            turn_on(runner, current)?;
            Ok(Reply {
                code: MessageCode::TurnYoloOn,
                state: "on",
            })
        }
        YoloArg::Off => {
            turn_off(runner, current)?;
            Ok(Reply {
                code: MessageCode::TurnYoloOff,
                state: "off",
            })
        }
    }
}

fn turn_on(runner: &TurnRunner, current: YoloState) -> Result<(), TurnError> {
    if current == YoloState::Off {
        let direct_was_on = runner.trust.modes()?.direct_mode;
        if !direct_was_on {
            runner.trust.set_direct_mode(true)?;
        }
        let next = if direct_was_on {
            YoloState::On
        } else {
            YoloState::OnOpenedDirect
        };
        runner.trust.set_yolo(next)?;
    }
    Ok(())
}

fn turn_off(runner: &TurnRunner, current: YoloState) -> Result<(), TurnError> {
    if current == YoloState::OnOpenedDirect {
        runner.trust.set_direct_mode(false)?;
    }
    runner.trust.set_yolo(YoloState::Off)?;
    Ok(())
}

#[cfg(test)]
#[path = "yolo_command_tests.rs"]
mod yolo_command_tests;
