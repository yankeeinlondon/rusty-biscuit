//! Integration tests for `claudine compose <TAB>` positional completion.
//!
//! Phase 3 of the `2026-04-24-improved-shell-completions` feature. These
//! tests drive the compiled `claudine` binary's hidden `__complete`
//! subcommand against seeded temp-directory fixtures and assert that the
//! composition completer's contract holds end-to-end:
//!
//! - `.md` files whose frontmatter **does not** carry a `prompt` key
//!   surface in compose mode.
//! - `inline-compose` targets (files with a `prompt` frontmatter key) do
//!   **not** surface.
//! - `@`-prefixed partials resolve to relative paths (the `@` sigil is
//!   stripped on selection).
//! - Prefix-length progression: 0 chars → no directories; 3+ chars →
//!   directories are surfaced with a trailing `/`.
//! - `docs/` and `.claude/skills/` — inline-compose-only extras — do NOT
//!   surface in compose mode.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::cargo::cargo_bin_cmd;

mod common;
use common::TestWorkspace;

/// Seed a minimal Cargo workspace so `sniff::detect_repo_structure`
/// recognizes the tempdir as a monorepo. A plain `.git` is not enough for
/// the composition completer because the scope resolver uses `sniff` for
/// monorepo shape; the git-root fallback only covers non-workspace scopes.
fn seed_cargo_workspace(root: &Path) {
    seed_cargo_workspace_members(root, &["pkg"]);
}

fn seed_plain_git_repo(root: &Path) {
    fs::create_dir_all(root.join(".git")).unwrap();
}

fn seed_cargo_workspace_members(root: &Path, members: &[&str]) {
    fs::create_dir_all(root.join(".git")).unwrap();
    let members_list = members
        .iter()
        .map(|member| format!("\"{member}\""))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        root.join("Cargo.toml"),
        format!("[workspace]\nresolver = \"2\"\nmembers = [{members_list}]\n"),
    )
    .unwrap();
    for member in members {
        let pkg = root.join(member);
        fs::create_dir_all(pkg.join("src")).unwrap();
        let name = member.replace('/', "-");
        fs::write(
            pkg.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"),
        )
        .unwrap();
        fs::write(pkg.join("src").join("lib.rs"), "").unwrap();
    }
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn fake_home(cwd: &Path) -> PathBuf {
    let parent = cwd.parent().unwrap_or(cwd);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let leaf = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("claudine-home");
    let home = parent.join(format!("{leaf}-home-{nonce}-{}", std::process::id()));
    fs::create_dir_all(&home).expect("create fake home");
    home
}

fn run_complete(cwd: &Path, argv_tail: &[&str]) -> Vec<String> {
    let home = fake_home(cwd);
    run_complete_with_home(cwd, &home, argv_tail)
}

fn run_complete_with_home(cwd: &Path, home: &Path, argv_tail: &[&str]) -> Vec<String> {
    let current = argv_tail.len();
    let reference = cargo_bin_cmd!("claudine");
    let program = reference.get_program().to_os_string();
    let mut cmd = Command::new(program);
    cmd.current_dir(cwd)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .env_remove("COMPLETE")
        .env_remove("_CLAP_COMPLETE_INDEX")
        .env_remove("_CLAP_IFS")
        .arg("__complete")
        .arg("--current")
        .arg(current.to_string())
        .arg("--")
        .arg("claudine");
    for arg in argv_tail {
        cmd.arg(arg);
    }
    let output = cmd.output().expect("completion subprocess to run");
    assert!(
        output.status.success(),
        "completion subprocess failed: status={:?}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8(output.stdout)
        .expect("utf-8")
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

// ---------------------------------------------------------------------
// compose empty partial — files only, no prompt-keyed files
// ---------------------------------------------------------------------

#[test]
fn compose_empty_partial_surfaces_plain_markdown() {
    let ws = TestWorkspace::named("complete-compose-empty");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("plan.md"), "---\ntitle: Plan\n---\nBody\n");
    write_file(&prompts.join("notes.md"), "# plain body only\n");

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/plan.md"),
        "compose empty partial must surface plan.md: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "prompts/notes.md"),
        "compose empty partial must surface notes.md: {got:?}"
    );
}

#[test]
fn compose_empty_partial_renders_repo_claudine_scope() {
    let ws = TestWorkspace::named("complete-compose-repo-claudine-empty");
    seed_cargo_workspace(ws.path());
    write_file(
        &ws.path().join(".claudine").join("prompts").join("plan.md"),
        "# plan\n",
    );

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        got.iter().any(|c| c == ".claudine/prompts/plan.md"),
        "repo .claudine prompt must render with .claudine prefix: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "prompts/plan.md"),
        "repo .claudine prompt must not collapse to prompts/: {got:?}"
    );
}

#[test]
fn compose_word_partial_renders_repo_claudine_scope() {
    let ws = TestWorkspace::named("complete-compose-repo-claudine-word");
    seed_cargo_workspace(ws.path());
    write_file(
        &ws.path().join(".claudine").join("prompts").join("plan.md"),
        "# plan\n",
    );

    let got = run_complete(ws.path(), &["compose", "plan"]);
    assert!(
        got.iter().any(|c| c == ".claudine/prompts/plan.md"),
        "repo .claudine word match must render with .claudine prefix: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "prompts/plan.md"),
        "repo .claudine word match must not collapse to prompts/: {got:?}"
    );
}

#[test]
fn compose_empty_partial_renders_user_global_scope() {
    let ws = TestWorkspace::named("complete-compose-user-global-empty");
    seed_cargo_workspace(ws.path());
    let home = fake_home(ws.path());
    write_file(
        &home.join(".claudine").join("prompts").join("plan.md"),
        "# plan\n",
    );

    let got = run_complete_with_home(ws.path(), &home, &["compose", ""]);
    assert!(
        got.iter().any(|c| c == "~/.claudine/prompts/plan.md"),
        "user-global prompt must render home-relative: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "prompts/plan.md"),
        "user-global prompt must not collapse to prompts/: {got:?}"
    );
}

#[test]
fn compose_word_partial_renders_user_global_scope() {
    let ws = TestWorkspace::named("complete-compose-user-global-word");
    seed_cargo_workspace(ws.path());
    let home = fake_home(ws.path());
    write_file(
        &home.join(".claudine").join("prompts").join("plan.md"),
        "# plan\n",
    );

    let got = run_complete_with_home(ws.path(), &home, &["compose", "plan"]);
    assert!(
        got.iter().any(|c| c == "~/.claudine/prompts/plan.md"),
        "user-global word match must render home-relative: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "prompts/plan.md"),
        "user-global word match must not collapse to prompts/: {got:?}"
    );
}

#[test]
fn compose_package_prompt_renders_repo_relative_path() {
    let ws = TestWorkspace::named("complete-compose-package-render");
    seed_cargo_workspace(ws.path());
    let pkg = ws.path().join("pkg");
    write_file(&pkg.join("prompts").join("plan.md"), "# plan\n");

    let got = run_complete(&pkg, &["compose", ""]);
    assert!(
        got.iter().any(|c| c == "pkg/prompts/plan.md"),
        "package prompt must render repo-relative package path: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "prompts/plan.md"),
        "package prompt must not collapse to prompts/: {got:?}"
    );
}

#[test]
fn compose_package_area_prompt_renders_repo_relative_path() {
    let ws = TestWorkspace::named("complete-compose-package-area-render");
    seed_cargo_workspace_members(ws.path(), &["claudine/lib", "claudine/cli"]);
    let area = ws.path().join("claudine");
    write_file(&area.join("prompts").join("plan.md"), "# plan\n");

    let got = run_complete(&area, &["compose", ""]);
    assert!(
        got.iter().any(|c| c == "claudine/prompts/plan.md"),
        "package-area prompt must render repo-relative area path: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "prompts/plan.md"),
        "package-area prompt must not collapse to prompts/: {got:?}"
    );
}

#[test]
fn compose_empty_partial_skips_prompt_frontmatter_files() {
    let ws = TestWorkspace::named("complete-compose-skip-prompt");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("plain.md"), "# plain\n");
    write_file(
        &prompts.join("inline.md"),
        "---\nprompt: Say hi\n---\nBody\n",
    );

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/plain.md"),
        "compose must surface plain.md: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("inline.md")),
        "compose must NOT surface files with `prompt:` frontmatter: {got:?}"
    );
}

#[test]
fn compose_empty_partial_does_not_include_directories() {
    let ws = TestWorkspace::named("complete-compose-no-dirs-empty");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    fs::create_dir_all(prompts.join("subdir")).unwrap();
    write_file(&prompts.join("subdir").join("inner.md"), "# inner\n");

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        !got.iter().any(|c| c == "prompts/subdir/"),
        "empty partial must not surface directories: {got:?}"
    );
}

// ---------------------------------------------------------------------
// compose — docs/ and skills/ are NOT in scope for compose
// ---------------------------------------------------------------------

#[test]
fn compose_does_not_surface_docs_or_skills() {
    let ws = TestWorkspace::named("complete-compose-no-extras");
    seed_cargo_workspace(ws.path());
    write_file(&ws.path().join("docs").join("guide.md"), "# g\n");
    write_file(
        &ws.path().join(".claude").join("skills").join("expert.md"),
        "# ex\n",
    );

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        !got.iter().any(|c| c.contains("docs/guide.md")),
        "compose must NOT surface docs/: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("expert.md")),
        "compose must NOT surface .claude/skills/: {got:?}"
    );
}

// ---------------------------------------------------------------------
// compose prefix-length progression
// ---------------------------------------------------------------------

#[test]
fn compose_short_prefix_matches_filenames_and_dirs_with_prefix() {
    // Spec §5.3 (review-1 finding #3): short (1–2 char) prefixes must
    // surface BOTH file matches in high-profile scopes AND directory
    // matches across the repo (or CWD fallback). Directory matching at
    // short prefixes is starting-substring (case-insensitive), not
    // fuzzy — so `pl` matches `planning/` but not, say, `models/`.
    let ws = TestWorkspace::named("complete-compose-short");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("plan.md"), "# p\n");
    // Directory at the repo root, NOT under prompts/, so it can only
    // surface via the repo-wide directory walk.
    fs::create_dir_all(ws.path().join("planning")).unwrap();
    // Non-matching directory must NOT appear (prefix gate).
    fs::create_dir_all(ws.path().join("models")).unwrap();

    let got = run_complete(ws.path(), &["compose", "pl"]);
    assert!(
        got.iter().any(|c| c == "prompts/plan.md"),
        "short prefix must match plan.md (high-profile scope): {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "planning/"),
        "short prefix must surface matching repo-wide directory `planning/`: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "models/"),
        "short prefix is starting-substring; non-matching `models/` must NOT appear: {got:?}"
    );
}

#[test]
fn compose_long_prefix_includes_directories() {
    let ws = TestWorkspace::named("complete-compose-long");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("plan.md"), "# p\n");
    fs::create_dir_all(prompts.join("planning")).unwrap();

    let got = run_complete(ws.path(), &["compose", "pla"]);
    assert!(
        got.iter().any(|c| c == "prompts/plan.md"),
        "long prefix must still match files: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "prompts/planning/"),
        "long prefix must include directories with trailing `/`: {got:?}"
    );
}

// ---------------------------------------------------------------------
// compose committed-directory navigation
// ---------------------------------------------------------------------

#[test]
fn compose_committed_dir_walks_inside_only() {
    let ws = TestWorkspace::named("complete-compose-committed");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("outer.md"), "# o\n");
    write_file(&prompts.join("planning").join("deep.md"), "# d\n");

    let got = run_complete(ws.path(), &["compose", "prompts/planning/"]);
    assert!(
        got.iter().any(|c| c == "prompts/planning/deep.md"),
        "committed dir must surface inside: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("outer.md")),
        "committed dir must not leak parent: {got:?}"
    );
}

#[test]
fn compose_plain_git_committed_dir_uses_git_root() {
    let ws = TestWorkspace::named("complete-compose-plain-git-committed");
    seed_plain_git_repo(ws.path());
    let nested = ws.path().join("nested").join("child");
    fs::create_dir_all(&nested).unwrap();
    write_file(
        &ws.path().join("prompts").join("planning").join("deep.md"),
        "# deep\n",
    );

    let got = run_complete(&nested, &["compose", "prompts/planning/"]);
    assert!(
        got.iter().any(|c| c == "prompts/planning/deep.md"),
        "plain git committed dir must resolve from git root, not nested cwd: {got:?}"
    );
}

// ---------------------------------------------------------------------
// compose @ magic-path resolution
// ---------------------------------------------------------------------

#[test]
fn compose_magic_path_strips_sigil_and_renders_relative() {
    let ws = TestWorkspace::named("complete-compose-magic");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("plan.md"), "# p\n");

    let got = run_complete(ws.path(), &["compose", "@plan"]);
    assert!(
        got.iter().any(|c| c == "prompts/plan.md"),
        "@ sigil must strip on selection; got: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.starts_with('@')),
        "no @-prefixed candidates should be emitted: {got:?}"
    );
}

#[test]
fn compose_magic_repo_prompts_shadows_repo_claudine_and_user() {
    let ws = TestWorkspace::named("complete-compose-magic-repo-first");
    seed_cargo_workspace(ws.path());
    let home = fake_home(ws.path());

    write_file(&ws.path().join("prompts").join("plan.md"), "# repo\n");
    write_file(
        &ws.path().join(".claudine").join("prompts").join("plan.md"),
        "# repo claudine\n",
    );
    write_file(
        &home.join(".claudine").join("prompts").join("plan.md"),
        "# user\n",
    );

    let got = run_complete_with_home(ws.path(), &home, &["compose", "@plan"]);
    assert_eq!(
        got,
        vec!["prompts/plan.md".to_string()],
        "repo prompts hit must suppress lower-priority tiers"
    );
}

#[test]
fn compose_magic_repo_claudine_shadows_user_when_repo_prompts_absent() {
    let ws = TestWorkspace::named("complete-compose-magic-claudine-first");
    seed_cargo_workspace(ws.path());
    let home = fake_home(ws.path());

    write_file(
        &ws.path().join(".claudine").join("prompts").join("plan.md"),
        "# repo claudine\n",
    );
    write_file(
        &home.join(".claudine").join("prompts").join("plan.md"),
        "# user\n",
    );

    let got = run_complete_with_home(ws.path(), &home, &["compose", "@plan"]);
    assert_eq!(
        got,
        vec![".claudine/prompts/plan.md".to_string()],
        "repo .claudine hit must suppress user-global tier"
    );
}

#[test]
fn compose_magic_user_global_emits_when_no_repo_tier_matches() {
    let ws = TestWorkspace::named("complete-compose-magic-user-only");
    seed_cargo_workspace(ws.path());
    let home = fake_home(ws.path());

    write_file(
        &home.join(".claudine").join("prompts").join("plan.md"),
        "# user\n",
    );

    let got = run_complete_with_home(ws.path(), &home, &["compose", "@plan"]);
    assert_eq!(
        got,
        vec!["~/.claudine/prompts/plan.md".to_string()],
        "user-global hit should emit only when no repo tier matches"
    );
}

// ---------------------------------------------------------------------
// compose underscore and gitignore filters
// ---------------------------------------------------------------------

#[test]
fn compose_hides_underscore_prefixed_files() {
    let ws = TestWorkspace::named("complete-compose-underscore");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("_wip.md"), "# w\n");
    write_file(&prompts.join("ok.md"), "# o\n");

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        !got.iter().any(|c| c.contains("_wip.md")),
        "underscore-prefixed file must be hidden: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "prompts/ok.md"),
        "non-underscore file must surface: {got:?}"
    );
}

#[test]
fn compose_honors_gitignore_at_nested_depth() {
    let ws = TestWorkspace::named("complete-compose-gitignore");
    seed_cargo_workspace(ws.path());
    write_file(&ws.path().join(".gitignore"), "prompts/hidden/\n");
    write_file(
        &ws.path().join("prompts").join("hidden").join("bad.md"),
        "# b\n",
    );
    write_file(&ws.path().join("prompts").join("good.md"), "# g\n");

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/good.md"),
        "non-ignored file must surface: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("bad.md")),
        "gitignored file must not surface: {got:?}"
    );
}

// ---------------------------------------------------------------------
// compose size-gate contract (finding #6)
// ---------------------------------------------------------------------

#[test]
fn compose_rejects_oversized_markdown() {
    // Finding #6: oversized Markdown files must be rejected (not
    // silently accepted). The size-guard cap is MAX_FRONTMATTER_BYTES
    // (1 MiB); write a file larger than that.
    let ws = TestWorkspace::named("complete-compose-oversized");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    // 1 MiB + 1 byte of padding — larger than MAX_FRONTMATTER_BYTES.
    let padding = "a".repeat((1024 * 1024) + 1);
    write_file(&prompts.join("huge.md"), &padding);
    write_file(&prompts.join("small.md"), "# small\n");

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/small.md"),
        "small.md must surface: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("huge.md")),
        "oversized markdown must NOT surface: {got:?}"
    );
}

#[test]
fn compose_rejects_non_utf8_markdown() {
    // Finding #6: non-UTF-8 Markdown must be rejected uniformly. Write
    // raw invalid UTF-8 bytes so `fs::read_to_string` fails.
    let ws = TestWorkspace::named("complete-compose-non-utf8");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    // Byte slice with invalid UTF-8 sequences.
    fs::write(prompts.join("bad.md"), [0xFF, 0xFE, 0x00, 0x00, 0xC0, 0xAF]).unwrap();
    write_file(&prompts.join("good.md"), "# good\n");

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/good.md"),
        "good.md must surface: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("bad.md")),
        "non-UTF-8 markdown must NOT surface: {got:?}"
    );
}

// ---------------------------------------------------------------------
// compose @ magic-path shaped forms (finding #4)
// ---------------------------------------------------------------------

#[test]
fn compose_magic_path_shaped_prompts_slash_plan_resolves() {
    // Finding #4: `@prompts/plan` must resolve against the repo-scope
    // `prompts/` root and emit `prompts/plan.md`. The `prompts` portion
    // is peeled off so the join does not double up.
    let ws = TestWorkspace::named("complete-compose-magic-path-shaped");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("plan.md"), "# p\n");

    let got = run_complete(ws.path(), &["compose", "@prompts/plan"]);
    assert!(
        got.iter().any(|c| c == "prompts/plan.md"),
        "path-shaped magic `@prompts/plan` must resolve to `prompts/plan.md`: {got:?}"
    );
}

#[test]
fn compose_magic_path_shaped_claudine_prompts_resolves() {
    // Finding #4: `@.claudine/prompts/plan` must resolve against the
    // repo-scope `.claudine/prompts/` root and emit
    // `.claudine/prompts/plan.md`. Scope leaf is `prompts`, so `dir`
    // `.claudine/prompts` peels the `prompts` segment; the remaining
    // `.claudine` join is not a directory under the scope, so the repo
    // `.claudine/prompts` scope itself is the only hit (via raw join
    // fallback on non-matching leaf elsewhere — see walk-root
    // resolution).
    let ws = TestWorkspace::named("complete-compose-magic-claudine");
    seed_cargo_workspace(ws.path());
    let claudine = ws.path().join(".claudine").join("prompts");
    write_file(&claudine.join("plan.md"), "# p\n");

    let got = run_complete(ws.path(), &["compose", "@.claudine/prompts/plan"]);
    assert!(
        got.iter().any(|c| c == ".claudine/prompts/plan.md"),
        "path-shaped magic `@.claudine/prompts/plan` must resolve: {got:?}"
    );
}

#[test]
fn compose_plain_git_magic_renders_repo_claudine_relative() {
    let ws = TestWorkspace::named("complete-compose-plain-git-magic-claudine");
    seed_plain_git_repo(ws.path());
    let nested = ws.path().join("nested").join("child");
    fs::create_dir_all(&nested).unwrap();
    write_file(
        &ws.path().join(".claudine").join("prompts").join("plan.md"),
        "# plan\n",
    );

    let got = run_complete(&nested, &["compose", "@.claudine/prompts/plan"]);
    assert_eq!(
        got,
        vec![".claudine/prompts/plan.md".to_string()],
        "plain git magic should render repo .claudine path relative to git root"
    );
}

#[test]
fn compose_plain_git_non_magic_renders_repo_claudine_relative() {
    let ws = TestWorkspace::named("complete-compose-plain-git-word-claudine");
    seed_plain_git_repo(ws.path());
    let nested = ws.path().join("nested").join("child");
    fs::create_dir_all(&nested).unwrap();
    write_file(
        &ws.path().join(".claudine").join("prompts").join("plan.md"),
        "# plan\n",
    );

    let got = run_complete(&nested, &["compose", "plan"]);
    assert!(
        got.iter().any(|c| c == ".claudine/prompts/plan.md"),
        "plain git non-magic should render repo .claudine path relative to git root: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "prompts/plan.md"),
        "plain git repo .claudine prompt must not collapse to prompts/: {got:?}"
    );
}

// ---------------------------------------------------------------------
// repo-wide directory walk (review-1 findings #2 and #3)
// ---------------------------------------------------------------------

#[test]
fn compose_one_char_prefix_surfaces_repo_directories() {
    // Finding #2 and #3: 1-char prefix surfaces directories from the
    // repo-wide walk (case-insensitive starting-substring match), not
    // just from high-profile scopes. `docs/` and `features/` at the
    // repo root must surface even though they are NOT compose scopes.
    let ws = TestWorkspace::named("complete-compose-one-char-dirs");
    seed_cargo_workspace(ws.path());
    fs::create_dir_all(ws.path().join("docs")).unwrap();
    fs::create_dir_all(ws.path().join("features")).unwrap();

    let got = run_complete(ws.path(), &["compose", "d"]);
    assert!(
        got.iter().any(|c| c == "docs/"),
        "1-char prefix `d` must surface `docs/`: {got:?}"
    );
    // `f` would match `features/` but we asked for `d`; verify a
    // non-matching directory is excluded.
    assert!(
        !got.iter().any(|c| c == "features/"),
        "1-char prefix `d` must NOT surface `features/`: {got:?}"
    );
}

#[test]
fn compose_two_char_prefix_surfaces_repo_directories() {
    // Finding #3: 2-char prefix uses starting-substring matching, NOT
    // fuzzy. `cl` matches `claudine/` but NOT `biscuit-speaks/`
    // (which is a fuzzy subsequence hit on `cl` via the 'c'-'l' in
    // "biscui*-speaks" — but starting-substring excludes it).
    let ws = TestWorkspace::named("complete-compose-two-char-dirs");
    seed_cargo_workspace(ws.path());
    fs::create_dir_all(ws.path().join("claudine")).unwrap();
    fs::create_dir_all(ws.path().join("biscuit-speaks")).unwrap();

    let got = run_complete(ws.path(), &["compose", "cl"]);
    assert!(
        got.iter().any(|c| c == "claudine/"),
        "2-char prefix `cl` must surface `claudine/`: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "biscuit-speaks/"),
        "2-char prefix is starting-substring; `biscuit-speaks/` must NOT appear: {got:?}"
    );
}

#[test]
fn compose_three_char_prefix_fuzzy_matches_directory_names() {
    // Finding #3: 3+ char prefix uses fuzzy subsequence matching for
    // directories. `dcm` is a subsequence of `documentation`.
    let ws = TestWorkspace::named("complete-compose-three-char-fuzzy-dir");
    seed_cargo_workspace(ws.path());
    fs::create_dir_all(ws.path().join("documentation")).unwrap();

    let got = run_complete(ws.path(), &["compose", "dcm"]);
    assert!(
        got.iter().any(|c| c == "documentation/"),
        "3-char prefix `dcm` must fuzzy-match `documentation/`: {got:?}"
    );
}

#[test]
fn compose_empty_partial_still_does_not_surface_repo_dirs() {
    // Regression guard: Empty prefix → no directory candidates at all,
    // even from the repo-wide walk. The repo-wide pass returns
    // immediately when DirMatchMode::None.
    let ws = TestWorkspace::named("complete-compose-empty-no-dirs");
    seed_cargo_workspace(ws.path());
    fs::create_dir_all(ws.path().join("docs")).unwrap();
    fs::create_dir_all(ws.path().join("features")).unwrap();

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        !got.iter().any(|c| c.ends_with('/')),
        "empty prefix must surface no directories at all: {got:?}"
    );
}
