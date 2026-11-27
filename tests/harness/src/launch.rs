use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use crate::fake_clock::FakeClock;
use crate::process::{PollState, collect_stream, finish_poll, kill_child, spawn_reader};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct Program {
    pub path: PathBuf,
    pub args: Vec<String>,
    pub timeout: Duration,
    pub env: Vec<(String, String)>,
}

impl Program {
    #[must_use]
    pub fn new(path: &str, args: Vec<String>) -> Self {
        Program {
            path: PathBuf::from(path),
            args,
            timeout: DEFAULT_TIMEOUT,
            env: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    #[must_use]
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_owned(), value.to_owned()));
        self
    }

    fn build_command(&self, dir: &Path) -> Command {
        let mut cmd = Command::new(&self.path);
        cmd.args(&self.args);
        cmd.current_dir(dir);
        for (key, value) in &self.env {
            cmd.env(key, value);
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        isolate_child(&mut cmd);
        cmd
    }

    fn build_command_piped_stdin(&self, dir: &Path) -> Command {
        let mut cmd = self.build_command(dir);
        cmd.stdin(Stdio::piped());
        cmd
    }

    #[must_use]
    pub fn label(&self) -> String {
        format!("{} {:?}", self.path.display(), self.args)
    }
}

#[derive(Debug)]
pub struct Outcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_program(program: &Program, dir: &Path) -> Result<Outcome, String> {
    run_program_with(program, dir, |_| Ok(()))
}

pub fn run_program_stdin(
    program: &Program,
    dir: &Path,
    stdin_text: &str,
) -> Result<Outcome, String> {
    let mut sent = false;
    let payload = stdin_text.to_owned();
    let mut child = program
        .build_command_piped_stdin(dir)
        .spawn()
        .map_err(|e| format!("start {} failed: {e}", program.label()))?;
    let pid = child.id();
    let out_rx = spawn_reader(child.stdout.take(), "stdout", program.label());
    let err_rx = spawn_reader(child.stderr.take(), "stderr", program.label());
    let Some(deadline) = Instant::now().checked_add(program.timeout) else {
        return Err("launch deadline overflow".to_owned());
    };
    let mut observe = |kid: &mut std::process::Child| -> Result<(), String> {
        if sent {
            return Ok(());
        }
        sent = true;
        let pipe = kid.stdin.as_mut().ok_or("no stdin pipe".to_owned())?;
        pipe.write_all(payload.as_bytes())
            .map_err(|e| format!("write stdin PID {pid}: {e}"))?;
        pipe.flush()
            .map_err(|e| format!("flush stdin PID {pid}: {e}"))?;
        Ok(())
    };
    let poll = exit_poll(&mut child, pid, program, deadline, &mut observe)?;
    let code = exit_code(&poll)?;
    let stdout = collect_stream(&out_rx, program.timeout)?;
    let stderr = collect_stream(&err_rx, program.timeout)?;
    let _result = finish_poll(poll);
    Ok(Outcome {
        code,
        stdout,
        stderr,
    })
}

pub fn run_program_with(
    program: &Program,
    dir: &Path,
    mut observe: impl FnMut(&mut std::process::Child) -> Result<(), String>,
) -> Result<Outcome, String> {
    let mut child = program
        .build_command(dir)
        .spawn()
        .map_err(|e| format!("start {} failed: {e}", program.label()))?;
    let pid = child.id();
    let out_rx = spawn_reader(child.stdout.take(), "stdout", program.label());
    let err_rx = spawn_reader(child.stderr.take(), "stderr", program.label());
    let Some(deadline) = Instant::now().checked_add(program.timeout) else {
        return Err("launch deadline overflow".to_owned());
    };
    let poll = exit_poll(&mut child, pid, program, deadline, &mut observe)?;
    let code = exit_code(&poll)?;
    let stdout = collect_stream(&out_rx, program.timeout)?;
    let stderr = collect_stream(&err_rx, program.timeout)?;
    let _result = finish_poll(poll);
    Ok(Outcome {
        code,
        stdout,
        stderr,
    })
}

fn isolate_child(cmd: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        let _detached = cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        let _grouped = cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
}

pub(crate) fn start_child(program: &Program, dir: &Path) -> Result<std::process::Child, String> {
    program
        .build_command(dir)
        .spawn()
        .map_err(|e| format!("start {} failed: {e}", program.label()))
}

pub fn exit_code(poll: &PollState) -> Result<i32, String> {
    let PollState::Exited(status) = poll;
    crate::process::exit_status_code(status)
}

fn exit_poll(
    child: &mut std::process::Child,
    pid: u32,
    program: &Program,
    deadline: Instant,
    observe: &mut impl FnMut(&mut std::process::Child) -> Result<(), String>,
) -> Result<PollState, String> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(PollState::Exited(status)),
            Ok(None) => {}
            Err(err) => return Err(format!("wait PID {pid} failed: {err}")),
        }
        observe(child).map_err(|e| format!("observation of PID {pid} failed: {e}"))?;
        if Instant::now() >= deadline {
            kill_child(child, pid, &program.label());
            return Err(format!(
                "timed out after {:?} waiting for PID {pid} ({}) to exit",
                program.timeout,
                program.label()
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

pub fn child_is_dead(child: &mut std::process::Child) -> Result<bool, String> {
    match child.try_wait() {
        Ok(Some(_)) => Ok(true),
        Ok(None) => Ok(false),
        Err(err) => Err(format!("try_wait failed: {err}")),
    }
}

pub async fn run_program_clocked(
    program: &Program,
    dir: &Path,
    clock: &Arc<FakeClock>,
) -> Result<Outcome, String> {
    use tokio::process::Command as TokioCommand;
    let mut cmd = TokioCommand::new(&program.path);
    cmd.args(&program.args);
    cmd.current_dir(dir);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    isolate_tokio(&mut cmd);
    let child = cmd
        .spawn()
        .map_err(|e| format!("start {} failed: {e}", program.label()))?;
    let Some(pid) = child.id() else {
        return Err(format!("no PID for {}", program.label()));
    };
    let mut child = child;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let sleeper = crate::fake_clock::sleep_for(clock, program.timeout);
    let status = tokio::select! {
        result = child.wait() => result,
        () = sleeper => {
            #[cfg(windows)]
            crate::reap::stop_spawned(pid);
            let _killed = child.kill().await.is_ok();
            return Err(format!(
                "timed out after {:?} waiting for PID {pid} ({}) to exit",
                program.timeout,
                program.label()
            ));
        }
    };
    let status = status.map_err(|e| format!("wait PID {pid} failed: {e}"))?;
    let code = crate::process::exit_status_code(&status)?;
    let stdout = drain_pipe(stdout, program.timeout).await?;
    let stderr = drain_pipe(stderr, program.timeout).await?;
    Ok(Outcome {
        code,
        stdout,
        stderr,
    })
}

fn isolate_tokio(cmd: &mut tokio::process::Command) {
    #[cfg(unix)]
    {
        let _detached = cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        let _grouped = cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
}

async fn drain_pipe<R: tokio::io::AsyncReadExt + Unpin>(
    pipe: Option<R>,
    timeout: Duration,
) -> Result<String, String> {
    let Some(mut pipe) = pipe else {
        return Ok(String::new());
    };
    let mut buf = Vec::new();
    let read = tokio::time::timeout(timeout, pipe.read_to_end(&mut buf)).await;
    let Ok(Ok(_count)) = read else {
        return Err("read child pipe timed out".to_owned());
    };
    Ok(String::from_utf8_lossy(&buf).into_owned())
}
