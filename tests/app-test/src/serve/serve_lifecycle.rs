use std::sync::Arc;

use anybot_foundation::{Clock, SystemClock};
use test_harness::FakeClock;

use crate::n106;
use crate::serve::{Served, ServedReal, bind_fake};

impl Served {
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.db.listen_port)
    }

    #[must_use]
    pub fn base_url_v6(&self) -> String {
        format!("http://[::1]:{}", self.v6_port)
    }

    #[must_use]
    pub fn origin_addr(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, self.db.listen_port))
    }

    pub async fn stop_server(&mut self) -> Result<(), String> {
        self.server_shutdown.notify_waiters();
        let running = std::mem::take(&mut self.server_tasks);
        if running.is_empty() {
            return Err("server is not running".to_owned());
        }
        let deadline = std::time::Instant::now()
            .checked_add(std::time::Duration::from_secs(10))
            .ok_or("deadline overflow".to_owned())?;
        for (index, task) in running.into_iter().enumerate() {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            let waited = tokio::time::timeout(left, task)
                .await
                .map_err(|_| format!("server task {index} did not finish"))?;
            waited.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub async fn kill_server(&mut self) -> Result<(), String> {
        let running = std::mem::take(&mut self.server_tasks);
        if running.is_empty() {
            return Err("server is not running".to_owned());
        }
        for task in &running {
            task.abort();
        }
        for task in running {
            match task.await {
                Ok(()) => {}
                Err(e) if e.is_cancelled() => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    }

    pub async fn restart_server(&mut self) -> Result<(), String> {
        let config = anybot_server::load_config(&self.pool, &*self.db.secret_keys()?)
            .await
            .map_err(|e| format!("{e:?}"))?;
        let clock_fast: Arc<FakeClock> = Arc::clone(&self.clock);
        let clock_arc: Arc<dyn Clock> = clock_fast;
        let (shutdown, task, v6_port, fresh) = bind_fake(
            &self.db,
            clock_arc,
            self.pool.clone(),
            config,
            &Arc::clone(&self.state.releases),
        )
        .await?;
        let _old = std::mem::replace(&mut self.state, fresh);
        let _old = std::mem::replace(&mut self.server_shutdown, shutdown);
        self.server_tasks = task;
        self.v6_port = v6_port;
        Ok(())
    }

    pub async fn set_config_and_restart(&mut self, key: &str, value: &str) -> Result<(), String> {
        anybot_server::write_config_row(&self.pool, key, value)
            .await
            .map_err(|e| e.to_string())?;
        self.stop_server().await?;
        self.restart_server().await
    }

    pub async fn stop(mut self) -> Result<(), String> {
        self.stop_server().await?;
        self.db.stop().await
    }
}

impl ServedReal {
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.db.listen_port)
    }

    #[must_use]
    pub fn base_url_v6(&self) -> String {
        format!("http://[::1]:{}", self.v6_port)
    }

    pub async fn stop_server(&mut self) -> Result<(), String> {
        self.server_shutdown.notify_waiters();
        let running = std::mem::take(&mut self.server_tasks);
        if running.is_empty() {
            return Err("server is not running".to_owned());
        }
        let deadline = std::time::Instant::now()
            .checked_add(std::time::Duration::from_secs(10))
            .ok_or("deadline overflow".to_owned())?;
        for (index, task) in running.into_iter().enumerate() {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            let waited = tokio::time::timeout(left, task)
                .await
                .map_err(|_| format!("server task {index} did not finish"))?;
            waited.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub async fn restart_server(&mut self) -> Result<(), String> {
        let config = anybot_server::load_config(&self.pool, &*self.db.secret_keys()?)
            .await
            .map_err(|e| format!("{e:?}"))?;
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let (shutdown, task, v6_port, fresh) = bind_fake(
            &self.db,
            clock,
            self.pool.clone(),
            config,
            &Arc::clone(&self.state.releases),
        )
        .await?;
        let _old = std::mem::replace(&mut self.state, fresh);
        let _old = std::mem::replace(&mut self.server_shutdown, shutdown);
        self.server_tasks = task;
        self.v6_port = v6_port;
        Ok(())
    }

    pub async fn set_config_and_restart(&mut self, key: &str, value: &str) -> Result<(), String> {
        anybot_server::write_config_row(&self.pool, key, value)
            .await
            .map_err(|e| e.to_string())?;
        self.stop_server().await?;
        self.restart_server().await
    }

    pub async fn stop(mut self) -> Result<(), String> {
        self.stop_server().await?;
        self.db.stop().await
    }
}

pub async fn boot_pair(served: &Served) -> Result<String, String> {
    let invite = n106::invite_of(&served.started)?;
    n106::register(
        &served.base_url(),
        "alice",
        "alicepassword",
        &invite,
        "k-admin",
    )
    .await?;
    n106::register(&served.base_url(), "bob", "bobpassword", &invite, "k-bob").await?;
    Ok(invite)
}
