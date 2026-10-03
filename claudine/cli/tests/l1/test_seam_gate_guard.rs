//! A process-level test seam is an environment variable named
//! `CLAUDINE_TEST_*` that changes what the shipped binary does. Every such name
//! in `claudine/lib/src` and `claudine/cli/src` must sit inside an item or
//! statement whose `#[cfg(...)]` compiles it only in a test build, so a
//! default or installed build never honors it. The predicate is evaluated by
//! [`crate::cfg_gate`]: `#[cfg(all(unix, feature = "terminal-tests"))]`
//! qualifies, `#[cfg(any(feature = "test-fixtures", unix))]` does not. Gates
//! on enclosing modules, functions, and statements are ANDed, so one
//! test-only gate anywhere on the path is enough.
//!
//! The source is parsed with `syn`. A name inside a macro invocation's tokens
//! is invisible to the parser, so the guard also counts the name's raw
//! occurrences and fails when the two disagree. Files named `tests.rs` or under
//! a `tests/` directory are `#[cfg(test)]` modules declared by their parent and
//! are skipped.

use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};

use crate::cfg_gate::attrs_are_test_only;

const SEAM_PREFIX: &str = "CLAUDINE_TEST_";

/// Seams that exist today; finding them proves the scan reached both crates.
const KNOWN_SEAMS: &[&str] = &[
    "CLAUDINE_TEST_DESKTOP_NOTIFICATION",
    "CLAUDINE_TEST_DIAGNOSTIC_SNAPSHOT",
    "CLAUDINE_TEST_TEARDOWN_HOLD",
];

#[derive(Debug)]
struct Seam {
    file: String,
    line: usize,
    name: String,
    gated: bool,
}

fn expr_attrs(expr: &syn::Expr) -> &[syn::Attribute] {
    use syn::Expr;
    match expr {
        Expr::If(e) => &e.attrs,
        Expr::Call(e) => &e.attrs,
        Expr::MethodCall(e) => &e.attrs,
        Expr::Block(e) => &e.attrs,
        Expr::Macro(e) => &e.attrs,
        Expr::Match(e) => &e.attrs,
        Expr::Assign(e) => &e.attrs,
        _ => &[],
    }
}

struct SeamVisitor<'a> {
    file: &'a str,
    depth_gated: usize,
    seams: Vec<Seam>,
    malformed: Vec<String>,
}

impl SeamVisitor<'_> {
    fn scoped(&mut self, attrs: &[syn::Attribute], walk: impl FnOnce(&mut Self)) {
        // A predicate the evaluator cannot read is reported and never counted
        // as a gate.
        let gated = attrs_are_test_only(attrs).unwrap_or_else(|error| {
            self.malformed.push(format!("{}: {error}", self.file));
            false
        });
        self.depth_gated += usize::from(gated);
        walk(self);
        self.depth_gated -= usize::from(gated);
    }
}

impl<'ast> Visit<'ast> for SeamVisitor<'_> {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        let attrs: &[syn::Attribute] = match node {
            syn::Item::Fn(item) => &item.attrs,
            syn::Item::Const(item) => &item.attrs,
            syn::Item::Static(item) => &item.attrs,
            syn::Item::Mod(item) => &item.attrs,
            syn::Item::Impl(item) => &item.attrs,
            syn::Item::Struct(item) => &item.attrs,
            syn::Item::Enum(item) => &item.attrs,
            _ => &[],
        };
        self.scoped(attrs, |this| visit::visit_item(this, node));
    }

    fn visit_impl_item(&mut self, node: &'ast syn::ImplItem) {
        let attrs: &[syn::Attribute] = match node {
            syn::ImplItem::Fn(item) => &item.attrs,
            syn::ImplItem::Const(item) => &item.attrs,
            _ => &[],
        };
        self.scoped(attrs, |this| visit::visit_impl_item(this, node));
    }

    fn visit_stmt(&mut self, node: &'ast syn::Stmt) {
        let attrs: &[syn::Attribute] = match node {
            syn::Stmt::Local(local) => &local.attrs,
            syn::Stmt::Expr(expr, _) => expr_attrs(expr),
            syn::Stmt::Macro(mac) => &mac.attrs,
            syn::Stmt::Item(_) => &[],
        };
        self.scoped(attrs, |this| visit::visit_stmt(this, node));
    }

    fn visit_lit_str(&mut self, node: &'ast syn::LitStr) {
        let value = node.value();
        if value.starts_with(SEAM_PREFIX) {
            self.seams.push(Seam {
                file: self.file.to_string(),
                line: node.span().start().line,
                name: value,
                gated: self.depth_gated > 0,
            });
        }
    }
}

fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "tests") {
                rust_files(&path, files);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && path.file_name().is_some_and(|name| name != "tests.rs")
        {
            files.push(path);
        }
    }
}

fn scan_source<'a>(file: &'a str, source: &str) -> SeamVisitor<'a> {
    let syntax = syn::parse_file(source).unwrap_or_else(|error| panic!("parse {file}: {error}"));
    let mut visitor = SeamVisitor {
        file,
        depth_gated: 0,
        seams: Vec::new(),
        malformed: Vec::new(),
    };
    visitor.visit_file(&syntax);
    visitor
}

#[test]
fn every_test_seam_in_shipped_source_is_compiled_only_by_a_test_feature() {
    let cli_manifest = biscuit_test_harness::manifest_dir!();
    let area = cli_manifest
        .parent()
        .expect("cli crate has a parent package area");

    let mut seams = Vec::new();
    let mut unparsed = Vec::new();
    let mut malformed = Vec::new();
    for sub in ["lib/src", "cli/src"] {
        let mut files = Vec::new();
        rust_files(&area.join(sub), &mut files);
        assert!(
            !files.is_empty(),
            "no .rs files under {sub}: scan root is broken"
        );
        for path in files {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            if !source.contains(SEAM_PREFIX) {
                continue;
            }
            let file = path
                .strip_prefix(area)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let visitor = scan_source(&file, &source);
            let literal_count = source.matches(&format!("\"{SEAM_PREFIX}")).count();
            if literal_count != visitor.seams.len() {
                unparsed.push(format!(
                    "{file}: {literal_count} quoted `{SEAM_PREFIX}*` names, {} parsed \
                     (move the name out of the macro into a gated `const`)",
                    visitor.seams.len()
                ));
            }
            malformed.extend(visitor.malformed);
            seams.extend(visitor.seams);
        }
    }

    for known in KNOWN_SEAMS {
        assert!(
            seams.iter().any(|seam| seam.name == *known),
            "`{known}` was not found; the scan no longer reaches it:\n{seams:#?}"
        );
    }
    assert!(
        unparsed.is_empty(),
        "seams hidden in macro tokens:\n{}",
        unparsed.join("\n")
    );
    assert!(
        malformed.is_empty(),
        "cfg predicates the guard cannot evaluate:\n{}",
        malformed.join("\n")
    );
    let ungated: Vec<String> = seams
        .iter()
        .filter(|seam| !seam.gated)
        .map(|seam| format!("{}:{} `{}`", seam.file, seam.line, seam.name))
        .collect();
    assert!(
        ungated.is_empty(),
        "test seams compiled into a default build; gate each so that every enabling \
         configuration requires `test` or a feature in `cfg_gate::TEST_ONLY_FEATURES`, \
         e.g. `#[cfg(feature = \"test-fixtures\")]`:\n{}",
        ungated.join("\n")
    );
}

mod evaluator {
    use crate::cfg_gate::attrs_are_test_only;

    fn verdict(attrs: &str) -> Result<bool, String> {
        let item: syn::ItemFn = syn::parse_str(&format!("{attrs} fn gated() {{}}"))
            .unwrap_or_else(|error| panic!("fixture `{attrs}` parses: {error}"));
        attrs_are_test_only(&item.attrs)
    }

    #[track_caller]
    fn assert_test_only(attrs: &str) {
        assert_eq!(
            verdict(attrs),
            Ok(true),
            "`{attrs}` must count as test-only"
        );
    }

    #[track_caller]
    fn assert_production(attrs: &str) {
        assert_eq!(
            verdict(attrs),
            Ok(false),
            "`{attrs}` can compile into a shipped build"
        );
    }

    #[test]
    fn the_shipped_gate_shapes_are_test_only() {
        assert_test_only(r#"#[cfg(feature = "test-fixtures")]"#);
        assert_test_only(r#"#[cfg(all(unix, feature = "terminal-tests"))]"#);
        assert_test_only("#[cfg(test)]");
    }

    #[test]
    fn an_all_needs_one_test_only_child() {
        assert_test_only(r#"#[cfg(all(any(test, feature = "test-fixtures"), unix))]"#);
        assert_production("#[cfg(all(unix, windows))]");
        assert_production("#[cfg(all())]");
    }

    #[test]
    fn an_any_needs_every_child_test_only() {
        assert_production(r#"#[cfg(any(feature = "test-fixtures", unix))]"#);
        assert_production(r#"#[cfg(any(test, not(feature = "x")))]"#);
        assert_test_only(r#"#[cfg(any(test, feature = "daemon-tests"))]"#);
        assert_production("#[cfg(any())]");
    }

    #[test]
    fn a_negation_or_a_platform_is_never_test_only() {
        assert_production("#[cfg(not(test))]");
        assert_production(r#"#[cfg(not(not(feature = "test-fixtures")))]"#);
        assert_production("#[cfg(unix)]");
    }

    #[test]
    fn only_approved_features_count() {
        assert_production(r#"#[cfg(feature = "whatever")]"#);
        assert_production(r#"#[cfg(feature = "test-fixtures-extra")]"#);
        assert_production(r#"#[cfg(test_fixtures)]"#);
        assert_production(r#"#[cfg(target_os = "test-fixtures")]"#);
    }

    #[test]
    fn a_cfg_attr_does_not_gate_compilation() {
        assert_production(r#"#[cfg_attr(test, allow(dead_code))]"#);
        assert_production(r#"#[cfg_attr(feature = "test-fixtures", allow(dead_code))]"#);
        assert_test_only(r#"#[cfg_attr(test, allow(dead_code))] #[cfg(test)]"#);
    }

    #[test]
    fn a_malformed_predicate_is_rejected() {
        for attrs in [
            "#[cfg]",
            "#[cfg()]",
            "#[cfg(test, unix)]",
            "#[cfg(feature = 1)]",
            "#[cfg(not(test, unix))]",
            "#[cfg(core::test)]",
        ] {
            assert!(
                verdict(attrs).is_err(),
                "`{attrs}` must be rejected as malformed"
            );
        }
    }

    #[test]
    fn a_test_only_ancestor_gates_everything_under_it() {
        let source = r#"
            #[cfg(feature = "test-fixtures")]
            mod fixtures {
                #[cfg(unix)]
                fn stall() { let _ = "CLAUDINE_TEST_NESTED"; }
            }
            #[cfg(unix)]
            mod shipped {
                #[cfg_attr(test, allow(dead_code))]
                const SEAM: &str = "CLAUDINE_TEST_SHIPPED";
            }
            fn run() {
                #[cfg(any(feature = "test-fixtures", unix))]
                let _ = "CLAUDINE_TEST_STATEMENT";
            }
        "#;
        let visitor = super::scan_source("fixture.rs", source);
        let gated: Vec<(&str, bool)> = visitor
            .seams
            .iter()
            .map(|seam| (seam.name.as_str(), seam.gated))
            .collect();
        assert_eq!(
            gated,
            [
                ("CLAUDINE_TEST_NESTED", true),
                ("CLAUDINE_TEST_SHIPPED", false),
                ("CLAUDINE_TEST_STATEMENT", false),
            ]
        );
        assert!(visitor.malformed.is_empty(), "{:?}", visitor.malformed);
    }

    #[test]
    fn a_malformed_gate_is_reported_and_not_counted() {
        let visitor = super::scan_source(
            "fixture.rs",
            r#"#[cfg(test, unix)] const SEAM: &str = "CLAUDINE_TEST_MALFORMED";"#,
        );
        assert!(!visitor.seams[0].gated);
        assert_eq!(visitor.malformed.len(), 1, "{:?}", visitor.malformed);
    }
}
