//! apps/desktop · `AnyBot` 桌面殼（`Tauri` 2）骨架。
//! 照 `spec/06` §2.4、`spec/07` §2/§3：**視窗只是畫面**，做事的是背景程式 `apps/agent`；
//! 這個殼不准依賴 server、store-server、runner、models、tools、computers、mcp、external-agents。
//! M2 骨架只開一個視窗載入 `web/ui` 的外殼，沒有任何指令、沒有任何功能。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

fn main() -> tauri::Result<()> {
    let app = tauri::Builder::default().build(tauri::generate_context!())?;
    app.run(|_app, _event| {});
    Ok(())
}
