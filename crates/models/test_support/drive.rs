use std::time::Duration;

use test_harness::FakeClock;

pub(crate) async fn drive_until(clock: &FakeClock, done: impl Fn() -> bool) {
    loop {
        test_harness::sleep_real(Duration::from_millis(20)).await;
        let _woke = clock.advance(Duration::from_secs(2));
        if done() {
            break;
        }
    }
}
