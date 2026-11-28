use std::time::Duration;

use serde_json::json;

use anybot_app_test::http::{
    FORWARDED_HEADER, LOGIN_STEP, PROXY_SECRET_HEADER, RequestSpec, post_driven,
};
use anybot_app_test::serve::{Served, boot_pair, serve_start};

pub(crate) const SECRET_VALUE: &str = "testproxysecret";

#[derive(Clone, Copy)]
enum Peer {
    Proxy,
    Other,
}

pub(crate) struct Via {
    peer: Peer,
    forwarded: Option<String>,
    secret: bool,
}

impl Via {
    pub(crate) fn proxy(forwarded: Option<&str>) -> Via {
        Via {
            peer: Peer::Proxy,
            forwarded: forwarded.map(str::to_owned),
            secret: false,
        }
    }

    pub(crate) fn other(forwarded: Option<&str>) -> Via {
        Via {
            peer: Peer::Other,
            forwarded: forwarded.map(str::to_owned),
            secret: false,
        }
    }

    pub(crate) fn with_secret(mut self) -> Via {
        self.secret = true;
        self
    }

    fn base(&self, served: &Served) -> String {
        match self.peer {
            Peer::Proxy => served.base_url(),
            Peer::Other => served.base_url_v6(),
        }
    }
}

pub(crate) async fn start(name: &str) -> Result<Served, String> {
    let served = serve_start(name).await?;
    boot_pair(&served).await?;
    Ok(served)
}

pub(crate) async fn trust_proxy(served: &mut Served, secret: Option<&str>) -> Result<(), String> {
    served
        .set_config_and_restart(anybot_server::KEY_TRUST_PROXY, "127.0.0.1")
        .await?;
    if let Some(value) = secret {
        served
            .set_config_and_restart(anybot_server::KEY_PROXY_SECRET, value)
            .await?;
    }
    Ok(())
}

async fn login(served: &Served, via: &Via, password: &str, key: &str) -> Result<u16, String> {
    let body = json!({"account": "bob", "password": password, "label": "test-device", "credential": "bearer"});
    let mut spec = RequestSpec::new(&body.to_string()).with_key(key);
    if let Some(value) = &via.forwarded {
        spec = spec.with_header(FORWARDED_HEADER, value);
    }
    if via.secret {
        spec = spec.with_header(PROXY_SECRET_HEADER, SECRET_VALUE);
    }
    let answer = post_driven(
        &served.clock,
        &via.base(served),
        "/v1/account/login",
        &spec,
        LOGIN_STEP,
        Duration::from_secs(15),
    )
    .await?;
    Ok(answer.status)
}

pub(crate) async fn fail_times(
    served: &Served,
    via: &Via,
    times: u32,
    tag: &str,
) -> Result<(), String> {
    for round in 1..=times {
        let status = login(served, via, "wrongpassword", &format!("{tag}-{round}")).await?;
        if status != 401 {
            return Err(format!(
                "{tag} attempt {round} answered {status}, expected 401"
            ));
        }
    }
    Ok(())
}

pub(crate) async fn right_password(served: &Served, via: &Via, key: &str) -> Result<u16, String> {
    login(served, via, "bobpassword", key).await
}
