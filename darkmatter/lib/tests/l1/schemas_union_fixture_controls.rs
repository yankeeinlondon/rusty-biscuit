//! Controls for the shared union fixtures (`tests/union_fixtures`): each
//! fixture loads and validates as an ordinary property union, so a later
//! failure through `U[]` or `U(required)` is known to come from the new form
//! and not from the fixture.

use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{DarkmatterSchemas, analyze_frontmatter};

use crate::request_support::context_at;
use crate::union_fixtures::{
    CONTENT_POLICY_FILE, FILE_TYPES, MULTI_HOP_FILES, OVERLAPPING_TYPES, REPAIRABLE_SIBLING,
    TAGGED_TYPES, content_policy_bare_schema, content_policy_union_schema, write_files,
};

/// Validates a document in `dir` whose `$schema` mapping body is `schema`,
/// loading it from disk the way `md schema validate` does (an unparsable
/// frontmatter block is an error here, never an absent one).
fn validates(dir: &Path, schema: &str, frontmatter: &str) -> bool {
    let path = dir.join("doc.md");
    std::fs::write(&path, format!("---\n$schema:\n{schema}{frontmatter}---\nBody\n"))
        .expect("write document");
    let markdown = Markdown::try_from(path.as_path())
        .expect("document loads")
        .with_source(ComposeSource::File(path));
    DarkmatterSchemas::new(context_at(dir))
        .validate(&markdown)
        .expect("schema resolves")
        .valid
}

fn temp_root() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    (dir, root)
}

#[test]
fn content_policy_fixture_edits_only_the_three_reference_lines() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let bare = content_policy_bare_schema(shipped);
    let union = content_policy_union_schema(shipped);

    let changed = |edited: &str| -> Vec<(String, String)> {
        assert_eq!(edited.lines().count(), shipped.lines().count());
        shipped
            .lines()
            .zip(edited.lines())
            .filter(|(before, after)| before != after)
            .map(|(before, after)| (before.to_string(), after.to_string()))
            .collect()
    };
    assert_eq!(
        changed(&bare),
        [
            ("    rule: \"short_form@./content-policy.yaml\"", "    rule: short_form"),
            ("    - \"short_form@./content-policy.yaml\"", "    - short_form"),
            ("    - \"long_form@./content-policy.yaml\"", "    - long_form"),
        ]
        .map(|(before, after)| (before.to_string(), after.to_string()))
    );
    assert_eq!(
        changed(&union)[0],
        (
            "    rule: \"short_form@./content-policy.yaml\"".to_string(),
            "    rule: short_form(required)".to_string()
        )
    );
}

#[test]
fn bare_content_policy_fixture_validates_each_entry_as_a_policy_property() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_dir, root) = temp_root();
    write_files(&root, &[(CONTENT_POLICY_FILE, &content_policy_bare_schema(shipped))]);
    let schema = "  p: \"policy@./content-policy.yaml\"\n";

    assert!(validates(&root, schema, "p: \"ValidFor(3mo)\"\n"));
    assert!(validates(&root, schema, "p: {rule: \"ValidUntil(2027-01-01)\", action: archive}\n"));
    assert!(!validates(&root, schema, "p: \"Duration(3mo)\"\n"), "no arm matches");
    assert!(
        !validates(&root, schema, "p: {rule: \"ValidFor(3mo)\", action: delete}\n"),
        "`delete` is not an action"
    );
    // `rule` is optional until the acceptance fixture makes it required.
    assert!(validates(&root, schema, "p: {action: archive}\n"));
}

#[test]
fn union_fixtures_validate_as_property_unions() {
    let (_dir, root) = temp_root();
    write_files(
        &root,
        &[
            ("tagged.yaml", TAGGED_TYPES),
            ("overlapping.yaml", OVERLAPPING_TYPES),
            ("files.yaml", FILE_TYPES),
            ("present.md", "present\n"),
        ],
    );
    write_files(&root, MULTI_HOP_FILES);

    let tagged = "  step: \"step@./tagged.yaml\"\n";
    assert!(validates(&root, tagged, "step: {kind: shell, run: ls}\n"));
    assert!(validates(&root, tagged, "step: {kind: say, text: hi}\n"));
    assert!(!validates(&root, tagged, "step: {kind: shell, text: hi}\n"), "the tag selects the shell arm");

    let overlapping = "  entry: \"entry@./overlapping.yaml\"\n";
    assert!(validates(&root, overlapping, "entry: {name: x}\n"), "both arms match");
    assert!(validates(&root, overlapping, "entry: {name: x, size: 2}\n"));
    assert!(!validates(&root, overlapping, "entry: {size: 2}\n"));

    let files = "  asset: \"asset@./files.yaml\"\n  lazy: \"lazy_asset@./files.yaml\"\n";
    assert!(validates(&root, files, "asset: ./present.md\nlazy: 3\n"));

    let multi_hop = "  choice: \"choice@./outer.yaml\"\n";
    assert!(validates(&root, multi_hop, "choice: abc\n"));
    assert!(validates(&root, multi_hop, "choice: 3\n"));
    assert!(!validates(&root, multi_hop, "choice: ABC\n"), "the middle file's `word` pattern applies");

}

/// On its own, the sibling is the sole raw problem, so clean's proven
/// quoting repair applies to it.
#[test]
fn repairable_sibling_is_quoted_by_clean_analysis() {
    let (_dir, root) = temp_root();
    let (sibling_schema, sibling_value) = REPAIRABLE_SIBLING;
    let body = format!("$schema:\n  {sibling_schema}\n{sibling_value}");
    let markdown = Markdown::try_from_content(format!("---\n{body}---\nBody\n")).expect("document loads");
    let effective = DarkmatterSchemas::new(context_at(&root))
        .effective_for(&markdown)
        .expect("schema resolves")
        .expect("the document declares a schema");
    let repairs: Vec<biscuit_file::YamlRepair> = analyze_frontmatter(&effective, &body)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.classification.is_auto_apply_eligible())
        .flat_map(|diagnostic| diagnostic.repairs.iter().cloned())
        .collect();
    assert_eq!(
        biscuit_file::apply_edit_set(&body, &repairs).source,
        body.replace("label: 1.20", "label: \"1.20\""),
    );
}
