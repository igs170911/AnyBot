use crate::estimate::text_tokens;
use crate::numbers::K114_7_STATE_MAX_TOKENS;
use crate::state::{StateBuilder, StepStateKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryItem {
    pub text: String,
    pub source_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSummary {
    pub items: Vec<SummaryItem>,
}

impl StateSummary {
    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::new();
        for item in &self.items {
            out.push_str(&item.text);
            out.push('\n');
        }
        out
    }

    #[must_use]
    pub fn tokens(&self) -> u64 {
        self.items.iter().map(|i| text_tokens(&i.text)).sum()
    }
}

fn item(text: String, source_seq: u64) -> SummaryItem {
    SummaryItem { text, source_seq }
}

#[must_use]
pub fn render(b: &StateBuilder, compress: bool) -> StateSummary {
    let mut items = Vec::new();
    push_goal(b, &mut items);
    push_criteria(b, &mut items);
    push_steps(b, &mut items, compress);
    push_simple(b, &mut items);
    items.sort_by_key(|i| (i.source_seq, i.text.clone()));
    StateSummary { items }
}

#[must_use]
pub fn fit_summary(b: &StateBuilder) -> StateSummary {
    let full = render(b, false);
    if full.tokens() <= K114_7_STATE_MAX_TOKENS {
        return full;
    }
    render(b, true)
}

fn push_goal(b: &StateBuilder, items: &mut Vec<SummaryItem>) {
    if let Some((goal, seq)) = &b.goal {
        items.push(item(format!("goal: {goal} [seq {seq}]"), *seq));
    }
}

fn push_criteria(b: &StateBuilder, items: &mut Vec<SummaryItem>) {
    for (idx, c) in b.criteria.iter().enumerate() {
        let outcome = match c.outcome {
            Some((true, _)) => "passed",
            Some((false, _)) => "failed",
            None => "pending",
        };
        items.push(item(
            format!(
                "criterion {}: {text} {outcome} [seq {seq}]",
                idx.saturating_add(1),
                text = c.text,
                seq = c.declared_seq
            ),
            c.declared_seq,
        ));
    }
}

fn push_steps(b: &StateBuilder, items: &mut Vec<SummaryItem>, compress: bool) {
    let mut done = 0usize;
    let mut failed = 0usize;
    let mut last_seq = 0u64;
    for (id, s) in &b.steps {
        match s.state {
            StepStateKind::Done => {
                done = done.saturating_add(1);
                last_seq = last_seq.max(s.intent_seq);
                if !compress {
                    items.push(item(
                        format!("step {id} {} done [seq {seq}]", s.tool, seq = s.intent_seq),
                        s.intent_seq,
                    ));
                }
            }
            StepStateKind::Failed => {
                failed = failed.saturating_add(1);
                last_seq = last_seq.max(s.intent_seq);
                if !compress {
                    items.push(item(
                        format!(
                            "step {id} {} failed [seq {seq}]",
                            s.tool,
                            seq = s.intent_seq
                        ),
                        s.intent_seq,
                    ));
                }
            }
            StepStateKind::Open => {
                items.push(item(
                    format!("step {id} {} open [seq {seq}]", s.tool, seq = s.intent_seq),
                    s.intent_seq,
                ));
            }
        }
    }
    if !compress || (done == 0 && failed == 0) {
        return;
    }
    items.push(item(
        format!("completed: {done} failed: {failed}"),
        last_seq,
    ));
}

fn push_simple(b: &StateBuilder, items: &mut Vec<SummaryItem>) {
    for (seq, summary) in &b.evidence {
        items.push(item(format!("evidence: {summary} [seq {seq}]"), *seq));
    }
    for (seq, action) in b.open_cards() {
        items.push(item(format!("open question: {action} [seq {seq}]"), seq));
    }
    for (seq, answer) in &b.answers {
        items.push(item(format!("card answer: {answer} [seq {seq}]"), *seq));
    }
    for (id, seq) in &b.pending {
        items.push(item(format!("pending action {id} [seq {seq}]"), *seq));
    }
    for (id, seq) in &b.unknowns {
        items.push(item(format!("unknown outcome {id} [seq {seq}]"), *seq));
    }
    for (name, (used, seq)) in &b.budget {
        items.push(item(format!("budget {name:?}: {used} [seq {seq}]"), *seq));
    }
    for (seq, provider, model) in &b.swaps {
        items.push(item(
            format!("model swapped to {provider} {model} [seq {seq}]"),
            *seq,
        ));
    }
}
