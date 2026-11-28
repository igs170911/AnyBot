use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;

use anybot_channels::web_push::endpoint_shape_ok;
use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_protocol::{
    ActionAck, ProblemJson, PushMineReply, PushServerKey, PushSubscribeReply, PushSubscribeRequest,
};
use anybot_store_server::push_subscriptions::{self, NewSubscription};

use crate::app::AppState;
use crate::caller::{Caller, CredentialKind};
use crate::computer_view::signed;
use crate::push_vapid::ensure_vapid;
use crate::req_write::{
    CallerDenied, denied, denied_detail, idempotency_key, idempotency_key_of, internal_sqlx,
    internal_text, json_response, pool_of, read_json, value_of,
};
use crate::two_step_check::secrets_of;

const P256DH_LEN: usize = 87;
const AUTH_LEN: usize = 22;

fn require_login(caller: &Caller) -> Result<(), CallerDenied> {
    match caller.credential {
        CredentialKind::Cookie | CredentialKind::Bearer => Ok(()),
        CredentialKind::Computer | CredentialKind::Runner => Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::DeviceNoPermission,
        )),
    }
}

fn device_of(caller: &Caller) -> &str {
    if caller.device_id.is_empty() {
        &caller.session_id
    } else {
        &caller.device_id
    }
}

#[utoipa::path(
    get,
    path = "/v1/push/server-key",
    operation_id = "get_push_server_key",
    summary = "Public key phones need to subscribe to push",
    description = "The VAPID public key of this server (base64url, uncompressed P-256). The key pair is made on first use and kept sealed in the database.",
    tag = "push",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Done",
            content_type = "application/json",
            body = PushServerKey,
            example = json!({"public_key": "BNcRdreALRFXTkOOUHK1EtK2wtaz5Ry4YfYCA_0QTpQtUbVlUls0VJXg7A8u-Ts1XbjhazAkj7I99e8QcYP7DkM"})),
        (status = 401, description = "Credential rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    ),
)]
pub async fn server_key(
    State(state): State<AppState>,
    caller: Caller,
) -> Result<Response, CallerDenied> {
    require_login(&caller)?;
    let pool = pool_of(&state)?;
    let keys = secrets_of(&state)?;
    let now = signed(state.clock.now_ms().millis())?;
    let vapid = ensure_vapid(pool, &keys, now)
        .await
        .map_err(internal_text)?;
    let reply = PushServerKey {
        public_key: vapid.public_key,
    };
    Ok(json_response(StatusCode::OK, value_of(&reply)?))
}

#[utoipa::path(
    post,
    path = "/v1/push/subscriptions",
    operation_id = "post_push_subscriptions",
    summary = "Subscribe this device to push",
    description = "The address must be https and at most 2048 characters. The same address replaces the older entry. An account keeps at most 16 subscriptions; the oldest one is dropped when a new one would be the 17th. Requires the Idempotency-Key header.",
    tag = "push",
    security(("bearer" = [])),
    params(("Idempotency-Key" = String, Header, description = "client-generated idempotency key")),
    request_body(content = PushSubscribeRequest, example = json!({"endpoint": "https://push.example.org/send/abc123", "keys": {"p256dh": "BLMbF9ffKBiWQLCKvTHb6LO8Nb6dcUh6TItC455vu2kElga6PQvUmaFyCdykxY2nOSSL3yKgfbmFLRTUaGv4yV8", "auth": "xS03Fi5ErfTNH_l9WHE9Ig"}})),
    responses(
        (status = 200, description = "Done",
            content_type = "application/json",
            body = PushSubscribeReply,
            example = json!({"id": "018f-push-uuidv7"})),
        (status = 400, description = "Address or keys invalid",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/push_endpoint_invalid", "title": "The push address must be https and at most 2048 characters", "status": 400, "code": "push-endpoint-invalid", "params": {}})),
        (status = 401, description = "Credential rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 403, description = "Only a login can subscribe",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/device_no_permission", "title": "This device has no permission to do this", "status": 403, "code": "device-no-permission", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    ),
)]
pub async fn subscribe(
    State(state): State<AppState>,
    caller: Caller,
    request: Request,
) -> Result<Response, CallerDenied> {
    require_login(&caller)?;
    let _key = idempotency_key(&request)?;
    let (body, _raw) = read_json::<PushSubscribeRequest>(request).await?;
    if !endpoint_shape_ok(&body.endpoint) {
        return Err(denied(
            StatusCode::BAD_REQUEST,
            MessageCode::PushEndpointInvalid,
        ));
    }
    if body.keys.p256dh.len() != P256DH_LEN || body.keys.auth.len() != AUTH_LEN {
        return Err(denied_detail(
            StatusCode::BAD_REQUEST,
            "push subscription keys have the wrong length",
        ));
    }
    let pool = pool_of(&state)?;
    let keys = secrets_of(&state)?;
    let id = uuid::Uuid::now_v7().to_string();
    let auth = Secret::from_untrusted(body.keys.auth);
    let new = NewSubscription {
        id: &id,
        account: &caller.account,
        device_id: device_of(&caller),
        endpoint: &body.endpoint,
        p256dh: &body.keys.p256dh,
        auth: &auth,
        created_at_ms: signed(state.clock.now_ms().millis())?,
    };
    push_subscriptions::upsert(pool, &keys, &new)
        .await
        .map_err(internal_text)?;
    Ok(json_response(
        StatusCode::OK,
        value_of(&PushSubscribeReply { id })?,
    ))
}

#[utoipa::path(
    delete,
    path = "/v1/push/subscriptions/{id}",
    operation_id = "delete_push_subscription",
    summary = "Remove one push subscription of the account",
    description = "Deletes one subscription that belongs to the signed-in account. Requires the Idempotency-Key header.",
    tag = "push",
    security(("bearer" = [])),
    params(
        ("id" = String, Path, description = "subscription id"),
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key"),
    ),
    responses(
        (status = 200, description = "Done",
            content_type = "application/json",
            body = ActionAck,
            example = json!({"ok": true})),
        (status = 401, description = "Credential rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 403, description = "Only a login can do this",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/device_no_permission", "title": "This device has no permission to do this", "status": 403, "code": "device-no-permission", "params": {}})),
        (status = 404, description = "No such subscription",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/push_subscription_not_found", "title": "There is no such push subscription for this account", "status": 404, "code": "push-subscription-not-found", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    ),
)]
pub async fn unsubscribe(
    State(state): State<AppState>,
    caller: Caller,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, CallerDenied> {
    require_login(&caller)?;
    let _key = idempotency_key_of(&headers)?;
    let pool = pool_of(&state)?;
    let removed = push_subscriptions::delete_owned(pool, &caller.account, &id)
        .await
        .map_err(internal_sqlx)?;
    if !removed {
        return Err(denied(
            StatusCode::NOT_FOUND,
            MessageCode::PushSubscriptionNotFound,
        ));
    }
    Ok(json_response(
        StatusCode::OK,
        value_of(&ActionAck { ok: true })?,
    ))
}

#[utoipa::path(
    get,
    path = "/v1/push/subscriptions/mine",
    operation_id = "get_push_subscription_mine",
    summary = "Whether this device has a push subscription",
    description = "Looks up the device of the signed-in session and says whether it has a push subscription.",
    tag = "push",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Done",
            content_type = "application/json",
            body = PushMineReply,
            example = json!({"subscribed": true})),
        (status = 401, description = "Credential rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 403, description = "Only a login can do this",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/device_no_permission", "title": "This device has no permission to do this", "status": 403, "code": "device-no-permission", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    ),
)]
pub async fn mine(State(state): State<AppState>, caller: Caller) -> Result<Response, CallerDenied> {
    require_login(&caller)?;
    let pool = pool_of(&state)?;
    let subscribed =
        push_subscriptions::exists_for_device(pool, &caller.account, device_of(&caller))
            .await
            .map_err(internal_sqlx)?;
    Ok(json_response(
        StatusCode::OK,
        value_of(&PushMineReply { subscribed })?,
    ))
}
