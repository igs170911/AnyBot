fn main() -> Result<(), String> {
    let attrs = tauri_build::Attributes::new().config_path("../src-tauri/tauri.conf.json");
    tauri_build::try_build(attrs).map_err(|e| e.to_string())
}
