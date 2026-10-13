//! The consumers of the re-export report: `imported_symbols()`'s
//! `is_reexport`, the stored import marker and the `unused-import`
//! exemption all come from proven forwarding entries, and agree.

use tree_hugger::{Capability, SymbolQuery};

use crate::reexports_support::{Snippet, snippet};

fn flagged(snippet: &Snippet) -> Vec<String> {
    let mut names: Vec<String> = snippet
        .file
        .imported_symbols()
        .expect("imports")
        .into_iter()
        .filter(|import| import.is_reexport)
        .map(|import| import.name)
        .collect();
    names.sort();
    names
}

fn marked(snippet: &Snippet) -> Vec<String> {
    let mut names: Vec<String> = snippet
        .indexed()
        .imports
        .into_iter()
        .filter(|import| import.reexported)
        .map(|import| import.name)
        .collect();
    names.sort();
    names
}

fn unused(snippet: &Snippet) -> Vec<String> {
    let mut names: Vec<String> = snippet
        .file
        .lint_diagnostics()
        .into_iter()
        .filter(|diagnostic| diagnostic.rule.as_deref() == Some("unused-import"))
        .map(|diagnostic| diagnostic.message)
        .collect();
    names.sort();
    names
}

#[test]
fn rust_flags_follow_the_forwarding_entries_and_pub_self_is_not_one() {
    let source = "pub use a::Public;\npub(crate) use b::Crate;\npub(self) use c::SelfOnly;\nuse d::Private;\n";
    let snippet = snippet("lib.rs", source);
    assert_eq!(flagged(&snippet), ["Crate", "Public"], "direct imported_symbols");
    assert_eq!(marked(&snippet), flagged(&snippet), "the stored marker agrees with the direct flag");

    let messages = unused(&snippet);
    assert_eq!(messages.len(), 2, "{messages:?}");
    assert!(messages.iter().any(|m| m.contains("SelfOnly")), "pub(self) loses the exemption: {messages:?}");
    assert!(messages.iter().any(|m| m.contains("Private")), "{messages:?}");
}

#[test]
fn the_planner_diagnostics_and_the_direct_lint_agree_on_the_exemption() {
    let snippet = snippet("lib.rs", "pub use a::Public;\npub(self) use c::SelfOnly;\nuse d::Private;\n");
    let query = SymbolQuery::new().kinds([]).capabilities([Capability::Diagnostics]);
    let index = snippet.file.query(&query).expect("query").index;
    assert!(index.reexports.is_some(), "Diagnostics requires Reexports");
    let mut from_index: Vec<String> = index
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.rule.as_deref() == Some("unused-import"))
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    from_index.sort();
    assert_eq!(from_index, unused(&snippet));
    assert!(!from_index.iter().any(|m| m.contains("Public")), "{from_index:?}");
}

#[test]
fn only_declared_or_syntax_evidence_exempts_a_python_import() {
    let source = "from m import redundant as redundant\nfrom m import plain\nfrom m import listed\n__all__ = [\"listed\"]\n";
    let snippet = snippet("mod.py", source);
    assert_eq!(flagged(&snippet), ["listed", "redundant"]);
    assert_eq!(marked(&snippet), flagged(&snippet));
    let messages = unused(&snippet);
    assert!(messages.iter().any(|m| m.contains("plain")), "an unlisted import stays reported: {messages:?}");
    assert!(!messages.iter().any(|m| m.contains("redundant") || m.contains("listed")), "{messages:?}");
}

#[test]
fn a_replaced_python_import_keeps_its_unused_import_warning() {
    let snippet = snippet("mod.py", "from old import read\nfrom new import read\n__all__ = [\"read\"]\n");
    let imports = snippet.file.imported_symbols().expect("imports");
    let forwarded: Vec<_> = imports.iter().map(|import| (import.source.as_deref(), import.is_reexport)).collect();
    assert_eq!(forwarded, [(Some("old"), false), (Some("new"), true)]);
    assert_eq!(marked(&snippet), ["read"]);

    let diagnostics: Vec<_> = snippet
        .file
        .lint_diagnostics()
        .into_iter()
        .filter(|diagnostic| diagnostic.rule.as_deref() == Some("unused-import"))
        .collect();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let range = |r: &tree_hugger::CodeRange| (r.start_byte, r.end_byte);
    assert_eq!(range(&diagnostics[0].range), range(&imports[0].range), "the warning is on the replaced import");
}

#[test]
fn a_conditional_all_append_does_not_exempt_the_import() {
    let guarded = snippet("mod.py", "from m import read\n__all__ = []\nif enabled:\n    __all__.append(\"read\")\n");
    assert_eq!(flagged(&guarded), Vec::<String>::new());
    assert_eq!(marked(&guarded), flagged(&guarded));
    let messages = unused(&guarded);
    assert!(messages.iter().any(|m| m.contains("read")), "a guessed listing keeps the warning: {messages:?}");

    let unguarded = snippet("mod.py", "from m import read\n__all__ = []\n__all__.append(\"read\")\n");
    assert_eq!(flagged(&unguarded), ["read"]);
    assert_eq!(marked(&unguarded), flagged(&unguarded));
    assert!(unused(&unguarded).is_empty(), "{:?}", unused(&unguarded));
}

#[test]
fn a_python_import_with_no_all_is_exposed_but_not_exempt() {
    let snippet = snippet("mod.py", "from m import plain\n");
    let report = snippet.report();
    assert_eq!(report.entries.len(), 1, "the module-level import is exposed");
    assert_eq!(flagged(&snippet), Vec::<String>::new());
    assert_eq!(marked(&snippet), Vec::<String>::new());
    assert_eq!(unused(&snippet).len(), 1);
}

#[test]
fn an_ecma_local_export_of_an_import_exempts_only_that_import() {
    let snippet = snippet("index.ts", "import { kept } from \"./k\";\nimport { dropped } from \"./d\";\nexport { kept };\n");
    assert_eq!(flagged(&snippet), ["kept"]);
    assert_eq!(marked(&snippet), ["kept"]);
    let messages = unused(&snippet);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(messages[0].contains("dropped"));
}

#[test]
fn a_query_without_reexports_marks_no_import() {
    let snippet = snippet("lib.rs", "pub use a::Public;\n");
    let index = snippet.file.query(&SymbolQuery::new()).expect("query").index;
    assert!(index.reexports.is_none());
    assert!(index.imports.iter().all(|import| !import.reexported), "not computed is not marked");
}

#[test]
fn an_ecma_import_forwarded_through_a_replaced_commonjs_member_is_not_flagged() {
    let replaced = snippet("mod.js", "import { a } from \"m\";\nexports.x = a;\nexports.x = 1;\n");
    assert_eq!(flagged(&replaced), Vec::<String>::new());
    assert_eq!(marked(&replaced), flagged(&replaced));
    // `exports.x = a` still reads `a`, so the import is used either way.
    assert!(unused(&replaced).is_empty(), "{:?}", unused(&replaced));

    let kept = snippet("mod.js", "import { a } from \"m\";\nexports.x = a;\nexports.y = 1;\n");
    assert_eq!(flagged(&kept), ["a"]);
    assert_eq!(marked(&kept), flagged(&kept));
}
