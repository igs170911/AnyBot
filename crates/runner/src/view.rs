use anybot_messages::{Catalog, Lang, MessageCode, Params};
use anybot_protocol::{ActualResult, GateRecord};
use anybot_trust::{ModelResultView, OpRejection, OutcomeText};

use crate::error::TurnError;

#[derive(Debug)]
pub(crate) struct ActionView {
    lang: Lang,
    edit_lines: u32,
    read_from: u32,
    read_to: u32,
    output: std::sync::Mutex<Option<String>>,
}

impl ActionView {
    #[must_use]
    pub(crate) fn new(lang: Lang) -> Self {
        ActionView {
            lang,
            edit_lines: 0,
            read_from: 1,
            read_to: 0,
            output: std::sync::Mutex::new(None),
        }
    }

    #[must_use]
    pub(crate) fn with_edit_lines(mut self, lines: u32) -> Self {
        self.edit_lines = lines;
        self
    }

    #[must_use]
    pub(crate) fn with_read_range(mut self, from: u32, to: u32) -> Self {
        self.read_from = from;
        self.read_to = to;
        self
    }

    pub(crate) fn render(&self, text: &OutcomeText) -> Result<String, TurnError> {
        render_text(self.lang, text)
    }
}

impl ModelResultView for ActionView {
    fn result_text(&self, _tool: &str, result: &ActualResult) -> OutcomeText {
        match result {
            ActualResult::Command(outcome) => {
                self.capture(&outcome.output);
                command_text(outcome.exit_code)
            }
            ActualResult::Write(outcome) => write_text(outcome, self.edit_lines),
            ActualResult::Read(outcome) => OutcomeText::of(
                "tool-read-lines",
                vec![
                    ("path".to_owned(), outcome.path.clone()),
                    ("from".to_owned(), self.read_from.to_string()),
                    ("to".to_owned(), self.read_to.to_string()),
                ],
            ),
            ActualResult::Path(_) | ActualResult::Caps(_) => {
                OutcomeText::of("tool-empty", Vec::new())
            }
        }
    }

    fn failure_text(&self, rejection: &OpRejection) -> OutcomeText {
        OutcomeText::of(
            "tool-operation-failed",
            vec![("reason".to_owned(), rejection.reason.clone())],
        )
    }

    fn rejection_text(&self, gate: &GateRecord, params: &[(String, String)]) -> OutcomeText {
        let mut all = vec![("rule".to_owned(), gate.rule.clone())];
        all.extend(params.iter().map(|(k, v)| (k.clone(), v.clone())));
        OutcomeText::of("gate-denied", all)
    }
}

impl ActionView {
    fn capture(&self, output: &str) {
        if let Ok(mut guard) = self.output.lock() {
            *guard = Some(output.to_owned());
        }
    }

    #[must_use]
    pub(crate) fn take_output(&self) -> Option<String> {
        let Ok(mut guard) = self.output.lock() else {
            return None;
        };
        guard.take()
    }
}

fn command_text(exit_code: Option<i32>) -> OutcomeText {
    match exit_code {
        Some(0) | None => OutcomeText::of("turn-completed", Vec::new()),
        Some(code) => OutcomeText::of(
            "tool-shell-failed",
            vec![("code".to_owned(), code.to_string())],
        ),
    }
}

fn write_text(outcome: &anybot_protocol::WriteOutcome, lines: u32) -> OutcomeText {
    match outcome.before {
        anybot_protocol::BeforeDigest::Absent => OutcomeText::of(
            "tool-edit-created",
            vec![
                ("path".to_owned(), outcome.path.clone()),
                ("lines".to_owned(), lines.to_string()),
            ],
        ),
        anybot_protocol::BeforeDigest::Present(_) => OutcomeText::of(
            "tool-edit-modified",
            vec![("path".to_owned(), outcome.path.clone())],
        ),
    }
}

pub(crate) fn render_text(lang: Lang, text: &OutcomeText) -> Result<String, TurnError> {
    let Some(code) = MessageCode::from_code(&text.code) else {
        return Err(TurnError::UnknownMessageCode {
            code: text.code.clone(),
        });
    };
    let mut params = Params::new();
    for (key, value) in &text.params {
        params = params.with_text(key, value);
    }
    let mut catalog = Catalog::new();
    catalog
        .format(lang, &code, &params)
        .map_err(TurnError::Messages)
}
