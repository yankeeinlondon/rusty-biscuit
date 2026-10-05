//! Acceptance Criteria 2, 3, and 5 of `2026-09-30-file-refs-use-magic` for
//! the `claudine` binary: it reads its process once, at dispatch
//! (`request.rs`), and every invocation context, completion scope, and hook
//! dispatch resolves through contexts built from that snapshot. The process
//! reads that remain decide rendering, test hooks, timeouts, provider
//! profiles, or what a child inherits, or locate Claudine's own
//! configuration; none feeds file resolution, `ctx.*`, or composition
//! `env.*`. The gates are documented in
//! `darkmatter/cli/tests/common/context_guard.rs`.

#[path = "../../../../darkmatter/cli/tests/common/context_guard.rs"]
mod context_guard;

use context_guard::{Allowance, Gate};

/// One ambient-state allowance.
const fn ambient(path: &'static str, identifier: &'static str, count: usize, reason: &'static str) -> Allowance {
    Allowance { gate: Gate::AmbientState, path, identifier, count, reason }
}

const FORCE_COLOR: &str = "`FORCE_COLOR` decides whether an error block shows its frontmatter excerpt (rendering)";
const COMMAND_REPOSITORY: &str =
    "the repository a non-composition command reports on or configures; resolves no file reference";

const ALLOWLIST: &[Allowance] = &[
    Allowance {
        gate: Gate::Construction,
        path: "request.rs",
        identifier: "RequestSnapshot::from_process",
        count: 1,
        reason: "the `claudine` binary's one process read (R8)",
    },
    ambient("argv/mod.rs", "std::env::var_os", 1, "`COMPLETE` selects clap's dynamic completion mode"),
    ambient(
        "cli_utils.rs",
        "std::env::current_dir",
        1,
        "absolute hyperlink target for a relative path (rendering)",
    ),
    ambient("commands/agents.rs", "std::env::current_dir", 1, COMMAND_REPOSITORY),
    ambient(
        "commands/compose/interrupt.rs",
        "std::env::current_dir",
        1,
        "hyperlink target in the interrupt notice (rendering)",
    ),
    ambient("commands/compose/mod.rs", "std::env::var_os", 1, FORCE_COLOR),
    ambient("commands/compose/prep.rs", "std::env::var_os", 4, FORCE_COLOR),
    ambient("commands/config_tui/mod.rs", "std::env::current_dir", 1, COMMAND_REPOSITORY),
    ambient(
        "commands/exec_prep/mod.rs",
        "std::env::var",
        1,
        "a provider's model variables select the model a non-interactive launch needs",
    ),
    ambient(
        "commands/handle.rs",
        "std::env::current_dir",
        1,
        "a hook's event metadata describes the provider's directory, not the wrapper's launch directory",
    ),
    ambient(
        "commands/handle.rs",
        "std::env::var",
        7,
        "hook deadline, wrapper flags, session id, rendezvous switch, and provider name the wrapper exports",
    ),
    ambient("commands/hooks/variables.rs", "std::env::current_dir", 1, COMMAND_REPOSITORY),
    ambient("commands/mcp/mod.rs", "std::env::current_dir", 1, COMMAND_REPOSITORY),
    ambient("commands/providers.rs", "std::env::var_os", 1, "`NO_COLOR` (rendering)"),
    ambient("commands/sequence.rs", "std::env::var_os", 1, FORCE_COLOR),
    ambient("commands/skills.rs", "std::env::current_dir", 1, COMMAND_REPOSITORY),
    ambient("commands/slash_commands.rs", "std::env::current_dir", 1, COMMAND_REPOSITORY),
    ambient(
        "commands/steer/service.rs",
        "biscuit_file::home_dir",
        1,
        "locates Claude Code's session registry for steering",
    ),
    ambient(
        "commands/steer/service.rs",
        "std::env::var",
        1,
        "`CLAUDE_CONFIG_DIR` locates Claude Code's session registry for steering",
    ),
    ambient("commands/uninstall.rs", "biscuit_file::home_dir", 1, "locates provider configuration to unregister hooks from"),
    ambient(
        "commands/wrap/composition/dry_run.rs",
        "std::env::current_dir",
        1,
        "shortens a path in the dry-run report (rendering)",
    ),
    ambient("commands/wrap/composition/staged_boot.rs", "std::env::var_os", 1, FORCE_COLOR),
    ambient(
        "commands/wrap/composition/timeouts.rs",
        "biscuit_file::home_dir",
        1,
        "shortens a path in a timeout notice (rendering)",
    ),
    ambient(
        "commands/wrap/composition/timeouts.rs",
        "std::env::current_dir",
        1,
        "shortens a path in a timeout notice (rendering)",
    ),
    ambient(
        "commands/wrap/composition/timeouts.rs",
        "std::env::var",
        2,
        "timeout and stall-timeout variables (`CLAUDINE_*_TIMEOUT`)",
    ),
    ambient(
        "commands/wrap/env/sanitize.rs",
        "std::env::vars_os",
        1,
        "the environment a wrapped provider inherits, sanitized for the child",
    ),
    ambient(
        "commands/wrap/exec/mod.rs",
        "std::env::current_dir",
        1,
        "compares the process directory before switching it to the child's",
    ),
    ambient(
        "commands/wrap/exec/mod.rs",
        "std::env::var_os",
        1,
        "`CLAUDINE_TEST_TEARDOWN_HOLD` test hook",
    ),
    ambient(
        "commands/wrap/exec/stream_capture.rs",
        "biscuit_file::home_dir",
        1,
        "expands `~` in `CLAUDINE_RAW_STREAM_DIR` (diagnostic capture)",
    ),
    ambient(
        "commands/wrap/exec/stream_capture.rs",
        "std::env::var",
        1,
        "`CLAUDINE_RAW_STREAM_DIR` enables raw stream capture (diagnostics)",
    ),
    ambient("commands/wrap/exec/timeouts.rs", "std::env::var", 1, "timeout variables (`CLAUDINE_*_TIMEOUT`)"),
    ambient(
        "commands/wrap/harness_orch/launch.rs",
        "std::env::var",
        2,
        "`CLAUDINE_OPENCODE_STALL_TIMEOUT`",
    ),
    ambient("commands/wrap/harness_orch/loop_control.rs", "std::env::var_os", 1, FORCE_COLOR),
    ambient(
        "commands/wrap/harness_orch/loop_control/requeue.rs",
        "biscuit_file::home_dir",
        1,
        "locates Claudine's requeue fallback directory",
    ),
    ambient(
        "commands/wrap/harness_orch/loop_control/requeue.rs",
        "std::env::var_os",
        1,
        "overrides Claudine's requeue fallback directory",
    ),
    ambient(
        "commands/wrap/live_semantic_sink/mod.rs",
        "biscuit_file::home_dir",
        2,
        "shortens paths in streamed event rendering",
    ),
    ambient("commands/wrap/profile/gemini.rs", "biscuit_file::home_dir", 1, "locates Gemini CLI's configuration for its overlay"),
    ambient("commands/wrap/profile/opencode.rs", "biscuit_file::home_dir", 1, "locates OpenCode's configuration for its overlay"),
    ambient("commands/wrap/profile/opencode.rs", "std::env::var_os", 1, "`XDG_CONFIG_HOME` locates OpenCode's configuration"),
    ambient("commands/wrap/runaway_guard.rs", "std::env::var_os", 1, "the automatic-steering switch"),
    ambient("commands/wrap/session_report.rs", "std::env::var", 1, "the session-report switch"),
    ambient("log.rs", "std::env::var", 2, "`TERM_WIDTH` and terminal sizing (rendering)"),
    ambient("log.rs", "std::env::var_os", 2, "`NO_COLOR` and color forcing (rendering)"),
    ambient(
        "main.rs",
        "std::env::var_os",
        1,
        "`CLAUDINE_TEST_DIAGNOSTIC_SNAPSHOT` (`test-fixtures` only)",
    ),
    ambient("output/mod.rs", "std::env::var", 1, "`CLAUDINE_SYSTEM_PROMPT` sets system-prompt report verbosity"),
    ambient(
        "telemetry.rs",
        "std::env::current_dir",
        2,
        "repository named on tracing spans and source locations (telemetry)",
    ),
    ambient("telemetry.rs", "std::env::var", 1, "`RUST_LOG` (telemetry)"),
];

#[test]
fn production_source_builds_contexts_only_through_the_builder() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../../darkmatter/cli/tests/common/context_guard.rs");
    let _ = include_str!("../../../../darkmatter/cli/tests/common/source_scan.rs");
    context_guard::assert_guarded(
        "claudine-cli",
        &biscuit_test_harness::manifest_dir!().join("src"),
        ALLOWLIST,
    );
}
