use test_harness::{Program, http_get, run_program, test_dir, write_file_with_mode};

#[test]
fn harness_run_program_captures_stdout_and_exit_code() {
    let dir = test_dir("harness-run").unwrap();
    let program = Program::new("/bin/echo", vec!["hi-there".to_owned()]);
    let outcome = run_program(&program, &dir).unwrap();
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.stdout.trim(), "hi-there");
    assert_eq!(outcome.stderr, "");
}

#[test]
fn harness_run_program_captures_nonzero_exit_and_stderr() {
    let dir = test_dir("harness-fail").unwrap();
    let program = Program::new(
        "/bin/sh",
        vec!["-c".to_owned(), "echo oops >&2; exit 3".to_owned()],
    );
    let outcome = run_program(&program, &dir).unwrap();
    assert_eq!(outcome.code, 3);
    assert!(outcome.stderr.contains("oops"));
}

#[test]
fn harness_running_pid_matches_child_and_kill_stops_it() {
    let dir = test_dir("harness-kill").unwrap();
    let program = Program::new("/bin/sleep", vec!["60".to_owned()]);
    let mut running = test_harness::Running::start(&program, &dir).unwrap();
    assert!(running.pid() > 0);
    let outcome = running.kill().unwrap();
    assert_ne!(outcome.code, 0);
}

#[test]
fn harness_test_dir_is_under_target_and_is_fresh() {
    let dir = test_dir("harness-fresh").unwrap();
    assert!(dir.starts_with(test_harness::scratch::repo_root().join("target")));
    write_file_with_mode(&dir.join("a.txt"), "x", 0o600).unwrap();
    let second = test_dir("harness-fresh").unwrap();
    assert_eq!(dir, second);
    assert!(!second.join("a.txt").exists());
}

#[cfg(unix)]
#[test]
fn harness_write_file_with_mode_sets_exact_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = test_dir("harness-mode").unwrap();
    let path = dir.join("boot.conf");
    write_file_with_mode(&path, "postgresql://u:p@h/db\n", 0o600).unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn harness_http_get_reports_connection_failure_without_crashing() {
    let result = http_get(1, "/v1/health");
    assert!(result.is_err());
}
