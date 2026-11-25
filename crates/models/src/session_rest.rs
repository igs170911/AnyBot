use crate::capabilities::{self, Learned};
use crate::session::{REST_MS, RestMark, Session};
use crate::types::{ProviderEndpoint, ThinkingMode, TurnRequest, TurnResult, UsageReport};
use crate::{GatewayFail, ask};

pub(crate) fn learned_for<'a>(
    rows: &'a [Learned],
    endpoint: &ProviderEndpoint,
) -> Option<&'a Learned> {
    let key = capabilities::model_key(endpoint.kind, &endpoint.base_url, &endpoint.model);
    rows.iter().find(|row| row.key == key)
}

pub(crate) fn note_no_thinking(rows: &mut Vec<Learned>, endpoint: &ProviderEndpoint) {
    let key = capabilities::model_key(endpoint.kind, &endpoint.base_url, &endpoint.model);
    if let Some(row) = rows.iter_mut().find(|row| row.key == key) {
        row.thinking = Some(false);
        return;
    }
    let mut row = Learned::new(&key);
    row.thinking = Some(false);
    rows.push(row);
}

pub(crate) fn rest(rows: &mut Vec<RestMark>, endpoint: &ProviderEndpoint, now_ms: u64) {
    let key = capabilities::model_key(endpoint.kind, &endpoint.base_url, &endpoint.model);
    let until_ms = now_ms.saturating_add(REST_MS);
    if let Some(mark) = rows.iter_mut().find(|mark| mark.key == key) {
        mark.until_ms = until_ms;
        return;
    }
    rows.push(RestMark { key, until_ms });
}

pub(crate) fn note_usage(
    rows: &mut Vec<Learned>,
    endpoint: &ProviderEndpoint,
    estimated: u64,
    result: &TurnResult,
) {
    let UsageReport::Present(usage) = &result.usage else {
        return;
    };
    if estimated == 0 {
        return;
    }
    let key = capabilities::model_key(endpoint.kind, &endpoint.base_url, &endpoint.model);
    if !rows.iter().any(|row| row.key == key) {
        rows.push(Learned::new(&key));
    }
    if let Some(row) = rows.iter_mut().find(|row| row.key == key) {
        row.calibration.record(estimated, usage.input_total());
    }
}

pub(crate) fn note_too_long(rows: &mut Vec<Learned>, endpoint: &ProviderEndpoint, limit: u64) {
    let key = capabilities::model_key(endpoint.kind, &endpoint.base_url, &endpoint.model);
    if let Some(row) = rows.iter_mut().find(|row| row.key == key) {
        row.note_too_long(limit);
        return;
    }
    let mut row = Learned::new(&key);
    row.note_too_long(limit);
    rows.push(row);
}

pub(crate) async fn send_tries(
    session: &mut Session,
    model: &crate::session::ModelRef,
    request: &TurnRequest,
    stream: bool,
    audience: crate::types::Audience,
    calibration: &crate::estimate::Calibration,
    thinking: bool,
) -> Result<TurnResult, GatewayFail> {
    let mut sent = 0_u32;
    let mut current = request.clone();
    if !thinking {
        current.thinking = ThinkingMode::Auto;
    }
    loop {
        sent = sent.saturating_add(1);
        let outcome = session
            .gateway()
            .complete(&model.endpoint, &current, stream)
            .await;
        match crate::session_step::decide(outcome, sent) {
            crate::session_step::Step::Done(result) => {
                return Ok(done_try(session, model, request, calibration, result));
            }
            crate::session_step::Step::DropParam => {
                current.thinking = ThinkingMode::Auto;
                note_no_thinking(session.learned_rows(), &model.endpoint);
                sent = sent.saturating_sub(1);
            }
            crate::session_step::Step::BlankAgain => {
                sent = sent.saturating_sub(1);
            }
            crate::session_step::Step::Wait(mut fail) => {
                if !wait_try(session, model, &mut fail, sent, audience).await? {
                    return Err(ask::classified_fail(fail.classified, fail.notices));
                }
            }
            crate::session_step::Step::Stop(err) => return Err(err),
        }
    }
}

fn done_try(
    session: &mut Session,
    model: &crate::session::ModelRef,
    request: &TurnRequest,
    calibration: &crate::estimate::Calibration,
    result: TurnResult,
) -> TurnResult {
    let estimated = crate::session_step::estimate_request(request, calibration);
    note_usage(session.learned_rows(), &model.endpoint, estimated, &result);
    result
}

async fn wait_try(
    session: &mut Session,
    model: &crate::session::ModelRef,
    fail: &mut ask::AttemptFail,
    sent: u32,
    audience: crate::types::Audience,
) -> Result<bool, GatewayFail> {
    crate::session_switch::remember(session, &model.endpoint, &fail.classified);
    let again = session.wait_or_stop(fail, sent, audience).await?;
    if again {
        session
            .notices_mut()
            .extend(std::mem::take(&mut fail.notices));
    }
    Ok(again)
}

pub(crate) fn resting_now(rows: &[RestMark], endpoint: &ProviderEndpoint, now_ms: u64) -> bool {
    let key = capabilities::model_key(endpoint.kind, &endpoint.base_url, &endpoint.model);
    rows.iter()
        .any(|mark| mark.key == key && mark.until_ms > now_ms)
}
