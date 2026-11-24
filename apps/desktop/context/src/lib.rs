#![forbid(unsafe_code)]

#[must_use]
pub fn make_context() -> tauri::Context {
    tauri::generate_context!("../src-tauri/tauri.conf.json")
}
