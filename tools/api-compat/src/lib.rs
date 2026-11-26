#![forbid(unsafe_code)]

extern crate clap as _;
extern crate serde as _;
extern crate serde_json as _;
extern crate sha2 as _;
extern crate yaml_serde as _;

pub mod compat;
#[cfg(test)]
mod compat_tests;
pub mod log;
pub mod schema;
pub mod sha;
