use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use crate::launch::{Outcome, Program, start_child};

pub struct Running {
    child: std::process::Child,
    stdout: BufReader<std::process::ChildStdout>,
    stderr: Arc<Mutex<Vec<u8>>>,
    stderr_thread: Option<thread::JoinHandle<()>>,
}

impl Running {
    pub fn start(program: &Program, dir: &Path) -> Result<Self, String> {
        let mut child = start_child(program, dir)?;
        let stdout = BufReader::new(child.stdout.take().ok_or("stdout 沒有接出來")?);
        let mut stderr_stream = child.stderr.take().ok_or("stderr 沒有接出來")?;
        let stderr = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&stderr);
        let stderr_thread = thread::spawn(move || {
            let mut buf = Vec::new();
            let _result = stderr_stream.read_to_end(&mut buf);
            if let Ok(mut guard) = sink.lock() {
                *guard = buf;
            }
        });
        Ok(Running {
            child,
            stdout,
            stderr,
            stderr_thread: Some(stderr_thread),
        })
    }

    #[must_use]
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn next_stdout_line(&mut self) -> Result<String, String> {
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .map_err(|e| format!("讀 stdout 失敗：{e}"))?;
        Ok(line)
    }

    pub fn kill(&mut self) -> Result<Outcome, String> {
        self.child
            .kill()
            .map_err(|e| format!("殺 PID {} 失敗：{e}", self.pid()))?;
        self.finish()
    }

    pub fn finish(&mut self) -> Result<Outcome, String> {
        let stdout = String::new();
        let status = self
            .child
            .wait()
            .map_err(|e| format!("等 PID {} 失敗：{e}", self.pid()))?;
        let stderr_text = self.join_stderr();
        Ok(Outcome {
            code: status.code().unwrap_or(-1),
            stdout,
            stderr: stderr_text,
        })
    }

    fn join_stderr(&mut self) -> String {
        let Some(thread) = self.stderr_thread.take() else {
            return String::new();
        };
        if thread.join().is_err() {
            return String::new();
        }
        let Ok(guard) = self.stderr.lock() else {
            return String::new();
        };
        String::from_utf8_lossy(&guard).into_owned()
    }
}
