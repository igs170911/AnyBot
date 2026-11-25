use std::io::BufRead;
use std::io::Write as _;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::launch::{Outcome, Program};
use crate::process::{
    PollState, collect_stream, exit_status_code, finish_poll, kill_child, poll_exit, spawn_bytes,
};

pub struct Running {
    child: std::process::Child,
    label: String,
    timeout: Duration,
    lines: mpsc::Receiver<String>,
    lines_thread: Option<thread::JoinHandle<()>>,
    stderr: mpsc::Receiver<Vec<u8>>,
}

impl Running {
    pub fn start(program: &Program, dir: &std::path::Path) -> Result<Self, String> {
        let mut child = crate::launch::start_child(program, dir)?;
        let (lines, lines_thread) = spawn_line_reader(child.stdout.take(), program.label());
        let stderr = spawn_bytes(
            child.stderr.take().map(boxed_read),
            "stderr",
            program.label(),
        );
        Ok(Running {
            child,
            label: program.label(),
            timeout: program.timeout,
            lines,
            lines_thread: Some(lines_thread),
            stderr,
        })
    }

    pub fn child(&mut self) -> &mut std::process::Child {
        &mut self.child
    }

    #[must_use]
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn next_stdout_line(&mut self) -> Result<String, String> {
        match self.lines.recv_timeout(self.timeout) {
            Ok(line) => Ok(line),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let pid = self.pid();
                let label = self.label.clone();
                let note = self.stderr_note();
                kill_child(&mut self.child, pid, &label);
                Err(format!(
                    "timed out after {:?} waiting for one stdout line from {}; stderr so far: {note}",
                    self.timeout, self.label
                ))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let note = self.stderr_note();
                Err(format!(
                    "stdout ended for {}; stderr so far: {note}",
                    self.label
                ))
            }
        }
    }

    pub fn kill(&mut self) -> Result<Outcome, String> {
        let pid = self.pid();
        let label = self.label.clone();
        kill_child(&mut self.child, pid, &label);
        let Some(deadline) = Instant::now().checked_add(Duration::from_secs(5)) else {
            return Err("kill deadline overflow".to_owned());
        };
        let poll = poll_exit(&mut self.child, pid, Duration::from_secs(5), deadline)?;
        self.collect(poll)
    }

    pub fn finish(&mut self) -> Result<Outcome, String> {
        let pid = self.pid();
        let Some(deadline) = Instant::now().checked_add(self.timeout) else {
            return Err("finish deadline overflow".to_owned());
        };
        let poll = poll_exit(&mut self.child, pid, self.timeout, deadline)?;
        self.collect(poll)
    }

    fn collect(&mut self, poll: PollState) -> Result<Outcome, String> {
        let status = finish_poll(poll);
        let stderr = self.stderr_text();
        Ok(Outcome {
            code: exit_status_code(&status)?,
            stdout: String::new(),
            stderr,
        })
    }

    fn stderr_text(&mut self) -> String {
        let Some(thread) = self.lines_thread.take() else {
            return String::new();
        };
        let _result = thread.join();
        collect_stream(&self.stderr, self.timeout)
            .unwrap_or_else(|err| format!("stderr unreadable: {err}"))
    }

    pub fn stderr_note(&mut self) -> String {
        match self.stderr.try_recv() {
            Ok(buf) => String::from_utf8_lossy(&buf).into_owned(),
            Err(_) => "(nothing collected yet)".to_owned(),
        }
    }
}

fn boxed_read<R: std::io::Read + Send + 'static>(stream: R) -> Box<dyn std::io::Read + Send> {
    Box::new(stream)
}

fn spawn_line_reader(
    stream: Option<std::process::ChildStdout>,
    label: String,
) -> (mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let Some(stream) = stream else {
            let mut stderr = std::io::stderr();
            let _result = writeln!(stderr, "harness: {label} has no stdout pipe");
            let _result = tx.send(String::new());
            return;
        };
        let mut reader = std::io::BufReader::new(stream);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            }
        }
    });
    (rx, handle)
}
