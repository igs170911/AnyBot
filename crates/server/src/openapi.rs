use std::collections::BTreeMap;

use utoipa::openapi::RefOr;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::openapi_examples::ExampleFiller;

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
        crate::api_task_status::task_status,
        crate::api_streams_action::action,
        crate::api_settings::settings,
        crate::api_referee::referee,
        crate::api_streams_sse::notifications,
        crate::api_streams_typing::typing,
        crate::api_two_step_login::login_two_step,
        crate::api_two_step_setup::setup_two_step,
        crate::api_two_step_toggle::confirm_two_step,
        crate::api_two_step_toggle::disable_two_step,
        crate::api_computers::list_computers,
        crate::api_computers::rename_computer,
        crate::api_computers_act::remove_computer,
        crate::api_computers_act::delegate_computer,
        crate::api_pairing::create_pairing_code,
        crate::api_pairing::reauth_password,
        crate::api_pairing_redeem::redeem_pairing,
        crate::api_mailbox::deliver_round,
        crate::api_mailbox::claim_letter,
        crate::api_push::server_key,
        crate::api_push::subscribe,
        crate::api_push::unsubscribe,
        crate::api_push::mine,
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
        anybot_protocol::WakeReason,
        anybot_protocol::TypingRequest,
                                        anybot_protocol::StreamSelector,
        anybot_protocol::StoredRecord,
        anybot_protocol::TaskReport,
        anybot_protocol::TaskStatusReply,
        anybot_protocol::TaskStatus,
        anybot_protocol::TaskDetail,
        anybot_protocol::InviteResponse,
        anybot_protocol::LoginRequest,
        anybot_protocol::LoginSession,
        anybot_protocol::PasswordChangeRequest,
        anybot_protocol::ProblemJson,
        anybot_protocol::PublicUrlRequest,
        anybot_protocol::RegisterAck,
        anybot_protocol::RegisterRequest,
        anybot_protocol::ResetPasswordRequest,
        anybot_protocol::TwoStepChallenge,
        anybot_protocol::TwoStepCodeRequest,
        anybot_protocol::TwoStepLoginRequest,
        anybot_protocol::TwoStepSetup,
        anybot_protocol::ComputerView,
        anybot_protocol::ComputersView,
        anybot_protocol::ComputerRenameRequest,
        anybot_protocol::ComputerRemoveRequest,
        anybot_protocol::ComputerDelegateRequest,
        anybot_protocol::PairingCodeRequest,
        anybot_protocol::PairingCodeReply,
        anybot_protocol::PairRedeemRequest,
        anybot_protocol::PasswordReenterRequest,
        anybot_protocol::RoundRequest,
        anybot_protocol::RoundReply,
        anybot_protocol::ClaimRequest,
        anybot_protocol::ClaimReply,
        anybot_protocol::PushServerKey,
        anybot_protocol::PushSubscriptionKeys,
        anybot_protocol::PushSubscribeRequest,
        anybot_protocol::PushSubscribeReply,
        anybot_protocol::PushMineReply,
    )),
    servers(
        (url = "http://127.0.0.1:8787", description = "loopback development server (M3 listens on loopback only)"),
        (url = "http://[::1]:8787", description = "loopback IPv6 development server"),
    ),
    security(("bearer" = [])),
    modifiers(&BearerAuth, &ExampleFiller),
    tags(
        (name = "system", description = "the server itself: health, status, stop"),
        (name = "account", description = "accounts, sessions and passwords"),
        (name = "admin", description = "admin-only server operations"),
        (name = "streams", description = "record streams: append and replay after N"),
        (name = "computers", description = "the computers of an account: list, rename, remove"),
        (name = "mailbox", description = "letters for a computer: hand a round over, claim a letter"),
        (name = "push", description = "Web Push: the server key and the push subscriptions of an account")
    )
)]
pub struct ApiDoc;
