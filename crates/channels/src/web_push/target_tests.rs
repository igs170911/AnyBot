use std::net::IpAddr;

use super::*;

struct FixedResolver(Vec<IpAddr>);

impl PushResolver for FixedResolver {
    fn resolve<'a>(&'a self, _host: &'a str, _port: u16) -> ResolveFuture<'a> {
        let found = self.0.clone();
        Box::pin(async move { Ok(found) })
    }
}

fn public() -> FixedResolver {
    FixedResolver(vec!["93.184.216.34".parse().unwrap()])
}

#[tokio::test]
async fn public_https_host_is_allowed() {
    let verdict = check_target("https://push.example.org/x", &public()).await;
    assert_eq!(verdict, TargetVerdict::Allowed);
}

#[tokio::test]
async fn http_localhost_local_and_private_targets_are_refused() {
    let private = FixedResolver(vec!["10.0.0.5".parse().unwrap()]);
    assert_eq!(
        check_target("http://push.example.org/x", &public()).await,
        TargetVerdict::NotHttps
    );
    assert_eq!(
        check_target("https://localhost/x", &public()).await,
        TargetVerdict::LocalName
    );
    assert_eq!(
        check_target("https://printer.local/x", &public()).await,
        TargetVerdict::LocalName
    );
    assert_eq!(
        check_target("https://push.example.org/x", &private).await,
        TargetVerdict::NotPublic
    );
    assert_eq!(
        check_target("https://127.0.0.1/x", &public()).await,
        TargetVerdict::NotPublic
    );
    assert_eq!(
        check_target("https://[::1]/x", &public()).await,
        TargetVerdict::NotPublic
    );
    let mixed = FixedResolver(vec![
        "93.184.216.34".parse().unwrap(),
        "192.168.1.9".parse().unwrap(),
    ]);
    assert_eq!(
        check_target("https://push.example.org/x", &mixed).await,
        TargetVerdict::NotPublic
    );
}

#[tokio::test]
async fn unresolvable_host_is_refused() {
    let none = FixedResolver(Vec::new());
    assert_eq!(
        check_target("https://push.example.org/x", &none).await,
        TargetVerdict::Unresolvable
    );
}

#[test]
fn non_public_ranges_are_classified() {
    for text in [
        "100.64.0.1",
        "169.254.1.1",
        "172.16.0.1",
        "0.1.2.3",
        "fc00::1",
        "fe80::1",
        "::ffff:10.0.0.1",
    ] {
        let ip: IpAddr = text.parse().unwrap();
        assert!(!is_public_ip(ip), "{text}");
    }
    for text in ["8.8.8.8", "93.184.216.34", "2606:4700::1111"] {
        let ip: IpAddr = text.parse().unwrap();
        assert!(is_public_ip(ip), "{text}");
    }
}

#[test]
fn endpoint_shape_follows_the_subscription_rules() {
    assert!(endpoint_shape_ok("https://push.example.org/abc"));
    assert!(!endpoint_shape_ok("http://push.example.org/abc"));
    let long = format!("https://push.example.org/{}", "a".repeat(ENDPOINT_MAX));
    assert!(!endpoint_shape_ok(&long));
    let exact = format!("https://push.example.org/{}", "a".repeat(ENDPOINT_MAX - 25));
    assert_eq!(exact.chars().count(), ENDPOINT_MAX);
    assert!(endpoint_shape_ok(&exact));
    assert!(!endpoint_shape_ok("not a url"));
}
