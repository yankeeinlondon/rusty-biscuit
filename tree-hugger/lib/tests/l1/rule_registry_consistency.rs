//! The rule registry, the `lint.scm` queries, the Rust-emitted rule list and
//! the message table describe the same rules, so none can drift alone.

use std::collections::{BTreeMap, BTreeSet};

use tree_hugger::file::tree_file::SEMANTIC_RULE_IDS;
use tree_hugger::queries::{GrammarRef, QueryKind, format_rule_message, query_for};
use tree_hugger::{ProgrammingLanguage, RuleRegistry};

/// Languages are keyed by query name, since `ProgrammingLanguage` is not `Ord`.
type Captures = BTreeMap<&'static str, BTreeSet<String>>;
type Rules = BTreeMap<String, BTreeSet<&'static str>>;

/// The `@diagnostic.<id>` capture IDs of every language's lint query.
fn lint_captures() -> Captures {
    ProgrammingLanguage::all()
        .into_iter()
        .map(|language| {
            let grammar = language.tree_sitter_language();
            let grammar_ref = GrammarRef { language, grammar: &grammar, id: language.query_name() };
            let query = query_for(grammar_ref, QueryKind::Lint).expect("lint query compiles");
            let ids = query
                .capture_names()
                .iter()
                .filter_map(|name| name.strip_prefix("diagnostic."))
                .map(str::to_string)
                .collect();
            (language.query_name(), ids)
        })
        .collect()
}

/// Canonical registered rules (aliases excluded) and their languages.
fn registered_rules() -> Rules {
    let registry = RuleRegistry::builtin();
    registry
        .rule_ids()
        .map(|id| (id.clone(), registry.get(id).unwrap().languages.iter().map(|language| language.query_name()).collect()))
        .collect()
}

/// Every way `rules`, `semantic` and `captures` disagree; empty when they agree.
fn disagreements(rules: &Rules, semantic: &[&str], captures: &Captures) -> Vec<String> {
    let mut problems = Vec::new();
    for (language, captured) in captures {
        let expected: BTreeSet<String> = rules
            .iter()
            .filter(|(id, languages)| !semantic.contains(&id.as_str()) && languages.contains(language))
            .map(|(id, _)| id.clone())
            .collect();
        if *captured != expected {
            problems.push(format!("{language}: lint.scm captures {captured:?}, registry lists {expected:?}"));
        }
    }
    for id in rules.keys() {
        let queried = captures.values().any(|captured| captured.contains(id));
        let in_rust = semantic.contains(&id.as_str());
        if queried == in_rust {
            problems.push(format!("{id}: emitted by a query: {queried}, listed as Rust-emitted: {in_rust}"));
        }
    }
    for id in semantic {
        if !rules.contains_key(*id) {
            problems.push(format!("{id}: listed as Rust-emitted but not registered"));
        }
    }
    problems
}

#[test]
fn registry_queries_and_semantic_rules_agree() {
    let problems = disagreements(&registered_rules(), SEMANTIC_RULE_IDS, &lint_captures());
    assert!(problems.is_empty(), "{problems:#?}");
}

/// The check above must catch a rule added on one side only.
#[test]
fn a_one_sided_rule_addition_is_a_disagreement() {
    let (rules, captures) = (registered_rules(), lint_captures());

    let mut query_only = captures.clone();
    query_only.get_mut("php").unwrap().insert("new-rule".to_string());
    assert!(!disagreements(&rules, SEMANTIC_RULE_IDS, &query_only).is_empty());

    let mut registry_only = rules.clone();
    registry_only.insert("new-rule".to_string(), BTreeSet::from(["php"]));
    assert!(!disagreements(&registry_only, SEMANTIC_RULE_IDS, &captures).is_empty());

    let mut language_only = rules.clone();
    language_only.get_mut("eval-call").unwrap().insert("lua");
    assert!(!disagreements(&language_only, SEMANTIC_RULE_IDS, &captures).is_empty());

    let semantic_extra: Vec<&str> = SEMANTIC_RULE_IDS.iter().copied().chain(["new-rule"]).collect();
    assert!(!disagreements(&rules, &semantic_extra, &captures).is_empty());
}

#[test]
fn semantic_rule_ids_are_the_rust_emitted_rules() {
    assert_eq!(
        SEMANTIC_RULE_IDS.iter().copied().collect::<BTreeSet<_>>(),
        BTreeSet::from(["dead-code", "undefined-symbol", "unused-symbol", "unused-import", "undefined-module", "invalid-syntax"])
    );
}

/// The match-arm IDs of `format_rule_message`, read from its source because
/// the table has no other enumeration.
fn message_table_ids() -> Vec<String> {
    let source = include_str!("../../src/queries/mod.rs");
    let body = source
        .split("pub fn format_rule_message")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("format_rule_message is in queries/mod.rs");
    body.lines()
        .filter_map(|line| line.trim().strip_prefix('"'))
        .filter_map(|line| line.split_once("\" =>").map(|(id, _)| id.to_string()))
        .collect()
}

#[test]
fn message_table_names_only_registered_rules() {
    let ids = message_table_ids();
    assert!(ids.len() >= 10, "the arm parser found too few IDs: {ids:?}");
    let registry = RuleRegistry::builtin();
    let unregistered: Vec<&String> = ids.iter().filter(|id| registry.get(id).is_none()).collect();
    assert!(unregistered.is_empty(), "message arms for unregistered rules: {unregistered:?}");
}

/// Every pattern rule has its own message rather than the generic fallback.
#[test]
fn every_query_emitted_rule_has_a_message() {
    for id in RuleRegistry::builtin().rule_ids().filter(|id| !SEMANTIC_RULE_IDS.contains(&id.as_str())) {
        assert_ne!(format_rule_message(id), format!("Lint rule: {id}"), "{id}");
    }
}

#[test]
fn removed_rules_are_not_registered() {
    let registry = RuleRegistry::builtin();
    for id in ["unused-variable", "shadowed-variable", "deprecated-syntax"] {
        assert!(!registry.has_rule(id), "{id}");
        assert_eq!(format_rule_message(id), format!("Lint rule: {id}"));
    }
    assert!(registry.has_rule("unreachable-code"), "the dead-code alias still resolves");
}

#[test]
fn eval_and_exec_metadata() {
    let registry = RuleRegistry::builtin();
    let eval = registry.get("eval-call").unwrap();
    let exec = registry.get("exec-call").unwrap();
    assert_eq!((eval.version.as_str(), exec.version.as_str()), ("1.1.0", "1.1.0"));
    assert!(eval.languages.contains(&ProgrammingLanguage::Python));
    assert_eq!(exec.title, "Dynamic code or command execution (exec)");
    assert!(exec.examples.iter().any(|example| example.language == ProgrammingLanguage::Php));
    for excluded in ["shell_exec", "system", "passthru", "proc_open", "popen", "backtick"] {
        assert!(exec.caveats.iter().any(|caveat| caveat.contains(excluded)), "{excluded}");
    }
}
