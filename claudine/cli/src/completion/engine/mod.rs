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
//! - **Composition positional** (`claudine compose <TAB>` /
//!   `claudine inline-compose <TAB>` / `claudine sequence <TAB>`) —
//!   rendered by [`super::composition::run`] with per-mode scope sets,
//!   fuzzy matching, and frontmatter gating.
//! - **Setter value** (`claudine compose foo.md spec=@<TAB>`) — rendered
//!   by [`super::setter_value::run`] with `@`-gated file completion
//!   against `docs/`, `features/`, `fixes/`, and `reviews/` at repo,
//!   package-area, and package levels.
//!
//! Remaining slots (wrapper flag values, administrative subcommands, etc.)
//! emit zero candidates so the shell's native file / flag completion takes
//! over. The new engine intentionally does **not** attach file completers
//! to `--append-system-prompt` / `--replace-system-prompt` — the spec
//! leaves that slot to clap's default (`_files` on zsh, default file
//! completion on bash/fish).
//!
//! The classifier never invokes clap — wrapper subcommands use
//! `ignore_errors(true)` in the lenient parse path so clap's view of argv is
//! unreliable for completion purposes. A purely syntactic scan is cheaper
//! and more correct.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::completion::composition;
use crate::completion::root_menu;
use crate::completion::schema_completion;
use crate::completion::scopes::{ComposeMode, ScopeContext};
use crate::completion::setter_value;
use std::ffi::OsString;

mod tokens;

use tokens::{
    is_flag_token, is_global_bool_flag, is_setter_name_partial, is_setter_shaped,
    is_value_bearing_flag, split_setter,
};

/// Top-level classification of the cursor position in argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompletionTarget {
    /// Cursor is at the root subcommand slot (argv position 1, or after
    /// a run of global flags). Phase 1 renders a curated menu here.
    Root(RootPartial),
    /// Cursor is at the positional `<FILE>` slot of a composition
    /// subcommand (`compose`, `inline-compose`, or `sequence`). Phase 3
    /// routes this to [`super::composition::run`].
    CompositionPositional { mode: ComposeMode, partial: String },
    /// Cursor is on a setter-shaped token (`name=value`) inside a
    /// composition subcommand. Phase 4 routes this to
    /// [`super::setter_value::run`], which `@`-gates file completion
    /// against a curated set of documentation-style subdirectories.
    /// The full `name=...` token is carried so the completer can split
    /// name from value and emit a whole-token replacement.
    ///
    /// `file_arg` carries the committed prompt-file positional (when
    /// present), so the schema-aware completer can resolve `$schema` and
    /// surface enum / file candidates per the
    /// [`2026-05-15-schemas`][plan] Phase 4 contract.
    ///
    /// [plan]: ../../../../features/2026-05-15-schemas/plan.md
    SetterValue {
        token: String,
        file_arg: Option<String>,
    },
    /// Cursor is on a partial setter NAME (no `=` typed yet) AFTER the
    /// prompt-file positional slot has been committed. Routes to the
    /// schema-aware property-name completer.
    SetterName { partial: String, file_arg: String },
    /// Cursor is on a flag-shaped token inside a composition subcommand
    /// (`compose` / `inline-compose` / `sequence`). Offers the
    /// provider-selection switches (`--<provider>`, catalog-derived) merged
    /// with clap's other composition flags, so the `--kilo` / `--pi` /
    /// `--antigravity` (and every future provider) switch is discoverable
    /// via `<TAB>` even though the argv normalizer — not a clap field —
    /// is what actually rewrites it to `--provider <slug>`.
    CompositionProviderFlag { partial: String },
    /// Cursor is anywhere else — a wrapper flag value, an administrative
    /// subcommand, etc. The engine emits zero candidates so the shell
    /// falls back to its native file / flag completion.
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
///
/// ## Notes
///
/// Wrapped in a `claudine::completion::run` span so callers running
/// `RUST_LOG=claudine::completion=trace` (and optionally
/// `CLAUDINE_COMPLETION_PROFILE=1`) capture per-invocation latency
/// without instrumenting every internal call site.
pub(crate) fn run(argv: &[String], current_index: usize) -> Vec<String> {
    let _span = tracing::trace_span!(
        target: "claudine::completion",
        "completion::run",
        current_index = current_index,
        argc = argv.len(),
    )
    .entered();
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
        CompletionTarget::Root(partial) => {
            let _span = tracing::trace_span!(
                target: "claudine::completion",
                "completion::root_menu",
            )
            .entered();
            root_menu::render(&partial, ctx)
        }
        CompletionTarget::CompositionPositional { mode, partial } => {
            let _span = tracing::trace_span!(
                target: "claudine::completion",
                "completion::composition",
                mode = ?mode,
            )
            .entered();
            let scope_ctx = ScopeContext::discover();
            composition::run(mode, &scope_ctx, &partial)
        }
        CompletionTarget::CompositionProviderFlag { partial } => {
            let _span = tracing::trace_span!(
                target: "claudine::completion",
                "completion::composition_provider_flag",
            )
            .entered();
            run_composition_provider_flag(&partial, argv, current_index)
        }
        CompletionTarget::SetterValue { token, file_arg } => {
            let _span = tracing::trace_span!(
                target: "claudine::completion",
                "completion::setter_value",
            )
            .entered();
            let scope_ctx = ScopeContext::discover();
            run_setter_value(&token, file_arg.as_deref(), &scope_ctx)
        }
        CompletionTarget::SetterName { partial, file_arg } => {
            let _span = tracing::trace_span!(
                target: "claudine::completion",
                "completion::setter_name",
            )
            .entered();
            let scope_ctx = ScopeContext::discover();
            run_setter_name(&partial, &file_arg, argv, current_index, &scope_ctx)
        }
        CompletionTarget::Other => {
            // Unrecognized slot — try clap's dynamic completion for
            // administrative subcommand flags and other static-command-tree
            // positions before giving up.
            clap_dynamic_fallback(argv, current_index)
        }
    }
}

/// Dispatch the setter-value slot.
///
/// When `file_arg` is known, first try schema-aware completion: enum members
/// for `enum` properties, filesystem paths for `file(match(...))` properties.
/// If the schema does not contribute candidates (string / number / unknown
/// property / no `$schema`), fall back to the existing `@`-gated path so the
/// historical `spec=@docs/...` behavior remains intact.
fn run_setter_value(token: &str, file_arg: Option<&str>, ctx: &ScopeContext) -> Vec<String> {
    if let Some(file_arg) = file_arg
        && let Some(effective) = schema_completion::load_effective_schema(file_arg, ctx)
        && let Some((name, value)) = split_setter(token)
    {
        let schema_candidates = schema_completion::property_value(&effective, name, value, ctx);
        if !schema_candidates.is_empty() {
            return schema_candidates;
        }
    }
    setter_value::run(token, ctx)
}

/// Dispatch the setter-name slot.
///
/// Loads the schema for `file_arg`, collects names already supplied in
/// argv, and asks the schema-aware completer for ordered candidates
/// (required first, then optional, both in declaration order).
fn run_setter_name(
    partial: &str,
    file_arg: &str,
    argv: &[String],
    current_index: usize,
    ctx: &ScopeContext,
) -> Vec<String> {
    let Some(effective) = schema_completion::load_effective_schema(file_arg, ctx) else {
        return Vec::new();
    };
    let supplied = collect_supplied_setter_names(argv, current_index);
    let declared_order = schema_completion::declared_property_order(file_arg, ctx);
    schema_completion::property_names(&effective, partial, &supplied, &declared_order)
}

/// Walk argv (excluding the cursor token itself) and collect setter names
/// already supplied. Used by the setter-name completer to suppress
/// re-suggesting keys the user has already passed.
fn collect_supplied_setter_names(argv: &[String], current_index: usize) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();
    for (idx, tok) in argv.iter().enumerate() {
        if idx == current_index {
            continue;
        }
        if let Some((name, _)) = split_setter(tok) {
            out.insert(name.to_string());
        }
    }
    out
}

/// Fall back to clap's dynamic completion for slots the custom engine does
/// not own (administrative subcommand flags, wrapper flags, etc.).
///
/// Activates when:
/// - the cursor partial is flag-shaped (`-` or `--`), or
/// - the previous token is flag-shaped (indicating the cursor is on a flag
///   value).
///
/// Empty or non-flag partials at vanilla positional slots still defer to the
/// shell's native file completion, preserving the contract tested by
/// `non_targeted_subcommand_emits_no_candidates`.
///
/// Uses the same `ignore_errors(true)` command tree that powers the legacy
/// `CompleteEnv` path so wrapper subcommands do not short-circuit on unknown
/// passthrough tokens.
/// Complete a flag-shaped token inside a composition subcommand.
///
/// Offers the catalog-derived `--<provider>` selection switches first
/// (drift-proof — see [`crate::completion::root_menu::wrapper_tokens`]; the
/// argv normalizer's Rule 1 rewrites each to `--provider <slug>`), then
/// merges clap's other composition flags (`--model`, `--output`,
/// `--provider`, …) so the whole flag surface completes. Deduped, providers
/// first. Kilo/Pi/Antigravity (and any future provider) appear automatically
/// even though they have no dedicated clap boolean field.
fn run_composition_provider_flag(
    partial: &str,
    argv: &[String],
    current_index: usize,
) -> Vec<String> {
    let mut out: Vec<String> = crate::completion::root_menu::wrapper_tokens()
        .into_iter()
        .map(|token| format!("--{token}"))
        .filter(|flag| flag.starts_with(partial))
        .collect();
    for candidate in clap_dynamic_fallback(argv, current_index) {
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    }
    out
}

fn clap_dynamic_fallback(argv: &[String], current_index: usize) -> Vec<String> {
    let partial = argv.get(current_index).map(String::as_str).unwrap_or("");
    let prev_flag = current_index
        .checked_sub(1)
        .and_then(|i| argv.get(i))
        .is_some_and(|p| p.starts_with('-'));
    if !partial.starts_with('-') && !prev_flag {
        return Vec::new();
    }
    let mut cmd = super::completion_command();
    let args: Vec<OsString> = argv.iter().map(|s| s.into()).collect();
    match clap_complete::engine::complete(
        &mut cmd,
        args,
        current_index,
        std::env::current_dir().ok().as_deref(),
    ) {
        Ok(candidates) => candidates
            .into_iter()
            .map(|c| c.get_value().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Decide which completion slot the cursor is in.
///
/// The rule is syntactic: walk argv from position 1 up to `current_index`,
/// skipping global flags and their values. If the walk completes without
/// consuming a non-flag token, the cursor is at the root slot. Otherwise
/// the first non-global token is the subcommand, and the cursor slot is
/// classified per-subcommand — composition positional for `compose` /
/// `inline-compose` / `sequence`; Other for anything else.
pub(crate) fn classify_completion_target(
    argv: &[String],
    current_index: usize,
) -> CompletionTarget {
    if current_index == 0 {
        return CompletionTarget::Other;
    }

    // A literal `--` before the cursor means we've crossed into wrapper
    // passthrough. The root menu and every slot-specific completer
    // decline to touch anything past that separator.
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
        // committed. Delegate to the per-subcommand classifier.
        return classify_post_subcommand(argv, current_index, i);
    }

    let partial = argv.get(current_index).map(String::as_str).unwrap_or("");
    CompletionTarget::Root(classify_root_partial(partial))
}

/// Composition subcommand → [`ComposeMode`] mapping.
fn compose_mode_for(sub: &str) -> Option<ComposeMode> {
    match sub {
        "compose" => Some(ComposeMode::Compose),
        "inline-compose" => Some(ComposeMode::InlineCompose),
        "sequence" => Some(ComposeMode::Sequence),
        _ => None,
    }
}

/// Classify the cursor position given that a subcommand token sits at
/// `sub_idx`. Returns [`CompletionTarget::CompositionPositional`] when
/// the cursor is at the first (file) positional of a composition
/// subcommand, [`CompletionTarget::SetterValue`] when it is on a
/// `name=value` token, and `Other` otherwise.
///
/// The setter-shape check runs **before** the positional walk so a
/// setter at the cursor wins regardless of how many positionals the
/// user has already committed: `claudine compose foo.md spec=@<TAB>`
/// still routes to the setter-value completer even though `foo.md`
/// already occupies the file slot.
fn classify_post_subcommand(
    argv: &[String],
    current_index: usize,
    sub_idx: usize,
) -> CompletionTarget {
    let Some(sub) = argv.get(sub_idx).map(String::as_str) else {
        return CompletionTarget::Other;
    };
    let Some(mode) = compose_mode_for(sub) else {
        return CompletionTarget::Other;
    };

    let current = argv.get(current_index).map(String::as_str).unwrap_or("");

    // A flag-shaped cursor token routes to the provider-flag completer, so the
    // `--<provider>` selection switches (catalog-derived, drift-proof) are
    // discoverable via `<TAB>`. Non-provider composition flags still surface
    // through the clap merge inside the completer.
    if current.starts_with('-') {
        return CompletionTarget::CompositionProviderFlag {
            partial: current.to_string(),
        };
    }

    // Walk tokens between the subcommand and the cursor to find the first
    // committed prompt-file positional (skipping flags, flag values, and
    // `key=value` setters). The file_arg, when present, unlocks the
    // schema-aware completers downstream.
    let (file_arg, has_seen_positional_before_cursor) =
        scan_committed_positional(argv, sub_idx + 1, current_index);

    // A setter-shaped cursor routes to the setter-value completer
    // regardless of the positional state — the `name=` prefix
    // disambiguates against the file slot.
    if is_setter_shaped(current) {
        return CompletionTarget::SetterValue {
            token: current.to_string(),
            file_arg,
        };
    }

    // If the file slot has been committed AND the cursor token is a valid
    // setter-name partial (no `=`, plain identifier shape), route to the
    // schema-aware setter-name completer. This is the new Phase 4 path —
    // historically these tokens fell through to `Other`.
    if let Some(file) = file_arg.as_ref()
        && is_setter_name_partial(current)
    {
        return CompletionTarget::SetterName {
            partial: current.to_string(),
            file_arg: file.clone(),
        };
    }

    if has_seen_positional_before_cursor {
        return CompletionTarget::Other;
    }

    CompletionTarget::CompositionPositional {
        mode,
        partial: current.to_string(),
    }
}

/// Walk argv between `start` and `cursor` and return:
///
/// - The first committed positional file argument (non-flag, non-setter)
///   when one exists.
/// - `true` when at least one positional was committed before the cursor.
fn scan_committed_positional(
    argv: &[String],
    start: usize,
    cursor: usize,
) -> (Option<String>, bool) {
    let mut idx = start;
    let mut file_arg: Option<String> = None;
    let mut seen = false;
    while idx < cursor {
        let token = argv[idx].as_str();
        if token == "--" {
            idx += 1;
            continue;
        }
        if is_flag_token(token) {
            if is_value_bearing_flag(token) && !token.contains('=') {
                idx += 2;
            } else {
                idx += 1;
            }
            continue;
        }
        if is_setter_shaped(token) {
            idx += 1;
            continue;
        }
        seen = true;
        if file_arg.is_none() {
            file_arg = Some(token.to_string());
        }
        idx += 1;
    }
    (file_arg, seen)
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
/// a `.git` directory or worktree pointer is found.
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
    fn classifier_composition_flag_slot_routes_to_provider_flag() {
        let a = argv(&["claudine", "compose", "--ki"]);
        assert_eq!(
            classify_completion_target(&a, 2),
            CompletionTarget::CompositionProviderFlag {
                partial: "--ki".to_string(),
            },
        );
    }

    #[test]
    fn composition_provider_flag_completer_is_catalog_derived() {
        // Every provider's switch is offered — including kilo/pi/antigravity,
        // which have no dedicated clap boolean field — so the derived surface
        // never drifts as providers are added.
        let a = argv(&["claudine", "compose", "--"]);
        let got = run_composition_provider_flag("--", &a, 2);
        for flag in ["--claude", "--kilo", "--pi", "--antigravity"] {
            assert!(got.iter().any(|c| c == flag), "{flag} missing: {got:?}");
        }
        // Prefix filtering keeps only matching switches. (argv[current_index]
        // must equal the partial — the clap merge reads the token from argv.)
        let narrowed_argv = argv(&["claudine", "compose", "--kil"]);
        let narrowed = run_composition_provider_flag("--kil", &narrowed_argv, 2);
        assert!(narrowed.iter().any(|c| c == "--kilo"));
        assert!(!narrowed.iter().any(|c| c == "--claude"));
    }

    #[test]
    fn classifier_composition_positional_on_compose() {
        let a = argv(&["claudine", "compose", ""]);
        assert_eq!(
            classify_completion_target(&a, 2),
            CompletionTarget::CompositionPositional {
                mode: ComposeMode::Compose,
                partial: String::new(),
            },
        );
    }

    #[test]
    fn classifier_composition_positional_on_inline_compose_with_partial() {
        let a = argv(&["claudine", "inline-compose", "pl"]);
        assert_eq!(
            classify_completion_target(&a, 2),
            CompletionTarget::CompositionPositional {
                mode: ComposeMode::InlineCompose,
                partial: "pl".to_string(),
            },
        );
    }

    #[test]
    fn classifier_composition_positional_on_sequence() {
        let a = argv(&["claudine", "sequence", "@s"]);
        assert_eq!(
            classify_completion_target(&a, 2),
            CompletionTarget::CompositionPositional {
                mode: ComposeMode::Sequence,
                partial: "@s".to_string(),
            },
        );
    }

    #[test]
    fn classifier_setter_name_after_first_positional_is_committed() {
        // Schemas Phase 4: once the file slot is committed, a plain
        // identifier-shaped token at the cursor is treated as a setter-
        // name partial. The schema-aware completer downstream decides
        // whether the name maps to a schema property; the classifier is
        // happy as long as the shape and file_arg are right.
        let a = argv(&["claudine", "compose", "plan.md", "more"]);
        assert_eq!(
            classify_completion_target(&a, 3),
            CompletionTarget::SetterName {
                partial: "more".to_string(),
                file_arg: "plan.md".to_string(),
            },
        );
    }

    #[test]
    fn classifier_other_when_second_positional_has_non_identifier_chars() {
        // Tokens that look like paths or contain unsupported characters
        // are not setter-name partials and still route to `Other`.
        let a = argv(&["claudine", "compose", "plan.md", "more/stuff"]);
        assert_eq!(classify_completion_target(&a, 3), CompletionTarget::Other);
    }

    #[test]
    fn classifier_setter_value_when_cursor_is_setter_shaped() {
        let a = argv(&["claudine", "compose", "key=val"]);
        assert_eq!(
            classify_completion_target(&a, 2),
            CompletionTarget::SetterValue {
                token: "key=val".to_string(),
                file_arg: None,
            },
        );
    }

    #[test]
    fn classifier_setter_value_after_committed_positional() {
        // Phase 4 (improved-shell-completions) contract: a setter-shaped
        // cursor wins even when the file slot has already been filled.
        // The classifier must not return `Other` just because a positional
        // was observed earlier — and the schemas-Phase-4 work now also
        // surfaces the committed file as `file_arg` so the schema-aware
        // value completer can use it.
        let a = argv(&["claudine", "compose", "foo.md", "spec=@s"]);
        assert_eq!(
            classify_completion_target(&a, 3),
            CompletionTarget::SetterValue {
                token: "spec=@s".to_string(),
                file_arg: Some("foo.md".to_string()),
            },
        );
    }

    #[test]
    fn classifier_setter_value_on_inline_compose() {
        let a = argv(&["claudine", "inline-compose", "foo.md", "ref=@d"]);
        assert_eq!(
            classify_completion_target(&a, 3),
            CompletionTarget::SetterValue {
                token: "ref=@d".to_string(),
                file_arg: Some("foo.md".to_string()),
            },
        );
    }

    #[test]
    fn classifier_setter_value_on_sequence() {
        let a = argv(&["claudine", "sequence", "foo.md", "plan=@p"]);
        assert_eq!(
            classify_completion_target(&a, 3),
            CompletionTarget::SetterValue {
                token: "plan=@p".to_string(),
                file_arg: Some("foo.md".to_string()),
            },
        );
    }

    #[test]
    fn classifier_other_on_setter_shaped_cursor_for_non_composition_sub() {
        // Wrappers and admin subcommands never route through the setter
        // classifier — they fall through to the legacy supplement.
        let a = argv(&["claudine", "claude", "spec=@s"]);
        assert_eq!(classify_completion_target(&a, 2), CompletionTarget::Other);
    }

    #[test]
    fn classifier_skips_setter_earlier_still_reaches_positional() {
        // Earlier setter isn't a positional; cursor is still the first
        // positional for the composition completer.
        let a = argv(&["claudine", "compose", "key=val", "@plan"]);
        assert_eq!(
            classify_completion_target(&a, 3),
            CompletionTarget::CompositionPositional {
                mode: ComposeMode::Compose,
                partial: "@plan".to_string(),
            },
        );
    }

    #[test]
    fn classifier_skips_value_of_preceding_value_bearing_flag() {
        let a = argv(&["claudine", "compose", "--model", "gpt-4", ""]);
        assert_eq!(
            classify_completion_target(&a, 4),
            CompletionTarget::CompositionPositional {
                mode: ComposeMode::Compose,
                partial: String::new(),
            },
        );
    }

    #[test]
    fn classifier_other_on_wrapper_subcommand() {
        let a = argv(&["claudine", "claude", ""]);
        assert_eq!(classify_completion_target(&a, 2), CompletionTarget::Other);
    }

    #[test]
    fn classifier_other_on_non_composition_subcommand() {
        let a = argv(&["claudine", "hooks", ""]);
        assert_eq!(classify_completion_target(&a, 2), CompletionTarget::Other);
    }

    #[test]
    fn classifier_other_when_cursor_crosses_double_dash_separator() {
        let a = argv(&["claudine", "claude", "--", "--model"]);
        assert_eq!(classify_completion_target(&a, 3), CompletionTarget::Other,);
    }

    #[test]
    fn classifier_other_when_current_index_is_zero() {
        let a = argv(&["claudine", ""]);
        assert_eq!(classify_completion_target(&a, 0), CompletionTarget::Other,);
    }

    #[test]
    fn classifier_other_when_debug_value_slot_is_the_cursor() {
        // `claudine --debug <TAB>` — the cursor sits on the value slot of
        // `--debug`, not on the root subcommand slot. We treat this as
        // Other so the (currently no-op) value-slot completer handles it.
        let a = argv(&["claudine", "--debug", ""]);
        assert_eq!(classify_completion_target(&a, 2), CompletionTarget::Other,);
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

    #[test]
    fn clap_fallback_completes_providers_flags() {
        let a = argv(&["claudine", "providers", "--des"]);
        let got = run_with_context(&a, 2, &test_ctx(true, true, true));
        assert!(
            got.iter().any(|c| c == "--describe"),
            "expected --describe in candidates, got {got:?}"
        );
    }

    #[test]
    fn clap_fallback_completes_providers_format_flag() {
        let a = argv(&["claudine", "providers", "--format"]);
        let got = run_with_context(&a, 2, &test_ctx(true, true, true));
        assert!(
            got.iter().any(|c| c == "--format"),
            "expected --format in candidates, got {got:?}"
        );
    }

    // -- SetterName classification (schemas Phase 4) -----------------------

    #[test]
    fn classifier_setter_name_on_inline_compose() {
        let a = argv(&["claudine", "inline-compose", "plan.md", "tit"]);
        assert_eq!(
            classify_completion_target(&a, 3),
            CompletionTarget::SetterName {
                partial: "tit".to_string(),
                file_arg: "plan.md".to_string(),
            },
        );
    }

    #[test]
    fn classifier_setter_name_on_sequence() {
        let a = argv(&["claudine", "sequence", "plan.md", "stat"]);
        assert_eq!(
            classify_completion_target(&a, 3),
            CompletionTarget::SetterName {
                partial: "stat".to_string(),
                file_arg: "plan.md".to_string(),
            },
        );
    }

    #[test]
    fn classifier_setter_name_with_empty_cursor_after_committed_file() {
        // Empty cursor token after `plan.md spec=foo <TAB>` is treated as
        // an unfiltered setter-name lookup — show every (unsupplied)
        // property.
        let a = argv(&["claudine", "compose", "plan.md", "spec=foo", ""]);
        // Empty token has no shape, so `is_setter_name_partial` returns
        // false: classifier returns Other (the value slot is fully filled).
        assert_eq!(classify_completion_target(&a, 4), CompletionTarget::Other);
    }

    #[test]
    fn classifier_setter_name_only_when_file_arg_present() {
        // Without a committed file_arg, an identifier-shaped cursor stays
        // at the composition positional slot — there's no schema to load
        // yet.
        let a = argv(&["claudine", "compose", "tit"]);
        assert_eq!(
            classify_completion_target(&a, 2),
            CompletionTarget::CompositionPositional {
                mode: ComposeMode::Compose,
                partial: "tit".to_string(),
            },
        );
    }

    #[test]
    fn is_setter_name_partial_accepts_identifiers() {
        assert!(is_setter_name_partial("foo"));
        assert!(is_setter_name_partial("_bar"));
        assert!(is_setter_name_partial("a1-b2"));
        assert!(is_setter_name_partial("snake_case"));
    }

    #[test]
    fn is_setter_name_partial_rejects_non_identifier_shapes() {
        assert!(!is_setter_name_partial(""));
        assert!(!is_setter_name_partial("1abc"));
        assert!(!is_setter_name_partial("foo/bar"));
        assert!(!is_setter_name_partial("foo=bar"));
        assert!(!is_setter_name_partial("foo.bar"));
    }

    #[test]
    fn scan_committed_positional_finds_file_arg_among_setters_and_flags() {
        let a = argv(&[
            "claudine", "compose", "--model", "gpt-4", "spec=val", "plan.md", "",
        ]);
        let (file, seen) = scan_committed_positional(&a, 2, 6);
        assert_eq!(file.as_deref(), Some("plan.md"));
        assert!(seen);
    }

    #[test]
    fn scan_committed_positional_returns_none_when_no_positional() {
        let a = argv(&["claudine", "compose", "spec=val", "--model", "gpt", ""]);
        let (file, seen) = scan_committed_positional(&a, 2, 5);
        assert!(file.is_none());
        assert!(!seen);
    }

    #[test]
    fn collect_supplied_setter_names_skips_cursor_token() {
        let a = argv(&["claudine", "compose", "plan.md", "title=hi", "des"]);
        let got = collect_supplied_setter_names(&a, 4);
        assert!(got.contains("title"));
        assert!(!got.contains("des"));
    }

    #[test]
    fn split_setter_handles_typical_cases() {
        assert_eq!(split_setter("title=hi"), Some(("title", "hi")));
        assert_eq!(split_setter("a_b-c=v=w"), Some(("a_b-c", "v=w")));
        assert!(split_setter("=missing-name").is_none());
        assert!(split_setter("no-equals").is_none());
        assert!(split_setter("9bad=v").is_none());
    }

    #[test]
    fn clap_fallback_completes_providers_format_values() {
        let a = argv(&["claudine", "providers", "--format", ""]);
        let got = run_with_context(&a, 3, &test_ctx(true, true, true));
        assert!(
            got.iter().any(|c| c == "text"),
            "expected 'text' format value, got {got:?}"
        );
        assert!(
            got.iter().any(|c| c == "json"),
            "expected 'json' format value, got {got:?}"
        );
    }
}
