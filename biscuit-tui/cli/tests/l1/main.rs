//! Level 1 integration tests for `biscuit-tui-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout` rejects one.

#[path = "../common/mod.rs"]
mod common;

mod boolean_switch_output;
mod choose_cli;
mod choose_many_output;
mod choose_one_output;
mod completions;
mod completions_shell;
mod exit_codes;
mod help_contract;
mod input_table_output;
// The file nests `mod keyboard_protocol { … }`; renaming it would change the
// `keyboard_protocol::keyboard_protocol::…` test paths the move must preserve.
#[allow(clippy::module_inception)]
mod keyboard_protocol;
mod test_layout;
mod text_area_input_output;
mod text_input_output;
