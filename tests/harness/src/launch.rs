use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::time::Instant;

use crate::process::{PollState, collect_stream, finish_poll, kill_child, spawn_reader};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct Program {
    pub path: PathBuf,
    pub args: Vec<String>,
    pub timeout: Duration,
}

impl Program {
    #[must_use]
    pub fn new(path: &str, args: Vec<String>) -> Self {
        Program {
            path: PathBuf::from(path),
            args,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn build_command(&self, dir: &Path) -> Command {
        let mut cmd = Command::new(&self.path);
        cmd.args(&self.args);
        cmd.current_dir(dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
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
