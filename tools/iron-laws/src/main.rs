#![forbid(unsafe_code)]

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    match manifest.join("../..").canonicalize() {
        Ok(p) => p,
        Err(_) => manifest,
    }
}

fn main() -> ExitCode {
    let violations = iron_laws::run(&repo_root());
    let mut stdout = std::io::stdout();
    for v in &violations {
        if writeln!(stdout, "{v}").is_err() {
            return ExitCode::FAILURE;
        }
    }
    if violations.is_empty() {
        if writeln!(stdout, "iron-laws: 四條鐵律全部通過").is_err() {
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }
    let tail = format!(
        "iron-laws: {} 個違規（請修到通過，不要關掉檢查）",
        violations.len()
    );
    if writeln!(stdout, "{tail}").is_err() {
        return ExitCode::FAILURE;
    }
    ExitCode::FAILURE
}
