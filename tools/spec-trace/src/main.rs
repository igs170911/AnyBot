//! spec-trace 執行檔：跑一次檢查，印覆蓋數字；有違規就非零 exit。
//! 基準檔缺失＝關卡失敗（不自動產生）。產生基準檔：`make spec-baseline`
//! （=`cargo run -p spec-trace -- --write-baseline`）。

#![forbid(unsafe_code)]
// regex 是 lib 相依，bin 本身用不到。
#![allow(unused_crate_dependencies)]
// 開發工具：讀 repo 的檔是本職（clippy.toml 的檔案禁用在這打開）。
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use spec_trace::{code_ids_from_source, spec_ids_from_md};

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn main() -> Result<(), String> {
    let write_baseline = std::env::args().any(|a| a == "--write-baseline");
    let repo = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|e| format!("找不到 repo 根目錄：{e}"))?;

    // 1) 規格測試編號：spec/10-behavior/[1-6]-*.md。
    let mut spec_ids: BTreeSet<String> = BTreeSet::new();
    let behavior = repo.join("spec/10-behavior");
    let mut spec_files: Vec<PathBuf> = Vec::new();
    for entry in
        std::fs::read_dir(&behavior).map_err(|e| format!("讀 spec/10-behavior 失敗：{e}"))?
    {
        let path = entry
            .map_err(|e| format!("讀 spec/10-behavior 失敗：{e}"))?
            .path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.len() >= 5
            && matches!(name.as_bytes()[0], b'1'..=b'6')
            && name[1..].starts_with('-')
            && name.to_ascii_lowercase().ends_with(".md")
        {
            spec_files.push(path);
        }
    }
    spec_files.sort();
    for path in spec_files {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("讀 {} 失敗：{e}", path.display()))?;
        spec_ids.extend(spec_ids_from_md(&content)?);
    }

    // 2) 產品程式碼的標記：crates/、apps/、tests/、web/、adapters/（不掃 tools/）。
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in ["crates", "apps", "tests", "web", "adapters"] {
        collect_rs(&repo.join(dir), &mut files);
    }
    files.sort();
    let mut code_ids: BTreeSet<String> = BTreeSet::new();
    for path in files {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("讀 {} 失敗：{e}", path.display()))?;
        code_ids.extend(code_ids_from_source(&content)?);
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

    let report = spec_trace::check(&spec_ids, &code_ids, baseline_set.as_ref());
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
