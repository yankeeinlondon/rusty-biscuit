use super::*;

const MANIFEST: &str = "[package]\nname = \"fixture\"\nautotests = false\n\n\
    [[test]]\nname = \"l1\"\npath = \"tests/l1/main.rs\"\n";

fn tree(extra: &[(&str, &str)]) -> BTreeMap<String, String> {
    let mut files: BTreeMap<String, String> = [
        (
            "tests/l1/main.rs",
            "#[path = \"../common/mod.rs\"]\nmod common;\n\n#[cfg(windows)]\nmod windows_only;\nmod guard;\n",
        ),
        ("tests/l1/windows_only.rs", "#[test]\nfn case() {}\n"),
        ("tests/l1/guard.rs", "mod scan;\n"),
        ("tests/l1/guard/scan.rs", ""),
        ("tests/common/mod.rs", "pub(crate) mod pty;\n"),
        ("tests/common/pty.rs", ""),
    ]
    .into_iter()
    .map(|(path, source)| (path.to_string(), source.to_string()))
    .collect();
    for (path, source) in extra {
        files.insert(path.to_string(), source.to_string());
    }
    files
}

#[test]
fn a_fully_reached_layout_has_no_violations() {
    assert_eq!(layout_violations(MANIFEST, &tree(&[])), Vec::<String>::new());
}

#[test]
fn a_stray_top_level_file_is_rejected() {
    let stray = layout_violations(MANIFEST, &tree(&[("tests/wrap_env.rs", "#[test]\nfn a() {}\n")]));
    assert_eq!(stray.len(), 1);
    assert!(stray[0].starts_with("tests/wrap_env.rs: a top-level test file"), "{stray:?}");
}

#[test]
fn a_fixture_binary_declared_under_tests_is_reached() {
    // `claudine-cli`'s `claudine-fake-goose` lives in `tests/bin/` and is
    // compiled by its own `[[bin]]`, not by any test target.
    let manifest = format!(
        "{MANIFEST}\n[[bin]]\nname = \"fake\"\npath = \"tests/bin/fake/main.rs\"\n"
    );
    let files = tree(&[
        ("tests/bin/fake/main.rs", "mod support;\nfn main() {}\n"),
        ("tests/bin/fake/support.rs", ""),
    ]);
    assert_eq!(layout_violations(&manifest, &files), Vec::<String>::new());
    // Without the declaration, the same root is still an undeclared one.
    let undeclared = layout_violations(MANIFEST, &files);
    assert!(
        undeclared.iter().any(|violation| violation.starts_with("tests/bin/fake/main.rs: an undeclared")),
        "{undeclared:?}"
    );
}

#[test]
fn an_undeclared_crate_root_is_rejected() {
    let root = layout_violations(MANIFEST, &tree(&[("tests/level9/main.rs", "")]));
    assert_eq!(root.len(), 1);
    assert!(root[0].starts_with("tests/level9/main.rs: an undeclared test crate root"), "{root:?}");
}

#[test]
fn undeclared_modules_are_rejected_wherever_they_sit() {
    // Beside the root, inside a helper directory, and a declaration that exists
    // only in a comment or a string literal.
    let modules = layout_violations(
        MANIFEST,
        &tree(&[
            ("tests/l1/orphan.rs", ""),
            ("tests/l1/guard/unused.rs", ""),
            ("tests/l1/quoted.rs", ""),
            ("tests/common/wrap.rs", "// mod quoted;\nconst S: &str = \"mod quoted;\";\n"),
        ]),
    );
    assert_eq!(
        modules.iter().map(|v| v.split(':').next().unwrap()).collect::<Vec<_>>(),
        ["tests/common/wrap.rs", "tests/l1/guard/unused.rs", "tests/l1/orphan.rs", "tests/l1/quoted.rs"]
    );
}

#[test]
fn autotests_and_explicit_existing_paths_are_required() {
    let files = BTreeMap::from([("tests/l1/main.rs".to_string(), String::new())]);
    let autodiscovered = layout_violations(
        "[package]\nname = \"fixture\"\n\n[[test]]\nname = \"l1\"\npath = \"tests/l1/main.rs\"\n",
        &files,
    );
    assert_eq!(autodiscovered.len(), 1);
    assert!(autodiscovered[0].contains("autotests = false"));

    let unpathed = layout_violations(
        "[package]\nname = \"fixture\"\nautotests = false\n\n[[test]]\nname = \"l1\"\n\n\
         [[test]]\nname = \"gone\"\npath = \"tests/gone/main.rs\"\n",
        &files,
    );
    assert!(unpathed.iter().any(|v| v == "[[test]] l1: declare its `path` explicitly"), "{unpathed:?}");
    assert!(unpathed.iter().any(|v| v == "[[test]] gone: tests/gone/main.rs does not exist"), "{unpathed:?}");
    assert!(unpathed.iter().any(|v| v.starts_with("tests/l1/main.rs: an undeclared test crate root")));
}

#[test]
fn an_unparseable_manifest_is_reported_not_passed() {
    let violations = layout_violations("[package\n", &tree(&[]));
    assert_eq!(violations.len(), 1);
    assert!(violations[0].starts_with("Cargo.toml does not parse"), "{violations:?}");
}

#[test]
fn module_declarations_read_attributes_visibility_and_path() {
    let source = "//! mod in_docs;\n#[cfg(unix)]\n#[path = \"../common/mod.rs\"]\nmod common;\n\
                  pub(crate) mod helpers;\nmod inline { }\nlet remod = 1;\n";
    assert_eq!(
        module_declarations(source),
        [
            ModuleDeclaration { name: "common".into(), path: Some("../common/mod.rs".into()) },
            ModuleDeclaration { name: "helpers".into(), path: None },
        ]
    );
}

#[test]
fn declarations_resolve_like_rustc() {
    let common = &module_declarations("#[path = \"../common/mod.rs\"] mod common;")[0];
    assert_eq!(declared_module_files("tests/l1/main.rs", common), ["tests/common/mod.rs"]);

    let child = &module_declarations("mod source_scan;")[0];
    assert_eq!(
        declared_module_files("tests/l1/error_guards.rs", child),
        ["tests/l1/error_guards/source_scan.rs", "tests/l1/error_guards/source_scan/mod.rs"]
    );
    assert_eq!(
        declared_module_files("tests/l1/error_snapshots/mod.rs", child),
        ["tests/l1/error_snapshots/source_scan.rs", "tests/l1/error_snapshots/source_scan/mod.rs"]
    );
}

#[test]
fn literals_and_comments_hide_declarations_without_desynchronizing() {
    // A raw string with hashes, a char literal holding a quote, a lifetime, a
    // nested block comment, and prefixed literals beside a raw identifier each
    // precede a real declaration.
    let source = "const R: &str = r#\"mod raw; \"quoted\"\"#;\nmod after_raw;\n\
                  const C: char = '\"';\nmod after_char;\n\
                  fn f<'a>(x: &'a str) -> &'a str { x }\nmod after_lifetime;\n\
                  /* outer /* mod nested; */ mod still_comment; */\nmod after_comment;\n\
                  const K: &CStr = c\"mod c_string;\"; let r#type = b'\"';\nmod after_prefixed;\n";
    assert_eq!(
        module_declarations(source).into_iter().map(|d| d.name).collect::<Vec<_>>(),
        ["after_raw", "after_char", "after_lifetime", "after_comment", "after_prefixed"]
    );
    assert_eq!(sanitize(source).len(), source.len());
}

#[test]
fn a_module_token_inside_a_macro_declares_nothing() {
    // The macro is never expanded here, so rustc does not compile `orphan.rs`;
    // counting its token as a declaration would hide an orphaned test file.
    let dormant = "macro_rules! dormant {\n    () => { mod orphan; };\n}\nmod guard;\n";
    assert_eq!(
        module_declarations(dormant).into_iter().map(|d| d.name).collect::<Vec<_>>(),
        ["guard"]
    );
    // A token-discarding invocation, in each delimiter, and one nested inside
    // another macro's body.
    for source in [
        "discard!(mod orphan;);\nmod guard;\n",
        "discard![mod orphan;];\nmod guard;\n",
        "discard! { mod orphan; }\nmod guard;\n",
        "outer! { inner! { mod orphan; } }\nmod guard;\n",
        // Delimiters inside literals and comments must not unbalance the body.
        "discard! { \"}\" '}' /* ) */ mod orphan; }\nmod guard;\n",
        "discard! { ['\\x41','{']; mod orphan; }\nmod guard;\n",
        // Raw C strings, hashed and bare, hide a closing delimiter too.
        "macro_rules! discard { ($($token:tt)*) => {}; }\ndiscard! { cr#\"\"}\"\"#; mod orphan; }\nmod guard;\n",
        // Without hashes a backslash still ends nothing: `cr"\"` is one byte.
        "discard! { cr\"\\\"; mod orphan; }\nmod guard;\n",
        // Rust allows trivia between the macro path, the `!`, and what follows
        // (review 3); comments reach this pass already blanked to spaces.
        "macro_rules ! discard { ($($token:tt)*) => {} }\ndiscard ! { mod orphan; }\nmod guard;\n",
        "discard /* a */ ! // b\n { mod orphan; }\nmod guard;\n",
        "discard\n!\n(mod orphan;);\nmod guard;\n",
        // Path-qualified and raw-identifier invocation paths.
        "self::discard ! [mod orphan;];\nmod guard;\n",
        "crate :: m :: discard! { mod orphan; }\nmod guard;\n",
        "r#discard ! { mod orphan; }\nmod guard;\n",
        // A raw-identifier definition name, with and without trivia.
        "macro_rules! r#type { () => { mod orphan; }; }\nmod guard;\n",
        "macro_rules /* a */ ! /* b */ r#type\n{ () => { mod orphan; }; }\nmod guard;\n",
        // Rust identifiers are Unicode (XID), in definition names and in every
        // segment of an invocation path (review 4).
        "macro_rules! café { () => { mod orphan; }; }\nmod guard;\n",
        "café! { mod orphan; }\nmod guard;\n",
        "crate::m::café ! [mod orphan;];\nmod guard;\n",
        "данные!(mod orphan;);\nmod guard;\n",
        "macro_rules! r#données { () => { mod orphan; }; }\nmod guard;\n",
    ] {
        assert_eq!(
            module_declarations(source).into_iter().map(|d| d.name).collect::<Vec<_>>(),
            ["guard"],
            "{source}"
        );
    }
    // The walker must not mistake `!=` for a macro and blank what follows.
    let comparison = "fn f(a: usize, b: usize) -> bool { a != b }\nmod after;\n";
    assert_eq!(
        module_declarations(comparison).into_iter().map(|d| d.name).collect::<Vec<_>>(),
        ["after"]
    );
}

#[test]
fn a_bang_that_is_not_a_macro_leaves_real_declarations_visible() {
    // Inner attributes, `!=`, and unary `!` after an operator or a keyword are
    // not invocations; `real` sits in a block a misread `!` would blank.
    for source in [
        "#![allow(unused)]\nmod guard;\n",
        "fn f(a: u8, b: u8) -> bool { a != b }\nmod guard;\n",
        "fn f(a: u8, b: u8) -> bool { a != (b) }\nmod guard;\n",
        "const X: bool = ! Y;\nmod guard;\n",
        "fn f(x: bool) -> bool { if !x { return !{ #[path = \"real.rs\"] mod real; x }; } x }\nmod guard;\n",
        "fn f(x: bool) -> bool { match ! { #[path = \"real.rs\"] mod real; x } { _ => x } }\nmod guard;\n",
        "fn f(x: bool) -> bool { 'a: loop { break 'a ! { #[path = \"real.rs\"] mod real; x }; } }\nmod guard;\n",
    ] {
        let names = module_declarations(source).into_iter().map(|d| d.name).collect::<Vec<_>>();
        let expected: &[&str] = if source.contains("mod real") { &["real", "guard"] } else { &["guard"] };
        assert_eq!(names, expected, "{source}");
    }
}

#[test]
fn a_unicode_module_name_is_a_real_declaration() {
    // rustc requires `#[path]` for a non-ASCII module name (E0754).
    let source = "#[path = \"donnees.rs\"] mod données;\n#[path = \"namae.rs\"] pub(crate) mod 名前;\nmod guard;\n";
    let declarations = module_declarations(source);
    assert_eq!(declarations.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["données", "名前", "guard"]);
    assert_eq!(declarations[0].path.as_deref(), Some("donnees.rs"));
}

#[test]
fn a_file_reachable_only_through_a_dormant_macro_is_a_violation() {
    let violations = layout_violations(
        MANIFEST,
        &tree(&[
            (
                "tests/l1/guard.rs",
                "macro_rules! dormant {\n    () => { mod orphan; };\n}\nmod scan;\n",
            ),
            ("tests/l1/guard/orphan.rs", "#[test]\nfn case() {}\n"),
        ]),
    );
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert!(
        violations[0].starts_with("tests/l1/guard/orphan.rs: no declared test target compiles this module"),
        "{violations:?}"
    );
}

#[test]
fn a_file_named_only_after_a_raw_c_string_in_a_dormant_macro_is_a_violation() {
    let violations = layout_violations(
        MANIFEST,
        &tree(&[
            (
                "tests/l1/guard.rs",
                "macro_rules! discard { ($($token:tt)*) => {}; }\ndiscard! { cr#\"\"}\"\"#; mod orphan; }\nmod scan;\n",
            ),
            ("tests/l1/guard/orphan.rs", "#[test]\nfn case() {}\n"),
        ]),
    );
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert!(
        violations[0].starts_with("tests/l1/guard/orphan.rs: no declared test target compiles this module"),
        "{violations:?}"
    );
}

#[test]
fn a_file_named_only_in_a_spaced_raw_or_unicode_named_dormant_macro_is_a_violation() {
    for guard in [
        "macro_rules ! discard { ($($token:tt)*) => {} }\ndiscard ! { mod orphan; }\nmod scan;\n",
        "macro_rules! r#type { () => { mod orphan; }; }\nmod scan;\n",
        "r#discard /* a */ ! { mod orphan; }\nmod scan;\n",
        "macro_rules! café { () => { mod orphan; }; }\nmod scan;\n",
        "café! { mod orphan; }\nmod scan;\n",
    ] {
        let violations = layout_violations(
            MANIFEST,
            &tree(&[("tests/l1/guard.rs", guard), ("tests/l1/guard/orphan.rs", "#[test]\nfn case() {}\n")]),
        );
        assert_eq!(violations.len(), 1, "{guard}: {violations:?}");
        assert!(
            violations[0].starts_with("tests/l1/guard/orphan.rs: no declared test target compiles this module"),
            "{guard}: {violations:?}"
        );
    }
}

#[test]
fn collect_test_sources_skips_data_directories_and_uses_forward_slashes() {
    let root = tempfile::tempdir().expect("tempdir");
    for (path, source) in [
        ("tests/l1/main.rs", "mod a;\n"),
        ("tests/l1/a.rs", ""),
        ("tests/fixtures/sample.rs", ""),
        ("tests/l1/snapshots/nested.rs", ""),
        ("tests/l1/notes.md", ""),
    ] {
        let file = root.path().join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, source).unwrap();
    }
    let files = collect_test_sources(root.path()).expect("scan");
    assert_eq!(files.keys().collect::<Vec<_>>(), ["tests/l1/a.rs", "tests/l1/main.rs"]);
    assert_eq!(files["tests/l1/main.rs"], "mod a;\n");
}
