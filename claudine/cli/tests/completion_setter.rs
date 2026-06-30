//! Integration tests for `claudine <compose|inline-compose|sequence>
//! name=@<TAB>` setter-value completion.
//!
//! Phase 4 of the `2026-04-24-improved-shell-completions` feature. These
//! tests drive the compiled `claudine` binary's hidden `__complete`
//! subcommand against seeded temp-directory fixtures and assert that the
//! setter-value completer's contract holds end-to-end:
//!
//! - Only `@`-prefixed values trigger file completion; all other values
//!   return zero candidates so the shell falls back to its default.
//! - Emitted candidates always wrap the value in `'...'` — a user-typed
//!   opening `"` is normalized to `'`.
//! - Scope includes `docs/`, `features/`, `fixes/`, and `reviews/` under
//!   the invoking `cwd` (the launch area), rendered cwd-relative.
//! - Scope works on all three composition subcommands (`compose`,
//!   `inline-compose`, `sequence`).
//! - A setter-shaped cursor wins over the positional classification
//!   even when the file slot has already been committed.

mod common;
use std::fs;

use common::TestWorkspace;
use common::completion::{
    run_complete, seed_cargo_workspace_members as seed_cargo_workspace, seed_plain_git_repo,
    write_file,
};

// ---------------------------------------------------------------------
// @ trigger — file completion fires only for `@`-prefixed values
// ---------------------------------------------------------------------

#[test]
fn setter_at_sigil_triggers_file_completion() {
    let ws = TestWorkspace::named("complete-setter-at-trigger");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join("docs").join("spec.md"), "# s\n");

    let got = run_complete(ws.path(), &["compose", "foo.md", "spec=@s"]);
    assert!(
        got.iter().any(|c| c == "spec='docs/spec.md'"),
        "expected spec='docs/spec.md' in {got:?}"
    );
}

#[test]
fn setter_non_at_value_returns_no_candidates() {
    let ws = TestWorkspace::named("complete-setter-non-at");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join("docs").join("spec.md"), "# s\n");

    // Even though `docs/spec.md` exists and would match `@s`, a literal
    // non-`@` value returns zero candidates — the setter-value completer
    // declines to offer file suggestions when the user has not opted in
    // via the `@` sigil.
    let got = run_complete(ws.path(), &["compose", "foo.md", "spec=bar"]);
    assert!(
        got.is_empty(),
        "non-@ value must produce zero candidates: {got:?}"
    );
}

#[test]
fn setter_empty_value_returns_no_candidates() {
    let ws = TestWorkspace::named("complete-setter-empty-value");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join("docs").join("spec.md"), "# s\n");

    let got = run_complete(ws.path(), &["compose", "foo.md", "spec="]);
    assert!(
        got.is_empty(),
        "empty value must produce zero candidates: {got:?}"
    );
}

// ---------------------------------------------------------------------
// Quote normalization
// ---------------------------------------------------------------------

#[test]
fn setter_double_quote_is_normalized_to_single_quote() {
    let ws = TestWorkspace::named("complete-setter-double-quote");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join("docs").join("spec.md"), "# s\n");

    // Unclosed `"` — the shell passes the token through verbatim. The
    // completer must strip the opening `"` for classification and wrap
    // the emitted value in `'...'`.
    let got = run_complete(ws.path(), &["compose", "foo.md", "spec=\"@s"]);
    assert!(
        got.iter().all(|c| !c.contains('"')),
        "double quote must be normalized away: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "spec='docs/spec.md'"),
        "expected spec='docs/spec.md' after quote normalization: {got:?}"
    );
}

#[test]
fn setter_single_quote_round_trips_to_single_quote() {
    let ws = TestWorkspace::named("complete-setter-single-quote");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join("docs").join("spec.md"), "# s\n");

    let got = run_complete(ws.path(), &["compose", "foo.md", "spec='@s"]);
    assert!(
        got.iter().any(|c| c == "spec='docs/spec.md'"),
        "expected spec='docs/spec.md' in {got:?}"
    );
}

// ---------------------------------------------------------------------
// Scope resolution — docs / features / fixes / reviews
// ---------------------------------------------------------------------

#[test]
fn setter_scope_covers_all_four_subdirs() {
    let ws = TestWorkspace::named("complete-setter-all-subdirs");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join("docs").join("a.md"), "# a\n");
    write_file(&ws.path().join("features").join("b.md"), "# b\n");
    write_file(&ws.path().join("fixes").join("c.md"), "# c\n");
    write_file(&ws.path().join("reviews").join("d.md"), "# d\n");

    let got = run_complete(ws.path(), &["compose", "foo.md", "ref=@"]);
    assert!(got.iter().any(|c| c == "ref='docs/a.md'"), "{got:?}");
    assert!(got.iter().any(|c| c == "ref='features/b.md'"), "{got:?}");
    assert!(got.iter().any(|c| c == "ref='fixes/c.md'"), "{got:?}");
    assert!(got.iter().any(|c| c == "ref='reviews/d.md'"), "{got:?}");
}

#[test]
fn setter_scope_anchors_on_cwd_excludes_parent_dirs() {
    let ws = TestWorkspace::named("complete-setter-area-scope");
    seed_cargo_workspace(ws.path(), &["claudine/lib"]);
    // A feature doc one level ABOVE the cwd (at the package-area level) and
    // one UNDER the cwd. Only the cwd-local one may surface.
    write_file(
        &ws.path().join("claudine").join("features").join("plan.md"),
        "# parent\n",
    );
    write_file(
        &ws.path()
            .join("claudine")
            .join("lib")
            .join("features")
            .join("plan.md"),
        "# local\n",
    );

    // cwd is the package directory; candidates anchor on it and render
    // cwd-relative — the parent-area doc must NOT surface.
    let cwd = ws.path().join("claudine").join("lib");
    let got = run_complete(&cwd, &["compose", "foo.md", "spec=@p"]);
    assert!(
        got.iter().any(|c| c == "spec='features/plan.md'"),
        "expected cwd-local feature doc rendered cwd-relative: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("claudine/features")),
        "parent-area doc above the cwd must NOT surface: {got:?}"
    );
}

#[test]
fn setter_scope_renders_package_docs_relative_to_cwd() {
    let ws = TestWorkspace::named("complete-setter-pkg-scope");
    seed_cargo_workspace(ws.path(), &["claudine/lib"]);
    // Package-level doc, under the cwd.
    let pkg_docs = ws.path().join("claudine").join("lib").join("docs");
    write_file(&pkg_docs.join("api.md"), "# api\n");

    let cwd = ws.path().join("claudine").join("lib");
    let got = run_complete(&cwd, &["compose", "foo.md", "ref=@a"]);
    assert!(
        got.iter().any(|c| c == "ref='docs/api.md'"),
        "expected cwd-relative package doc: {got:?}"
    );
}

// ---------------------------------------------------------------------
// Subcommand coverage — fires on all three composition subcommands
// ---------------------------------------------------------------------

#[test]
fn setter_value_fires_on_inline_compose() {
    let ws = TestWorkspace::named("complete-setter-inline-compose");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join("docs").join("spec.md"), "# s\n");

    let got = run_complete(ws.path(), &["inline-compose", "foo.md", "spec=@s"]);
    assert!(
        got.iter().any(|c| c == "spec='docs/spec.md'"),
        "inline-compose setter must resolve: {got:?}"
    );
}

#[test]
fn setter_value_fires_on_sequence() {
    let ws = TestWorkspace::named("complete-setter-sequence");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join("docs").join("spec.md"), "# s\n");

    let got = run_complete(ws.path(), &["sequence", "foo.md", "spec=@s"]);
    assert!(
        got.iter().any(|c| c == "spec='docs/spec.md'"),
        "sequence setter must resolve: {got:?}"
    );
}

// ---------------------------------------------------------------------
// Cursor classification — setter wins over positional
// ---------------------------------------------------------------------

#[test]
fn setter_value_wins_after_committed_positional() {
    // Contract: even when the positional file slot has been filled,
    // a setter-shaped cursor is NOT treated as "extra positional" —
    // the setter-value completer fires instead.
    let ws = TestWorkspace::named("complete-setter-after-positional");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join("docs").join("spec.md"), "# s\n");
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("prompt.md"), "# p\n");

    let got = run_complete(ws.path(), &["compose", "prompts/prompt.md", "spec=@s"]);
    assert!(
        got.iter().any(|c| c == "spec='docs/spec.md'"),
        "setter after positional must route through setter-value: {got:?}"
    );
}

// ---------------------------------------------------------------------
// Underscore and gitignore filters
// ---------------------------------------------------------------------

#[test]
fn setter_scope_excludes_underscore_prefixed() {
    let ws = TestWorkspace::named("complete-setter-underscore");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    let docs = ws.path().join("docs");
    write_file(&docs.join("_draft.md"), "# d\n");
    write_file(&docs.join("final.md"), "# f\n");

    let got = run_complete(ws.path(), &["compose", "foo.md", "ref=@"]);
    assert!(
        !got.iter().any(|c| c.contains("_draft")),
        "underscore-prefixed file must be elided: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "ref='docs/final.md'"),
        "non-underscore file must surface: {got:?}"
    );
}

// ---------------------------------------------------------------------
// Markdown extension gate (finding #1)
// ---------------------------------------------------------------------

#[test]
fn setter_excludes_non_markdown_files() {
    // Finding #1: setter-value completion must reject non-Markdown files
    // regardless of basename match. Seed a mix of extensions and verify
    // only `.md` surfaces.
    let ws = TestWorkspace::named("complete-setter-non-markdown");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    let docs = ws.path().join("docs");
    write_file(&docs.join("spec.md"), "# s\n");
    write_file(&docs.join("plan.txt"), "plain text\n");
    write_file(&docs.join("notes.yaml"), "key: value\n");
    write_file(&docs.join("extless"), "no extension\n");

    let got = run_complete(ws.path(), &["compose", "foo.md", "spec=@"]);
    assert!(
        got.iter().any(|c| c == "spec='docs/spec.md'"),
        ".md must surface: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains(".txt")),
        ".txt must NOT surface: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains(".yaml")),
        ".yaml must NOT surface: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("'docs/extless'")),
        "extensionless files must NOT surface: {got:?}"
    );
}

#[test]
fn setter_accepts_uppercase_md_and_markdown() {
    // Finding #1: extension gate is case-insensitive, so uppercase
    // `.MD` and `.MARKDOWN` must still surface.
    let ws = TestWorkspace::named("complete-setter-uppercase-md");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    let docs = ws.path().join("docs");
    write_file(&docs.join("PLAN.MD"), "# p\n");
    write_file(&docs.join("README.MARKDOWN"), "# r\n");

    let got = run_complete(ws.path(), &["compose", "foo.md", "ref=@"]);
    assert!(
        got.iter().any(|c| c == "ref='docs/PLAN.MD'"),
        "uppercase .MD must surface: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "ref='docs/README.MARKDOWN'"),
        "uppercase .MARKDOWN must surface: {got:?}"
    );
}

#[test]
fn setter_scope_honors_gitignore() {
    let ws = TestWorkspace::named("complete-setter-gitignore");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join(".gitignore"), "docs/private/\n");
    write_file(
        &ws.path().join("docs").join("private").join("secret.md"),
        "# s\n",
    );
    write_file(&ws.path().join("docs").join("public.md"), "# p\n");

    let got = run_complete(ws.path(), &["compose", "foo.md", "ref=@"]);
    assert!(
        !got.iter().any(|c| c.contains("secret")),
        "gitignored file must not surface: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "ref='docs/public.md'"),
        "non-ignored file must surface: {got:?}"
    );
}

// ---------------------------------------------------------------------
// review-3 finding 2 — nested feature-directory setter-value paths
// ---------------------------------------------------------------------

#[test]
fn setter_resolves_nested_feature_directory_path() {
    // Spec example:
    //   `claudine compose foobar.md spec=@spec<tab>`
    //   resolves to `'docs/2026-04-24-improved-shell-completions/spec.md'`
    //
    // The recursion contract is already implemented in the walker; this
    // test exists purely to lock it in. The single-quote wrapping and
    // exact relative path (with the dated feature directory) are both
    // load-bearing — assert the full token, not just `contains("spec.md")`.
    let ws = TestWorkspace::named("complete-setter-nested-feature-dir");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path()
            .join("docs")
            .join("2026-04-24-improved-shell-completions")
            .join("spec.md"),
        "# spec\n",
    );

    let got = run_complete(ws.path(), &["compose", "foobar.md", "spec=@spec"]);

    assert!(
        got.iter()
            .any(|c| c == "spec='docs/2026-04-24-improved-shell-completions/spec.md'"),
        "nested feature-dir spec path not surfaced: {got:?}"
    );
}

// ---------------------------------------------------------------------
// review-3 finding 3 — plain git checkout (no Cargo workspace)
// ---------------------------------------------------------------------

#[test]
fn setter_resolves_in_plain_git_checkout() {
    // review-3 finding 3: lock in that setter-value completion works in a
    // repo without a Cargo workspace (sniff::detect_repo_structure returns
    // None). The walk anchors on the invoking `cwd`, so seed the doc under
    // the cwd and assert it surfaces cwd-relative.
    let ws = TestWorkspace::named("complete-setter-plain-git");
    seed_plain_git_repo(ws.path());
    // intentionally NO Cargo.toml — sniff::detect_repo_structure
    // returns None for this layout.
    let nested = ws.path().join("nested").join("child");
    fs::create_dir_all(&nested).unwrap();
    write_file(&nested.join("docs").join("spec.md"), "# spec\n");

    let got = run_complete(&nested, &["compose", "foobar.md", "spec=@spec"]);

    assert!(
        got.iter().any(|c| c == "spec='docs/spec.md'"),
        "plain-git setter-value did not resolve cwd-local docs/spec.md: {got:?}"
    );
}
