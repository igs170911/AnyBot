use anybot_protocol::LoginSession;

use crate::api_login_issue::SessionIssue;
use crate::caller::CallerDenied;
use crate::computers_join::{JoinSpec, join_account};

pub(crate) async fn open_computer(issue: &SessionIssue<'_>) -> Result<LoginSession, CallerDenied> {
    let joined = join_account(
        issue.state,
        issue.pool,
        &JoinSpec {
            account: issue.account,
            label: issue.label,
            paired: false,
        },
    )
    .await?;
    Ok(LoginSession {
        session_key: None,
        bearer_key: Some(joined.key.expose().to_owned()),
        account: issue.account.to_owned(),
        device_id: joined.device_id,
        used_recovery: None,
    })
}
