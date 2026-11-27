use axum::http::StatusCode;
use axum::response::Response;

use anybot_domain::two_step::{CHALLENGE_MAX_TRIES, challenge_open};
use anybot_messages::MessageCode;
use anybot_store_server::two_step::open_active_secret;
use anybot_store_server::two_step_challenges::{
    CHALLENGE_OPEN, CHALLENGE_PASSED, CHALLENGE_VOID, ChallengeRow, close_challenge,
    find_challenge, take_try,
};
use anybot_store_server::{Pool, SecretKeys};

use crate::api_login_issue::{SessionIssue, issue_session};
use crate::app::AppState;
use crate::caller::CallerDenied;
use crate::limits::Limiter;
use crate::problems;
use crate::req_write::{Outcome, denied, internal_sqlx, internal_text, record_outcome};
use crate::two_step_challenge::CREDENTIAL_COOKIE;
use crate::two_step_check::{
    AcceptedCode, LoginCode, login_code_passes, now_ms, opened, wrong_code_pause,
};

pub(crate) struct StepTwo<'a> {
    pub state: &'a AppState,
    pub pool: &'a Pool,
    pub keys: &'a SecretKeys,
    pub address: &'a str,
    pub origin: Option<&'a str>,
    pub native: bool,
    pub id_hash: &'a [u8],
    pub code: &'a str,
    pub key: &'a str,
    pub fingerprint: &'a [u8],
    pub scope: &'a str,
}

pub(crate) async fn run_step_two(step: &StepTwo<'_>) -> Result<Response, CallerDenied> {
    let now = now_ms(step.state)?;
    let found = find_challenge(step.pool, step.id_hash)
        .await
        .map_err(internal_sqlx)?;
    let Some(row) = found.filter(|row| is_open(row, now)) else {
        return failed(
            step,
            StatusCode::BAD_REQUEST,
            MessageCode::AccountTwoStepExpired,
        )
        .await;
    };
    refuse_native_cookie(step, &row)?;
    let limiter = Limiter::new(step.pool, std::sync::Arc::clone(&step.state.clock));
    let admitted = limiter
        .begin_two_step(step.address, &row.account)
        .await
        .map_err(internal_sqlx)?;
    let Some(attempt) = admitted else {
        close_challenge(step.pool, step.id_hash, CHALLENGE_VOID, now, false)
            .await
            .map_err(internal_sqlx)?;
        return failed(
            step,
            StatusCode::TOO_MANY_REQUESTS,
            MessageCode::AccountTwoStepRate,
        )
        .await;
    };
    let verdict = try_code(step, &row).await;
    if !matches!(verdict, Ok(Verdict::Wrong)) {
        limiter.cancel_login(attempt).await.map_err(internal_sqlx)?;
    }
    match verdict? {
        Verdict::Passed(kind) => passed(step, &row, now, kind).await,
        Verdict::Expired => {
            failed(
                step,
                StatusCode::BAD_REQUEST,
                MessageCode::AccountTwoStepExpired,
            )
            .await
        }
        Verdict::Wrong => {
            wrong_code_pause(step.state).await;
            failed(
                step,
                StatusCode::UNAUTHORIZED,
                MessageCode::AccountTwoStepCodeWrong,
            )
            .await
        }
    }
}

enum Verdict {
    Passed(AcceptedCode),
    Wrong,
    Expired,
}

fn is_open(row: &ChallengeRow, now: i64) -> bool {
    row.state == CHALLENGE_OPEN && challenge_open(row.created_at_ms, row.tries, now)
}

fn refuse_native_cookie(step: &StepTwo<'_>, row: &ChallengeRow) -> Result<(), CallerDenied> {
    if step.native && row.credential == CREDENTIAL_COOKIE {
        return Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::AccountLoginCredentials,
        ));
    }
    Ok(())
}

async fn try_code(step: &StepTwo<'_>, row: &ChallengeRow) -> Result<Verdict, CallerDenied> {
    let taken = take_try(step.pool, step.id_hash, CHALLENGE_MAX_TRIES)
        .await
        .map_err(internal_sqlx)?;
    if taken.is_none() {
        return Ok(Verdict::Expired);
    }
    let result = open_active_secret(step.pool, step.keys, &row.account).await;
    let Some(secret) = opened(&row.account, result)? else {
        return Ok(Verdict::Expired);
    };
    let check = LoginCode {
        state: step.state,
        pool: step.pool,
        keys: step.keys,
        account: &row.account,
        secret: &secret,
        code: step.code,
    };
    Ok(login_code_passes(&check)
        .await?
        .map_or(Verdict::Wrong, Verdict::Passed))
}

async fn passed(
    step: &StepTwo<'_>,
    row: &ChallengeRow,
    now: i64,
    kind: AcceptedCode,
) -> Result<Response, CallerDenied> {
    let used_recovery = kind == AcceptedCode::Recovery;
    let closed = close_challenge(
        step.pool,
        step.id_hash,
        CHALLENGE_PASSED,
        now,
        used_recovery,
    )
    .await
    .map_err(internal_sqlx)?;
    if !closed {
        return failed(
            step,
            StatusCode::BAD_REQUEST,
            MessageCode::AccountTwoStepExpired,
        )
        .await;
    }
    let row = ChallengeRow {
        used_recovery,
        ..row.clone()
    };
    issue_for(step, &row).await
}

pub(crate) async fn issue_for(
    step: &StepTwo<'_>,
    row: &ChallengeRow,
) -> Result<Response, CallerDenied> {
    issue_session(&SessionIssue {
        state: step.state,
        pool: step.pool,
        account: &row.account,
        label: &row.label,
        cookie_mode: row.credential == CREDENTIAL_COOKIE,
        used_recovery: row.used_recovery,
        origin: step.origin,
        key: step.key,
        raw: step.fingerprint,
        scope: step.scope,
    })
    .await
}

pub(crate) async fn failed(
    step: &StepTwo<'_>,
    status: StatusCode,
    code: MessageCode,
) -> Result<Response, CallerDenied> {
    let value = serde_json::to_value(problems::auth_problem(status.as_u16(), code))
        .map_err(internal_text)?;
    record_outcome(
        step.pool,
        step.state,
        &Outcome {
            key: step.key,
            raw: step.fingerprint,
            scope: step.scope,
            status,
            value: &value,
            marker: false,
        },
    )
    .await?;
    Err(denied(status, code))
}

pub(crate) async fn replay_passed(step: &StepTwo<'_>) -> Result<Response, CallerDenied> {
    let now = now_ms(step.state)?;
    let found = find_challenge(step.pool, step.id_hash)
        .await
        .map_err(internal_sqlx)?;
    let live = found
        .filter(|row| row.state == CHALLENGE_PASSED && challenge_open(row.created_at_ms, 0, now));
    let Some(row) = live else {
        return Err(denied(
            StatusCode::BAD_REQUEST,
            MessageCode::AccountTwoStepExpired,
        ));
    };
    refuse_native_cookie(step, &row)?;
    issue_for(step, &row).await
}
