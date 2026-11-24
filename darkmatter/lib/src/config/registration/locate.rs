//! Where a refused member's problem is: the file and 1-based line and
//! column a registration warning reports, recovered from the typed error
//! before it becomes the warning's message.
//!
//! The rule is the most specific location the error itself records. An
//! error raised in a file another member imports is reported in that file,
//! at the declaration that failed; a declaration inside a union arm at that
//! arm, never another arm or a same-named declaration under the other of
//! `$schema` and `types`; a grammar failure at the parser's rejected token;
//! a missing field at the mapping that lacks it; a file reference that
//! cannot be followed (an import, an `example(...)`, a `$schema` file) at
//! the declaration that wrote it, in the file that wrote it, as the
//! resolver recorded them ([`SchemaError::reference_site`]); so is a file
//! that was found but refused for the role the declaration named it in (a
//! types library, YAML kind, namespace, or trigger named as a document
//! schema or import source, a file closing a reference cycle), also once
//! that refusal is reported as a trigger payload cycle. A file that does
//! not parse is reported at its own parse position. A position is never
//! recovered by searching the text: a reference error that records no
//! declaration is reported at the file alone (a refused file at that
//! file).

use std::path::{Path, PathBuf};

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
        // A file refused for the role a declaration named it in is reported
        // at that declaration; one refused for its own content, at itself.
        SchemaError::NotASchemaDocument { path, .. } | SchemaError::UnsupportedJsonDialect { path, .. } => {
            match error.reference_site() {
                Some(site) => reference_site(site, None, file, texts, carriers),
                None => Site::unlocated(path),
            }
        }
        SchemaError::InvalidJsonSchema { path, .. } => Site::unlocated(path),
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
        SchemaError::BundleBoundary { .. }
        | SchemaError::NoMatch { .. }
        | SchemaError::Unresolved { .. }
        | SchemaError::RemoteUnsupported { .. }
        | SchemaError::BareNameSiblingExists { .. }
        | SchemaError::InvalidExample { .. }
        | SchemaError::TriggerSchemaReferenced { .. }
        | SchemaError::TriggerPayloadCycle { .. }
        | SchemaError::ReferenceCycle { .. }
        | SchemaError::ReferenceDepthExceeded { .. } => match error.reference_site() {
            Some(site) => reference_site(site, None, file, texts, carriers),
            None => here(None),
        },
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

/// The value at `segments` of the YAML (or JSON) `text` read from `file`.
pub(super) fn field(text: &str, file: &Path, segments: &[String]) -> Option<SourcePosition> {
    let loaded = load_yaml_document(text, file).ok()?;
    loaded.byte_range(&locating_problem(segments)).map(|range| line_column(text, range.start))
}

/// Where an artifact reader's `site` is in the JSON `text`: the parser's
/// position, or the deepest existing value along the field's JSON pointer,
/// so a field the reader found in a shape the locator cannot follow is still
/// reported at its nearest container.
pub(super) fn artifact(text: &str, site: Option<&ReasonSite>) -> Option<SourcePosition> {
    match site? {
        // `serde_json` counts the column in bytes.
        ReasonSite::Position(line, column) => reported_position(text, *line, *column, true),
        ReasonSite::Field(segments) => (0..=segments.len())
            .rev()
            .find_map(|depth| biscuit_file::data_locations::locate_json(text, &artifact_pointer(&segments[..depth])))
            .map(|offset| line_column(text, offset)),
    }
}

/// `segments` as an RFC 6901 JSON pointer.
pub(super) fn artifact_pointer(segments: &[String]) -> String {
    segments.iter().map(|segment| format!("/{}", segment.replace('~', "~0").replace('/', "~1"))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // The line and column of the locator under test.
    fn artifact(text: &str, site: Option<&ReasonSite>) -> Option<(usize, usize)> {
        super::artifact(text, site).map(|at| (at.line, at.column))
    }

    #[test]
    fn an_artifact_field_is_located_as_deep_as_it_exists() {
        let text = "{\n  \"a\": {\n    \"b\": 1\n  }\n}\n";
        let at = |segments: &[&str]| artifact(text, Some(&ReasonSite::Field(segments.iter().map(|s| (*s).to_string()).collect())));
        assert_eq!(at(&["a", "b"]), Some((3, 10)));
        assert_eq!(at(&["a", "missing"]), Some((2, 8)), "the nearest container");
        assert_eq!(at(&[]), Some((1, 1)), "the root");
        assert_eq!(artifact(text, Some(&ReasonSite::Position(4, 2))), Some((4, 2)));
        assert_eq!(artifact(text, None), None);
    }
}
