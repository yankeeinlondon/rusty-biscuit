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

// -- SetterToken::parse ------------------------------------------------

#[test]
fn parse_accepts_plain_setter() {
    let got = SetterToken::parse("spec=@s").unwrap();
    assert_eq!(got.name, "spec");
    assert_eq!(got.raw_value, "@s");
}

#[test]
fn parse_accepts_underscore_and_dash() {
    let got = SetterToken::parse("_foo-bar=val").unwrap();
    assert_eq!(got.name, "_foo-bar");
    assert_eq!(got.raw_value, "val");
}

#[test]
fn parse_accepts_empty_value() {
    let got = SetterToken::parse("spec=").unwrap();
    assert_eq!(got.name, "spec");
    assert_eq!(got.raw_value, "");
}

#[test]
fn parse_rejects_missing_equals() {
    assert!(SetterToken::parse("spec").is_none());
}

#[test]
fn parse_rejects_leading_equals() {
    assert!(SetterToken::parse("=value").is_none());
}

#[test]
fn parse_rejects_digit_first() {
    assert!(SetterToken::parse("1spec=val").is_none());
}

#[test]
fn parse_rejects_bad_char_in_name() {
    assert!(SetterToken::parse("sp ec=val").is_none());
    assert!(SetterToken::parse("sp.ec=val").is_none());
}

// -- strip_leading_quote -----------------------------------------------

#[test]
fn strip_leading_quote_handles_double_quote() {
    let (body, quote) = strip_leading_quote("\"@s");
    assert_eq!(body, "@s");
    assert_eq!(quote, Some('"'));
}

#[test]
fn strip_leading_quote_handles_single_quote() {
    let (body, quote) = strip_leading_quote("'@s");
    assert_eq!(body, "@s");
    assert_eq!(quote, Some('\''));
}

#[test]
fn strip_leading_quote_unquoted_is_passthrough() {
    let (body, quote) = strip_leading_quote("@s");
    assert_eq!(body, "@s");
    assert_eq!(quote, None);
}

#[test]
fn strip_leading_quote_empty_returns_empty() {
    let (body, quote) = strip_leading_quote("");
    assert_eq!(body, "");
    assert_eq!(quote, None);
}

// -- strip_file_extension ----------------------------------------------

#[test]
fn strip_file_extension_removes_last_extension() {
    assert_eq!(strip_file_extension("plan.md"), "plan");
    assert_eq!(strip_file_extension("plan.tar.gz"), "plan.tar");
    assert_eq!(strip_file_extension("no-ext"), "no-ext");
}

// -- run: @-gated trigger ---------------------------------------------

#[test]
fn run_returns_empty_for_non_at_value() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let docs = tmp.path().join("docs");
    write(&docs.join("plan.md"), "# p\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    assert!(run("spec=bar", &ctx).is_empty());
}

#[test]
fn run_returns_empty_for_empty_value() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let docs = tmp.path().join("docs");
    write(&docs.join("plan.md"), "# p\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    assert!(run("spec=", &ctx).is_empty());
}

#[test]
fn run_returns_empty_for_invalid_shape() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let ctx = ScopeContext::discover_from(tmp.path());
    assert!(run("not-a-setter", &ctx).is_empty());
    assert!(run("=value", &ctx).is_empty());
}

// -- run: repo-level scopes -------------------------------------------

#[test]
fn run_surfaces_repo_docs_under_at_sigil() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let docs = tmp.path().join("docs");
    write(&docs.join("spec.md"), "# s\n");
    write(&docs.join("unrelated.md"), "# u\n");

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("spec=@s", &ctx);
    assert!(
        got.iter().any(|c| c == "spec='docs/spec.md'"),
        "expected spec='docs/spec.md' in {got:?}"
    );
    // `u` is not a subsequence of `s` so unrelated.md should not match.
    assert!(
        !got.iter().any(|c| c.contains("unrelated.md")),
        "unrelated.md should not match on `s` prefix: {got:?}"
    );
}

#[test]
fn run_wraps_value_in_single_quotes_even_when_user_typed_double() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let docs = tmp.path().join("docs");
    write(&docs.join("spec.md"), "# s\n");

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("spec=\"@s", &ctx);
    assert!(
        got.iter().all(|c| !c.contains('"')),
        "double quote must be normalized to single quote: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "spec='docs/spec.md'"),
        "expected spec='docs/spec.md' after quote normalization: {got:?}"
    );
}

#[test]
fn run_wraps_value_in_single_quotes_when_user_typed_single() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let docs = tmp.path().join("docs");
    write(&docs.join("spec.md"), "# s\n");

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("spec='@s", &ctx);
    assert!(
        got.iter().any(|c| c == "spec='docs/spec.md'"),
        "expected spec='docs/spec.md' with single-quoted body: {got:?}"
    );
}

// -- run: empty @ returns everything in scope -------------------------

#[test]
fn run_empty_at_surfaces_every_file_in_scope() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    write(&tmp.path().join("docs").join("one.md"), "# 1\n");
    write(&tmp.path().join("features").join("two.md"), "# 2\n");

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("ref=@", &ctx);
    assert!(got.iter().any(|c| c == "ref='docs/one.md'"), "{got:?}");
    assert!(got.iter().any(|c| c == "ref='features/two.md'"), "{got:?}");
}

// -- run: scope resolution (repo + area + package) --------------------

#[test]
fn run_cwd_at_repo_root_only_repo_scope() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    write(&tmp.path().join("docs").join("plan.md"), "# p\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("spec=@p", &ctx);
    assert_eq!(got, vec!["spec='docs/plan.md'"]);
}

#[test]
fn run_cwd_inside_package_area_renders_relative_to_cwd() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["claudine/lib"]);
    // Feature doc under the area directory (the cwd).
    let area_feat = tmp.path().join("claudine").join("features");
    write(&area_feat.join("plan.md"), "# p\n");

    // cwd is the area directory itself; candidates anchor on it.
    let ctx = ScopeContext::discover_from(&tmp.path().join("claudine"));
    let got = run("spec=@p", &ctx);
    assert!(
        got.iter().any(|c| c == "spec='features/plan.md'"),
        "expected cwd-relative plan: {got:?}"
    );
}

#[test]
fn run_cwd_inside_package_renders_relative_to_cwd() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["claudine/lib"]);
    // Doc under the package directory (the cwd).
    let pkg_docs = tmp.path().join("claudine").join("lib").join("docs");
    write(&pkg_docs.join("pkg.md"), "# x\n");

    // cwd is the package directory; candidates anchor on it.
    let ctx = ScopeContext::discover_from(&tmp.path().join("claudine").join("lib"));
    let got = run("ref=@pk", &ctx);
    assert!(
        got.iter().any(|c| c == "ref='docs/pkg.md'"),
        "expected cwd-relative doc: {got:?}"
    );
}

#[test]
fn run_only_surfaces_files_under_cwd_not_repo_root() {
    // Regression: setter-value `@` completion walked the repo root, so a
    // user typing in a package area saw docs from the whole repo. It must
    // anchor on the launch `cwd` and surface only files beneath it.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["claudine/lib"]);
    // Repo-root-level doc (ABOVE the cwd) and a cwd-local doc.
    write(&tmp.path().join("docs").join("plan.md"), "# repo\n");
    write(
        &tmp.path().join("claudine").join("docs").join("plan.md"),
        "# area\n",
    );

    let ctx = ScopeContext::discover_from(&tmp.path().join("claudine"));
    let got = run("spec=@p", &ctx);
    assert_eq!(
        got,
        vec!["spec='docs/plan.md'".to_string()],
        "only the cwd-local docs/plan.md must surface, repo-root doc excluded: {got:?}"
    );
}

#[test]
fn run_returns_empty_when_no_matches() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    write(&tmp.path().join("docs").join("unrelated.md"), "# u\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("spec=@xyz", &ctx);
    assert!(
        got.is_empty(),
        "expected no matches for 'xyz' query: {got:?}"
    );
}

#[test]
fn run_excludes_directories() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    fs::create_dir_all(tmp.path().join("docs").join("planning")).unwrap();
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("spec=@pla", &ctx);
    // Even with a 3+ char prefix we never emit directories — setter
    // values target files only.
    assert!(
        !got.iter()
            .any(|c| c.contains("planning") && c.ends_with("/'")),
        "directory should not appear in setter-value output: {got:?}"
    );
}

// -- run: Markdown extension gate (finding #1) ------------------------

#[test]
fn setter_value_skips_txt_files() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let docs = tmp.path().join("docs");
    write(&docs.join("spec.md"), "# s\n");
    write(&docs.join("spec.txt"), "not markdown\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("spec=@s", &ctx);
    assert!(
        got.iter().any(|c| c == "spec='docs/spec.md'"),
        "expected .md to surface: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("spec.txt")),
        ".txt must be rejected by the extension gate: {got:?}"
    );
}

#[test]
fn setter_value_skips_yaml_files() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let docs = tmp.path().join("docs");
    write(&docs.join("plan.yaml"), "key: value\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("plan=@p", &ctx);
    assert!(
        !got.iter().any(|c| c.contains("plan.yaml")),
        ".yaml must be rejected by the extension gate: {got:?}"
    );
}

#[test]
fn setter_value_skips_extensionless_files() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let docs = tmp.path().join("docs");
    write(&docs.join("notes"), "just a text file\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("notes=@n", &ctx);
    assert!(
        !got.iter().any(|c| c.contains("'docs/notes'")),
        "extensionless files must be rejected: {got:?}"
    );
}

#[test]
fn setter_value_accepts_uppercase_md() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let docs = tmp.path().join("docs");
    write(&docs.join("PLAN.MD"), "# P\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("ref=@P", &ctx);
    assert!(
        got.iter().any(|c| c == "ref='docs/PLAN.MD'"),
        "uppercase .MD must be accepted (case-insensitive): {got:?}"
    );
}

#[test]
fn setter_value_accepts_uppercase_markdown() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let docs = tmp.path().join("docs");
    write(&docs.join("README.MARKDOWN"), "# R\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("ref=@R", &ctx);
    assert!(
        got.iter().any(|c| c == "ref='docs/README.MARKDOWN'"),
        "uppercase .MARKDOWN must be accepted (case-insensitive): {got:?}"
    );
}

// -- has_markdown_extension unit tests --------------------------------

#[test]
fn has_markdown_extension_accepts_md_and_markdown_case_insensitive() {
    assert!(has_markdown_extension(Path::new("a.md")));
    assert!(has_markdown_extension(Path::new("a.MD")));
    assert!(has_markdown_extension(Path::new("a.markdown")));
    assert!(has_markdown_extension(Path::new("a.Markdown")));
    assert!(has_markdown_extension(Path::new("a.MARKDOWN")));
    assert!(!has_markdown_extension(Path::new("a.txt")));
    assert!(!has_markdown_extension(Path::new("a.yaml")));
    assert!(!has_markdown_extension(Path::new("a")));
    assert!(!has_markdown_extension(Path::new("no-extension")));
}

#[test]
fn run_excludes_underscore_prefixed_files() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let docs = tmp.path().join("docs");
    write(&docs.join("_draft.md"), "# d\n");
    write(&docs.join("published.md"), "# p\n");
    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("spec=@", &ctx);
    assert!(
        !got.iter().any(|c| c.contains("_draft")),
        "underscore-prefixed files must be elided: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "spec='docs/published.md'"),
        "non-underscore file must surface: {got:?}"
    );
}

#[test]
fn run_orders_subdirs_then_path_within_cwd() {
    // With a single cwd anchor, candidates sort by subdir scope rank
    // (docs before features) then by relative path.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["claudine/lib"]);
    write(&tmp.path().join("docs").join("plan.md"), "# d\n");
    write(&tmp.path().join("features").join("plan.md"), "# f\n");

    let ctx = ScopeContext::discover_from(tmp.path());
    let got = run("spec=@p", &ctx);
    assert!(got.iter().any(|c| c == "spec='docs/plan.md'"), "{got:?}");
    assert!(
        got.iter().any(|c| c == "spec='features/plan.md'"),
        "{got:?}"
    );
    let idx_docs = got.iter().position(|c| c == "spec='docs/plan.md'").unwrap();
    let idx_feat = got
        .iter()
        .position(|c| c == "spec='features/plan.md'")
        .unwrap();
    assert!(
        idx_docs < idx_feat,
        "docs subdir must sort before features subdir: {got:?}"
    );
}

// -- scope resolution helpers -----------------------------------------

#[test]
fn resolve_setter_scopes_includes_all_four_subdirs() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let ctx = ScopeContext::discover_from(tmp.path());
    let scopes = resolve_setter_scopes(&ctx);
    for sub in SETTER_VALUE_SUBDIRS {
        let expected = tmp.path().join(sub);
        assert!(
            scopes.iter().any(|s| s.path == expected),
            "missing scope {sub}: {scopes:?}"
        );
    }
}

#[test]
fn resolve_setter_scopes_dedups_when_area_equals_repo() {
    // cwd at repo root: package_area_for_dir() returns None, so
    // no area-level scopes are produced. But the dedup logic also
    // protects against the case where they DO coincide.
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let ctx = ScopeContext::discover_from(tmp.path());
    let scopes = resolve_setter_scopes(&ctx);
    // No duplicate paths.
    let mut seen: HashSet<PathBuf> = HashSet::new();
    for scope in &scopes {
        assert!(
            seen.insert(scope.path.clone()),
            "duplicate scope path: {scope:?}"
        );
    }
}

#[test]
fn resolve_setter_scopes_all_follow_links() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let ctx = ScopeContext::discover_from(tmp.path());
    let scopes = resolve_setter_scopes(&ctx);
    for scope in &scopes {
        assert!(
            scope.follow_links,
            "setter scope must follow links: {scope:?}"
        );
    }
}

#[test]
fn format_relative_portably_renders_windows_shaped_segments() {
    let base = PathBuf::from("repo");
    let entry = base.join(r"docs\nested\plan.md");
    assert_eq!(
        format_relative(&base, &entry),
        Some("docs/nested/plan.md".to_string())
    );
}
