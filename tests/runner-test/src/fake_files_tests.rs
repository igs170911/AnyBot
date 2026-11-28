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
fn test_file_tree_resolves_dir_links_dotdot_and_case_insensitive() {
    let mut tree = FileTree::new();
    tree.set_home_dir("/home/alice");
    tree.set_workdir("/home/alice/work");
    tree.set_case_insensitive(true);
    tree.write("/home/alice/.ssh/config", "Host x");
    tree.write("/home/alice/.aws/config", "[default]");
    tree.write("/home/alice/work/.ENV", "SECRET=1");
    tree.link("link", "~/.ssh");
    assert_eq!(tree.canonical("link/config"), "/home/alice/.ssh/config");
    assert_eq!(tree.canonical("../.aws/config"), "/home/alice/.aws/config");
    assert_eq!(tree.canonical(".env"), "/home/alice/work/.ENV");
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
