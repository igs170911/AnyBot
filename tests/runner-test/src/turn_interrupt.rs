use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::TaskId;
use anybot_protocol::KnownTaskEventKind;
use anybot_runner::StoredBody;
use test_harness::{model_accept_then_delay, model_replies_text, model_tool_calls, timeout_real};

use crate::turn::{Device, TurnWorld};
use crate::turn_helpers::drive;
use crate::turn_tool_helpers::edit_call;

#[must_use]
pub fn first_task_of(world: &TurnWorld) -> Option<TaskId> {
    for row in world.record.snapshot() {
        let StoredBody::Task { task, event, .. } = &row.body else {
            continue;
        };
        if event.seq == 1 {
            return Some(*task);
        }
    }
    None
}

async fn wait_for_request(world: &TurnWorld, count: usize) -> Result<(), String> {
    let arrived = world.model.wait_recorded(count);
    timeout_real(arrived, Duration::from_secs(10))
        .await
        .map_err(|e| format!("the second model request never arrived: {e}"))?
}

async fn send_shared(
    world: Arc<TurnWorld>,
    text: String,
) -> Result<anybot_runner::TurnOutcome, anybot_runner::TurnError> {
    world.send(Device::Phone, &text).await
}

pub async fn edit_then_interrupt(
    world: &Arc<TurnWorld>,
    edit_args: &str,
    interrupt: KnownTaskEventKind,
) -> Result<TaskId, String> {
    world.script(vec![
        model_tool_calls(vec![edit_call(edit_args, "c1")], "tool_calls", false),
        model_accept_then_delay(5, model_replies_text("done", false)),
    ]);
    world.model.header_clock(Arc::clone(&world.clock));
    let handle = tokio::spawn(send_shared(Arc::clone(world), "work".to_owned()));
    wait_for_request(world, 2).await?;
    let task = first_task_of(world).ok_or("no created task")?;
    world
        .runner()
        .append_task_event(&task, interrupt)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let joined = async move { handle.await.map_err(|e| format!("{e:?}")) };
    let outcome = drive(&world.clock, joined).await??;
    outcome.map_err(|e| format!("{e:?}"))?;
    Ok(task)
}
