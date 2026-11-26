use std::time::Duration;

use super::LOOKUP_FAILS;

struct ClosedLookup;

impl ClosedLookup {
    fn arm() -> Self {
        LOOKUP_FAILS.store(true, std::sync::atomic::Ordering::Relaxed);
        ClosedLookup
    }
}

impl Drop for ClosedLookup {
    fn drop(&mut self) {
        LOOKUP_FAILS.store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

#[test]
fn unread_executable_does_not_signal_an_unrelated_pid() {
    let dir = crate::scratch::test_dir("reap-unread").unwrap();
    let program = crate::launch::Program::new("/bin/sleep", vec!["30".to_owned()])
        .with_timeout(Duration::from_secs(8));
    let mut running = crate::running::Running::start(&program, &dir).unwrap();
    let pid = running.pid();
    assert!(
        running.child().try_wait().unwrap().is_none(),
        "sleep PID {pid} exited before the scan"
    );
    let hold_dir = crate::scratch::repo_root()
        .join("target")
        .join("tests")
        .join("reap-unrelated");
    std::fs::create_dir_all(&hold_dir).unwrap();
    let body = format!(
        "{pid}\n/missing/unrelated-tool\n{}\n",
        crate::scratch::repo_root().display()
    );
    std::fs::write(hold_dir.join("tool.pid"), body).unwrap();
    let _closed = ClosedLookup::arm();
    let reaped = crate::reap_except(Some(&dir));
    assert!(reaped.is_ok(), "{reaped:?}");
    let exited = running.child().try_wait().unwrap();
    assert!(
        exited.is_none(),
        "an unreadable executable path still signaled PID {pid}: {exited:?}"
    );
    running.kill().unwrap();
    let _removed = std::fs::remove_dir_all(hold_dir);
}
