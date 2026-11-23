//! The authored node a resolver failure is about, in a whole YAML document
//! that declares schemas under `$schema` and `types`.
//!
//! A resolver records where a failing reference was written as a
//! [`SchemaReferenceSite`]: the source that wrote it and a structural path
//! relative to the declaration (a `$schema` payload or one `types` entry),
//! which does not say which of the two carriers holds it. These functions
//! put the carrier back, follow every union-arm and sequence index, and
//! never report a same-named declaration in the other carrier, or an
//! enclosing node that is not on the reference's path.

use crate::simplified::grammar::without_descriptions;
use crate::{SchemaError, SchemaReferenceSite, SchemaSourcePath, SchemaSourcePathSegment};

use super::load::{LoadedYaml, locating_problem, pointer_string};

/// The reference site `error` carries, for the resolver failures that
/// record one, and the type name the site spells when the failure is about
/// a named type (a file reference's spelling may have been normalized, so
/// it names none).
#[must_use]
pub fn reference_site(error: &SchemaError) -> Option<(&SchemaReferenceSite, Option<&str>)> {
    match error {
        SchemaError::UnknownNamedType { site, name, .. }
        | SchemaError::ConstraintConflict { site, name, .. }
        | SchemaError::UnionReference { site, name, .. }
        | SchemaError::InvalidCompositionTarget { site, name, .. } => Some((site, Some(name))),
        _ => error.reference_site().map(|site| (site, None)),
    }
}

/// A schema source path as instance-path segments: a property is its name,
/// a union arm its index (a root-union arm and a `$extends` item are
/// sequence items). A spread is text inside its object's scalar, not a YAML
/// node, so the path ends at the scalar that holds it.
#[must_use]
pub fn source_path_segments(path: &SchemaSourcePath) -> Vec<String> {
    path.segments()
        .iter()
        .map_while(|segment| match segment {
            SchemaSourcePathSegment::Property(name) => Some(name.clone()),
            SchemaSourcePathSegment::UnionArm(index) => Some(index.to_string()),
            SchemaSourcePathSegment::Spread(_) => None,
        })
        .collect()
}

/// The instance-path segments of the declaration a kind claim `problem` is
/// about: its `$schema` property, or its root-union arm. Every consumer that
/// places a claim failure (the whole-YAML check, a host's registration)
/// reads it from here.
#[must_use]
pub fn claim_segments(problem: &crate::ClaimProblem) -> Vec<String> {
    std::iter::once("$schema".to_string()).chain(source_path_segments(&problem.path)).collect()
}

/// The instance-path segments of the declaration at `path` in `loaded` (whose
/// original text is `text`), under the first of `carriers` that holds it.
///
/// `carriers` lists the top-level keys the path may be relative to, most
/// likely first. A node that exists at the whole path is preferred when its
/// authored text outside any `-> description` contains `spelled` (the name
/// the failing reference wrote), so a same-named declaration in the other
/// carrier, or one whose prose mentions the name, never stands in for it;
/// then any node that exists at the whole path, in carrier order. A path
/// that already starts with one of `carriers` the document has (the
/// resolver writes a payload's private `types` as `types.<name>`) is
/// followed from the document's root when no carrier holds it relatively.
/// When the whole path exists in no carrier, the deepest node on it is
/// returned, under the carrier it names or else the first carrier the
/// document has: the nearest enclosing declaration that is actually on the
/// path, the carrier itself at worst. `None` when the document has none of
/// `carriers`.
#[must_use]
pub fn declaration_segments(
    loaded: &LoadedYaml,
    text: &str,
    path: &SchemaSourcePath,
    carriers: &[&str],
    spelled: Option<&str>,
) -> Option<Vec<String>> {
    let relative = source_path_segments(path);
    let under = |carrier: &str| -> Vec<String> { std::iter::once(carrier.to_string()).chain(relative.iter().cloned()).collect() };
    let exists = |segments: &[String]| loaded.json.pointer(&pointer_string(segments)).is_some();
    let spells = |segments: &[String]| {
        spelled.is_none_or(|name| {
            loaded
                .byte_range(&locating_problem(segments))
                .and_then(|range| text.get(range))
                .is_some_and(|node| without_descriptions(node).contains(name))
        })
    };
    let whole: Vec<Vec<String>> = carriers.iter().map(|carrier| under(carrier)).filter(|segments| exists(segments)).collect();
    if let Some(found) = whole.iter().find(|segments| spells(segments)).or_else(|| whole.first()) {
        return Some(found.clone());
    }
    // A path that starts with its carrier (a payload's private `types`, or
    // the `$schema` beside them) is already whole.
    let full = match relative.first() {
        Some(first) if carriers.contains(&first.as_str()) && exists(std::slice::from_ref(first)) => relative,
        _ => under(carriers.iter().find(|carrier| exists(&[(*carrier).to_string()]))?),
    };
    (1..=full.len()).rev().map(|depth| full[..depth].to_vec()).find(|segments| exists(segments))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::yaml_documents::load_yaml_document;

    const TEXT: &str = "types:\n    count: number\n$schema:\n    count:\n        - number\n        - Missing\n    label: string\n";

    fn segments(path: &SchemaSourcePath, carriers: &[&str], spelled: Option<&str>) -> Option<Vec<String>> {
        let loaded = load_yaml_document(TEXT, Path::new("x.yaml")).unwrap();
        declaration_segments(&loaded, TEXT, path, carriers, spelled)
    }

    fn owned(segments: &[&str]) -> Option<Vec<String>> {
        Some(segments.iter().map(|segment| (*segment).to_string()).collect())
    }

    #[test]
    fn a_union_arm_is_followed_as_a_sequence_index() {
        let path = SchemaSourcePath::root().property("count").union_arm(1);
        assert_eq!(segments(&path, &["types", "$schema"], Some("Missing")), owned(&["$schema", "count", "1"]));
    }

    #[test]
    fn the_spelled_name_picks_the_carrier_over_the_order() {
        let path = SchemaSourcePath::root().property("count");
        assert_eq!(segments(&path, &["types", "$schema"], Some("Missing")), owned(&["$schema", "count"]));
        assert_eq!(segments(&path, &["types", "$schema"], None), owned(&["types", "count"]), "carrier order without a name");
    }

    #[test]
    fn a_name_in_a_description_does_not_pick_the_carrier() {
        let text = "types:\n    count: \"number -> Not Missing here.\"\n$schema:\n    count: Missing\n";
        let loaded = load_yaml_document(text, Path::new("x.yaml")).unwrap();
        let path = SchemaSourcePath::root().property("count");
        let found = declaration_segments(&loaded, text, &path, &["types", "$schema"], Some("Missing"));
        assert_eq!(found, owned(&["$schema", "count"]));
    }

    #[test]
    fn a_path_that_names_its_carrier_is_followed_from_the_root() {
        let path = SchemaSourcePath::root().property("types").property("count");
        assert_eq!(segments(&path, &["$schema", "types"], None), owned(&["types", "count"]));
        let deeper = SchemaSourcePath::root().property("$schema").property("count").union_arm(1).property("absent");
        assert_eq!(segments(&deeper, &["$schema", "types"], None), owned(&["$schema", "count", "1"]), "the nearest node on it");
    }

    #[test]
    fn a_path_found_nowhere_falls_back_to_the_nearest_node_on_it() {
        let path = SchemaSourcePath::root().property("count").union_arm(5);
        assert_eq!(segments(&path, &["$schema", "types"], Some("Missing")), owned(&["$schema", "count"]));
        let path = SchemaSourcePath::root().property("absent");
        assert_eq!(segments(&path, &["$schema", "types"], None), owned(&["$schema"]));
        assert_eq!(segments(&path, &["match"], None), None);
    }
}
