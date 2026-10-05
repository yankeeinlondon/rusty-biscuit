//! Acceptance Criteria 2, 3, and 5 of `2026-09-30-file-refs-use-magic` for
//! the `claudine` library: it never builds a file-resolution context itself
//! and never reads the process. Every context comes from Darkmatter's builder
//! over the request snapshot the `claudine` binary captured, or is derived
//! from one. The process reads that remain decide where Claudine's own
//! configuration, logs, and provider files live, or what a child process
//! inherits; none feeds file resolution, `ctx.*`, or composition `env.*`. The
//! gates are documented in `darkmatter/cli/tests/common/context_guard.rs`.

#[path = "../../../../darkmatter/cli/tests/common/context_guard.rs"]
mod context_guard;

use context_guard::{Allowance, Gate};

/// One ambient-state allowance.
const fn ambient(path: &'static str, identifier: &'static str, count: usize, reason: &'static str) -> Allowance {
    Allowance { gate: Gate::AmbientState, path, identifier, count, reason }
}

const HOME_CONFIG: &str = "locates a provider's or Claudine's own configuration under the user's home";

const ALLOWLIST: &[Allowance] = &[
    ambient(
        "child_environment.rs",
        "std::env::current_dir",
        1,
        "process-entry launch directory exported to children as `AGENT_CWD`",
    ),
    ambient(
        "child_environment.rs",
        "std::env::var_os",
        1,
        "the `AGENT_CWD` a wrapper hands a provider hook, exported to children",
    ),
    ambient(
        "composition/error/render/mod.rs",
        "std::env::current_dir",
        1,
        "absolute hyperlink target for a relative path in an error block (rendering)",
    ),
    ambient(
        "composition/looping/config.rs",
        "std::env::var",
        4,
        "`CLAUDINE_FAIL_FAST`, `FAIL_FAST`, `CLAUDINE_MAX_ITERATIONS`, `CLAUDINE_PAUSE_RESET_MARGIN` loop settings",
    ),
    ambient(
        "composition/select.rs",
        "std::env::var",
        6,
        "provider and model selection variables (`MODEL` and provider model overrides)",
    ),
    ambient("config/antigravity.rs", "biscuit_file::home_dir", 1, HOME_CONFIG),
    ambient("config/backup.rs", "biscuit_file::home_dir", 1, "locates Claudine's configuration backups under the user's home"),
    ambient("config/claude.rs", "biscuit_file::home_dir", 1, HOME_CONFIG),
    ambient("config/codex.rs", "biscuit_file::home_dir", 2, HOME_CONFIG),
    ambient("config/gemini.rs", "biscuit_file::home_dir", 1, HOME_CONFIG),
    ambient("config/goose.rs", "biscuit_file::home_dir", 1, HOME_CONFIG),
    ambient("config/kilo.rs", "biscuit_file::home_dir", 2, HOME_CONFIG),
    ambient("config/kimicode.rs", "biscuit_file::home_dir", 1, HOME_CONFIG),
    ambient("config/mod.rs", "biscuit_file::home_dir", 1, HOME_CONFIG),
    ambient("config/opencode.rs", "biscuit_file::home_dir", 2, HOME_CONFIG),
    ambient("config/pi.rs", "biscuit_file::home_dir", 1, HOME_CONFIG),
    ambient("config/qwen.rs", "biscuit_file::home_dir", 1, HOME_CONFIG),
    ambient(
        "dispatch/expression.rs",
        "std::env::var",
        1,
        "hook-template `env.NAME` reads the hook process's environment when the event fires, not a composition request's",
    ),
    ambient("dispatch/loader.rs", "biscuit_file::home_dir", 1, "locates the user's Claudine hook configuration"),
    ambient(
        "dispatch/logging.rs",
        "std::env::var",
        1,
        "`CLAUDINE_SESSION_ID` the wrapper exports, recorded on hook event logs",
    ),
    ambient(
        "dispatch/wrapper_flags.rs",
        "std::env::var",
        2,
        "the wrapper's interactive flag, reported in hook event metadata",
    ),
    ambient(
        "events/environment.rs",
        "std::env::var",
        1,
        "package and process identifiers the wrapper exports, reported in event metadata",
    ),
    ambient(
        "invocation_context.rs",
        "std::env::var_os",
        1,
        "raw home variables a child process must reproduce byte for byte; `~` reads the snapshot",
    ),
    ambient(
        "invocation_context.rs",
        "std::env::vars",
        1,
        "lazy `current.agent`/`current.model` read the live environment at reference time, like `current_env`",
    ),
    ambient(
        "invocation_context.rs",
        "std::env::vars_os",
        1,
        "raw launch environment a child process must reproduce; `ctx.*` and `env.*` read the snapshot",
    ),
    ambient("linking/paths.rs", "biscuit_file::home_dir", 1, "locates provider skill, command, and agent directories for linking"),
    ambient("linking/paths.rs", "std::env::current_dir", 1, "the repository whose shared resources `claudine skills` links"),
    ambient("mcp/import.rs", "biscuit_file::home_dir", 4, "locates provider MCP configuration to import"),
    ambient("mcp/types.rs", "biscuit_file::home_dir", 1, "locates the user's MCP catalog"),
    ambient("messaging/resolve.rs", "biscuit_file::home_dir", 1, "expands `~` in a messaging route's configured attachment path"),
    ambient(
        "messaging/resolve.rs",
        "std::env::current_dir",
        1,
        "last-resort base for a messaging route's relative attachment path",
    ),
    ambient("messaging/resolve.rs", "std::env::var", 1, "a messaging route's credential named by environment variable"),
    ambient(
        "messaging/send.rs",
        "std::env::var_os",
        1,
        "`test-fixtures` hook that stalls desktop notifications for delivery tests",
    ),
    ambient("model_catalog/cache.rs", "biscuit_file::home_dir", 1, "locates the model catalog cache"),
    ambient(
        "model_catalog/service.rs",
        "std::env::var",
        1,
        "`CLAUDINE_BACKGROUND_REFRESH` selects blocking model-catalog refresh",
    ),
    ambient("permissions/context.rs", "biscuit_file::home_dir", 1, "classifies tool paths under the user's home for permission policy"),
    ambient("protect/path.rs", "biscuit_file::home_dir", 1, "classifies protected write paths under the user's home"),
    ambient("protect/scrub.rs", "biscuit_file::home_dir", 1, "redacts the user's home from protect diagnostics"),
    ambient("provider/claude/behavior.rs", "biscuit_file::home_dir", 1, HOME_CONFIG),
    ambient("provider/codex/behavior.rs", "biscuit_file::home_dir", 1, HOME_CONFIG),
    ambient("provider/gemini/behavior.rs", "biscuit_file::home_dir", 1, HOME_CONFIG),
    ambient("provider/opencode/behavior.rs", "biscuit_file::home_dir", 1, HOME_CONFIG),
    ambient(
        "render/event_renderer/provider_extension.rs",
        "std::env::var",
        1,
        "whether `ANTHROPIC_API_KEY` is set decides a billing label (rendering)",
    ),
    ambient("reporting/paths.rs", "biscuit_file::home_dir", 2, "locates Claudine's log and report directories"),
    ambient("signals/harvest.rs", "std::env::var", 1, "`CLAUDINE_HARVEST` toggles unmatched-signal harvesting"),
    ambient(
        "stream/providers/opencode.rs",
        ".resolve()",
        3,
        "`OpenCodeTool::resolve` normalizes a stream tool event; not a file reference",
    ),
];

#[test]
fn production_source_builds_contexts_only_through_the_builder() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../../darkmatter/cli/tests/common/context_guard.rs");
    let _ = include_str!("../../../../darkmatter/cli/tests/common/source_scan.rs");
    context_guard::assert_guarded("claudine", &biscuit_test_harness::manifest_dir!().join("src"), ALLOWLIST);
}
