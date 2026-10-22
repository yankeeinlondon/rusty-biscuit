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
pub(crate) enum Edit {
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
    // A native YAML null definition is the `null` type: a non-null value
    // fails, null passes, and the other definitions are untouched.
    cell(
        "title null",
        Set("title", " null"),
        Accepts(&[("title: ab", false), ("title: null", true), ("status: word", false)]),
    ),
    cell(
        "title tilde",
        Set("title", " ~"),
        Accepts(&[("title: abcd", false), ("title: null", true)]),
    ),
    cell(
        "title empty value",
        Set("title", ""),
        Accepts(&[("title: ab", false), ("title: null", true), ("status: word", false)]),
    ),
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
    // A native null arm is the `null` type's arm, never a discarded one.
    cell(
        "status null arm",
        Replace("- boolean", "- null"),
        Accepts(&[("status: true", false), ("status: 5", true), ("status: null", true)]),
    ),
    cell(
        "status empty arm",
        Replace("- boolean", "-"),
        Accepts(&[("status: true", false), ("status: 5", true), ("status: null", true)]),
    ),
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
    cell(
        "level null",
        Set("level", " null"),
        Accepts(&[("meta:\n  level: 3", false), ("meta:\n  level: null", true), ("meta:\n  mode: word", false)]),
    ),
    cell(
        "level empty value",
        Set("level", ""),
        Accepts(&[("meta:\n  level: 3", false), ("meta:\n  level: null", true), ("meta:\n  mode: word", false)]),
    ),
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

pub(crate) fn apply(seed: &str, edit: Edit) -> String {
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

/// The same cells against the `serde_yaml_ng` captures, whose block sequences
/// are indentless: the presentation changes no cell's outcome.
#[test]
fn tagged_envelope_robustness_matrix_indentless() {
    let seed = include_str!("../fixtures/schema_yaml_spellings/tagged.serialized.yaml");
    walk_matrix("tagged-serialized", seed, StandaloneSchemaEnvelope::Tagged, TAGGED_CELLS);
}

#[test]
fn pure_mapping_robustness_matrix_indentless() {
    let seed = include_str!("../fixtures/schema_yaml_spellings/nested.serialized.yaml");
    walk_matrix("nested-serialized", seed, StandaloneSchemaEnvelope::Pure, NESTED_CELLS);
}

#[test]
fn pure_root_union_robustness_matrix_indentless() {
    let seed = include_str!("../fixtures/schema_yaml_spellings/root-union.serialized.yaml");
    walk_matrix("root-union-serialized", seed, StandaloneSchemaEnvelope::Pure, ROOT_UNION_CELLS);
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
/// and the source-aware standalone loader must agree: both spellings load,
/// resolve to structurally equal JSON Schema, and keep a span for the sibling
/// `next` after the list or continuation. Before the strict locator accepted
/// these presentations, most failed at the projection boundary, and a
/// continuation wholly inside a description loaded with its remaining lines
/// unread and no span for `next`.
#[test]
fn presentation_pairs_load_with_equal_semantics_and_keep_their_siblings() {
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
    let next = SchemaSourcePath::root().property("next");
    for (label, conventional, presented) in pairs {
        let expected = to_json_schema(&parse_yaml_schema(&envelope_payload(conventional)).expect(label))
            .expect(label);
        let actual = to_json_schema(&parse_yaml_schema(&envelope_payload(presented)).expect(label))
            .expect(label);
        assert_eq!(actual, expected, "{label}: presentations must resolve to equal JSON Schema");

        for (spelling, source) in [("conventional", conventional), ("presented", presented)] {
            let document = parse_standalone_schema_document(source, "/w/schema.yaml")
                .unwrap_or_else(|error| panic!("{label} {spelling}: must load: {error}"))
                .unwrap_or_else(|| panic!("{label} {spelling}: must be recognized"));
            let loaded = to_json_schema(document.schema().expect("inline schema")).expect(label);
            assert_eq!(loaded, expected, "{label} {spelling}: source-aware load must match the semantic parse");
            if source.contains("next:") {
                let keys = document.source_map.spans(&next, SchemaSpanKind::MappingKey);
                assert_eq!(keys.len(), 1, "{label} {spelling}: sibling `next` keeps its key span");
                assert_eq!(&source[keys[0].clone()], "next", "{label} {spelling}");
            }
        }
    }
}

/// The authored text of every span recorded for `path` and `kind`.
fn span_texts<'a>(
    source: &'a str,
    document: &darkmatter::markdown::schemas::StandaloneSchemaDocument,
    path: &SchemaSourcePath,
    kind: SchemaSpanKind,
) -> Vec<&'a str> {
    document
        .source_map
        .spans(path, kind)
        .iter()
        .map(|span| &source[span.clone()])
        .collect()
}

/// The union-arm span text of each of `count` arms under `path`.
fn arm_texts<'a>(
    source: &'a str,
    document: &darkmatter::markdown::schemas::StandaloneSchemaDocument,
    path: &SchemaSourcePath,
    count: usize,
) -> Vec<&'a str> {
    (0..count)
        .flat_map(|index| span_texts(source, document, &path.union_arm(index), SchemaSpanKind::UnionArm))
        .collect()
}

fn load(source: &str) -> darkmatter::markdown::schemas::StandaloneSchemaDocument {
    parse_standalone_schema_document(source, "/w/schema.yaml")
        .unwrap_or_else(|error| panic!("{source:?} must load: {error}"))
        .unwrap_or_else(|| panic!("{source:?} must be recognized"))
}

/// A continued plain scalar is read whole: a token on a continuation line has
/// a range on that line without its indentation, and a token crossing a fold
/// has one range from its first authored byte to its last, line break and
/// indentation inside. A trailing comment is not part of the definition.
/// Both line endings are built explicitly, so the checkout never decides.
#[test]
fn continued_plain_scalars_project_structural_spans_onto_authored_lines() {
    let lf = include_str!("../fixtures/schema_yaml_spellings/continued.continued.yaml");
    for newline in ["\n", "\r\n"] {
        let source = lf.replace('\n', newline);
        let document = load(&source);
        let root = SchemaSourcePath::root();
        let title = root.property("title");
        let texts = |path: &SchemaSourcePath, kind| span_texts(&source, &document, path, kind);

        assert_eq!(
            texts(&title, SchemaSpanKind::Definition),
            [format!("string(min(3);{newline}    max(40)) -> the display{newline}    title")],
            "{newline:?}"
        );
        assert_eq!(
            texts(&title, SchemaSpanKind::Atom),
            [format!("string(min(3);{newline}    max(40))")],
            "{newline:?}"
        );
        assert_eq!(texts(&title, SchemaSpanKind::Constraint), ["min(3)", "max(40)"], "{newline:?}");
        assert_eq!(texts(&title, SchemaSpanKind::Argument), ["3", "40"], "{newline:?}");

        // `max(40)` sits on the second physical line, after its indentation.
        let max = document.source_map.spans(&title, SchemaSpanKind::Constraint)[1].clone();
        let line_start = source[..max.start].rfind('\n').expect("continuation line") + 1;
        assert_eq!(&source[line_start..max.start], "    ", "{newline:?}");

        let tags = root.property("tags");
        assert_eq!(
            texts(&tags.union_arm(0), SchemaSpanKind::Atom),
            [format!("string(min(2){newline}      )")],
            "{newline:?}"
        );
        assert_eq!(texts(&tags.union_arm(0), SchemaSpanKind::Constraint), ["min(2)"], "{newline:?}");
        assert_eq!(texts(&tags.union_arm(1), SchemaSpanKind::Atom), ["number(max(9))"], "{newline:?}");
        assert_eq!(texts(&root.property("next"), SchemaSpanKind::Definition), ["boolean"], "{newline:?}");
    }
}

/// A blank line inside a continued scalar is a line break in the decoded
/// text, exactly as `serde_yaml_ng` reads it, and the scalar without a final
/// newline still ends at its last authored byte.
#[test]
fn a_blank_line_in_a_continued_scalar_decodes_like_the_yaml_parser() {
    let source = include_str!("../fixtures/schema_yaml_spellings/blank-line.continued.yaml");
    let payload = envelope_payload(source);
    let title = payload.get("title").and_then(serde_yaml_ng::Value::as_str).expect("title");
    assert_eq!(title, "string(min(3)) -> first paragraph\nsecond paragraph");

    let document = load(source);
    let root = SchemaSourcePath::root();
    assert_eq!(
        span_texts(source, &document, &root.property("title"), SchemaSpanKind::Definition),
        ["string(min(3)) -> first paragraph\n\n    second paragraph"]
    );
    assert_eq!(
        span_texts(source, &document, &root.property("next"), SchemaSpanKind::Definition),
        ["boolean"]
    );
    assert!(!source.ends_with('\n'), "the fixture must end without a newline");
}

/// Indentless block sequences under both envelopes, in a nested mapping arm,
/// as a pure root union of compact mappings, and after a mapping-value anchor
/// project each arm to its own item, and the sibling after the list stays a
/// sibling.
#[test]
fn indentless_sequences_project_each_arm_and_stop_at_the_sibling() {
    let root = SchemaSourcePath::root();

    let tagged = include_str!("../fixtures/schema_yaml_spellings/tagged.serialized.yaml");
    let document = load(tagged);
    let status = root.property("status");
    assert_eq!(
        arm_texts(tagged, &document, &status, 3),
        ["number", "boolean", "code: number(min(1))"]
    );
    assert_eq!(
        span_texts(tagged, &document, &status.union_arm(2).property("code"), SchemaSpanKind::Constraint),
        ["min(1)"]
    );
    assert_eq!(
        span_texts(tagged, &document, &root.property("next"), SchemaSpanKind::MappingKey),
        ["next"]
    );

    let nested = include_str!("../fixtures/schema_yaml_spellings/nested.serialized.yaml");
    let document = load(nested);
    assert_eq!(
        arm_texts(nested, &document, &root.property("meta").property("mode"), 2),
        ["number", "boolean"]
    );
    assert_eq!(
        span_texts(nested, &document, &root.property("next"), SchemaSpanKind::Definition),
        ["boolean"]
    );

    let union = include_str!("../fixtures/schema_yaml_spellings/root-union.serialized.yaml");
    let document = load(union);
    assert_eq!(
        arm_texts(union, &document, &root, 2),
        ["title: string(required)", "count: number(required)"]
    );
    assert_eq!(
        span_texts(union, &document, &root.union_arm(1).property("count"), SchemaSpanKind::Constraint),
        ["required"]
    );

    let anchors = include_str!("../fixtures/schema_yaml_spellings/anchors.indentless.yaml");
    let document = load(anchors);
    assert_eq!(
        span_texts(anchors, &document, &root.property("status"), SchemaSpanKind::Definition),
        ["&arms\n  - number\n  - boolean"]
    );
    assert_eq!(
        arm_texts(anchors, &document, &root.property("status"), 2),
        ["number", "boolean"]
    );
    assert_eq!(
        span_texts(anchors, &document, &root.property("fallback"), SchemaSpanKind::Definition),
        ["*arms"]
    );
    assert_eq!(
        span_texts(anchors, &document, &root.property("next"), SchemaSpanKind::Definition),
        ["boolean"]
    );
}

/// Non-ASCII text before and inside a fold leaves byte ranges exact.
#[test]
fn continued_scalars_project_byte_ranges_through_non_ascii_text() {
    let source = "kind: schema\ntypes:\n  café: string(min(1)) -> naïve\n    ünïcode max(2) text\n  t: string(min(1);\n    max(2)) -> é\n";
    let document = load(source);
    let root = SchemaSourcePath::root();
    assert_eq!(
        span_texts(source, &document, &root.property("café"), SchemaSpanKind::Definition),
        ["string(min(1)) -> naïve\n    ünïcode max(2) text"]
    );
    assert_eq!(
        span_texts(source, &document, &root.property("t"), SchemaSpanKind::Constraint),
        ["min(1)", "max(2)"]
    );
    let max = document.source_map.spans(&root.property("t"), SchemaSpanKind::Constraint)[1].clone();
    assert_eq!(max.start, source.rfind("max(2)").expect("continuation token"));
}

/// The source-aware entry points that take a caller's document offset add it
/// once, after projecting through the fold.
#[test]
fn a_continued_definition_projects_through_a_caller_offset() {
    use darkmatter::markdown::schemas::parse_property_definition_with_source;

    for newline in ["\n", "\r\n"] {
        let yaml = format!("string(min(1);{newline}  max(5)) -> é{newline}");
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml).expect("YAML");
        let projected = parse_property_definition_with_source("p", &value, &yaml, 100)
            .unwrap_or_else(|error| panic!("{yaml:?}: {error}"));
        let constraints: Vec<&str> = projected
            .source_map
            .spans(&SchemaSourcePath::root(), SchemaSpanKind::Constraint)
            .iter()
            .map(|span| &yaml[span.start - 100..span.end - 100])
            .collect();
        assert_eq!(constraints, ["min(1)", "max(5)"], "{newline:?}");
    }
}

/// Every presentation outside the two this grammar accepts still fails at the
/// projection boundary through the standalone loader, wrapped in
/// `SchemaDocument`, including when it hides behind a continuation or an
/// indentless list. Where the source-free parser accepts the value, the
/// failure is the presentation boundary rather than the schema.
#[test]
fn strict_source_aware_parsing_keeps_every_other_presentation_closed() {
    let cases: &[(&str, &str)] = &[
        ("literal block scalar", "$schema:\n  t: |\n    string\n  next: boolean\n"),
        ("folded block scalar", "$schema:\n  t: >-\n    string(min(1))\n  next: boolean\n"),
        ("block scalar arm", "$schema:\n  t:\n  - |\n    string\n  - number\n"),
        ("tag", "$schema:\n  t: !!str string\n  next: boolean\n"),
        ("tag on an indentless arm", "$schema:\n  t:\n  - !!str string\n  - number\n  next: boolean\n"),
        ("tag before a continuation", "$schema:\n  t: !!str string -> a\n    b\n  next: boolean\n"),
        ("multi-line double-quoted", "$schema:\n  t: \"string -> a\n    b\"\n  next: boolean\n"),
        ("multi-line single-quoted", "$schema:\n  t: 'string -> a\n    b'\n  next: boolean\n"),
        ("multi-line quoted arm", "$schema:\n  t:\n  - \"string -> a\n    b\"\n  - number\n"),
        ("complex key", "$schema:\n  ? [a, b]\n  : string\n"),
        ("explicit flow-mapping key", "$schema:\n  t: {? a: string}\n"),
        ("directive", "%YAML 1.2\n---\n$schema:\n  t: string\n"),
    ];
    for (label, source) in cases {
        let semantic_ok = parse_yaml_schema(&envelope_payload(source)).is_ok();
        let error = parse_standalone_schema_document(source, "/w/schema.yaml")
            .expect_err(label);
        assert!(matches!(error, SchemaError::SchemaDocument { .. }), "{label}: {error:?}");
        if semantic_ok {
            assert!(
                error
                    .to_string()
                    .contains("could not project SimplifiedSchema expression spans through YAML source"),
                "{label}: {error}"
            );
        }
    }
}

/// The authored spans of each `suggest(...)` candidate of the single-atom
/// top-level property `name`.
fn suggestion_spans(
    schema: &darkmatter::markdown::schemas::SimplifiedSchema,
    name: &str,
) -> Vec<std::ops::Range<usize>> {
    use darkmatter::markdown::schemas::{Constraint, PropertyDef, SimplifiedSchema};
    let SimplifiedSchema::Single(shape) = schema else {
        panic!("expected a single root shape");
    };
    let PropertyDef::Single(atom) = &shape.properties[name] else {
        panic!("expected a single atom for {name}");
    };
    atom.constraints
        .iter()
        .find_map(|constraint| match constraint {
            Constraint::Suggest(candidates) => {
                Some(candidates.iter().map(|candidate| candidate.span.clone()).collect())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("{name} has no suggest(...)"))
}

/// `suggest(...)` candidates on a continuation line project to their authored
/// bytes, each on one physical line. A lint-invalid candidate on a
/// continuation line is advisory: the schema still loads, and the problem
/// points at that line. A candidate whose own range crosses a fold is
/// `a_suggestion_candidate_crossing_a_fold_projects_one_authored_range`.
#[test]
fn suggestion_candidates_on_continuation_lines_project_exactly() {
    let lf = "kind: schema\ntypes:\n  mode: string(min(5);\n    suggest(no,\n      valid)) -> pick\n    one\n  next: boolean\n";
    for newline in ["\n", "\r\n"] {
        let source = lf.replace('\n', newline);
        let document = load(&source);
        let schema = document.schema().expect("inline schema");
        let candidates = suggestion_spans(schema, "mode");
        let texts: Vec<&str> = candidates.iter().map(|span| &source[span.clone()]).collect();
        assert_eq!(texts, ["no", "valid"], "{newline:?}");

        let valid = source.find("valid").expect("candidate");
        let line_start = source[..valid].rfind('\n').expect("continuation line") + 1;
        assert_eq!(&source[line_start..valid], "      ", "{newline:?}");

        assert_eq!(document.suggestion_lints.len(), 1, "{newline:?}: {:?}", document.suggestion_lints);
        let problem = &document.suggestion_lints[0];
        assert_eq!(&source[problem.span.clone()], "no", "{newline:?}");
        assert_eq!(problem.span.start, source.find("no,").expect("lint target"), "{newline:?}");

        // Structural and suggestion coordinates agree on the same arguments.
        let path = SchemaSourcePath::root().property("mode");
        let arguments = span_texts(&source, &document, &path, SchemaSpanKind::Argument);
        assert_eq!(arguments, ["5", "no", "valid"], "{newline:?}");
        let argument_spans = document.source_map.spans(&path, SchemaSpanKind::Argument);
        assert_eq!(&argument_spans[1..], candidates.as_slice(), "{newline:?}");
    }
}

/// A quoted candidate split by a fold decodes with the fold as one space, and
/// its one range runs from the opening quote to the closing quote, line break
/// and indentation inside. The structural `Argument` span and the advisory
/// `min(40)` lint report that same range, and the schema still loads.
#[test]
fn a_suggestion_candidate_crossing_a_fold_projects_one_authored_range() {
    for newline in ["\n", "\r\n"] {
        let source = format!(
            "kind: schema{newline}types:{newline}  mode: string(min(40); suggest('first{newline}    second')){newline}  next: boolean{newline}"
        );
        let document = load(&source);
        let schema = document.schema().expect("inline schema");

        let start = source.find("'first").expect("opening quote");
        let end = source.find("second'").expect("closing quote") + "second'".len();
        assert_eq!(&source[start..end], format!("'first{newline}    second'"), "{newline:?}");
        let candidate = start..end;
        assert_eq!(suggestion_spans(schema, "mode"), [candidate], "{newline:?}");

        use darkmatter::markdown::schemas::{Constraint, PropertyDef, SimplifiedSchema};
        let SimplifiedSchema::Single(shape) = schema else { panic!("single root shape") };
        let PropertyDef::Single(atom) = &shape.properties["mode"] else { panic!("single atom") };
        let decoded: Vec<&str> = atom
            .constraints
            .iter()
            .filter_map(|constraint| match constraint {
                Constraint::Suggest(candidates) => Some(candidates),
                _ => None,
            })
            .flatten()
            .map(|candidate| candidate.decoded.as_str())
            .collect();
        assert_eq!(decoded, ["first second"], "{newline:?}");

        let path = SchemaSourcePath::root().property("mode");
        let arguments = document.source_map.spans(&path, SchemaSpanKind::Argument);
        assert_eq!(arguments.len(), 2, "{newline:?}: {arguments:?}");
        assert_eq!(&source[arguments[0].clone()], "40", "{newline:?}");
        assert_eq!(arguments[1], start..end, "{newline:?}");

        assert_eq!(document.suggestion_lints.len(), 1, "{newline:?}: {:?}", document.suggestion_lints);
        assert_eq!(document.suggestion_lints[0].span, start..end, "{newline:?}");

        // The sibling after the fold keeps its own definition.
        assert_eq!(
            span_texts(&source, &document, &SchemaSourcePath::root().property("next"), SchemaSpanKind::Definition),
            ["boolean"],
            "{newline:?}"
        );
    }
}

/// The inline-schema suggestion projector carries a candidate crossing a fold
/// as one range enclosing both authored lines, with the caller's document
/// offset applied exactly once.
#[test]
fn a_folded_suggestion_candidate_projects_through_a_caller_offset() {
    use darkmatter::markdown::schemas::simplified::parse_yaml_schema_with_source;

    const OFFSET: usize = 100;
    for newline in ["\n", "\r\n"] {
        let yaml = format!("mode: string(min(40); suggest('first{newline}  second')){newline}next: boolean{newline}");
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml).expect("YAML");
        let schema = parse_yaml_schema_with_source(&value, &yaml, OFFSET)
            .unwrap_or_else(|error| panic!("{yaml:?}: {error}"));

        let start = yaml.find("'first").expect("opening quote");
        let end = yaml.find("second'").expect("closing quote") + "second'".len();
        let projected = start + OFFSET..end + OFFSET;
        assert_eq!(suggestion_spans(&schema, "mode"), [projected], "{newline:?}");
        assert_eq!(&yaml[start..end], format!("'first{newline}  second'"), "{newline:?}");
    }
}

/// Identical definitions, folded and single-line, each keep their own
/// occurrence's candidate spans, with unrelated definitions between them.
#[test]
fn repeated_identical_definitions_keep_their_own_suggestion_spans() {
    let source = "$schema:\n  plain: string\n  a: string(suggest(x,\n    y))\n  other: string(min(1))\n  b: string(suggest(x, y))\n  c: string(suggest(x,\n      y))\n";
    let document = load(source);
    let schema = document.schema().expect("inline schema");
    let x_positions: Vec<usize> = source.match_indices("x").map(|(at, _)| at).collect();
    let y_positions: Vec<usize> = source.match_indices("y))").map(|(at, _)| at).collect();
    for (index, name) in ["a", "b", "c"].into_iter().enumerate() {
        let spans = suggestion_spans(schema, name);
        assert_eq!(spans, [x_positions[index]..x_positions[index] + 1, y_positions[index]..y_positions[index] + 1], "{name}");
    }
}

/// The suggestion projector's document offset is added once, after the fold,
/// with non-ASCII text before the candidate.
#[test]
fn folded_suggestions_project_through_a_caller_offset_and_non_ascii_text() {
    use darkmatter::markdown::schemas::simplified::parse_yaml_schema_with_source;

    for newline in ["\n", "\r\n"] {
        let yaml = format!("café: string -> naïve{newline}  text{newline}value: string(suggest(ünï,{newline}    beta)){newline}");
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml).expect("YAML");
        let schema = parse_yaml_schema_with_source(&value, &yaml, 100)
            .unwrap_or_else(|error| panic!("{yaml:?}: {error}"));
        let spans = suggestion_spans(&schema, "value");
        let texts: Vec<&str> = spans.iter().map(|span| &yaml[span.start - 100..span.end - 100]).collect();
        assert_eq!(texts, ["ünï", "beta"], "{newline:?}");
    }
}

/// A folded value decodes exactly as `serde_yaml_ng` reads it, through
/// multiple continuation lines, trailing whitespace, a trailing comment, and
/// a final line without a newline, and means what its single-line spelling
/// means.
#[test]
fn folded_values_decode_like_the_yaml_parser_and_match_single_line_semantics() {
    let cases: &[(&str, &str)] = &[
        (
            "$schema:\n  t: string(min(1);   \n    max(9))   \n    -> one   \n    two # note\n  next: boolean",
            "$schema:\n  t: string(min(1); max(9)) -> one two\n  next: boolean\n",
        ),
        (
            "$schema:\n  t:\n  - string(min(1)\n    ) -> a\n\n\n    b\n  - number\n  next: boolean\n",
            "$schema:\n  t:\n  - \"string(min(1) ) -> a\\n\\nb\"\n  - number\n  next: boolean\n",
        ),
    ];
    for (folded, single) in cases {
        let folded_value = envelope_payload(folded);
        let single_value = envelope_payload(single);
        assert_eq!(folded_value, single_value, "{folded:?}: the YAML parser must read both alike");
        let expected = to_json_schema(&parse_yaml_schema(&single_value).expect("single")).expect("single");
        let document = load(folded);
        assert_eq!(to_json_schema(document.schema().expect("schema")).expect("folded"), expected, "{folded:?}");
        assert_eq!(
            span_texts(folded, &document, &SchemaSourcePath::root().property("next"), SchemaSpanKind::Definition),
            ["boolean"],
            "{folded:?}"
        );
    }
}

/// A source buffer that does not author the parsed value is a projection
/// error, never a successful range: the structural parser compares every
/// located scalar with the parser's text, and the suggestion projector finds
/// no scalar meaning the atom.
#[test]
fn a_source_that_disagrees_with_the_value_is_a_projection_error() {
    use darkmatter::markdown::schemas::{
        parse_property_definition_with_source, simplified::parse_yaml_schema_with_source,
    };

    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str("string(min(1)) -> one two").expect("YAML");
    for source in ["string(min(1)) -> one\n  three\n", "string(min(1)) -> one\n"] {
        let error = parse_property_definition_with_source("p", &value, source, 0)
            .expect_err(source);
        assert!(error.to_string().contains("could not project"), "{source:?}: {error}");
    }

    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str("value: string(suggest(alpha, beta))").expect("YAML");
    let error = parse_yaml_schema_with_source(&value, "value: string(suggest(alpha,\n  gamma))\n", 0)
        .expect_err("mismatched candidates");
    assert!(error.to_string().contains("could not project"), "{error}");
}

/// A cursor on a continuation line reads the same token and role as on the
/// single-line spelling, with a range on its authored line.
#[test]
fn a_cursor_on_a_continuation_line_reads_the_authored_token() {
    use darkmatter::markdown::schemas::{SchemaCursorRole, locate_type_definition_cursor};

    for newline in ["\n", "\r\n"] {
        let value = format!("string(min(1);{newline}    ma");
        let state = locate_type_definition_cursor(&value, 40, 40 + value.len()).expect("cursor");
        let single = locate_type_definition_cursor("string(min(1); ma", 0, 17).expect("single-line cursor");
        assert_eq!(state.token, "ma", "{newline:?}");
        assert_eq!(state.role, single.role, "{newline:?}");
        assert!(matches!(state.role, SchemaCursorRole::Constraint { .. }), "{newline:?}");
        assert_eq!(state.token_span, 40 + value.len() - 2..40 + value.len(), "{newline:?}");

        let value = format!("string(min(1);{newline}    suggest(a,{newline}      b");
        let state = locate_type_definition_cursor(&value, 0, value.len()).expect("argument cursor");
        assert!(
            matches!(&state.role, SchemaCursorRole::Argument { constraint, .. } if constraint == "suggest"),
            "{newline:?}: {:?}",
            state.role
        );
        assert_eq!(&value[state.token_span.clone()], "b", "{newline:?}");
    }
}

/// A pattern key authored before a property with an identical, folded
/// definition keeps its own candidate spans. Properties and pattern keys are
/// separate collections in the parsed schema, so only the located definition
/// (not source order) can tell the two occurrences apart.
#[test]
fn a_pattern_key_before_an_identical_property_keeps_its_own_suggestion_spans() {
    use darkmatter::markdown::schemas::{Constraint, PropertyDef, SimplifiedSchema};

    for newline in ["\n", "\r\n"] {
        let source = format!(
            "$schema:{newline}  \"<starting::x>\": string(suggest(a,{newline}    b)){newline}  k: string(suggest(a,{newline}    b)){newline}"
        );
        let document = load(&source);
        let SimplifiedSchema::Single(shape) = document.schema().expect("schema") else {
            panic!("expected a single root shape");
        };
        let pattern_line = source.find("<starting").expect("pattern key");
        let property_line = source.find("  k:").expect("property");

        let PropertyDef::Single(pattern) = &shape.pattern_keys[0].def else {
            panic!("expected a single pattern atom");
        };
        let Some(Constraint::Suggest(candidates)) = pattern.constraints.first() else {
            panic!("expected suggest on the pattern key");
        };
        assert!(
            candidates.iter().all(|candidate| (pattern_line..property_line).contains(&candidate.span.start)),
            "{newline:?}: pattern-key candidates must stay in its own definition: {candidates:?}"
        );
        assert!(
            suggestion_spans(document.schema().expect("schema"), "k")
                .iter()
                .all(|span| span.start > property_line),
            "{newline:?}: property candidates must stay in its own definition"
        );
    }
}
