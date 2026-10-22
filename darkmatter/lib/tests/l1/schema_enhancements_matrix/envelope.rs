//! Reader (a): the YAML SimplifiedSchema standalone envelope, walked from two
//! shipped schemas rather than hand-authored seeds. The seeds in
//! `schema_yaml_spellings` already pin these shapes against the classifier;
//! these cells add the real writers' presentation (four-space indentation,
//! quoted descriptions, interleaved comments) and assert only what a caller
//! sees: a schema error, or a schema that validates a sample document.
//!
//! Cells whose outcome waits on a pending ruling are absent: a null named
//! type (R5, today the documented `null` type) and an absent `kind` (R5).

use super::validate_document;
use crate::schema_yaml_spellings::{Edit, Edit::*, apply};
use darkmatter::markdown::schemas::SchemaError;

/// The public outcome of one cell.
#[derive(Clone, Copy)]
enum Outcome {
    /// The schema loads; each frontmatter instance validates as stated.
    Validates(&'static [(&'static str, bool)]),
    /// A recognized envelope with a malformed payload: `SchemaDocument`.
    Rejects,
    /// Loading fails and nothing usable is produced. Used where the reported
    /// class is known to be wrong today (duplicate keys and trailing documents
    /// surface as `AmbiguousReferenced`); the Wave 4 Schema adapter and Wave 2
    /// Grammar tasks own the class.
    Fails,
}

use Outcome::*;

struct Cell {
    label: &'static str,
    edit: Edit,
    outcome: Outcome,
}

const fn cell(label: &'static str, edit: Edit, outcome: Outcome) -> Cell {
    Cell { label, edit, outcome }
}

/// Walks `cells` over `seed`, one schema file per cell so a path-keyed
/// validator cache never serves an earlier cell.
fn walk(seed_name: &str, seed: &str, cells: &[Cell]) {
    let workspace = tempfile::tempdir().expect("matrix workspace");
    let dir = workspace.path();
    let mut failures = Vec::new();
    for (index, cell) in cells.iter().enumerate() {
        let source = apply(seed, cell.edit);
        if !matches!(cell.edit, Control) {
            assert_ne!(source, seed, "{seed_name}/{}: the edit must change the seed", cell.label);
        }
        let file_name = format!("{seed_name}-{index}.yaml");
        std::fs::write(dir.join(&file_name), &source).expect("write cell schema");
        let reference = format!("$schema: ./{file_name}");
        let problem = match cell.outcome {
            Validates(instances) => instances.iter().find_map(|(instance, expected)| {
                match validate_document(dir, &format!("{reference}\n{instance}")) {
                    Ok(report) if report.valid == *expected => None,
                    Ok(report) => Some(format!(
                        "{instance:?}: expected valid={expected}, problems {:?}",
                        report.problems
                    )),
                    Err(error) => Some(format!("{instance:?}: schema failed to load: {error}")),
                }
            }),
            Rejects => match validate_document(dir, &reference) {
                Err(SchemaError::SchemaDocument { .. }) => None,
                other => Some(format!("expected SchemaDocument, got {other:?}")),
            },
            Fails => validate_document(dir, &reference)
                .ok()
                .map(|report| format!("expected a load error, got a report {report:?}")),
        };
        if let Some(problem) = problem {
            failures.push(format!("{seed_name}/{}: {problem}\n--- source ---\n{source}", cell.label));
        }
    }
    assert!(failures.is_empty(), "{} cell(s) failed:\n\n{}", failures.len(), failures.join("\n\n"));
}

const TIMING_CONTROL: &[(&str, bool)] = &[
    ("timing:\n  document_ms: 5", true),
    ("timing:\n  document_ms: -1", false),
    ("timing:\n  document_ms: word", false),
];

/// `darkmatter/schemas/partials/timing.yaml`: the tagged envelope's `types`
/// map and its one named type `timing`.
const TAGGED_CELLS: &[Cell] = &[
    cell("control", Control, Validates(TIMING_CONTROL)),
    // `types`
    cell("types absent", Remove("types"), Rejects),
    cell("types null", Set("types", " null"), Rejects),
    cell("types number", Set("types", " 5"), Rejects),
    cell("types list", Set("types", " [timing]"), Rejects),
    cell("types one wrong element", Replace("    timing:\n", "    bad: 5\n    timing:\n"), Rejects),
    cell("types every element wrong", Set("types", "\n    first: 5\n    second: true"), Rejects),
    cell(
        "types empty mapping",
        Set("types", " {}"),
        Validates(&[("timing:\n  document_ms: -1", true)]),
    ),
    cell("types duplicate", Duplicate("types"), Fails),
    cell("second document", Append("---\nkind: schema\ntypes: {}\n"), Fails),
    cell("trailing garbage", Append("]\n"), Fails),
    // named type `timing`
    cell("timing number", Set("timing", " 5"), Rejects),
    cell("timing bool", Set("timing", " true"), Rejects),
    cell("timing one wrong union arm", Set("timing", " [object, 5]"), Rejects),
    cell("timing every union arm wrong", Set("timing", " [5]"), Rejects),
    cell("timing empty union", Set("timing", " []"), Rejects),
    cell("timing empty string", Set("timing", " \"\""), Rejects),
    cell(
        "timing invalid member constraint",
        Replace("number(integer; min(0); generated)", "number(integer; bogus(0); generated)"),
        Rejects,
    ),
    cell("timing duplicate", Duplicate("timing"), Fails),
];

const SLASH_VALID: &str = "last_updated: 2026-10-21\nagent: a\nmodel: m\nsupport: partial\nrequires_claudine_update: false";
const SLASH_BAD_SUPPORT: &str = "last_updated: 2026-10-21\nagent: a\nmodel: m\nsupport: bogus\nrequires_claudine_update: false";

/// `claudine/docs/research/slash-commands/_schema.yaml`: the pure envelope's
/// root `$schema` mapping.
const PURE_CELLS: &[Cell] = &[
    cell("control", Control, Validates(&[(SLASH_VALID, true), (SLASH_BAD_SUPPORT, false)])),
    // Removing the sole key leaves only comments: the requested schema has no
    // usable reading.
    cell("$schema absent", Remove("$schema"), Fails),
    cell("$schema null", Set("$schema", " null"), Rejects),
    cell("$schema number", Set("$schema", " 5"), Rejects),
    cell("$schema bool", Set("$schema", " true"), Rejects),
    cell("$schema one wrong element", Set("$schema", " [./a.yaml, 5]"), Rejects),
    cell("$schema every element wrong", Set("$schema", " [5]"), Rejects),
    cell("$schema empty mapping", Set("$schema", " {}"), Validates(&[(SLASH_BAD_SUPPORT, true)])),
    cell("$schema empty list", Set("$schema", " []"), Rejects),
    cell("$schema empty reference", Set("$schema", " \"\""), Rejects),
    cell("$schema duplicate", Duplicate("$schema"), Fails),
    cell("$schema second document", Append("---\n$schema: {}\n"), Fails),
    cell("$schema trailing garbage", Append("]\n"), Fails),
    cell("member invalid constraint", Replace("date(required)", "date(bogus)"), Rejects),
];

#[test]
fn shipped_tagged_envelope_matrix() {
    let seed = include_str!("../../fixtures/schema_enhancements_matrix/timing.shipped.yaml");
    walk("timing", seed, TAGGED_CELLS);
}

#[test]
fn shipped_pure_envelope_matrix() {
    let seed = include_str!("../../fixtures/schema_enhancements_matrix/slash-commands.shipped.yaml");
    walk("slash-commands", seed, PURE_CELLS);
}
