fn main() -> tauri::Result<()> {
    let app = tauri::Builder::default().build(tauri::generate_context!())?;
    app.run(|_app, _event| {});
    Ok(())
}
