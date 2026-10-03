//! Filesystem canonicalization in `darkmatter-cli` goes through
//! `biscuit_file::canonicalize_simplified`. This crate owns the shared engine
//! (`tests/common/path_lookup_guard.rs`), so the engine's self-tests, which
//! scan small fixture packages, live here too.

#[path = "../common/path_lookup_guard.rs"]
mod engine;

use std::path::PathBuf;

use engine::{Exception, Kind, Rule, Site};

#[test]
fn production_source_canonicalizes_only_through_the_shared_helper() {
    engine::assert_guarded(
        "darkmatter/cli",
        &biscuit_test_harness::manifest_dir!(),
        &[Rule::Canonicalize],
        &[],
    );
}

/// A throwaway package at `<temp>/fixture/lib`, scanned as `fixture/lib`.
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

const PACKAGE: &str = "fixture/lib";
const MANIFEST: &str = "[package]\nname = \"fixture\"\n\n[lib]\npath = \"src/lib.rs\"\n";

impl Fixture {
    fn new(manifest: &str, files: &[(&str, &str)]) -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().join("fixture").join("lib");
        std::fs::create_dir_all(&root).expect("package dir");
        std::fs::write(root.join("Cargo.toml"), manifest).expect("manifest");
        let fixture = Self { _temp: temp, root };
        for (path, text) in files {
            fixture.write(path, text);
        }
        fixture
    }

    /// A package whose `src/lib.rs` is `lib`.
    fn lib(lib: &str) -> Self {
        Self::new(MANIFEST, &[("src/lib.rs", lib)])
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        std::fs::write(path, text).expect("write");
    }

    fn problems(&self, rules: &[Rule], exceptions: &[Exception]) -> Vec<String> {
        engine::problems(PACKAGE, &self.root, rules, exceptions)
    }

    /// `(operation, item, line, resolved)` per site, in scan order.
    fn sites(&self, rules: &[Rule]) -> Vec<(String, String, usize, bool)> {
        let (sources, problems) = engine::package_sources(PACKAGE, &self.root);
        assert!(problems.is_empty(), "{problems:?}");
        engine::scan(&sources, rules)
            .into_iter()
            .map(|Site { operation, item, line, resolved, .. }| (operation, item, line, resolved))
            .collect()
    }
}

fn site(operation: &str, item: &str, line: usize, resolved: bool) -> (String, String, usize, bool) {
    (operation.to_string(), item.to_string(), line, resolved)
}

const fn invariant(item: &'static str, operation: &'static str, count: usize) -> Exception {
    Exception {
        rule: Rule::Canonicalize,
        kind: Kind::Invariant,
        path: "fixture/lib/src/lib.rs",
        item,
        operation,
        count,
        reason: "both operands are canonicalized identically and only the bool leaves",
    }
}

const SAME_FILE: &str = "\
use std::path::Path;

pub fn same_file(left: &Path, right: &Path) -> bool {
    std::fs::canonicalize(left).ok() == std::fs::canonicalize(right).ok()
}
";

#[test]
fn a_new_call_fails_with_its_file_and_line() {
    let problems = Fixture::lib(SAME_FILE).problems(&[Rule::Canonicalize], &[]);

    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert!(
        problems[0].starts_with("new canonicalize call `std::fs::canonicalize` in same_file at fixture/lib/src/lib.rs:4, fixture/lib/src/lib.rs:4;"),
        "{}",
        problems[0]
    );
    assert!(problems[0].contains("biscuit_file::canonicalize_simplified"), "{}", problems[0]);
    assert!(problems[0].contains("count: 2"), "{}", problems[0]);
}

#[test]
fn an_excepted_call_passes_and_survives_a_moved_line() {
    let exceptions = [invariant("same_file", "std::fs::canonicalize", 2)];
    assert_eq!(Fixture::lib(SAME_FILE).problems(&[Rule::Canonicalize], &exceptions), Vec::<String>::new());

    let moved = format!("// a new header comment\n\n\n{SAME_FILE}");
    assert_eq!(Fixture::lib(&moved).problems(&[Rule::Canonicalize], &exceptions), Vec::<String>::new());
}

#[test]
fn a_deleted_exception_or_a_deleted_call_fails() {
    // The exception is deleted while the calls remain.
    let without_exception = Fixture::lib(SAME_FILE).problems(&[Rule::Canonicalize], &[]);
    assert!(without_exception[0].starts_with("new canonicalize call"), "{without_exception:?}");

    // The calls are deleted while the exception remains: it is stale.
    let converted = "pub fn same_file(left: &std::path::Path) -> bool { left.exists() }\n";
    let stale = Fixture::lib(converted).problems(&[Rule::Canonicalize], &[invariant("same_file", "std::fs::canonicalize", 2)]);
    assert_eq!(
        stale,
        vec!["stale exception: canonicalize `std::fs::canonicalize` in same_file (fixture/lib/src/lib.rs) no longer occurs"]
    );
}

#[test]
fn a_changed_count_fails() {
    let problems =
        Fixture::lib(SAME_FILE).problems(&[Rule::Canonicalize], &[invariant("same_file", "std::fs::canonicalize", 1)]);

    assert_eq!(
        problems,
        vec![
            "changed count for canonicalize `std::fs::canonicalize` in same_file: excepted 1, found 2 at \
             fixture/lib/src/lib.rs:4, fixture/lib/src/lib.rs:4"
        ]
    );
}

#[test]
fn an_exception_without_a_reason_or_a_duplicate_fails() {
    let mut empty = invariant("same_file", "std::fs::canonicalize", 2);
    empty.reason = " ";
    let problems = Fixture::lib(SAME_FILE).problems(&[Rule::Canonicalize], &[empty, empty]);

    assert!(problems.iter().any(|problem| problem.contains("needs a reason")), "{problems:#?}");
    assert!(problems.iter().any(|problem| problem.starts_with("duplicate exception")), "{problems:#?}");
}

#[test]
fn every_spelling_of_a_filesystem_canonicalize_is_found() {
    let source = "\
use std::fs as filesystem;
use std::fs::canonicalize as canon;
use dunce::canonicalize;
use std::path::PathBuf;

pub fn spellings(path: &Path, paths: &[PathBuf]) {
    let _ = std::fs::canonicalize(path);
    let _ = filesystem::canonicalize(path);
    let _ = canon(path);
    let _ = canonicalize(path);
    let _ = path.canonicalize();
    let _ = Path::canonicalize(path);
    let _ = tokio::fs::canonicalize(path);
    let _ = paths.iter().map(std::fs::canonicalize);
    let _ = paths.iter().map(canon);
}
";
    let sites = Fixture::lib(source).sites(&[Rule::Canonicalize]);

    assert_eq!(
        sites,
        vec![
            site("std::fs::canonicalize", "spellings", 7, true),
            site("filesystem::canonicalize", "spellings", 8, true),
            site("canonicalize (imported dunce::canonicalize)", "spellings", 10, true),
            site(".canonicalize()", "spellings", 11, true),
            site("Path::canonicalize", "spellings", 12, true),
            site("tokio::fs::canonicalize", "spellings", 13, true),
            site("std::fs::canonicalize", "spellings", 14, true),
            site("canon (alias of fs::canonicalize)", "spellings", 9, true),
            site("canon (alias of fs::canonicalize)", "spellings", 15, true),
        ]
    );
}

#[test]
fn test_code_comments_and_strings_are_not_calls() {
    let lib = r#"
//! Never call `std::fs::canonicalize` here.

/// `path.canonicalize()` would keep the verbatim prefix.
pub fn describe() -> &'static str {
    /* std::fs::canonicalize(path) */
    let _raw = r"dunce::canonicalize(path)";
    "std::fs::canonicalize(path)"
}

#[cfg(test)]
mod tests {
    #[test]
    fn inline() {
        let _ = std::fs::canonicalize(".");
    }
}

#[cfg(test)]
mod separate;

#[cfg(all(test, unix))]
fn helper() -> std::io::Result<std::path::PathBuf> {
    std::path::Path::new(".").canonicalize()
}
"#;
    let fixture = Fixture::lib(lib);
    fixture.write("src/separate.rs", "fn f() { let _ = std::fs::canonicalize(\".\"); }\n");

    assert_eq!(fixture.sites(&[Rule::Canonicalize]), Vec::new());
}

#[test]
fn a_platform_branch_is_scanned() {
    let source = "\
#[cfg(windows)]
pub fn native(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    dunce::canonicalize(path)
}
";
    assert_eq!(Fixture::lib(source).sites(&[Rule::Canonicalize]), vec![site("dunce::canonicalize", "native", 3, true)]);
}

#[test]
fn a_style_name_normalizer_is_not_filesystem_canonicalization() {
    // Darkmatter's `style::descriptor::canonicalize` normalizes style names.
    let descriptor = "\
pub fn canonicalize(raw_path: &str) -> Option<&'static str> {
    (!raw_path.is_empty()).then_some(\"leaf\")
}

pub fn lookup(raw: &str) -> Option<&'static str> {
    canonicalize(raw)
}
";
    let fixture = Fixture::new(MANIFEST, &[("src/lib.rs", "pub mod descriptor;\n"), ("src/descriptor.rs", descriptor)]);
    assert_eq!(fixture.sites(&[Rule::Canonicalize]), Vec::new());

    // Called from another file through a qualifier the scanner cannot place,
    // it is an unresolved candidate: reported, and accepted only as Reviewed.
    fixture.write("src/lib.rs", "pub mod descriptor;\n\npub fn style() {\n    let _ = descriptor::canonicalize(\"page.margin\");\n}\n");
    let problems = fixture.problems(&[Rule::Canonicalize], &[]);
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert!(
        problems[0].starts_with(
            "unresolved canonicalize candidate `descriptor::canonicalize` in style at fixture/lib/src/lib.rs:4: the \
             scanner cannot tell what it calls"
        ),
        "{}",
        problems[0]
    );

    let reviewed = Exception {
        rule: Rule::Canonicalize,
        kind: Kind::Reviewed,
        path: "fixture/lib/src/lib.rs",
        item: "style",
        operation: "descriptor::canonicalize",
        count: 1,
        reason: "style-name normalizer, not the filesystem",
    };
    assert_eq!(fixture.problems(&[Rule::Canonicalize], &[reviewed]), Vec::<String>::new());

    let mistaken = Exception { kind: Kind::Invariant, ..reviewed };
    let problems = fixture.problems(&[Rule::Canonicalize], &[mistaken]);
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert!(problems[0].contains("is Invariant, but the site is unresolved (use Reviewed)"), "{}", problems[0]);
}

#[test]
fn a_method_with_arguments_is_not_path_canonicalize() {
    // The permission backends' `canonicalize(ctx, &native)` is policy, not I/O.
    let source = "\
pub trait Backend {
    fn canonicalize(&self, context: &str, native: &str) -> String;
}

pub fn policy(backend: &dyn Backend) -> String {
    backend.canonicalize(\"ctx\", \"native\")
}
";
    assert_eq!(Fixture::lib(source).sites(&[Rule::Canonicalize]), Vec::new());
}

#[test]
fn the_enclosing_item_names_the_impl_type() {
    let source = "\
pub struct Store;

impl Store {
    pub fn key(&self, path: &std::path::Path) -> Option<std::path::PathBuf> {
        path.canonicalize().ok()
    }
}

impl<T> From<T> for Wrapper<T> {
    fn from(_: T) -> Self {
        let _ = std::fs::canonicalize(\".\");
        unimplemented!()
    }
}
";
    assert_eq!(
        Fixture::lib(source).sites(&[Rule::Canonicalize]),
        vec![site(".canonicalize()", "Store::key", 5, true), site("std::fs::canonicalize", "Wrapper::from", 11, true)]
    );
}

#[test]
fn every_spelling_of_a_direct_home_lookup_is_found() {
    let source = "\
use dirs::home_dir as user_home;
use std::env;
use std::path::PathBuf;

pub fn homes(ctx: &Context, home_dir: Option<PathBuf>) -> Vec<Option<PathBuf>> {
    vec![
        dirs::home_dir(),
        std::env::home_dir(),
        env::home_dir(),
        home::home_dir(),
        user_home(),
        None.or_else(dirs::home_dir),
        biscuit_file::home_dir(),
        ctx.home_dir(),
        home_dir,
    ]
}
";
    assert_eq!(
        Fixture::lib(source).sites(&[Rule::HomeLookup]),
        vec![
            site("dirs::home_dir", "homes", 7, true),
            site("std::env::home_dir", "homes", 8, true),
            site("env::home_dir", "homes", 9, true),
            site("home::home_dir", "homes", 10, true),
            site("dirs::home_dir", "homes", 12, true),
            site("user_home (alias of dirs::home_dir)", "homes", 11, true),
        ]
    );
}

#[test]
fn a_home_lookup_imported_from_the_helper_is_allowed() {
    let source = "\
use biscuit_file::home_dir;

pub fn home() -> Option<std::path::PathBuf> {
    home_dir()
}
";
    assert_eq!(Fixture::lib(source).problems(&[Rule::HomeLookup], &[]), Vec::<String>::new());

    let direct = source.replace("biscuit_file::home_dir", "dirs::home_dir");
    let problems = Fixture::lib(&direct).problems(&[Rule::HomeLookup], &[]);
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert!(
        problems[0].starts_with(
            "new home-lookup call `home_dir (imported dirs::home_dir)` in home at fixture/lib/src/lib.rs:4; use \
             biscuit_file::home_dir"
        ),
        "{}",
        problems[0]
    );
}

#[test]
fn a_rule_the_package_does_not_take_is_not_reported() {
    let source = "pub fn home() -> Option<std::path::PathBuf> { dirs::home_dir() }\n";
    assert_eq!(Fixture::lib(source).problems(&[Rule::Canonicalize], &[]), Vec::<String>::new());
}

#[test]
fn a_manifest_declared_root_and_the_build_script_are_scanned() {
    let manifest = format!("{MANIFEST}\n[[bin]]\nname = \"tool\"\npath = \"tools/tool/main.rs\"\n\n[[bin]]\nname = \"fake\"\npath = \"tests/bin/fake.rs\"\n");
    let fixture = Fixture::new(
        &manifest,
        &[
            ("src/lib.rs", "pub fn nothing() {}\n"),
            ("tools/tool/main.rs", "fn main() {\n    let _ = std::fs::canonicalize(\".\");\n}\n"),
            ("tests/bin/fake.rs", "fn main() {\n    let _ = std::fs::canonicalize(\".\");\n}\n"),
            ("build.rs", "fn main() {\n    let _ = std::path::Path::new(\".\").canonicalize();\n}\n"),
        ],
    );
    let (sources, problems) = engine::package_sources(PACKAGE, &fixture.root);
    assert!(problems.is_empty(), "{problems:?}");
    let sites = engine::scan(&sources, &[Rule::Canonicalize]);
    let locations = sites.iter().map(|site| format!("{}:{}", site.path, site.line)).collect::<Vec<_>>();

    assert_eq!(locations, vec!["fixture/lib/build.rs:2", "fixture/lib/tools/tool/main.rs:2"]);
}

#[test]
fn a_missing_root_or_an_empty_scan_fails() {
    let missing_src = Fixture::new(MANIFEST, &[]);
    let problems = missing_src.problems(&[Rule::Canonicalize], &[]);
    assert!(problems.contains(&"declared source root fixture/lib/src/lib.rs is missing".to_string()), "{problems:#?}");
    assert!(problems.contains(&"required source root fixture/lib/src is missing".to_string()), "{problems:#?}");
    assert!(problems.iter().any(|problem| problem.contains("an empty scan cannot pass")), "{problems:#?}");

    let no_manifest = tempfile::tempdir().expect("temp dir");
    let package = no_manifest.path().join("fixture").join("lib");
    std::fs::create_dir_all(package.join("src")).expect("dirs");
    let problems = engine::problems(PACKAGE, &package, &[Rule::Canonicalize], &[]);
    assert!(problems[0].starts_with("required manifest"), "{problems:#?}");

    let fixture = Fixture::lib(SAME_FILE);
    let problems = engine::problems("fixture/cli", &fixture.root, &[Rule::Canonicalize], &[]);
    assert!(problems[0].ends_with("is not the package directory `fixture/cli`"), "{problems:#?}");
}
