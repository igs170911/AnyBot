use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use anybot_protocol::{SettingEntry, SettingsView, StreamSelector, selector_key};
use anybot_store_server::streams_settings;

use crate::api_settings_write::settings_state;
use crate::app::AppState;
use crate::caller::Caller;
use crate::req_write::{CallerDenied, internal_sqlx, pool_of, value_of};

#[utoipa::path(
    get,
    path = "/v1/settings",
    operation_id = "get_settings",
    summary = "Current synced settings of the caller",
    description = "Replays the setting records of the account stream and returns the winner of every field. A list item is its own field. Device-only settings are never stored here.",
    tag = "account",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Winning value of every synced setting",
            content_type = "application/json",
            body = SettingsView,
            example = json!({"settings": [{"field": "accent_color", "value": "green"}, {"field": "bot_name", "item": "018f-bot-uuidv7", "value": "Helper"}]})),
        (status = 401, description = "Session rejected",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/internal"})),
    ),
)]
pub async fn settings(
    State(state): State<AppState>,
    caller: Caller,
) -> Result<Response, CallerDenied> {
    let pool = pool_of(&state)?;
    let key = selector_key(&StreamSelector::Account, &caller.account);
    let rows = streams_settings::setting_rows(pool, &key)
        .await
        .map_err(internal_sqlx)?;
    let winners = settings_state(&rows)?;
    let view = SettingsView {
        settings: winners
            .winners
            .values()
            .map(|edit| SettingEntry {
                field: edit.field.clone(),
                item: edit.list_item.clone(),
                value: edit.value.clone(),
            })
            .collect(),
    };
    let value = value_of(&view)?;
    Ok((StatusCode::OK, axum::Json(value)).into_response())
}
