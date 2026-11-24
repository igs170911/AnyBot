//! spec-trace 執行檔：讀 `spec/10-behavior/[1-6]-*.md` 和程式碼，跑追蹤表關卡。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(unused_crate_dependencies)] // regex 由 spec_trace:: 間接使用。
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

use std::collections::BTreeSet;
use std::io::Read;

fn read_lossy(path: &std::path::Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn fail(msg: &str) -> ! {
    eprintln!("spec-trace：{msg}");
    std::process::exit(2);
}

/// 收集 spec/10-behavior/[1-6]-*.md 的內容。
fn spec_contents() -> Vec<String> {
    let dir = std::path::Path::new("spec/10-behavior");
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => fail(&format!("讀不了 spec/10-behavior：{e}（請在 repo 根跑）")),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let numbered = name.len() > 1
            && name.starts_with(|c: char| c.is_ascii_digit())
            && name.to_ascii_lowercase().ends_with(".md")
            && !name.starts_with('0');
        if numbered {
            match read_lossy(&path) {
                Ok(c) => out.push(c),
                Err(e) => fail(&format!("讀不了 {}: {e}", path.display())),
            }
        }
    }
    if out.is_empty() {
        fail("spec/10-behavior 裡找不到 [1-6]-*.md 規格檔");
    }
    out
}

/// 收集要掃標記的程式碼檔。
fn collect_code_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_code_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
        {
            out.push(path);
        }
    }
}

fn main() {
    let mut spec_ids = BTreeSet::new();
    for content in spec_contents() {
        match spec_trace::spec_ids_from_md(&content) {
            Ok(ids) => spec_ids.extend(ids),
            Err(e) => fail(&e),
        }
    }

    let mut code_ids = BTreeSet::new();
    let mut files = Vec::new();
    for dir in ["crates", "apps", "tools", "tests", "web", "adapters"] {
        collect_code_files(std::path::Path::new(dir), &mut files);
    }
    for path in &files {
        let Ok(content) = read_lossy(path) else {
            continue;
        };
        match spec_trace::code_ids_from_source(&content) {
            Ok(ids) => code_ids.extend(ids),
            Err(e) => fail(&e),
        }
    }

    let baseline_path = std::path::Path::new("tools/spec-trace/baseline.txt");
    let baseline = match read_lossy(baseline_path) {
        Ok(c) => Some(spec_trace::read_baseline(&c)),
        Err(_) => None,
    };

    let report = spec_trace::check(&spec_ids, &code_ids, baseline.as_ref());

    if let Some(content) = &report.generated_baseline {
        println!(
            "spec-trace：基準檔還沒有，已產生 {}（共 {} 個編號）",
            baseline_path.display(),
            content.lines().count().saturating_sub(1)
        );
        if let Err(e) = std::fs::write(baseline_path, content) {
            fail(&format!("寫基準檔失敗：{e}"));
        }
    }

    println!(
        "spec-trace 覆蓋：總數 {}、有測試 {}、還沒有 {}",
        report.total, report.tested, report.missing
    );
    if report.violations.is_empty() {
        println!("spec-trace：全綠");
    } else {
        for v in &report.violations {
            eprintln!("spec-trace 違規：{v}");
        }
        eprintln!("spec-trace：{} 個違規", report.violations.len());
        std::process::exit(1);
    }
}
