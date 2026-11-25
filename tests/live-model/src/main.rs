use std::process::Termination;

use anybot_foundation as _;
use live_model::run;
use reqwest as _;
use rustls as _;
use rustls_pki_types as _;
use serde_json as _;
use test_harness as _;
use toml as _;

#[tokio::main]
async fn main() -> impl Termination {
    run().await
}
