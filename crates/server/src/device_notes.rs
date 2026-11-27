use anybot_bus::Notification;

use crate::app::AppState;

pub const DEVICE_REVOKE_PREFIX: &str = "device-revoked:";
pub const MAIL_PREFIX: &str = "mail:";

#[must_use]
pub fn device_revoke_kind(device_id: &str) -> String {
    format!("{DEVICE_REVOKE_PREFIX}{device_id}")
}

#[must_use]
pub fn device_of_revoke(kind: &str) -> Option<&str> {
    kind.strip_prefix(DEVICE_REVOKE_PREFIX)
}

#[must_use]
pub fn mail_kind(device_id: &str) -> String {
    format!("{MAIL_PREFIX}{device_id}")
}

#[must_use]
pub fn device_of_mail(kind: &str) -> Option<&str> {
    kind.strip_prefix(MAIL_PREFIX)
}

pub fn revoke_device(state: &AppState, device_id: &str) {
    state.bus.publish(Notification {
        kind: device_revoke_kind(device_id),
        at: state.clock.now_ms(),
        seq: None,
    });
}

pub fn notify_mail(state: &AppState, device_id: &str) {
    state.bus.publish(Notification {
        kind: mail_kind(device_id),
        at: state.clock.now_ms(),
        seq: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_round_trip_and_do_not_mix() {
        assert_eq!(device_of_revoke(&device_revoke_kind("d1")), Some("d1"));
        assert_eq!(device_of_mail(&mail_kind("d1")), Some("d1"));
        assert_eq!(device_of_mail(&device_revoke_kind("d1")), None);
        assert_eq!(device_of_revoke(&mail_kind("d1")), None);
    }
}
