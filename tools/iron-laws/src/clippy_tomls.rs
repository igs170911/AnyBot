use crate::exemptions::clippy_exemptions;
use crate::table::package_of_path;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Default)]
pub struct Config {
    pub scalars: BTreeMap<String, String>,
    pub lists: BTreeMap<String, BTreeSet<String>>,
}

fn paths_in_line(line: &str) -> Vec<String> {
    let marker = "path = \"";
    let mut out = Vec::new();
    let mut rest = line.to_owned();
    while let Some((_, tail)) = rest.split_once(marker) {
        let Some((p, _)) = tail.split_once('"') else {
            break;
        };
        out.push(p.to_owned());
        rest = tail.to_owned();
    }
    out
}

fn clean(raw: &str) -> String {
    match raw.split('#').next() {
        Some(x) => x.trim().to_owned(),
        None => String::new(),
    }
}

fn push_paths(line: &str, current_key: &str, cfg: &mut Config) {
    if current_key.is_empty() {
        return;
    }
    for p in paths_in_line(line) {
        cfg.lists
            .entry(current_key.to_owned())
            .or_default()
            .insert(p);
    }
}

fn apply_pair(cfg: &mut Config, current_key: &mut String, key: &str, value: &str) {
    if value.starts_with('[') {
        let key = key.to_owned();
        current_key.clone_from(&key);
        let entry = cfg.lists.entry(key).or_default();
        for p in paths_in_line(value) {
            entry.insert(p);
        }
        return;
    }
    if current_key.is_empty() {
        cfg.scalars.insert(key.to_owned(), value.to_owned());
    }
}

#[must_use]
pub fn parse(content: &str) -> Config {
    let mut cfg = Config::default();
    let mut current_key = String::new();
    for raw in content.lines() {
        let line = clean(raw);
        if line.is_empty() {
            continue;
        }
        if line.starts_with('{') {
            push_paths(&line, &current_key, &mut cfg);
            continue;
        }
        if let Some(inner) = line.strip_prefix("[[").and_then(|x| x.strip_suffix("]]")) {
            current_key.clone_from(&inner.to_owned());
            cfg.lists.entry(current_key.clone()).or_default();
            continue;
        }
        if !current_key.is_empty() && line.starts_with("path = ") {
            push_paths(&line, &current_key, &mut cfg);
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            apply_pair(&mut cfg, &mut current_key, key.trim(), value.trim());
        }
    }
    cfg
}

#[must_use]
pub fn root_config(repo: &Path) -> Config {
    match crate::collect::read_file_lossy(&repo.join("clippy.toml")) {
        Ok(c) => parse(&c),
        Err(_) => Config::default(),
    }
}

fn check_scalars(root: &Config, cfg: &Config, path: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (key, value) in &cfg.scalars {
        if root.scalars.get(key.as_str()) != Some(value) {
            out.push(format!(
                "{path}：「{key}」跟根目錄的 clippy.toml 不一樣（K75）"
            ));
        }
    }
    for key in root.scalars.keys() {
        if !cfg.scalars.contains_key(key.as_str()) {
            out.push(format!("{path}：少了根目錄有的「{key}」（K75）"));
        }
    }
    out
}

fn pick<'a>(
    m: &'a BTreeMap<String, BTreeSet<String>>,
    key: &str,
    empty: &'a BTreeSet<String>,
) -> &'a BTreeSet<String> {
    match m.get(key) {
        Some(s) => s,
        None => empty,
    }
}

fn check_lists(
    root: &Config,
    cfg: &Config,
    allowed: &BTreeMap<String, BTreeSet<String>>,
    path: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    let empty = BTreeSet::new();
    for key in ["disallowed-methods", "disallowed-types"] {
        let root_set = pick(&root.lists, key, &empty);
        let crate_set = pick(&cfg.lists, key, &empty);
        let allowed_set = pick(allowed, key, &empty);
        for extra in crate_set.difference(root_set) {
            out.push(format!(
                "{path}：多了根目錄沒有的禁用項目「{extra}」（K75）"
            ));
        }
        for missing in root_set.difference(crate_set) {
            if !allowed_set.contains(missing.as_str()) {
                out.push(format!(
                    "{path}：少了根目錄的禁用項目「{missing}」，但這個套件不准豁免它（K75）"
                ));
            }
        }
        for extra_allowed in allowed_set.difference(root_set) {
            out.push(format!(
                "{path}：豁免清單裡的「{extra_allowed}」根目錄根本沒禁用（K75）"
            ));
        }
    }
    out
}

#[must_use]
pub fn violations(root: &Config, files: &[(String, String)]) -> Vec<String> {
    let exemptions = clippy_exemptions();
    let mut out = Vec::new();
    for (path, content) in files {
        let Some(pkg) = package_of_path(path) else {
            out.push(format!("{path}：clippy.toml 放在不认识的資料夾（K75）"));
            continue;
        };
        let Some(allowed) = exemptions.get(&pkg) else {
            out.push(format!(
                "{path}：不准有自己的 clippy.toml（K75：只有准許的套件才有）"
            ));
            continue;
        };
        let cfg = parse(content);
        out.extend(check_scalars(root, &cfg, path));
        out.extend(check_lists(root, &cfg, allowed, path));
    }
    for pkg in exemptions.keys() {
        let has_file = files
            .iter()
            .any(|(path, _)| package_of_path(path).as_ref() == Some(pkg));
        if !has_file {
            out.push(format!(
                "准許的套件 {pkg} 沒有自己的 clippy.toml（K75：豁免要寫在套件自己的 clippy.toml）"
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> String {
        "msrv = \"1.98\"\ndisallowed-methods = [ { path = \"std::env::var\", reason = \"r\" }, { path = \"std::fs::write\", reason = \"r\" }, { path = \"std::process::Command::new\", reason = \"r\" } ]\ndisallowed-types = [ { path = \"std::process::Command\", reason = \"r\" } ]\n".to_owned()
    }

    #[test]
    fn test_iron_laws_clippy_toml_in_unauthorized_package_is_blocked() {
        let root = parse(&base());
        let v = violations(&root, &[("crates/domain/clippy.toml".to_owned(), base())]);
        assert!(v.iter().any(|x| x.contains("不准有自己的 clippy.toml")));
    }

    #[test]
    fn test_iron_laws_unauthorized_removal_is_blocked() {
        let root = parse(&base());
        let trimmed = base().replace("std::env::var", "x-removed");
        let v = violations(
            &root,
            &[("crates/computers/clippy.toml".to_owned(), trimmed)],
        );
        assert!(
            v.iter()
                .any(|x| x.contains("少了根目錄的禁用項目") && x.contains("env::var"))
        );
    }

    #[test]
    fn test_iron_laws_scalar_and_extra_item_mismatches_are_blocked() {
        let root = parse(&base());
        let changed = base().replace("1.98", "1.97");
        let extra = base() + "{ path = \"std::extra::thing\", reason = \"r\" }\n";
        let v = violations(
            &root,
            &[
                ("crates/foundation/clippy.toml".to_owned(), changed),
                ("crates/foundation/clippy.toml".to_owned(), extra),
            ],
        );
        assert!(v.iter().any(|x| x.contains("msrv")));
        assert!(v.iter().any(|x| x.contains("std::extra::thing")));
    }

    #[test]
    fn test_iron_laws_real_repo_clippy_tomls_match_their_packages() {
        let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root = root_config(&repo);
        let files = crate::collect::clippy_toml_files(&repo);
        assert!(!files.is_empty());
        assert_eq!(violations(&root, &files), Vec::<String>::new());
    }
}
