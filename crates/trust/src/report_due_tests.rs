use anybot_foundation::{BotId, CallId, DeviceId, EventId, SystemClock, TimestampMs};
use anybot_protocol::{
    AttentionKind, BudgetItem, ComputerPlatform, ComputerRef, FinishReason, GateOutcome,
    GateRecord, GateStep, JudgementSource, KnownTaskEventKind, ModelName, NotRunReason, RuleLayer,
    TaskDetail, TaskEvent, TaskEventKind, TaskOpener, TaskStatus, ThinkingChoice, ToolArgs,
    ToolEffect, ToolRequest, TurnEndReason, TurnInitiator,
};

use crate::{ReportDue, delivery_report, report_due};

struct Script {
    events: Vec<TaskEvent>,
    seq: u64,
}

impl Script {
    fn new() -> Self {
        let mut script = Script {
            events: Vec::new(),
            seq: 0,
        };
        script.push(KnownTaskEventKind::TaskCreated {
            goal: "goal".to_owned(),
            opened_by: TaskOpener::You,
            bot: BotId::generate(&SystemClock),
            parent: None,
        });
        script.push(KnownTaskEventKind::TurnStarted {
            turn: 1,
            initiator: TurnInitiator::You {
                device: DeviceId::generate(&SystemClock),
            },
            computer: ComputerRef {
                name: "c".to_owned(),
                platform: ComputerPlatform::BotMachine,
            },
            model: ModelName {
                provider: "OpenAi".to_owned(),
                base_url: "http://127.0.0.1:9/v1".to_owned(),
                model: "m".to_owned(),
            },
            thinking: ThinkingChoice::Auto,
        });
        script
    }

    fn push(&mut self, kind: KnownTaskEventKind) {
        self.seq = self.seq.saturating_add(1);
        let seq = self.seq;
        self.events.push(TaskEvent {
            event_id: EventId::generate(&SystemClock),
            seq,
            at_ms: TimestampMs::from_millis(seq),
            kind: TaskEventKind::Known(kind),
        });
    }

    fn answer(&mut self, text: &str, requests: Vec<ToolRequest>) {
        self.push(KnownTaskEventKind::ModelAnswer {
            text: text.to_owned(),
            thinking: None,
            tool_requests: requests,
            finish: FinishReason::Finished,
            native: None,
        });
    }

    fn end(&mut self) {
        self.push(KnownTaskEventKind::TurnEnded {
            reason: TurnEndReason::Answered,
            reply_delivered: true,
        });
    }

    fn run_call(&mut self, call: &ToolRequest) {
        self.intent(call);
        self.push(KnownTaskEventKind::ToolStarted {
            call_id: call.call_id,
        });
        self.push(KnownTaskEventKind::ToolCompleted {
            call_id: call.call_id,
            summary: "ok".to_owned(),
            raw_id: None,
        });
    }

    fn gate(&mut self, call: &ToolRequest, source: JudgementSource) {
        self.push(KnownTaskEventKind::GateResult {
            call_id: call.call_id,
            gate: GateRecord::new(
                GateStep::AskYou,
                RuleLayer::User,
                "rule".to_owned(),
                source,
                GateOutcome::Allow,
                false,
            ),
        });
    }

    fn intent(&mut self, call: &ToolRequest) {
        self.push(KnownTaskEventKind::ToolIntent {
            call_id: call.call_id,
            name: call.tool.clone(),
            args_summary: "{}".to_owned(),
            level: ToolEffect::Irreversible,
        });
    }
}

fn edit_request(path: &str) -> ToolRequest {
    ToolRequest {
        call_id: CallId::generate(&SystemClock),
        provider_call_id: "p".to_owned(),
        tool: "edit_file".to_owned(),
        arguments: ToolArgs::Json {
            value: serde_json::json!({ "path": path, "old_text": "", "new_text": "1" }),
        },
    }
}

fn chat_only() -> Script {
    let mut script = Script::new();
    script.answer("hello", Vec::new());
    script.end();
    script
}

fn edited() -> Script {
    let mut script = Script::new();
    let call = edit_request("a.txt");
    script.answer("", vec![call.clone()]);
    script.run_call(&call);
    script.answer("done", Vec::new());
    script.end();
    script
}

fn over_budget() -> Script {
    let mut script = Script::new();
    script.push(KnownTaskEventKind::BudgetLimit {
        item: BudgetItem::ToolCalls,
        limit: 1,
    });
    let first = edit_request("a.txt");
    let second = edit_request("b.txt");
    script.answer("", vec![first.clone(), second.clone()]);
    script.run_call(&first);
    script.intent(&second);
    script.push(KnownTaskEventKind::ToolNotRun {
        call_id: second.call_id,
        reason: NotRunReason::OverBudget,
    });
    script.end();
    script
}

#[test]
fn report_due_none_for_chat_only() {
    assert_eq!(report_due(&chat_only().events), Ok(ReportDue::None));
}

#[test]
fn report_due_full_after_an_action() {
    assert_eq!(report_due(&edited().events), Ok(ReportDue::Full));
}

#[test]
fn report_due_none_after_stop_or_cancel() {
    let mut stopped = edited();
    stopped.push(KnownTaskEventKind::TaskStopped);
    assert_eq!(report_due(&stopped.events), Ok(ReportDue::None));
    let mut cancelled = edited();
    cancelled.push(KnownTaskEventKind::TaskCancelled {
        reason: "user".to_owned(),
    });
    assert_eq!(report_due(&cancelled.events), Ok(ReportDue::None));
}

#[test]
fn report_due_progress_when_over_budget() {
    let script = over_budget();
    assert_eq!(report_due(&script.events), Ok(ReportDue::Progress));
    let view = anybot_domain::replay(&script.events).unwrap();
    assert_eq!(view.status, TaskStatus::Stuck);
    assert_eq!(
        view.detail,
        Some(TaskDetail::OverBudget {
            item: BudgetItem::ToolCalls,
            used: 1,
            limit: 1
        })
    );
}

#[test]
fn over_budget_report_lists_done_and_not_done() {
    let script = over_budget();
    let report = delivery_report(&script.events, "goal", None).unwrap();
    let progress = report.progress.unwrap();
    assert_eq!(progress.done, ["a.txt"]);
    assert_eq!(progress.not_done, ["b.txt"]);
    assert_eq!(progress.stuck.code, crate::CODE_REPORT_STUCK_OVER_BUDGET);
    assert_eq!(
        progress.stuck.params,
        [
            ("used".to_owned(), "1".to_owned()),
            ("limit".to_owned(), "1".to_owned()),
            ("kind".to_owned(), "tool_calls".to_owned())
        ]
    );
    assert_eq!(progress.next.code, crate::CODE_REPORT_NEXT_RAISE_LIMIT);
    assert_eq!(
        progress.next.params,
        [("kind".to_owned(), "tool_calls".to_owned())]
    );
}

#[test]
fn finished_report_has_no_progress() {
    let report = delivery_report(&edited().events, "goal", Some("done")).unwrap();
    assert_eq!(report.progress, None);
}

fn approved_by(source: JudgementSource) -> Script {
    let mut script = Script::new();
    let call = edit_request("a.txt");
    script.answer("", vec![call.clone()]);
    script.gate(&call, source);
    script.run_call(&call);
    script.answer("done", Vec::new());
    script.end();
    script
}

#[test]
fn timeout_answer_is_listed_in_attention() {
    let script = approved_by(JudgementSource::RuleTimeout);
    assert_eq!(report_due(&script.events), Ok(ReportDue::Full));
    let report = delivery_report(&script.events, "goal", None).unwrap();
    let kinds: Vec<AttentionKind> = report.attention.iter().map(|a| a.kind).collect();
    assert_eq!(kinds, [AttentionKind::TimeoutAuto]);
    let item = report.attention.first().unwrap();
    assert_eq!(item.tool, "edit_file");
}

#[test]
fn always_approve_answer_is_listed_in_attention() {
    let script = approved_by(JudgementSource::AlwaysApprove);
    let report = delivery_report(&script.events, "goal", None).unwrap();
    let kinds: Vec<AttentionKind> = report.attention.iter().map(|a| a.kind).collect();
    assert_eq!(kinds, [AttentionKind::Yolo]);
}

#[test]
fn hand_answer_is_not_listed_in_attention() {
    let script = approved_by(JudgementSource::You);
    let report = delivery_report(&script.events, "goal", None).unwrap();
    assert!(report.attention.is_empty());
}
