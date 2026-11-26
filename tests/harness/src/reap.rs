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
    exe_still_matches(live_exe(record.pid).as_deref(), expected)
}

fn exe_still_matches(live: Option<&Path>, expected: &Path) -> bool {
    match live {
        Some(found) => resolved(found) == resolved(expected),
        None => false,
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

fn process_alive(pid: u32) -> bool {
    os::process_alive(pid)
}

fn live_exe(pid: u32) -> Option<PathBuf> {
    #[cfg(test)]
    if lookup_fails() {
        return None;
    }
    os::live_exe(pid)
}

#[cfg(test)]
fn lookup_fails() -> bool {
    LOOKUP_FAILS.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
static LOOKUP_FAILS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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

fn signal_stop(pid: u32) -> Result<(), String> {
    os::signal_stop(pid)
}

#[cfg(windows)]
pub(crate) fn stop_spawned(pid: u32) {
    os::stop_spawned(pid);
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
#[path = "reap_record.rs"]
mod record_tests;

#[cfg(test)]
#[path = "reap_unread.rs"]
mod unread_tests;

#[path = "reap_os.rs"]
mod os;

pub fn remove_tool_pid(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("remove {} failed: {err}", path.display())),
    }
}
