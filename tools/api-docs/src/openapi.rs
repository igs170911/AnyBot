use utoipa::OpenApi;

pub fn document() -> Result<String, String> {
    let doc = anybot_server::ApiDoc::openapi();
    noyalib::to_string(&doc).map_err(|e| e.to_string())
}
