/// Unit tests that `main.rs` would otherwise compile into the `wt` binary a
/// second time. The binary declares the same modules, and expands this macro
/// to nothing, so a test wrapped in it runs once, in this target.
macro_rules! lib_target_tests {
    ($($item:item)*) => { $($item)* };
}

pub mod args;
pub mod commands;
pub mod env;
pub mod exit;
mod perf;
pub mod shell_integration;

pub use args::{Cli, Commands, PerfFormat};
