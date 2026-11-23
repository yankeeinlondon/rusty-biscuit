//! Where a refused member's problem is: the file and 1-based line and
//! column a registration warning reports, recovered from the typed error
//! before it becomes the warning's message.
//!
//! The rule is the most specific location available. An error raised in a
//! file another member imports is reported in that file, at the declaration
//! that failed; a declaration inside a union arm at that arm, never another
//! arm or a same-named declaration under the other of `$schema` and
//! `types`; a grammar failure at the parser's rejected token; a missing
//! field at the mapping that lacks it; a reference that cannot be followed
//! at its spelling as an import or `example(...)` argument in the
//! declaration that writes it, never in prose that repeats it (a comment, a
//! top-level `description`, or any declaration's `-> description`). A
//! position is never invented: when nothing more specific is known the
//! nearest known declaration, or the file alone, is reported.

use std::path::{Path, PathBuf};

use simplified_schema::simplified::grammar::without_descriptions;
use simplified_schema::yaml_documents::{declaration_segments, load_yaml_document, locating_problem};
use simplified_schema::{SchemaError, SchemaReferenceSite, SchemaSourceId, SchemaSourcePath};

use crate::config::load::{SourcePosition, line_column, reported_position};
use crate::markdown::compose::expression::host::artifact::ReasonSite;

/// A file and, when known, the 1-based line and character column in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Site {
    pub file: PathBuf,
    pub at: Option<SourcePosition>,
}

impl Site {
    fn unlocated(file: &Path) -> Self {
        Self { file: file.to_path_buf(), at: None }
    }
}

/// The text of a file the bundle verified, by its location.
pub(super) type Texts<'a> = dyn Fn(&Path) -> Option<&'a str> + 'a;

/// The top-level keys a member's payload declaration may be under, most
/// likely first: a payload resolves its `$schema` with its private `types`.
const PAYLOAD: &[&str] = &["$schema", "types"];
/// The same keys for a member's `types` table, and for a file another member
/// imports a named type from.
const TYPES: &[&str] = &["types", "$schema"];

/// Where `error`, raised while preparing the payload of the file `file`, is
/// about.
pub(super) fn schema_error(error: &SchemaError, file: &Path, texts: &Texts<'_>) -> Site {
    located(error, file, texts, PAYLOAD)
}

/// Where `error`, raised while checking the `types` table of the file
/// `file`, is about.
pub(super) fn declared_type_error(error: &SchemaError, file: &Path, texts: &Texts<'_>) -> Site {
    located(error, file, texts, TYPES)
}

/// Where `error` is about, `carriers` naming where in `file` a declaration
/// path is relative to.
fn located(error: &SchemaError, file: &Path, texts: &Texts<'_>, carriers: &[&str]) -> Site {
    let text = texts(file);
    let here = |at: Option<SourcePosition>| Site { file: file.to_path_buf(), at };
    match error {
        SchemaError::ReferencedSchema { path, source } | SchemaError::TriggerLoad { path, source } => {
            located(source, path, texts, carriers_in(path, file, carriers))
        }
        SchemaError::SchemaDocument { path, span: Some(span), .. } => {
            Site { file: path.clone(), at: texts(path).map(|text| line_column(text, span.start)) }
        }
        SchemaError::SchemaDocument { path, source: Some(source), .. } => located(source, path, texts, carriers_in(path, file, carriers)),
        // The standalone parser reports a syntax error without a span; the
        // loader reports the parser's position in the same verified text.
        SchemaError::SchemaDocument { path, .. } => Site { file: path.clone(), at: texts(path).and_then(|text| parse_failure(text, path)) },
        SchemaError::NotASchemaDocument { path, .. }
        | SchemaError::UnsupportedJsonDialect { path, .. }
        | SchemaError::InvalidJsonSchema { path, .. } => Site::unlocated(path),
        SchemaError::Aggregate { errors } => errors.first().map_or_else(|| here(None), |first| located(first, file, texts, carriers)),
        // The cause first; when it names no position, the operand or member
        // declaration it was reached through, which is on the failing path.
        SchemaError::CompositionReference { source, site, operand } => {
            let cause = located(source, file, texts, carriers);
            if cause.at.is_some() { cause } else { reference_site(site, Some(operand), file, texts, carriers) }
        }
        SchemaError::ComposedMember { source, site } => {
            let cause = located(source, file, texts, carriers);
            if cause.at.is_some() { cause } else { reference_site(site, None, file, texts, carriers) }
        }
        SchemaError::Baseline { source: Some(source), .. } => located(source, file, texts, carriers),
        SchemaError::UnknownNamedType { site, name, .. }
        | SchemaError::ConstraintConflict { site, name, .. }
        | SchemaError::UnionReference { site, name, .. }
        | SchemaError::InvalidCompositionTarget { site, name, .. } => reference_site(site, Some(name), file, texts, carriers),
        SchemaError::BundleBoundary { reference, .. }
        | SchemaError::NoMatch { reference, .. }
        | SchemaError::Unresolved { reference, .. }
        | SchemaError::RemoteUnsupported { reference }
        | SchemaError::BareNameSiblingExists { reference, .. }
        | SchemaError::InvalidExample { reference, .. }
        | SchemaError::TriggerSchemaReferenced { reference, .. } => {
            here(text.and_then(|text| file_reference(text, file, reference, carriers)))
        }
        SchemaError::TriggerEnvelopeYaml { line: Some(line), column: Some(column), .. } => {
            here(text.and_then(|text| reported_position(text, *line, *column, false)))
        }
        SchemaError::TriggerMatch { .. }
        | SchemaError::TriggerRenamed { .. }
        | SchemaError::TriggerMixedMapping { .. }
        | SchemaError::TriggerVacuousMatch
        | SchemaError::TriggerForbiddenConstraint { .. } => here(text.and_then(|text| {
            simplified_schema::triggers::trigger_error_span(text, error).map(|span| line_column(text, span.start))
        })),
        // A `match` condition's failure is a trigger error naming its field,
        // never one of these, so a payload property of the same name cannot
        // stand in for it.
        SchemaError::Grammar { property, .. } | SchemaError::Convert { property, .. } => {
            here(text.and_then(|text| definition(error, property, text, file, carriers)))
        }
        _ => here(None),
    }
}

/// The carriers of `path`, a file `file` led to: the caller's own when it is
/// `file`, else those of an imported file.
fn carriers_in<'c>(path: &Path, file: &Path, carriers: &'c [&'c str]) -> &'c [&'c str] {
    if path == file { carriers } else { TYPES }
}

/// Where the grammar or conversion failure `error` (about `property`) is in
/// `text`: the parser's own rejected token, or the rejected declaration,
/// when re-parsing a carrier fails the same way, so a failure inside a union
/// arm is found in that arm; else the declaration named `property` on the
/// carrier, or the nearest node on that path.
fn definition(error: &SchemaError, property: &str, text: &str, file: &Path, carriers: &[&str]) -> Option<SourcePosition> {
    let message = error.to_string();
    let parsed = carriers.iter().find_map(|carrier| {
        simplified_schema::locate_carrier_definition_failure(text, carrier)
            .filter(|failure| failure.error.to_string() == message)
            .and_then(|failure| failure.authored_span.or(failure.value_span))
    });
    if let Some(span) = parsed {
        return Some(line_column(text, span.start));
    }
    declaration(text, file, &SchemaSourcePath::root().property(property), carriers, None)
}

/// The declaration a named-type reference site addresses: in the file that
/// wrote it, under the carrier that holds its whole path, through every
/// union arm and sequence index (see [`declaration_segments`]).
fn reference_site(site: &SchemaReferenceSite, name: Option<&str>, file: &Path, texts: &Texts<'_>, carriers: &[&str]) -> Site {
    let (file, carriers) = match &site.source {
        SchemaSourceId::File(path) => (path.as_path(), carriers_in(path, file, carriers)),
        SchemaSourceId::InlineDocument | SchemaSourceId::Embedded(_) => (file, carriers),
    };
    let at = texts(file).and_then(|text| declaration(text, file, &site.path, carriers, name));
    Site { file: file.to_path_buf(), at }
}

/// The position of the declaration at `path` in `text` (see
/// [`declaration_segments`]).
fn declaration(text: &str, file: &Path, path: &SchemaSourcePath, carriers: &[&str], spelled: Option<&str>) -> Option<SourcePosition> {
    let loaded = load_yaml_document(text, file).ok()?;
    let segments = declaration_segments(&loaded, text, path, carriers, spelled)?;
    loaded.byte_range(&locating_problem(&segments)).map(|range| line_column(text, range.start))
}

/// Where `text`, read from `file`, stops loading as YAML; `None` when it loads.
fn parse_failure(text: &str, file: &Path) -> Option<SourcePosition> {
    match load_yaml_document(text, file) {
        Err(SchemaError::SchemaDocument { span: Some(span), .. }) => Some(line_column(text, span.start)),
        _ => None,
    }
}

/// Where the file `reference` is spelled in `text` (read from `file`): in a
/// declaration scalar under `carriers`, at a spelling the declaration
/// grammar reads as a reference — the file of an import (`Name@file`, a
/// `$extends` or `...` operand included) or an argument of `example(...)` —
/// or, for a bare `$schema: file` payload, the carrier's own scalar; the
/// earliest such scalar of the first carrier that has one. Only the
/// declaration part of a scalar is searched, never its `-> description` (see
/// [`without_descriptions`]), and prose outside the carriers (a top-level
/// `description`, a comment) is never read, so neither a sibling's
/// description nor the member's own prose stands in for the reference. When
/// no declaration spells it so, the first carrier present is the nearest
/// known declaration.
///
/// The resolver reports only the reference's spelling, not the declaration
/// that wrote it, so two declarations importing the same file are told
/// apart by authored order alone.
fn file_reference(text: &str, file: &Path, reference: &str, carriers: &[&str]) -> Option<SourcePosition> {
    let reference = reference.trim();
    if reference.is_empty() {
        return None;
    }
    let loaded = load_yaml_document(text, file).ok()?;
    let spelled = carriers.iter().find_map(|carrier| {
        let mut scalars = Vec::new();
        string_scalars(loaded.json.get(*carrier)?, &mut vec![(*carrier).to_string()], &mut scalars);
        scalars
            .iter()
            .filter_map(|segments| {
                let range = loaded.byte_range(&locating_problem(segments))?;
                let node = text.get(range.clone())?;
                Some(range.start + reference_in(node, reference, segments.len() == 1)?)
            })
            .min()
    });
    // No declaration spells the reference as reported (the resolver may
    // have normalized it): the nearest known declaration is the first
    // carrier the error was reached through.
    spelled
        .or_else(|| {
            carriers
                .iter()
                .find_map(|carrier| loaded.byte_range(&locating_problem(&[(*carrier).to_string()])).map(|range| range.start))
        })
        .map(|offset| line_column(text, offset))
}

/// The offset in `node`, one declaration scalar's authored text, of the
/// first spelling of `reference` that its declaration part reads as a
/// reference: after an import's `@`, or as an `example(...)` argument; when
/// `whole` (the scalar is a carrier), also the scalar being the reference.
fn reference_in(node: &str, reference: &str, whole: bool) -> Option<usize> {
    const QUOTING: [char; 4] = [' ', '\t', '"', '\''];
    let declared = without_descriptions(node);
    if whole && declared.trim().trim_matches(QUOTING) == reference {
        return declared.find(reference);
    }
    declared.match_indices(reference).map(|(at, _)| at).find(|&at| {
        let ends = declared[at + reference.len()..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_alphanumeric() || matches!(next, '.' | '/' | '\\' | '_' | '-' | '~')));
        let before = declared[..at].trim_end_matches(QUOTING);
        ends && (before.ends_with('@') || ((before.ends_with('(') || before.ends_with(',')) && in_example(before)))
    })
}

/// Whether the innermost `(` left open in `before` opens an `example`
/// argument list.
fn in_example(before: &str) -> bool {
    let mut closed = 0usize;
    for (at, byte) in before.bytes().enumerate().rev() {
        match byte {
            b')' => closed += 1,
            b'(' if closed == 0 => return before[..at].trim_end().ends_with("example"),
            b'(' => closed -= 1,
            _ => {}
        }
    }
    false
}

/// The paths of every string under `value`, which is at `at`.
fn string_scalars(value: &serde_json::Value, at: &mut Vec<String>, found: &mut Vec<Vec<String>>) {
    match value {
        serde_json::Value::String(_) => found.push(at.clone()),
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                at.push(key.clone());
                string_scalars(child, at, found);
                at.pop();
            }
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                at.push(index.to_string());
                string_scalars(item, at, found);
                at.pop();
            }
        }
        _ => {}
    }
}

/// The value at `segments` of the YAML (or JSON) `text` read from `file`.
pub(super) fn field(text: &str, file: &Path, segments: &[String]) -> Option<SourcePosition> {
    let loaded = load_yaml_document(text, file).ok()?;
    loaded.byte_range(&locating_problem(segments)).map(|range| line_column(text, range.start))
}

/// Where an artifact reader's `site` is in `text` (read from `file`): the
/// parser's position, or the deepest existing value along the field's path,
/// so a field the reader found in a shape the locator cannot follow is still
/// reported at its nearest container.
pub(super) fn artifact(text: &str, file: &Path, site: Option<&ReasonSite>) -> Option<SourcePosition> {
    match site? {
        // `serde_json` counts the column in bytes.
        ReasonSite::Position(line, column) => reported_position(text, *line, *column, true),
        ReasonSite::Field(segments) => {
            let loaded = load_yaml_document(text, file).ok()?;
            (0..=segments.len())
                .rev()
                .find_map(|depth| loaded.byte_range(&locating_problem(&segments[..depth])))
                .map(|range| line_column(text, range.start))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The line and column of the locators under test.
    fn file_reference(text: &str, file: &Path, reference: &str, carriers: &[&str]) -> Option<(usize, usize)> {
        super::file_reference(text, file, reference, carriers).map(|at| (at.line, at.column))
    }

    fn artifact(text: &str, file: &Path, site: Option<&ReasonSite>) -> Option<(usize, usize)> {
        super::artifact(text, file, site).map(|at| (at.line, at.column))
    }

    #[test]
    fn a_reference_is_found_in_its_declaration_and_never_in_prose_or_a_comment() {
        let file = Path::new("x.yaml");
        let text = "# uses ./x.yaml\nkind: schema-trigger\ndescription: see ./x.yaml\n$schema:\n    a: thing@./x.yaml\n";
        assert_eq!(file_reference(text, file, "./x.yaml", PAYLOAD), Some((5, 14)));
        let bare = "description: see ./x.yaml\n$schema: ./x.yaml\n";
        assert_eq!(file_reference(bare, file, "./x.yaml", PAYLOAD), Some((2, 10)));
        let example = "description: see ./x.yaml\ntypes:\n    a: string\n    b: string(example(./x.yaml))\n";
        assert_eq!(file_reference(example, file, "./x.yaml", TYPES), Some((4, 23)));
        assert_eq!(file_reference(text, file, "./absent.yaml", PAYLOAD), Some((5, 5)), "unspelled: the carrier");
        assert_eq!(file_reference("description: ./x.yaml\n", file, "./x.yaml", PAYLOAD), None, "prose alone");
        assert_eq!(file_reference(text, file, " ", PAYLOAD), None);
    }

    #[test]
    fn a_reference_is_found_past_a_sibling_s_description_and_never_in_one() {
        let file = Path::new("x.yaml");
        let sibling = "types:\n    a: 'string -> Read ./x.yaml.'\n";
        let at = |declaration: &str| file_reference(&format!("{sibling}{declaration}"), file, "./x.yaml", TYPES);
        assert_eq!(at("    b: Options@./x.yaml\n"), Some((3, 16)), "an import");
        assert_eq!(at("    b: string(example(./x.yaml))\n"), Some((3, 23)), "an example");
        assert_eq!(at("    b: string(example(./y.yaml, ./x.yaml))\n"), Some((3, 33)), "a later example argument");
        assert_eq!(at("    b: '{ c: string -> from ./x.yaml, d: D@./x.yaml }'\n"), Some((3, 44)), "an inline object's member");
        assert_eq!(at("    b: \"string(example(./x.yaml)) -> Read ./x.yaml.\"\n"), Some((3, 24)), "an example ahead of its own prose");
        assert_eq!(at("    b: Options@./x.yaml.bak\n"), Some((2, 5)), "a longer path: the carrier");
        assert_eq!(at("    b: string\n"), Some((2, 5)), "prose alone: the carrier");
        let extends = "$schema:\n    - a: 'string -> Read ./x.yaml.'\n      $extends: Options@./x.yaml\n";
        assert_eq!(file_reference(extends, file, "./x.yaml", PAYLOAD), Some((3, 25)), "a `$extends` operand");
    }

    #[test]
    fn an_artifact_field_is_located_as_deep_as_it_exists() {
        let text = "{\n  \"a\": {\n    \"b\": 1\n  }\n}\n";
        let file = Path::new("x.json");
        let at = |segments: &[&str]| artifact(text, file, Some(&ReasonSite::Field(segments.iter().map(|s| (*s).to_string()).collect())));
        assert_eq!(at(&["a", "b"]), Some((3, 10)));
        assert_eq!(at(&["a", "missing"]), Some((2, 8)), "the nearest container");
        assert_eq!(artifact(text, file, Some(&ReasonSite::Position(4, 2))), Some((4, 2)));
        assert_eq!(artifact(text, file, None), None);
    }
}
