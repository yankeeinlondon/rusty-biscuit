//! Engine behavior the public matrix cannot reach: nesting depth, overflow,
//! a sibling-run body, and error placement. Per-language totals are pinned
//! through the public query in `tests/l1/symbols_metrics.rs`.

use super::*;
use tree_sitter::{Parser, Tree};

fn parse(language: ProgrammingLanguage, source: &str) -> Tree {
    let mut parser = Parser::new();
    parser.set_language(&language.tree_sitter_language()).expect("grammar loads");
    parser.parse(source, None).expect("source parses")
}

/// The first declaration of `kind` in pre-order.
fn first_of_kind<'tree>(root: Node<'tree>, kind: &str) -> Node<'tree> {
    let mut cursor = root.walk();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if node.kind() == kind {
            return node;
        }
        pending.extend(node.children(&mut cursor).collect::<Vec<_>>().into_iter().rev());
    }
    panic!("no `{kind}` node");
}

fn rust_total(source: &str) -> Option<u32> {
    let tree = parse(ProgrammingLanguage::Rust, source);
    let function = first_of_kind(tree.root_node(), "function_item");
    let body = callable_body(function, ProgrammingLanguage::Rust).expect("a body");
    cyclomatic(body, ProgrammingLanguage::Rust)
}

/// 20 000 nested `if`s on a 256 KiB stack: a walk that recursed per syntax
/// node would overflow it, the cursor walk counts every one.
#[test]
fn deep_nesting_is_counted_without_recursion() {
    const DEPTH: usize = 20_000;
    let source = format!("fn f(a: bool) {{ {}{} }}", "if a { ".repeat(DEPTH), "} ".repeat(DEPTH));
    let tree = parse(ProgrammingLanguage::Rust, &source);
    assert!(!tree.root_node().has_error(), "the nested fixture parses cleanly");
    let total = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let function = first_of_kind(tree.root_node(), "function_item");
            let body = callable_body(function, ProgrammingLanguage::Rust).expect("a body");
            cyclomatic(body, ProgrammingLanguage::Rust)
        })
        .expect("thread starts")
        .join()
        .expect("the walk does not overflow the stack");
    assert_eq!(total, Some(DEPTH as u32 + 1));
}

#[test]
fn a_total_beyond_u32_is_unavailable_not_wrapped() {
    let mut total = Tally(u32::MAX - 1);
    assert_eq!(total.add(1), Some(()));
    assert_eq!(total.0, u32::MAX);
    assert_eq!(total.add(1), None);
}

/// A syntax error counts against the body only where the walk reads it: in
/// the visited region and inside an opaque macro token tree, never inside a
/// pruned nested declaration.
#[test]
fn only_errors_in_the_counted_body_make_the_total_unavailable() {
    assert_eq!(rust_total("fn f(a: bool) { if a { let x = ; } }"), None);
    assert_eq!(rust_total("fn f(a: bool) { if a {} m!(a ]); }"), None);
    assert_eq!(rust_total("fn f(a: bool) { if a {} fn inner() { let x = ; } }"), Some(2));
    assert_eq!(rust_total("fn f(a: bool) { if a {} let c = || { let x = ; }; }"), Some(2));
}

#[test]
fn languages_without_reviewed_rules_have_no_value() {
    let tree = parse(ProgrammingLanguage::Go, "package p\nfunc f(a bool) { if a {} }\n");
    let function = first_of_kind(tree.root_node(), "function_declaration");
    let body = callable_body(function, ProgrammingLanguage::Go).expect("a body");
    assert_eq!(cyclomatic(body, ProgrammingLanguage::Go), None);
}

/// A Python body is a run of the function's children; the walk visits each
/// of them, from the first after `:` to the last, and nothing after.
#[test]
fn a_sibling_run_body_is_walked_from_first_to_last() {
    static PYTHON_IFS: Rules = Rules {
        boundaries: &["function_definition"],
        opaque: &[],
        types: &[],
        type_sides: &[],
        decisions: &[Decision { kind: "if_statement", counts: Counts::Always }],
    };
    let source = "def f(a):  # if a:\n    # if a:\n    if a:\n        pass\n    if a:\n        pass\n\nif a:\n    pass\n";
    let tree = parse(ProgrammingLanguage::Python, source);
    let function = first_of_kind(tree.root_node(), "function_definition");
    let body = callable_body(function, ProgrammingLanguage::Python).expect("a body");
    assert!(matches!(body, CallableBody::Run { .. }), "{body:?}");
    assert_eq!(count(body, &PYTHON_IFS), Some(3));
}

/// The kinds a grammar's `node-types.json` files under `supertypes`.
fn subtypes(node_types: &str, supertypes: &[&str]) -> Vec<String> {
    let entries: Vec<serde_json::Value> = serde_json::from_str(node_types).expect("node types parse");
    let mut found = 0;
    let mut kinds = Vec::new();
    for entry in entries.iter().filter(|entry| supertypes.iter().any(|name| entry["type"] == *name)) {
        // The keyword `type` has an entry of its own, with no subtypes.
        let Some(listed) = entry["subtypes"].as_array() else { continue };
        found += 1;
        for subtype in listed {
            let kind = subtype["type"].as_str().expect("a kind");
            if !supertypes.contains(&kind) {
                kinds.push(kind.to_string());
            }
        }
    }
    assert_eq!(found, supertypes.len(), "the grammar declares every supertype in {supertypes:?}");
    kinds
}

/// Every kind the grammar files under its type supertypes is pruned as type
/// syntax, except the kinds it shares with value syntax.
#[test]
fn every_grammar_type_kind_is_pruned_or_a_shared_value_kind() {
    const TS_SHARED: &[&str] = &["call_expression", "member_expression", "const"];
    let cases: [(&str, &[&str], &Rules, &[&str]); 3] = [
        (tree_sitter_rust::NODE_TYPES, &["_type"], &rust::RULES, &["macro_invocation", "metavariable"]),
        (tree_sitter_typescript::TYPESCRIPT_NODE_TYPES, &["type", "primary_type"], &typescript::RULES, TS_SHARED),
        (tree_sitter_typescript::TSX_NODE_TYPES, &["type", "primary_type"], &typescript::RULES, TS_SHARED),
    ];
    for (node_types, supertypes, rules, shared) in cases {
        for kind in subtypes(node_types, supertypes) {
            let kind = kind.as_str();
            assert!(
                rules.types.contains(&kind) != shared.contains(&kind),
                "{supertypes:?}: `{kind}` is either pruned as a type or a listed value kind"
            );
        }
    }
}
