use std::sync::Arc;

use anybot_channels::web_push::{
    PushClient, PushMessage, SendOutcome, Sender, Subscription, SystemResolver, VapidSigning,
};
use anybot_foundation::Clock;
use anybot_store_server::push_subscriptions::{self, StoredSubscription};
use anybot_store_server::{Pool, SecretKeys};

use crate::app::AppState;
use crate::computer_view::signed;
use crate::push_vapid::ensure_vapid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NotifyReport {
    pub delivered: usize,
    pub removed_gone: usize,
    pub skipped: usize,
    pub failed: usize,
}

#[derive(Clone)]
pub struct WebPushNotifier {
    pool: Pool,
    keys: Arc<SecretKeys>,
    clock: Arc<dyn Clock>,
    sender: Sender,
    contact: Option<String>,
}

impl WebPushNotifier {
    #[must_use]
    pub fn new(
        pool: Pool,
        keys: Arc<SecretKeys>,
        clock: Arc<dyn Clock>,
        sender: Sender,
        contact: Option<String>,
    ) -> WebPushNotifier {
        WebPushNotifier {
            pool,
            keys,
            clock,
            sender,
            contact,
        }
    }

    pub fn from_state(
        state: &AppState,
        contact: Option<String>,
    ) -> Result<WebPushNotifier, String> {
        let pool = state.pool().ok_or("no database pool")?;
        let keys = state.secrets.clone().ok_or("no secret keys")?;
        let client = PushClient::standard().map_err(|e| e.to_string())?;
        let sender = Sender::new(client, Arc::new(SystemResolver));
        Ok(WebPushNotifier::new(
            pool,
            keys,
            Arc::clone(&state.clock),
            sender,
            contact,
        ))
    }

    pub async fn notify_account(
        &self,
        account: &str,
        message: &PushMessage,
    ) -> Result<NotifyReport, String> {
        let targets = push_subscriptions::list_for_account(&self.pool, &self.keys, account)
            .await
            .map_err(|e| e.to_string())?;
        self.notify_targets(&targets, message).await
    }

    pub async fn notify_targets(
        &self,
        targets: &[StoredSubscription],
        message: &PushMessage,
    ) -> Result<NotifyReport, String> {
        let now = signed(self.clock.now_ms().millis()).map_err(|_| "clock out of range")?;
        let vapid = ensure_vapid(&self.pool, &self.keys, now).await?;
        let signing = VapidSigning::new(&vapid.key, self.contact.as_deref(), self.clock.as_ref());
        let mut report = NotifyReport::default();
        for target in targets {
            let subscription = Subscription {
                endpoint: target.endpoint.clone(),
                p256dh: target.p256dh.clone(),
                auth: anybot_foundation::Secret::from_untrusted(target.auth.expose().to_owned()),
            };
            match self.sender.send(&subscription, message, &signing).await {
                Ok(SendOutcome::Delivered) => report.delivered = report.delivered.saturating_add(1),
                Ok(SendOutcome::Skipped) => report.skipped = report.skipped.saturating_add(1),
                Ok(SendOutcome::Gone) => {
                    push_subscriptions::delete_gone(&self.pool, &target.id)
                        .await
                        .map_err(|e| e.to_string())?;
                    report.removed_gone = report.removed_gone.saturating_add(1);
                }
                Ok(SendOutcome::Rejected(_)) | Err(_) => {
                    report.failed = report.failed.saturating_add(1);
                }
            }
        }
        Ok(report)
    }
}
