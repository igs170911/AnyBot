use std::sync::Mutex;

use anybot_protocol::{AnswerOrigin, CardAnswer, ToolRequest};
use anybot_trust::StepDenial;

use crate::error::TurnError;
use crate::runner::TurnRunner;

#[derive(Debug, Default)]
pub(crate) struct RejectMemory {
    entries: Mutex<Vec<(String, String)>>,
}

impl RejectMemory {
    pub(crate) fn remember(&self, tool: &str, args: &str) -> Result<(), TurnError> {
        let Ok(mut guard) = self.entries.lock() else {
            return Err(TurnError::ParkedBusy);
        };
        let key = (tool.to_owned(), args.to_owned());
        if !guard.contains(&key) {
            guard.push(key);
        }
        Ok(())
    }

    pub(crate) fn contains(&self, tool: &str, args: &str) -> Result<bool, TurnError> {
        let Ok(guard) = self.entries.lock() else {
            return Err(TurnError::ParkedBusy);
        };
        Ok(guard.iter().any(|(t, a)| t == tool && a == args))
    }

    pub(crate) fn clear(&self) -> Result<(), TurnError> {
        let Ok(mut guard) = self.entries.lock() else {
            return Err(TurnError::ParkedBusy);
        };
        guard.clear();
        Ok(())
    }
}

pub(crate) fn should_remember(answer: CardAnswer, origin: AnswerOrigin) -> bool {
    answer == CardAnswer::Deny && origin == AnswerOrigin::Hand
}

pub(crate) fn note_answer(
    runner: &TurnRunner,
    request: &ToolRequest,
    answer: CardAnswer,
    origin: AnswerOrigin,
) -> Result<(), TurnError> {
    if !should_remember(answer, origin) {
        return Ok(());
    }
    let args = crate::action::args_json(&request.arguments)?;
    runner.rejects.remember(&request.tool, &args)
}

pub(crate) fn repeat_denial(
    runner: &TurnRunner,
    request: &ToolRequest,
) -> Result<Option<StepDenial>, TurnError> {
    let args = crate::action::args_json(&request.arguments)?;
    if runner.rejects.contains(&request.tool, &args)? {
        return Ok(Some(anybot_trust::rejected_just_now_denial()));
    }
    Ok(None)
}

#[cfg(test)]
#[path = "reject_memory_tests.rs"]
mod reject_memory_tests;
