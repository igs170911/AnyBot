fn main() {
    let path = anybot_trust::PathView::resolved("/Users/me/.zshrc", "/workspace/notes.md");
    println!("{}", path.raw_text());
}
