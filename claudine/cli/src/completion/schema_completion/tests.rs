use super::candidates::{property_value_hint, MatchGlobs};
use super::*;
use std::collections::HashSet;
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

fn effective_from_doc(doc: &str) -> EffectiveSchema {
    let md: Markdown = doc.into();
    DarkmatterSchemas::new()
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
fn property_value_match_pattern_anchors_on_cwd_not_repo_root() {
    // Regression: a `file(match(...))` property walked the effective repo
    // root, so a user completing inside a package area saw matches from
    // the whole repo — and the offered repo-relative path did not resolve
    // at runtime (read-side refs anchor on the launch `cwd`). The walk
    // must start at `cwd` and surface only files beneath it, rendered
    // cwd-relative.
    let effective = effective_from_doc(concat!(
        "---\n",
        "$schema:\n",
        "  review: \"file(match('**/*.md'))\"\n",
        "---\nbody\n",
    ));
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    // A doc above the cwd (repo root) and one under the cwd (package area).
    write(&tmp.path().join("docs").join("top.md"), "# top\n");
    write(
        &tmp.path().join("claudine").join("docs").join("area.md"),
        "# area\n",
    );

    let ctx = ScopeContext::discover_from(&tmp.path().join("claudine"));
    let got = property_value(&effective, "review", "", &ctx);
    assert!(
        got.iter().any(|c| c == "review='docs/area.md'"),
        "cwd-local doc must surface, rendered cwd-relative: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("top.md")),
        "repo-root doc above the cwd must NOT surface: {got:?}"
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

#[test]
fn match_globs_basename_pattern_matches_anywhere_in_tree() {
    let matcher = MatchGlobs::compile(&["*.png".to_string()]).unwrap();
    assert!(matcher.is_match("cover.png", "cover.png"));
    assert!(matcher.is_match("assets/cover.png", "cover.png"));
    assert!(matcher.is_match("a/b/c/cover.png", "cover.png"));
    assert!(!matcher.is_match("cover.jpg", "cover.jpg"));
}

#[test]
fn match_globs_honors_negation_against_basename() {
    let matcher =
        MatchGlobs::compile(&["*.md".to_string(), "!_*.md".to_string()]).unwrap();
    assert!(matcher.is_match("plan.md", "plan.md"));
    assert!(matcher.is_match("docs/plan.md", "plan.md"));
    assert!(!matcher.is_match("_draft.md", "_draft.md"));
    assert!(!matcher.is_match("docs/_draft.md", "_draft.md"));
    assert!(!matcher.is_match("notes.txt", "notes.txt"));
}

#[test]
fn match_globs_path_qualified_glob_matches_relative_path() {
    let matcher = MatchGlobs::compile(&["src/**/*.rs".to_string()]).unwrap();
    assert!(matcher.is_match("src/lib.rs", "lib.rs"));
    assert!(matcher.is_match("src/inner/mod.rs", "mod.rs"));
    // Files outside `src/` must NOT match a path-qualified pattern,
    // even when the basename would match `*.rs`.
    assert!(!matcher.is_match("tests/integration.rs", "integration.rs"));
    assert!(!matcher.is_match("benches/perf.rs", "perf.rs"));
}

#[test]
fn match_globs_path_qualified_negation_filters_subset() {
    let matcher = MatchGlobs::compile(&[
        "src/**/*.rs".to_string(),
        "!src/**/test_*.rs".to_string(),
    ])
    .unwrap();
    assert!(matcher.is_match("src/lib.rs", "lib.rs"));
    assert!(matcher.is_match("src/inner/mod.rs", "mod.rs"));
    assert!(!matcher.is_match("src/test_helpers.rs", "test_helpers.rs"));
    assert!(!matcher.is_match("src/inner/test_util.rs", "test_util.rs"));
}
