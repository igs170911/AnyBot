use std::collections::BTreeMap;

use utoipa::openapi::RefOr;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.security_schemes = BTreeMap::from([(
            "bearer".to_owned(),
            RefOr::T(SecurityScheme::Http(
                HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build(),
            )),
        )]);
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "AnyBot 伺服器 API",
        description = "AnyBot 新版伺服器的 HTTP API。跟著程式碼產生，文件即合約。",
        version = env!("CARGO_PKG_VERSION"),
        license(
            name = "MIT OR Apache-2.0",
            identifier = "MIT OR Apache-2.0"
        ),
        contact(name = "AnyBot 開發團隊"),
    ),
    paths(crate::health::health),
    components(schemas(crate::health::HealthResponse, anybot_protocol::ProblemJson)),
    servers(
        (url = "http://127.0.0.1:8787", description = "本機開發伺服器（M3 只聽本機）"),
        (url = "http://[::1]:8787", description = "本機 IPv6 開發伺服器"),
    ),
    security(("bearer" = [])),
    modifiers(&BearerAuth),
    tags(
        (name = "system", description = "伺服器本身：健康檢查與啟動狀態")
    )
)]
pub struct ApiDoc;
