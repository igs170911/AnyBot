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
        title = "AnyBot Server API",
        description = "HTTP API of the new AnyBot server. Generated from the code; the document is the contract.",
        version = env!("CARGO_PKG_VERSION"),
        license(
            name = "MIT OR Apache-2.0",
            identifier = "MIT OR Apache-2.0"
        ),
        contact(name = "AnyBot dev team"),
    ),
    paths(crate::health::health),
    components(schemas(crate::health::HealthResponse, anybot_protocol::ProblemJson)),
    servers(
        (url = "http://127.0.0.1:8787", description = "loopback development server (M3 listens on loopback only)"),
        (url = "http://[::1]:8787", description = "loopback IPv6 development server"),
    ),
    security(("bearer" = [])),
    modifiers(&BearerAuth),
    tags(
        (name = "system", description = "the server itself: health check and startup state")
    )
)]
pub struct ApiDoc;
