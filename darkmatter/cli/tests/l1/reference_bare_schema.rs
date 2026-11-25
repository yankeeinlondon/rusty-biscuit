//! The standalone reference commands resolve a bare `$schema` name from the
//! document's schema roots, as `md compose` does, and still refuse a name no
//! root holds; `--disable schema-triggers` leaves that lookup on.

use crate::common;

use common::CliProcessFixture;

fn review_repository(name: &str, schema: &str) -> (CliProcessFixture, std::path::PathBuf) {
    let fixture = CliProcessFixture::named(name);
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/schemas/review.yaml", "$schema:\n  description: string(required)\n");
    fixture.write_file("cwd/sibling.md", "# Sibling\n");
    let input = fixture.write_file(
        "cwd/review.md",
        &format!("---\n$schema: {schema}\ndescription: Review heading\n---\n\n[sibling](./sibling.md)\n"),
    );
    (fixture, input)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn reference_commands_resolve_a_bare_schema_name() {
    let (fixture, input) = review_repository("reference_bare_schema", "review.yaml");

    let refs = fixture.command().args(["validate", "refs"]).arg(&input).args(["--output", "plain"]).output().unwrap();
    assert_eq!(refs.status.code(), Some(0), "{}{}", text(&refs.stdout), text(&refs.stderr));
    assert_eq!(text(&refs.stdout), "Checked 1 reference across 1 document: all ok\n");

    let graph = fixture.command().arg("graph").arg(&input).args(["--validate", "--output", "plain"]).output().unwrap();
    assert_eq!(graph.status.code(), Some(0), "{}{}", text(&graph.stdout), text(&graph.stderr));
    assert!(text(&graph.stdout).ends_with("Checked 1 reference across 1 document: all ok\n"), "{}", text(&graph.stdout));

    let schema = fixture
        .command()
        .args(["schema", "validate", "--disable", "schema-triggers"])
        .arg(&input)
        .args(["--output", "plain"])
        .output()
        .unwrap();
    assert_eq!(schema.status.code(), Some(0), "{}{}", text(&schema.stdout), text(&schema.stderr));
}

#[test]
fn reference_commands_refuse_a_schema_no_root_holds() {
    let (fixture, input) = review_repository("reference_missing_schema", "absent.yaml");

    for args in [&["validate", "refs"][..], &["graph", "--validate"][..]] {
        let output = fixture.command().args(args).arg(&input).args(["--output", "json"]).output().unwrap();
        let stdout = text(&output.stdout);
        assert_eq!(output.status.code(), Some(1), "{args:?}: {stdout}{}", text(&output.stderr));
        assert!(stdout.contains("absent.yaml"), "{args:?}: {stdout}");
    }
}
