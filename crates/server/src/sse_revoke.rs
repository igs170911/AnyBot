use anybot_bus::Notification;

use crate::app::AppState;

pub fn revoke_sse(state: &AppState, account: &str) {
    let kind = anybot_bus::revoke_note(account);
    state.bus.publish(Notification {
        kind,
        at: state.clock.now_ms(),
        seq: None,
    });
}
