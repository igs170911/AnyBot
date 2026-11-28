use super::*;

fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

#[test]
fn public_peer_stops_at_the_limit_and_frees_on_drop() {
    let limiter = ConnLimiter::new();
    let public = ip("1.2.3.4");
    let mut held: Vec<ConnSlot> = (0..PUBLIC_PEER_CONNECTIONS)
        .map(|_| limiter.try_open(public).unwrap())
        .collect();
    assert!(limiter.try_open(public).is_none());
    assert!(limiter.try_open(ip("5.6.7.8")).is_some());
    held.pop();
    assert_eq!(limiter.open_count(public), PUBLIC_PEER_CONNECTIONS - 1);
    assert!(limiter.try_open(public).is_some());
    held.clear();
    assert_eq!(limiter.open_count(public), 0);
}

#[test]
fn mapped_v6_counts_as_the_same_public_peer() {
    let limiter = ConnLimiter::new();
    let _held: Vec<ConnSlot> = (0..PUBLIC_PEER_CONNECTIONS)
        .map(|_| limiter.try_open(ip("1.2.3.4")).unwrap())
        .collect();
    assert!(limiter.try_open(ip("::ffff:1.2.3.4")).is_none());
}

#[test]
fn unlimited_peers_are_never_counted() {
    let limiter = ConnLimiter::new();
    let _held: Vec<ConnSlot> = (0..200)
        .map(|_| limiter.try_open(ip("192.168.1.20")).unwrap())
        .collect();
    assert_eq!(limiter.open_count(ip("192.168.1.20")), 0);
}
