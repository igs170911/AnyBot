use anybot_protocol::{ActualResult, GateRecord};
use anybot_trust::{ModelResultView, OpRejection, OutcomeText};

#[derive(Debug, Default)]
pub struct TestView;

impl ModelResultView for TestView {
    fn result_text(&self, tool: &str, result: &ActualResult) -> OutcomeText {
        let code = format!("tool-{tool}-result");
        OutcomeText::of(&code, result_params(result))
    }

    fn failure_text(&self, rejection: &OpRejection) -> OutcomeText {
        OutcomeText::of(
            anybot_trust::CODE_TOOL_EXECUTION_FAILED,
            vec![("reason".to_owned(), rejection.reason.clone())],
        )
    }

    fn rejection_text(&self, gate: &GateRecord, params: &[(String, String)]) -> OutcomeText {
        let base = vec![("rule".to_owned(), gate.rule.clone())];
        let mut all = base;
        all.extend(params.iter().map(|(k, v)| (k.clone(), v.clone())));
        OutcomeText::of(anybot_trust::CODE_GATE_DENIED, all)
    }
}

fn result_params(result: &ActualResult) -> Vec<(String, String)> {
    match result {
        ActualResult::Command(outcome) => vec![
            ("exit".to_owned(), exit_word(outcome.exit_code)),
            ("elapsed".to_owned(), outcome.elapsed_ms.to_string()),
        ],
        ActualResult::Write(outcome) => vec![
            ("path".to_owned(), outcome.path.clone()),
            ("after".to_owned(), outcome.after.hex().to_owned()),
        ],
        ActualResult::Read(outcome) => vec![
            ("path".to_owned(), outcome.path.clone()),
            ("digest".to_owned(), outcome.digest.hex().to_owned()),
        ],
        ActualResult::List(outcome) => vec![
            ("path".to_owned(), outcome.canonical.clone()),
            ("count".to_owned(), outcome.entries.len().to_string()),
        ],
        ActualResult::Path(outcome) => vec![("canonical".to_owned(), outcome.canonical.clone())],
        ActualResult::Caps(caps) => vec![("caps".to_owned(), caps.capability_label())],
    }
}

fn exit_word(code: Option<i32>) -> String {
    match code {
        Some(value) => value.to_string(),
        None => "unknown".to_owned(),
    }
}
