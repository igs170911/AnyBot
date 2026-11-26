use std::io::Read;
use std::io::Write as _;
use std::process::ExitStatus;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub enum PollState {
    Exited(ExitStatus),
}

pub(crate) fn spawn_reader<R: Read + Send + 'static>(
    stream: Option<R>,
    name: &str,
    label: String,
) -> mpsc::Receiver<Vec<u8>> {
    spawn_bytes(
        stream.map(|s| Box::new(s) as Box<dyn Read + Send>),
        name,
        label,
    )
}

pub(crate) fn spawn_bytes(
    stream: Option<Box<dyn Read + Send>>,
    name: &str,
    label: String,
) -> mpsc::Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    let stream_name = name.to_owned();
    thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut stream) = stream {
            let _result = stream.read_to_end(&mut buf);
        }
        if tx.send(buf).is_err() {
            let mut stderr = std::io::stderr();
            let _result = writeln!(
                stderr,
                "harness: {label} {stream_name} reader dropped its payload"
            );
        }
    });
    rx
}

pub(crate) fn collect_stream(
    rx: &mpsc::Receiver<Vec<u8>>,
    timeout: Duration,
) -> Result<String, String> {
    let raw = rx
        .recv_timeout(timeout)
        .map_err(|e| format!("stream reader ended: {e}"))?;
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

pub(crate) fn poll_exit(
    child: &mut std::process::Child,
    pid: u32,
    grace: Duration,
    deadline: Instant,
) -> Result<PollState, String> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(PollState::Exited(status)),
            Ok(None) => {}
            Err(err) => return Err(format!("wait PID {pid} failed: {err}")),
        }
        if Instant::now() >= deadline {
            kill_child(child, pid, &format!("PID {pid}"));
            return Err(format!(
                "timed out after {grace:?}+ poll waiting for PID {pid} to exit"
            ));
        }
        thread::sleep(Duration::from_millis(50));
    }
}

pub fn exit_status_code(status: &ExitStatus) -> Result<i32, String> {
    if status.success() {
        return Ok(0);
    }
    if let Some(code) = status.code() {
        return Ok(code);
    }
    signaled_code(*status)
}

#[cfg(unix)]
fn signaled_code(status: ExitStatus) -> Result<i32, String> {
    use std::os::unix::process::ExitStatusExt as _;
    match status.signal() {
        Some(signal) => match 128_i32.checked_add(signal) {
            Some(total) => Ok(total),
            None => Err("signal exit code overflow".to_owned()),
        },
        None => Err("program exited without a computable exit code".to_owned()),
    }
}

#[cfg(not(unix))]
fn signaled_code(_status: ExitStatus) -> Result<i32, String> {
    Err("program exited without a computable exit code".to_owned())
}

pub(crate) fn finish_poll(poll: PollState) -> ExitStatus {
    match poll {
        PollState::Exited(status) => status,
    }
}

pub(crate) fn kill_child(child: &mut std::process::Child, pid: u32, label: &str) {
    #[cfg(windows)]
    crate::reap::stop_spawned(pid);
    if let Err(err) = child.kill() {
        let mut stderr = std::io::stderr();
        let _result = writeln!(stderr, "harness: kill PID {pid} ({label}) failed: {err}");
    }
    let _result = child.wait();
}

pub(crate) fn terminate_child(pid: u32, label: &str) -> Result<(), String> {
    crate::reap::signal_recorded_pid(pid).map_err(|e| format!("harness: TERM {label}: {e}"))
}
