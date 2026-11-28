use anybot_foundation::Clock;
use anybot_protocol::{
    GateOutcome, GateRecord, GateStep, JudgementSource, ReasonPayload, RuleLayer,
};

use crate::computer_use::StepFour;
use crate::evidence::rejected_evidence;
use crate::message_codes_answers::CODE_GATE_RULE_UNREADABLE;
use crate::message_codes_flow::CODE_GATE_LOCAL_LOCKED;
use crate::pipeline::ActionRequest;
use crate::pipeline_exec::finish;
use crate::pipeline_run::{ActionRun, OutcomeText};

const RULE_LOCAL_NEVER: &str = "local-never";
const RULE_LOCAL_LOCKED: &str = "local-locked";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepDenial {
    pub step: GateStep,
    pub layer: RuleLayer,
    pub rule: String,
    pub code: String,
    pub params: Vec<(String, String)>,
}

#[must_use]
pub fn step_four_denial(step: &StepFour, computer: &str) -> Option<StepDenial> {
    let StepFour::Deny { code } = step else {
        return None;
    };
    let locked = *code == CODE_GATE_LOCAL_LOCKED;
    Some(StepDenial {
        step: GateStep::ComputerUse,
        layer: if locked {
            RuleLayer::Admin
        } else {
            RuleLayer::User
        },
        rule: if locked {
            RULE_LOCAL_LOCKED
        } else {
            RULE_LOCAL_NEVER
        }
        .to_owned(),
        code: (*code).to_owned(),
        params: vec![("computer".to_owned(), computer.to_owned())],
    })
}

#[must_use]
pub fn unreadable_denial(rule: &str) -> StepDenial {
    StepDenial {
        step: GateStep::AllowLists,
        layer: RuleLayer::User,
        rule: "rule-unreadable".to_owned(),
        code: CODE_GATE_RULE_UNREADABLE.to_owned(),
        params: vec![("rule".to_owned(), rule.to_owned())],
    }
}

#[must_use]
pub fn step_denied_run(
    request: &ActionRequest,
    denial: &StepDenial,
    clock: &dyn Clock,
) -> ActionRun {
    let gate = GateRecord::new(
        denial.step,
        denial.layer,
        denial.rule.clone(),
        JudgementSource::Rule,
        GateOutcome::Deny,
        false,
    );
    let reason = ReasonPayload {
        code: denial.code.clone(),
        params: denial.params.clone(),
    };
    let evidence = rejected_evidence(&request.context, &gate, reason, clock);
    let model = OutcomeText::of(&denial.code, denial.params.clone());
    finish(gate, vec![evidence], model, &request.context)
}
