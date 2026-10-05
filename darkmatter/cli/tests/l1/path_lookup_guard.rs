//! Filesystem canonicalization in `darkmatter-cli` goes through
//! `biscuit_file::canonicalize_simplified`. This crate owns the shared engine
//! (`tests/common/path_lookup_guard.rs`), so the engine's self-tests, which
//! scan small fixture packages, live here too.

#[path = "../common/path_lookup_guard.rs"]
mod engine;

use std::path::PathBuf;

use engine::{Exception, FixtureBin, Kind, Rule, Site};

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
    let manifest = format!(
        "{MANIFEST}\n[[bin]]\nname = \"tool\"\npath = \"tools/tool/main.rs\"\n\n[[bin]]\nname = \"fake\"\npath = \"tests/bin/fake.rs\"\nrequired-features = [\"test-fixtures\"]\n"
    );
    let fixture = Fixture::new(
        &manifest,
        &[
            ("src/lib.rs", "pub fn nothing() {}\n"),
            ("tools/tool/main.rs", "fn main() {\n    let _ = std::fs::canonicalize(\".\");\n}\n"),
            ("tests/bin/fake.rs", "fn main() {\n    let _ = std::fs::canonicalize(\".\");\n}\n"),
            ("build.rs", "fn main() {\n    let _ = std::path::Path::new(\".\").canonicalize();\n}\n"),
        ],
    );
    let fixtures = [FixtureBin { name: "fake", reason: "driven by an integration test" }];
    let (sources, problems) = engine::package_sources_with_fixtures(PACKAGE, &fixture.root, &fixtures);
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

/// One row per guarded provider: the rule, the module or type path that
/// provides the function, and the arguments a call takes.
const PROVIDERS: [(Rule, &str, &str); 8] = [
    (Rule::Canonicalize, "std::fs", "(path)"),
    (Rule::Canonicalize, "tokio::fs", "(path)"),
    (Rule::Canonicalize, "dunce", "(path)"),
    (Rule::Canonicalize, "std::path::Path", "(path)"),
    (Rule::Canonicalize, "std::path::PathBuf", "(path)"),
    (Rule::HomeLookup, "std::env", "()"),
    (Rule::HomeLookup, "dirs", "()"),
    (Rule::HomeLookup, "home", "()"),
];

fn function_name(rule: Rule) -> &'static str {
    match rule {
        Rule::Canonicalize => "canonicalize",
        Rule::HomeLookup => "home_dir",
    }
}

#[test]
fn every_provider_is_found_through_every_import_form() {
    // The scan is lexical, so type providers (`Path`, `PathBuf`) go through
    // the same import forms as modules even where rustc would reject them.
    let forms: [(&str, &str, &str); 9] = [
        ("qualified call", "", "{module}::{name}{args}"),
        ("qualified reference", "", "{module}::{name}"),
        ("direct alias call", "use {module}::{name} as invoke;", "invoke{args}"),
        ("chained alias call", "use {module} as provider;\nuse provider::{name} as invoke;", "invoke{args}"),
        ("chained alias reference", "use {module} as provider;\nuse provider::{name} as invoke;", "invoke"),
        ("module alias call", "use {module} as provider;", "provider::{name}{args}"),
        ("alias of an alias", "use {module}::{name} as first;\nuse first as invoke;", "invoke{args}"),
        ("grouped alias", "use {{{module}::{{{name} as invoke}}}};", "invoke{args}"),
        ("glob import", "use {module}::*;", "{name}{args}"),
    ];
    let mut failures = Vec::new();
    for (rule, module, args) in PROVIDERS {
        let name = function_name(rule);
        for (form, imports, expression) in forms {
            let expand = |template: &str| {
                template.replace("{{", "\u{1}").replace("}}", "\u{2}").replace("{module}", module).replace("{name}", name)
                    .replace("{args}", args).replace('\u{1}', "{").replace('\u{2}', "}")
            };
            let source = format!(
                "{}\n\npub fn probe(path: &std::path::Path) {{\n    let _ = {};\n}}\n",
                expand(imports),
                expand(expression)
            );
            let fixture = Fixture::lib(&source);
            let sites = fixture.sites(&[rule]);
            let reported = fixture.problems(&[rule], &[]);
            let found = sites.len() == 1 && sites[0].3 && sites[0].1 == "probe";
            let rejected = reported.len() == 1 && reported[0].starts_with(&format!("new {rule} call"));
            if !(found && rejected) {
                failures.push(format!("{module} {form}: sites {sites:?}, problems {reported:?}\n{source}"));
            }
        }
    }
    assert!(failures.is_empty(), "{} of {} cells failed:\n{}", failures.len(), PROVIDERS.len() * forms.len(), failures.join("\n"));
}

#[test]
fn a_name_imported_from_an_unknown_module_is_an_unresolved_candidate() {
    let source = "\
use other::canonicalize as invoke;
use other::home_dir as home;
use elsewhere as provider;
use provider::home_dir;
use crate::util::canonicalize;

pub fn probe(path: &std::path::Path) {
    let _ = invoke(path);
    let _ = invoke;
    let _ = home();
    let _ = home_dir();
    let _ = canonicalize(path);
}
";
    let fixture = Fixture::lib(source);
    assert_eq!(
        fixture.sites(&[Rule::Canonicalize, Rule::HomeLookup]),
        vec![
            site("canonicalize (imported util::canonicalize)", "probe", 12, false),
            site("invoke (alias of other::canonicalize)", "probe", 8, false),
            site("invoke (alias of other::canonicalize)", "probe", 9, false),
            site("home_dir (imported elsewhere::home_dir)", "probe", 11, false),
            site("home (alias of other::home_dir)", "probe", 10, false),
        ]
    );

    let problems = fixture.problems(&[Rule::Canonicalize, Rule::HomeLookup], &[]);
    assert_eq!(problems.len(), 4, "{problems:#?}");
    assert!(problems.iter().all(|problem| problem.starts_with("unresolved ")), "{problems:#?}");

    let reviewed = |rule, operation, count| Exception {
        rule,
        kind: Kind::Reviewed,
        path: "fixture/lib/src/lib.rs",
        item: "probe",
        operation,
        count,
        reason: "reviewed: names what it calls",
    };
    let exceptions = [
        reviewed(Rule::Canonicalize, "canonicalize (imported util::canonicalize)", 1),
        reviewed(Rule::Canonicalize, "invoke (alias of other::canonicalize)", 2),
        reviewed(Rule::HomeLookup, "home_dir (imported elsewhere::home_dir)", 1),
        reviewed(Rule::HomeLookup, "home (alias of other::home_dir)", 1),
    ];
    assert_eq!(fixture.problems(&[Rule::Canonicalize, Rule::HomeLookup], &exceptions), Vec::<String>::new());
}

#[test]
fn an_import_applies_only_to_its_own_module_or_function() {
    let source = "\
use dirs::home_dir as user_home;

mod approved {
    use biscuit_file::home_dir;

    pub fn home() -> Option<std::path::PathBuf> {
        home_dir()
    }
}

mod unrelated {
    use unknown::home_dir;

    pub fn home() -> Option<std::path::PathBuf> {
        home_dir()
    }
}

mod unimported {
    pub fn home(user_home: Option<std::path::PathBuf>) -> Option<std::path::PathBuf> {
        user_home.or_else(|| home_dir())
    }
}

pub fn local() -> Option<std::path::PathBuf> {
    use std::env::home_dir;
    home_dir()
}

pub fn outside() -> Option<std::path::PathBuf> {
    user_home()
}
";
    assert_eq!(
        Fixture::lib(source).sites(&[Rule::HomeLookup]),
        vec![
            site("home_dir (imported unknown::home_dir)", "unrelated::home", 15, false),
            site("home_dir", "unimported::home", 21, false),
            site("home_dir (imported env::home_dir)", "local", 27, true),
            site("user_home (alias of dirs::home_dir)", "outside", 31, true),
        ]
    );
}

#[test]
fn glob_and_in_file_module_paths_are_followed() {
    let source = "\
use std::fs as filesystem;

mod inner {
    use super::*;

    pub fn raw(path: &std::path::Path) {
        let _ = filesystem::canonicalize(path);
    }
}

mod style {
    pub fn canonicalize(raw: &str) -> &str {
        raw
    }

    pub use std::fs::canonicalize as real;
}

pub fn names() {
    let _ = style::canonicalize(\"page.margin\");
    let _ = self::style::canonicalize(\"page.margin\");
}

pub fn reexported(path: &std::path::Path) {
    let _ = style::real(path);
}
";
    assert_eq!(
        Fixture::lib(source).sites(&[Rule::Canonicalize]),
        vec![
            site("filesystem::canonicalize", "inner::raw", 7, true),
            site("style::real", "reexported", 25, true),
        ]
    );
}

/// What the guard reports for one manifest cell.
#[derive(Clone, Copy, Debug)]
enum Expect {
    /// Exactly one problem, the cell's unapproved call, which an exception
    /// keyed on the file's normalized path then accepts.
    RejectsCall,
    NoProblem,
    /// Problems naming this manifest field or condition, and no call.
    Manifest(&'static str),
}

#[test]
fn manifest_target_fields_are_read_as_cargo_reads_them() {
    // `(cell, before [package], inside [package], after [dependencies], file
    // holding the call, expect)`: one edit to a Cargo-generated library
    // manifest whose `src/lib.rs`, `build.rs`, and `production/main.rs` are
    // safe except for the file named. `cargo metadata --no-deps --offline`
    // selects `production/main.rs` for every valid alternate spelling below.
    use Expect::{Manifest, NoProblem, RejectsCall};
    const MAIN: &str = "production/main.rs";
    const LIB: &str = "src/lib.rs";
    const BUILD: &str = "build.rs";
    let invalid = "Cargo.toml is not valid TOML";
    let lib = "`[lib].path`";
    let bin = "`[[bin]].path` of `tool`";
    let build = "`[package].build`";
    let cells: &[(&str, &str, &str, &str, &str, Expect)] = &[
        ("lib: double-quoted", "", "", "[lib]\npath = \"production/main.rs\"\n", MAIN, RejectsCall),
        ("lib: single-quoted", "", "", "[lib]\npath = 'production/main.rs'\n", MAIN, RejectsCall),
        ("lib: spaced header", "", "", "[ lib ]\npath = \"production/main.rs\"\n", MAIN, RejectsCall),
        ("lib: inline table", "lib = { path = \"production/main.rs\" }\n", "", "", MAIN, RejectsCall),
        ("lib: dotted key", "lib.path = 'production/main.rs'\n", "", "", MAIN, RejectsCall),
        ("lib: dot-prefixed", "", "", "[lib]\npath = \"./production/main.rs\"\n", MAIN, RejectsCall),
        ("lib: parent segment", "", "", "[lib]\npath = \"production/../production/main.rs\"\n", MAIN, RejectsCall),
        ("lib: absent path scans src/lib.rs", "", "", "[lib]\nname = \"fixture\"\n", LIB, RejectsCall),
        ("lib: absolute", "", "", "[lib]\npath = \"/production/main.rs\"\n", MAIN, Manifest(lib)),
        ("lib: null", "", "", "[lib]\npath = null\n", MAIN, Manifest(invalid)),
        ("lib: integer", "", "", "[lib]\npath = 123\n", MAIN, Manifest(lib)),
        ("lib: one wrong element", "", "", "[lib]\npath = [\"production/main.rs\", 123]\n", MAIN, Manifest(lib)),
        ("lib: every element wrong", "", "", "[lib]\npath = [123]\n", MAIN, Manifest(lib)),
        ("lib: empty array", "", "", "[lib]\npath = []\n", MAIN, Manifest(lib)),
        ("lib: empty string", "", "", "[lib]\npath = \"\"\n", MAIN, Manifest(lib)),
        (
            "lib: duplicate key",
            "",
            "",
            "[lib]\npath = \"production/main.rs\"\npath = \"src/lib.rs\"\n",
            MAIN,
            Manifest(invalid),
        ),
        ("lib: trailing garbage", "", "", "[lib]\npath = \"production/main.rs\" garbage\n", MAIN, Manifest(invalid)),
        ("bin: double-quoted", "", "", "[[bin]]\nname = \"tool\"\npath = \"production/main.rs\"\n", MAIN, RejectsCall),
        ("bin: single-quoted", "", "", "[[bin]]\nname = \"tool\"\npath = 'production/main.rs'\n", MAIN, RejectsCall),
        ("bin: spaced header", "", "", "[[ bin ]]\nname = \"tool\"\npath = \"production/main.rs\"\n", MAIN, RejectsCall),
        ("bin: inline array", "bin = [{ name = \"tool\", path = 'production/main.rs' }]\n", "", "", MAIN, RejectsCall),
        ("bin: dot-prefixed", "", "", "[[bin]]\nname = \"tool\"\npath = \"./production/main.rs\"\n", MAIN, RejectsCall),
        (
            "bin: same file as lib, other spelling",
            "",
            "",
            "[lib]\npath = \"production/main.rs\"\n\n[[bin]]\nname = \"tool\"\npath = \"./production/../production/main.rs\"\n",
            MAIN,
            RejectsCall,
        ),
        (
            "bin: absent path",
            "",
            "",
            "[[bin]]\nname = \"tool\"\n",
            MAIN,
            Manifest("`tool` has no `path` and none of its inferred sources"),
        ),
        ("bin: neither name nor path", "", "", "[[bin]]\n", MAIN, Manifest("has neither `name` nor `path`")),
        ("bin: null", "", "", "[[bin]]\nname = \"tool\"\npath = null\n", MAIN, Manifest(invalid)),
        ("bin: integer", "", "", "[[bin]]\nname = \"tool\"\npath = 123\n", MAIN, Manifest(bin)),
        (
            "bin: one wrong element",
            "",
            "",
            "[[bin]]\nname = \"tool\"\npath = [\"production/main.rs\", 123]\n",
            MAIN,
            Manifest(bin),
        ),
        ("bin: every element wrong", "", "", "[[bin]]\nname = \"tool\"\npath = [123]\n", MAIN, Manifest(bin)),
        ("bin: empty array", "", "", "[[bin]]\nname = \"tool\"\npath = []\n", MAIN, Manifest(bin)),
        ("bin: empty string", "", "", "[[bin]]\nname = \"tool\"\npath = \"\"\n", MAIN, Manifest(bin)),
        (
            "bin: duplicate key",
            "",
            "",
            "[[bin]]\nname = \"tool\"\npath = \"production/main.rs\"\npath = \"src/lib.rs\"\n",
            MAIN,
            Manifest(invalid),
        ),
        (
            "bin: trailing garbage",
            "",
            "",
            "[[bin]]\nname = \"tool\"\npath = \"production/main.rs\" garbage\n",
            MAIN,
            Manifest(invalid),
        ),
        ("build: double-quoted", "", "build = \"production/main.rs\"\n", "", MAIN, RejectsCall),
        ("build: single-quoted", "", "build = 'production/main.rs'\n", "", MAIN, RejectsCall),
        ("build: dot-prefixed", "", "build = \"./build.rs\"\n", "", BUILD, RejectsCall),
        ("build: absent scans build.rs", "", "", "", BUILD, RejectsCall),
        ("build: true scans build.rs", "", "build = true\n", "", BUILD, RejectsCall),
        ("build: false skips build.rs", "", "build = false\n", "", BUILD, NoProblem),
        ("build: null", "", "build = null\n", "", MAIN, Manifest(invalid)),
        ("build: integer", "", "build = 123\n", "", MAIN, Manifest(build)),
        ("build: one wrong element", "", "build = [\"production/main.rs\", 123]\n", "", MAIN, Manifest(build)),
        ("build: every element wrong", "", "build = [123]\n", "", MAIN, Manifest(build)),
        ("build: empty array", "", "build = []\n", "", MAIN, Manifest(build)),
        ("build: empty string", "", "build = \"\"\n", "", MAIN, Manifest(build)),
        ("build: duplicate key", "", "build = \"production/main.rs\"\nbuild = \"build.rs\"\n", "", MAIN, Manifest(invalid)),
        ("build: trailing garbage", "", "build = \"production/main.rs\" garbage\n", "", MAIN, Manifest(invalid)),
    ];

    const CALL: &str = "fn main() {\n    let _ = std::fs::canonicalize(\".\");\n}\n";
    let mut failures = Vec::new();
    for &(cell, before, inside, after, call_in, expect) in cells {
        let manifest = format!(
            "{before}[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{inside}\n[dependencies]\n{after}"
        );
        let files = [LIB, BUILD, MAIN].map(|path| (path, if path == call_in { CALL } else { "pub fn nothing() {}\n" }));
        let fixture = Fixture::new(&manifest, &files);
        let problems = fixture.problems(&[Rule::Canonicalize], &[]);
        let holds = match expect {
            RejectsCall => {
                // `Exception` holds static strings; one small leak per cell.
                let path: &'static str = format!("fixture/lib/{call_in}").leak();
                let excepted = Exception {
                    rule: Rule::Canonicalize,
                    kind: Kind::Invariant,
                    path,
                    item: "main",
                    operation: "std::fs::canonicalize",
                    count: 1,
                    reason: "keyed on the normalized path",
                };
                problems.len() == 1
                    && problems[0].starts_with(&format!("new canonicalize call `std::fs::canonicalize` in main at {path}:2;"))
                    && problems[0].contains("count: 1")
                    && fixture.problems(&[Rule::Canonicalize], &[excepted]).is_empty()
            }
            NoProblem => problems.is_empty(),
            Manifest(field) => {
                problems.iter().any(|problem| problem.starts_with("fixture/lib/Cargo.toml") && problem.contains(field))
                    && !problems.iter().any(|problem| problem.contains("canonicalize call"))
            }
        };
        if !holds {
            failures.push(format!("{cell}: expected {expect:?}, got {problems:#?}\n{manifest}"));
        }
    }
    assert!(failures.is_empty(), "{} of {} cells failed:\n{}", failures.len(), cells.len(), failures.join("\n"));
}

#[test]
fn source_discovery_follows_the_compiled_module_tree() {
    // `(cell, manifest after [package], files, listed fixtures, expected
    // problem fragments)`. Every file not named holds no call; a call is
    // reported as `in <item> at <path>:<line>;`.
    const MAIN: &str = "fn main() {\n    let _ = std::fs::canonicalize(\".\");\n}\n";
    const MODULE: &str = "pub fn f() {\n    let _ = std::fs::canonicalize(\".\");\n}\n";
    const SAFE: &str = "pub fn nothing() {}\n";
    const APP: &str = "[[bin]]\nname = \"app\"\npath = \"tools/app/main.rs\"\n";
    const TEST_BIN: &str = "[[bin]]\nname = \"app\"\npath = \"tests/bin/app.rs\"\n";
    const GATED_TEST_BIN: &str = "[[bin]]\nname = \"app\"\npath = \"tests/bin/app.rs\"\nrequired-features = [\"test-fixtures\"]\n";
    const LISTED: &[FixtureBin] = &[FixtureBin { name: "app", reason: "fake provider driven by an integration test" }];
    type Cell<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)], &'a [FixtureBin], &'a [&'a str]);
    let cells: &[Cell<'_>] = &[
        (
            "bin under tests/, not listed",
            TEST_BIN,
            &[("src/lib.rs", SAFE), ("tests/bin/app.rs", MAIN)],
            &[],
            &["in main at fixture/lib/tests/bin/app.rs:2;"],
        ),
        (
            "bin under tests/ with required-features, not listed",
            GATED_TEST_BIN,
            &[("src/lib.rs", SAFE), ("tests/bin/app.rs", MAIN)],
            &[],
            &["in main at fixture/lib/tests/bin/app.rs:2;"],
        ),
        ("listed fixture bin", GATED_TEST_BIN, &[("src/lib.rs", SAFE), ("tests/bin/app.rs", MAIN)], LISTED, &[]),
        (
            "listed fixture bin without required-features",
            TEST_BIN,
            &[("src/lib.rs", SAFE), ("tests/bin/app.rs", MAIN)],
            LISTED,
            &["fixture bin `app` of fixture/lib declares no `required-features`"],
        ),
        (
            "listed fixture bin outside tests/",
            "[[bin]]\nname = \"app\"\npath = \"tools/app/main.rs\"\nrequired-features = [\"test-fixtures\"]\n",
            &[("src/lib.rs", SAFE), ("tools/app/main.rs", MAIN)],
            LISTED,
            &["fixture bin `app` of fixture/lib is at tools/app/main.rs, not under tests/"],
        ),
        ("stale fixture listing", "", &[("src/lib.rs", SAFE)], LISTED, &["stale fixture bin `app` of fixture/lib"]),
        (
            "inferred library root with an external #[path] module",
            "",
            &[("src/lib.rs", "#[path = \"../production/main.rs\"]\nmod external;\n"), ("production/main.rs", MODULE)],
            &[],
            &["in f at fixture/lib/production/main.rs:2;"],
        ),
        (
            "declared library root with an external #[path] module",
            "[lib]\npath = \"src/lib.rs\"\n",
            &[("src/lib.rs", "#[path = \"../production/main.rs\"]\npub mod external;\n"), ("production/main.rs", MODULE)],
            &[],
            &["in f at fixture/lib/production/main.rs:2;"],
        ),
        (
            "nested module in a non-mod-rs file",
            APP,
            &[("src/lib.rs", SAFE), ("tools/app/main.rs", "mod a;\nfn main() {}\n"), ("tools/app/a.rs", "mod b;\n"), ("tools/app/a/b.rs", MODULE)],
            &[],
            &["in f at fixture/lib/tools/app/a/b.rs:2;"],
        ),
        (
            "nested module in a mod.rs file",
            APP,
            &[("src/lib.rs", SAFE), ("tools/app/main.rs", "mod a;\nfn main() {}\n"), ("tools/app/a/mod.rs", "pub(crate) mod b;\n"), ("tools/app/a/b.rs", MODULE)],
            &[],
            &["in f at fixture/lib/tools/app/a/b.rs:2;"],
        ),
        (
            "#[path] inside an inline module of a non-mod-rs file",
            APP,
            &[
                ("src/lib.rs", SAFE),
                ("tools/app/main.rs", "mod a;\nfn main() {}\n"),
                ("tools/app/a.rs", "mod inner {\n    #[path = \"deep.rs\"]\n    mod deep;\n}\n"),
                ("tools/app/a/inner/deep.rs", MODULE),
            ],
            &[],
            &["in f at fixture/lib/tools/app/a/inner/deep.rs:2;"],
        ),
        (
            "every cfg_attr path branch",
            APP,
            &[
                ("src/lib.rs", SAFE),
                (
                    "tools/app/main.rs",
                    "#[cfg_attr(windows, path = \"win.rs\")]\n#[cfg_attr(not(windows), path = \"unix.rs\")]\nmod imp;\nfn main() {}\n",
                ),
                ("tools/app/win.rs", MODULE),
                ("tools/app/unix.rs", MODULE),
            ],
            &[],
            &["in f at fixture/lib/tools/app/unix.rs:2;", "in f at fixture/lib/tools/app/win.rs:2;"],
        ),
        (
            "cfg_attr path and the default file",
            APP,
            &[
                ("src/lib.rs", SAFE),
                ("tools/app/main.rs", "#[cfg_attr(windows, path = \"win.rs\")]\nmod imp;\nfn main() {}\n"),
                ("tools/app/win.rs", MODULE),
                ("tools/app/imp.rs", MODULE),
            ],
            &[],
            &["in f at fixture/lib/tools/app/imp.rs:2;", "in f at fixture/lib/tools/app/win.rs:2;"],
        ),
        (
            "test-only modules and items",
            APP,
            &[
                ("src/lib.rs", SAFE),
                (
                    "tools/app/main.rs",
                    "#[cfg(test)]\nmod t;\n#[cfg(test)]\nmod absent;\n#[cfg(test)]\nmod inline {\n    fn g() { let _ = std::fs::canonicalize(\".\"); }\n}\n#[cfg(all(test, unix))]\nfn h() { let _ = std::fs::canonicalize(\".\"); }\nfn main() {}\n",
                ),
                ("tools/app/t.rs", MODULE),
            ],
            &[],
            &[],
        ),
        (
            "missing declared module",
            APP,
            &[("src/lib.rs", SAFE), ("tools/app/main.rs", "mod gone;\nfn main() {}\n")],
            &[],
            &["production module `gone` declared at fixture/lib/tools/app/main.rs:1 has no source"],
        ),
        (
            "missing cfg_attr path",
            APP,
            &[("src/lib.rs", SAFE), ("tools/app/main.rs", "#[cfg_attr(windows, path = \"win.rs\")]\nmod imp;\nfn main() {}\n")],
            &[],
            &["production source fixture/lib/tools/app/win.rs is unreadable"],
        ),
        (
            "root at the package top",
            "[[bin]]\nname = \"app\"\npath = \"main.rs\"\n",
            &[
                ("src/lib.rs", SAFE),
                ("main.rs", "mod helper;\nfn main() {}\n"),
                ("helper.rs", MODULE),
                ("orphan.rs", MODULE),
                ("tests/integration.rs", MAIN),
            ],
            &[],
            &["in f at fixture/lib/helper.rs:2;"],
        ),
        (
            "files under src/ no root declares",
            "",
            &[("src/lib.rs", "mod declared;\n"), ("src/declared.rs", MODULE), ("src/orphan.rs", MODULE)],
            &[],
            &["in f at fixture/lib/src/declared.rs:2;", "in f at fixture/lib/src/orphan.rs:2;"],
        ),
    ];

    let mut failures = Vec::new();
    for &(cell, targets, files, fixtures, expected) in cells {
        let manifest = format!("[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n{targets}");
        let fixture = Fixture::new(&manifest, files);
        let problems = engine::problems_with_fixtures(PACKAGE, &fixture.root, &[Rule::Canonicalize], &[], fixtures);
        let holds = problems.len() == expected.len()
            && expected.iter().all(|fragment| problems.iter().any(|problem| problem.contains(fragment)));
        if !holds {
            failures.push(format!("{cell}: expected {expected:#?}, got {problems:#?}"));
        }
    }
    assert!(failures.is_empty(), "{} of {} cells failed:\n{}", failures.len(), cells.len(), failures.join("\n"));
}
