fn main() -> tauri::Result<()> {
    let app = tauri::Builder::default().build(anybot_app_desktop_context::make_context())?;
    app.run(|_app, _event| {});
    Ok(())
}
