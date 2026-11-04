//! The shipped `model-config` research documents against their sidecar.
//!
//! The sidecar declares `notes: string[]` on its three record types
//! (`model_config_paths[]`, `api_standards[]`, `local_runners[]`). Documents
//! once shipped scalar notes, which the generator rejects as
//! `ResearchInvalid`; these tests keep the documents and the contract in step
//! through the generator's own validation path.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use claudine_gen::GenError;
use claudine_gen::inputs::{area_resolution_context, load_validated_frontmatter};
use darkmatter::markdown::compose::RequestSnapshot;

fn validate(area: &Path, document: &Path) -> Result<serde_json::Value, GenError> {
    let context = area_resolution_context(area, &RequestSnapshot::new(area)).expect("area resolution context builds");
    load_validated_frontmatter(document, &context)
}

#[test]
fn every_shipped_model_config_document_satisfies_its_sidecar() {
    let area = biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("gen crate lives under the claudine package area")
        .to_path_buf();
    let documents = [
        area.join("docs/research/model-config/antigravity.md"),
        area.join("docs/research/model-config/claude.md"),
        area.join("docs/research/model-config/codex.md"),
        area.join("docs/research/model-config/gemini.md"),
        area.join("docs/research/model-config/goose.md"),
        area.join("docs/research/model-config/kilo.md"),
        area.join("docs/research/model-config/kimi.md"),
        area.join("docs/research/model-config/opencode.md"),
        area.join("docs/research/model-config/pi.md"),
        area.join("docs/research/model-config/qwen.md"),
    ];

    // A provider document added later must be listed above, or it would go
    // unvalidated here.
    let on_disk: BTreeSet<String> = fs::read_dir(area.join("docs/research/model-config"))
        .expect("read the model-config topic directory")
        .map(|entry| entry.expect("directory entry").file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".md") && !name.starts_with('_'))
        .collect();
    let listed: BTreeSet<String> = documents
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(listed, on_disk, "the test's document list drifted from the topic directory");

    let mut failures = Vec::new();
    for document in &documents {
        match validate(&area, document) {
            Ok(frontmatter) => {
                for record in ["model_config_paths", "api_standards", "local_runners"] {
                    for (index, entry) in frontmatter[record].as_array().into_iter().flatten().enumerate() {
                        if let Some(notes) = entry.get("notes") {
                            assert!(
                                notes.is_array(),
                                "{}: {record}[{index}].notes is not a list after validation: {notes}",
                                document.display()
                            );
                        }
                    }
                }
            }
            Err(err) => failures.push(err.to_string()),
        }
    }
    assert!(failures.is_empty(), "shipped model-config documents fail their sidecar:\n{}", failures.join("\n"));
}

/// Returns `document` with the first `notes` list under `record:` written back
/// as the scalar it holds (`notes:\n  - 'text'` → `notes: 'text'`).
fn unwrap_first_notes(document: &str, record: &str) -> String {
    let lines: Vec<&str> = document.lines().collect();
    let section = lines
        .iter()
        .position(|line| *line == format!("{record}:"))
        .unwrap_or_else(|| panic!("document has a `{record}:` section"));
    let notes = (section + 1..lines.len())
        .find(|&index| lines[index].trim() == "notes:")
        .unwrap_or_else(|| panic!("`{record}` has a record with notes"));
    let item = lines[notes + 1].trim_start();
    let text = item.strip_prefix("- ").expect("notes holds a one-item list");
    let mut rewritten: Vec<String> = lines.iter().map(|line| (*line).to_string()).collect();
    rewritten[notes] = format!("{} {text}", lines[notes]);
    rewritten.remove(notes + 1);
    rewritten.join("\n") + "\n"
}

#[test]
fn a_scalar_notes_is_rejected_in_every_record_type() {
    let real_area = biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("gen crate lives under the claudine package area")
        .to_path_buf();
    let sidecar = real_area.join("docs/research/model-config/_schema.yaml");
    let shipped = include_str!("../../../docs/research/model-config/claude.md");

    let dir = tempfile::tempdir().expect("tempdir");
    let topic = dir.path().join("docs/research/model-config");
    fs::create_dir_all(&topic).unwrap();
    fs::copy(&sidecar, topic.join("_schema.yaml")).expect("copy the real sidecar");

    // Control: the shipped document validates in this synthetic area, so each
    // rejection below is caused by its one scalar.
    let control = topic.join("claude.md");
    fs::write(&control, shipped).unwrap();
    validate(dir.path(), &control).expect("the shipped document validates in the synthetic area");

    for record in ["model_config_paths", "api_standards", "local_runners"] {
        let document = topic.join(format!("scalar-{record}.md"));
        fs::write(&document, unwrap_first_notes(shipped, record)).unwrap();
        match validate(dir.path(), &document) {
            Err(GenError::ResearchInvalid { problems, .. }) => {
                let expected = format!("/{record}/0/notes:");
                assert!(
                    problems.lines().any(|line| line.contains(&expected) && line.contains("is not of type \"array\"")),
                    "{record}: expected a non-array problem at {expected}, got:\n{problems}"
                );
                assert_eq!(problems.lines().count(), 1, "{record}: only the scalar is a problem, got:\n{problems}");
            }
            other => panic!("{record}: a scalar notes must be ResearchInvalid, got: {other:?}"),
        }
    }
}

/// The schema/catalog gate reads the sidecar's record types from its own
/// `types:` mapping (`local_runners: runner[]`), so `offering_sources` sees the
/// record fields rather than an opaque `runner[]@this` reference.
#[test]
fn the_gate_inlines_record_types_from_the_sidecar_types_mapping() {
    let real_area = biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("gen crate lives under the claudine package area")
        .to_path_buf();
    let sidecar = real_area.join("docs/research/model-config/_schema.yaml");
    let schema = claudine_gen::schema_compat::load_sidecar_schema(&sidecar).expect("the sidecar loads");
    let simplified_schema::SchemaRoot::Single(shape) = schema else {
        panic!("the sidecar is one schema");
    };
    for (record, fields) in [
        ("model_config_paths", ["scope", "path", "notes"]),
        ("api_standards", ["standard", "base_url_site", "notes"]),
        ("local_runners", ["runner", "integration", "notes"]),
    ] {
        let Some(simplified_schema::PropertyDef::Single(atom)) = shape.properties.get(record) else {
            panic!("{record} is a single property");
        };
        assert!(atom.is_array, "{record} stays a list");
        let simplified_schema::TypeExpr::InlineObject(object) = &atom.ty else {
            panic!("{record} was not inlined: {:?}", atom.ty);
        };
        for field in fields {
            assert!(object.properties.contains_key(field), "{record} lacks `{field}`");
        }
    }

    // A bare name the sidecar does not declare is a sidecar error, not an
    // opaque reference the gate later reports as an incompatible shape.
    let dir = tempfile::tempdir().expect("tempdir");
    let broken = dir.path().join("_schema.yaml");
    fs::write(&broken, "$schema:\n    local_runners: runner[]\ntypes:\n    env_var:\n        name: string\n").unwrap();
    match claudine_gen::schema_compat::load_sidecar_schema(&broken) {
        Err(GenError::SidecarInvalid { message, .. }) => assert!(message.contains("runner"), "{message}"),
        other => panic!("an undeclared local type must be SidecarInvalid, got: {other:?}"),
    }
}
