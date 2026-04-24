//! Top-level classifier and dispatcher for dynamic shell completion.
//!
//! Phase 1 of the `2026-04-24-improved-shell-completions` feature. This
//! module owns the `claudine __complete` entry point: it inspects the argv
//! the shell forwards in, classifies which completion slot the cursor sits
//! in, and dispatches to a slot-specific completer.
//!
//! Currently implemented slots:
//!
//! - **Root menu** (`claudine <TAB>`) — rendered by
//!   [`root_menu::render`] from a curated, spec-ordered subcommand list.
//!   The sole flag surfaced here is `--help`.
//!
//! Future phases introduce slot-specific completers:
//!
//! - **Composition positional** (`claudine compose <TAB>`) — Phase 3.
//! - **Setter value** (`claudine compose foo.md spec=@<TAB>`) — Phase 4.
//!
//! Until those slots are re-implemented, non-root slots fall through to the
//! legacy [`super::supplement`] engine so existing behavior is preserved.
//! The bridge is removed in Phase 3/4 as each slot is rewritten.
//!
//! The classifier never invokes clap — wrapper subcommands use
//! `ignore_errors(true)` in the lenient parse path so clap's view of argv is
//! unreliable for completion purposes. A purely syntactic scan is cheaper
//! and more correct.

use std::path::{Path, PathBuf};

use crate::completion::root_menu;

/// Top-level classification of the cursor position in argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompletionTarget {
    /// Cursor is at the root subcommand slot (argv position 1, or after
    /// a run of global flags). Phase 1 renders a curated menu here.
    Root(RootPartial),
    /// Cursor is anywhere else — a composition positional, a setter value,
    /// a wrapper flag value, etc. Phase 1 defers these to the legacy
    /// supplement engine.
    Other,
}

/// Shape of the partial token sitting under the cursor at the root slot.
///
/// Root-slot rendering branches on the leading character of the partial:
/// empty and word partials produce the subcommand menu; flag-shaped
/// partials (`-` / `--` / `-h` / `--h...`) produce the `--help` candidate
/// as the sole option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootPartial {
    /// No token typed at the cursor yet (empty string).
    Empty,
    /// A non-flag partial — a prefix of a subcommand name.
    Word(String),
    /// A flag-shaped partial (starts with `-`). Only `--help` is offered
    /// from the root slot.
    FlagLike(String),
}

/// Context needed to compute the root menu.
///
/// Kept as a plain struct (rather than a trait) so tests can construct
/// arbitrary combinations of config presence without filesystem setup.
#[derive(Debug, Clone)]
pub(crate) struct RootContext {
    pub(crate) user_config_exists: bool,
    pub(crate) repo_config_exists: bool,
    pub(crate) in_repo: bool,
}

impl RootContext {
    /// Build a context by inspecting the filesystem.
    ///
    /// Only `stat`s are performed — no config files are parsed. File
    /// presence is sufficient to decide whether `init` should appear in
    /// the root menu.
    pub(crate) fn discover() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let home = dirs::home_dir();
        let user_config_exists = user_config_exists(home.as_deref());
        let (repo_config_exists, in_repo) = detect_repo_config(&cwd);
        Self {
            user_config_exists,
            repo_config_exists,
            in_repo,
        }
    }
}

/// Entry point for the hidden `claudine __complete` subcommand.
///
/// Classifies the cursor position, dispatches to the appropriate
/// slot-specific completer, and returns the rendered candidate strings
/// (one per line on stdout).
pub(crate) fn run(argv: &[String], current_index: usize) -> Vec<String> {
    let ctx = RootContext::discover();
    run_with_context(argv, current_index, &ctx)
}

/// Dependency-injected variant of [`run`] used by tests. Production code
/// calls [`run`], which discovers the context from the filesystem.
pub(crate) fn run_with_context(
    argv: &[String],
    current_index: usize,
    ctx: &RootContext,
) -> Vec<String> {
    match classify_completion_target(argv, current_index) {
        CompletionTarget::Root(partial) => root_menu::render(&partial, ctx),
        CompletionTarget::Other => {
            // Phase 1 bridge — non-root slots keep the legacy supplement
            // behavior until Phases 3 and 4 rewrite them.
            super::supplement::run(argv, current_index)
        }
    }
}

/// Decide which completion slot the cursor is in.
///
/// The rule is syntactic: walk argv from position 1 up to `current_index`,
/// skipping global flags and their values. If the walk completes without
/// consuming a non-flag token, the cursor is at the root slot. Any non-flag
/// token before `current_index` means a subcommand has been committed and
/// the slot is delegated to the (Phase 1 legacy, later phases dedicated)
/// post-subcommand completer.
pub(crate) fn classify_completion_target(
    argv: &[String],
    current_index: usize,
) -> CompletionTarget {
    if current_index == 0 {
        return CompletionTarget::Other;
    }

    // A literal `--` before the cursor means we've crossed into wrapper
    // passthrough. Root-menu and supplement rules both decline to touch
    // anything past that separator.
    for token in argv.iter().take(current_index) {
        if token == "--" {
            return CompletionTarget::Other;
        }
    }

    let mut i = 1usize;
    while i < current_index {
        let token = argv[i].as_str();
        if token == "--debug" {
            // Global value-bearing flag; skip flag + value. If `--debug`
            // sits immediately before the cursor, the cursor is ON the
            // value slot — that is not the root slot.
            if i + 1 == current_index {
                return CompletionTarget::Other;
            }
            i += 2;
            continue;
        }
        if token.starts_with("--debug=") {
            i += 1;
            continue;
        }
        if is_global_bool_flag(token) {
            i += 1;
            continue;
        }
        // A non-global, non-flag token at i means a subcommand has been
        // committed — we are past the root slot.
        return CompletionTarget::Other;
    }

    let partial = argv.get(current_index).map(String::as_str).unwrap_or("");
    CompletionTarget::Root(classify_root_partial(partial))
}

fn classify_root_partial(token: &str) -> RootPartial {
    if token.is_empty() {
        RootPartial::Empty
    } else if token.starts_with('-') {
        RootPartial::FlagLike(token.to_string())
    } else {
        RootPartial::Word(token.to_string())
    }
}

fn is_global_bool_flag(token: &str) -> bool {
    matches!(
        token,
        "--plain" | "--verbose" | "-v" | "-vv" | "-vvv" | "--help" | "-h"
    )
}

/// Return `true` when a user-scope Claudine config exists under `$HOME`.
///
/// Both the canonical `.claudine/config.json` path and the JSON5 variant
/// (`.claudine/config.json5`) are accepted — JSON5 is a common hand-edited
/// form in the wild even though `user_config_path()` in the library only
/// probes `.json`.
fn user_config_exists(home: Option<&Path>) -> bool {
    let Some(home) = home else {
        return false;
    };
    let candidates = [
        home.join(".claudine").join("config.json"),
        home.join(".claudine").join("config.json5"),
    ];
    candidates.iter().any(|p| p.is_file())
}

/// Look upward from `cwd` for a git repo root and check for a repo-scope
/// config.
///
/// Returns `(repo_config_exists, in_repo)`. `in_repo` is `true` as soon as
/// a `.git` directory or worktree pointer is found — this matches the
/// supplement engine's `find_enclosing_repo` semantics.
fn detect_repo_config(cwd: &Path) -> (bool, bool) {
    for ancestor in cwd.ancestors() {
        let dot_git = ancestor.join(".git");
        if dot_git.is_dir() || dot_git.is_file() {
            let cfg = ancestor.join(".claudine").join("config.json");
            return (cfg.is_file(), true);
        }
    }
    (false, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn classifier_root_slot_at_position_1_empty() {
        let a = argv(&["claudine", ""]);
        assert_eq!(
            classify_completion_target(&a, 1),
            CompletionTarget::Root(RootPartial::Empty),
        );
    }

    #[test]
    fn classifier_root_slot_at_position_1_partial_word() {
        let a = argv(&["claudine", "com"]);
        assert_eq!(
            classify_completion_target(&a, 1),
            CompletionTarget::Root(RootPartial::Word("com".to_string())),
        );
    }

    #[test]
    fn classifier_root_slot_with_flag_partial_single_dash() {
        let a = argv(&["claudine", "-"]);
        assert_eq!(
            classify_completion_target(&a, 1),
            CompletionTarget::Root(RootPartial::FlagLike("-".to_string())),
        );
    }

    #[test]
    fn classifier_root_slot_with_flag_partial_long_prefix() {
        let a = argv(&["claudine", "--h"]);
        assert_eq!(
            classify_completion_target(&a, 1),
            CompletionTarget::Root(RootPartial::FlagLike("--h".to_string())),
        );
    }

    #[test]
    fn classifier_root_slot_after_global_plain_flag() {
        let a = argv(&["claudine", "--plain", ""]);
        assert_eq!(
            classify_completion_target(&a, 2),
            CompletionTarget::Root(RootPartial::Empty),
        );
    }

    #[test]
    fn classifier_root_slot_after_global_verbose_flag() {
        let a = argv(&["claudine", "-v", "c"]);
        assert_eq!(
            classify_completion_target(&a, 2),
            CompletionTarget::Root(RootPartial::Word("c".to_string())),
        );
    }

    #[test]
    fn classifier_root_slot_after_global_debug_with_value() {
        let a = argv(&["claudine", "--debug", "trace", ""]);
        assert_eq!(
            classify_completion_target(&a, 3),
            CompletionTarget::Root(RootPartial::Empty),
        );
    }

    #[test]
    fn classifier_root_slot_after_debug_equals_form() {
        let a = argv(&["claudine", "--debug=info", ""]);
        assert_eq!(
            classify_completion_target(&a, 2),
            CompletionTarget::Root(RootPartial::Empty),
        );
    }

    #[test]
    fn classifier_other_when_subcommand_committed() {
        let a = argv(&["claudine", "compose", ""]);
        assert_eq!(
            classify_completion_target(&a, 2),
            CompletionTarget::Other,
        );
    }

    #[test]
    fn classifier_other_when_cursor_crosses_double_dash_separator() {
        let a = argv(&["claudine", "claude", "--", "--model"]);
        assert_eq!(
            classify_completion_target(&a, 3),
            CompletionTarget::Other,
        );
    }

    #[test]
    fn classifier_other_when_current_index_is_zero() {
        let a = argv(&["claudine", ""]);
        assert_eq!(
            classify_completion_target(&a, 0),
            CompletionTarget::Other,
        );
    }

    #[test]
    fn classifier_other_when_debug_value_slot_is_the_cursor() {
        // `claudine --debug <TAB>` — the cursor sits on the value slot of
        // `--debug`, not on the root subcommand slot. We treat this as
        // Other so the (currently no-op) value-slot completer handles it.
        let a = argv(&["claudine", "--debug", ""]);
        assert_eq!(
            classify_completion_target(&a, 2),
            CompletionTarget::Other,
        );
    }

    #[test]
    fn classify_root_partial_detects_empty_and_word_and_flag() {
        assert_eq!(classify_root_partial(""), RootPartial::Empty);
        assert_eq!(
            classify_root_partial("com"),
            RootPartial::Word("com".to_string())
        );
        assert_eq!(
            classify_root_partial("-h"),
            RootPartial::FlagLike("-h".to_string())
        );
        assert_eq!(
            classify_root_partial("--help"),
            RootPartial::FlagLike("--help".to_string())
        );
    }

    #[test]
    fn user_config_exists_detects_json_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dot_claudine = tmp.path().join(".claudine");
        std::fs::create_dir_all(&dot_claudine).unwrap();
        std::fs::write(dot_claudine.join("config.json"), "{}").unwrap();
        assert!(user_config_exists(Some(tmp.path())));
    }

    #[test]
    fn user_config_exists_detects_json5_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dot_claudine = tmp.path().join(".claudine");
        std::fs::create_dir_all(&dot_claudine).unwrap();
        std::fs::write(dot_claudine.join("config.json5"), "{}").unwrap();
        assert!(user_config_exists(Some(tmp.path())));
    }

    #[test]
    fn user_config_exists_returns_false_when_absent() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(!user_config_exists(Some(tmp.path())));
    }

    #[test]
    fn user_config_exists_returns_false_when_home_missing() {
        assert!(!user_config_exists(None));
    }

    #[test]
    fn detect_repo_config_finds_repo_and_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".claudine")).unwrap();
        std::fs::write(tmp.path().join(".claudine").join("config.json"), "{}").unwrap();
        let (has_cfg, in_repo) = detect_repo_config(tmp.path());
        assert!(in_repo);
        assert!(has_cfg);
    }

    #[test]
    fn detect_repo_config_finds_repo_without_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        let (has_cfg, in_repo) = detect_repo_config(tmp.path());
        assert!(in_repo);
        assert!(!has_cfg);
    }

    #[test]
    fn detect_repo_config_handles_no_repo() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (has_cfg, in_repo) = detect_repo_config(tmp.path());
        assert!(!in_repo);
        assert!(!has_cfg);
    }

    // -- end-to-end dispatch --------------------------------------------

    fn test_ctx(user: bool, repo: bool, in_repo: bool) -> RootContext {
        RootContext {
            user_config_exists: user,
            repo_config_exists: repo,
            in_repo,
        }
    }

    #[test]
    fn run_with_context_emits_full_root_menu_on_bare_tab() {
        let a = argv(&["claudine", ""]);
        let got = run_with_context(&a, 1, &test_ctx(true, true, true));
        assert_eq!(got.first().map(String::as_str), Some("compose"));
        assert!(got.contains(&"config".to_string()));
        assert!(!got.contains(&"init".to_string()));
    }

    #[test]
    fn run_with_context_includes_init_when_no_configs() {
        let a = argv(&["claudine", ""]);
        let got = run_with_context(&a, 1, &test_ctx(false, false, false));
        assert!(got.contains(&"init".to_string()));
    }

    #[test]
    fn run_with_context_filters_by_word_prefix() {
        let a = argv(&["claudine", "com"]);
        let got = run_with_context(&a, 1, &test_ctx(true, true, true));
        assert_eq!(got, vec!["compose", "commands", "completions"]);
    }

    #[test]
    fn run_with_context_emits_help_only_for_flag_partial() {
        let a = argv(&["claudine", "--h"]);
        let got = run_with_context(&a, 1, &test_ctx(true, true, true));
        assert_eq!(got, vec!["--help"]);
    }

    #[test]
    fn run_with_context_handles_global_flag_before_cursor() {
        let a = argv(&["claudine", "--plain", "c"]);
        let got = run_with_context(&a, 2, &test_ctx(true, true, true));
        assert!(got.iter().any(|c| c == "compose"));
        assert!(got.iter().any(|c| c == "commands"));
    }
}
