//! The global `--magic-root <DIR>` flag: an extra `@` root registered on the
//! one request snapshot every `md` route resolves against.

use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;

/// A launch directory holding `roots/doc.md` (reachable only through the
/// configured root) and `main.md`, which transcludes it as `@doc.md`.
fn fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file("cwd/roots/doc.md", "---\ntitle: Configured\n---\n## FROM CONFIGURED ROOT\n");
    fixture.write_file("cwd/main.md", "# Main\n\n::file @doc.md\n");
    fixture
}

/// A route's own success: the text it prints when it read `roots/doc.md`.
type Observable = fn(&str, &RouteFacts) -> bool;

/// What the routes' observables compare against.
struct RouteFacts {
    /// `md hash roots/doc.md`, through a plain path.
    hash: String,
}

/// Every route that resolves a file reference, each naming `@doc.md` either
/// as its argument or inside `main.md`, with its own observable.
const ROUTES: [(&[&str], Observable); 14] = [
    (&["@doc.md"], |out, _| out.contains("FROM CONFIGURED ROOT")),
    (&["compose", "@doc.md"], |out, _| out.contains("FROM CONFIGURED ROOT")),
    (&["compose", "main.md"], |out, _| out.contains("FROM CONFIGURED ROOT")),
    (&["toc", "--json", "@doc.md"], |out, _| out.contains("\"title\": \"Configured\"")),
    (&["clean", "@doc.md"], |out, _| out.contains("FROM CONFIGURED ROOT")),
    (&["get", "@doc.md", "title"], |out, _| out.trim() == "\"Configured\""),
    (&["hash", "@doc.md"], |out, facts| out.trim() == facts.hash),
    (&["graph", "--json", "@doc.md"], |out, _| {
        serde_json::from_str::<serde_json::Value>(out).is_ok_and(|graph| {
            graph["source"].as_str().is_some_and(|source| {
                let source = std::path::Path::new(source);
                source.ends_with("doc.md") && source.parent().is_some_and(|parent| parent.ends_with("roots"))
            })
        })
    }),
    (&["validate", "refs", "main.md"], |out, _| out.contains("Valid: 1")),
    (&["validate", "refs", "--graph", "mermaid", "@doc.md"], |out, _| out.contains("roots_doc_md")),
    (&["schema", "validate", "@doc.md"], |out, _| out.contains("roots/doc.md") || out.contains("roots\\doc.md")),
    (&["schema", "detect", "@doc.md"], |out, _| out.contains("title:")),
    (&["code-block", "--file", "@doc.md"], |out, _| out.contains("FROM CONFIGURED ROOT")),
    (&["code-block", "@doc.md"], |out, _| out.contains("FROM CONFIGURED ROOT")),
];

#[test]
fn every_route_resolves_at_references_through_a_configured_root() {
    let fixture = fixture("magic_root_routes");
    let hashed = fixture.command().args(["hash", "roots/doc.md"]).output().expect("run md hash");
    let facts = RouteFacts { hash: String::from_utf8_lossy(&hashed.stdout).trim().to_string() };
    assert!(hashed.status.success() && !facts.hash.is_empty(), "md hash roots/doc.md: {hashed:?}");
    let mut failures = Vec::new();
    for (args, observable) in ROUTES {
        let configured = fixture
            .command()
            .args(["--magic-root", "roots"])
            .args(args)
            .output()
            .expect("run md");
        let stdout = String::from_utf8_lossy(&configured.stdout);
        if !configured.status.success() || !observable(&stdout, &facts) {
            failures.push(format!(
                "{args:?} with --magic-root: {} stdout {stdout:?} stderr {:?}",
                configured.status,
                String::from_utf8_lossy(&configured.stderr)
            ));
        }
        // Without the flag `@doc.md` names nothing: the route must fail
        // (except `code-block` without `--file`, which renders an unresolved
        // value as code) and must not print its own observable, so a passing
        // route proves the root came from the flag.
        let unconfigured = fixture.command().args(args).output().expect("run md");
        let stdout = String::from_utf8_lossy(&unconfigured.stdout);
        let literal_fallback = args == ["code-block", "@doc.md"];
        if (unconfigured.status.success() && !literal_fallback) || observable(&stdout, &facts) {
            failures.push(format!(
                "{args:?} without --magic-root: {} stdout {stdout:?} stderr {:?}",
                unconfigured.status,
                String::from_utf8_lossy(&unconfigured.stderr)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn magic_roots_are_searched_in_the_order_given() {
    let fixture = fixture("magic_root_order");
    fixture.write_file("cwd/later/doc.md", "## FROM LATER ROOT\n");
    fixture
        .command()
        .args(["--magic-root", "later", "--magic-root", "roots", "compose", "@doc.md"])
        .assert()
        .success()
        .stdout(predicate::str::contains("FROM LATER ROOT"))
        .stdout(predicate::str::contains("FROM CONFIGURED ROOT").not());
}

#[test]
fn a_magic_root_that_is_not_a_directory_is_rejected() {
    let fixture = fixture("magic_root_missing");
    for root in ["missing", "main.md"] {
        fixture
            .command()
            .args(["--magic-root", root, "compose", "main.md"])
            .assert()
            .failure()
            .stderr(predicate::str::contains(format!("--magic-root {root}:")))
            .stderr(predicate::str::contains("is not a directory"));
    }
}

#[test]
fn help_documents_the_magic_root_flag() {
    let fixture = fixture("magic_root_help");
    fixture
        .command()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--magic-root <DIR>"));
}
