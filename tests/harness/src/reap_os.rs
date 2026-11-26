use std::path::PathBuf;

#[cfg(unix)]
pub(super) fn process_alive(pid: u32) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|code| code.success())
}

#[cfg(windows)]
pub(super) fn process_alive(pid: u32) -> bool {
    tasklist_pid(pid)
}

#[cfg(unix)]
pub(super) fn live_exe(pid: u32) -> Option<PathBuf> {
    let link = PathBuf::from(format!("/proc/{pid}/exe"));
    if let Ok(exe) = std::fs::read_link(&link) {
        return Some(exe);
    }
    lsof_field(pid, "txt")
}

#[cfg(windows)]
pub(super) fn live_exe(pid: u32) -> Option<PathBuf> {
    image_from_powershell(pid)
}

#[cfg(unix)]
pub(super) fn signal_stop(pid: u32) -> Result<(), String> {
    send_term(pid)?;
    let _stopped = super::wait_dead(pid, super::STOP_WAIT);
    Ok(())
}

#[cfg(windows)]
pub(super) fn signal_stop(pid: u32) -> Result<(), String> {
    stop_tree(pid)?;
    super::wait_dead(pid, super::STOP_WAIT)
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

#[cfg(unix)]
fn send_term(pid: u32) -> Result<(), String> {
    let helper = signal_helper()?;
    let output = std::process::Command::new(&helper)
        .arg(pid.to_string())
        .output()
        .map_err(|e| format!("term PID {pid} failed: {e}"))?;
    if output.status.success() || !process_alive(pid) {
        return Ok(());
    }
    Err(format!("term PID {pid} was rejected"))
}

#[cfg(unix)]
fn signal_helper() -> Result<PathBuf, String> {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/signal_term.c");
    let out_dir = super::repo_root()
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

#[cfg(windows)]
fn tasklist_pid(pid: u32) -> bool {
    let output = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .ok()?;
    if !output.status.success() {
        return false;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().any(|line| csv_pid_matches(line, pid))
}

#[cfg(windows)]
fn csv_pid_matches(line: &str, pid: u32) -> bool {
    let mut fields = line.split(',');
    let _image = fields.next();
    let Some(raw) = fields.next() else {
        return false;
    };
    raw.trim().trim_matches('"').parse::<u32>().ok() == Some(pid)
}

#[cfg(windows)]
fn image_from_powershell(pid: u32) -> Option<PathBuf> {
    let script =
        format!("(Get-CimInstance Win32_Process -Filter \"ProcessId={pid}\").ExecutablePath");
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let path = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    Some(PathBuf::from(path))
}

#[cfg(windows)]
fn stop_tree(pid: u32) -> Result<(), String> {
    let output = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output()
        .map_err(|e| format!("term PID {pid} failed: {e}"))?;
    if output.status.success() || !process_alive(pid) {
        return Ok(());
    }
    Err(format!("term PID {pid} was rejected"))
}

#[cfg(windows)]
pub(super) fn stop_spawned(pid: u32) {
    let output = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output();
    let _ran = output.is_ok();
}
