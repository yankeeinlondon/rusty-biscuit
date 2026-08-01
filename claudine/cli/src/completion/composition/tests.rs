use super::*;
use std::fs;
use tempfile::TempDir;

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn seed_cargo_workspace(root: &Path, members: &[&str]) {
    fs::create_dir_all(root.join(".git")).unwrap();
    let members_list = members
        .iter()
        .map(|m| format!("    \"{m}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    let root_manifest =
        format!("[workspace]\nresolver = \"2\"\nmembers = [\n{members_list}\n]\n");
    fs::write(root.join("Cargo.toml"), root_manifest).unwrap();
    for member in members {
        let member_dir = root.join(member);
        fs::create_dir_all(member_dir.join("src")).unwrap();
        let name = member.replace('/', "-");
        fs::write(
            member_dir.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
        )
        .unwrap();
        fs::write(member_dir.join("src").join("lib.rs"), "").unwrap();
    }
}

// -- name_stem --------------------------------------------------------

#[test]
fn name_stem_strips_markdown_extensions() {
    assert_eq!(name_stem("plan.md"), "plan");
    assert_eq!(name_stem("plan.MD"), "plan");
    assert_eq!(name_stem("readme.markdown"), "readme");
    assert_eq!(name_stem("no-ext"), "no-ext");
}

#[test]
fn name_stem_strips_yaml_extensions() {
    assert_eq!(name_stem("steps.yaml"), "steps");
    assert_eq!(name_stem("steps.yml"), "steps");
}

// -- file_name_matches ------------------------------------------------

#[test]
fn file_name_matches_stem_and_extension_typing() {
    // Empty query matches everything.
    assert!(file_name_matches("plan.md", ""));
    // Stem match (the common case).
    assert!(file_name_matches("plan.md", "plan"));
    assert!(file_name_matches("plan.md", "pl"));
    // Typing into the extension must keep matching — the regression:
    // a `.` is not in the stem, so stem-only matching dropped these.
    assert!(file_name_matches("plan.md", "plan."));
    assert!(file_name_matches("plan.md", "plan.m"));
    assert!(file_name_matches("plan.md", "plan.md"));
    // Non-matches.
    assert!(!file_name_matches("plan.md", "xyz"));
    assert!(!file_name_matches("plan.md", "plan.xx"));
}

// -- end-to-end (compose) ---------------------------------------------

#[test]
fn compose_empty_partial_surfaces_files_without_prompt() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(&prompts.join("plan.md"), "---\ntitle: X\n---\nBody\n");
    write(
        &prompts.join("inline.md"),
        "---\nprompt: Write\n---\nBody\n",
    );

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "");
    assert!(
        got.iter().any(|c| c.ends_with("plan.md")),
        "compose must surface plan.md: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("inline.md")),
        "compose must NOT surface inline.md (has prompt key): {got:?}"
    );
}

#[test]
fn compose_empty_partial_does_not_surface_directories() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(&prompts.join("plan.md"), "---\ntitle: X\n---\n");
    fs::create_dir_all(prompts.join("subdir")).unwrap();

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "");
    // Empty partial → no directories in output.
    assert!(
        !got.iter().any(|c| c.ends_with("subdir/")),
        "empty partial must not emit directories: {got:?}"
    );
}

#[test]
fn compose_short_prefix_fuzzy_matches_filenames_and_dirs() {
    // Spec §5.3 (review-1 finding #3): short (1–2 char) prefixes
    // surface files via fuzzy matching in high-profile scopes AND
    // directories via prefix matching from the repo-wide walk.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(&prompts.join("plan.md"), "---\ntitle: X\n---\n");
    write(&prompts.join("notes.md"), "---\ntitle: Y\n---\n");
    // Directory at the repo root surfaces via the repo-wide walk.
    fs::create_dir_all(tmp.path().join("planning")).unwrap();

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "pl");
    assert!(
        got.iter().any(|c| c.ends_with("plan.md")),
        "short prefix `pl` must match plan.md: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("notes.md")),
        "short prefix `pl` must not match notes.md: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "planning/"),
        "short prefix `pl` must surface matching repo-wide dir: {got:?}"
    );
}

#[test]
fn compose_long_prefix_surfaces_directories() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    fs::create_dir_all(prompts.join("planning")).unwrap();
    write(&prompts.join("plan.md"), "---\ntitle: X\n---\n");

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "pla");
    assert!(
        got.iter().any(|c| c.ends_with("plan.md")),
        "long prefix must still match files: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "prompts/planning/"),
        "long prefix must surface directories with trailing slash: {got:?}"
    );
}

#[test]
fn compose_committed_dir_walks_only_inside() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(&prompts.join("outer.md"), "---\ntitle: O\n---\n");
    write(
        &prompts.join("planning").join("deep.md"),
        "---\ntitle: D\n---\n",
    );

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "prompts/planning/");
    assert!(
        got.iter().any(|c| c == "prompts/planning/deep.md"),
        "committed dir must surface inside content: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("outer.md")),
        "committed dir must not surface outer content: {got:?}"
    );
}

#[test]
fn compose_magic_keeps_sigil_and_renders_filename_only() {
    // Filename-magic contract: `@plan` completes to `@plan.md` — the `@`
    // is kept and only the basename is inserted (no path). The committed
    // `@plan.md` is resolved to the closest file at launch.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(&prompts.join("plan.md"), "---\ntitle: X\n---\n");

    // Null out HOME so the user-global scope (real `~/.claudine/prompts`)
    // cannot leak files into this exact-match assertion.
    let mut ctx = ScopeContext::discover_from(tmp.path());
    ctx.home = None;
    let got = run(ComposeMode::Compose, &ctx, "@plan");
    assert_eq!(
        got,
        vec!["@plan.md".to_string()],
        "magic must render @<basename>, no path: {got:?}"
    );
}

#[test]
fn compose_magic_matches_while_typing_extension() {
    // Regression: `@plan.` (and `@plan.md`) must keep matching `plan.md`,
    // not just the bare-stem `@plan`.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    write(
        &tmp.path().join("prompts").join("plan.md"),
        "---\ntitle: X\n---\n",
    );
    let mut ctx = ScopeContext::discover_from(tmp.path());
    ctx.home = None;
    for q in ["@plan", "@plan.", "@plan.m", "@plan.md"] {
        let got = run(ComposeMode::Compose, &ctx, q);
        assert!(
            got.iter().any(|c| c == "@plan.md"),
            "{q} must complete to @plan.md: {got:?}"
        );
    }
}

#[test]
fn compose_word_matches_while_typing_extension() {
    // The same extension-typing fix applies to non-magic Word mode.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    write(
        &tmp.path().join("prompts").join("plan.md"),
        "---\ntitle: X\n---\n",
    );
    let mut ctx = ScopeContext::discover_from(tmp.path());
    ctx.home = None;
    for q in ["plan", "plan.", "plan.md"] {
        let got = run(ComposeMode::Compose, &ctx, q);
        assert!(
            got.iter().any(|c| c == "prompts/plan.md"),
            "{q} must complete to prompts/plan.md: {got:?}"
        );
    }
}

#[test]
fn compose_magic_dedups_basename_across_scopes() {
    // The same filename in two scopes (repo `prompts/` and repo
    // `.claudine/prompts/`) must surface once as `@plan.md`.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    write(
        &tmp.path().join("prompts").join("plan.md"),
        "---\ntitle: X\n---\n",
    );
    write(
        &tmp.path().join(".claudine").join("prompts").join("plan.md"),
        "---\ntitle: Y\n---\n",
    );

    let mut ctx = ScopeContext::discover_from(tmp.path());
    ctx.home = None;
    let got = run(ComposeMode::Compose, &ctx, "@plan");
    assert_eq!(
        got.iter().filter(|c| *c == "@plan.md").count(),
        1,
        "duplicate basename across scopes must collapse: {got:?}"
    );
}

#[test]
fn compose_magic_path_shaped_uses_shared_rendered_prefix() {
    // The shared parser keeps the authored scope in the emitted value.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(&prompts.join("plan.md"), "---\ntitle: X\n---\n");

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "@prompts/plan");
    assert!(
        got.iter().any(|c| c == "@prompts/plan.md"),
        "path-shaped magic must retain the shared prefix: {got:?}"
    );
}

#[test]
fn compose_magic_nested_path_shaped_uses_shared_rendered_prefix() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(
        &prompts.join("drafts").join("plan.md"),
        "---\ntitle: X\n---\n",
    );

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "@prompts/drafts/plan");
    assert!(
        got.iter().any(|c| c == "@prompts/drafts/plan.md"),
        "nested path-shaped magic must retain the shared prefix: {got:?}"
    );
}

#[test]
fn compose_magic_does_not_emit_a_nested_file_without_its_scope() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    write(
        &tmp.path().join("prompts/drafts/plan.md"),
        "---\ntitle: X\n---\n",
    );

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "@plan");
    assert!(
        !got.iter().any(|candidate| candidate == "@plan.md"),
        "completion must not flatten a nested path that runtime cannot resolve: {got:?}",
    );
}

#[test]
fn compose_magic_path_shaped_misses_when_dir_absent() {
    // Seed only `prompts/plan.md`; query `@prompts/drafts/plan`.
    // `<repo>/prompts/drafts/` does not exist, so the walk-root
    // `is_dir()` check fails and zero candidates are emitted.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(&prompts.join("plan.md"), "---\ntitle: X\n---\n");

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "@prompts/drafts/plan");
    assert!(
        !got.iter().any(|c| c.ends_with("plan.md")),
        "missing dir join must yield no candidates: {got:?}"
    );
}

// -- end-to-end (inline-compose) --------------------------------------

#[test]
fn inline_compose_surfaces_files_with_prompt_key() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(&prompts.join("plain.md"), "---\ntitle: X\n---\n");
    write(
        &prompts.join("inline.md"),
        "---\nprompt: Write\n---\nBody\n",
    );

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::InlineCompose, &ctx, "");
    assert!(
        got.iter().any(|c| c.ends_with("inline.md")),
        "inline-compose must surface inline.md: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("plain.md")),
        "inline-compose must not surface plain.md: {got:?}"
    );
}

#[test]
fn inline_compose_includes_docs_extra() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let docs = tmp.path().join("docs");
    write(&docs.join("spec.md"), "---\nprompt: Describe\n---\n");

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::InlineCompose, &ctx, "");
    assert!(
        got.iter().any(|c| c.ends_with("spec.md")),
        "inline-compose must include docs/ extras: {got:?}"
    );
}

// -- end-to-end (sequence) --------------------------------------------

#[test]
fn sequence_surfaces_markdown_with_sequence_key() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(
        &prompts.join("steps.md"),
        "---\nsequence:\n  - a\n  - b\n---\n",
    );
    write(&prompts.join("nope.md"), "---\ntitle: X\n---\n");

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Sequence, &ctx, "");
    assert!(
        got.iter().any(|c| c.ends_with("steps.md")),
        "sequence must surface steps.md: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("nope.md")),
        "sequence must not surface nope.md (no sequence key): {got:?}"
    );
}

#[test]
fn sequence_surfaces_yaml_files() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(&prompts.join("steps.yaml"), "sequence:\n  - one\n  - two\n");

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Sequence, &ctx, "");
    assert!(
        got.iter().any(|c| c.ends_with("steps.yaml")),
        "sequence must surface YAML files: {got:?}"
    );
}

// -- prefix progression ------------------------------------------------

#[test]
fn empty_prefix_does_not_include_any_directory() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    fs::create_dir_all(prompts.join("planning")).unwrap();
    write(
        &prompts.join("planning").join("a.md"),
        "---\ntitle: A\n---\n",
    );

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "");
    assert!(
        !got.iter().any(|c| c.ends_with('/')),
        "no directory candidate for empty prefix: {got:?}"
    );
}

#[test]
fn three_plus_chars_includes_directories() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    fs::create_dir_all(prompts.join("planning")).unwrap();

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "pla");
    assert!(
        got.iter().any(|c| c.ends_with('/')),
        "3+ char prefix must include directories: {got:?}"
    );
}

// -- repo-wide directory walk (review-1 finding #2 + #3) --------------

#[test]
fn compose_one_char_prefix_surfaces_matching_repo_dir() {
    // Spec §5.3: 1-char prefix must surface matching directories
    // from the repo-wide walk (case-insensitive prefix match).
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    fs::create_dir_all(tmp.path().join("claudine")).unwrap();

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "c");
    assert!(
        got.iter().any(|c| c == "claudine/"),
        "1-char prefix must surface `claudine/` from repo root: {got:?}"
    );
}

#[test]
fn compose_two_char_prefix_surfaces_matching_repo_dir() {
    // Spec §5.3: 2-char prefix surfaces matching directories.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    fs::create_dir_all(tmp.path().join("docs")).unwrap();

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "do");
    assert!(
        got.iter().any(|c| c == "docs/"),
        "2-char prefix must surface `docs/` from repo root: {got:?}"
    );
}

#[test]
fn compose_short_prefix_directory_match_is_starting_substring() {
    // Spec §5.3: short prefixes use prefix matching, not fuzzy. So
    // `do` matches `docs/` but NOT `widgets/` (no 'd' at start).
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    fs::create_dir_all(tmp.path().join("docs")).unwrap();
    fs::create_dir_all(tmp.path().join("widgets")).unwrap();

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "do");
    assert!(
        got.iter().any(|c| c == "docs/"),
        "starting-substring prefix must hit `docs/`: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "widgets/"),
        "starting-substring prefix must miss `widgets/`: {got:?}"
    );
}

#[test]
fn compose_long_prefix_directory_match_is_fuzzy() {
    // Spec §5.3: 3+ char prefixes use fuzzy subsequence matching.
    // `fbb` is a subsequence of `foo-bar-baz` but not a prefix.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    fs::create_dir_all(tmp.path().join("foo-bar-baz")).unwrap();

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "fbb");
    assert!(
        got.iter().any(|c| c == "foo-bar-baz/"),
        "long prefix must fuzzy-match `foo-bar-baz/` against `fbb`: {got:?}"
    );
}

#[test]
fn compose_repo_dir_walk_skips_high_profile_roots_once() {
    // A directory that the high-profile scope walker also surfaces
    // (e.g. `prompts/planning/` at Long prefix) must dedup across
    // both passes — appearing exactly once in output.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    fs::create_dir_all(prompts.join("planning")).unwrap();

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "pla");
    let count = got.iter().filter(|c| c.contains("planning")).count();
    assert_eq!(
        count, 1,
        "`prompts/planning/` must dedup across both passes: {got:?}"
    );
}

// -- magic mode is filename-only (no directory candidates) ------------

#[test]
fn compose_magic_short_prefix_surfaces_no_directories() {
    // Filename-magic contract: `@pl<TAB>` surfaces the matching file as
    // `@plan.md` and NEVER a directory — directory drilling is reserved
    // for non-`@` (Word-mode) paths.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(&prompts.join("plan.md"), "---\ntitle: P\n---\n");
    fs::create_dir_all(tmp.path().join("planning")).unwrap();

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "@pl");
    assert!(
        got.iter().any(|c| c == "@plan.md"),
        "magic short prefix must surface file: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with('/')),
        "magic mode must never surface directories: {got:?}"
    );
}

#[test]
fn compose_magic_long_prefix_surfaces_no_directories() {
    // A directory whose name fuzzy-matches the partial must NOT surface
    // under `@` — only prompt files do.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    fs::create_dir_all(tmp.path().join("documentation")).unwrap();

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "@dcm");
    assert!(
        !got.iter().any(|c| c.ends_with('/')),
        "magic mode must not surface directories: {got:?}"
    );
}

#[test]
fn compose_magic_empty_partial_surfaces_no_directories() {
    // `@<TAB>` lists prompt filenames only, never directories.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    write(
        &tmp.path().join("prompts").join("plan.md"),
        "---\ntitle: X\n---\n",
    );
    fs::create_dir_all(tmp.path().join("planning")).unwrap();
    fs::create_dir_all(tmp.path().join("docs")).unwrap();

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "@");
    assert!(
        got.iter().any(|c| c == "@plan.md"),
        "empty magic partial must surface prompt files: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with('/')),
        "empty magic partial must not surface dirs: {got:?}"
    );
}

#[test]
fn compose_magic_path_shaped_surfaces_file_not_subdir() {
    // Path-shaped magic retains the shared scope and never emits a directory.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let prompts = tmp.path().join("prompts");
    write(&prompts.join("plan.md"), "---\ntitle: P\n---\n");
    fs::create_dir_all(prompts.join("planning")).unwrap();

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "@prompts/pl");
    assert!(
        got.iter().any(|c| c == "@prompts/plan.md"),
        "path-shaped magic must surface file: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with('/')),
        "path-shaped magic must not surface dirs: {got:?}"
    );
}

// -- dedup + sort ------------------------------------------------------

#[test]
fn finalize_dedups_and_sorts_by_rank() {
    let cs = vec![
        Candidate {
            insert: "prompts/b.md".into(),
            source_rank: 1,
        },
        Candidate {
            insert: "prompts/a.md".into(),
            source_rank: 0,
        },
        Candidate {
            insert: "prompts/a.md".into(),
            source_rank: 1,
        },
    ];
    let got = finalize(cs);
    assert_eq!(got, vec!["prompts/a.md", "prompts/b.md"]);
}

#[test]
fn committed_directory_output_is_portable_without_test_normalization() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    write(
        &tmp.path().join("prompts").join(r"nested\plan.md"),
        "---\ntitle: P\n---\n",
    );

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run(ComposeMode::Compose, &ctx, "prompts/");
    assert!(
        got.iter()
            .any(|candidate| candidate == "prompts/nested/plan.md"),
        "completion must emit slash-separated paths directly: {got:?}"
    );
}
