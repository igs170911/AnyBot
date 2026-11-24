//! depcheck 執行檔：在 repo 根跑 `cargo metadata`，檢查依賴方向和 allow 出處。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]
#![allow(unused_crate_dependencies)] // serde／serde_json 由 depcheck lib 使用。

use std::io::Read;

/// 收集要掃的原始檔（crates/、apps/、tools/ 下的 .rs）。
fn collect_source_files(dir: &std::path::Path, prefix: &str, out: &mut Vec<(String, String)>) {
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
        }
    }
}

fn read_to_string_lossy(path: &std::path::Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn main() {
    let meta_out = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .output();
    let output = match meta_out {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            eprintln!(
                "depcheck：cargo metadata 失敗：{}",
                String::from_utf8_lossy(&o.stderr)
            );
            std::process::exit(2);
        }
        Err(e) => {
            eprintln!("depcheck：跑不了 cargo：{e}（請在 repo 根跑）");
            std::process::exit(2);
        }
    };
    let json = String::from_utf8_lossy(&output.stdout).into_owned();

    let mut violations = Vec::new();
    match depcheck::check_metadata(&json) {
        Ok(v) => violations.extend(v),
        Err(e) => {
            eprintln!("depcheck：{e}");
            std::process::exit(2);
        }
    }

    let mut files = Vec::new();
    for dir in ["crates", "apps", "tools"] {
        collect_source_files(std::path::Path::new(dir), "", &mut files);
    }
    violations.extend(depcheck::scan_exemption_attrs(&files));

    if violations.is_empty() {
        println!("depcheck：依賴方向全綠（{} 個原始檔掃過）", files.len());
    } else {
        for v in &violations {
            eprintln!("depcheck 違規：{v}");
        }
        eprintln!("depcheck：{} 個違規", violations.len());
        std::process::exit(1);
    }
}
