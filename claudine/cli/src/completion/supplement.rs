//! Supplement completion engine (feature `2026-04-18-file-completion-supplement`).
//!
//! This module owns the new, hidden `claudine __complete` code path. It is a
//! **parallel** engine: the older `CompleteEnv`-driven path in
//! [`super::command_factory`] stays compilable and behaviorally unchanged for
//! stale installed bootstrap scripts. Newly generated shell scripts (bash,
//! zsh, fish) shell out to `__complete` so they reach the supplement contract.
//!
//! The supplement rules enforced here, in order:
//!
//! 1. **Trigger matrix** — completion fires only at the positional `<FILE>`
//!    slot of `compose`, `inline-compose`, `sequence`, and at the value slot
//!    of `--append-system-prompt` / `--asp` / `--replace-system-prompt` /
//!    `--rsp` on those three plus every wrapper subcommand
//!    (`claude`, `codex`, `gemini`, `goose`, `kimi`, `opencode`, `qwen`).
//!    Anything else returns an empty candidate list so static clap completion
//!    takes over.
//! 2. **Entry forms** — only the two forms supported by
//!    [`biscuit_file::FileReference::complete_partial`]: `@`-prefixed magic
//!    paths and implicit-relative paths.
//! 3. **Typed-length scope** — 0-2 meaningful characters use the curated
//!    scope only; 3+ characters additionally walk the enclosing git repo via
//!    [`sniff::filesystem::docs::collect_markdown_paths`]. Meaningful
//!    characters are counted from the **active segment** (the portion after
//!    the last `/`, excluding a leading `@`).
//! 4. **Markdown-only** — every emitted candidate is a `*.md` file; no
//!    directory candidates, no `!` package sigil, no `./` / `../` UI.
//! 5. **Matching** — case-insensitive substring on the filename with the
//!    `.md` extension stripped for matching only. Directory components are
//!    never considered.
//! 6. **Dedup** — candidates whose canonical resolved paths collide are
//!    emitted at most once.
//!
//! The engine never walks a bespoke ignore list. Curated roots and the broad
//! scan both go through `sniff::collect_markdown_paths`, which reuses the
//! authoritative `.gitignore`-aware `ignore::WalkBuilder` configuration.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use biscuit_file::{CompletionEntryForm, FileReference, PartialCompletion};
use sniff::filesystem::docs::collect_markdown_paths;
use sniff::filesystem::repo::detect_repo_structure;

/// Composition subcommands that accept a positional `<FILE>` argument.
const COMPOSITION_SUBCOMMANDS: &[&str] = &["compose", "inline-compose", "sequence"];

/// Subcommands that accept `--append-system-prompt` / `--replace-system-prompt`
/// (plus their `--asp` / `--rsp` aliases). Derived from the clap surface in
/// [`crate::commands::wrap::WrapperArgs`] and the composition `SharedComposeArgs`.
const FILE_FLAG_SUBCOMMANDS: &[&str] = &[
    "compose",
    "inline-compose",
    "sequence",
    "claude",
    "codex",
    "gemini",
    "goose",
    "kimi",
    "opencode",
    "qwen",
];

/// Flags whose value slot triggers supplement file completion.
const FILE_FLAGS: &[&str] = &[
    "--append-system-prompt",
    "--asp",
    "--replace-system-prompt",
    "--rsp",
];

/// Value-bearing flags whose value must be skipped when scanning for
/// positional-argument position. Mirrors
/// [`crate::argv::COMPOSITION_FLAGS_WITH_VALUE`] but is duplicated here so
/// the supplement engine stays self-contained.
const VALUE_BEARING_FLAGS: &[&str] = &[
    "--provider",
    "--exclude",
    "--include",
    "--model",
    "-m",
    "--output",
    "-o",
    "--append-system-prompt",
    "--asp",
    "--replace-system-prompt",
    "--rsp",
    "--timeout",
    "-t",
    "--operation",
    "--op",
    "--set",
    "--use",
    "--fail-fast",
];

/// Curated subdirectories under each scope root when the token does not
/// commit to a specific subdirectory (the active scope is empty).
const CURATED_SUBDIRS: &[&str] = &["prompts", "sequences"];

/// Run the supplement engine against a full argv and a cursor index.
///
/// Returns the candidate strings to emit on stdout (one per line). An empty
/// return value means "no custom candidates" — the caller should fall back to
/// whatever static completion clap generated.
pub(crate) fn run(argv: &[String], current_index: usize) -> Vec<String> {
    if classify_completion_target(argv, current_index).is_none() {
        return Vec::new();
    }

    let partial: &str = argv.get(current_index).map(String::as_str).unwrap_or("");
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    emit_candidates(partial, &cwd)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionTarget {
    Positional,
    FileFlag,
}

/// Decide whether the token at `current_index` is a completion target.
///
/// The classifier is intentionally self-contained: it does not invoke clap.
/// Wrapper subcommands use `ignore_errors(true)` in the lenient parse path,
/// which means the clap-derived view of argv is unreliable for the supplement
/// contract; a manual scan is cheaper and more correct.
fn classify_completion_target(argv: &[String], current_index: usize) -> Option<CompletionTarget> {
    if current_index >= argv.len() {
        // Cursor can legitimately sit one past the last argv element (the
        // user is typing a new trailing token); we still need the
        // subcommand context to decide whether completion fires.
        return classify_completion_target_relaxed(argv, current_index);
    }
    classify_completion_target_relaxed(argv, current_index)
}

fn classify_completion_target_relaxed(
    argv: &[String],
    current_index: usize,
) -> Option<CompletionTarget> {
    let sub_idx = find_subcommand_index(argv)?;
    let sub = argv.get(sub_idx)?.as_str();

    if !FILE_FLAG_SUBCOMMANDS.contains(&sub) {
        return None;
    }

    if current_index == 0 || current_index <= sub_idx {
        return None;
    }

    // File-flag value slot: the previous argv token is exactly a file flag.
    if current_index >= sub_idx + 2 {
        let prev = argv.get(current_index - 1).map(String::as_str).unwrap_or("");
        if FILE_FLAGS.contains(&prev) {
            return Some(CompletionTarget::FileFlag);
        }
    }

    // Wrappers only accept the file-flag target; no positional completion.
    if !COMPOSITION_SUBCOMMANDS.contains(&sub) {
        return None;
    }

    // Positional target: walk tokens between the subcommand and the cursor.
    // If we have not yet seen a positional (ignoring flags, flag values, and
    // key=value setters), the cursor slot IS the first positional.
    let mut cursor = sub_idx + 1;
    let mut seen_positional = false;
    while cursor < current_index {
        let token = argv[cursor].as_str();
        if token == "--" {
            cursor += 1;
            continue;
        }
        if looks_like_flag(token) {
            if VALUE_BEARING_FLAGS.contains(&token) && !token.contains('=') {
                cursor += 2;
            } else {
                cursor += 1;
            }
            continue;
        }
        if is_setter_shaped(token) {
            cursor += 1;
            continue;
        }
        seen_positional = true;
        cursor += 1;
    }

    if seen_positional {
        return None;
    }

    // Reject setter-shaped tokens AT the cursor — they are not file refs.
    let current = argv.get(current_index).map(String::as_str).unwrap_or("");
    if is_setter_shaped(current) {
        return None;
    }

    Some(CompletionTarget::Positional)
}

/// Locate the first subcommand token in argv, skipping global flags.
fn find_subcommand_index(argv: &[String]) -> Option<usize> {
    let mut i = 1;
    while i < argv.len() {
        let token = argv[i].as_str();
        if token == "--" {
            return None;
        }
        if token == "--debug" {
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
        if looks_like_flag(token) {
            i += 1;
            continue;
        }
        return Some(i);
    }
    None
}

fn is_global_bool_flag(token: &str) -> bool {
    matches!(
        token,
        "--plain" | "--verbose" | "-v" | "-vv" | "-vvv" | "--help" | "-h"
    )
}

fn looks_like_flag(token: &str) -> bool {
    token.starts_with('-') && token != "-" && token != "--"
}

/// `^[A-Za-z_][A-Za-z0-9_-]*=` — same shape as
/// [`crate::argv::looks_like_setter`]. Duplicated here to keep the supplement
/// engine self-contained.
fn is_setter_shaped(token: &str) -> bool {
    let Some(eq_pos) = token.find('=') else {
        return false;
    };
    if eq_pos == 0 {
        return false;
    }
    let bytes = token.as_bytes();
    if !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'_') {
        return false;
    }
    bytes[1..eq_pos]
        .iter()
        .all(|&c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

/// Top-level emission pipeline: parse the partial, compute roots, walk, match,
/// dedup, render.
pub(crate) fn emit_candidates(partial: &str, cwd: &Path) -> Vec<String> {
    let Ok(Some(completion)) = FileReference::complete_partial(partial, cwd) else {
        return Vec::new();
    };

    let form = completion.entry_form();
    let active = completion.active_segment();
    let enclosing_repo = find_enclosing_repo(cwd);
    let home = dirs::home_dir();

    let roots = curated_roots(&completion, cwd, enclosing_repo.as_deref(), home.as_deref());

    let mut discovered: Vec<PathBuf> = Vec::new();
    for root in &roots {
        for path in collect_markdown_paths(root) {
            discovered.push(path);
        }
    }

    let active_lower = active.to_ascii_lowercase();
    let mut seen_canonical: HashSet<PathBuf> = HashSet::new();
    let mut rendered: BTreeSet<String> = BTreeSet::new();

    for path in discovered {
        let Some(filename) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !has_markdown_extension(filename) {
            continue;
        }
        let stem_lower = strip_md_extension(&filename.to_ascii_lowercase()).to_string();
        if !stem_lower.contains(&active_lower) {
            continue;
        }
        let canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if !seen_canonical.insert(canonical) {
            continue;
        }
        if let Some(value) = render_candidate(
            &path,
            form,
            cwd,
            enclosing_repo.as_deref(),
            home.as_deref(),
        ) {
            rendered.insert(value);
        }
    }

    rendered.into_iter().collect()
}

/// Compute the curated scope roots that should be enumerated for this
/// partial.
///
/// The engine searches exactly three curated scope bases, each joined with
/// the curated subdirs (`prompts/`, `sequences/`) when the user has not
/// committed to a subdirectory, or with the active scope when they have:
///
/// 1. **Repo root** — `<repo>/{prompts,sequences}/`
/// 2. **Package-area root** — `<repo>/<area>/{prompts,sequences}/` (where
///    `<area>` is the enclosing package area from `sniff`, excluding `root`)
/// 3. **User scope** — `~/.claudine/{prompts,sequences}/`
///
/// No per-package roots (those would be too narrow and noisy in monorepos),
/// no raw `$HOME/` roots, and no broad repo scan. If the user has committed
/// to a subdirectory via `/` in an `ImplicitRelative` form, only the repo
/// scope is searched (matches the `prompts/<tab>` semantics — repo-local).
fn curated_roots(
    completion: &PartialCompletion,
    cwd: &Path,
    enclosing_repo: Option<&Path>,
    home: Option<&Path>,
) -> Vec<PathBuf> {
    let form = completion.entry_form();
    let rendered_prefix = completion.rendered_prefix();
    let scope = match form {
        CompletionEntryForm::Magic => rendered_prefix.strip_prefix('@').unwrap_or(rendered_prefix),
        CompletionEntryForm::ImplicitRelative => rendered_prefix,
    };
    let scope_trimmed = scope.trim_end_matches('/');

    let mut roots: Vec<PathBuf> = Vec::new();

    let include_user_scope = match form {
        CompletionEntryForm::Magic => true,
        CompletionEntryForm::ImplicitRelative => scope_trimmed.is_empty(),
    };

    let push_scope = |roots: &mut Vec<PathBuf>, base: &Path| {
        if scope_trimmed.is_empty() {
            for sub in CURATED_SUBDIRS {
                roots.push(base.join(sub));
            }
        } else {
            roots.push(base.join(scope_trimmed));
        }
    };

    if let Some(repo_root) = enclosing_repo {
        push_scope(&mut roots, repo_root);
        if let Ok(Some(info)) = detect_repo_structure(repo_root)
            && let Some(area) = info.package_area_for_dir(cwd)
            && area != "root"
        {
            let area_root = repo_root.join(area);
            push_scope(&mut roots, &area_root);
        }
    }

    if include_user_scope
        && let Some(home_dir) = home
    {
        let claudine = home_dir.join(".claudine");
        push_scope(&mut roots, &claudine);
    }

    // Deduplicate — repo-scope and area-scope can coincide when cwd is at
    // the repo root (area == repo).
    let mut seen: HashSet<PathBuf> = HashSet::new();
    roots.retain(|r| seen.insert(r.clone()));
    roots
}

/// Walk upward from `cwd` until a `.git` entry is found.
///
/// Both a `.git` directory (standard checkout) and a `.git` file (worktree
/// pointer) count as repo roots. Missing the file branch would silently
/// skip git worktrees — the very environment Claudine development runs in.
fn find_enclosing_repo(cwd: &Path) -> Option<PathBuf> {
    for ancestor in cwd.ancestors() {
        let dot_git = ancestor.join(".git");
        if dot_git.is_dir() || dot_git.is_file() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn has_markdown_extension(filename: &str) -> bool {
    filename.to_ascii_lowercase().ends_with(".md")
}

fn strip_md_extension(lower_filename: &str) -> &str {
    lower_filename.strip_suffix(".md").unwrap_or(lower_filename)
}

/// Render the candidate string the shell should insert.
///
/// Rules:
///
/// - Magic form: prefix with `@`, body is the path relative to the git root
///   (preferred) or the user home. Files under neither are dropped.
/// - ImplicitRelative form: body is the path relative to cwd or git root (in
///   that order). Files that fall outside both are rendered with the Magic
///   `@`-prefixed form against the user home so they still resolve.
fn render_candidate(
    path: &Path,
    form: CompletionEntryForm,
    cwd: &Path,
    git_root: Option<&Path>,
    home: Option<&Path>,
) -> Option<String> {
    match form {
        CompletionEntryForm::Magic => {
            if let Some(root) = git_root
                && let Ok(rel) = path.strip_prefix(root)
            {
                return rel.to_str().map(|s| format!("@{s}"));
            }
            if let Some(home_dir) = home
                && let Ok(rel) = path.strip_prefix(home_dir)
            {
                return rel.to_str().map(|s| format!("@{s}"));
            }
            None
        }
        CompletionEntryForm::ImplicitRelative => {
            if let Ok(rel) = path.strip_prefix(cwd) {
                return rel.to_str().map(|s| s.to_string());
            }
            if let Some(root) = git_root
                && let Ok(rel) = path.strip_prefix(root)
            {
                return rel.to_str().map(|s| s.to_string());
            }
            if let Some(home_dir) = home
                && let Ok(rel) = path.strip_prefix(home_dir)
            {
                return rel.to_str().map(|s| format!("@{s}"));
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::TempDir;

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    // -- classifier ------------------------------------------------------

    fn argv(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn classifier_detects_positional_on_compose() {
        let a = argv(&["claudine", "compose", ""]);
        assert_eq!(
            classify_completion_target(&a, 2),
            Some(CompletionTarget::Positional)
        );
    }

    #[test]
    fn classifier_detects_positional_on_sequence() {
        let a = argv(&["claudine", "sequence", "@pr"]);
        assert_eq!(
            classify_completion_target(&a, 2),
            Some(CompletionTarget::Positional)
        );
    }

    #[test]
    fn classifier_skips_positional_after_first_has_been_seen() {
        let a = argv(&["claudine", "compose", "file.md", "more"]);
        assert_eq!(classify_completion_target(&a, 3), None);
    }

    #[test]
    fn classifier_skips_setter_token_at_cursor() {
        let a = argv(&["claudine", "compose", "key=val"]);
        assert_eq!(classify_completion_target(&a, 2), None);
    }

    #[test]
    fn classifier_skips_setter_earlier_still_detects_positional() {
        // Setter before cursor isn't a positional, so cursor is still the first positional.
        let a = argv(&["claudine", "compose", "key=val", "@pro"]);
        assert_eq!(
            classify_completion_target(&a, 3),
            Some(CompletionTarget::Positional)
        );
    }

    #[test]
    fn classifier_detects_file_flag_value_slot_on_compose() {
        let a = argv(&["claudine", "compose", "--asp", ""]);
        assert_eq!(
            classify_completion_target(&a, 3),
            Some(CompletionTarget::FileFlag)
        );
    }

    #[test]
    fn classifier_detects_file_flag_value_slot_on_wrapper() {
        let a = argv(&["claudine", "claude", "--append-system-prompt", ""]);
        assert_eq!(
            classify_completion_target(&a, 3),
            Some(CompletionTarget::FileFlag)
        );
    }

    #[test]
    fn classifier_ignores_wrapper_positional() {
        let a = argv(&["claudine", "claude", ""]);
        assert_eq!(classify_completion_target(&a, 2), None);
    }

    #[test]
    fn classifier_ignores_non_targeted_subcommand() {
        let a = argv(&["claudine", "hooks", ""]);
        assert_eq!(classify_completion_target(&a, 2), None);
    }

    #[test]
    fn classifier_skips_value_of_preceding_value_bearing_flag() {
        // `--model gpt-4` consumes its value, so cursor after it is
        // STILL the first positional.
        let a = argv(&["claudine", "compose", "--model", "gpt-4", ""]);
        assert_eq!(
            classify_completion_target(&a, 4),
            Some(CompletionTarget::Positional)
        );
    }

    #[test]
    fn classifier_detects_positional_after_global_flag() {
        let a = argv(&["claudine", "--plain", "compose", ""]);
        assert_eq!(
            classify_completion_target(&a, 3),
            Some(CompletionTarget::Positional)
        );
    }

    // -- emit_candidates end-to-end -------------------------------------

    #[test]
    fn empty_input_without_repo_returns_empty_when_home_has_no_prompts() {
        let tmp = TempDir::new().unwrap();
        // cwd is the tmp dir, which is not a git repo. No curated dirs
        // exist under it. We can't easily seed HOME here without a
        // full env manipulation, so we just verify that we do not
        // panic or surface unrelated matches from a bare cwd.
        let candidates = emit_candidates("", tmp.path());
        // Emission from the real user HOME may produce matches; just
        // assert none of them reach outside the supported entry forms.
        for c in &candidates {
            assert!(
                c.starts_with('@') || !c.starts_with('/'),
                "bare path candidate not allowed: {c}"
            );
        }
    }

    #[test]
    fn magic_matches_curated_prompts_inside_fake_repo() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        write_file(&tmp.path().join("prompts").join("prompt.md"), "# p");
        write_file(&tmp.path().join("prompts").join("my-prompt.md"), "# m");
        write_file(&tmp.path().join("prompts").join("suppress.md"), "# s");
        write_file(&tmp.path().join("docs").join("guide.md"), "# g"); // not matched at 2 chars

        let got = emit_candidates("@pr", tmp.path());
        assert!(
            got.iter().any(|c| c == "@prompts/prompt.md"),
            "expected @prompts/prompt.md in {got:?}",
        );
        assert!(
            got.iter().any(|c| c == "@prompts/my-prompt.md"),
            "expected @prompts/my-prompt.md in {got:?}",
        );
        assert!(
            got.iter().any(|c| c == "@prompts/suppress.md"),
            "expected @prompts/suppress.md in {got:?}",
        );
    }

    #[test]
    fn non_curated_locations_are_never_searched() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        // Non-curated location — must NEVER surface regardless of typed
        // length. The engine only walks prompts/ and sequences/.
        write_file(&tmp.path().join("random").join("process.md"), "# p");

        for partial in &["@pr", "@pro", "@process", "process"] {
            let got = emit_candidates(partial, tmp.path());
            assert!(
                !got.iter().any(|c| c.contains("random/process.md")),
                "non-curated path must not surface for {partial:?}: {got:?}"
            );
        }
    }

    #[test]
    fn implicit_relative_scope_is_repo_only() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        write_file(&tmp.path().join("prompts").join("a.md"), "# a");

        let got = emit_candidates("prompts/", tmp.path());
        assert!(
            got.iter().any(|c| c == "prompts/a.md"),
            "expected implicit-relative repo match in {got:?}"
        );
        // No @ prefix for repo matches in ImplicitRelative form.
        assert!(
            got.iter().all(|c| !c.starts_with('@') || !c.contains("prompts/a.md")),
            "implicit-relative repo match must not gain an @ prefix: {got:?}"
        );
    }

    #[test]
    fn mid_filename_substring_match_fires() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        write_file(&tmp.path().join("prompts").join("prompt.md"), "# p");
        let got = emit_candidates("@omp", tmp.path());
        assert!(
            got.iter().any(|c| c == "@prompts/prompt.md"),
            "mid-filename substring match failed: {got:?}"
        );
    }

    #[test]
    fn non_markdown_files_never_surface() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        write_file(&tmp.path().join("prompts").join("plain.txt"), "t");
        write_file(&tmp.path().join("prompts").join("code.rs"), "fn main(){}");
        let got = emit_candidates("@", tmp.path());
        assert!(
            !got.iter().any(|c| c.ends_with(".txt")),
            "non-markdown candidate leaked: {got:?}"
        );
        assert!(
            !got.iter().any(|c| c.ends_with(".rs")),
            "non-markdown candidate leaked: {got:?}"
        );
    }

    #[test]
    fn gitignored_files_excluded_from_curated_scan() {
        let tmp = TempDir::new().unwrap();
        // `ignore::WalkBuilder` (the crate backing
        // `sniff::collect_markdown_paths`) uses the presence of a `.git`
        // directory to locate gitignore files. An empty stub suffices.
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        write_file(&tmp.path().join(".gitignore"), "prompts/ignored/\n");
        write_file(
            &tmp.path().join("prompts").join("ignored").join("secret.md"),
            "# s",
        );
        write_file(&tmp.path().join("prompts").join("kept.md"), "# k");

        let got = emit_candidates("@", tmp.path());
        assert!(
            got.iter().any(|c| c == "@prompts/kept.md"),
            "non-ignored curated match must surface: {got:?}"
        );
        assert!(
            !got.iter().any(|c| c.contains("ignored/secret.md")),
            "gitignored match must be excluded: {got:?}"
        );
    }

    #[test]
    fn no_repo_fallback_skips_repo_scope() {
        let tmp = TempDir::new().unwrap();
        // No .git directory — cwd is not inside a repo.
        write_file(&tmp.path().join("other").join("process.md"), "# p");
        write_file(&tmp.path().join("prompts").join("process.md"), "# p");
        let got = emit_candidates("@pro", tmp.path());
        // Without a repo root, neither the non-curated file nor the
        // cwd-local prompts/ dir should surface — repo scope only applies
        // when cwd is inside a git repository.
        assert!(
            !got.iter().any(|c| c.contains("other/process.md")),
            "no-repo fallback must not walk arbitrary dirs: {got:?}"
        );
        assert!(
            !got.iter().any(|c| c.contains("prompts/process.md")),
            "no-repo fallback must not walk the cwd prompts/ dir: {got:?}"
        );
    }

    #[test]
    fn unsupported_form_returns_empty() {
        let tmp = TempDir::new().unwrap();
        assert!(emit_candidates("!pkg", tmp.path()).is_empty());
        assert!(emit_candidates("vault:x", tmp.path()).is_empty());
        assert!(emit_candidates("./local", tmp.path()).is_empty());
        assert!(emit_candidates("/abs/path", tmp.path()).is_empty());
    }
}
