use anybot_foundation::CallId;
use anybot_protocol::{
    AttentionKind, EvidenceKind, EvidencePayload, JudgementSource, KnownTaskEventKind,
    ReportAttention, TaskEvent, TaskEventKind, ToolArgs,
};

#[derive(Default)]
struct Seen {
    requests: Vec<(CallId, String, String)>,
    last_gated: Option<CallId>,
}

impl Seen {
    fn args_of_last_gated(&self) -> String {
        let Some(call) = self.last_gated else {
            return String::new();
        };
        self.requests
            .iter()
            .find(|(id, _, _)| *id == call)
            .map(|(_, _, args)| args.clone())
            .unwrap_or_default()
    }

    fn tool_of(&self, call: CallId) -> String {
        self.requests
            .iter()
            .find(|(id, _, _)| *id == call)
            .map(|(_, tool, _)| tool.clone())
            .unwrap_or_default()
    }
}

fn automatic_kind(source: JudgementSource) -> Option<AttentionKind> {
    match source {
        JudgementSource::RuleTimeout => Some(AttentionKind::TimeoutAuto),
        JudgementSource::AlwaysApprove => Some(AttentionKind::Yolo),
        JudgementSource::You
        | JudgementSource::Rule
        | JudgementSource::ProgramCheck
        | JudgementSource::ModelJudgement
        | JudgementSource::Ungated
        | JudgementSource::ForwardedUnverified => None,
    }
}

pub(crate) fn collect_attention(events: &[TaskEvent]) -> Vec<ReportAttention> {
    let mut out: Vec<ReportAttention> = Vec::new();
    let mut seen = Seen::default();
    for event in events {
        let TaskEventKind::Known(kind) = &event.kind else {
            continue;
        };
        note(kind, &mut seen, &mut out);
    }
    out
}

fn raw_args(arguments: &ToolArgs) -> String {
    match arguments {
        ToolArgs::Json { value } => value.to_string(),
        ToolArgs::Broken { raw } => raw.clone(),
    }
}

fn note(kind: &KnownTaskEventKind, seen: &mut Seen, out: &mut Vec<ReportAttention>) {
    if let KnownTaskEventKind::ModelAnswer { tool_requests, .. } = kind {
        for request in tool_requests {
            seen.requests.push((
                request.call_id,
                request.tool.clone(),
                raw_args(&request.arguments),
            ));
        }
    } else if let KnownTaskEventKind::GateResult { call_id, gate } = kind {
        seen.last_gated = Some(*call_id);
        if let Some(attention) = automatic_kind(gate.source) {
            out.push(ReportAttention {
                kind: attention,
                tool: seen.tool_of(*call_id),
                args_summary: seen.args_of_last_gated(),
                error: String::new(),
            });
        }
    } else if let KnownTaskEventKind::ToolFailed {
        name,
        args_summary,
        error,
        ..
    } = kind
    {
        out.push(ReportAttention {
            kind: AttentionKind::Failed,
            tool: name.clone(),
            args_summary: args_summary.clone(),
            error: error.clone(),
        });
    } else if let KnownTaskEventKind::EvidenceAdded { record } = kind
        && record.kind == EvidenceKind::Rejected
    {
        out.push(ReportAttention {
            kind: AttentionKind::Rejected,
            tool: record.tool.clone(),
            args_summary: seen.args_of_last_gated(),
            error: rejected_code(&record.payload),
        });
    }
}

fn rejected_code(payload: &EvidencePayload) -> String {
    if let EvidencePayload::Rejected { reason } = payload {
        return reason.code.clone();
    }
    String::new()
}
