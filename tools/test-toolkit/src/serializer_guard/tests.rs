use super::*;

fn sites(files: &[(&str, &str)]) -> Vec<Site> {
    scan_sources(files).expect("fixture scans").sites
}

fn calls(files: &[(&str, &str)]) -> Vec<(String, String)> {
    sites(files)
        .into_iter()
        .map(|s| (s.symbol, s.call))
        .collect()
}

fn one(symbol: &str, call: &str) -> Vec<(String, String)> {
    vec![(symbol.to_string(), call.to_string())]
}

#[test]
fn qualified_calls_are_sites() {
    let found = calls(&[(
        "src/lib.rs",
        "fn a(v: &V) -> String { serde_yaml_ng::to_string(v).unwrap() }\n\
         fn b(v: &V) { ::serde_yaml::to_writer(std::io::stdout(), v).unwrap(); }\n\
         fn c(v: &V) -> String { biscuit_file::serde_yaml_ng::to_string(v).unwrap() }\n\
         fn d(buf: &mut Vec<u8>) { let _s = serde_yaml_ng::Serializer::new(buf); }\n",
    )]);
    assert_eq!(
        found,
        vec![
            ("a".into(), "serde_yaml_ng::to_string".into()),
            ("b".into(), "serde_yaml::to_writer".into()),
            ("c".into(), "serde_yaml_ng::to_string".into()),
            ("d".into(), "serde_yaml_ng::Serializer::new".into()),
        ]
    );
}

#[test]
fn a_module_alias_resolves_to_the_backend() {
    let found = calls(&[(
        "src/lib.rs",
        "use serde_yaml_ng as yaml;\nfn emit(v: &V) -> String { yaml::to_string(v).unwrap() }\n",
    )]);
    assert_eq!(found, one("emit", "serde_yaml_ng::to_string"));
}

#[test]
fn a_leaf_alias_resolves_to_the_backend() {
    let found = calls(&[(
        "src/lib.rs",
        "use serde_yaml::to_string as ser;\nfn emit(v: &V) -> String { ser(v).unwrap() }\n",
    )]);
    assert_eq!(found, one("emit", "serde_yaml::to_string"));
}

#[test]
fn grouped_imports_resolve_each_name() {
    let found = calls(&[(
        "src/lib.rs",
        "use biscuit_file::serde_yaml_ng::{self as y, to_writer, Serializer, Value};\n\
         impl Doc { fn write(&self, out: &mut Vec<u8>) { to_writer(&mut *out, &self.0).unwrap(); \
         let _ser = Serializer::new(out); let _v: Value = y::from_str(\"a\").unwrap(); } }\n",
    )]);
    assert_eq!(
        found,
        vec![
            ("Doc::write".into(), "serde_yaml_ng::Serializer::new".into()),
            ("Doc::write".into(), "serde_yaml_ng::to_writer".into()),
        ]
    );
}

#[test]
fn a_glob_import_makes_bare_serializer_names_sites_but_not_methods() {
    let found = calls(&[(
        "src/lib.rs",
        "use serde_yaml_ng::*;\n\
         fn emit(v: &V) -> String { let _n = v.to_string(); to_string(v).unwrap() }\n",
    )]);
    assert_eq!(found, one("emit", "serde_yaml_ng::to_string"));
}

#[test]
fn a_local_function_shadows_a_glob_import() {
    let found = calls(&[(
        "src/lib.rs",
        "use serde_yaml_ng::*;\nfn to_string(v: &V) -> String { String::new() }\n\
         fn emit(v: &V) -> String { to_string(v) }\n",
    )]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_crate_reexport_links_two_modules() {
    let found = calls(&[
        ("src/lib.rs", "mod backend;\nmod writer;\n"),
        (
            "src/backend.rs",
            "pub use serde_yaml_ng as y;\npub use y::to_string as emit;\n",
        ),
        (
            "src/writer.rs",
            "use crate::backend::y;\nuse super::backend::*;\n\
             pub fn first(v: &V) -> String { y::to_string(v).unwrap() }\n\
             pub fn second(v: &V) -> String { emit(v).unwrap() }\n\
             pub fn third(v: &V) -> String { crate::backend::y::to_string(v).unwrap() }\n",
        ),
    ]);
    assert_eq!(
        found,
        vec![
            ("first".into(), "serde_yaml_ng::to_string".into()),
            ("second".into(), "serde_yaml_ng::to_string".into()),
            ("third".into(), "serde_yaml_ng::to_string".into()),
        ]
    );
}

#[test]
fn a_reexport_through_a_module_that_never_names_the_backend_is_followed() {
    let found = calls(&[
        ("src/lib.rs", "mod backend;\npub mod relay;\nmod writer;\n"),
        ("src/backend.rs", "pub use serde_yaml_ng::to_string;\n"),
        ("src/relay.rs", "pub use crate::backend::*;\n"),
        (
            "src/writer.rs",
            "use crate::relay::to_string;\nfn emit() { to_string(&1).unwrap(); }\n",
        ),
    ]);
    assert_eq!(found, one("emit", "serde_yaml_ng::to_string"));
}

#[test]
fn a_child_glob_of_a_parent_backend_glob_is_followed() {
    let found = calls(&[
        ("src/lib.rs", "use serde_yaml_ng::*;\nmod child;\n"),
        (
            "src/child.rs",
            "use super::*;\nfn emit() { to_writer(out(), &1).unwrap(); }\n",
        ),
    ]);
    assert_eq!(found, one("emit", "serde_yaml_ng::to_writer"));
}

#[test]
fn a_crate_root_reexport_of_the_backend_is_the_backend() {
    let found = calls(&[
        (
            "src/lib.rs",
            "pub use serde_yaml_ng;\nmod yaml { pub mod write; }\n",
        ),
        (
            "src/yaml/write.rs",
            "pub fn emit(v: &V) -> String { crate::serde_yaml_ng::to_string(v).unwrap() }\n",
        ),
    ]);
    assert_eq!(found, one("emit", "serde_yaml_ng::to_string"));
}

#[test]
fn calls_inside_macros_are_sites() {
    let found = calls(&[(
        "src/main.rs",
        "use serde_yaml_ng as yaml;\n\
         fn main() { println!(\"{}\", yaml::to_string(&1).unwrap()); println!(\"{}\", x.to_string()); }\n",
    )]);
    assert_eq!(found, one("main", "serde_yaml_ng::to_string"));
}

#[test]
fn similarly_named_serializers_are_not_sites() {
    let found = calls(&[(
        "src/lib.rs",
        "use serde_json::to_string;\nmod local { pub fn to_writer() {} }\n\
         fn a(v: &V) -> String { serde_json::to_string(v).unwrap() }\n\
         fn b(v: &V) -> String { to_string(v).unwrap() }\n\
         fn c(v: &V) -> String { v.to_string() }\n\
         fn d() { local::to_writer() }\n\
         fn e(v: &V) -> String { toml::to_string(v).unwrap() }\n",
    )]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn test_only_code_is_not_scanned() {
    let found = calls(&[
        (
            "src/lib.rs",
            "#[cfg(test)]\nmod tests { fn f() { serde_yaml_ng::to_string(&1).unwrap(); } }\n\
             #[cfg(test)]\nmod more_tests;\n\
             #[cfg(all(test, unix))]\nfn helper() { serde_yaml_ng::to_string(&1).unwrap(); }\n\
             #[test]\nfn direct() { serde_yaml_ng::to_string(&1).unwrap(); }\n\
             struct S;\nimpl S {\n#[cfg(test)]\nfn probe() { serde_yaml::to_string(&1).unwrap(); } }\n\
             #[cfg(not(test))]\nfn shipped() { serde_yaml_ng::to_string(&1).unwrap(); }\n",
        ),
        (
            "src/more_tests.rs",
            "fn g() { serde_yaml_ng::to_string(&1).unwrap(); }\n",
        ),
    ]);
    assert_eq!(found, one("shipped", "serde_yaml_ng::to_string"));
}

#[test]
fn out_of_line_modules_and_bin_roots_are_scanned_with_lines() {
    let report = scan_sources(&[
        ("src/lib.rs", "pub mod outer;\n"),
        ("src/outer.rs", "pub mod inner;\n"),
        (
            "src/outer/inner/mod.rs",
            "\n\npub fn emit() { serde_yaml_ng::to_string(&1).unwrap(); }\n",
        ),
        (
            "src/bin/tool.rs",
            "fn main() { serde_yaml::to_string(&1).unwrap(); }\n",
        ),
    ])
    .expect("fixture scans");
    let found: Vec<(&str, usize, &str)> = report
        .sites
        .iter()
        .map(|s| (s.file.as_str(), s.line, s.symbol.as_str()))
        .collect();
    assert_eq!(
        found,
        vec![
            ("src/bin/tool.rs", 1, "main"),
            ("src/outer/inner/mod.rs", 3, "emit")
        ]
    );
    assert_eq!(report.files.len(), 4);
}

#[test]
fn a_mod_without_a_file_is_an_error() {
    let error = scan_sources(&[("src/lib.rs", "mod missing;\n")]).expect_err("missing module");
    assert!(error.to_string().contains("mod missing;"), "{error}");
}

#[test]
fn an_unlisted_site_names_file_line_symbol_and_the_fix() {
    let report = scan_sources(&[(
        "src/lib.rs",
        "struct W;\nimpl W {\n    fn emit(&self) -> String { serde_yaml_ng::to_string(&1).unwrap() }\n}\n",
    )])
    .expect("fixture scans");
    let problems = violations(&report, &[]);
    assert_eq!(problems.len(), 1, "{problems:?}");
    let message = &problems[0];
    for needle in [
        "src/lib.rs:3",
        "`W::emit`",
        "serde_yaml_ng::to_string",
        "biscuit_file::to_yaml_string",
        "symbol: \"W::emit\"",
    ] {
        assert!(message.contains(needle), "missing {needle:?} in {message}");
    }
}

#[test]
fn a_listed_site_passes_and_a_stale_entry_fails() {
    let report = scan_sources(&[(
        "src/lib.rs",
        "fn emit() { serde_yaml_ng::to_string(&1).unwrap(); }\n",
    )])
    .expect("fixture scans");
    let live = Allow {
        file: "src/lib.rs",
        symbol: "emit",
        reason: "byte contract",
    };
    assert!(violations(&report, &[live]).is_empty());

    let stale = Allow {
        file: "src/lib.rs",
        symbol: "gone",
        reason: "was a contract",
    };
    let problems = violations(&report, &[live, stale]);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].contains("stale allowlist entry src/lib.rs::gone"),
        "{}",
        problems[0]
    );
}

#[test]
fn an_entry_without_a_reason_fails() {
    let report = scan_sources(&[(
        "src/lib.rs",
        "fn emit() { serde_yaml_ng::to_string(&1).unwrap(); }\n",
    )])
    .expect("fixture scans");
    let problems = violations(
        &report,
        &[Allow {
            file: "src/lib.rs",
            symbol: "emit",
            reason: " ",
        }],
    );
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("has no reason"), "{}", problems[0]);
}
