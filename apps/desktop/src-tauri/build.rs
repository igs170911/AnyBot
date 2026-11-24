//! Tauri 建置腳本：產生 tauri-build 需要的設定（照 Tauri 2 的標準做法）。

#![forbid(unsafe_code)]

fn main() {
    tauri_build::build();
}
