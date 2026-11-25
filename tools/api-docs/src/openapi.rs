use utoipa::OpenApi;

pub fn document() -> Result<String, String> {
    let doc = anybot_server::ApiDoc::openapi();
    doc.to_yaml().map_err(|e| e.to_string())
}
