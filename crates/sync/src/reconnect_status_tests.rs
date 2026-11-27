use std::sync::Arc;

use test_harness::FakeClock;

use super::*;

fn board(start: u64) -> (Arc<FakeClock>, StatusBoard) {
    let clock = FakeClock::new(start);
    let shared: Arc<dyn Clock> = Arc::<FakeClock>::clone(&clock);
    (clock, StatusBoard::new(shared))
}

#[test]
fn a_new_board_says_connecting_with_no_reason() {
    let (_clock, board) = board(10_000);
    let status = board.current();
    assert_eq!(status.phase, LinkPhase::Connecting);
    assert_eq!(status.reason, None);
    assert_eq!(status.retry_at, None);
    assert_eq!(status.since, TimestampMs::from_millis(10_000));
}

#[test]
fn connected_clears_the_reason_and_the_failure_count() {
    let (_clock, board) = board(10_000);
    board.ended(&SseEnd::Idle, Some(TimestampMs::from_millis(12_000)), 3);
    board.connected();
    let status = board.current();
    assert_eq!(status.phase, LinkPhase::Connected);
    assert_eq!(status.reason, None);
    assert_eq!(status.failures_in_row, 0);
    assert_eq!(status.retry_at, None);
}

#[test]
fn an_end_becomes_retrying_with_its_reason_and_the_retry_time() {
    let (clock, board) = board(10_000);
    let _woken = test_harness::advance_clock(&clock, std::time::Duration::from_secs(7));
    board.ended(
        &SseEnd::ConnectTimeout,
        Some(TimestampMs::from_millis(18_500)),
        2,
    );
    let status = board.current();
    assert_eq!(status.phase, LinkPhase::Retrying);
    assert_eq!(status.reason, Some(LinkReason::ConnectTimeout));
    assert_eq!(status.retry_at, Some(TimestampMs::from_millis(18_500)));
    assert_eq!(status.failures_in_row, 2);
    assert_eq!(status.since, TimestampMs::from_millis(17_000));
}

#[test]
fn rejected_credentials_and_old_versions_are_their_own_phases() {
    let (_clock, board) = board(1);
    board.ended(&SseEnd::CredentialRejected, None, 0);
    assert_eq!(board.current().phase, LinkPhase::CredentialRejected);
    board.ended(&SseEnd::VersionTooOld, None, 0);
    assert_eq!(board.current().phase, LinkPhase::VersionTooOld);
}

#[test]
fn a_subscriber_sees_every_phase_change() {
    let (_clock, board) = board(1);
    let mut watcher = board.subscribe();
    board.connected();
    assert!(watcher.has_changed().unwrap());
    assert_eq!(watcher.borrow_and_update().phase, LinkPhase::Connected);
    board.connecting();
    assert_eq!(watcher.borrow_and_update().phase, LinkPhase::Connecting);
}

#[test]
fn every_end_maps_to_a_reason_except_the_ones_that_are_not_failures_of_the_link() {
    assert_eq!(
        reason_of(&SseEnd::Dropped("x".to_owned())),
        Some(LinkReason::Dropped)
    );
    assert_eq!(
        reason_of(&SseEnd::Rejected { status: 500 }),
        Some(LinkReason::Rejected)
    );
    assert_eq!(reason_of(&SseEnd::ReceiverGone), None);
    assert_eq!(reason_of(&SseEnd::VersionTooOld), None);
}
