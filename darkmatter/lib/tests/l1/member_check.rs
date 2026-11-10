//! The undeclared-member check at the library's public API:
//! [`expression_member_findings`] and [`expression_member_diagnostics`] over
//! a typed site. Every case is passive: a schema parsed in memory (or
//! resolved from a temp dir for named types), an expression parsed from its
//! text, and nothing evaluated.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use std::sync::Arc;

use darkmatter::markdown::compose::expression::{
    Availability, BindingView, HostFunctionCatalog, HostFunctions, install_host_functions, DeclaredTypes, MemberFinding, Polarity, ScopeId, Severity, TypeScope, UNDECLARED_MEMBER_CODE,
    WriteSet, expression_member_diagnostics, expression_member_findings, parse, parse_spanned,
};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use simplified_schema::{SchemaRoot, parse_yaml_schema};

fn schema(yaml: &str) -> SchemaRoot {
    parse_yaml_schema(&serde_yaml_ng::from_str(yaml).expect("YAML parses")).expect("schema parses")
}

fn findings_at(site: &TypeScope<'_>, text: &str) -> Vec<MemberFinding> {
    let expr = parse_spanned(text).unwrap_or_else(|error| panic!("{text}: {error}"));
    expression_member_findings(&expr, text, site)
}

/// The findings of `text` at an unguarded site of the schema `yaml`.
fn findings(yaml: &str, text: &str) -> Vec<MemberFinding> {
    let schema = schema(yaml);
    let declared = DeclaredTypes::new(Some(&schema));
    findings_at(&TypeScope::unguarded(&declared), text)
}

/// Each finding as `(member, subject)`.
fn pairs(found: &[MemberFinding]) -> Vec<(&str, &str)> {
    found.iter().map(|finding| (finding.member.as_str(), finding.subject.as_str())).collect()
}

/// The member names reported for `text` under `yaml`.
fn members(yaml: &str, text: &str) -> Vec<String> {
    findings(yaml, text).into_iter().map(|finding| finding.member).collect()
}

const SERVER: &str = "server: '{ name: string }'\n";

/// Every row of the spec's type table, `server` typed by the first column.
#[test]
fn type_table_rows() {
    for (ty, member, warns) in [
        ("'{ name: string }'", "name", false),
        ("'{ name: string }'", "nmae", true),
        ("'{ name: string } | null'", "nmae", true),
        ("'null'", "nmae", false),
        ("'{ name: string } | { port: number }'", "name", false),
        ("'{ name: string } | { port: number }'", "nmae", true),
        ("'{ name: string } | string'", "nmae", false),
        ("unknown", "nmae", false),
        ("any", "nmae", false),
        ("object", "nmae", false),
        ("'{ <string>: string }'", "nmae", false),
    ] {
        let text = format!("server.{member}");
        let found = members(&format!("server: {ty}\n"), &text);
        assert_eq!(!found.is_empty(), warns, "server: {ty} / {text}: {found:?}");
    }
    // A declared optional member stays silent whether or not it is required.
    assert!(members("server: '{ name: string(required), port: number }'\n", "server.port").is_empty());
}

/// The same undeclared member through every access form, each reporting
/// the same member text.
#[test]
fn access_forms() {
    let yaml = "server: '{ name: string }'\nhosts: '{ name: string }[]'\n";
    for (text, subject) in [
        ("server.nmae", "server"),
        ("(server).nmae", "(server)"),
        ("server[\"nmae\"]", "server"),
        ("fm.server.nmae", "fm.server"),
        ("fm[\"server\"].nmae", "fm[\"server\"]"),
        ("map(hosts, h => local.h.nmae)", "local.h"),
        ("map(hosts, h => h.nmae)", "h"),
        ("{ name: \"x\" }.nmae", "{ name: \"x\" }"),
    ] {
        let found = findings(yaml, text);
        assert_eq!(pairs(&found), [("nmae", subject)], "{text}");
        assert_eq!(&text[found[0].span.clone()].trim_matches('"'), &"nmae", "{text}");
    }
    // The declared member is silent in every form.
    for text in ["(server).name", "server['name']", "fm['server'].name", "map(hosts, h => h.name)", "{ name: 1 }.name"] {
        assert!(findings(yaml, text).is_empty(), "{text}");
    }
}

const HOST_CATALOG: &str = "kind: schema\ncategories:\n    - Project -> Test host.\ntypes:\n    project_server: |-\n        function(\n            parameters(id: string);\n            returns({ name: string });\n            category(Project);\n        ) -> The project's server.\n";

/// A function's declared return types its result: a host registration
/// returning a closed object (no built-in expression function returns one).
#[test]
fn a_typed_function_return() {
    let catalog = HostFunctionCatalog::load(HOST_CATALOG, "tests/member_check.yaml").expect("catalog loads");
    let host = HostFunctions::builder(catalog)
        .function(
            "project_server",
            &[],
            |registration| registration,
            Arc::new(|_| panic!("a handler was entered during passive analysis")),
        )
        .build()
        .expect("host registers");
    // Process-wide: a second installation in one process is ignored.
    let _ = install_host_functions(host);
    let text = "project_server(\"a\").nmae";
    let found = findings(SERVER, text);
    assert_eq!(pairs(&found), [("nmae", "project_server(\"a\")")]);
    assert_eq!(&text[found[0].span.clone()], "nmae");
    assert!(findings(SERVER, "project_server(\"a\").name").is_empty());
}

#[test]
fn paths_and_keys() {
    // Only the first missing step reports; later steps are unknown.
    assert_eq!(members("a: '{ x: { c: string } }'\n", "a.b.c"), ["b"]);
    assert_eq!(members(SERVER, "server.nmae.length"), ["nmae"]);
    assert_eq!(members(SERVER, "(server).nmae.length"), ["nmae"]);
    // A bracket key is one key, dots and all.
    assert_eq!(members(SERVER, "server[\"a.b\"]"), ["a.b"]);
    assert_eq!(members("server: '{ a: { b: string } }'\n", "server[\"a.b\"]"), ["a.b"]);
    assert!(members("server: '{ a: { b: string } }'\n", "server.a.b").is_empty());
    // A computed key is not checked, and makes the steps after it unknown.
    assert!(members(SERVER, "server[key]").is_empty());
    assert!(members(SERVER, "server[key].nmae").is_empty());
    // An array index projects the item type.
    assert_eq!(members("hosts: '{ name: string }[]'\n", "hosts[0].nmae"), ["nmae"]);
    // An undeclared root has no known type: its own warning is elsewhere.
    assert!(members(SERVER, "sever.name").is_empty());
}

/// Namespaces, host globals, and `null` are never checked; a value selected
/// through `fm` or `local` is.
#[test]
fn namespaces_and_host_globals_are_not_checked() {
    for text in ["ctx.today.nmae", "env.HOME.nmae", "null.nmae", "fm.nmae", "local.nmae", "fm['nmae']"] {
        assert!(members(SERVER, text).is_empty(), "{text}");
    }
    let view = BindingView::builder(ScopeId::new("test.event"))
        .declare("err", "The failure.", Availability::Available)
        .build()
        .expect("valid view");
    let schema = schema(SERVER);
    let declared = DeclaredTypes::new(Some(&schema)).with_binding_view(&view);
    let site = TypeScope::unguarded(&declared);
    for text in ["err.nmae", "(err).nmae", "err['nmae']"] {
        assert!(findings_at(&site, text).is_empty(), "{text}");
    }
    assert_eq!(pairs(&findings_at(&site, "server.nmae")), [("nmae", "server")]);
}

/// Literal `load_data` contracts type the call's result; anything the
/// check cannot resolve passively stays unknown.
#[test]
fn contracts() {
    let yaml = "kind: string\n";
    assert_eq!(members(yaml, "load_data(\"h.yaml\", \"{ name: string }[]\")[0].nmae"), ["nmae"]);
    assert_eq!(members(yaml, "map(load_data(\"h.yaml\", \"{ name: string }[]\"), h => h.nmae)"), ["nmae"]);
    assert!(members(yaml, "map(load_data(\"h.yaml\", \"{ name: string }[]\"), h => h.name)").is_empty());
    assert!(members(yaml, "load_data(\"h.yaml\", kind)[0].nmae").is_empty(), "a computed contract");
    assert!(members(yaml, "load_data(\"h.yaml\")[0].nmae").is_empty(), "no contract");
    assert!(
        members(yaml, "load_data(\"h.yaml\", \"Server@./types.yaml\").nmae").is_empty(),
        "a contract naming an unresolved import"
    );
}

/// A named type in the site's definitions is checked through the bounded
/// reference projection.
#[test]
fn named_references_in_the_site_definitions() {
    let temp = tempfile::tempdir().unwrap();
    let dir = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
    std::fs::write(
        dir.join("types.yaml"),
        "kind: schema\ntypes:\n    node:\n        label: string(required)\n        children: node[](required)\n",
    )
    .unwrap();
    let path = dir.join("doc.md");
    std::fs::write(&path, "---\n$schema:\n  tree: node(required)@./types.yaml\n---\nBody\n").unwrap();
    let markdown = Markdown::try_from(path.as_path()).unwrap().with_source(ComposeSource::File(path.clone()));
    let context = biscuit_file::FileResolutionContext::new(&dir).with_repository_root(&dir);
    let effective = DarkmatterSchemas::new(context).effective_for(&markdown).unwrap().expect("a schema");
    let declared = DeclaredTypes::for_schema(Some(&effective));
    let site = TypeScope::unguarded(&declared);
    assert_eq!(pairs(&findings_at(&site, "tree.nmae")), [("nmae", "tree")]);
    assert_eq!(pairs(&findings_at(&site, "tree.children[0].nmae")), [("nmae", "tree.children[0]")]);
    assert!(findings_at(&site, "tree.children[0].children[0].label").is_empty());
    assert_eq!(members_of(&findings_at(&site, "map(tree.children, c => c.lable)")), ["lable"]);
}

fn members_of(found: &[MemberFinding]) -> Vec<&str> {
    found.iter().map(|finding| finding.member.as_str()).collect()
}

/// `try(a, b)` is typed as the union of both branches.
#[test]
fn try_typing() {
    let yaml = SERVER;
    assert_eq!(
        members(yaml, "map(try(load_data(\"h.yaml\", \"{ a: string }[]\"), []), h => h.nmae)"),
        ["nmae"],
        "an empty-array fallback keeps the item type"
    );
    assert_eq!(members(yaml, "try(server, {}).nmae"), ["nmae"], "both closed arms lack it");
    assert!(members(yaml, "try(server, {}).name").is_empty(), "one arm declares it");
    assert!(members(yaml, "try(server, { nmae: 1 }).nmae").is_empty());
    assert_eq!(members(yaml, "try(server.nmae, \"x\")"), ["nmae"], "try is not an absence guard");
}

#[test]
fn narrowing() {
    let yaml = "server: '{ name: string } | string'\n";
    assert!(members(yaml, "server.nmae").is_empty(), "not every alternative is an object");
    assert_eq!(members(yaml, "is_string(server) ? \"\" : server.nmae"), ["nmae"], "the false side narrows");
    assert!(members(yaml, "is_string(server) ? server.nmae : \"\"").is_empty());

    let schema = schema(yaml);
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let narrowed = site.assume(&parse("is_string(server)").unwrap(), Polarity::False);
    assert_eq!(members_of(&findings_at(&narrowed, "server.nmae")), ["nmae"]);
    let written = narrowed.invalidate(&WriteSet::destination("server"));
    assert!(findings_at(&written, "server.nmae").is_empty(), "a write drops the narrowing");
}

#[test]
fn shadowing() {
    let yaml = "server: '{ name: string }'\nhosts: '{ port: number }[]'\napps: '{ title: string }[]'\n";
    // A callback parameter is typed by its item, not the document property
    // it shadows.
    assert!(members(yaml, "map(hosts, server => server.port)").is_empty());
    assert_eq!(members(yaml, "map(hosts, server => server.name)"), ["name"]);
    // An outer capture keeps the outer type.
    assert_eq!(pairs(&findings(yaml, "map(hosts, h => server.nmae)")), [("nmae", "server")]);
    // Equal spellings over unrelated item types are separate sites.
    let found = findings(yaml, "[map(hosts, h => h.nmae), map(apps, h => h.nmae)]");
    assert_eq!(members_of(&found), ["nmae", "nmae"]);
    assert_ne!(found[0].node_ordinal, found[1].node_ordinal);
    assert!(found[0].span.start < found[1].span.start);
}

/// Spans are byte offsets in the parsed text and cover the member name.
#[test]
fn spans() {
    let yaml = "server: '{ a: { name: string } }'\n";
    for (text, expected) in [
        ("server.a.nmae", 9..13),
        ("(server).a.nmae", 11..15),
        ("(server).nmae", 9..13),
        ("server[\"nmae\"]", 7..13),
        ("\"é\" + server.nmae", 14..18),
        ("\"é\" + (server).nmae", 16..20),
    ] {
        let found = findings(yaml, text);
        assert_eq!(found.len(), 1, "{text}: {found:?}");
        assert_eq!(found[0].span, expected, "{text}");
        assert_eq!(text[expected].trim_matches('"'), "nmae", "{text}");
    }
}

#[test]
fn diagnostic_fields() {
    let schema = schema(SERVER);
    let declared = DeclaredTypes::new(Some(&schema));
    let text = "server.nmae || fm.server.nmae";
    let expr = parse_spanned(text).unwrap();
    let diagnostics = expression_member_diagnostics(&expr, text, &TypeScope::unguarded(&declared));
    assert_eq!(diagnostics.len(), 1, "the fallback primary handles its absence: {diagnostics:?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.code, UNDECLARED_MEMBER_CODE);
    assert_eq!(diagnostic.code, "dm.expression.undeclared_member");
    assert_eq!(diagnostic.severity, Severity::Warning);
    assert_eq!(diagnostic.message, "Member 'nmae' is not declared by the known object type of 'fm.server'.");
    assert_eq!(diagnostic.span, 25..29);
    assert_eq!(diagnostic.expected_type, None);
    assert_eq!(diagnostic.actual_type, None);
}

/// Findings follow the absence table: only unhandled reads report.
#[test]
fn absence_table() {
    for (text, count) in [
        ("server.nmae", 1),
        ("server.nmae || \"none\"", 0),
        ("server.nmae ? 1 : 0", 0),
        ("is_null(server.nmae)", 0),
        ("min(server.nmae, 5)", 1),
        ("is_empty(trim(server.nmae))", 1),
        ("server.nmae == \"x\" ? 1 : 0", 1),
        ("server.nmae ? server.nmae : \"none\"", 0),
        ("server.nmae ? \"some\" : server.nmae", 1),
    ] {
        assert_eq!(findings(SERVER, text).len(), count, "{text}");
    }
}
