use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

#[derive(Debug)]
pub struct Program {
    pub path: PathBuf,
    pub args: Vec<String>,
}

impl Program {
    #[must_use]
    pub fn new(path: &str, args: Vec<String>) -> Self {
        Program {
            path: PathBuf::from(path),
            args,
        }
    }

    fn build_command(&self, dir: &Path) -> Command {
        let mut cmd = Command::new(&self.path);
        cmd.args(&self.args);
        cmd.current_dir(dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd
    }
}

#[derive(Debug)]
pub struct Outcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_program(program: &Program, dir: &Path) -> Result<Outcome, String> {
    let mut child = start_child(program, dir)?;
    let stdout = take_stream(&mut child.stdout);
    let stderr = take_stream(&mut child.stderr);
    let status = child
        .wait()
        .map_err(|e| format!("等 {} 失敗：{e}", program.path.display()))?;
    Ok(Outcome {
        code: status.code().unwrap_or(-1),
        stdout,
        stderr,
    })
}

pub(crate) fn start_child(program: &Program, dir: &Path) -> Result<Child, String> {
    program
        .build_command(dir)
        .spawn()
        .map_err(|e| format!("啟動 {} 失敗：{e}", program.path.display()))
}

fn take_stream<R: Read>(stream: &mut Option<R>) -> String {
    let Some(mut s) = stream.take() else {
        return String::new();
    };
    let mut buf = Vec::new();
    let _result = s.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}
