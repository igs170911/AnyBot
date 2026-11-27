use super::*;

fn direct(raw: &str) -> bool {
    goes_direct(&Url::parse(raw).unwrap())
}

#[test]
fn local_and_lan_places_connect_directly() {
    for raw in [
        "http://127.0.0.1:8080/x",
        "http://[::1]:8080/x",
        "http://localhost/x",
        "http://192.168.1.4/x",
        "http://10.0.0.7/x",
        "http://172.16.3.3/x",
        "http://169.254.1.1/x",
        "http://100.64.0.9/x",
        "http://[fd00::1]/x",
        "http://[fe80::1]/x",
        "http://[::ffff:127.0.0.1]/x",
        "http://nas.local/x",
        "http://printer.lan/x",
        "http://box/x",
        "https://host.ts.net/x",
    ] {
        assert!(direct(raw), "{raw} must be direct");
    }
}

#[test]
fn cloud_places_use_the_proxy() {
    for raw in [
        "https://example.com/x",
        "http://203.0.113.5/x",
        "http://8.8.8.8/x",
        "http://[2001:db8::1]/x",
        "https://example.com./x",
        "http://127.0.0.1.example.com/x",
        "http://100.200.0.1/x",
    ] {
        assert!(!direct(raw), "{raw} must use the proxy");
    }
}
