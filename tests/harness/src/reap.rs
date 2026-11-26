use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::scratch::repo_root;

const STOP_WAIT: Duration = Duration::from_secs(8);
const MARKER: &str = "tool.pid";

pub fn reap_leftover_tools() -> Result<(), String> {
    reap_except(None)
}

pub fn signal_recorded_pid(pid: u32) -> Result<(), String> {
    signal_stop(pid)
}

pub fn reap_except(keep: Option<&Path>) -> Result<(), String> {
    let root = repo_root().join("target").join("tests");
    if !root.is_dir() {
        return Ok(());
    }
    walk_markers(&root, keep)
}

fn walk_markers(dir: &Path, keep: Option<&Path>) -> Result<(), String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => return Err(format!("read {} failed: {err}", dir.display())),
    };
    for entry in entries {
        let path = entry.map_err(|e| format!("reap entry failed: {e}"))?.path();
        if path.is_dir() {
            if keep.is_some_and(|held| same_dir(&path, held)) {
                continue;
            }
            walk_markers(&path, keep)?;
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some(MARKER) {
            stop_recorded(&path)?;
        }
    }
    Ok(())
}

fn stop_recorded(path: &Path) -> Result<(), String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => return Err(format!("read {} failed: {err}", path.display())),
    };
    let Some(record) = parse_record(text.trim()) else {
        return Ok(());
    };
    if record.pid == std::process::id() || !process_alive(record.pid) {
        return Ok(());
    }
    if !marker_in_tests(path) || !workspace_record(&record) {
        return Ok(());
    }
    signal_stop(record.pid)
}

fn marker_in_tests(marker: &Path) -> bool {
    let root = repo_root().canonicalize().unwrap_or_else(|_| repo_root());
    let tests = root.join("target").join("tests");
    let marker_dir = marker.parent().unwrap_or(marker);
    path_under(&resolved(marker_dir), &tests)
}

fn workspace_record(record: &Record) -> bool {
    let root = repo_root().canonicalize().unwrap_or_else(|_| repo_root());
    let Some(marked) = record.workspace.as_deref() else {
        return false;
    };
    if resolved(marked) != root {
        return false;
    }
    let Some(expected) = record.exe.as_deref() else {
        return false;
    };
    if expected.as_os_str().is_empty() {
        return false;
    }
    match live_exe(record.pid) {
        Some(live) => resolved(&live) == resolved(expected),
        None => true,
    }
}

struct Record {
    pid: u32,
    exe: Option<PathBuf>,
    workspace: Option<PathBuf>,
}

fn parse_record(text: &str) -> Option<Record> {
    let mut lines = text.lines();
    let pid = lines.next()?.trim().parse::<u32>().ok()?;
    if pid == 0 {
        return None;
    }
    let exe = lines
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from);
    let workspace = lines
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from);
    Some(Record {
        pid,
        exe,
        workspace,
    })
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|code| code.success())
}

#[cfg(not(unix))]
fn process_alive(_pid: u32) -> bool {
    false
}

#[cfg(unix)]
fn live_exe(pid: u32) -> Option<PathBuf> {
    let link = PathBuf::from(format!("/proc/{pid}/exe"));
    if let Ok(exe) = std::fs::read_link(&link) {
        return Some(exe);
    }
    lsof_field(pid, "txt")
}

#[cfg(not(unix))]
fn live_exe(_pid: u32) -> Option<PathBuf> {
    None
}

#[cfg(unix)]
fn lsof_field(pid: u32, descriptor: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("/usr/sbin/lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", descriptor, "-Fn"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().find_map(|line| {
        let path = line.strip_prefix('n')?;
        Some(PathBuf::from(path))
    })
}

fn resolved(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn path_under(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

fn same_dir(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(one), Ok(two)) => one == two,
        _ => left == right,
    }
}

#[cfg(unix)]
fn signal_stop(pid: u32) -> Result<(), String> {
    send_term(pid)?;
    let _stopped = wait_dead(pid, STOP_WAIT);
    Ok(())
}

#[cfg(not(unix))]
fn signal_stop(pid: u32) -> Result<(), String> {
    Err(format!("stop PID {pid} is not available on this platform"))
}

#[cfg(unix)]
fn send_term(pid: u32) -> Result<(), String> {
    let helper = signal_helper()?;
    let output = std::process::Command::new(&helper)
        .arg(pid.to_string())
        .output()
        .map_err(|e| format!("term PID {pid} failed: {e}"))?;
    let status = output.status;
    let _note = String::from_utf8_lossy(&output.stderr);
    if status.success() || !process_alive(pid) {
        return Ok(());
    }
    Err(format!("term PID {pid} was rejected"))
}

#[cfg(unix)]
fn signal_helper() -> Result<PathBuf, String> {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/signal_term.c");
    let out_dir = repo_root()
        .join("target")
        .join("tests")
        .join("signal-helper");
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("create signal helper dir failed: {e}"))?;
    let binary = out_dir.join("signal_term");
    let built = std::process::Command::new("cc")
        .args([
            "-O2",
            "-o",
            &binary.to_string_lossy(),
            &source.to_string_lossy(),
        ])
        .status()
        .map_err(|e| format!("build signal helper failed: {e}"))?;
    if !built.success() {
        return Err("build signal helper was rejected".to_owned());
    }
    Ok(binary)
}

fn wait_dead(pid: u32, budget: Duration) -> Result<(), String> {
    let Some(deadline) = Instant::now().checked_add(budget) else {
        return Err("reap deadline overflow".to_owned());
    };
    while Instant::now() < deadline {
        if !process_alive(pid) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if process_alive(pid) {
        return Err(format!("PID {pid} still alive after {budget:?}"));
    }
    Ok(())
}

pub fn write_tool_pid(dir: &Path) -> Result<PathBuf, String> {
    let path = dir.join(MARKER);
    let exe = std::env::current_exe().unwrap_or_default();
    let root = repo_root();
    let body = format!(
        "{}\n{}\n{}\n",
        std::process::id(),
        exe.display(),
        root.display()
    );
    std::fs::write(&path, body).map_err(|e| format!("write {} failed: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reap_parse_record_rejects_blank_and_zero() {
        assert!(parse_record("").is_none());
        assert!(parse_record("0").is_none());
        let parsed =
            parse_record("42\n/work/m6a/target/debug/testenv\n/work/m6a\n").expect("record");
        assert_eq!(parsed.pid, 42);
        assert_eq!(
            parsed.exe.as_deref(),
            Some(Path::new("/work/m6a/target/debug/testenv"))
        );
        assert_eq!(parsed.workspace.as_deref(), Some(Path::new("/work/m6a")));
    }

    #[test]
    fn reap_path_under_requires_the_workspace_prefix() {
        let root = Path::new("/work/m6a");
        assert!(path_under(
            Path::new("/work/m6a/target/debug/testenv"),
            root
        ));
        assert!(!path_under(
            Path::new("/work/m7a/target/debug/testenv"),
            root
        ));
    }
}
