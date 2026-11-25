use super::APP_IDENTIFIER;

const DESKTOP_CONFIG: &str = include_str!("../../../apps/desktop/src-tauri/tauri.conf.json");

fn config_field(json: &str, field: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    value.get(field)?.as_str().map(str::to_owned)
}

#[test]
fn app_identifier_matches_the_desktop_config() {
    let identifier = config_field(DESKTOP_CONFIG, "identifier").unwrap();
    assert_eq!(identifier, APP_IDENTIFIER);
}

#[test]
fn default_data_dir_ends_with_the_app_identifier() {
    let dir = default_data_dir_checked();
    assert_eq!(dir.file_name().unwrap(), APP_IDENTIFIER);
}

fn default_data_dir_checked() -> std::path::PathBuf {
    super::default_data_dir().unwrap()
}
