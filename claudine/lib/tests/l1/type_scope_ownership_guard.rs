//! Claudine's static presence and type decisions go through the shared
//! `TypeScope` (`2026-10-16-type-narrowing`, acceptance 8).
//!
//! The guard reads production source under `claudine/lib/src` and
//! `darkmatter/lib/src/lifecycle`, where Claudine's lifecycle walk lives (comments,
//! literals, and test-only code blanked by `source_scan`) and fails when:
//!
//! - a scope primitive is called outside its reviewed sites: an unguarded
//!   scope, a declared layer, a guard fact (`assume`), or a presence lookup.
//!   Each [`Allowed`] entry names its file, function, count, and reason; an
//!   unlisted call, a changed count, and a stale entry all fail.
//! - a function in a static pre-flight module refuses a missing value
//!   without asking [`PRESENCE_DECISION`] whether a guard proves it present,
//!   or the retired classifier `resolve_shell_command_expr` returns.
//! - a named decision site disappears or stops consulting the scope.
//!
//! `is_null` is never banned: runtime dispatch, predicate implementations,
//! and warning-policy absence handling need it, and an approval read that
//! only reports nulls decides nothing.

#[path = "../../../../darkmatter/cli/tests/common/source_scan.rs"]
mod source_scan;

use std::collections::BTreeMap;

use source_scan::{ident_offsets, line_at, matching_brace, production_sources, sanitize};

/// The one function that decides whether a guard proves a placeholder present.
const PRESENCE_DECISION: &str = "placeholder_guard";

/// Files whose functions make static (approval-time) presence decisions.
fn is_static_preflight_module(path: &str) -> bool {
    path == "composition/preflight.rs"
        || path == "darkmatter/lib/src/lifecycle/preflight/shell_walk.rs"
        || path.starts_with("composition/sequence/preflight/")
}

/// A scope primitive whose call sites are reviewed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Primitive {
    /// `TypeScope::unguarded(..)`: a scope with no guard facts.
    Unguarded,
    /// `DeclaredTypes::new(..)` / `DeclaredTypes::for_schema(..)`: the
    /// declared layer a scope reads.
    DeclaredTypes,
    /// `assume` / `assume_at`: the only fact producers.
    Assume,
    /// `type_of_expr` / `guard_presence` / `proves_present`: presence lookups.
    PresenceLookup,
}

impl Primitive {
    const ALL: [Self; 4] = [Self::Unguarded, Self::DeclaredTypes, Self::Assume, Self::PresenceLookup];

    /// Offsets of this primitive's calls in sanitized `text`.
    fn calls(self, text: &str) -> Vec<usize> {
        let names: &[&str] = match self {
            Self::Unguarded => &["unguarded"],
            Self::DeclaredTypes => &["DeclaredTypes"],
            Self::Assume => &["assume", "assume_at"],
            Self::PresenceLookup => &["type_of_expr", "guard_presence", "proves_present"],
        };
        let mut offsets: Vec<usize> = names
            .iter()
            .flat_map(|name| {
                ident_offsets(text, name).into_iter().filter(move |&at| {
                    let after = text[at + name.len()..].trim_start();
                    let called = match self {
                        Self::DeclaredTypes => after.strip_prefix("::").is_some_and(|rest| {
                            let rest = rest.trim_start();
                            rest.starts_with("new") || rest.starts_with("for_schema")
                        }),
                        _ => after.starts_with('(') || after.starts_with("::<"),
                    };
                    called && !is_definition(text, at)
                })
            })
            .collect();
        offsets.sort_unstable();
        offsets
    }
}

/// One reviewed call site of a [`Primitive`].
struct Allowed {
    primitive: Primitive,
    /// Path relative to `claudine/lib/src`, or the repository-relative path
    /// of a file in Darkmatter's lifecycle module.
    path: &'static str,
    function: &'static str,
    count: usize,
    reason: &'static str,
}

const fn allowed(primitive: Primitive, path: &'static str, function: &'static str, count: usize, reason: &'static str) -> Allowed {
    Allowed { primitive, path, function, count, reason }
}

const ALLOWED: &[Allowed] = &[
    allowed(
        Primitive::Unguarded,
        "darkmatter/lib/src/lifecycle/preflight/shell_walk.rs",
        "stamp",
        1,
        "each lifecycle event's walk starts from the declarations alone; `walk_scoped` adds its guards",
    ),
    allowed(
        Primitive::Unguarded,
        "composition/sequence/preflight/mod.rs",
        "site_types",
        1,
        "a task site's scope starts unguarded and assumes the task's own `when:`",
    ),
    allowed(
        Primitive::Unguarded,
        "darkmatter/lib/src/lifecycle/validate.rs",
        "lifecycle_call_checks",
        1,
        "each lifecycle event's call check starts from its declarations alone; `walk_stack_scoped` adds its guards",
    ),
    allowed(
        Primitive::DeclaredTypes,
        "darkmatter/lib/src/lifecycle/shell_template.rs",
        "lifecycle_declared_types",
        1,
        "the lifecycle declared layer: the owner's schema with the lifecycle host globals",
    ),
    allowed(
        Primitive::DeclaredTypes,
        "composition/sequence/preflight/mod.rs",
        "prepare_site",
        1,
        "a task `shell:` site is typed against its authoring document with the sequence host globals",
    ),
    allowed(
        Primitive::DeclaredTypes,
        "composition/sequence/preflight/mod.rs",
        "record_prompt_reference",
        1,
        "a task `with:` value is decided against the same declared layer as its `shell:`",
    ),
    allowed(
        Primitive::Assume,
        "darkmatter/lib/src/lifecycle/actions.rs",
        "branch_scopes",
        2,
        "every scoped lifecycle walk gives a conditional's `then` and `else` bodies the true and false sides of its `when`",
    ),
    allowed(
        Primitive::Assume,
        "composition/sequence/preflight/mod.rs",
        "site_types",
        1,
        "a task's `when:` holds on its true side for its `shell:` and `with:`",
    ),
    allowed(
        Primitive::PresenceLookup,
        "darkmatter/lib/src/lifecycle/shell_template.rs",
        PRESENCE_DECISION,
        2,
        "the one presence decision: the scope's type of the placeholder, then its guard",
    ),
];

/// A named static decision site and what it must (or must not) call.
struct DecisionSite {
    path: &'static str,
    function: &'static str,
    requires: &'static [&'static str],
    forbids: &'static [&'static str],
}

const DECISION_SITES: &[DecisionSite] = &[
    DecisionSite {
        path: "darkmatter/lib/src/lifecycle/preflight/shell_walk.rs",
        function: "resolve_shell_text",
        requires: &[PRESENCE_DECISION],
        forbids: &[],
    },
    DecisionSite {
        path: "darkmatter/lib/src/lifecycle/preflight/shell_walk.rs",
        function: "guarded_template",
        requires: &["project_types", "retime_guarded"],
        forbids: &[],
    },
    DecisionSite {
        path: "darkmatter/lib/src/lifecycle/preflight/shell_walk.rs",
        function: "sequence_lifecycle_template",
        requires: &[PRESENCE_DECISION, "typed_template"],
        forbids: &[],
    },
    DecisionSite {
        path: "composition/sequence/preflight/mod.rs",
        function: "prepare_site",
        requires: &[PRESENCE_DECISION, "site_types"],
        forbids: &[],
    },
    // It reports every null read; `prepare_site` decides each one.
    DecisionSite {
        path: "composition/sequence/preflight/mod.rs",
        function: "resolve_approval_time",
        requires: &[],
        forbids: &["MissingValue", "missing_value", PRESENCE_DECISION],
    },
];

/// Functions a static pre-flight module may call that are retired competing
/// classifiers.
const RETIRED: &[&str] = &["resolve_shell_command_expr"];

/// Builds the missing-value error; every caller is itself held to the rule.
const MISSING_VALUE_CONSTRUCTORS: &[(&str, &str)] = &[("darkmatter/lib/src/lifecycle/preflight/shell_walk.rs", "missing_value")];

/// One `fn` and its body's byte range in sanitized text.
struct Function<'a> {
    name: &'a str,
    body: std::ops::Range<usize>,
}

/// Whether the identifier at `at` is the name in a `fn` item.
fn is_definition(text: &str, at: usize) -> bool {
    let before = text[..at].trim_end();
    before.strip_suffix("fn").is_some_and(|rest| !rest.bytes().last().is_some_and(source_scan::is_ident))
}

/// Every `fn` with a body in sanitized `text`.
fn functions(text: &str) -> Vec<Function<'_>> {
    let bytes = text.as_bytes();
    ident_offsets(text, "fn")
        .into_iter()
        .filter_map(|at| {
            let rest = &text[at + 2..];
            let name_start = at + 2 + (rest.len() - rest.trim_start().len());
            let name_len = text[name_start..].bytes().take_while(|&byte| source_scan::is_ident(byte)).count();
            if name_len == 0 {
                return None; // a `fn(..)` pointer type
            }
            let after = name_start + name_len;
            let open = after + text[after..].find(['{', ';'])?;
            (bytes[open] == b'{').then(|| Function { name: &text[name_start..after], body: open..matching_brace(bytes, open) })
        })
        .collect()
}

/// The innermost function whose body holds `offset`.
fn enclosing<'a>(functions: &'a [Function<'a>], offset: usize) -> Option<&'a str> {
    functions
        .iter()
        .filter(|function| function.body.contains(&offset))
        .max_by_key(|function| function.body.start)
        .map(|function| function.name)
}

/// Whether sanitized `body` names `ident` (a call, a path, or a variant).
fn names(body: &str, ident: &str) -> bool {
    !ident_offsets(body, ident).is_empty()
}

/// Calls of each [`Primitive`] outside, or beyond, its [`Allowed`] entries,
/// and entries that no longer match a live site.
fn allowlist_violations(sources: &BTreeMap<String, String>, allowed: &[Allowed]) -> Vec<String> {
    let mut found: BTreeMap<(Primitive, String, String), (usize, Vec<usize>)> = BTreeMap::new();
    for (path, text) in sources {
        let functions = functions(text);
        for primitive in Primitive::ALL {
            for at in primitive.calls(text) {
                let function = enclosing(&functions, at).unwrap_or("<item>").to_string();
                let entry = found.entry((primitive, path.clone(), function)).or_default();
                entry.0 += 1;
                entry.1.push(line_at(text, at));
            }
        }
    }
    let mut violations = Vec::new();
    for entry in allowed {
        if entry.reason.trim().is_empty() {
            violations.push(format!("{:?} in {}::{} has no reason", entry.primitive, entry.path, entry.function));
        }
        let key = (entry.primitive, entry.path.to_string(), entry.function.to_string());
        match found.remove(&key) {
            None => violations.push(format!(
                "stale entry: no {:?} call in {}::{} (remove it)",
                entry.primitive, entry.path, entry.function
            )),
            Some((count, lines)) if count != entry.count => violations.push(format!(
                "{:?} in {}::{}: {count} calls (lines {lines:?}), {} reviewed",
                entry.primitive, entry.path, entry.function, entry.count
            )),
            Some(_) => {}
        }
    }
    for ((primitive, path, function), (_, lines)) in found {
        violations.push(format!(
            "unreviewed {primitive:?} call in {path}::{function} (lines {lines:?}); \
             thread the enclosing site's scope, or add a reviewed entry with its reason"
        ));
    }
    violations
}

/// Functions in a static pre-flight module that refuse a missing value
/// without consulting [`PRESENCE_DECISION`], and any retired classifier.
fn presence_violations(sources: &BTreeMap<String, String>) -> Vec<String> {
    let mut violations = Vec::new();
    for (path, text) in sources {
        for retired in RETIRED {
            for at in ident_offsets(text, retired) {
                violations.push(format!("{path}:{}: retired classifier `{retired}`", line_at(text, at)));
            }
        }
        if !is_static_preflight_module(path) {
            continue;
        }
        for function in functions(text) {
            let body = &text[function.body.clone()];
            if MISSING_VALUE_CONSTRUCTORS.contains(&(path.as_str(), function.name)) {
                continue;
            }
            let builds = ident_offsets(body, "MissingValue")
                .into_iter()
                .any(|at| body[at + "MissingValue".len()..].trim_start().strip_prefix('{').is_some_and(|rest| !rest.trim_start().starts_with("..")));
            let calls = ident_offsets(body, "missing_value")
                .into_iter()
                .any(|at| body[at + "missing_value".len()..].trim_start().starts_with('('));
            if (builds || calls) && !names(body, PRESENCE_DECISION) {
                violations.push(format!(
                    "{path}::{}: refuses a missing value without asking `{PRESENCE_DECISION}` whether a guard proves it present",
                    function.name
                ));
            }
        }
    }
    violations
}

/// Named decision sites that disappeared or changed what they consult.
fn decision_site_violations(sources: &BTreeMap<String, String>, sites: &[DecisionSite]) -> Vec<String> {
    let mut violations = Vec::new();
    for site in sites {
        let Some(text) = sources.get(site.path) else {
            violations.push(format!("{}: no such production file", site.path));
            continue;
        };
        let functions = functions(text);
        let matching: Vec<_> = functions.iter().filter(|function| function.name == site.function).collect();
        let [function] = matching.as_slice() else {
            violations.push(format!("{}::{}: {} definitions, expected one", site.path, site.function, matching.len()));
            continue;
        };
        let body = &text[function.body.clone()];
        for required in site.requires {
            if !names(body, required) {
                violations.push(format!("{}::{} no longer calls `{required}`", site.path, site.function));
            }
        }
        for forbidden in site.forbids {
            if names(body, forbidden) {
                violations.push(format!("{}::{} must not name `{forbidden}`; it only reports reads", site.path, site.function));
            }
        }
    }
    violations
}

#[test]
fn static_presence_decisions_go_through_the_type_scope() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../../darkmatter/cli/tests/common/source_scan.rs");
    let mut sources = production_sources(&biscuit_test_harness::manifest_dir!().join("src"));
    // Claudine's lifecycle walk and templates run from Darkmatter's lifecycle
    // module, so its reviewed sites are held to this allowlist too.
    let lifecycle = biscuit_test_harness::manifest_dir!().join("../../darkmatter/lib/src/lifecycle");
    sources.extend(
        production_sources(&lifecycle).into_iter().map(|(path, text)| (format!("darkmatter/lib/src/lifecycle/{path}"), text)),
    );

    let mut violations = allowlist_violations(&sources, ALLOWED);
    violations.extend(presence_violations(&sources));
    violations.extend(decision_site_violations(&sources, DECISION_SITES));
    for (path, constructor) in MISSING_VALUE_CONSTRUCTORS {
        if !sources.get(*path).is_some_and(|text| functions(text).iter().any(|function| function.name == *constructor)) {
            violations.push(format!("stale constructor entry: no `fn {constructor}` in {path}"));
        }
    }

    assert!(violations.is_empty(), "type-scope ownership violations:\n{}", violations.join("\n"));
}

// -- self-tests: the scan must reject what it guards against ----------------

fn sources(files: &[(&str, &str)]) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|(path, text)| (path.to_string(), String::from_utf8(sanitize(text)).expect("UTF-8")))
        .collect()
}

#[test]
fn a_resurrected_classifier_that_ignores_the_scope_is_rejected() {
    let snippet = r#"
fn resolve_shell_command_expr(read: &Value, expression: &str) -> Result<(), CompositionError> {
    if read.is_null() {
        return Err(missing_value(raw, expression, property, source_path));
    }
    Ok(())
}
"#;
    let violations = presence_violations(&sources(&[("composition/preflight.rs", snippet)]));
    assert_eq!(violations.len(), 2, "{violations:#?}");
    assert!(violations[0].contains("retired classifier"), "{violations:#?}");
    assert!(violations[1].contains("without asking `placeholder_guard`"), "{violations:#?}");
}

#[test]
fn a_null_decision_in_sequence_preflight_must_consult_the_scope() {
    let unscoped = r#"
fn decide(missing: &[&str]) -> Result<(), CompositionError> {
    for expression in missing {
        return Err(site_failure(ShellExpansionError::MissingValue { ctx, command, expression, origin }));
    }
    Ok(())
}
"#;
    let violations = presence_violations(&sources(&[("composition/sequence/preflight/mod.rs", unscoped)]));
    assert_eq!(violations.len(), 1, "{violations:#?}");

    let scoped = r#"
fn decide(missing: &[&str], types: &TypeScope<'_>) -> Result<(), CompositionError> {
    for expression in missing {
        let Some(_guard) = placeholder_guard(expression, types) else {
            return Err(site_failure(ShellExpansionError::MissingValue { ctx, command, expression, origin }));
        };
    }
    Ok(())
}
"#;
    assert_eq!(presence_violations(&sources(&[("composition/sequence/preflight/mod.rs", scoped)])), Vec::<String>::new());
}

#[test]
fn an_unallowlisted_unguarded_scope_is_rejected_and_a_stale_entry_too() {
    let snippet = r#"
fn type_a_new_site(declared: &DeclaredTypes<'_>) -> TypeScope<'_> {
    TypeScope::unguarded(declared)
}
"#;
    let files = sources(&[("composition/lifecycle/new_site.rs", snippet)]);

    let violations = allowlist_violations(&files, &[]);
    assert_eq!(violations.len(), 1, "{violations:#?}");
    assert!(violations[0].contains("unreviewed Unguarded call in composition/lifecycle/new_site.rs::type_a_new_site"));

    let reviewed = [allowed(Primitive::Unguarded, "composition/lifecycle/new_site.rs", "type_a_new_site", 1, "a test reason")];
    assert_eq!(allowlist_violations(&files, &reviewed), Vec::<String>::new());

    let miscounted = [allowed(Primitive::Unguarded, "composition/lifecycle/new_site.rs", "type_a_new_site", 2, "a test reason")];
    assert_eq!(allowlist_violations(&files, &miscounted).len(), 1);

    let stale = allowlist_violations(&sources(&[("composition/lifecycle/new_site.rs", "fn other() {}")]), &reviewed);
    assert!(stale.len() == 1 && stale[0].starts_with("stale entry"), "{stale:#?}");
}

#[test]
fn a_second_presence_lookup_is_rejected() {
    let snippet = r#"
fn my_own_presence(expr: &Expr, scope: &TypeScope<'_>) -> bool {
    scope.type_of_expr(expr).is_some_and(|typed| typed.guard_presence().is_some())
}
"#;
    let violations = allowlist_violations(&sources(&[("composition/preflight.rs", snippet)]), &[]);
    assert_eq!(violations.len(), 1, "{violations:#?}");
    assert!(violations[0].contains("PresenceLookup"), "{violations:#?}");
}

#[test]
fn runtime_null_checks_absence_warnings_and_prose_are_permitted() {
    let executor = r#"
/// Mentions `TypeScope::unguarded(..)` and `resolve_shell_command_expr` in prose only.
fn fill(value: &Value) -> Result<String, HoleError> {
    if value.is_null() {
        return Err(HoleError::Missing);
    }
    let _ = "TypeScope::unguarded(&declared)";
    Ok(value.to_string())
}
"#;
    let warning = r#"
fn warn_absent(value: &Value) {
    if value.is_null() { warn!("absent"); }
}
"#;
    // An approval read that only reports nulls decides nothing.
    let reads = r#"
fn resolve_approval_time(reads: &[Value]) -> Vec<usize> {
    reads.iter().enumerate().filter(|(_, read)| read.is_null()).map(|(index, _)| index).collect()
}
"#;
    let files = sources(&[
        ("composition/lifecycle/executor.rs", executor),
        ("composition/lifecycle/warnings.rs", warning),
        ("composition/sequence/preflight/mod.rs", reads),
    ]);
    assert_eq!(allowlist_violations(&files, &[]), Vec::<String>::new());
    assert_eq!(presence_violations(&files), Vec::<String>::new());
    let site = DecisionSite {
        path: "composition/sequence/preflight/mod.rs",
        function: "resolve_approval_time",
        requires: &[],
        forbids: &["MissingValue", "missing_value", PRESENCE_DECISION],
    };
    assert_eq!(decision_site_violations(&files, &[site]), Vec::<String>::new());
}

#[test]
fn a_decision_site_that_stops_consulting_the_scope_is_rejected() {
    let snippet = r#"
fn resolve_shell_text(read: &Value) -> Result<(), CompositionError> {
    if read.is_null() { return Err(missing_value(raw, expression, property, source_path)); }
    Ok(())
}
"#;
    let files = sources(&[("darkmatter/lib/src/lifecycle/preflight/shell_walk.rs", snippet)]);
    let violations = decision_site_violations(&files, &DECISION_SITES[..1]);
    assert_eq!(violations, vec!["darkmatter/lib/src/lifecycle/preflight/shell_walk.rs::resolve_shell_text no longer calls `placeholder_guard`"]);
    let missing = decision_site_violations(&sources(&[("darkmatter/lib/src/lifecycle/preflight/shell_walk.rs", "fn other() {}")]), &DECISION_SITES[..1]);
    assert_eq!(missing.len(), 1, "{missing:#?}");
}
