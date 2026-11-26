#![forbid(unsafe_code)]

extern crate clap as _;
extern crate serde_json as _;
extern crate sha2 as _;
extern crate yaml_serde as _;

pub mod compat;
#[cfg(test)]
mod compat_tests;
pub mod dev_helpers;
pub mod git_ops;
pub mod log;
#[cfg(test)]
mod release_tests;
pub mod schema;
pub mod sha;
