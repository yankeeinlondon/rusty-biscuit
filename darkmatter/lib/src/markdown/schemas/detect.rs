//! Schema detection — infer a [`SimplifiedSchema`] from one or more documents.
//!
//! The single-file algorithm walks a document's frontmatter and produces a
//! [`SchemaShape`] of base types (no constraints are inferred; see the
//! spec). The multi-file merge step widens disagreeing types along a fixed
//! supertype hierarchy and only marks a property `required` when it is
//! present in every input file.
//!
//! See `darkmatter/features/2026-05-11-schemas/spec.md` § "Schema
//! Detection" for the authoritative behaviour.

use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileResolutionContext};
use indexmap::IndexMap;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::Value;
use url::Url;

use super::simplified::{
    Constraint, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema, SimplifiedType, TypeExpr,
};
use crate::markdown::Markdown;
use crate::markdown::compose::ComposeSource;

/// Options controlling detection behaviour.
#[derive(Debug, Clone, Copy, Default)]
pub struct DetectOptions {
    /// When true, multiple sources are merged with type widening and shared
    /// properties may be promoted to `required`. When false, the result is
    /// the union of detected properties from each source with `required`
    /// never inferred.
    pub merge: bool,
}

/// Detects a [`SimplifiedSchema`] from one or more Markdown documents.
///
/// Empty `sources` returns an empty single-shape schema. When
/// `opts.merge` is false (or only one source is supplied), the per-file
/// shapes are unioned by property name without `required` promotion. When
/// `opts.merge` is true, types are widened along the spec's hierarchy and a
/// property is `required` only if it appears in every source.
pub fn detect_schema(sources: &[&Markdown], opts: DetectOptions) -> SimplifiedSchema {
    let contexts: Vec<FileResolutionContext> = sources
        .iter()
        .map(|md| compatibility_detection_context(md))
        .collect();
    detect_schema_with_contexts(sources, opts, &contexts)
}

/// Detects a schema using one explicit resolution snapshot per source.
///
/// The contexts and sources must have the same length. Each context is derived
/// to its corresponding document before `file` inference, and resolution uses
/// the shared detailed candidate/probe contract without ambient reads.
///
/// ## Panics
///
/// Panics when `contexts.len() != sources.len()`.
pub fn detect_schema_with_contexts(
    sources: &[&Markdown],
    opts: DetectOptions,
    contexts: &[FileResolutionContext],
) -> SimplifiedSchema {
    assert_eq!(sources.len(), contexts.len(), "one resolution context is required per source");
    if sources.is_empty() {
        return SimplifiedSchema::Single(SchemaShape::default());
    }

    let shapes: Vec<SchemaShape> = sources
        .iter()
        .zip(contexts)
        .map(|(md, context)| detect_from_document_with_context(md, context))
        .collect();

    if shapes.len() == 1 {
        return SimplifiedSchema::Single(shapes.into_iter().next().unwrap());
    }

    let merged = if opts.merge {
        merge_shapes_widening(&shapes)
    } else {
        union_shapes_no_promote(&shapes)
    };
    SimplifiedSchema::Single(merged)
}

/// Detects a [`SchemaShape`] from a single document's frontmatter.
///
/// `$schema` is skipped because it is reserved by Darkmatter; all other
/// top-level keys are mapped to base types.
pub fn detect_from_document(md: &Markdown) -> SchemaShape {
    let context = compatibility_detection_context(md);
    detect_from_document_with_context(md, &context)
}

/// Detects one document's schema using an explicit resolution snapshot.
pub fn detect_from_document_with_context(
    md: &Markdown,
    request_context: &FileResolutionContext,
) -> SchemaShape {
    let context = match md.source() {
        Some(ComposeSource::File(path)) => request_context.for_source(path),
        _ => request_context.for_base(request_context.base_dir()),
    };
    let mut properties: IndexMap<String, PropertyDef> = IndexMap::new();
    for (key, value) in md.frontmatter().as_map() {
        if key == "$schema" {
            continue;
        }
        let atom = detect_value_atom(value, &context);
        properties.insert(key.clone(), PropertyDef::Single(atom));
    }
    SchemaShape {
        properties,
        ..Default::default()
    }
}

/// Compatibility policy for the context-free detection API.
///
/// Detection is an authoring heuristic, so invalid references and probe errors
/// classify as strings rather than becoming schema-detection failures. The
/// legacy entry points capture their CWD/environment inputs once here, then use
/// the same repository-first detailed resolver as explicit callers.
fn compatibility_detection_context(md: &Markdown) -> FileResolutionContext {
    let base_dir = base_dir_for(md);
    let repository_root = crate::markdown::compose::find_git_root_from(&base_dir);
    crate::markdown::compose::document_resolution_context(
        &base_dir,
        match md.source() {
            Some(ComposeSource::File(path)) => Some(path.as_path()),
            _ => None,
        },
        &[],
        repository_root.as_deref(),
        None,
    )
}

fn base_dir_for(md: &Markdown) -> PathBuf {
    match md.source() {
        Some(ComposeSource::File(path)) => path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")),
        _ => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    }
}

fn detect_value_atom(value: &Value, context: &FileResolutionContext) -> PropertyAtom {
    match value {
        Value::Bool(_) => PropertyAtom::bare(SimplifiedType::Boolean),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                PropertyAtom {
                    ty: TypeExpr::Primitive(SimplifiedType::Number),
                    is_array: false,
                    constraints: vec![Constraint::Integer],
                    array_constraints: vec![],
                    description: None,
                }
            } else {
                PropertyAtom::bare(SimplifiedType::Number)
            }
        }
        Value::String(s) => PropertyAtom::bare(classify_string(s, context)),
        Value::Array(items) => detect_array_atom(items, context),
        Value::Object(_) => PropertyAtom::bare(SimplifiedType::Object),
        Value::Null => PropertyAtom::bare(SimplifiedType::Any),
    }
}

fn detect_array_atom(items: &[Value], context: &FileResolutionContext) -> PropertyAtom {
    if items.is_empty() {
        return PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Any),
            is_array: true,
            constraints: vec![],
            array_constraints: vec![],
            description: None,
        };
    }

    let mut iter = items.iter();
    let first = detect_value_atom(iter.next().unwrap(), context);
    let mut item_ty = first.ty;
    let mut item_constraints = first.constraints;
    for v in iter {
        let next = detect_value_atom(v, context);
        match unify_types(&item_ty, &next.ty) {
            Some(t) => {
                if t != SimplifiedType::Number {
                    item_constraints.clear();
                }
                if t == SimplifiedType::Number
                    && !(item_constraints.contains(&Constraint::Integer)
                        && next.constraints.contains(&Constraint::Integer))
                {
                    item_constraints.clear();
                }
                item_ty = TypeExpr::Primitive(t);
            }
            None => {
                return PropertyAtom {
                    ty: TypeExpr::Primitive(SimplifiedType::Any),
                    is_array: true,
                    constraints: vec![],
                    array_constraints: vec![],
                    description: None,
                };
            }
        }
    }

    PropertyAtom {
        ty: item_ty,
        is_array: true,
        constraints: item_constraints,
        array_constraints: vec![],
        description: None,
    }
}

fn classify_string(value: &str, context: &FileResolutionContext) -> SimplifiedType {
    if is_date(value) {
        return SimplifiedType::Date;
    }
    if is_datetime(value) {
        return SimplifiedType::DateTime;
    }
    if is_time(value) {
        return SimplifiedType::Time;
    }
    if is_email(value) {
        return SimplifiedType::Email;
    }
    if is_url(value) {
        return SimplifiedType::Url;
    }
    if resolves_to_existing_file(value, context) {
        return SimplifiedType::File;
    }
    SimplifiedType::String
}

lazy_static! {
    static ref DATE_RE: Regex = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    static ref DATETIME_RE: Regex =
        Regex::new(r"^\d{4}-\d{2}-\d{2}[Tt ]\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:?\d{2})?$")
            .unwrap();
    static ref TIME_RE: Regex =
        Regex::new(r"^\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:?\d{2})?$").unwrap();
    // Tight addr-spec — local@domain with at least one dot in the domain.
    static ref EMAIL_RE: Regex =
        Regex::new(r"^[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}$").unwrap();
}

fn is_date(s: &str) -> bool {
    DATE_RE.is_match(s)
}

fn is_datetime(s: &str) -> bool {
    DATETIME_RE.is_match(s)
}

fn is_time(s: &str) -> bool {
    TIME_RE.is_match(s)
}

fn is_email(s: &str) -> bool {
    EMAIL_RE.is_match(s)
}

fn is_url(s: &str) -> bool {
    match Url::parse(s) {
        Ok(u) => u.has_host() || u.scheme() == "file",
        Err(_) => false,
    }
}

/// File inference is deliberately best-effort: an invalid reference, no match,
/// or probe failure remains a `string` because detection has no error channel.
/// Candidate order and ambient-state policy still come from the shared detailed
/// resolver rather than a manual join or existence check.
fn resolves_to_existing_file(value: &str, context: &FileResolutionContext) -> bool {
    // Avoid expensive resolution for short / unlikely paths. We require at
    // least one path-like character.
    if !value.chars().any(|c| c == '/' || c == '.' || c == '\\') {
        return false;
    }
    let Ok(reference) = FileReference::new(value) else {
        return false;
    };
    reference.resolve_detailed(context).matched_path().is_some()
}

// ── Multi-file merging ────────────────────────────────────────────────────

fn union_shapes_no_promote(shapes: &[SchemaShape]) -> SchemaShape {
    let mut properties: IndexMap<String, PropertyDef> = IndexMap::new();
    for shape in shapes {
        for (name, def) in &shape.properties {
            if let Some(existing) = properties.get_mut(name) {
                *existing = merge_defs(existing.clone(), def.clone(), false);
            } else {
                properties.insert(name.clone(), strip_required(def.clone()));
            }
        }
    }
    SchemaShape {
        properties,
        ..Default::default()
    }
}

fn merge_shapes_widening(shapes: &[SchemaShape]) -> SchemaShape {
    let total = shapes.len();
    let mut counts: IndexMap<String, usize> = IndexMap::new();
    let mut acc: IndexMap<String, PropertyDef> = IndexMap::new();

    for shape in shapes {
        for (name, def) in &shape.properties {
            *counts.entry(name.clone()).or_insert(0) += 1;
            if let Some(existing) = acc.get_mut(name) {
                *existing = merge_defs(existing.clone(), def.clone(), false);
            } else {
                acc.insert(name.clone(), strip_required(def.clone()));
            }
        }
    }

    // Promote `required` for properties present in every input.
    for (name, count) in &counts {
        if *count == total
            && let Some(def) = acc.get_mut(name)
        {
            mark_required(def);
        }
    }

    SchemaShape {
        properties: acc,
        ..Default::default()
    }
}

/// Walk both sides; widen single+single via the type hierarchy, fall back to
/// a `PropertyDef::Union` of distinct atoms when widening fails.
fn merge_defs(left: PropertyDef, right: PropertyDef, _: bool) -> PropertyDef {
    let left_atoms = atoms_of(left);
    let right_atoms = atoms_of(right);

    let mut combined: Vec<PropertyAtom> = left_atoms;
    for atom in right_atoms {
        absorb_atom(&mut combined, atom);
    }

    if combined.len() == 1 {
        PropertyDef::Single(combined.into_iter().next().unwrap())
    } else {
        PropertyDef::Union(combined)
    }
}

fn atoms_of(def: PropertyDef) -> Vec<PropertyAtom> {
    match def {
        PropertyDef::Single(atom) => vec![atom],
        PropertyDef::Union(atoms) => atoms,
    }
}

/// Absorb `atom` into `combined`, widening with the first compatible existing
/// atom or appending it as a new arm if nothing matches.
fn absorb_atom(combined: &mut Vec<PropertyAtom>, atom: PropertyAtom) {
    for slot in combined.iter_mut() {
        if let Some(widened) = widen_atoms(slot.clone(), atom.clone()) {
            *slot = widened;
            return;
        }
    }
    combined.push(atom);
}

fn widen_atoms(left: PropertyAtom, right: PropertyAtom) -> Option<PropertyAtom> {
    if left.is_array != right.is_array {
        return None;
    }
    let ty = unify_types(&left.ty, &right.ty)?;
    let constraints = unify_atom_constraints(ty, &left, &right);
    Some(PropertyAtom {
        ty: TypeExpr::Primitive(ty),
        is_array: left.is_array,
        constraints,
        array_constraints: vec![],
        description: None,
    })
}

fn unify_atom_constraints(
    ty: SimplifiedType,
    left: &PropertyAtom,
    right: &PropertyAtom,
) -> Vec<Constraint> {
    if ty == SimplifiedType::Number {
        let both_int = left.constraints.contains(&Constraint::Integer)
            && right.constraints.contains(&Constraint::Integer);
        if both_int {
            return vec![Constraint::Integer];
        }
    }
    Vec::new()
}

/// Returns the unified type for `a` and `b` if they share a common ancestor
/// along the widening hierarchy. Returns `None` when the two types are
/// genuinely disjoint.
fn unify_types(a: &TypeExpr, b: &TypeExpr) -> Option<SimplifiedType> {
    let a = primitive(a)?;
    let b = primitive(b)?;
    use SimplifiedType::*;
    if a == b {
        return Some(a);
    }
    let pair = (a, b);
    match pair {
        // string-family widening
        (String, Date | Time | DateTime | Url | Email | File)
        | (Date | Time | DateTime | Url | Email | File, String) => Some(String),

        (Date | Time | DateTime | Url | Email, Date | Time | DateTime | Url | Email) => {
            Some(String)
        }

        // file & a string-family type → file is just a tagged string
        (File, Date | Time | DateTime | Url | Email)
        | (Date | Time | DateTime | Url | Email, File) => Some(String),

        // number-family widening
        (Number, NumberLike) | (NumberLike, Number) => Some(NumberLike),

        // boolean-family widening
        (Boolean, Boolish) | (Boolish, Boolean) => Some(Boolish),

        // `Any` swallows everything
        (Any, t) | (t, Any) => Some(t),

        _ => None,
    }
}

fn primitive(ty: &TypeExpr) -> Option<SimplifiedType> {
    match ty {
        TypeExpr::Primitive(p) => Some(*p),
        // Inline objects and imported types do not unify with any other type
        // in detection (detecting either is a non-goal of this feature).
        TypeExpr::InlineObject(_) | TypeExpr::Imported { .. } => None,
    }
}

fn strip_required(def: PropertyDef) -> PropertyDef {
    match def {
        PropertyDef::Single(mut atom) => {
            atom.constraints
                .retain(|c| !matches!(c, Constraint::Required));
            PropertyDef::Single(atom)
        }
        PropertyDef::Union(mut arms) => {
            for atom in &mut arms {
                atom.constraints
                    .retain(|c| !matches!(c, Constraint::Required));
            }
            PropertyDef::Union(arms)
        }
    }
}

fn mark_required(def: &mut PropertyDef) {
    match def {
        PropertyDef::Single(atom) => {
            if !atom.constraints.contains(&Constraint::Required) {
                atom.constraints.insert(0, Constraint::Required);
            }
        }
        PropertyDef::Union(arms) => {
            if let Some(first) = arms.first_mut()
                && !first.constraints.contains(&Constraint::Required)
            {
                first.constraints.insert(0, Constraint::Required);
            }
        }
    }
}

// ── YAML serialisation of a detected SimplifiedSchema ─────────────────────

/// Serialises a [`SimplifiedSchema`] as a `$schema:` YAML mapping, matching
/// the shape consumers can paste back into a Markdown frontmatter.
///
/// Property values are emitted as quoted type-and-constraint strings when
/// they contain characters that YAML would otherwise interpret.
pub fn schema_to_yaml(schema: &SimplifiedSchema) -> String {
    let mut out = String::from("$schema:\n");
    match schema {
        SimplifiedSchema::Single(shape) => {
            write_shape(&mut out, shape, 1);
        }
        SimplifiedSchema::Union(arms) => {
            for arm in arms {
                match arm {
                    super::simplified::SchemaArm::Inline(shape) => {
                        write_shape_list_item(&mut out, shape, 1);
                    }
                    super::simplified::SchemaArm::FileRef(path) => {
                        out.push_str(&format!("  - {path}\n"));
                    }
                }
            }
        }
    }
    out
}

fn write_shape(out: &mut String, shape: &SchemaShape, indent_levels: usize) {
    let indent = "  ".repeat(indent_levels);
    for (name, def) in &shape.properties {
        let value = property_def_to_yaml_scalar(def);
        out.push_str(&format!("{indent}{name}: {value}\n"));
    }
}

/// Emits `shape` as a YAML list item:
///
/// ```yaml
/// - first: value
///   second: value
/// ```
///
/// `indent_levels` is the number of two-space steps preceding the `- ` marker.
/// Empty shapes produce an `- {}` entry so the surrounding sequence remains
/// well-formed.
fn write_shape_list_item(out: &mut String, shape: &SchemaShape, indent_levels: usize) {
    let marker_indent = "  ".repeat(indent_levels);
    let body_indent = format!("{marker_indent}  ");
    let mut iter = shape.properties.iter();
    let Some((first_name, first_def)) = iter.next() else {
        out.push_str(&format!("{marker_indent}- {{}}\n"));
        return;
    };
    let first_value = property_def_to_yaml_scalar(first_def);
    out.push_str(&format!("{marker_indent}- {first_name}: {first_value}\n"));
    for (name, def) in iter {
        let value = property_def_to_yaml_scalar(def);
        out.push_str(&format!("{body_indent}{name}: {value}\n"));
    }
}

fn property_def_to_yaml_scalar(def: &PropertyDef) -> String {
    match def {
        PropertyDef::Single(atom) => {
            quote_if_needed(&super::simplified::serialize_property_atom(atom))
        }
        PropertyDef::Union(arms) => {
            // Inline flow sequence keeps the YAML compact and easy to read.
            let pieces: Vec<String> = arms
                .iter()
                .map(|a| quote_if_needed(&super::simplified::serialize_property_atom(a)))
                .collect();
            format!("[{}]", pieces.join(", "))
        }
    }
}

fn quote_if_needed(s: &str) -> String {
    let needs_quote = s.is_empty()
        || starts_with_yaml_indicator(s)
        || contains_yaml_breaker(s)
        || is_yaml_reserved_literal(s);
    if needs_quote {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

fn starts_with_yaml_indicator(s: &str) -> bool {
    matches!(
        s.chars().next(),
        Some('[' | '{' | '|' | '>' | '\'' | '"' | '%' | '@' | '`' | '!' | '&' | '*' | '?' | ',')
    ) || s.starts_with("- ")
}

fn contains_yaml_breaker(s: &str) -> bool {
    s.contains(": ") || s.contains(" #") || s.contains('\n') || s.contains('\t')
}

fn is_yaml_reserved_literal(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "true" | "false" | "null" | "yes" | "no" | "on" | "off" | "~"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md_with_frontmatter(yaml_body: &str) -> Markdown {
        let content = format!("---\n{yaml_body}---\nbody\n");
        content.as_str().into()
    }

    #[test]
    fn detects_simple_scalars() {
        let md = md_with_frontmatter("title: Hello\nactive: true\ncount: 42\nrating: 3.14\n");
        let schema = detect_schema(&[&md], DetectOptions::default());
        let SimplifiedSchema::Single(shape) = schema else {
            panic!("expected Single");
        };
        let kinds: Vec<(&str, TypeExpr, bool)> = shape
            .properties
            .iter()
            .map(|(k, def)| match def {
                PropertyDef::Single(a) => (
                    k.as_str(),
                    a.ty.clone(),
                    a.constraints.contains(&Constraint::Integer),
                ),
                _ => panic!("unexpected union"),
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("title", TypeExpr::Primitive(SimplifiedType::String), false),
                (
                    "active",
                    TypeExpr::Primitive(SimplifiedType::Boolean),
                    false
                ),
                ("count", TypeExpr::Primitive(SimplifiedType::Number), true),
                (
                    "rating",
                    TypeExpr::Primitive(SimplifiedType::Number),
                    false
                ),
            ]
        );
    }

    #[test]
    fn detects_date_datetime_time() {
        let md = md_with_frontmatter(
            "published: '2026-05-11'\nwhen: '2026-05-11T10:00:00Z'\nstart: '10:30'\n",
        );
        let schema = detect_schema(&[&md], DetectOptions::default());
        let SimplifiedSchema::Single(shape) = schema else {
            panic!();
        };
        let types: Vec<TypeExpr> = shape
            .properties
            .values()
            .map(|d| match d {
                PropertyDef::Single(a) => a.ty.clone(),
                _ => panic!(),
            })
            .collect();
        assert_eq!(
            types,
            vec![
                TypeExpr::Primitive(SimplifiedType::Date),
                TypeExpr::Primitive(SimplifiedType::DateTime),
                TypeExpr::Primitive(SimplifiedType::Time),
            ]
        );
    }

    #[test]
    fn detects_url_and_email() {
        let md = md_with_frontmatter("homepage: https://example.com\nauthor: alice@example.com\n");
        let schema = detect_schema(&[&md], DetectOptions::default());
        let SimplifiedSchema::Single(shape) = schema else {
            panic!();
        };
        let types: Vec<TypeExpr> = shape
            .properties
            .values()
            .map(|d| match d {
                PropertyDef::Single(a) => a.ty.clone(),
                _ => panic!(),
            })
            .collect();
        assert_eq!(
            types,
            vec![
                TypeExpr::Primitive(SimplifiedType::Url),
                TypeExpr::Primitive(SimplifiedType::Email),
            ]
        );
    }

    #[test]
    fn explicit_context_detection_uses_repository_first_candidates() {
        let repo = tempfile::TempDir::new().unwrap();
        let docs = repo.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(repo.path().join("asset.txt"), "root").unwrap();
        let source = docs.join("page.md");
        let md = md_with_frontmatter("asset: asset.txt\n")
            .with_source(ComposeSource::File(source));
        let context = FileResolutionContext::from_snapshot(
            repo.path(),
            None,
            std::collections::HashMap::new(),
        )
        .with_repository_root(repo.path());

        let shape = detect_from_document_with_context(&md, &context);
        let PropertyDef::Single(asset) = shape.properties.get("asset").unwrap() else {
            panic!("expected one detected file property");
        };
        assert_eq!(asset.ty, TypeExpr::Primitive(SimplifiedType::File));
    }

    #[test]
    fn explicit_context_detection_keeps_invalid_reference_as_string() {
        let context = FileResolutionContext::from_snapshot(
            "/nonexistent/darkmatter-detection",
            None,
            std::collections::HashMap::new(),
        );
        let md = md_with_frontmatter("asset: '{{}}'\n");

        let shape = detect_from_document_with_context(&md, &context);
        let PropertyDef::Single(asset) = shape.properties.get("asset").unwrap() else {
            panic!("expected one detected string property");
        };
        assert_eq!(asset.ty, TypeExpr::Primitive(SimplifiedType::String));
    }

    #[test]
    fn skips_reserved_schema_property() {
        let md = md_with_frontmatter("$schema:\n  title: string\ntitle: Hello\n");
        let schema = detect_schema(&[&md], DetectOptions::default());
        let SimplifiedSchema::Single(shape) = schema else {
            panic!();
        };
        assert!(!shape.properties.contains_key("$schema"));
        assert!(shape.properties.contains_key("title"));
    }

    #[test]
    fn array_of_strings_detected() {
        let md = md_with_frontmatter("tags:\n  - a\n  - b\n  - c\n");
        let schema = detect_schema(&[&md], DetectOptions::default());
        let SimplifiedSchema::Single(shape) = schema else {
            panic!();
        };
        let def = shape.properties.get("tags").unwrap();
        match def {
            PropertyDef::Single(atom) => {
                assert!(atom.is_array);
                assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::String));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn array_with_disjoint_items_falls_back_to_any() {
        let md = md_with_frontmatter("mixed:\n  - 1\n  - true\n  - hi\n");
        let schema = detect_schema(&[&md], DetectOptions::default());
        let SimplifiedSchema::Single(shape) = schema else {
            panic!();
        };
        let def = shape.properties.get("mixed").unwrap();
        match def {
            PropertyDef::Single(atom) => {
                assert!(atom.is_array);
                assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::Any));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn merge_widens_number_int_and_number_to_number() {
        let a = md_with_frontmatter("score: 1\n");
        let b = md_with_frontmatter("score: 1.5\n");
        let schema = detect_schema(&[&a, &b], DetectOptions { merge: true });
        let SimplifiedSchema::Single(shape) = schema else {
            panic!();
        };
        let def = shape.properties.get("score").unwrap();
        match def {
            PropertyDef::Single(atom) => {
                assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::Number));
                assert!(!atom.constraints.contains(&Constraint::Integer));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn merge_widens_date_and_string_to_string() {
        let a = md_with_frontmatter("v: '2026-05-11'\n");
        let b = md_with_frontmatter("v: 'plain text'\n");
        let schema = detect_schema(&[&a, &b], DetectOptions { merge: true });
        let SimplifiedSchema::Single(shape) = schema else {
            panic!();
        };
        let def = shape.properties.get("v").unwrap();
        match def {
            PropertyDef::Single(atom) => {
                assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::String))
            }
            _ => panic!(),
        }
    }

    #[test]
    fn merge_produces_union_on_disjoint_types() {
        let a = md_with_frontmatter("flag: true\n");
        let b = md_with_frontmatter("flag: hello\n");
        let schema = detect_schema(&[&a, &b], DetectOptions { merge: true });
        let SimplifiedSchema::Single(shape) = schema else {
            panic!();
        };
        let def = shape.properties.get("flag").unwrap();
        match def {
            PropertyDef::Union(arms) => {
                let types: Vec<_> = arms.iter().map(|a| a.ty.clone()).collect();
                assert!(types.contains(&TypeExpr::Primitive(SimplifiedType::Boolean)));
                assert!(types.contains(&TypeExpr::Primitive(SimplifiedType::String)));
            }
            _ => panic!("expected Union, got {:?}", def),
        }
    }

    #[test]
    fn merge_marks_required_when_present_in_every_file() {
        let a = md_with_frontmatter("title: A\nextra: 1\n");
        let b = md_with_frontmatter("title: B\n");
        let schema = detect_schema(&[&a, &b], DetectOptions { merge: true });
        let SimplifiedSchema::Single(shape) = schema else {
            panic!();
        };
        let title = shape.properties.get("title").unwrap();
        match title {
            PropertyDef::Single(atom) => {
                assert!(atom.constraints.contains(&Constraint::Required));
            }
            _ => panic!(),
        }
        let extra = shape.properties.get("extra").unwrap();
        match extra {
            PropertyDef::Single(atom) => {
                assert!(!atom.constraints.contains(&Constraint::Required));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn no_merge_never_marks_required() {
        let a = md_with_frontmatter("title: A\n");
        let b = md_with_frontmatter("title: B\n");
        let schema = detect_schema(&[&a, &b], DetectOptions { merge: false });
        let SimplifiedSchema::Single(shape) = schema else {
            panic!();
        };
        let title = shape.properties.get("title").unwrap();
        match title {
            PropertyDef::Single(atom) => {
                assert!(!atom.constraints.contains(&Constraint::Required));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn yaml_serialisation_emits_schema_block() {
        let md = md_with_frontmatter("title: Hello\ncount: 1\n");
        let schema = detect_schema(&[&md], DetectOptions::default());
        let yaml = schema_to_yaml(&schema);
        assert!(yaml.starts_with("$schema:"));
        assert!(yaml.contains("title: string"));
        assert!(yaml.contains("count: number(integer)"));
    }
}
