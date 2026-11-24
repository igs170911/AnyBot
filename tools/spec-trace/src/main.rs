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
        .map_err(|e| format!("跑 git 失敗：{e}"))?;
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
            "讀 {git_ref} 的 spec/10-behavior 失敗（ref 存在嗎？）"
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
        let content =
            git_show(repo, git_ref, line)?.ok_or_else(|| format!("讀 {git_ref}:{line} 失敗"))?;
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
            .map_err(|e| format!("讀 {} 失敗：{e}", path.display()))?;
        code_ids.extend(code_ids_from_source(&content)?);
        code_ids.extend(code_ids_from_ts_source(&content)?);
    }
    Ok(code_ids)
}

fn read_spec_ids(repo: &Path) -> Result<BTreeSet<String>, String> {
    let mut spec_ids: BTreeSet<String> = BTreeSet::new();
    for path in spec_file_names(&repo.join("spec/10-behavior")) {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("讀 {} 失敗：{e}", path.display()))?;
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
            "--against {git_ref}：那個 ref 裡沒有 tools/spec-trace/baseline.txt"
        )),
        Some(c) => {
            let ref_set = read_baseline(&c);
            let ref_ids = ref_spec_ids(repo, git_ref)?;
            let new_spec: BTreeSet<String> = spec_ids.difference(&ref_ids).cloned().collect();
            report
                .violations
                .extend(spec_trace::check_baseline_against_ref(
                    current, &ref_set, &new_spec,
                ));
            say(&[format!(
                "spec-trace 基準比對：ref {git_ref} 的基準檔 {} 個、現在 {} 個、這段變更新增的規格編號 {} 個",
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
        .map_err(|e| format!("找不到 repo 根目錄：{e}"))?;
    let spec_ids = read_spec_ids(&repo)?;
    let code_ids = read_code_ids(&repo)?;
    let baseline_path = repo.join("tools/spec-trace/baseline.txt");
    if write_baseline {
        let content = generate_baseline(&spec_ids, &code_ids);
        std::fs::write(&baseline_path, content).map_err(|e| format!("寫基準檔失敗：{e}"))?;
        say(&[format!(
            "spec-trace：已重新產生 {}",
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
                "基準檔 tools/spec-trace/baseline.txt 不存在：--against 比對需要本機基準檔"
                    .to_owned(),
            ),
            Some(current) => against_report(&repo, &git_ref, &current, &spec_ids, &mut report)?,
        }
    }

    say(&[format!(
        "spec-trace 覆蓋：總數 {}、有測試 {}、還沒有 {}",
        report.total,
        report.tested(),
        report.missing()
    )])
    .map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = Vec::new();
    for v in &report.violations {
        lines.push(format!("spec-trace 違規：{v}"));
    }
    if lines.is_empty() {
        lines.push("spec-trace：全綠".to_owned());
    }
    say(&lines).map_err(|e| e.to_string())?;
    if !report.violations.is_empty() {
        return Err(format!("spec-trace：{} 個違規", report.violations.len()));
    }
    Ok(())
}
