use std::collections::{BTreeMap, BTreeSet};

use anybot_foundation::ContentDigest;

pub const WORKDIR: &str = "/workspace";

const MAX_LINK_HOPS: usize = 8;

#[derive(Debug, Default, Clone)]
pub struct FileTree {
    files: BTreeMap<String, String>,
    links: BTreeMap<String, String>,
    dirs: BTreeSet<String>,
}

impl FileTree {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn canonical(&self, path: &str) -> String {
        let joined = Self::join_workdir(path);
        self.follow_links(&joined)
    }

    fn join_workdir(path: &str) -> String {
        if path.starts_with('/') {
            return path.to_owned();
        }
        format!("{WORKDIR}/{path}")
    }

    fn follow_links(&self, joined: &str) -> String {
        let mut current = joined.to_owned();
        for _ in 0..MAX_LINK_HOPS {
            let Some(target) = self.links.get(&current) else {
                return current;
            };
            current = Self::join_workdir(target);
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
        self.files.insert(canonical, content.to_owned());
    }

    pub fn write_exact(&mut self, path: &str, content: &str) {
        self.files.insert(path.to_owned(), content.to_owned());
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
        self.links.insert(canonical, target.to_owned());
    }

    pub fn add_dir(&mut self, path: &str) {
        let canonical = self.canonical(path);
        self.dirs.insert(canonical);
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
