//! Parent-side helper tools — `git`, `rustc`, `md` — with the inherited
//! environment the fixture owns removed.
//!
//! `CliProcessFixture` isolates the `claudine` child. A test that shells out to
//! `git` *itself*, to build the repository the child will then discover, runs
//! from the test process with its whole environment intact — so the isolation
//! stops at a door the fixture never closed.
//!
//! Kept in its own file, rather than in `common/mod.rs`, so a binary can take
//! it with `#[path = "common/host_tools.rs"] mod host_tools;` without also
//! compiling the fixture surface (`common::wrap`'s `claudine::mcp::types`,
//! `common::pty`'s expectrl). `spawn_site_guard.rs` includes
//! `common/source_scan.rs` the same way and for the same reason.

#![allow(dead_code)]

use std::process::Command;

/// The `GIT_*` plumbing variables that override cwd-based repository discovery.
///
/// An inherited pair defeats `current_dir` entirely, in the fixture as well as
/// in the child: on 2026-08-31 a pre-push hook run of this suite inherited
/// `GIT_DIR` and drove fixture `git` commands into the real repository,
/// committing fixture files onto a feature branch.
pub const GIT_PLUMBING_VARS: [&str; 5] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
];

/// A parent-side helper tool with [`GIT_PLUMBING_VARS`] removed.
///
/// Nothing else is scrubbed. A helper tool is not the subject of any assertion
/// — it builds the fixture's inputs — and it needs the host `PATH` and `HOME`
/// to run at all, which is the opposite of what the `claudine` child needs.
pub fn helper_command(program: &str) -> Command {
    test_toolkit::init_test_tracing();
    let mut command = Command::new(program);
    for key in GIT_PLUMBING_VARS {
        command.env_remove(key);
    }
    command
}
