//! Standalone SimplifiedSchema authoring input: the YAML robustness matrix and
//! the paired presentation fixtures under `tests/fixtures/schema_yaml_spellings/`.
//!
//! The matrix walks every load-bearing envelope and payload field from an
//! unedited seed, applies exactly one edit per cell, and asserts the public
//! result: classification by [`parse_standalone_schema_document`], loading
//! through [`resolve_schema_with_roots`], and — for accepted cells —
//! validation through [`DarkmatterSchemas::effective_for`] of a document that
//! references the edited schema file. A control row per seed proves each
//! declaration under test changes validation, so every edit is load-bearing.
//!
//! The seeds are the conventional (indented, single-line) presentations. Each
//! has a `serialized` twin that is byte-for-byte `serde_yaml_ng` output for the
//! same value (see `serializer_captures_are_serde_yaml_ng_output`), which
//! writes block sequences indentless. Edits address keys and items without
//! their indentation so the same matrix can run against either presentation.

use std::path::Path;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::{
    Markdown,
    compose::ComposeSource,
    schemas::{
        DarkmatterSchemas, SchemaError, SchemaSourcePath, SchemaSpanKind, StandaloneSchemaEnvelope,
        parse_standalone_schema_document,
        parse_yaml_schema, resolve::resolve_schema_with_roots, simplified::to_json_schema,
    },
};
use serde_json::json;

/// One edit applied to a seed. Keys are matched at the start of a line after
/// indentation (`{key}:`), first occurrence; items and lines by exact text.
#[derive(Clone, Copy)]
enum Edit {
    /// The unedited seed.
    Control,
    /// Replaces everything after `{key}:` on its line, plus the key's block
    /// (more-indented lines and same-indent `- ` items), with the raw text.
    Set(&'static str, &'static str),
    /// Removes the key's line and block.
    Remove(&'static str),
    /// Repeats the key's line and block immediately after itself.
    Duplicate(&'static str),
    /// Replaces the first occurrence of exact text.
    Replace(&'static str, &'static str),
    /// Repeats the first physical line containing the text.
    DuplicateLine(&'static str),
    /// Removes the first physical line containing the text.
    RemoveLine(&'static str),
    /// Appends raw text to the end of the document.
    Append(&'static str),
}

/// The defined outcome of one cell.
#[derive(Clone, Copy)]
enum Outcome {
    /// The classifier declines (`Ok(None)`) and no SimplifiedSchema is applied:
    /// valid YAML loads as raw JSON Schema, invalid YAML is rejected.
    Declines,
    /// A recognized envelope with a malformed payload: `SchemaDocument`, and
    /// normal loading fails.
    Rejects,
    /// Accepted; each instance (frontmatter YAML) validates as stated.
    Accepts(&'static [(&'static str, bool)]),
}

use Edit::*;
use Outcome::*;

struct Cell {
    label: &'static str,
    edit: Edit,
    outcome: Outcome,
}

const fn cell(label: &'static str, edit: Edit, outcome: Outcome) -> Cell {
    Cell { label, edit, outcome }
}

const TAGGED_CONTROL: &[(&str, bool)] = &[
    ("title: abcd", true),
    ("title: ab", false),
    ("status: 5", true),
    ("status: true", true),
    ("status:\n  code: 2", true),
    ("status:\n  code: 0", false),
    ("status: word", false),
    ("next: true", true),
    ("next: [1]", false),
];

/// Tagged envelope: `kind`, `types`, the named definition `title`, and the
/// property union `status` (string and mapping arms).
const TAGGED_CELLS: &[Cell] = &[
    cell("control", Control, Accepts(TAGGED_CONTROL)),
    // `kind`
    cell("kind absent", Remove("kind"), Declines),
    cell("kind null", Set("kind", " null"), Declines),
    cell("kind empty value", Set("kind", ""), Declines),
    cell("kind number", Set("kind", " 5"), Declines),
    cell("kind list", Set("kind", " [schema]"), Declines),
    cell("kind empty string", Set("kind", " \"\""), Declines),
    cell("kind empty list", Set("kind", " []"), Declines),
    cell("kind empty mapping", Set("kind", " {}"), Declines),
    cell("kind duplicate", Duplicate("kind"), Declines),
    cell("kind second document", Append("---\nkind: schema\n"), Declines),
    cell("kind trailing garbage", Append("]\n"), Declines),
    // `types`
    cell("types absent", Remove("types"), Rejects),
    cell("types null", Set("types", " null"), Rejects),
    cell("types empty value", Set("types", ""), Rejects),
    cell("types number", Set("types", " 5"), Rejects),
    cell("types flow list", Set("types", " [string, number]"), Rejects),
    cell("types indentless list", Set("types", "\n- string\n- number"), Rejects),
    cell("types one wrong element", Set("types", "\n  - string\n  - 5"), Rejects),
    cell("types every element wrong", Set("types", "\n  - 5"), Rejects),
    cell(
        "types empty mapping",
        Set("types", " {}"),
        Accepts(&[("title: ab", true), ("status: word", true)]),
    ),
    cell("types empty list", Set("types", " []"), Rejects),
    cell("types duplicate", Duplicate("types"), Declines),
    cell("types second document", Append("---\ntypes: {}\n"), Declines),
    cell("tagged extra root key", Append("extra: 1\n"), Rejects),
    // The unsupported-key message lists only string keys; the key count must
    // still reject a non-string one.
    cell("tagged extra non-string root key", Append("1: x\n"), Rejects),
    // named definition `title`
    cell(
        "title absent",
        Remove("title"),
        Accepts(&[("title: ab", true), ("status: word", false)]),
    ),
    cell("title null", Set("title", " null"), Rejects),
    cell("title empty value", Set("title", ""), Rejects),
    cell("title number", Set("title", " 5"), Rejects),
    cell("title bool", Set("title", " true"), Rejects),
    cell("title one wrong element", Set("title", " [string, 5]"), Rejects),
    cell("title every element wrong", Set("title", " [5]"), Rejects),
    cell(
        "title empty mapping",
        Set("title", " {}"),
        Accepts(&[("title: {}", true), ("title: abcd", false)]),
    ),
    cell("title empty list", Set("title", " []"), Rejects),
    cell("title empty string", Set("title", " \"\""), Rejects),
    cell("title duplicate", Duplicate("title"), Declines),
    cell("title invalid expression", Set("title", " string(bogus(2))"), Rejects),
    cell("title unbalanced expression", Set("title", " string(min(3)"), Rejects),
    cell("title invalid YAML", Set("title", " [string"), Declines),
    // property-union members of `status`
    cell(
        "status scalar arm absent",
        RemoveLine("- boolean"),
        Accepts(&[("status: 5", true), ("status: true", false), ("status:\n  code: 2", true)]),
    ),
    cell(
        "status mapping arm absent",
        RemoveLine("- code: number(min(1))"),
        Accepts(&[("status: true", true), ("status:\n  code: 2", false)]),
    ),
    cell("status null arm", Replace("- boolean", "- null"), Rejects),
    cell("status empty arm", Replace("- boolean", "-"), Rejects),
    cell("status number", Set("status", " 5"), Rejects),
    cell("status one wrong arm", Replace("- boolean", "- 5"), Rejects),
    cell("status every arm wrong", Set("status", " [5, true]"), Rejects),
    cell("status empty union", Set("status", " []"), Rejects),
    cell(
        "status duplicate key in mapping arm",
        Replace("- code: number(min(1))", "- {code: number(min(1)), code: number}"),
        Declines,
    ),
    // Repeating a scalar arm is not a duplicate key.
    cell(
        "status repeated scalar arm",
        DuplicateLine("- boolean"),
        Accepts(&[("status: true", true), ("status: word", false)]),
    ),
    cell("status invalid arm grammar", Replace("- boolean", "- boolean(bogus(1))"), Rejects),
    cell("status invalid arm YAML", Replace("- boolean", "- [boolean"), Declines),
];

const NESTED_CONTROL: &[(&str, bool)] = &[
    ("meta:\n  level: 3", true),
    ("meta:\n  level: 9", false),
    ("meta:\n  mode: true", true),
    ("meta:\n  mode: word", false),
    ("next: true", true),
];

/// Pure envelope with a mapping payload: `$schema` and the nested definition
/// `meta.level`.
const NESTED_CELLS: &[Cell] = &[
    cell("control", Control, Accepts(NESTED_CONTROL)),
    // `$schema`
    // Removing the sole key leaves an empty document.
    cell("$schema absent", Remove("$schema"), Declines),
    cell("$schema absent beside another key", Replace("$schema:", "schema:"), Declines),
    cell("$schema null", Set("$schema", " null"), Rejects),
    cell("$schema empty value", Set("$schema", ""), Rejects),
    cell("$schema number", Set("$schema", " 5"), Rejects),
    cell("$schema bool", Set("$schema", " true"), Rejects),
    cell("$schema one wrong element", Set("$schema", " [./a.yaml, 5]"), Rejects),
    cell("$schema every element wrong", Set("$schema", " [5]"), Rejects),
    cell(
        "$schema empty mapping",
        Set("$schema", " {}"),
        Accepts(&[("meta:\n  level: 9", true)]),
    ),
    cell("$schema empty list", Set("$schema", " []"), Rejects),
    cell("$schema empty reference", Set("$schema", " \"\""), Rejects),
    cell("$schema duplicate", Duplicate("$schema"), Declines),
    cell("$schema second document", Append("---\n$schema: {}\n"), Declines),
    cell("$schema trailing garbage", Append("]\n"), Declines),
    cell("pure extra root key", Append("extra: 1\n"), Declines),
    // nested definition `meta.level`
    cell(
        "level absent",
        Remove("level"),
        // Nested mapping shapes are closed: the removed definition becomes an
        // unknown key rather than an implicit empty definition.
        Accepts(&[("meta:\n  level: 3", false), ("meta:\n  mode: word", false)]),
    ),
    cell("level null", Set("level", " null"), Rejects),
    cell("level empty value", Set("level", ""), Rejects),
    cell("level number", Set("level", " 5"), Rejects),
    cell("level bool", Set("level", " true"), Rejects),
    cell("level one wrong element", Set("level", " [number, 5]"), Rejects),
    cell("level every element wrong", Set("level", " [5]"), Rejects),
    cell(
        "level empty mapping",
        Set("level", " {}"),
        Accepts(&[("meta:\n  level: {}", true), ("meta:\n  level: 3", false)]),
    ),
    cell("level empty list", Set("level", " []"), Rejects),
    cell("level empty string", Set("level", " \"\""), Rejects),
    cell("level duplicate", Duplicate("level"), Declines),
    cell("level invalid expression", Set("level", " number(bogus(1))"), Rejects),
    cell("level invalid YAML", Set("level", " [number"), Declines),
];

const ROOT_UNION_CONTROL: &[(&str, bool)] = &[
    ("title: x", true),
    ("count: 1", true),
    ("count: word", false),
    ("other: 1", false),
];

/// Pure envelope with a root-union payload of compact mapping arms.
const ROOT_UNION_CELLS: &[Cell] = &[
    cell("control", Control, Accepts(ROOT_UNION_CONTROL)),
    cell(
        "arm absent",
        RemoveLine("- count: number(required)"),
        Accepts(&[("title: x", true), ("count: 1", false)]),
    ),
    cell("null arm", Replace("- count: number(required)", "- null"), Rejects),
    cell("empty arm", Replace("- count: number(required)", "-"), Rejects),
    cell("number payload", Set("$schema", " 5"), Rejects),
    cell("one wrong arm", Replace("- count: number(required)", "- 5"), Rejects),
    cell("every arm wrong", Set("$schema", " [5, 6]"), Rejects),
    cell("empty union", Set("$schema", " []"), Rejects),
    cell(
        "duplicate key in mapping arm",
        Replace("- count: number(required)", "- {count: number(required), count: number}"),
        Declines,
    ),
    cell(
        "repeated arm",
        DuplicateLine("- count: number(required)"),
        Accepts(&[("count: 1", true), ("other: 1", false)]),
    ),
    cell(
        "invalid arm grammar",
        Replace("- count: number(required)", "- count: number(bogus)"),
        Rejects,
    ),
    cell(
        "remote reference arm",
        Replace("- count: number(required)", "- https://example.com/schema.yaml"),
        Rejects,
    ),
    cell("invalid arm YAML", Replace("- count: number(required)", "- [count"), Declines),
];

/// The line range of `{key}:` (first occurrence at a line start after
/// indentation) and its block: following lines indented deeper than the key,
/// plus same-indent `- ` items (an indentless sequence value).
fn key_block(text: &str, key: &str) -> (usize, usize, usize) {
    let needle = format!("{key}:");
    let mut offset = 0;
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    for (index, line) in lines.iter().enumerate() {
        let indent = line.len() - line.trim_start().len();
        if line[indent..].starts_with(&needle) {
            let key_end = offset + indent + needle.len();
            let mut end = offset + line.len();
            for next in &lines[index + 1..] {
                let next_indent = next.len() - next.trim_start().len();
                let content = next.trim();
                let deeper = next_indent > indent;
                let indentless_item = next_indent == indent && content.starts_with("- ");
                if content.is_empty() || deeper || indentless_item {
                    end += next.len();
                } else {
                    break;
                }
            }
            return (offset, key_end, end);
        }
        offset += line.len();
    }
    panic!("key `{key}` not found in seed:\n{text}");
}

/// The byte range of the first physical line (with its line ending) that
/// contains `find`.
fn line_of(seed: &str, find: &str) -> (usize, usize) {
    let at = seed.find(find).unwrap_or_else(|| panic!("`{find}` not found in seed:\n{seed}"));
    let start = seed[..at].rfind('\n').map_or(0, |newline| newline + 1);
    let end = seed[at..].find('\n').map_or(seed.len(), |newline| at + newline + 1);
    (start, end)
}

fn apply(seed: &str, edit: Edit) -> String {
    match edit {
        Control => seed.to_string(),
        Set(key, raw) => {
            let (_, key_end, end) = key_block(seed, key);
            let newline = if seed[..end].ends_with('\n') { "\n" } else { "" };
            format!("{}{raw}{newline}{}", &seed[..key_end], &seed[end..])
        }
        Remove(key) => {
            let (start, _, end) = key_block(seed, key);
            format!("{}{}", &seed[..start], &seed[end..])
        }
        Duplicate(key) => {
            let (start, _, end) = key_block(seed, key);
            format!("{}{}", &seed[..end], &seed[start..])
        }
        Replace(find, replacement) => {
            assert!(seed.contains(find), "`{find}` not found in seed:\n{seed}");
            seed.replacen(find, replacement, 1)
        }
        DuplicateLine(find) => {
            let (start, end) = line_of(seed, find);
            format!("{}{}", &seed[..end], &seed[start..])
        }
        RemoveLine(find) => {
            let (start, end) = line_of(seed, find);
            format!("{}{}", &seed[..start], &seed[end..])
        }
        Append(raw) => format!("{seed}{raw}"),
    }
}

/// Walks one seed's cells. Every cell gets its own schema file name so a
/// path-keyed schema cache can never serve an earlier cell's contents.
fn walk_matrix(seed_name: &str, seed: &str, envelope: StandaloneSchemaEnvelope, cells: &[Cell]) {
    let workspace = tempfile::tempdir().expect("matrix workspace");
    let dir = workspace.path();
    let context = FileResolutionContext::new(dir);
    let api = DarkmatterSchemas::new(context.clone());
    let mut failures = Vec::new();

    for (index, cell) in cells.iter().enumerate() {
        let source = apply(seed, cell.edit);
        if !matches!(cell.edit, Control) {
            assert_ne!(source, seed, "{seed_name}/{}: the edit must change the seed", cell.label);
        }
        let file_name = format!("{seed_name}-{index}.yaml");
        let schema_path = dir.join(&file_name);
        std::fs::write(&schema_path, &source).expect("write cell schema");
        if let Err(problem) = check_cell(&api, &context, dir, &file_name, &source, envelope, cell) {
            failures.push(format!("{seed_name}/{}: {problem}\n--- source ---\n{source}", cell.label));
        }
    }
    assert!(failures.is_empty(), "{} matrix cell(s) failed:\n\n{}", failures.len(), failures.join("\n\n"));
}

fn check_cell(
    api: &DarkmatterSchemas,
    context: &FileResolutionContext,
    dir: &Path,
    file_name: &str,
    source: &str,
    envelope: StandaloneSchemaEnvelope,
    cell: &Cell,
) -> Result<(), String> {
    let schema_path = dir.join(file_name);
    let classified = parse_standalone_schema_document(source, &schema_path);
    let loaded = resolve_schema_with_roots(&json!(format!("./{file_name}")), dir, &[], context);
    match cell.outcome {
        Declines => {
            match &classified {
                Ok(None) => {}
                other => return Err(format!("expected the classifier to decline, got {other:?}")),
            }
            // Only a YAML mapping has a raw JSON Schema reading; anything else
            // (invalid YAML, an empty document) is rejected by normal loading.
            let valid_yaml = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(source)
                .is_ok_and(|value| value.is_mapping());
            match (&loaded, valid_yaml) {
                (Ok(resolved), true) if resolved.simplified.is_none() => Ok(()),
                (Ok(resolved), true) => Err(format!(
                    "valid unrecognized YAML must load as raw JSON Schema, got a SimplifiedSchema: {:?}",
                    resolved.simplified
                )),
                (Err(SchemaError::AmbiguousReferenced { .. }), false) => Ok(()),
                (result, _) => Err(format!(
                    "unexpected load result for {} YAML: {:?}",
                    if valid_yaml { "mapping" } else { "non-mapping or invalid" },
                    result.as_ref().map(|resolved| &resolved.json_schema)
                )),
            }
        }
        Rejects => {
            match &classified {
                Err(SchemaError::SchemaDocument { .. }) => {}
                other => return Err(format!("expected SchemaDocument, got {other:?}")),
            }
            match &loaded {
                Err(SchemaError::SchemaDocument { .. }) => Ok(()),
                other => Err(format!(
                    "normal loading must fail with SchemaDocument, got {:?}",
                    other.as_ref().map(|resolved| &resolved.json_schema)
                )),
            }
        }
        Accepts(instances) => {
            let document = match &classified {
                Ok(Some(document)) => document,
                other => return Err(format!("expected acceptance, got {other:?}")),
            };
            if document.envelope != envelope {
                return Err(format!("expected {envelope:?}, got {:?}", document.envelope));
            }
            match &loaded {
                Ok(resolved) if resolved.simplified.is_some() => {}
                other => {
                    return Err(format!(
                        "normal loading must apply a SimplifiedSchema, got {:?}",
                        other.as_ref().map(|resolved| &resolved.json_schema)
                    ));
                }
            }
            let mut wrong = Vec::new();
            for (instance, expected) in instances {
                let markdown = Markdown::from(format!("---\n$schema: ./{file_name}\n{instance}\n---\nBody\n"))
                    .with_source(ComposeSource::File(dir.join("document.md")));
                let effective = api
                    .effective_for(&markdown)
                    .map_err(|error| format!("effective schema for {instance:?}: {error}"))?
                    .ok_or_else(|| format!("no effective schema for {instance:?}"))?;
                let frontmatter = markdown_frontmatter(instance);
                let report = effective.validate(&frontmatter);
                if report.valid != *expected {
                    wrong.push(format!(
                        "{instance:?}: expected valid={expected}, problems {:?}",
                        report.problems
                    ));
                }
            }
            if wrong.is_empty() { Ok(()) } else { Err(wrong.join("; ")) }
        }
    }
}

fn markdown_frontmatter(instance: &str) -> serde_json::Value {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(instance).expect("instance YAML");
    serde_json::to_value(yaml).expect("instance as JSON")
}

#[test]
fn tagged_envelope_robustness_matrix() {
    let seed = include_str!("../fixtures/schema_yaml_spellings/tagged.conventional.yaml");
    walk_matrix("tagged", seed, StandaloneSchemaEnvelope::Tagged, TAGGED_CELLS);
}

#[test]
fn pure_mapping_robustness_matrix() {
    let seed = include_str!("../fixtures/schema_yaml_spellings/nested.conventional.yaml");
    walk_matrix("nested", seed, StandaloneSchemaEnvelope::Pure, NESTED_CELLS);
}

#[test]
fn pure_root_union_robustness_matrix() {
    let seed = include_str!("../fixtures/schema_yaml_spellings/root-union.conventional.yaml");
    walk_matrix("root-union", seed, StandaloneSchemaEnvelope::Pure, ROOT_UNION_CELLS);
}

/// Provenance: each `serialized` fixture is exactly what `serde_yaml_ng`
/// writes for its conventional seed's value, so the indentless spelling under
/// test is real serializer output rather than a hand-written approximation.
#[test]
fn serializer_captures_are_serde_yaml_ng_output() {
    let pairs = [
        (
            include_str!("../fixtures/schema_yaml_spellings/tagged.conventional.yaml"),
            include_str!("../fixtures/schema_yaml_spellings/tagged.serialized.yaml"),
        ),
        (
            include_str!("../fixtures/schema_yaml_spellings/nested.conventional.yaml"),
            include_str!("../fixtures/schema_yaml_spellings/nested.serialized.yaml"),
        ),
        (
            include_str!("../fixtures/schema_yaml_spellings/root-union.conventional.yaml"),
            include_str!("../fixtures/schema_yaml_spellings/root-union.serialized.yaml"),
        ),
    ];
    for (conventional, serialized) in pairs {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(conventional).expect("seed YAML");
        assert_eq!(serde_yaml_ng::to_string(&value).expect("serialize seed"), serialized);
    }
}

fn envelope_payload(source: &str) -> serde_yaml_ng::Value {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(source).expect("fixture YAML");
    let mapping = value.as_mapping().expect("fixture mapping");
    mapping
        .get("types")
        .or_else(|| mapping.get("$schema"))
        .cloned()
        .expect("fixture payload")
}

/// `(conventional, new presentation)` pairs: indentless sequences and plain
/// scalars continued across physical lines, each beside a sibling that must
/// not be consumed.
///
/// Each pair means the same schema through the source-free semantic parser,
/// so the source-aware loader is the only thing that can tell them apart.
///
/// The new presentations' outcome is the baseline before the strict locator
/// accepts them, captured so the locator change cannot land without replacing
/// it with structural equality through `parse_standalone_schema_document`.
/// Most fail at the documented projection boundary. A continuation that lies
/// wholly inside a description is worse: the strict locator stops at the first
/// physical line, leaves the remaining lines unread, and reports success with
/// a source map that has no span for the sibling `next` — an approximate range
/// presented as a successful one.
#[test]
fn presentation_pairs_share_semantics_and_record_the_strict_baseline() {
    let pairs: [(&str, &str, &str); 8] = [
        (
            "tagged serialized",
            include_str!("../fixtures/schema_yaml_spellings/tagged.conventional.yaml"),
            include_str!("../fixtures/schema_yaml_spellings/tagged.serialized.yaml"),
        ),
        (
            "nested serialized",
            include_str!("../fixtures/schema_yaml_spellings/nested.conventional.yaml"),
            include_str!("../fixtures/schema_yaml_spellings/nested.serialized.yaml"),
        ),
        (
            "root union serialized",
            include_str!("../fixtures/schema_yaml_spellings/root-union.conventional.yaml"),
            include_str!("../fixtures/schema_yaml_spellings/root-union.serialized.yaml"),
        ),
        (
            "anchored indentless",
            include_str!("../fixtures/schema_yaml_spellings/anchors.conventional.yaml"),
            include_str!("../fixtures/schema_yaml_spellings/anchors.indentless.yaml"),
        ),
        (
            "spec indentless example",
            include_str!("../fixtures/schema_yaml_spellings/spec-indentless.conventional.yaml"),
            include_str!("../fixtures/schema_yaml_spellings/spec-indentless.indentless.yaml"),
        ),
        (
            "spec continued example",
            include_str!("../fixtures/schema_yaml_spellings/spec-continued.conventional.yaml"),
            include_str!("../fixtures/schema_yaml_spellings/spec-continued.continued.yaml"),
        ),
        (
            "continued expression, constraint, and item",
            include_str!("../fixtures/schema_yaml_spellings/continued.conventional.yaml"),
            include_str!("../fixtures/schema_yaml_spellings/continued.continued.yaml"),
        ),
        (
            "blank line, no final newline",
            include_str!("../fixtures/schema_yaml_spellings/blank-line.conventional.yaml"),
            include_str!("../fixtures/schema_yaml_spellings/blank-line.continued.yaml"),
        ),
    ];
    let truncated_description = ["spec continued example", "blank line, no final newline"];

    for (label, conventional, presented) in pairs {
        let expected = to_json_schema(&parse_yaml_schema(&envelope_payload(conventional)).expect(label))
            .expect(label);
        let actual = to_json_schema(&parse_yaml_schema(&envelope_payload(presented)).expect(label))
            .expect(label);
        assert_eq!(actual, expected, "{label}: presentations must resolve to equal JSON Schema");

        let document = parse_standalone_schema_document(conventional, "/w/conventional.yaml")
            .unwrap_or_else(|error| panic!("{label}: conventional spelling must load: {error}"))
            .unwrap_or_else(|| panic!("{label}: conventional spelling must be recognized"));
        let loaded = to_json_schema(document.schema().expect("inline schema")).expect(label);
        assert_eq!(loaded, expected, "{label}: source-aware load must match the semantic parse");

        let result = parse_standalone_schema_document(presented, "/w/presented.yaml");
        if truncated_description.contains(&label) {
            let document = result
                .unwrap_or_else(|error| panic!("{label}: {error}"))
                .unwrap_or_else(|| panic!("{label}: recognized envelope"));
            let next = SchemaSourcePath::root().property("next");
            assert!(
                document.source_map.spans(&next, SchemaSpanKind::MappingKey).is_empty(),
                "{label}: baseline drops the sibling after the continued description"
            );
            continue;
        }
        let error = result.expect_err(label);
        assert!(matches!(error, SchemaError::SchemaDocument { .. }), "{label}: {error:?}");
        assert!(
            error
                .to_string()
                .contains("could not project SimplifiedSchema expression spans through YAML source"),
            "{label}: {error}"
        );
    }
}
