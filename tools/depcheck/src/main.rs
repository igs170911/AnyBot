#![forbid(unsafe_code)]

extern crate serde as _;
extern crate serde_json as _;

use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

fn collect_source_files(dir: &Path, prefix: &str, out: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let rel = match path.to_str() {
            Some(p) => format!("{prefix}{p}"),
            None => continue,
        };
        if path.is_dir() {
            collect_source_files(&path, prefix, out);
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
            && let Ok(content) = read_to_string_lossy(&path)
        {
            out.push((rel, content));
        } else if path.file_name().is_some_and(|n| n == "Cargo.toml")
            && let Ok(content) = read_to_string_lossy(&path)
        {
            out.push((rel, content));
        }
    }
}

fn read_to_string_lossy(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn cargo_metadata_json() -> Result<String, String> {
    let out = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .output()
        .map_err(|e| format!("depcheck: cannot run cargo: {e} (run it from the repo root)"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    Err(format!(
        "depcheck: cargo metadata failed: {}",
        String::from_utf8_lossy(&out.stderr)
    ))
}

fn report(violations: &[String], files_len: usize) -> ExitCode {
    let mut lines = Vec::new();
    if violations.is_empty() {
        lines.push(format!(
            "depcheck: dependency direction all green ({files_len} source files scanned)"
        ));
    } else {
        for v in violations {
            lines.push(format!("depcheck violation: {v}"));
        }
        lines.push(format!("depcheck: {} violations", violations.len()));
    }
    let mut stdout = std::io::stdout();
    for line in &lines {
        if writeln!(stdout, "{line}").is_err() {
            return ExitCode::FAILURE;
        }
    }
    if violations.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn warn_stderr(message: &str) {
    let mut stderr = std::io::stderr();
    if writeln!(stderr, "{message}").is_err() {}
}

fn main() -> ExitCode {
    let json = match cargo_metadata_json() {
        Ok(j) => j,
        Err(e) => {
            warn_stderr(&e);
            return ExitCode::from(2);
        }
    };
    let mut violations = match depcheck::check_metadata(&json) {
        Ok(v) => v,
        Err(e) => {
            warn_stderr(&format!("depcheck: {e}"));
            return ExitCode::from(2);
        }
    };
    let mut files = Vec::new();
    for dir in ["crates", "apps", "tools", "tests"] {
        collect_source_files(&PathBuf::from(dir), "", &mut files);
    }
    if let Ok(content) = read_to_string_lossy(&PathBuf::from("Cargo.toml")) {
        files.push(("Cargo.toml".to_owned(), content));
    }
    violations.extend(depcheck::scan_exemption_attrs(&files));
    violations.extend(depcheck::cargo_toml_violations(&files));
    violations.extend(depcheck::env_usage_violations(&files));
    report(&violations, files.len())
}
