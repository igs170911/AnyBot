extern crate self as runner_test;

pub mod fake_builder;
pub mod fake_computer;
#[path = "fake_computer_gate.rs"]
pub mod fake_computer_gate;
pub mod fake_files;
pub mod fake_helpers;
pub mod fake_scripts;
pub mod fake_trust;
pub mod mem_store;
pub(crate) mod ops_support;
pub mod ops_support_exports;
pub mod second_bot;
pub mod trust_world;
pub mod turn;
pub mod turn_helpers;
pub mod turn_loop1_checks;
pub mod turn_setup;
pub mod turn_tool_helpers;

pub use fake_builder::FakeComputerBuilder;
pub use fake_computer::FakeComputer;
pub use fake_files::FileTree;
pub use fake_helpers::{MISSING_FILE_REASON, NO_SCRIPT_REASON};
pub use fake_scripts::{CommandScript, ScriptTable};
pub mod test_view;

pub use test_view::TestView;
#[cfg(test)]
mod ops_tests;
#[cfg(test)]
mod pipeline_flow_tests;
#[cfg(test)]
mod trust_entry_tests;
