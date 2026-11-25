use libc as _;

pub mod http;
pub mod launch;
pub mod process;
pub mod running;
pub mod scratch;

pub use http::{http_get, http_get_addr, http_wait};
pub use launch::{Outcome, Program, child_is_dead, run_program, run_program_with};
pub use running::Running;
pub use scratch::{test_dir, write_file_with_mode};
