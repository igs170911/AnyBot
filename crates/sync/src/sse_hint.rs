use anybot_protocol::{NewRecordsEvent, ProgressEvent, StreamProgress};

use crate::sse_parse::SseEvent;

pub const EVENT_PROGRESS: &str = "progress";
pub const EVENT_NEW_RECORDS: &str = "new-records";
pub const EVENT_CLOSING_PROGRESS: &str = "stream-progress";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamMark {
    pub stream: String,
    pub seq: i64,
    pub device_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hint {
    Progress { streams: Vec<StreamMark> },
    News(StreamMark),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HintFault {
    Unreadable { event: String, reason: String },
}

pub fn hint_of(event: &SseEvent) -> Result<Option<Hint>, HintFault> {
    let Some(name) = event.name.as_deref() else {
        return Ok(None);
    };
    match name {
        EVENT_PROGRESS | EVENT_CLOSING_PROGRESS => progress_of(name, &event.data).map(Some),
        EVENT_NEW_RECORDS => news_of(name, &event.data).map(Some),
        _unknown => Ok(None),
    }
}

fn progress_of(name: &str, data: &str) -> Result<Hint, HintFault> {
    let parsed: ProgressEvent = serde_json::from_str(data).map_err(|e| fault(name, &e))?;
    Ok(Hint::Progress {
        streams: parsed.streams.into_iter().map(mark_of_progress).collect(),
    })
}

fn news_of(name: &str, data: &str) -> Result<Hint, HintFault> {
    let parsed: NewRecordsEvent = serde_json::from_str(data).map_err(|e| fault(name, &e))?;
    Ok(Hint::News(StreamMark {
        stream: parsed.stream,
        seq: parsed.seq,
        device_id: parsed.device_id,
    }))
}

fn mark_of_progress(row: StreamProgress) -> StreamMark {
    StreamMark {
        stream: row.stream,
        seq: row.seq,
        device_id: row.device_id,
    }
}

fn fault(event: &str, error: &serde_json::Error) -> HintFault {
    HintFault::Unreadable {
        event: event.to_owned(),
        reason: error.to_string(),
    }
}

#[cfg(test)]
#[path = "sse_hint_tests.rs"]
mod tests;
