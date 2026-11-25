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
    paths(
        crate::health::health,
        crate::status::status,
        
        crate::api_register::register,
        crate::api_account::login,
        crate::api_ops::logout,
        crate::api_ops::logout_all,
        crate::api_ops::change_password,
        crate::api_admin::invite,
        crate::api_admin::rotate_invite,
        crate::api_admin::set_public_url,
        crate::api_admin_accounts::disable_account,
        crate::api_admin_accounts::enable_account,
        crate::api_admin_accounts::reset_password,
    ),
    components(schemas(
        crate::health::HealthResponse,
        crate::status::StatusResponse,
        
        anybot_protocol::ActionAck,
        anybot_protocol::InviteResponse,
        anybot_protocol::LoginRequest,
        anybot_protocol::LoginSession,
        anybot_protocol::PasswordChangeRequest,
        anybot_protocol::ProblemJson,
        anybot_protocol::PublicUrlRequest,
        anybot_protocol::RegisterAck,
        anybot_protocol::RegisterRequest,
        anybot_protocol::ResetPasswordRequest,
    )),
    servers(
        (url = "http://127.0.0.1:8787", description = "loopback development server (M3 listens on loopback only)"),
        (url = "http://[::1]:8787", description = "loopback IPv6 development server"),
    ),
    security(("bearer" = [])),
    modifiers(&BearerAuth),
    tags(
        (name = "system", description = "the server itself: health, status, stop"),
        (name = "account", description = "accounts, sessions and passwords"),
        (name = "admin", description = "admin-only server operations")
    )
)]
pub struct ApiDoc;
