#![forbid(unsafe_code)]

extern crate regex as _;

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use spec_trace::{
    code_ids_from_source, code_ids_from_ts_source, generate_baseline, read_baseline,
    spec_ids_from_md,
};

const SKIP_DIRS: [&str; 7] = [
    "node_modules",
    "dist",
    "target",
    "gen",
    "coverage",
    ".pnpm",
    "build",
];

fn is_behavior_file(name: &str) -> bool {
    name.len() >= 5
        && matches!(name.as_bytes().first(), Some(b'1'..=b'6'))
        && matches!(name.as_bytes().get(1), Some(b'-'))
        && name.to_ascii_lowercase().ends_with(".md")
}

fn product_code_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !SKIP_DIRS.contains(&name) {
                product_code_files(&path, out);
            }
            continue;
        }
        if path
            .extension()
            .is_some_and(|e| e == "rs" || e == "ts" || e == "tsx")
        {
            out.push(path);
        }
    }
}

fn spec_file_names(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if is_behavior_file(name) {
            out.push(path);
        }
    }
    out.sort();
    out
}

fn run_git(repo: &Path, args: &[&str]) -> Result<(bool, String), String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("git run failed: {e}"))?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    ))
}

fn git_show(repo: &Path, git_ref: &str, path: &str) -> Result<Option<String>, String> {
    let (ok, stdout) = run_git(repo, &["show", &format!("{git_ref}:{path}")])?;
    if ok { Ok(Some(stdout)) } else { Ok(None) }
}

fn ref_spec_ids(repo: &Path, git_ref: &str) -> Result<BTreeSet<String>, String> {
    let (ok, stdout) = run_git(
        repo,
        &[
            "ls-tree",
            "-r",
            "--name-only",
            git_ref,
            "--",
            "spec/10-behavior",
        ],
    )?;
    if !ok {
        return Err(format!(
            "failed to read spec/10-behavior from {git_ref} (does the ref exist?)"
        ));
    }
    let mut ids = BTreeSet::new();
    for line in stdout.lines() {
        let Some(name) = Path::new(line).file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_behavior_file(name) {
            continue;
        }
        let content = git_show(repo, git_ref, line)?
            .ok_or_else(|| format!("failed to read {git_ref}:{line}"))?;
        ids.extend(spec_ids_from_md(&content)?);
    }
    Ok(ids)
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    let idx = args.iter().position(|a| a == flag)?;
    args.get(idx.saturating_add(1)).cloned()
}

fn read_code_ids(repo: &Path) -> Result<BTreeSet<String>, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in ["crates", "apps", "tests", "web", "adapters"] {
        product_code_files(&repo.join(dir), &mut files);
    }
    files.sort();
    let mut code_ids: BTreeSet<String> = BTreeSet::new();
    for path in files {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        code_ids.extend(code_ids_from_source(&content)?);
        code_ids.extend(code_ids_from_ts_source(&content)?);
    }
    Ok(code_ids)
}

fn read_spec_ids(repo: &Path) -> Result<BTreeSet<String>, String> {
    let mut spec_ids: BTreeSet<String> = BTreeSet::new();
    for path in spec_file_names(&repo.join("spec/10-behavior")) {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        spec_ids.extend(spec_ids_from_md(&content)?);
    }
    Ok(spec_ids)
}

fn say(lines: &[String]) -> Result<(), std::io::Error> {
    let mut text = String::new();
    for l in lines {
        text.push_str(l);
        text.push('\n');
    }
    let mut stdout = std::io::stdout();
    writeln!(stdout, "{text}")
}

fn read_returned(repo: &Path) -> Result<BTreeSet<String>, String> {
    let path = repo.join("tools/spec-trace/returned.txt");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    Ok(read_baseline(&content))
}

fn against_report(
    repo: &Path,
    git_ref: &str,
    current: &BTreeSet<String>,
    spec_ids: &BTreeSet<String>,
    report: &mut spec_trace::Report,
) -> Result<(), String> {
    let ref_baseline = git_show(repo, git_ref, "tools/spec-trace/baseline.txt")?;
    match ref_baseline {
        None => report.violations.push(format!(
            "--against {git_ref}: that ref has no tools/spec-trace/baseline.txt"
        )),
        Some(c) => {
            let ref_set = read_baseline(&c);
            let ref_ids = ref_spec_ids(repo, git_ref)?;
            let new_spec: BTreeSet<String> = spec_ids.difference(&ref_ids).cloned().collect();
            let returned = read_returned(repo)?;
            report
                .violations
                .extend(spec_trace::check_baseline_against_ref(
                    current, &ref_set, &new_spec, &returned,
                ));
            say(&[format!(
                "spec-trace baseline compare: ref {git_ref} baseline has {} ids, now {} ids, this change adds {} spec ids",
                ref_set.len(),
                current.len(),
                new_spec.len()
            )
])
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let write_baseline = args.iter().any(|a| a == "--write-baseline");
    let against = arg_value(&args, "--against");
    let repo = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|e| format!("cannot find the repo root: {e}"))?;
    let spec_ids = read_spec_ids(&repo)?;
    let code_ids = read_code_ids(&repo)?;
    let baseline_path = repo.join("tools/spec-trace/baseline.txt");
    if write_baseline {
        let content = generate_baseline(&spec_ids, &code_ids);
        std::fs::write(&baseline_path, content)
            .map_err(|e| format!("baseline write failed: {e}"))?;
        say(&[format!(
            "spec-trace: regenerated {}",
            baseline_path.display()
        )])
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let baseline = std::fs::read_to_string(&baseline_path).ok();
    let baseline_set = baseline.as_ref().map(|c| read_baseline(c));
    let mut report = spec_trace::check(&spec_ids, &code_ids, baseline_set.as_ref());

    if let Some(git_ref) = against {
        match baseline_set {
            None => report.violations.push(
                "baseline tools/spec-trace/baseline.txt is missing: --against needs a local baseline"
                    .to_owned(),
            ),
            Some(current) => against_report(&repo, &git_ref, &current, &spec_ids, &mut report)?,
        }
    }

    say(&[format!(
        "spec-trace coverage: total {}, tested {}, untested {}",
        report.total,
        report.tested(),
        report.missing()
    )])
    .map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = Vec::new();
    for v in &report.violations {
        lines.push(format!("spec-trace violation: {v}"));
    }
    if lines.is_empty() {
        lines.push("spec-trace: all green".to_owned());
    }
    say(&lines).map_err(|e| e.to_string())?;
    if !report.violations.is_empty() {
        return Err(format!(
            "spec-trace: {} violations",
            report.violations.len()
        ));
    }
    Ok(())
}
