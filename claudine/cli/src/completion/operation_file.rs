//! Runtime ENTER-path autocomplete for the operation-file positional.
//!
//! Phase 3 of the `2026-06-14-auto-complete` feature. When
//! `claudine compose|inline-compose|sequence <file>` fails to resolve the
//! file reference, this module offers an interactive picker: one match →
//! confirmation dialog; many matches → two-pane chooser.

// `CompositionError` carries variants with several `PathBuf` and other
// owned fields (e.g. `LoopIterationFailed`, `LoopRateLimited`) so the
// enum-on-the-stack is sizable. Boxing the inner data would ripple
// through every existing call site for marginal benefit; the closures
// in this file legitimately propagate the typed error and that's the
// shape they need to keep.
#![allow(clippy::result_large_err)]

use std::collections::{HashMap, HashSet};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use biscuit_file::to_portable_string;
use biscuit_tui::components::choose::ChoiceOption;
use claudine::composition::{
    CompositionError, FileDetail, extract_markdown_detail, extract_yaml_sequence_detail,
};

use super::autocomplete_ui::{choose_one_file, confirm_one_file};
use super::frontmatter;
use super::scopes::{self, ComposeMode, ScopeContext};
use super::walker::{self, WalkOutcome, MAX_CANDIDATES};

/// Interactively resolve an operation-file reference when the original
/// `FileReference` lookup failed.
///
/// Gates on an interactive terminal (stdin **and** stderr TTY). Returns the
/// selected runnable file reference string, or a typed error:
///
/// - [`CompositionError::AutocompleteNotInteractive`] when the session is
///   not interactive.
/// - [`CompositionError::AutocompleteNoMatches`] when zero candidates pass
///   the mode contract.
/// - [`CompositionError::AutocompleteOverCap`] when more than
///   [`MAX_CANDIDATES`] files match the query **and** satisfy the active
///   operation mode.
/// - [`CompositionError::AutocompleteCancelled`] when the user declines the
///   single-match confirmation or cancels the chooser.
pub(crate) fn autocomplete_operation_file(
    query: &str,
    mode: ComposeMode,
) -> Result<String, CompositionError> {
    if !is_interactive() {
        return Err(CompositionError::AutocompleteNotInteractive);
    }

    let ctx = ScopeContext::discover();
    let candidates = gather_candidates(query, mode, &ctx)?;
    present_and_select(candidates, query, mode, &ctx)
}

/// Collect query-matching candidates across the mode's scope set.
///
/// Both the query substring filter and the active-mode contract are pushed
/// into the walk so the cap counts candidates that satisfy both predicates,
/// not raw query matches. The walker is capped at [`MAX_CANDIDATES`] + 1 so
/// we can early-abort as soon as the valid count exceeds the cap.
fn gather_candidates(
    query: &str,
    mode: ComposeMode,
    ctx: &ScopeContext,
) -> Result<Vec<PathBuf>, CompositionError> {
    let scope_set = scopes::resolve_compose_scopes(ctx, mode);

    let mut collected: Vec<PathBuf> = Vec::new();
    let mut total_valid: usize = 0;
    let query_lower = query.to_ascii_lowercase();

    for scope in scope_set.iter_scopes() {
        let outcome = walker::walk_scope_filtered(scope, MAX_CANDIDATES + 1, |path| {
            scopes::path_matches_query(path, &query_lower) && frontmatter::valid_for_mode(path, mode)
        });
        match outcome {
            WalkOutcome::Complete(paths) => {
                total_valid += paths.len();
                if total_valid > MAX_CANDIDATES {
                    return Err(CompositionError::AutocompleteOverCap {
                        query: query.to_string(),
                        cap: MAX_CANDIDATES,
                    });
                }
                collected.extend(paths);
            }
            WalkOutcome::OverCapacity(_) => {
                return Err(CompositionError::AutocompleteOverCap {
                    query: query.to_string(),
                    cap: MAX_CANDIDATES,
                });
            }
        }
    }

    // Two-key dedup, applied *before* the alphabetical sort so the
    // priority order of `iter_scopes` (repo → area → package →
    // repo_claudine → user_claudine → extras) still decides which
    // same-named prompt survives: the first (most-local) one wins.
    //
    //  1. drop exact canonical-path duplicates (a scope symlinked into
    //     another surfaces the same file twice), and
    //  2. drop later candidates whose lowercased file stem was already
    //     seen from an earlier, more-local scope — this suppresses the
    //     stale global `~/.claudine/prompts/plan.md` when the repo's own
    //     `prompts/plan.md` is present.
    let mut seen_canonical: HashSet<PathBuf> = HashSet::new();
    let mut seen_stems: HashSet<String> = HashSet::new();
    collected.retain(|path| {
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
        if !seen_canonical.insert(canonical) {
            return false;
        }
        match path.file_stem().and_then(|s| s.to_str()) {
            Some(stem) => seen_stems.insert(stem.to_ascii_lowercase()),
            None => true,
        }
    });

    collected.sort_by_key(|a| a.display().to_string());

    if collected.is_empty() {
        return Err(CompositionError::AutocompleteNoMatches {
            query: query.to_string(),
        });
    }

    Ok(collected)
}

/// Present one or more candidates and return the user's selection.
fn present_and_select(
    candidates: Vec<PathBuf>,
    query: &str,
    mode: ComposeMode,
    ctx: &ScopeContext,
) -> Result<String, CompositionError> {
    let badge = badge_for_mode(mode);
    let (options, insert_map) = build_options(&candidates, badge, ctx);

    let selected_detail = match options.len() {
        1 => {
            let detail = &options[0].value;
            match confirm_one_file(detail) {
                Ok(true) => Some(detail.clone()),
                Ok(false) => None,
                Err(_) => return Err(CompositionError::AutocompleteNotInteractive),
            }
        }
        _ => match choose_one_file(options) {
            Ok(Some(detail)) => Some(detail),
            Ok(None) => None,
            Err(_) => return Err(CompositionError::AutocompleteNotInteractive),
        },
    };

    let Some(detail) = selected_detail else {
        return Err(CompositionError::AutocompleteCancelled {
            query: query.to_string(),
        });
    };

    let insert = insert_map
        .get(&detail.path)
        .cloned()
        .unwrap_or_else(|| format_relative_insert(&detail.path, ctx));
    Ok(insert)
}

fn is_interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
}

fn badge_for_mode(mode: ComposeMode) -> &'static str {
    match mode {
        ComposeMode::Compose => "COMPOSE",
        ComposeMode::InlineCompose => "INLINE_COMPOSE",
        ComposeMode::Sequence => "SEQUENCE",
    }
}

fn build_options(
    paths: &[PathBuf],
    badge: &str,
    ctx: &ScopeContext,
) -> (Vec<ChoiceOption<FileDetail>>, HashMap<PathBuf, String>) {
    let mut options: Vec<ChoiceOption<FileDetail>> = Vec::new();
    let mut insert_map: HashMap<PathBuf, String> = HashMap::new();

    for path in paths {
        let detail = if is_yaml_sequence(path) {
            extract_yaml_sequence_detail(path, badge)
        } else {
            extract_markdown_detail(path, badge)
        };
        let insert = format_relative_insert(path, ctx);
        insert_map.insert(detail.path.clone(), insert.clone());
        options.push(ChoiceOption::new(insert.clone(), insert, detail));
    }

    (options, insert_map)
}

fn is_yaml_sequence(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "yaml" | "yml"))
        .unwrap_or(false)
}

/// Render a runnable file reference from an absolute candidate path.
///
/// Mirrors the TAB-path contract: repo-root-relative when possible,
/// `~`-relative for user-global scopes, and absolute as a last resort.
fn format_relative_insert(path: &Path, ctx: &ScopeContext) -> String {
    if let Some(root) = scopes::effective_repo_root(ctx)
        && let Ok(rel) = path.strip_prefix(root)
        && !rel.as_os_str().is_empty()
    {
        return to_portable_string(rel);
    }

    if let Some(home) = &ctx.home
        && path.starts_with(home)
        && let Ok(rel) = path.strip_prefix(home)
        && !rel.as_os_str().is_empty()
    {
        return format!("~/{}", to_portable_string(rel));
    }

    to_portable_string(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn seed_repo(root: &Path) {
        fs::create_dir_all(root.join(".git")).unwrap();
    }

    fn seed_cargo_workspace(root: &Path, members: &[&str]) {
        seed_repo(root);
        let members_list = members
            .iter()
            .map(|m| format!("    \"{m}\""))
            .collect::<Vec<_>>()
            .join(",\n");
        let manifest = format!("[workspace]\nresolver = \"2\"\nmembers = [\n{members_list}\n]\n");
        fs::write(root.join("Cargo.toml"), manifest).unwrap();
        for member in members {
            let dir = root.join(member);
            fs::create_dir_all(dir.join("src")).unwrap();
            let name = member.replace('/', "-");
            fs::write(
                dir.join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
            )
            .unwrap();
            fs::write(dir.join("src").join("lib.rs"), "").unwrap();
        }
    }

    #[test]
    fn gather_candidates_filters_by_query() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("plan.md"), "---\ntitle: X\n---\nBody\n");
        write(&prompts.join("notes.md"), "---\ntitle: Y\n---\nBody\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = gather_candidates("plan", ComposeMode::Compose, &ctx).unwrap();
        assert!(
            got.iter().any(|p| p.ends_with("plan.md")),
            "plan.md must match query: {got:?}"
        );
        assert!(
            !got.iter().any(|p| p.ends_with("notes.md")),
            "notes.md must not match query: {got:?}"
        );
    }

    #[test]
    fn gather_candidates_zero_matches_errors() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("plan.md"), "---\ntitle: X\n---\nBody\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let err = gather_candidates("nothing", ComposeMode::Compose, &ctx).unwrap_err();
        assert!(matches!(err, CompositionError::AutocompleteNoMatches { query } if query == "nothing"));
    }

    #[test]
    fn gather_candidates_applies_mode_contract() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("plain.md"), "---\ntitle: X\n---\nBody\n");
        write(&prompts.join("inline.md"), "---\nprompt: Write\n---\nBody\n");

        let ctx = ScopeContext::discover_from(tmp.path());

        let compose = gather_candidates("", ComposeMode::Compose, &ctx).unwrap();
        assert!(compose.iter().any(|p| p.ends_with("plain.md")));
        assert!(!compose.iter().any(|p| p.ends_with("inline.md")));

        let inline = gather_candidates("", ComposeMode::InlineCompose, &ctx).unwrap();
        assert!(inline.iter().any(|p| p.ends_with("inline.md")));
        assert!(!inline.iter().any(|p| p.ends_with("plain.md")));
    }

    #[test]
    fn gather_candidates_includes_yaml_sequence_files() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let prompts = tmp.path().join("prompts");
        write(&prompts.join("steps.yaml"), "sequence:\n  - one\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = gather_candidates("steps", ComposeMode::Sequence, &ctx).unwrap();
        assert_eq!(got.len(), 1);
        assert!(got[0].ends_with("steps.yaml"));
    }

    #[test]
    fn gather_candidates_over_cap_reports_narrow_query() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let prompts = tmp.path().join("prompts");
        for i in 0..MAX_CANDIDATES + 1 {
            write(&prompts.join(format!("plan{i}.md")), "# x\n");
        }

        let ctx = ScopeContext::discover_from(tmp.path());
        let err = gather_candidates("plan", ComposeMode::Compose, &ctx).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::AutocompleteOverCap { query, cap }
            if query == "plan" && cap == MAX_CANDIDATES
        ));
    }

    #[test]
    fn gather_candidates_mode_filter_counts_not_raw_query_matches() {
        // Regression: more than MAX_CANDIDATES files may match the query
        // substring while fewer than MAX_CANDIDATES satisfy the active mode.
        // The cap must count only valid candidates.
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let prompts = tmp.path().join("prompts");

        // MAX_CANDIDATES + 1 query matches that are invalid for Compose
        // because they carry a `prompt:` key.
        for i in 0..MAX_CANDIDATES + 1 {
            write(
                &prompts.join(format!("plan{i}.md")),
                "---\nprompt: Do something\n---\nBody\n",
            );
        }

        // One query match that is valid for Compose.
        write(
            &prompts.join("plan_valid.md"),
            "---\ntitle: Valid\n---\nBody\n",
        );

        let ctx = ScopeContext::discover_from(tmp.path());
        let got = gather_candidates("plan", ComposeMode::Compose, &ctx).unwrap();
        assert!(
            got.iter().any(|p| p.ends_with("plan_valid.md")),
            "missing plan_valid.md: {got:?}"
        );
        // The key regression assertion: with >500 query matches but only one
        // valid compose candidate, the function must return the valid subset
        // rather than AutocompleteOverCap.
        assert!(
            got.len() <= MAX_CANDIDATES,
            "valid candidate count must not exceed cap: {got:?}"
        );
    }

    #[test]
    fn gather_candidates_over_cap_counts_valid_candidates() {
        // The cap must still fire when the number of *valid* candidates
        // exceeds MAX_CANDIDATES, even if extra query matches are invalid
        // for the active mode.
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let prompts = tmp.path().join("prompts");

        for i in 0..MAX_CANDIDATES + 1 {
            write(
                &prompts.join(format!("valid_plan{i}.md")),
                "---\ntitle: Valid\n---\nBody\n",
            );
        }
        for i in 0..50 {
            write(
                &prompts.join(format!("invalid_plan{i}.md")),
                "---\nprompt: Do something\n---\nBody\n",
            );
        }

        let ctx = ScopeContext::discover_from(tmp.path());
        let err = gather_candidates("plan", ComposeMode::Compose, &ctx).unwrap_err();
        assert!(matches!(
            err,
            CompositionError::AutocompleteOverCap { query, cap }
            if query == "plan" && cap == MAX_CANDIDATES
        ));
    }

    #[test]
    fn gather_candidates_dedups_same_stem_most_local_wins() {
        // Repo `prompts/plan.md` and a stale user-global
        // `~/.claudine/prompts/plan.md` both match the query. The repo
        // (most-local) copy must win; the global copy is suppressed.
        let repo = TempDir::new().unwrap();
        seed_repo(repo.path());
        write(
            &repo.path().join("prompts").join("plan.md"),
            "---\ntitle: Repo\n---\nRepo body\n",
        );

        let user_home = TempDir::new().unwrap();
        write(
            &user_home
                .path()
                .join(".claudine")
                .join("prompts")
                .join("plan.md"),
            "---\ntitle: Global\n---\nGlobal body\n",
        );

        let mut ctx = ScopeContext::discover_from(repo.path());
        ctx.home = Some(user_home.path().to_path_buf());

        let got = gather_candidates("plan", ComposeMode::Compose, &ctx).unwrap();
        assert_eq!(got.len(), 1, "same-stem prompts must collapse to one: {got:?}");
        assert!(
            got[0].starts_with(repo.path()),
            "the repo (most-local) plan.md must win: {got:?}"
        );
    }

    #[test]
    fn format_relative_insert_prefers_repo_root() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let path = tmp.path().join("prompts").join("plan.md");
        write(&path, "# x\n");

        let ctx = ScopeContext::discover_from(tmp.path());
        assert_eq!(
            format_relative_insert(&path, &ctx),
            "prompts/plan.md"
        );
    }

    #[test]
    fn format_relative_insert_portably_renders_windows_shaped_segments() {
        let root = PathBuf::from("repo");
        let path = root.join(r"prompts\nested\plan.md");
        let ctx = ScopeContext {
            cwd: root.clone(),
            home: None,
            repo_info: None,
            git_root: Some(root),
        };

        assert_eq!(
            format_relative_insert(&path, &ctx),
            "prompts/nested/plan.md"
        );
    }

    #[test]
    fn build_options_uses_portable_relative_path_as_visible_label() {
        let tmp = TempDir::new().unwrap();
        seed_cargo_workspace(tmp.path(), &["a/lib"]);
        let path = tmp.path().join("prompts").join(r"nested\plan.md");
        write(&path, "# x\n");
        let ctx = ScopeContext::discover_from(tmp.path());

        let (options, _) = build_options(&[path], "COMPOSE", &ctx);
        assert_eq!(options[0].label, "prompts/nested/plan.md");
    }
}
