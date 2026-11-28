use std::collections::{BTreeMap, BTreeSet};

use anybot_foundation::ContentDigest;

pub const WORKDIR: &str = "/workspace";

const MAX_LINK_HOPS: usize = 8;

#[derive(Debug, Clone)]
pub struct FileTree {
    pub(crate) files: BTreeMap<String, String>,
    links: BTreeMap<String, String>,
    pub(crate) dirs: BTreeSet<String>,
    pub(crate) workdir: String,
    home_dir: String,
    case_insensitive: bool,
}

impl Default for FileTree {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTree {
    #[must_use]
    pub fn new() -> Self {
        let mut dirs = BTreeSet::new();
        dirs.insert(WORKDIR.to_owned());
        FileTree {
            files: BTreeMap::new(),
            links: BTreeMap::new(),
            dirs,
            workdir: WORKDIR.to_owned(),
            home_dir: "/home/user".to_owned(),
            case_insensitive: false,
        }
    }

    pub fn set_workdir(&mut self, workdir: &str) {
        let normalized = normalize_path(workdir);
        self.dirs.insert(normalized.clone());
        self.workdir = normalized;
    }

    pub fn set_home_dir(&mut self, home_dir: &str) {
        self.home_dir = normalize_path(home_dir);
    }

    pub fn set_case_insensitive(&mut self, on: bool) {
        self.case_insensitive = on;
    }

    #[must_use]
    pub fn join_and_normalize(&self, path: &str) -> String {
        if path == "~" {
            return self.home_dir.clone();
        }
        if let Some(rest) = path.strip_prefix("~/") {
            let base = self.home_dir.trim_end_matches('/');
            return normalize_path(&format!("{base}/{rest}"));
        }
        if path.starts_with('/') {
            return normalize_path(path);
        }
        let base = self.workdir.trim_end_matches('/');
        normalize_path(&format!("{base}/{path}"))
    }

    #[must_use]
    pub fn canonical(&self, path: &str) -> String {
        let joined = self.join_and_normalize(path);
        let followed = self.follow_links(&joined);
        self.resolve_disk_case(&followed)
    }

    fn resolve_disk_case(&self, path: &str) -> String {
        if !self.case_insensitive {
            return path.to_owned();
        }
        let lower = path.to_lowercase();
        if let Some(key) = self.files.keys().find(|k| k.to_lowercase() == lower) {
            return key.clone();
        }
        if let Some(key) = self.dirs.iter().find(|k| k.to_lowercase() == lower) {
            return key.clone();
        }
        path.to_owned()
    }

    fn follow_links(&self, joined: &str) -> String {
        let mut current = joined.to_owned();
        for _ in 0..MAX_LINK_HOPS {
            if let Some(target) = self.lookup_link(&current) {
                current = self.join_and_normalize(target);
                continue;
            }
            if let Some(rewritten) = self.follow_prefix_link(&current) {
                current = rewritten;
                continue;
            }
            return current;
        }
        current
    }

    fn follow_prefix_link(&self, current: &str) -> Option<String> {
        let mut pos = current.len();
        while let Some(slash) = current.get(..pos).and_then(|s| s.rfind('/')) {
            if slash == 0 {
                break;
            }
            let prefix = current.get(..slash)?;
            let suffix = current.get(slash.saturating_add(1)..)?;
            if let Some(target) = self.lookup_link(prefix) {
                let target_base = self.join_and_normalize(target);
                let base = target_base.trim_end_matches('/');
                return Some(normalize_path(&format!("{base}/{suffix}")));
            }
            pos = slash;
        }
        None
    }

    fn lookup_link(&self, path: &str) -> Option<&str> {
        if !self.case_insensitive {
            return self.links.get(path).map(String::as_str);
        }
        let lower = path.to_lowercase();
        self.links
            .iter()
            .find(|(k, _)| k.to_lowercase() == lower)
            .map(|(_, v)| v.as_str())
    }

    #[must_use]
    pub fn exists(&self, path: &str) -> bool {
        let canonical = self.canonical(path);
        self.read(&canonical).is_some() || self.is_directory(&canonical)
    }

    #[must_use]
    pub fn is_directory(&self, path: &str) -> bool {
        let canonical = self.canonical(path);
        if !self.case_insensitive {
            return self.dirs.contains(&canonical);
        }
        let lower = canonical.to_lowercase();
        self.dirs.iter().any(|d| d.to_lowercase() == lower)
    }

    pub fn write(&mut self, path: &str, content: &str) {
        let canonical = self.canonical(path);
        self.register_parents(&canonical);
        self.files.insert(canonical, content.to_owned());
    }

    pub fn write_exact(&mut self, path: &str, content: &str) {
        let normalized = normalize_path(path);
        self.register_parents(&normalized);
        self.files.insert(normalized, content.to_owned());
    }

    #[must_use]
    pub fn read(&self, path: &str) -> Option<String> {
        let canonical = self.canonical(path);
        if !self.case_insensitive {
            return self.files.get(&canonical).cloned();
        }
        let lower = canonical.to_lowercase();
        self.files
            .iter()
            .find(|(k, _)| k.to_lowercase() == lower)
            .map(|(_, v)| v.clone())
    }

    #[must_use]
    pub fn digest(&self, path: &str) -> Option<ContentDigest> {
        self.read(path).map(|content| digest_text(&content))
    }

    pub fn link(&mut self, path: &str, target: &str) {
        let canonical = self.canonical(path);
        self.register_parents(&canonical);
        self.links.insert(canonical, target.to_owned());
    }

    pub fn add_dir(&mut self, path: &str) {
        let canonical = self.canonical(path);
        self.register_parents(&canonical);
        self.dirs.insert(canonical);
    }

    fn register_parents(&mut self, canonical: &str) {
        let mut rest = canonical;
        while let Some((parent, _)) = rest.rsplit_once('/') {
            if parent.is_empty() {
                self.dirs.insert("/".to_owned());
                break;
            }
            self.dirs.insert(parent.to_owned());
            rest = parent;
        }
    }
}

pub(crate) fn normalize_path(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            if parts.last().is_some_and(|last| *last != "..") {
                parts.pop();
            } else if !absolute {
                parts.push("..");
            }
            continue;
        }
        parts.push(segment);
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else if joined.is_empty() {
        ".".to_owned()
    } else {
        joined
    }
}

#[must_use]
pub fn digest_text(content: &str) -> ContentDigest {
    ContentDigest::sha256(content.as_bytes())
}

#[cfg(test)]
#[path = "fake_files_tests.rs"]
mod tests;
