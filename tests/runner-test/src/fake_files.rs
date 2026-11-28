use std::collections::{BTreeMap, BTreeSet};

use anybot_foundation::ContentDigest;
use anybot_protocol::{DirEntry, TargetKind};

pub const WORKDIR: &str = "/workspace";

const MAX_LINK_HOPS: usize = 8;

#[derive(Debug, Clone)]
pub struct FileTree {
    files: BTreeMap<String, String>,
    links: BTreeMap<String, String>,
    dirs: BTreeSet<String>,
    workdir: String,
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
        }
    }

    pub fn set_workdir(&mut self, workdir: &str) {
        let normalized = normalize_path(workdir);
        self.dirs.insert(normalized.clone());
        self.workdir = normalized;
    }

    #[must_use]
    pub fn canonical(&self, path: &str) -> String {
        let joined = self.join_workdir(path);
        self.follow_links(&joined)
    }

    fn join_workdir(&self, path: &str) -> String {
        if path.starts_with('/') {
            return normalize_path(path);
        }
        let base = self.workdir.trim_end_matches('/');
        normalize_path(&format!("{base}/{path}"))
    }

    fn follow_links(&self, joined: &str) -> String {
        let mut current = joined.to_owned();
        for _ in 0..MAX_LINK_HOPS {
            let Some(target) = self.links.get(&current) else {
                return current;
            };
            current = self.join_workdir(target);
        }
        current
    }

    #[must_use]
    pub fn exists(&self, path: &str) -> bool {
        let canonical = self.canonical(path);
        self.files.contains_key(&canonical) || self.dirs.contains(&canonical)
    }

    #[must_use]
    pub fn is_directory(&self, path: &str) -> bool {
        self.dirs.contains(&self.canonical(path))
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
        self.files.get(&self.canonical(path)).cloned()
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

    pub fn list_entries(&self, path: &str) -> Result<Vec<DirEntry>, String> {
        let canonical = self.canonical(path);
        if self.files.contains_key(&canonical) {
            return Err(format!("{canonical} is a file"));
        }
        if !self.dirs.contains(&canonical) {
            return Err(format!("{canonical} is not a directory"));
        }
        let prefix = if canonical == "/" {
            "/".to_owned()
        } else {
            format!("{canonical}/")
        };
        let mut out: BTreeMap<String, DirEntry> = BTreeMap::new();
        for dir_path in &self.dirs {
            if let Some(name) = immediate_child(dir_path, &prefix) {
                out.insert(
                    name.to_owned(),
                    DirEntry {
                        name: name.to_owned(),
                        kind: TargetKind::Directory,
                        size: 0,
                    },
                );
            }
        }
        for (file_path, content) in &self.files {
            if let Some(name) = immediate_child(file_path, &prefix) {
                out.insert(
                    name.to_owned(),
                    DirEntry {
                        name: name.to_owned(),
                        kind: TargetKind::File,
                        size: content.len() as u64,
                    },
                );
            }
        }
        Ok(out.into_values().collect())
    }

    pub fn search_files(&self, path: &str) -> Result<Vec<(String, String)>, String> {
        let canonical = self.canonical(path);
        if let Some(content) = self.files.get(&canonical) {
            let display = display_rel_path(&self.workdir, "/", &canonical);
            return Ok(vec![(display, content.clone())]);
        }
        if !self.dirs.contains(&canonical) {
            return Err(format!("{canonical} is not a directory"));
        }
        let dir_prefix = dir_prefix(&canonical);
        let workdir_prefix = dir_prefix_for(&self.workdir);
        let mut out = Vec::new();
        for (file_path, content) in &self.files {
            if file_path.starts_with(&dir_prefix) {
                let display = display_rel_path(&workdir_prefix, &dir_prefix, file_path);
                out.push((display, content.clone()));
            }
        }
        Ok(out)
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

fn immediate_child<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = path.strip_prefix(prefix)?;
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    Some(rest)
}

fn dir_prefix(canonical: &str) -> String {
    if canonical == "/" {
        "/".to_owned()
    } else {
        format!("{canonical}/")
    }
}

fn dir_prefix_for(workdir: &str) -> String {
    dir_prefix(&normalize_path(workdir))
}

fn display_rel_path(workdir_prefix: &str, dir_prefix: &str, file_path: &str) -> String {
    if let Some(rel) = file_path.strip_prefix(workdir_prefix) {
        return rel.to_owned();
    }
    if let Some(rel) = file_path.strip_prefix(dir_prefix) {
        return rel.to_owned();
    }
    file_path
        .rsplit_once('/')
        .map_or_else(|| file_path.to_owned(), |(_, name)| name.to_owned())
}

fn normalize_path(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            if parts.last().is_some_and(|last| *last != "..") {
                parts.pop();
                continue;
            }
            if absolute {
                continue;
            }
            parts.push("..");
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
mod tests {
    use super::*;

    #[test]
    fn test_file_tree_resolves_links_and_relative_paths_to_canonical_paths() {
        let mut tree = FileTree::new();
        tree.write("/elsewhere/data.txt", "abc");
        tree.link("link/data.txt", "/elsewhere/data.txt");
        tree.add_dir(WORKDIR);
        assert_eq!(tree.canonical("link/data.txt"), "/elsewhere/data.txt");
        assert_eq!(tree.canonical("notes.md"), "/workspace/notes.md");
        assert!(tree.exists("link/data.txt"));
        assert_eq!(tree.read("link/data.txt").as_deref(), Some("abc"));
    }

    #[test]
    fn test_file_tree_link_cycles_stop_at_the_hop_limit() {
        let mut tree = FileTree::new();
        tree.link("/a", "/b");
        tree.link("/b", "/a");
        let canonical = tree.canonical("/a");
        assert!(
            canonical == "/a" || canonical == "/b",
            "canonical {canonical}"
        );
    }
}
