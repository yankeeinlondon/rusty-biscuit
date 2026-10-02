use super::candidates::{property_value_hint, MatchGlobs};
use super::*;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
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

fn effective_from_doc(doc: &str) -> EffectiveSchema {
    let md: Markdown = doc.into();
    DarkmatterSchemas::new(crate::request::test_context())
        .effective_for(&md)
        .unwrap()
        .expect("effective schema")
}

#[test]
fn property_names_required_first_then_optional() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  title: 'string(required)'\n",
        "  status: 'enum(draft, published; required)'\n",
        "  description: string\n",
        "  count: number\n",
        "---\nbody\n",
    ));
    let got = property_names(&effective, "", &HashSet::new(), &[]);
    // Without an authored-order hint the fall-back is required-first
    // then optional, in `IndexMap` iteration order. The actual order
    // within each group can vary because Darkmatter stores nested
    // frontmatter values as `serde_json::Value` (alphabetised), so
    // the only contract we can assert here is the group boundary.
    let pos = |needle: &str| got.iter().position(|c| c == needle).unwrap();
    assert!(pos("status=") < pos("description="));
    assert!(pos("status=") < pos("count="));
    assert!(pos("title=") < pos("description="));
    assert!(pos("title=") < pos("count="));
    assert_eq!(got.len(), 4);
}

#[test]
fn property_names_respects_declared_order_within_groups() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  title: 'string(required)'\n",
        "  status: 'enum(draft, published; required)'\n",
        "  description: string\n",
        "  count: number\n",
        "---\nbody\n",
    ));
    let declared_order = vec![
        "title".to_string(),
        "status".to_string(),
        "description".to_string(),
        "count".to_string(),
    ];
    let got = property_names(&effective, "", &HashSet::new(), &declared_order);
    assert_eq!(
        got,
        vec![
            "title=".to_string(),
            "status=".to_string(),
            "description=".to_string(),
            "count=".to_string(),
        ],
        "required group must preserve `title` before `status`, optional \
         group must preserve `description` before `count`",
    );

    // Reversing the authored order must reverse the within-group output.
    let reversed = vec![
        "count".to_string(),
        "description".to_string(),
        "status".to_string(),
        "title".to_string(),
    ];
    let got = property_names(&effective, "", &HashSet::new(), &reversed);
    assert_eq!(
        got,
        vec![
            "status=".to_string(),
            "title=".to_string(),
            "count=".to_string(),
            "description=".to_string(),
        ],
    );
}

#[test]
fn property_names_offers_root_union_arm_properties() {
    // A root union (`$schema:` sequence) where each arm declares a single
    // file-typed property must offer every arm's property name, in arm
    // order. Regression: `single_shape` returned None for unions so the
    // setter-name slot produced nothing.
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  - spec: \"file(match('**/spec*.md'))\"\n",
        "  - design: \"file(match('**/design*.md'))\"\n",
        "---\nbody\n",
    ));
    let got = property_names(&effective, "", &HashSet::new(), &[]);
    assert_eq!(got, vec!["spec=".to_string(), "design=".to_string()]);
}

#[test]
fn property_value_offers_files_for_root_union_arm() {
    // The value slot for a root-union arm's file property must surface the
    // arm's `match(...)` candidates.
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  - spec: \"file(match('**/spec*.md'))\"\n",
        "  - design: \"file(match('**/design*.md'))\"\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join("features").join("a").join("spec.md"), "# s\n");
    write(
        &tmp.path().join("features").join("b").join("design.md"),
        "# d\n",
    );
    let ctx = ScopeContext::discover_from(tmp.path());

    let spec = property_value(&effective, "spec", "", &ctx);
    assert_eq!(spec, vec!["spec='features/a/spec.md'".to_string()]);

    let design = property_value(&effective, "design", "", &ctx);
    assert_eq!(design, vec!["design='features/b/design.md'".to_string()]);
}

#[test]
fn property_names_filters_supplied() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  title: 'string(required)'\n",
        "  description: string\n",
        "---\nbody\n",
    ));
    let mut supplied = HashSet::new();
    supplied.insert("title".to_string());
    let got = property_names(&effective, "", &supplied, &[]);
    assert_eq!(got, vec!["description="]);
}

#[test]
fn property_names_fuzzy_matches_partial() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  title: 'string(required)'\n",
        "  description: string\n",
        "---\nbody\n",
    ));
    let got = property_names(&effective, "des", &HashSet::new(), &[]);
    assert_eq!(got, vec!["description="]);
}

#[test]
fn declared_property_order_returns_authored_keys_for_inline_schema() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(
        &tmp.path().join("prompt.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "  status: 'enum(draft, published; required)'\n",
            "  description: string\n",
            "  count: number\n",
            "---\nbody\n",
        ),
    );
    let ctx = ScopeContext::discover_from(tmp.path());
    let order = declared_property_order("prompt.md", &ctx);
    assert_eq!(
        order,
        vec![
            "title".to_string(),
            "status".to_string(),
            "description".to_string(),
            "count".to_string(),
        ],
    );
}

#[test]
fn declared_property_order_returns_empty_for_root_union_schema() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(
        &tmp.path().join("p.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  - title: 'string(required)'\n",
            "  - name: 'string(required)'\n",
            "---\nbody\n",
        ),
    );
    let ctx = ScopeContext::discover_from(tmp.path());
    let order = declared_property_order("p.md", &ctx);
    assert!(
        order.is_empty(),
        "root unions have no single ordered property set: {order:?}",
    );
}

#[test]
fn declared_property_order_follows_yaml_file_reference() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(
        &tmp.path().join("schema.yaml"),
        "$schema:\n  zeta: 'string(required)'\n  alpha: number\n",
    );
    write(
        &tmp.path().join("p.md"),
        "---\n$schema: ./schema.yaml\n---\nbody\n",
    );
    let ctx = ScopeContext::discover_from(tmp.path());
    let order = declared_property_order("p.md", &ctx);
    assert_eq!(order, vec!["zeta".to_string(), "alpha".to_string()]);
}

#[test]
fn declared_property_order_returns_empty_when_no_frontmatter() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join("p.md"), "no frontmatter here\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    assert!(declared_property_order("p.md", &ctx).is_empty());
}

#[test]
fn property_value_returns_enum_members() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  status: 'enum(draft, published, archived; required)'\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = property_value(&effective, "status", "", &ctx);
    assert!(got.contains(&"status='draft'".to_string()));
    assert!(got.contains(&"status='published'".to_string()));
    assert!(got.contains(&"status='archived'".to_string()));
}

#[test]
fn property_value_filters_enum_by_partial() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  status: 'enum(draft, published, archived; required)'\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = property_value(&effective, "status", "pub", &ctx);
    assert_eq!(got, vec!["status='published'".to_string()]);
}

#[test]
fn property_value_returns_files_for_match_pattern() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  cover: \"file(match('*.png'))\"\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join("assets").join("cover.png"), "");
    write(&tmp.path().join("assets").join("other.jpg"), "");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = property_value(&effective, "cover", "", &ctx);
    assert!(
        got.iter().any(|c| c.ends_with("cover.png'")),
        "expected cover.png in candidates: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("other.jpg")),
        "non-matching extension must be filtered: {got:?}"
    );
}

#[test]
fn property_value_match_pattern_excludes_underscore_dirs_and_files() {
    // Regression: a `file(match(...))` property walked the repo with a
    // bespoke `WalkBuilder` that honored only `.hidden`/gitignore, so
    // `_`-prefixed archive directories (`_completed/`, `_unscheduled/`)
    // and `_`-prefixed files leaked into completion. The match path must
    // share the scope walker's `_`-prefix exclusion.
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  spec: \"file(match('**/*spec*.md'))\"\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(
        &tmp.path().join("features").join("live").join("spec.md"),
        "# live\n",
    );
    write(
        &tmp.path()
            .join("features")
            .join("_completed")
            .join("done")
            .join("spec.md"),
        "# done\n",
    );
    write(&tmp.path().join("_draft-spec.md"), "# draft\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = property_value(&effective, "spec", "", &ctx);
    assert!(
        got.iter().any(|c| c == "spec='features/live/spec.md'"),
        "live spec must surface: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("_completed")),
        "_completed dir must be elided from match path: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("_draft-spec.md")),
        "_-prefixed file must be elided from match path: {got:?}"
    );
}

#[test]
fn property_value_match_pattern_filters_by_path_substring() {
    // The typed partial is a `*active*` substring filter over the
    // repo-relative path, so a directory fragment narrows candidates that
    // share a basename (`spec.md`).
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  spec: \"file(match('**/*spec*.md'))\"\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(
        &tmp.path().join("features").join("realwork").join("spec.md"),
        "# r\n",
    );
    write(
        &tmp.path().join("features").join("other").join("spec.md"),
        "# o\n",
    );
    let ctx = ScopeContext::discover_from(tmp.path());

    let got = property_value(&effective, "spec", "real", &ctx);
    assert_eq!(
        got,
        vec!["spec='features/realwork/spec.md'".to_string()],
        "partial must filter by directory substring, not basename: {got:?}"
    );

    // Case-insensitive.
    let got_upper = property_value(&effective, "spec", "REAL", &ctx);
    assert_eq!(got_upper, got, "substring filter must be case-insensitive");

    // A fragment matching no path returns nothing.
    assert!(property_value(&effective, "spec", "zzz", &ctx).is_empty());
}

#[test]
fn property_value_emits_windows_shaped_path_portably() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  spec: \"file(match('**/*spec*.md'))\"\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join(r"features\live\spec.md"), "# live\n");
    let ctx = ScopeContext::discover_from(tmp.path());

    assert_eq!(
        property_value(&effective, "spec", "features/live", &ctx),
        vec!["spec='features/live/spec.md'".to_string()]
    );
}

#[test]
fn property_value_bare_match_offers_the_launch_folder_then_the_repository_root() {
    // A bare pattern runs under the launch directory, then the repository
    // root. A file outside the launch directory is spelled so it resolves
    // from there (`../docs/top.md`), never as a repository-relative path that
    // would not.
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  review: \"file(match('**/*.md'))\"\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    // A real repository: a bare `.git` directory is not one to a prepared
    // context, which would leave `../` outside the tree.
    git_init(tmp.path());
    write(&tmp.path().join("docs").join("top.md"), "# top\n");
    write(
        &tmp.path().join("claudine").join("docs").join("area.md"),
        "# area\n",
    );

    let ctx = ScopeContext::discover_from(&tmp.path().join("claudine"));
    let got = property_value(&effective, "review", "", &ctx);
    assert_eq!(
        got,
        ["review='docs/area.md'", "review='../docs/top.md'"],
        "launch-folder files first, in their bare spelling: {got:?}"
    );
}

#[test]
fn file_candidate_paths_match_pattern_excludes_underscore_dirs() {
    // The ENTER-path chooser shares the same exclusion contract.
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(
        &tmp.path().join("features").join("live").join("spec.md"),
        "# live\n",
    );
    write(
        &tmp.path()
            .join("features")
            .join("_completed")
            .join("spec.md"),
        "# done\n",
    );
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = file_candidate_paths(&["**/*spec*.md".to_string()], &ctx);
    assert!(
        got.iter().any(|p| p.ends_with("features/live/spec.md")),
        "live spec must surface: {got:?}"
    );
    assert!(
        !got.iter()
            .any(|p| p.components().any(|c| c.as_os_str() == "_completed")),
        "_completed dir must be elided from ENTER-path match walk: {got:?}"
    );
}

#[test]
fn property_value_returns_empty_for_string_property() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  title: 'string(required)'\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = property_value(&effective, "title", "", &ctx);
    assert!(got.is_empty());
}

#[test]
fn property_value_falls_back_to_default_glob_for_bare_file() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  cover: file\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join("readme.md"), "# R\n");
    write(&tmp.path().join("a.txt"), "text\n");
    write(&tmp.path().join("prompts").join("plan.md"), "# P\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = property_value(&effective, "cover", "", &ctx);
    assert!(
        got.iter().any(|c| c == "cover='readme.md'"),
        "bare file must fall back to default glob markdown candidates: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("a.txt")),
        "non-markdown file must be excluded by default glob: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("prompts")),
        "prompt directory must be excluded from default glob: {got:?}"
    );
}

#[test]
fn property_value_file_array_first_file_completion() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  attachments: file[]\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join("notes.md"), "# N\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = property_value(&effective, "attachments", "", &ctx);
    assert!(
        got.iter().any(|c| c == "attachments='notes.md'"),
        "file[] first file must complete from default glob: {got:?}"
    );
}

#[test]
fn property_value_file_array_comma_continuation_excludes_prior_files() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  attachments: file[]\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join("a.md"), "# A\n");
    write(&tmp.path().join("b.md"), "# B\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = property_value(&effective, "attachments", "a.md,", &ctx);
    assert!(
        got.iter().any(|c| c == "attachments='a.md,b.md'"),
        "trailing comma must re-open completion excluding prior file: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "attachments='a.md,a.md'"),
        "already-selected file must be excluded: {got:?}"
    );
}

#[test]
fn property_value_file_array_continuation_filters_by_active_partial() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  attachments: file[]\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join("alpha.md"), "# A\n");
    write(&tmp.path().join("beta.md"), "# B\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = property_value(&effective, "attachments", "alpha.md,b", &ctx);
    assert!(
        got.iter().any(|c| c == "attachments='alpha.md,beta.md'"),
        "active partial must filter continuation candidates: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("alpha.md,alpha.md")),
        "prior file must be excluded even when active partial matches it: {got:?}"
    );
}

#[test]
fn property_value_file_array_continuation_honors_unclosed_quote() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  attachments: file[]\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join("a.md"), "# A\n");
    write(&tmp.path().join("b.md"), "# B\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = property_value(&effective, "attachments", "'a.md,b", &ctx);
    assert!(
        got.iter().any(|c| c == "attachments='a.md,b.md'"),
        "unclosed quote must still produce a single-quoted candidate: {got:?}"
    );
}

#[test]
fn property_value_hint_returns_format_for_url() {
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  homepage: url\n",
        "---\nbody\n",
    ));
    let hint = property_value_hint(&effective, "homepage");
    assert!(hint.unwrap_or("").contains("URL"));
}

#[test]
fn load_effective_schema_resolves_cwd_relative_path() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    let doc_path = tmp.path().join("prompt.md");
    write(
        &doc_path,
        "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
    );
    let ctx = ScopeContext::discover_from(tmp.path());
    let effective = load_effective_schema("prompt.md", &ctx).expect("schema loads");
    let suggestions = ordered_completable_suggestions(&effective);
    // `title` is `string` so it's NOT a completable type.
    assert!(suggestions.is_empty(), "string is not completable");
}

#[test]
fn load_effective_schema_returns_none_for_missing_file() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    let ctx = ScopeContext::discover_from(tmp.path());
    assert!(load_effective_schema("does-not-exist.md", &ctx).is_none());
}

#[test]
fn load_effective_schema_returns_none_when_no_schema() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(&tmp.path().join("p.md"), "---\ntitle: hi\n---\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    assert!(load_effective_schema("p.md", &ctx).is_none());
}

#[test]
fn load_effective_schema_strips_surrounding_quotes() {
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    write(
        &tmp.path().join("p.md"),
        "---\n$schema:\n  status: 'enum(a, b)'\n---\n",
    );
    let ctx = ScopeContext::discover_from(tmp.path());
    assert!(load_effective_schema("'p.md'", &ctx).is_some());
    assert!(load_effective_schema("\"p.md\"", &ctx).is_some());
}

/// A launch context the `match()` globs are judged from; the judgment is
/// lexical, so no file needs to exist.
fn launch_context() -> biscuit_file::FileResolutionContext {
    biscuit_file::FileResolutionContext::new(std::env::temp_dir())
}

#[test]
fn match_globs_basename_pattern_matches_anywhere_in_tree() {
    let matcher = MatchGlobs::new(&["*.png".to_string()]).unwrap();
    let context = launch_context();
    assert!(matcher.matches(Path::new("cover.png"), &context));
    assert!(matcher.matches(Path::new("assets/cover.png"), &context));
    assert!(matcher.matches(Path::new("a/b/c/cover.png"), &context));
    assert!(!matcher.matches(Path::new("cover.jpg"), &context));
}

#[test]
fn match_globs_honors_negation_against_basename() {
    let matcher =
        MatchGlobs::new(&["*.md".to_string(), "!_*.md".to_string()]).unwrap();
    let context = launch_context();
    assert!(matcher.matches(Path::new("plan.md"), &context));
    assert!(matcher.matches(Path::new("docs/plan.md"), &context));
    assert!(!matcher.matches(Path::new("_draft.md"), &context));
    assert!(!matcher.matches(Path::new("docs/_draft.md"), &context));
    assert!(!matcher.matches(Path::new("notes.txt"), &context));
}

#[test]
fn match_globs_path_qualified_glob_matches_relative_path() {
    let matcher = MatchGlobs::new(&["src/**/*.rs".to_string()]).unwrap();
    let context = launch_context();
    assert!(matcher.matches(Path::new("src/lib.rs"), &context));
    assert!(matcher.matches(Path::new("src/inner/mod.rs"), &context));
    // Files outside `src/` must NOT match a path-qualified pattern,
    // even when the basename would match `*.rs`.
    assert!(!matcher.matches(Path::new("tests/integration.rs"), &context));
    assert!(!matcher.matches(Path::new("benches/perf.rs"), &context));
}

#[test]
fn match_globs_path_qualified_negation_filters_subset() {
    let matcher = MatchGlobs::new(&[
        "src/**/*.rs".to_string(),
        "!src/**/test_*.rs".to_string(),
    ])
    .unwrap();
    let context = launch_context();
    assert!(matcher.matches(Path::new("src/lib.rs"), &context));
    assert!(matcher.matches(Path::new("src/inner/mod.rs"), &context));
    assert!(!matcher.matches(Path::new("src/test_helpers.rs"), &context));
    assert!(!matcher.matches(Path::new("src/inner/test_util.rs"), &context));
}

// ── `match()` glob references: roots, order, spelling ─────────────────────

/// A canonical temporary directory, so expected paths compare equal on
/// macOS, where `/var` is a link to `/private/var`.
fn canonical_tempdir() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(tmp.path()).unwrap();
    (tmp, root)
}

fn git_init(root: &Path) {
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status()
        .expect("run git");
    assert!(status.success());
}

/// `{root}` as a Git repository with workspace package `pkg`.
fn repository_with_package(root: &Path) {
    git_init(root);
    write(&root.join("Cargo.toml"), "[workspace]\nmembers = [\"pkg\"]\n");
    write(
        &root.join("pkg").join("Cargo.toml"),
        "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&root.join("pkg").join("src").join("lib.rs"), "");
}

fn spec_schema(patterns: &str) -> EffectiveSchema {
    effective_from_doc(&format!(
        "---\n$schema:\n  spec: \"file(required;eager;match({patterns}))\"\n---\nbody\n"
    ))
}

fn patterns(list: &[&str]) -> Vec<String> {
    list.iter().map(|pattern| pattern.to_string()).collect()
}

/// The `spec` values the TAB walk offers for `partial`.
fn offered(effective: &EffectiveSchema, partial: &str, ctx: &ScopeContext) -> Vec<String> {
    property_value(effective, "spec", partial, ctx)
        .into_iter()
        .map(|candidate| {
            candidate
                .strip_prefix("spec='")
                .and_then(|rest| rest.strip_suffix('\''))
                .expect("a `spec='…'` candidate")
                .to_string()
        })
        .collect()
}

/// Every offered value resolves back to the file the ENTER walk lists at
/// the same position, and `match()` validation admits it (one-way parity).
fn assert_offers_resolve_and_are_admitted(effective: &EffectiveSchema, globs: &[String], ctx: &ScopeContext) {
    let resolution = crate::completion::scopes::file_resolution_context(ctx).unwrap();
    let values = offered(effective, "", ctx);
    let paths = file_candidate_paths(globs, ctx);
    assert_eq!(values.len(), paths.len(), "{values:?} vs {paths:?}");
    let matcher = MatchGlobs::new(globs).unwrap();
    for (value, path) in values.iter().zip(&paths) {
        assert!(!value.contains("{{"), "{value}");
        let resolved = biscuit_file::FileReference::new(value)
            .unwrap()
            .resolve_in_context(&resolution)
            .unwrap()
            .unwrap_or_else(|| panic!("`{value}` resolves"));
        assert_eq!(
            biscuit_file::canonicalize_simplified(&resolved).unwrap(),
            biscuit_file::canonicalize_simplified(path).unwrap(),
            "`{value}` resolves back to the walked file"
        );
        assert!(matcher.matches(path, &resolution), "{}", path.display());
        assert!(
            darkmatter::markdown::schemas::file_match::file_match_admits(value, globs, &resolution),
            "`{value}` is admitted"
        );
    }
}

#[test]
fn incident_2_completes_a_caret_pattern_from_the_repository_root() {
    let (_tmp, repo) = canonical_tempdir();
    repository_with_package(&repo);
    write(&repo.join("fixes/2026-09-29-ts-review-improvements/spec.md"), "# fix\n");
    write(&repo.join("fixes/2026-09-01-other/spec.md"), "# other\n");
    let effective = spec_schema("^**/*spec*.md");
    let ctx = ScopeContext::discover_from(&repo);

    assert_eq!(
        property_value(&effective, "spec", "ts-review", &ctx),
        ["spec='fixes/2026-09-29-ts-review-improvements/spec.md'"]
    );
    assert_offers_resolve_and_are_admitted(&effective, &patterns(&["^**/*spec*.md"]), &ctx);
}

#[test]
fn a_nested_launch_lists_each_prefix_in_native_order() {
    let (_tmp, repo) = canonical_tempdir();
    repository_with_package(&repo);
    for file in ["zz-spec.md", "fixes/y/spec.md", "pkg/spec.md", "pkg/b/spec.md", "pkg/a/spec.md"] {
        write(&repo.join(file), "# spec\n");
    }
    let ctx = ScopeContext::discover_from(&repo.join("pkg"));

    // Package root first (shallowest, then component-wise), then the
    // repository root's own files; a sort on the rendered text would put
    // every `../` first.
    let cases: [(&str, &[&str]); 3] = [
        ("^**/*spec*.md", &["spec.md", "a/spec.md", "b/spec.md", "../zz-spec.md", "../fixes/y/spec.md"]),
        ("&**/*spec*.md", &["../zz-spec.md", "spec.md", "../fixes/y/spec.md", "a/spec.md", "b/spec.md"]),
        ("./**/*spec*.md", &["spec.md", "a/spec.md", "b/spec.md"]),
    ];
    for (pattern, expected) in cases {
        let effective = spec_schema(pattern);
        assert_eq!(offered(&effective, "", &ctx), expected, "{pattern}");
        assert_offers_resolve_and_are_admitted(&effective, &patterns(&[pattern]), &ctx);
    }
}

#[test]
fn a_root_exclusion_removes_files_a_positive_caret_pattern_admits() {
    let (_tmp, repo) = canonical_tempdir();
    repository_with_package(&repo);
    write(&repo.join("live/spec.md"), "# live\n");
    write(&repo.join("done/spec.md"), "# done\n");
    write(&repo.join("fixes/_completed/x/spec.md"), "# archived\n");
    let ctx = ScopeContext::discover_from(&repo);
    let resolution = crate::completion::scopes::file_resolution_context(&ctx).unwrap();

    let effective = spec_schema("^**/*spec*.md, !&**/done/**");
    assert_eq!(offered(&effective, "", &ctx), ["live/spec.md"]);
    assert_offers_resolve_and_are_admitted(&effective, &patterns(&["^**/*spec*.md", "!&**/done/**"]), &ctx);

    // The walk never offers `_completed`; the exclusion is what rejects a
    // typed value under it.
    let archived = repo.join("fixes/_completed/x/spec.md");
    assert!(MatchGlobs::new(&patterns(&["^**/*spec*.md"])).unwrap().matches(&archived, &resolution));
    assert!(!MatchGlobs::new(&patterns(&["^**/*spec*.md", "!&**/_completed/**"]))
        .unwrap()
        .matches(&archived, &resolution));
}

#[test]
fn each_file_is_judged_by_its_nearest_root() {
    let (_tmp, repo) = canonical_tempdir();
    repository_with_package(&repo);
    let inside = repo.join("pkg/fixes/x/spec.md");
    let root_fix = repo.join("fixes/y/spec.md");
    let other = repo.join("other/z/spec.md");
    for file in [&inside, &root_fix, &other] {
        write(file, "# spec\n");
    }
    let globs = patterns(&["**/*spec*.md", "!fixes/**"]);
    let effective = spec_schema("**/*spec*.md, !fixes/**");
    let ctx = ScopeContext::discover_from(&repo.join("pkg"));
    let resolution = crate::completion::scopes::file_resolution_context(&ctx).unwrap();

    assert_eq!(offered(&effective, "", &ctx), ["../other/z/spec.md"]);
    let matcher = MatchGlobs::new(&globs).unwrap();
    assert!(!matcher.matches(&inside, &resolution));
    assert!(!matcher.matches(&root_fix, &resolution));
    assert!(matcher.matches(&other, &resolution));
    assert_offers_resolve_and_are_admitted(&effective, &globs, &ctx);
}

#[test]
fn a_file_name_negation_is_neither_offered_nor_admitted() {
    let (_tmp, repo) = canonical_tempdir();
    repository_with_package(&repo);
    write(&repo.join("docs/_x.md"), "# hidden by name\n");
    write(&repo.join("docs/y.md"), "# kept\n");
    let globs = patterns(&["*.md", "!_*.md"]);
    let effective = spec_schema("*.md, !_*.md");
    let ctx = ScopeContext::discover_from(&repo);
    let resolution = crate::completion::scopes::file_resolution_context(&ctx).unwrap();

    let values = offered(&effective, "", &ctx);
    assert!(values.contains(&"docs/y.md".to_string()), "{values:?}");
    assert!(!values.iter().any(|value| value.contains("_x.md")), "{values:?}");
    assert!(!file_candidate_paths(&globs, &ctx).iter().any(|path| path.ends_with("docs/_x.md")));
    assert!(!MatchGlobs::new(&globs).unwrap().matches(&repo.join("docs/_x.md"), &resolution));
    assert_offers_resolve_and_are_admitted(&effective, &globs, &ctx);
}

#[test]
fn candidates_outside_the_launch_folder_take_the_first_form_that_resolves() {
    let (_tmp, root) = canonical_tempdir();
    let repo = root.join("repo");
    let home = root.join("home");
    let outside = root.join("outside");
    std::fs::create_dir_all(&repo).unwrap();
    repository_with_package(&repo);
    write(&repo.join("top/x/spec.md"), "# repository\n");
    write(&home.join("notes/spec.md"), "# home\n");
    write(&outside.join("spec.md"), "# absolute\n");
    let launch = repo.join("pkg/deep");
    std::fs::create_dir_all(&launch).unwrap();
    let mut ctx = ScopeContext::discover_from(&launch);
    ctx.home = Some(home.clone());

    // Two levels up is no restricted relative form, so `&`.
    let effective = spec_schema("&top/**/*spec*.md");
    assert_eq!(offered(&effective, "", &ctx), ["&top/x/spec.md"]);
    assert_offers_resolve_and_are_admitted(&effective, &patterns(&["&top/**/*spec*.md"]), &ctx);

    // Outside the repository, under home: `~/`.
    let effective = spec_schema("~/notes/*spec*.md");
    assert_eq!(offered(&effective, "", &ctx), ["~/notes/spec.md"]);
    assert_offers_resolve_and_are_admitted(&effective, &patterns(&["~/notes/*spec*.md"]), &ctx);

    // Outside both: the absolute path.
    let absolute = format!("{}/*spec*.md", biscuit_file::to_portable_string(&outside));
    let effective = spec_schema(&absolute);
    let values = offered(&effective, "", &ctx);
    assert_eq!(values.len(), 1, "{values:?}");
    assert!(Path::new(&values[0]).is_absolute(), "{values:?}");
    assert_offers_resolve_and_are_admitted(&effective, &patterns(&[&absolute]), &ctx);
}

#[cfg(unix)]
#[test]
fn an_out_of_tree_file_symlink_is_omitted_without_a_warning() {
    let (_tmp, root) = canonical_tempdir();
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    repository_with_package(&repo);
    write(&root.join("secret.md"), "# outside\n");
    write(&repo.join("docs/real.md"), "# real\n");
    std::os::unix::fs::symlink(root.join("secret.md"), repo.join("docs/leak.md")).unwrap();
    let globs = patterns(&["docs/*.md"]);
    let effective = spec_schema("docs/*.md");
    let ctx = ScopeContext::discover_from(&repo);

    assert_eq!(offered(&effective, "", &ctx), ["docs/real.md"]);
    assert_eq!(file_candidate_paths(&globs, &ctx), [repo.join("docs/real.md")]);
}
