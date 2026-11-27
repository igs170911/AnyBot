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
        crate::api_ops_session::logout,
        crate::api_ops_session::logout_all,
        crate::api_ops::change_password,
        crate::api_admin::invite,
        crate::api_admin::rotate_invite,
        crate::api_admin::set_public_url,
        crate::api_admin_accounts::disable_account,
        crate::api_admin_accounts::enable_account,
        crate::api_admin_reset::reset_password,
        crate::api_streams_append::append,
        crate::api_streams_after::after,
        crate::api_streams_record::record,
        crate::api_streams_action::action,
        crate::api_settings::settings,
        crate::api_referee::referee,
        crate::api_streams_sse::notifications,
    ),
    components(schemas(
        crate::health::HealthResponse,
        crate::status::StatusResponse,
        anybot_protocol::ActionAck,
        anybot_protocol::AfterPage,
                        anybot_protocol::AppendReply,
        anybot_protocol::AppendRequest,
        anybot_protocol::AppendResult,
        anybot_protocol::RecordItem,
        anybot_protocol::RecordAction,
        anybot_protocol::RecordActionReply,
        anybot_protocol::RecordActionRequest,
        anybot_protocol::SettingEntry,
        anybot_protocol::SettingsView,
        anybot_protocol::RefereeAction,
        anybot_protocol::RefereeReply,
        anybot_protocol::RefereeRequest,
                                        anybot_protocol::StreamSelector,
        anybot_protocol::StoredRecord,
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
        (name = "admin", description = "admin-only server operations"),
        (name = "streams", description = "record streams: append and replay after N")
    )
)]
pub struct ApiDoc;
