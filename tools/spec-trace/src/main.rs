//! spec-trace 執行檔：跑一次檢查，印覆蓋數字；有違規就非零 exit。
//! 基準檔缺失＝關卡失敗（不自動產生）。產生基準檔：`make spec-baseline`
//! （=`cargo run -p spec-trace -- --write-baseline`）。
//! `--against <git ref>`：基準檔只能變短（跟那個 ref 比，見 `docs/tasks/M2` 第 5 項）。

#![forbid(unsafe_code)]
// regex 是 lib 相依，bin 本身用不到。
#![allow(unused_crate_dependencies)]
// 開發工具：讀 repo 的檔、跑 git 是本職（clippy.toml 的檔案禁用在這打開）。
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use spec_trace::{code_ids_from_source, code_ids_from_ts_source, spec_ids_from_md};

/// 掃產品程式碼時跳過的目錄（建置產物、 dependency 目錄）。
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    "target",
    "gen",
    "coverage",
    ".pnpm",
    "build",
];

fn product_code_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if SKIP_DIRS.contains(&name) {
                continue;
            }
            product_code_files(&path, out);
            continue;
        }
        // Rust 和 TypeScript 都算（M2 第 4 項：.ts、.tsx 裡註解的 spec: T-… 也算）。
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
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.len() >= 5
            && matches!(name.as_bytes()[0], b'1'..=b'6')
            && name[1..].starts_with('-')
            && name.to_ascii_lowercase().ends_with(".md")
        {
            out.push(path);
        }
    }
    out.sort();
    out
}

fn run_git(repo: &Path, args: &[&str]) -> Result<(bool, String), String> {
    let out = Command::new("git")
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

/// `git show <ref>:<path>`；path 在那個 ref 不存在時回 None。
fn git_show(repo: &Path, git_ref: &str, path: &str) -> Result<Option<String>, String> {
    let (ok, stdout) = run_git(repo, &["show", &format!("{git_ref}:{path}")])?;
    if ok { Ok(Some(stdout)) } else { Ok(None) }
}

/// 那個 ref 上的 spec/10-behavior 編號表（路徑用 ref 上的，不在 repo 裡）。
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
        let name = Path::new(line)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if name.len() >= 5
            && matches!(name.as_bytes()[0], b'1'..=b'6')
            && name[1..].starts_with('-')
            && name.to_ascii_lowercase().ends_with(".md")
        {
            let content = git_show(repo, git_ref, line)?
                .ok_or_else(|| format!("讀 {git_ref}:{line} 失敗"))?;
            ids.extend(spec_ids_from_md(&content)?);
        }
    }
    Ok(ids)
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    let idx = args.iter().position(|a| a == flag)?;
    args.get(idx.saturating_add(1)).cloned()
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let write_baseline = args.iter().any(|a| a == "--write-baseline");
    let against = arg_value(&args, "--against");
    let repo = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|e| format!("找不到 repo 根目錄：{e}"))?;

    // 1) 規格測試編號：spec/10-behavior/[1-6]-*.md。
    let mut spec_ids: BTreeSet<String> = BTreeSet::new();
    for path in spec_file_names(&repo.join("spec/10-behavior")) {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("讀 {} 失敗：{e}", path.display()))?;
        spec_ids.extend(spec_ids_from_md(&content)?);
    }

    // 2) 產品程式碼的標記：crates/、apps/、tests/、web/、adapters/（不掃 tools/）。
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

    // 3) 基準檔。
    let baseline_path = repo.join("tools/spec-trace/baseline.txt");
    if write_baseline {
        let content = spec_trace::generate_baseline(&spec_ids, &code_ids);
        std::fs::write(&baseline_path, content).map_err(|e| format!("寫基準檔失敗：{e}"))?;
        println!("spec-trace：已重新產生 {}", baseline_path.display());
        return Ok(());
    }
    let baseline = std::fs::read_to_string(&baseline_path).ok();
    let baseline_set = baseline.as_ref().map(|c| spec_trace::read_baseline(c));

    let mut report = spec_trace::check(&spec_ids, &code_ids, baseline_set.as_ref());

    // 4) 基準檔只能變短（M2 第 5 項）：跟指定的 git ref 比。
    if let Some(git_ref) = against {
        match baseline_set {
            None => report.violations.push(
                "基準檔 tools/spec-trace/baseline.txt 不存在：--against 比對需要本機基準檔"
                    .to_owned(),
            ),
            Some(current) => {
                let ref_baseline = git_show(&repo, &git_ref, "tools/spec-trace/baseline.txt")?;
                match ref_baseline {
                    None => report.violations.push(format!(
                        "--against {git_ref}：那個 ref 裡沒有 tools/spec-trace/baseline.txt"
                    )),
                    Some(c) => {
                        let ref_set = spec_trace::read_baseline(&c);
                        let ref_ids = ref_spec_ids(&repo, &git_ref)?;
                        let new_spec: BTreeSet<String> =
                            spec_ids.difference(&ref_ids).cloned().collect();
                        report
                            .violations
                            .extend(spec_trace::check_baseline_against_ref(
                                &current, &ref_set, &new_spec,
                            ));
                        println!(
                            "spec-trace 基準比對：ref {git_ref} 的基準檔 {} 個、現在 {} 個、這段變更新增的規格編號 {} 個",
                            ref_set.len(),
                            current.len(),
                            new_spec.len()
                        );
                    }
                }
            }
        }
    }

    println!(
        "spec-trace 覆蓋：總數 {}、有測試 {}、還沒有 {}",
        report.total, report.tested, report.missing
    );
    for v in &report.violations {
        println!("spec-trace 違規：{v}");
    }
    if !report.violations.is_empty() {
        return Err(format!("spec-trace：{} 個違規", report.violations.len()));
    }
    println!("spec-trace：全綠");
    Ok(())
}
