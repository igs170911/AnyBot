use anybot_protocol::{ActionOutcome, ReasonPayload};

use crate::claims_links::has_dead_bot_link;
use crate::claims_phrases::{
    has_done_claim, has_file_in_chat_claim, has_send_claim, is_non_claim_exclusion,
};
use crate::claims_text::{extract_claim_counts, strip_code_and_quotes};
use crate::message_codes::{
    CODE_CLAIM_CHAT_COUNT, CODE_CLAIM_CHAT_DEAD_LINK, CODE_CLAIM_CHAT_NO_FILE,
    CODE_CLAIM_CHAT_STEP_FAILED,
};
use crate::send_detection::is_send_tool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepFact {
    pub tool: String,
    pub read_only: bool,
    pub outcome: ActionOutcome,
    pub exit_code: Option<i32>,
    pub output_empty: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TurnFacts {
    pub steps: Vec<StepFact>,
    pub files_placed: Vec<String>,
    pub sends: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimVerdict {
    pub covers_claim: bool,
    pub verified: bool,
    pub reasons: Vec<ReasonPayload>,
}

#[must_use]
pub fn check_reply(reply: &str, facts: &TurnFacts) -> ClaimVerdict {
    let mut reasons: Vec<ReasonPayload> = Vec::new();
    let mut covers_claim = false;
    if has_dead_bot_link(reply, &facts.files_placed) {
        covers_claim = true;
        push_reason(&mut reasons, CODE_CLAIM_CHAT_DEAD_LINK, Vec::new());
    }
    let clean = strip_code_and_quotes(reply);
    for sentence in split_sentences(&clean) {
        if is_non_claim_exclusion(&sentence) {
            continue;
        }
        let matched = check_sentence(&sentence, facts, &mut reasons);
        if matched {
            covers_claim = true;
        }
    }
    ClaimVerdict {
        covers_claim,
        verified: reasons.is_empty(),
        reasons,
    }
}

fn check_sentence(sentence: &str, facts: &TurnFacts, reasons: &mut Vec<ReasonPayload>) -> bool {
    let done = has_done_claim(sentence);
    let send = has_send_claim(sentence);
    if !done && !send {
        return false;
    }
    if send {
        check_send_sentence(sentence, facts, reasons);
    }
    if done && last_non_readonly_failed(facts) {
        push_reason(reasons, CODE_CLAIM_CHAT_STEP_FAILED, Vec::new());
    }
    true
}

fn check_send_sentence(sentence: &str, facts: &TurnFacts, reasons: &mut Vec<ReasonPayload>) {
    if last_send_failed(facts) {
        push_reason(reasons, CODE_CLAIM_CHAT_STEP_FAILED, Vec::new());
    } else if has_file_in_chat_claim(sentence) && facts.files_placed.is_empty() {
        push_reason(reasons, CODE_CLAIM_CHAT_NO_FILE, Vec::new());
    }
    let (said_sends, said_files) = extract_claim_counts(sentence);
    if let Some(said) = said_sends
        && said != facts.sends
    {
        push_count_reason(reasons, said, facts.sends);
    }
    if let Some(said) = said_files {
        let found = u32::try_from(facts.files_placed.len()).unwrap_or(u32::MAX);
        if said != found {
            push_count_reason(reasons, said, found);
        }
    }
}

fn push_count_reason(reasons: &mut Vec<ReasonPayload>, said: u32, found: u32) {
    push_reason(
        reasons,
        CODE_CLAIM_CHAT_COUNT,
        vec![
            ("said".to_owned(), said.to_string()),
            ("found".to_owned(), found.to_string()),
        ],
    );
}

fn push_reason(reasons: &mut Vec<ReasonPayload>, code: &str, params: Vec<(String, String)>) {
    let candidate = ReasonPayload {
        code: code.to_owned(),
        params,
    };
    if !reasons.contains(&candidate) {
        reasons.push(candidate);
    }
}

fn last_non_readonly_failed(facts: &TurnFacts) -> bool {
    facts
        .steps
        .iter()
        .rfind(|step| !step.read_only)
        .is_some_and(step_failed_or_rejected)
}

fn last_send_failed(facts: &TurnFacts) -> bool {
    facts
        .steps
        .iter()
        .rfind(|step| is_send_tool(&step.tool))
        .is_some_and(step_failed_or_rejected)
}

fn step_failed_or_rejected(step: &StepFact) -> bool {
    if step.outcome == ActionOutcome::Rejected {
        return true;
    }
    if step.outcome == ActionOutcome::Failed {
        return !(step.exit_code == Some(1) && step.output_empty);
    }
    false
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut iter = text.chars().peekable();
    while let Some(ch) = iter.next() {
        if matches!(ch, '\n' | '\u{3002}' | '\u{ff01}' | '\u{ff1f}' | '!' | '?') {
            push_trimmed(&mut out, &mut cur);
            continue;
        }
        if ch == '.' && iter.peek().is_none_or(|next| next.is_whitespace()) {
            push_trimmed(&mut out, &mut cur);
            continue;
        }
        cur.push(ch);
    }
    push_trimmed(&mut out, &mut cur);
    out
}

fn push_trimmed(out: &mut Vec<String>, cur: &mut String) {
    let trimmed = cur.trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_owned());
    }
    cur.clear();
}
