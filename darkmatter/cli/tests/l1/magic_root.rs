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

/// Every route that resolves a file reference, each naming `@doc.md` either
/// as its argument or inside `main.md`, with the text its success prints.
const ROUTES: [(&[&str], &str); 9] = [
    (&["@doc.md"], "FROM CONFIGURED ROOT"),
    (&["compose", "@doc.md"], "FROM CONFIGURED ROOT"),
    (&["compose", "main.md"], "FROM CONFIGURED ROOT"),
    (&["toc", "@doc.md"], "FROM CONFIGURED ROOT"),
    (&["clean", "@doc.md"], "FROM CONFIGURED ROOT"),
    (&["get", "@doc.md", "title"], "Configured"),
    (&["hash", "@doc.md"], "-"),
    (&["graph", "@doc.md"], "doc.md"),
    (&["validate", "refs", "main.md"], "Valid: 1"),
];

#[test]
fn every_route_resolves_at_references_through_a_configured_root() {
    let fixture = fixture("magic_root_routes");
    let mut failures = Vec::new();
    for (args, expected) in ROUTES {
        let configured = fixture
            .command()
            .args(["--magic-root", "roots"])
            .args(args)
            .output()
            .expect("run md");
        let stdout = String::from_utf8_lossy(&configured.stdout);
        if !configured.status.success() || !stdout.contains(expected) {
            failures.push(format!(
                "{args:?} with --magic-root: {} stdout {stdout:?} stderr {:?}",
                configured.status,
                String::from_utf8_lossy(&configured.stderr)
            ));
        }
        // Without the flag `@doc.md` names nothing, so a passing route proves
        // the root came from the flag.
        let unconfigured = fixture.command().args(args).output().expect("run md");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&unconfigured.stdout),
            String::from_utf8_lossy(&unconfigured.stderr)
        );
        if unconfigured.status.success() && combined.contains("FROM CONFIGURED ROOT") {
            failures.push(format!("{args:?} resolved @doc.md without --magic-root"));
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
