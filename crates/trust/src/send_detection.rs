use anybot_protocol::{EvidencePayload, EvidenceRecord};

use crate::report_extract::extract_emails;

const SEND_NAME_MARKERS: [&str; 5] = ["send", "deliver", "mail", "post", "upload"];
const SEND_CLASSES: [&str; 4] = ["send", "outbound", "share", "publish"];
const RECIPIENT_CLASS_PREFIX: &str = "recipient:";

#[must_use]
pub fn is_send_tool(tool: &str) -> bool {
    let lower = tool.to_lowercase();
    SEND_NAME_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
        || lower.starts_with("connector")
}

pub(crate) fn is_send_record(rec: &EvidenceRecord) -> bool {
    is_send_tool(&rec.tool)
        || rec
            .classes
            .iter()
            .any(|class| SEND_CLASSES.contains(&class.as_str()))
}

pub(crate) fn records_recipients(records: &[&EvidenceRecord]) -> Vec<String> {
    let mut out = Vec::new();
    for rec in records {
        for class in &rec.classes {
            if let Some(val) = class.strip_prefix(RECIPIENT_CLASS_PREFIX) {
                let lower = val.trim().to_lowercase();
                if !lower.is_empty() && !out.contains(&lower) {
                    out.push(lower);
                }
            }
        }
        for email in payload_emails(&rec.payload) {
            if !out.contains(&email) {
                out.push(email);
            }
        }
    }
    out
}

fn payload_emails(payload: &EvidencePayload) -> Vec<String> {
    match payload {
        EvidencePayload::Executed { command, .. } => extract_emails(command),
        EvidencePayload::Changed { path, .. } | EvidencePayload::Seen { path, .. } => {
            extract_emails(path)
        }
        EvidencePayload::Relayed { text_head, .. } => extract_emails(text_head),
        EvidencePayload::Approved { .. }
        | EvidencePayload::Rejected { .. }
        | EvidencePayload::NotExecuted { .. } => Vec::new(),
    }
}
