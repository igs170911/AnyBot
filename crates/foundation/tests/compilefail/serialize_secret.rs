use anybot_foundation::Secret;

fn main() {
    let secret = Secret::from_untrusted("top-secret".to_owned());
    let json = serde_json::to_string(&secret).unwrap();
    println!("{json}");
}
