use anybot_foundation::{CallId, EventId, SystemClock, TaskId, TimestampMs};
use anybot_messages::Lang;
use anybot_protocol::{
    FinishReason, KnownTaskEventKind, TaskEvent, TaskEventKind, ToolArgs, ToolRequest,
};

use crate::context_input::steps_of;
use crate::error::TurnError;
use crate::record::{Stored, StoredBody};

fn model_answer(call_id: CallId, task: TaskId) -> Stored {
    let body = StoredBody::Task {
        task,
        turn: Some(1),
        event: TaskEvent {
            event_id: EventId::generate(&SystemClock),
            seq: 0,
            at_ms: TimestampMs::from_millis(1),
            kind: TaskEventKind::Known(KnownTaskEventKind::ModelAnswer {
                text: "will read".to_owned(),
                thinking: None,
                tool_requests: vec![ToolRequest {
                    call_id,
                    provider_call_id: "c".to_owned(),
                    tool: "read_file".to_owned(),
                    arguments: ToolArgs::Json {
                        value: serde_json::json!({ "path": "a.txt" }),
                    },
                }],
                finish: FinishReason::NeedsTool,
                native: None,
            }),
        },
    };
    Stored {
        stream_seq: 1,
        body,
    }
}

#[test]
fn step_without_result_is_an_error() {
    let call_id = CallId::generate(&SystemClock);
    let task = TaskId::generate(&SystemClock);
    let rows = vec![model_answer(call_id, task)];
    let result = steps_of(&rows, &task, Lang::En);
    match result {
        Err(TurnError::UnpairedToolRequest { call_id: got }) => assert_eq!(got, call_id),
        other => panic!("unexpected: {other:?}"),
    }
}
