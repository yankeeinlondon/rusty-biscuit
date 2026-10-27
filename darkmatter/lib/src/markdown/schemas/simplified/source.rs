//! Source-aware projection for passive SimplifiedSchema parsers.
//!
//! ## V1 presentation boundary
//!
//! The locator covers the SimplifiedSchema authoring presentations exercised by
//! the shipped schema corpus: plain, single-quoted, and double-quoted scalars,
//! with a plain scalar allowed to continue onto more-indented lines; literal
//! and folded block scalars; block sequences, including one aligned with its
//! parent key, and flow sequences; implicit scalar-key block and flow
//! mappings; explicit scalar-key block mapping pairs in mapping roots or
//! compact sequence items; and the mapping-value anchors and scalar aliases
//! used by that corpus. This is not a general YAML concrete-syntax tree. Other
//! YAML presentations accepted by `serde_yaml_ng` (tags, quoted scalars
//! continued across lines) are outside the v1 source-map contract and return a
//! projection error even when semantic parsing succeeds.

use std::{collections::BTreeMap, ops::Range};

use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;

use super::grammar;
use super::yaml_scalar::{self, DecodedScalar};
use super::{
    Constraint, DefinitionFailure, PropertyAtom, PropertyDef, SchemaArm, SchemaDeclaration,
    SchemaShape, SimplifiedSchema, SimplifiedType, TypeExpr, parse_property_definition_located,
    parse_schema_declaration, parse_schema_declaration_located, parse_yaml_schema,
};

/// One segment in a structural path through a schema declaration.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SchemaSourcePathSegment {
    /// A literal or pattern mapping property.
    Property(String),
    /// An arm of a property-level or root-level union.
    UnionArm(usize),
}

/// A structural path used to query a [`SchemaSourceMap`].
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaSourcePath(Vec<SchemaSourcePathSegment>);

impl SchemaSourcePath {
    /// Returns the declaration or property-definition root path.
    pub fn root() -> Self {
        Self::default()
    }

    /// Returns a child path for a mapping property.
    pub fn property(&self, name: impl Into<String>) -> Self {
        let mut path = self.clone();
        path.0.push(SchemaSourcePathSegment::Property(name.into()));
        path
    }

    /// Returns a child path for a union arm.
    pub fn union_arm(&self, index: usize) -> Self {
        let mut path = self.clone();
        path.0.push(SchemaSourcePathSegment::UnionArm(index));
        path
    }

    /// Borrows the structural segments in this path.
    pub fn segments(&self) -> &[SchemaSourcePathSegment] {
        &self.0
    }

    /// Returns `suffix` addressed from this path: a failure path recorded
    /// relative to a definition becomes a path relative to its envelope by
    /// prefixing (`root().property("types").join(&failure_path)`).
    pub fn join(&self, suffix: &SchemaSourcePath) -> Self {
        let mut path = self.clone();
        path.0.extend(suffix.0.iter().cloned());
        path
    }
}

/// Renders properties joined by `.` and union arms as `[N]`; the root is
/// `<root>`.
impl std::fmt::Display for SchemaSourcePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return f.write_str("<root>");
        }
        for (index, segment) in self.0.iter().enumerate() {
            match segment {
                SchemaSourcePathSegment::Property(name) if index == 0 => f.write_str(name)?,
                SchemaSourcePathSegment::Property(name) => write!(f, ".{name}")?,
                SchemaSourcePathSegment::UnionArm(arm) => write!(f, "[{arm}]")?,
            }
        }
        Ok(())
    }
}

/// The authored source a schema declaration was read from.
///
/// Spans recorded against a [`SchemaSourceId`] are byte ranges in that
/// source's text, so a consumer that holds the text (DMLS for an open
/// document, the resolver for a named-type file) can turn them into ranges.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SchemaSourceId {
    /// The `$schema` declaration being prepared: inline frontmatter or a
    /// caller-supplied schema value. The resolver receives a YAML value, not
    /// its text, so origins in this source carry no span until the consumer
    /// projects one from its own source map.
    InlineDocument,
    /// A schema file, by the canonical path the resolver loaded it from.
    File(std::path::PathBuf),
    /// A schema document compiled into the binary, by its repository-relative
    /// virtual path (`darkmatter/schemas/partials/fm.yaml`). It names no
    /// host file: never read, watched, or reported as one.
    Embedded(String),
}

impl std::fmt::Display for SchemaSourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InlineDocument => f.write_str("the inline `$schema`"),
            Self::File(path) => write!(f, "`{}`", path.display()),
            Self::Embedded(path) => write!(f, "embedded `{path}`"),
        }
    }
}

/// Where one authored constraint came from, as an address into its source's
/// [`SchemaSourceMap`].
///
/// `path` is the structural path of the atom that authored the constraint
/// in `source` (`[type-name, …]` in a named-type file, `[property, …]` under
/// an inline `$schema`); a `$constraints` entry is addressed as
/// `path.property("$constraints").property(key)`. `occurrence` selects among
/// the [`SchemaSpanKind::Constraint`] spans at `path` whose text starts with
/// `keyword` (see [`SchemaSourceMap::constraint_span`]); value-level groups
/// precede array-level groups, as in `T(c)[](c)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaConstraintOrigin {
    /// The source that authored the constraint.
    pub source: SchemaSourceId,
    /// Structural path of the authoring atom within `source`.
    pub path: SchemaSourcePath,
    /// The constraint keyword as authored (`min`, `max-keys`, `scheme`, …).
    pub keyword: String,
    /// Index among the `keyword` constraints at `path`, in source order.
    pub occurrence: usize,
    /// Byte range of the complete constraint in `source`, when the producer
    /// held that source's text.
    pub span: Option<Range<usize>>,
}

/// One constraint taking part in a
/// [`SchemaError::ConstraintConflict`]: where it was declared, and the value
/// that took part in the failing merge.
///
/// `constraint` is the effective value at the merge, which need not be the
/// text at `origin`: an inherited `scheme(...)` list is the intersection of
/// every earlier successful merge, and a named type declared as bare
/// `alphanumeric` takes part as `Constraint::Options(vec![])` at a synthetic
/// origin naming its declaration (no constraint span).
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaConstraintParticipant {
    /// Where the constraint was declared.
    pub origin: SchemaConstraintOrigin,
    /// The complete constraint that took part in the conflict.
    pub constraint: Constraint,
}

/// A semantic definition failure together with the authored location of the
/// node it rejected.
///
/// `path` addresses the rejected node (or, for a structural failure with no
/// scalar token, the nearest containing value) from the root of the supplied
/// YAML: a property-definition call starts at that definition, a declaration
/// call at the declaration, and a standalone schema document at its envelope,
/// so its paths begin with `types` or `$schema`. Union arms, including
/// root-union arms, are addressed by zero-based index.
///
/// Both ranges are UTF-8 byte offsets into the caller's document, with the
/// caller's `source_offset` applied once. `authored_span` is the rejected
/// scalar token; `value_span` is the whole rejected value, for fallback.
/// Either is `None` when it cannot be projected trustworthily; `error` is
/// returned regardless.
#[derive(Debug)]
pub struct LocatedDefinitionFailure {
    /// Structural path of the rejected node.
    pub path: SchemaSourcePath,
    /// The semantic error the source-free parser returns for the same input.
    /// Boxed to keep `Result<_, LocatedDefinitionFailure>` within clippy's
    /// `result_large_err` budget.
    pub error: Box<SchemaError>,
    /// The rejected scalar token's byte range, when projectable.
    pub authored_span: Option<Range<usize>>,
    /// The rejected value's byte range, when locatable.
    pub value_span: Option<Range<usize>>,
}

impl std::fmt::Display for LocatedDefinitionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&*self.error, f)
    }
}

impl std::error::Error for LocatedDefinitionFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.error)
    }
}

impl From<LocatedDefinitionFailure> for SchemaError {
    fn from(failure: LocatedDefinitionFailure) -> Self {
        *failure.error
    }
}

/// The consuming named-type reference a resolution diagnostic belongs to.
///
/// Its authored span is the [`SchemaSpanKind::ImportName`] span at `path`
/// in `source`'s map, for bare and explicit references alike.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaReferenceSite {
    /// The source that wrote the reference.
    pub source: SchemaSourceId,
    /// Structural path of the property (or named type) holding the reference.
    pub path: SchemaSourcePath,
}

/// Which merge rule two or more reused constraints contradict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaConstraintConflictKind {
    /// A lower bound exceeds an upper bound of the same kind (string length,
    /// number, item count, or key count).
    Bounds,
    /// Repeated `scheme(...)` lists share no scheme.
    Scheme,
    /// Repeated `default(...)` values differ after JSON normalization.
    Default,
    /// A use site writes `alphanumeric` options whose canonical set differs
    /// from the named type's.
    Options,
}

impl std::fmt::Display for SchemaConstraintConflictKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Bounds => "bounds",
            Self::Scheme => "schemes",
            Self::Default => "defaults",
            Self::Options => "`alphanumeric` options",
        })
    }
}

/// Why a use-site postfix cannot apply to a union-typed named type.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaUnionRejection {
    /// A constraint that would have to be pushed into every arm
    /// (`U(max(10))`, `U(default(x))`, `U(max(2))[]`), or one that is not
    /// valid on an array container (`U[](pattern(x))`). `array_level` tells
    /// the two apart.
    Constraint {
        /// The rejected constraint's keyword, as authored.
        keyword: String,
        /// Whether the constraint followed `[]`.
        array_level: bool,
    },
    /// `[]` over a union whose flattened arm `arm` is already an array.
    ArrayArm {
        /// Index of the offending arm among the union's flattened arms.
        arm: usize,
    },
    /// `[]` over a union whose arm `arm` carries a default that is not an
    /// array, so it cannot become the array's default.
    ScalarDefault {
        /// Index of the offending arm among the union's flattened arms.
        arm: usize,
        /// The inherited default.
        value: serde_json::Value,
    },
}

impl std::fmt::Display for SchemaUnionRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constraint {
                keyword,
                array_level: false,
            } => write!(f, "constraint `{keyword}` cannot apply to a union-typed type"),
            Self::Constraint {
                keyword,
                array_level: true,
            } => write!(f, "constraint `{keyword}` is not valid on an array of a union"),
            Self::ArrayArm { arm } => {
                write!(f, "cannot apply `[]` because union arm {arm} is already an array")
            }
            Self::ScalarDefault { arm, value } => write!(
                f,
                "cannot apply `[]` because union arm {arm} has the non-array default `{value}`"
            ),
        }
    }
}

/// The authored role described by one source-map span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SchemaSpanKind {
    /// The complete outer schema declaration.
    Declaration,
    /// A mapping key.
    MappingKey,
    /// A complete property definition, including YAML quoting or structure.
    Definition,
    /// One parsed property atom.
    Atom,
    /// A primitive type keyword.
    TypeKeyword,
    /// One complete constraint.
    Constraint,
    /// One constraint argument.
    Argument,
    /// The name of a named-type reference: the portion to the left of `@`,
    /// or the whole identifier of a bare local reference.
    ImportName,
    /// The file-reference portion to the right of `@`.
    ImportReference,
    /// One property-level or root-level union arm.
    UnionArm,
    /// A complete schema file reference.
    FileReference,
}

/// Authored byte ranges indexed by structural schema path and source role.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemaSourceMap {
    spans: BTreeMap<(SchemaSourcePath, SchemaSpanKind), Vec<Range<usize>>>,
}

impl SchemaSourceMap {
    /// Returns every authored span recorded for `path` and `kind`.
    pub fn spans(&self, path: &SchemaSourcePath, kind: SchemaSpanKind) -> &[Range<usize>] {
        self.spans
            .get(&(path.clone(), kind))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Every recorded path and role with its spans, ordered by path and then
    /// role, so a caller can check that nothing is missing or extra.
    pub fn entries(
        &self,
    ) -> impl Iterator<Item = (&SchemaSourcePath, SchemaSpanKind, &[Range<usize>])> {
        self.spans
            .iter()
            .map(|((path, kind), spans)| (path, *kind, spans.as_slice()))
    }

    /// The span of the `occurrence`-th complete constraint at `path` whose
    /// authored text in `source` starts with the whole word `keyword`
    /// (`min` does not select `min-keys(…)`).
    ///
    /// `source` must be the text this map was projected from. This is how a
    /// [`SchemaConstraintOrigin`] becomes a location.
    pub fn constraint_span(
        &self,
        source: &str,
        path: &SchemaSourcePath,
        keyword: &str,
        occurrence: usize,
    ) -> Option<Range<usize>> {
        self.spans(path, SchemaSpanKind::Constraint)
            .iter()
            .filter(|span| {
                source
                    .get((*span).clone())
                    .and_then(|text| text.strip_prefix(keyword))
                    .is_some_and(|rest| {
                        // A `,` ends the first of several `alphanumeric`
                        // options, which are located as one constraint.
                        rest.chars().next().is_none_or(|next| {
                            next == '(' || next == ':' || next == ',' || next.is_whitespace()
                        })
                    })
            })
            .nth(occurrence)
            .cloned()
    }

    fn insert(&mut self, path: &SchemaSourcePath, kind: SchemaSpanKind, span: Range<usize>) {
        self.spans.entry((path.clone(), kind)).or_default().push(span);
    }
}

/// A passive semantic parse product paired with its structural source map.
#[derive(Debug, Clone)]
pub struct SourceAware<T> {
    /// The semantic product returned by the corresponding source-free parser.
    pub value: T,
    /// Exact authored byte ranges for the parsed structure.
    pub source_map: SchemaSourceMap,
}

/// Parses one property definition and projects its structure into authored
/// source byte ranges.
///
/// `yaml_source` must be the authored YAML representation of `value` itself,
/// starting at a line boundary or dedented (see [`locate_schema_value`]).
/// `source_offset` moves all returned spans into the caller's document.
///
/// ## Errors
///
/// A semantic failure returns the source-free parser's error, its path from
/// this definition, and whatever authored ranges can be projected for the
/// rejected node (see [`LocatedDefinitionFailure`]). When the semantic parse
/// succeeds but the source is outside the strict presentation grammar or does
/// not decode to `value`, the projection error is returned at the root path
/// with no ranges.
pub fn parse_property_definition_with_source(
    property: &str,
    value: &YamlValue,
    yaml_source: &str,
    source_offset: usize,
) -> Result<SourceAware<PropertyDef>, LocatedDefinitionFailure> {
    let parsed = parse_property_definition_located(property, value).map_err(|failure| {
        locate_definition_failure(failure, value, yaml_source, source_offset)
    })?;
    let located = locate_yaml_value(yaml_source).map_err(unlocated_failure)?;
    verify_located(&located, value).map_err(unlocated_failure)?;
    let value = parsed;
    let mut source_map = SchemaSourceMap::default();
    project_property_source(
        &value,
        &located,
        &SchemaSourcePath::root(),
        source_offset,
        &mut source_map,
    )
    .map_err(unlocated_failure)?;
    Ok(SourceAware { value, source_map })
}

/// Parses one complete schema declaration and projects its structure into
/// authored source byte ranges without resolving references.
///
/// ## Errors
///
/// As [`parse_property_definition_with_source`], with failure paths starting
/// at the declaration.
pub fn parse_schema_declaration_with_source(
    value: &YamlValue,
    yaml_source: &str,
    source_offset: usize,
) -> Result<SourceAware<SchemaDeclaration>, LocatedDefinitionFailure> {
    let parsed = parse_schema_declaration_located(value).map_err(|failure| {
        locate_definition_failure(failure, value, yaml_source, source_offset)
    })?;
    let located = locate_yaml_value(yaml_source).map_err(unlocated_failure)?;
    verify_located(&located, value).map_err(unlocated_failure)?;
    let value = parsed;
    let mut source_map = SchemaSourceMap::default();
    let root = SchemaSourcePath::root();
    source_map.insert(
        &root,
        SchemaSpanKind::Declaration,
        offset_span(&located.span, source_offset),
    );
    match &value {
        SchemaDeclaration::Reference(_) => {
            record_scalar_content(
                &located,
                &root,
                SchemaSpanKind::FileReference,
                source_offset,
                &mut source_map,
            )
            .map_err(unlocated_failure)?;
        }
        SchemaDeclaration::Schema(schema) => {
            project_schema_source(schema, &located, source_offset, &mut source_map)
                .map_err(unlocated_failure)?;
        }
    }
    Ok(SourceAware { value, source_map })
}

/// Finds the first definition failure in a standalone schema document and
/// locates it in `source`.
///
/// Recognition and number admission are the standalone classifier's own
/// ([`parse_standalone_schema_document`](super::parse_standalone_schema_document)),
/// and the payload is parsed by the same shared parser, so the failure is the
/// semantic error that parser reports first. Its path starts at the envelope:
/// `types` for a tagged document, `$schema` for a pure one (including root-union
/// mapping arms, by index). Ranges are byte offsets into `source`.
///
/// ## Returns
///
/// `None` for a valid payload, an unrecognized document, invalid YAML, and
/// every outer envelope or declaration-shape failure (a global catalog, a
/// malformed envelope, a payload or root-union arm of the wrong type, an
/// invalid whole-file reference), all of which the classifier reports. A
/// recognized definition failure is returned even when neither range can be
/// projected.
///
/// ## Notes
///
/// Passive: parses `source` once, parses the payload once, and locates the
/// authored tree at most once; it reads no files, resolves no imports, and
/// evaluates nothing.
pub fn locate_standalone_definition_failure(source: &str) -> Option<LocatedDefinitionFailure> {
    let recognized = super::standalone::recognize_standalone(source).ok()??;
    let payload = recognized.payload();
    let failure = parse_schema_declaration_located(payload).err()?;
    // A failure on no property is the declaration's own shape, not a definition.
    if !failure
        .path
        .segments()
        .iter()
        .any(|segment| matches!(segment, SchemaSourcePathSegment::Property(_)))
    {
        return None;
    }
    let payload_key = recognized.payload_key();
    let located = locate_yaml_value(source).ok();
    let located_payload = located.as_ref().and_then(|located| match &located.kind {
        LocatedKind::Mapping(pairs) => pairs
            .iter()
            .find(|pair| pair.key == payload_key)
            .map(|pair| &pair.value),
        _ => None,
    });
    Some(project_definition_failure(
        failure,
        located_payload.map(|located| (located, payload)),
        &SchemaSourcePath::root().property(payload_key),
        0,
    ))
}

/// A projection error after a successful semantic parse: nothing was
/// rejected, so there is no node to locate.
fn unlocated_failure(error: SchemaError) -> LocatedDefinitionFailure {
    LocatedDefinitionFailure {
        path: SchemaSourcePath::root(),
        error: Box::new(error),
        authored_span: None,
        value_span: None,
    }
}

/// Locates `failure`, whose path is relative to `value`, in `yaml_source`
/// (the authored text of `value`). An unlocatable source keeps the semantic
/// error with no ranges.
fn locate_definition_failure(
    failure: DefinitionFailure,
    value: &YamlValue,
    yaml_source: &str,
    source_offset: usize,
) -> LocatedDefinitionFailure {
    let located = locate_yaml_value(yaml_source).ok();
    project_definition_failure(
        failure,
        located.as_ref().map(|located| (located, value)),
        &SchemaSourcePath::root(),
        source_offset,
    )
}

/// Projects `failure` through `root`, the located tree paired with the YAML
/// value the semantic parser read, and prefixes its path with `prefix`.
///
/// Every step along the path must agree structurally with the parsed value,
/// and a token is projected only through a scalar whose decoded text equals
/// the string the parser rejected; anything less leaves the range `None`
/// rather than pointing at another node.
fn project_definition_failure(
    failure: DefinitionFailure,
    root: Option<(&LocatedValue, &YamlValue)>,
    prefix: &SchemaSourcePath,
    source_offset: usize,
) -> LocatedDefinitionFailure {
    let rejected = root.and_then(|(located, value)| follow_path(located, value, &failure.path));
    let value_span = rejected
        .filter(|(located, value)| verify_located(located, value).is_ok())
        .map(|(located, _)| offset_span(&located.span, source_offset));
    let authored_span = match (&*failure.error, rejected) {
        (
            SchemaError::Grammar { span, .. },
            Some((
                LocatedValue {
                    kind: LocatedKind::Scalar(scalar),
                    ..
                },
                YamlValue::String(text),
            )),
        ) if !span.is_empty() && scalar.decoded() == text => scalar
            .project(span.clone())
            .filter(|projected| !projected.is_empty())
            .map(|projected| offset_span(&projected, source_offset)),
        _ => None,
    };
    LocatedDefinitionFailure {
        path: prefix.join(&failure.path),
        error: failure.error,
        authored_span,
        value_span,
    }
}

/// The located node and parsed value `path` addresses, following both trees
/// in lockstep. `None` as soon as they disagree in shape, size, or key.
fn follow_path<'a>(
    mut located: &'a LocatedValue,
    mut value: &'a YamlValue,
    path: &SchemaSourcePath,
) -> Option<(&'a LocatedValue, &'a YamlValue)> {
    for segment in path.segments() {
        match (segment, &located.kind, value) {
            (
                SchemaSourcePathSegment::Property(key),
                LocatedKind::Mapping(pairs),
                YamlValue::Mapping(map),
            ) if pairs.len() == map.len() => {
                let (index, (_, child)) = map
                    .iter()
                    .enumerate()
                    .find(|(_, (candidate, _))| candidate.as_str() == Some(key.as_str()))?;
                let pair = &pairs[index];
                if pair.key != *key {
                    return None;
                }
                (located, value) = (&pair.value, child);
            }
            (
                SchemaSourcePathSegment::UnionArm(index),
                LocatedKind::Sequence(items),
                YamlValue::Sequence(values),
            ) if items.len() == values.len() => {
                (located, value) = (items.get(*index)?, values.get(*index)?);
            }
            _ => return None,
        }
    }
    Some((located, value))
}

pub(super) fn parse_standalone_schema_payload_with_source(
    value: &YamlValue,
    yaml_source: &str,
    payload_key: &str,
) -> Result<SourceAware<SchemaDeclaration>, SchemaError> {
    let parsed = parse_schema_declaration(value)?;
    let located = locate_yaml_value(yaml_source)?;
    let LocatedKind::Mapping(pairs) = &located.kind else {
        return Err(projection_error());
    };
    let payload = pairs
        .iter()
        .find(|pair| pair.key == payload_key)
        .map(|pair| &pair.value)
        .ok_or_else(projection_error)?;
    verify_located(payload, value)?;
    let mut value = parsed;
    let mut source_map = SchemaSourceMap::default();
    let root = SchemaSourcePath::root();
    source_map.insert(&root, SchemaSpanKind::Declaration, payload.span.clone());
    match &mut value {
        SchemaDeclaration::Reference(_) => {
            record_scalar_content(payload, &root, SchemaSpanKind::FileReference, 0, &mut source_map)?;
        }
        SchemaDeclaration::Schema(schema) => {
            project_schema_source(schema, payload, 0, &mut source_map)?;
            project_located_schema_suggestions(schema, payload)?;
        }
    }
    Ok(SourceAware { value, source_map })
}

/// Projects every `suggest(...)` candidate span through the located scalar
/// authoring its own definition, the same decoded coordinates the structural
/// source map uses, so identical definitions cannot trade spans.
///
/// An alias stands for a definition authored elsewhere; one carrying
/// suggestions has no scalar of its own to project into and is a projection
/// error.
fn project_located_schema_suggestions(
    schema: &mut SimplifiedSchema,
    located: &LocatedValue,
) -> Result<(), SchemaError> {
    match (schema, &located.kind) {
        (SimplifiedSchema::Single(shape), LocatedKind::Mapping(_)) => {
            project_located_shape_suggestions(shape, located)
        }
        (SimplifiedSchema::Union(arms), LocatedKind::Sequence(items))
            if arms.len() == items.len() =>
        {
            for (arm, item) in arms.iter_mut().zip(items) {
                if let SchemaArm::Inline(shape) = arm {
                    project_located_shape_suggestions(shape, item)?;
                }
            }
            Ok(())
        }
        (schema, LocatedKind::Scalar(scalar)) if scalar.decoded().starts_with('*') => {
            let suggests = |shape: &SchemaShape| {
                shape.properties.values().any(property_has_suggestions)
                    || shape.pattern_keys.iter().any(|pattern| property_has_suggestions(&pattern.def))
            };
            let has_suggestions = match schema {
                SimplifiedSchema::Single(shape) => suggests(shape),
                SimplifiedSchema::Union(arms) => arms.iter().any(|arm| match arm {
                    SchemaArm::Inline(shape) => suggests(shape),
                    SchemaArm::FileRef(_) => false,
                }),
            };
            if has_suggestions { Err(projection_error()) } else { Ok(()) }
        }
        _ => Err(projection_error()),
    }
}

fn project_located_shape_suggestions(
    shape: &mut SchemaShape,
    located: &LocatedValue,
) -> Result<(), SchemaError> {
    let LocatedKind::Mapping(pairs) = &located.kind else {
        return Err(projection_error());
    };
    for pair in pairs {
        if pair.key == "$constraints" {
            continue;
        }
        let definition = match shape.properties.get_mut(&pair.key) {
            Some(definition) => definition,
            None => {
                let key = super::PatternKey::parse(&pair.key).map_err(|_| projection_error())?;
                shape
                    .pattern_keys
                    .iter_mut()
                    .find(|pattern| pattern.key == key)
                    .map(|pattern| &mut pattern.def)
                    .ok_or_else(projection_error)?
            }
        };
        project_located_property_suggestions(definition, &pair.value)?;
    }
    Ok(())
}

fn project_located_property_suggestions(
    definition: &mut PropertyDef,
    located: &LocatedValue,
) -> Result<(), SchemaError> {
    if let LocatedKind::Scalar(scalar) = &located.kind
        && scalar.decoded().starts_with('*')
    {
        return if property_has_suggestions(definition) {
            Err(projection_error())
        } else {
            Ok(())
        };
    }
    match (definition, &located.kind) {
        (PropertyDef::Single(atom), _) => project_located_atom_suggestions(atom, located),
        (PropertyDef::Union(atoms), LocatedKind::Scalar(scalar)) => {
            for atom in atoms {
                project_atom_suggestions(atom, scalar, 0)?;
            }
            Ok(())
        }
        (PropertyDef::Union(atoms), LocatedKind::Sequence(items)) if atoms.len() == items.len() => {
            for (atom, item) in atoms.iter_mut().zip(items) {
                project_located_atom_suggestions(atom, item)?;
            }
            Ok(())
        }
        _ => Err(projection_error()),
    }
}

fn project_located_atom_suggestions(
    atom: &mut PropertyAtom,
    located: &LocatedValue,
) -> Result<(), SchemaError> {
    match (&mut atom.ty, &located.kind) {
        (TypeExpr::InlineObject(shape), LocatedKind::Mapping(_)) => {
            project_located_shape_suggestions(shape, located)
        }
        (_, LocatedKind::Scalar(scalar)) => project_atom_suggestions(atom, scalar, 0),
        _ => Err(projection_error()),
    }
}

/// One authored node inside a schema value, with its exact source span.
///
/// This is the structural half of the sidecar, for consumers that must point at
/// something *inside* a value the semantic parser rejects — where
/// [`parse_property_definition_with_source`] cannot help, because there is no
/// parse product to project. The shapes come from the same block/flow locator
/// the source-aware parsers use, so a `[]` inside a quoted scalar or a `:`
/// inside a regex is never mistaken for structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaValueNode {
    /// Authored byte range of the complete node.
    pub span: Range<usize>,
    /// The node's authored shape, with its children.
    pub kind: SchemaValueKind,
}

impl SchemaValueNode {
    /// Every node in this subtree paired with its depth, deepest first.
    ///
    /// Ordering makes "the smallest authored node that fails" a single pass.
    pub fn deepest_first(&self) -> Vec<(usize, &SchemaValueNode)> {
        let mut out = Vec::new();
        self.collect(0, &mut out);
        out.sort_by_key(|(depth, _)| std::cmp::Reverse(*depth));
        out
    }

    fn collect<'a>(&'a self, depth: usize, out: &mut Vec<(usize, &'a SchemaValueNode)>) {
        out.push((depth, self));
        match &self.kind {
            SchemaValueKind::Scalar => {}
            SchemaValueKind::Mapping(entries) => {
                for entry in entries {
                    entry.value.collect(depth + 1, out);
                }
            }
            SchemaValueKind::Sequence(items) => {
                for item in items {
                    item.collect(depth + 1, out);
                }
            }
        }
    }
}

/// The authored shape of a [`SchemaValueNode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaValueKind {
    /// A plain, single-quoted, or double-quoted scalar.
    Scalar,
    /// A block or flow mapping.
    Mapping(Vec<SchemaValueEntry>),
    /// A block or flow sequence. Empty for `[]`.
    Sequence(Vec<SchemaValueNode>),
}

/// One key/value pair of a [`SchemaValueKind::Mapping`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaValueEntry {
    /// The decoded mapping key.
    pub key: String,
    /// Authored byte range of the key token.
    pub key_span: Range<usize>,
    /// The pair's value node.
    pub value: SchemaValueNode,
}

/// Locates the authored structure of one schema value without interpreting it.
///
/// `yaml_source` is the value's own authored YAML text and `source_offset` its
/// byte offset in the caller's document. The source is never
/// line-ending-normalized.
///
/// Indentation decides where a continued plain scalar ends, so `yaml_source`
/// must start at a line boundary, including the first line's indentation, or
/// be dedented as a whole. A slice starting at a block mapping's first key
/// reads that key at column 0 and its siblings as continuation text.
///
/// The grammar is strict: indentless block sequences, literal and folded
/// block scalars, and plain scalars continued onto more-indented lines are
/// followed, and every non-comment line must belong to the value. Tags,
/// multi-line quoted scalars, and empty (null) values are not.
///
/// ## Returns
///
/// `None` when the text is not a locatable YAML block or flow node — the signal
/// to fall back to the whole value's range.
pub fn locate_schema_value(yaml_source: &str, source_offset: usize) -> Option<SchemaValueNode> {
    locate_yaml_value(yaml_source)
        .ok()
        .map(|located| public_node(&located, source_offset))
}

/// [`locate_schema_value`] for a document's frontmatter, which also follows a
/// multi-line quoted scalar, a tag, and an empty value instead of giving up,
/// and does not require every line to belong to the value.
///
/// Schema documents keep [`locate_schema_value`]'s strict grammar, which
/// rejects those presentations.
pub(crate) fn locate_frontmatter_value(
    yaml_source: &str,
    source_offset: usize,
) -> Option<SchemaValueNode> {
    locate_yaml_value_with(yaml_source, LocatorGrammar::Tolerant)
        .ok()
        .map(|located| public_node(&located, source_offset))
}

fn public_node(located: &LocatedValue, offset: usize) -> SchemaValueNode {
    let kind = match &located.kind {
        LocatedKind::Scalar(_) => SchemaValueKind::Scalar,
        LocatedKind::Mapping(pairs) => SchemaValueKind::Mapping(
            pairs
                .iter()
                .map(|pair| SchemaValueEntry {
                    key: pair.key.clone(),
                    key_span: offset_span(&pair.key_span, offset),
                    value: public_node(&pair.value, offset),
                })
                .collect(),
        ),
        LocatedKind::Sequence(items) => SchemaValueKind::Sequence(
            items.iter().map(|item| public_node(item, offset)).collect(),
        ),
    };
    SchemaValueNode {
        span: offset_span(&located.span, offset),
        kind,
    }
}

#[derive(Debug, Clone)]
struct LocatedValue {
    span: Range<usize>,
    kind: LocatedKind,
}

#[derive(Debug, Clone)]
enum LocatedKind {
    Scalar(DecodedScalar),
    Mapping(Vec<LocatedPair>),
    Sequence(Vec<LocatedValue>),
}

#[derive(Debug, Clone)]
struct LocatedPair {
    key: String,
    key_span: Range<usize>,
    pair_span: Range<usize>,
    value: LocatedValue,
}

#[derive(Debug, Clone)]
struct SourceLine {
    end: usize,
    indent: usize,
    content_start: usize,
}

/// Which YAML presentations [`BlockLocator`] follows.
///
/// Both grammars accept a block sequence aligned with its parent key
/// (`key:\n- a`), including after a mapping-value anchor, and a plain scalar
/// continued onto more-indented lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocatorGrammar {
    /// The closed schema-document grammar. Every non-comment source line must
    /// belong to the located node, so a presentation the locator does not
    /// model fails instead of leaving lines unread.
    Strict,
    /// Frontmatter lookup, which also follows tags, multi-line quoted
    /// scalars, empty (null) values, and compact sequence items wider than
    /// `- `.
    Tolerant,
}

fn locate_yaml_value(source: &str) -> Result<LocatedValue, SchemaError> {
    locate_yaml_value_with(source, LocatorGrammar::Strict)
}

fn locate_yaml_value_with(
    source: &str,
    grammar: LocatorGrammar,
) -> Result<LocatedValue, SchemaError> {
    let lines = source_lines(source);
    if lines.is_empty() {
        return Err(projection_error());
    }
    let mut parser = BlockLocator {
        source,
        lines,
        next: 0,
        tolerant: grammar == LocatorGrammar::Tolerant,
    };
    let indent = parser.lines[0].indent;
    let located = parser.node(indent)?;
    if grammar == LocatorGrammar::Strict && parser.next != parser.lines.len() {
        return Err(projection_error());
    }
    Ok(located)
}

fn source_lines(source: &str) -> Vec<SourceLine> {
    let mut lines = Vec::new();
    let mut start = 0;
    for raw in source.split_inclusive('\n') {
        let mut end = start + raw.len();
        if raw.ends_with('\n') {
            end -= 1;
        }
        if end > start && source.as_bytes()[end - 1] == b'\r' {
            end -= 1;
        }
        let line = &source[start..end];
        let indent = line.bytes().take_while(|byte| byte.is_ascii_whitespace()).count();
        let content_start = start + indent;
        if content_start < end
            && !source[content_start..end].starts_with('#')
            && !(indent == 0 && matches!(&source[content_start..end], "---" | "..."))
        {
            lines.push(SourceLine {
                end,
                indent,
                content_start,
            });
        }
        start += raw.len();
    }
    if start < source.len() {
        let end = source.len();
        let line = &source[start..end];
        let indent = line.bytes().take_while(|byte| byte.is_ascii_whitespace()).count();
        let content_start = start + indent;
        if content_start < end
            && !source[content_start..end].starts_with('#')
            && !(indent == 0 && matches!(&source[content_start..end], "---" | "..."))
        {
            lines.push(SourceLine {
                end,
                indent,
                content_start,
            });
        }
    }
    lines
}

struct BlockLocator<'a> {
    source: &'a str,
    lines: Vec<SourceLine>,
    next: usize,
    /// [`LocatorGrammar::Tolerant`]: follow the presentations only
    /// frontmatter lookup accepts.
    tolerant: bool,
}

impl BlockLocator<'_> {
    fn node(&mut self, indent: usize) -> Result<LocatedValue, SchemaError> {
        let line = self.lines.get(self.next).ok_or_else(projection_error)?.clone();
        if line.indent != indent {
            return Err(projection_error());
        }
        let content = &self.source[line.content_start..line.end];
        if sequence_content(content).is_some() {
            self.sequence(indent)
        } else if explicit_indicator_content(content, '?').is_some()
            || mapping_separator(content, false).is_some()
        {
            self.mapping(indent)
        } else {
            self.next += 1;
            let range = line.content_start..line.end;
            if self.tolerant {
                // Frontmatter lookup keeps reading a scalar that opens its own
                // line as that line alone.
                locate_inline(self.source, range)
            } else {
                self.inline_value(range, indent)
            }
        }
    }

    fn mapping(&mut self, indent: usize) -> Result<LocatedValue, SchemaError> {
        let mut pairs = Vec::new();
        while let Some(line) = self.lines.get(self.next).cloned() {
            if line.indent != indent
                || sequence_content(&self.source[line.content_start..line.end]).is_some()
            {
                break;
            }
            let content = &self.source[line.content_start..line.end];
            if explicit_indicator_content(content, '?').is_some() {
                self.next += 1;
                pairs.push(self.explicit_pair(line.content_start..line.end, indent)?);
            } else if mapping_separator(content, false).is_some() {
                self.next += 1;
                pairs.push(self.pair(line.content_start..line.end, indent)?);
            } else {
                break;
            }
        }
        located_mapping(pairs)
    }

    fn sequence(&mut self, indent: usize) -> Result<LocatedValue, SchemaError> {
        let start = self
            .lines
            .get(self.next)
            .map(|line| line.content_start)
            .ok_or_else(projection_error)?;
        let mut items = Vec::new();
        while let Some(line) = self.lines.get(self.next).cloned() {
            if line.indent != indent {
                break;
            }
            let content = &self.source[line.content_start..line.end];
            let Some(relative) = sequence_content(content) else {
                break;
            };
            self.next += 1;
            let item_start = line.content_start + relative;
            if item_start < line.end {
                let item_content = &self.source[item_start..line.end];
                if explicit_indicator_content(item_content, '?').is_some()
                    || mapping_separator(item_content, false).is_some()
                {
                    // The item's keys align with its first key. The strict
                    // grammar keeps the conventional `- ` width.
                    let item_indent = if self.tolerant {
                        indent + relative
                    } else {
                        indent + 2
                    };
                    let first = if explicit_indicator_content(item_content, '?').is_some() {
                        self.explicit_pair(item_start..line.end, item_indent)?
                    } else {
                        self.pair(item_start..line.end, item_indent)?
                    };
                    let mut pairs = vec![first];
                    while let Some(next) = self.lines.get(self.next).cloned() {
                        if next.indent != item_indent
                            || sequence_content(&self.source[next.content_start..next.end]).is_some()
                        {
                            break;
                        }
                        let next_content = &self.source[next.content_start..next.end];
                        if explicit_indicator_content(next_content, '?').is_some() {
                            self.next += 1;
                            pairs.push(
                                self.explicit_pair(next.content_start..next.end, item_indent)?,
                            );
                        } else if mapping_separator(next_content, false).is_some() {
                            self.next += 1;
                            pairs.push(self.pair(next.content_start..next.end, item_indent)?);
                        } else {
                            break;
                        }
                    }
                    items.push(located_mapping(pairs)?);
                } else {
                    items.push(self.inline_value(item_start..line.end, indent)?);
                }
            } else if let Some(child_indent) = self
                .lines
                .get(self.next)
                .filter(|next| next.indent > indent)
                .map(|next| next.indent)
            {
                items.push(self.node(child_indent)?);
            } else {
                // An empty item is null, which a schema union may write as
                // its `null` arm.
                let at = item_start.min(line.end);
                items.push(LocatedValue {
                    span: at..at,
                    kind: LocatedKind::Scalar(DecodedScalar::empty(at)),
                });
            }
        }
        let end = items.last().map(|item| item.span.end).unwrap_or(start);
        Ok(LocatedValue {
            span: start..end,
            kind: LocatedKind::Sequence(items),
        })
    }

    fn pair(&mut self, range: Range<usize>, indent: usize) -> Result<LocatedPair, SchemaError> {
        let raw = &self.source[range.clone()];
        let colon = mapping_separator(raw, false).ok_or_else(projection_error)?;
        let key_range = trim_range(self.source, range.start..range.start + colon);
        let key = decoded_text(self.source, &key_range)?;
        let value_range = trim_range(self.source, range.start + colon + 1..range.end);
        let value = self.value(value_range, indent)?;
        let pair_span = key_range.start..value.span.end;
        Ok(LocatedPair {
            key,
            key_span: key_range,
            pair_span,
            value,
        })
    }

    fn explicit_pair(
        &mut self,
        key_line: Range<usize>,
        indent: usize,
    ) -> Result<LocatedPair, SchemaError> {
        let key_content = &self.source[key_line.clone()];
        let key_relative = explicit_indicator_content(key_content, '?')
            .ok_or_else(projection_error)?;
        let key_range = trim_range(self.source, key_line.start + key_relative..key_line.end);
        let key = decoded_text(self.source, &key_range)?;

        let value_line = self.lines.get(self.next).ok_or_else(projection_error)?.clone();
        if value_line.indent != indent {
            return Err(projection_error());
        }
        let value_content = &self.source[value_line.content_start..value_line.end];
        let value_relative = explicit_indicator_content(value_content, ':')
            .ok_or_else(projection_error)?;
        self.next += 1;
        let value_range =
            trim_range(self.source, value_line.content_start + value_relative..value_line.end);
        let value = self.value(value_range, indent)?;
        let pair_span = key_range.start..value.span.end;
        Ok(LocatedPair {
            key,
            key_span: key_range,
            pair_span,
            value,
        })
    }

    fn value(
        &mut self,
        value_range: Range<usize>,
        indent: usize,
    ) -> Result<LocatedValue, SchemaError> {
        if value_range.start < value_range.end {
            if let Some(after_properties) =
                node_properties_end(self.source, &value_range, self.tolerant)
            {
                let remainder = trim_range(self.source, after_properties..value_range.end);
                let mut value = if remainder.start < remainder.end {
                    self.inline_value(remainder, indent)?
                } else if let Some(node) = self.block_value(indent)? {
                    node
                } else if self.tolerant {
                    // A tagged empty value (`key: !!str`) has no node to descend into.
                    locate_inline(self.source, value_range.clone())?
                } else {
                    return Err(projection_error());
                };
                value.span.start = value_range.start;
                Ok(value)
            } else {
                self.inline_value(value_range, indent)
            }
        } else {
            match self.block_value(indent)? {
                Some(node) => Ok(node),
                // An empty value is null, which a schema may write as the
                // `null` type.
                None => Ok(LocatedValue {
                    span: value_range.start..value_range.start,
                    kind: LocatedKind::Scalar(DecodedScalar::empty(value_range.start)),
                }),
            }
        }
    }

    /// The node a mapping value at key column `indent` holds on the following
    /// lines: anything more indented, or a block sequence aligned with the key,
    /// which is how `serde_yaml_ng` writes a mapping's sequence value
    /// (`key:\n- a`). `None` when the next line holds neither.
    fn block_value(&mut self, indent: usize) -> Result<Option<LocatedValue>, SchemaError> {
        let Some(line) = self.lines.get(self.next).cloned() else {
            return Ok(None);
        };
        if line.indent > indent {
            self.node(line.indent).map(Some)
        } else if line.indent == indent
            && sequence_content(&self.source[line.content_start..line.end]).is_some()
        {
            self.sequence(indent).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl BlockLocator<'_> {
    /// Locates a value that starts inside a line. A plain scalar continued
    /// onto more-indented lines is followed and those lines consumed; the
    /// tolerant grammar also follows block scalars and quoted scalars that
    /// may span lines. `parent_indent` is the column of the holding mapping
    /// key or sequence entry.
    fn inline_value(
        &mut self,
        range: Range<usize>,
        parent_indent: usize,
    ) -> Result<LocatedValue, SchemaError> {
        let raw = &self.source[range.clone()];
        let continues = self
            .lines
            .get(self.next)
            .is_some_and(|line| line.indent > parent_indent);
        let multi_line = if self.tolerant {
            is_block_scalar_header(raw)
                || raw.starts_with(['\'', '"'])
                || (continues && !raw.starts_with(['[', '{']))
        } else {
            // A literal or folded block scalar, the form multi-line type
            // expressions such as `function(...)` signatures are written in,
            // or a continued plain scalar: a quoted, tagged, anchored, or
            // aliased first line stays single-line here, so its continuation
            // lines go unconsumed and the strict grammar rejects the document.
            is_block_scalar_header(raw)
                || (continues && !raw.starts_with(['\'', '"', '[', '{', '!', '&', '*']))
        };
        if !multi_line {
            return locate_inline(self.source, range);
        }
        let (scalar, end) =
            yaml_scalar::decode_scalar_node(self.source, range.start, parent_indent)
                .ok_or_else(projection_error)?;
        while self
            .lines
            .get(self.next)
            .is_some_and(|line| line.content_start < end)
        {
            self.next += 1;
        }
        Ok(LocatedValue {
            span: range.start..end,
            kind: LocatedKind::Scalar(scalar),
        })
    }
}

fn explicit_indicator_content(source: &str, indicator: char) -> Option<usize> {
    let rest = source.strip_prefix(indicator)?;
    if rest.is_empty() {
        return Some(indicator.len_utf8());
    }
    let spaces = rest.bytes().take_while(|byte| byte.is_ascii_whitespace()).count();
    (spaces > 0).then_some(indicator.len_utf8() + spaces)
}

/// Where the node properties opening `range` end: an anchor and, with `tags`
/// set, a tag, in either order. `None` when the value has none.
fn node_properties_end(source: &str, range: &Range<usize>, tags: bool) -> Option<usize> {
    let mut end = None;
    let mut pos = range.start;
    let (mut tag, mut anchor) = (false, false);
    loop {
        let rest = &source[pos..range.end];
        let token = rest
            .bytes()
            .take_while(|byte| !byte.is_ascii_whitespace())
            .count();
        match rest.as_bytes().first() {
            Some(b'&') if !anchor && token > 1 => anchor = true,
            Some(b'!') if tags && !tag => tag = true,
            _ => return end,
        }
        end = Some(pos + token);
        pos += token;
        pos += source[pos..range.end]
            .bytes()
            .take_while(u8::is_ascii_whitespace)
            .count();
    }
}

/// Checks that a strictly located tree describes `value`, the YAML parser's
/// reading of the same text: the same mapping keys in order, the same
/// sequence lengths, and every string scalar decoding to the parser's text.
/// Any disagreement means the decoded-to-authored maps describe other text,
/// so no span may be projected through them.
///
/// An alias stands for its anchored node and is not compared; a non-string
/// scalar (number, boolean, null) carries no expression to project.
fn verify_located(located: &LocatedValue, value: &YamlValue) -> Result<(), SchemaError> {
    let agrees = match (&located.kind, value) {
        (LocatedKind::Scalar(scalar), _) if scalar.decoded().starts_with('*') => true,
        (LocatedKind::Scalar(scalar), YamlValue::String(text)) => scalar.decoded() == text,
        (LocatedKind::Scalar(_), YamlValue::Null | YamlValue::Bool(_) | YamlValue::Number(_)) => {
            true
        }
        (LocatedKind::Mapping(pairs), YamlValue::Mapping(map)) if pairs.len() == map.len() => {
            for (pair, (key, child)) in pairs.iter().zip(map) {
                if key.as_str().is_some_and(|key| key != pair.key) {
                    return Err(projection_error());
                }
                verify_located(&pair.value, child)?;
            }
            true
        }
        (LocatedKind::Sequence(items), YamlValue::Sequence(values))
            if items.len() == values.len() =>
        {
            for (item, child) in items.iter().zip(values) {
                verify_located(item, child)?;
            }
            true
        }
        _ => false,
    };
    if agrees { Ok(()) } else { Err(projection_error()) }
}

fn located_mapping(pairs: Vec<LocatedPair>) -> Result<LocatedValue, SchemaError> {
    let start = pairs.first().map(|pair| pair.key_span.start).ok_or_else(projection_error)?;
    let end = pairs.last().map(|pair| pair.value.span.end).ok_or_else(projection_error)?;
    Ok(LocatedValue {
        span: start..end,
        kind: LocatedKind::Mapping(pairs),
    })
}

fn locate_inline(source: &str, range: Range<usize>) -> Result<LocatedValue, SchemaError> {
    let range = trim_range(source, range);
    let raw = &source[range.clone()];
    if is_block_scalar_header(raw) {
        return Err(projection_error());
    }
    if raw.starts_with('[') {
        let end = flow_collection_end(source, range.start)
            .filter(|end| *end + 1 == range.end)
            .ok_or_else(projection_error)?;
        // An empty flow collection (`[]`) splits into one empty item; it has
        // no child node, and is itself the smallest authored node there is.
        let items = split_flow_entries(source, range.start + 1..end)
            .into_iter()
            .map(|item| trim_range(source, item))
            .filter(|item| item.start < item.end)
            .map(|item| locate_inline(source, item))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(LocatedValue {
            span: range,
            kind: LocatedKind::Sequence(items),
        });
    }
    if raw.starts_with('{') {
        let end = flow_collection_end(source, range.start)
            .filter(|end| *end + 1 == range.end)
            .ok_or_else(projection_error)?;
        let mut pairs = Vec::new();
        for pair in split_flow_entries(source, range.start + 1..end) {
            let pair = trim_range(source, pair);
            if pair.start == pair.end {
                continue;
            }
            let raw_pair = &source[pair.clone()];
            let colon = mapping_separator(raw_pair, true).ok_or_else(projection_error)?;
            let key_span = trim_range(source, pair.start..pair.start + colon);
            let key = decoded_text(source, &key_span)?;
            let value = locate_inline(source, pair.start + colon + 1..pair.end)?;
            let pair_span = key_span.start..value.span.end;
            pairs.push(LocatedPair {
                key,
                key_span,
                pair_span,
                value,
            });
        }
        return Ok(LocatedValue {
            span: range,
            kind: LocatedKind::Mapping(pairs),
        });
    }
    let (scalar, consumed) = yaml_scalar::decode_scalar_at(raw, range.start)
        .ok_or_else(projection_error)?;
    Ok(LocatedValue {
        span: range.start..range.start + consumed,
        kind: LocatedKind::Scalar(scalar),
    })
}

/// Where the raw YAML flow collection opening at byte `start` of `source`
/// closes: the offset of its matching `]` or `}`.
///
/// A quote opens a quoted scalar only as the scalar's first character, so the
/// apostrophe in `[don't, "v"]` is content, and so is the one in
/// `[a:'b, "v"]`, whose `:` is plain-scalar content rather than a value
/// indicator; a `]` inside `"a]"` is not structure. A `#` after whitespace
/// starts a comment that runs to the end of its line.
///
/// ## Returns
///
/// `None` when `start` does not open a flow collection or the collection is
/// not closed, including by an unterminated quoted scalar.
pub fn flow_collection_end(source: &str, start: usize) -> Option<usize> {
    if !matches!(source.as_bytes().get(start), Some(b'[' | b'{')) {
        return None;
    }
    let mut depth = 0usize;
    let mut end = None;
    scan_flow_yaml(source, start..source.len(), |index, byte| {
        match byte {
            b'[' | b'{' => depth += 1,
            b']' | b'}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    end = Some(index);
                    return true;
                }
            }
            _ => {}
        }
        false
    });
    end
}

/// The top-level `,`-separated entries of a raw YAML flow collection's
/// interior, with [`flow_collection_end`]'s quote and comment rules.
///
/// Only the YAML collection delimiters `[`/`{` nest. Parentheses are plain
/// scalar content, so `[a(b, c)d, v]` has three entries; the decoded
/// expression splitter ([`split_top_level`]) is the one that nests `(`/`)`.
pub(super) fn split_flow_entries(source: &str, range: Range<usize>) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut start = range.start;
    let mut depth = 0usize;
    scan_flow_yaml(source, range.clone(), |index, byte| {
        match byte {
            b'[' | b'{' => depth += 1,
            b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                spans.push(start..index);
                start = index + 1;
            }
            _ => {}
        }
        false
    });
    spans.push(start..range.end);
    spans
}

/// Calls `visit` with each byte of `range` outside quoted scalars and
/// comments, in raw YAML flow context, until `visit` returns `true`.
///
/// `range` must start where a scalar may begin (at or just inside a
/// collection's opening bracket). Scalar boundaries follow
/// [`scan_raw_yaml`]; a content `:` inside a plain scalar (`a:'b`,
/// `http://x`) is not passed to `visit`. The decoded schema-expression
/// scanner ([`scan_expression`]) does not use these rules.
pub(super) fn scan_flow_yaml(
    source: &str,
    range: Range<usize>,
    mut visit: impl FnMut(usize, u8) -> bool,
) {
    scan_raw_yaml(source, range, true, |index, byte, _| visit(index, byte));
}

/// Calls `visit` with each byte of `range` outside quoted scalars and
/// comments, and the YAML flow-collection depth before that byte, until
/// `visit` returns `true`. `flow` says whether `range` lies inside a flow
/// collection; otherwise it is block context and starts at depth `0`.
///
/// A quote opens a quoted scalar only where a scalar begins: at the start of
/// `range`, or after `[`, `{`, `,`, or a mapping value indicator, with
/// optional whitespace between. Scanning stops at an unterminated one. A `#`
/// after whitespace starts a comment that runs to the end of its line.
///
/// A `:` is a mapping value indicator when whitespace or the end of `range`
/// follows it, and in flow context also when a flow indicator follows it or
/// it directly follows a quoted scalar or collection (`{"a":b}`). Otherwise it
/// is content of a plain scalar and is not passed to `visit`, so the quote in
/// `a:'b` stays content. This is `serde_yaml_ng`'s reading. In block context,
/// `[` and `{` open a collection only where a scalar begins, and `,`, `]`, and
/// `}` are content outside one.
fn scan_raw_yaml(
    source: &str,
    range: Range<usize>,
    flow: bool,
    mut visit: impl FnMut(usize, u8, usize) -> bool,
) {
    let bytes = source.as_bytes();
    let mut depth = usize::from(flow);
    let mut scalar_start = true;
    let mut in_plain = false;
    let mut after_blank = true;
    let mut index = range.start;
    while index < range.end {
        let byte = bytes[index];
        if scalar_start && matches!(byte, b'\'' | b'"') {
            let Some(end) = quoted_flow_scalar_end(bytes, index, range.end) else {
                return;
            };
            index = end;
            scalar_start = false;
            in_plain = false;
            after_blank = false;
            continue;
        }
        if byte == b'#' && after_blank {
            while index < range.end && !matches!(bytes[index], b'\n' | b'\r') {
                index += 1;
            }
            in_plain = false;
            continue;
        }
        let in_flow = depth > 0;
        let depth_before = depth;
        match byte {
            _ if byte.is_ascii_whitespace() => {}
            b'[' | b'{' if in_flow || !in_plain => {
                depth += 1;
                scalar_start = true;
                in_plain = false;
            }
            b']' | b'}' if in_flow => {
                depth -= 1;
                scalar_start = false;
                in_plain = false;
            }
            b',' if in_flow => {
                scalar_start = true;
                in_plain = false;
            }
            b':' => {
                let next = bytes.get(index + 1).filter(|_| index + 1 < range.end);
                let indicator = next.is_none_or(u8::is_ascii_whitespace)
                    || (in_flow
                        && (!in_plain || next.is_some_and(|next| b",[]{}".contains(next))));
                if !indicator {
                    in_plain = true;
                    scalar_start = false;
                    after_blank = false;
                    index += 1;
                    continue;
                }
                scalar_start = true;
                in_plain = false;
            }
            _ => {
                scalar_start = false;
                in_plain = true;
            }
        }
        if visit(index, byte, depth_before) {
            return;
        }
        after_blank = byte.is_ascii_whitespace();
        index += 1;
    }
}

/// Just past the closing quote of the quoted scalar opening at `start`,
/// honoring `''` in single quotes and `\` escapes in double quotes.
pub(super) fn quoted_flow_scalar_end(bytes: &[u8], start: usize, end: usize) -> Option<usize> {
    let quote = bytes[start];
    let mut index = start + 1;
    while index < end {
        match bytes[index] {
            b'\\' if quote == b'"' => index += 1,
            b'\'' if quote == b'\'' && index + 1 < end && bytes[index + 1] == b'\'' => index += 1,
            byte if byte == quote => return Some(index + 1),
            _ => {}
        }
        index += 1;
    }
    None
}

fn is_block_scalar_header(source: &str) -> bool {
    let header = source.split('#').next().unwrap_or(source).trim();
    let mut chars = header.chars();
    matches!(chars.next(), Some('|' | '>'))
        && chars.all(|character| matches!(character, '+' | '-' | '1'..='9'))
}

fn sequence_content(content: &str) -> Option<usize> {
    let rest = content.strip_prefix('-')?;
    if rest.is_empty() {
        return Some(1);
    }
    let spaces = rest.bytes().take_while(|byte| byte.is_ascii_whitespace()).count();
    (spaces > 0).then_some(1 + spaces)
}

/// The byte offset of the `:` separating a raw YAML mapping entry's key from
/// its value.
///
/// `source` is one entry: a block mapping line without its indentation, or,
/// with `flow` set, one entry of a flow mapping, where an adjacent `:` after
/// a quoted or collection key separates (`"a":b`). A quoted or collection key
/// is skipped whole, and a `:` inside a plain scalar is content, so `a:b: c`
/// has the key `a:b` and `a:'b: c` the key `a:'b`.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::schemas::mapping_separator;
///
/// assert_eq!(mapping_separator("a:b: c", false), Some(3));
/// assert_eq!(mapping_separator("'x: y': z", false), Some(6));
/// assert_eq!(mapping_separator("http://x", false), None);
/// assert_eq!(mapping_separator("\"a\":b", true), Some(3));
/// ```
///
/// ## Returns
///
/// `None` when `source` holds no separator, including when a quoted scalar
/// is not closed.
pub fn mapping_separator(source: &str, flow: bool) -> Option<usize> {
    let top = usize::from(flow);
    let mut separator = None;
    scan_raw_yaml(source, 0..source.len(), flow, |index, byte, depth| {
        if byte == b':' && depth == top {
            separator = Some(index);
            return true;
        }
        false
    });
    separator
}

fn trim_range(source: &str, mut range: Range<usize>) -> Range<usize> {
    while range.start < range.end && source.as_bytes()[range.start].is_ascii_whitespace() {
        range.start += 1;
    }
    while range.end > range.start && source.as_bytes()[range.end - 1].is_ascii_whitespace() {
        range.end -= 1;
    }
    range
}

fn decoded_text(source: &str, range: &Range<usize>) -> Result<String, SchemaError> {
    yaml_scalar::decode_scalar_at(&source[range.clone()], range.start)
        .map(|(scalar, _)| scalar.decoded().to_string())
        .ok_or_else(projection_error)
}

fn project_schema_source(
    schema: &SimplifiedSchema,
    located: &LocatedValue,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    match (schema, &located.kind) {
        (SimplifiedSchema::Single(shape), LocatedKind::Mapping(_)) => project_shape_source(
            shape,
            located,
            &SchemaSourcePath::root(),
            source_offset,
            source_map,
        ),
        (SimplifiedSchema::Union(arms), LocatedKind::Sequence(items))
            if arms.len() == items.len() =>
        {
            let root = SchemaSourcePath::root();
            for (index, (arm, item)) in arms.iter().zip(items).enumerate() {
                let path = root.union_arm(index);
                source_map.insert(
                    &path,
                    SchemaSpanKind::UnionArm,
                    offset_span(&item.span, source_offset),
                );
                match arm {
                    SchemaArm::Inline(shape) => project_shape_source(
                        shape,
                        item,
                        &path,
                        source_offset,
                        source_map,
                    )?,
                    SchemaArm::FileRef(_) => record_scalar_content(
                        item,
                        &path,
                        SchemaSpanKind::FileReference,
                        source_offset,
                        source_map,
                    )?,
                }
            }
            Ok(())
        }
        (_, LocatedKind::Scalar(scalar)) if scalar.decoded().starts_with('*') => Ok(()),
        _ => Err(projection_error()),
    }
}

fn project_shape_source(
    shape: &SchemaShape,
    located: &LocatedValue,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let LocatedKind::Mapping(pairs) = &located.kind else {
        return Err(projection_error());
    };
    for pair in pairs {
        let property_path = path.property(&pair.key);
        source_map.insert(
            &property_path,
            SchemaSpanKind::MappingKey,
            offset_span(&pair.key_span, source_offset),
        );
        if pair.key == "$constraints" {
            project_mapping_constraints_source(
                &pair.value,
                &property_path,
                source_offset,
                source_map,
            )?;
            continue;
        }
        let definition = shape.properties.get(&pair.key).or_else(|| {
            super::PatternKey::parse(&pair.key).ok().and_then(|key| {
                shape
                    .pattern_keys
                    .iter()
                    .find(|pattern| pattern.key == key)
                    .map(|pattern| &pattern.def)
            })
        });
        let definition = definition.ok_or_else(projection_error)?;
        project_property_source(
            definition,
            &pair.value,
            &property_path,
            source_offset,
            source_map,
        )?;
    }
    Ok(())
}

fn project_mapping_constraints_source(
    located: &LocatedValue,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let LocatedKind::Mapping(pairs) = &located.kind else {
        return Err(projection_error());
    };
    for pair in pairs {
        let constraint_path = path.property(&pair.key);
        source_map.insert(
            &constraint_path,
            SchemaSpanKind::MappingKey,
            offset_span(&pair.key_span, source_offset),
        );
        source_map.insert(
            &constraint_path,
            SchemaSpanKind::Definition,
            offset_span(&pair.value.span, source_offset),
        );
        source_map.insert(
            &constraint_path,
            SchemaSpanKind::Constraint,
            offset_span(&pair.pair_span, source_offset),
        );
        if matches!(pair.key.as_str(), "min-keys" | "max-keys" | "min-items" | "max-items") {
            source_map.insert(
                &constraint_path,
                SchemaSpanKind::Argument,
                offset_span(&pair.value.span, source_offset),
            );
        }
    }
    Ok(())
}

fn project_property_source(
    definition: &PropertyDef,
    located: &LocatedValue,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    source_map.insert(
        path,
        SchemaSpanKind::Definition,
        offset_span(&located.span, source_offset),
    );
    if let LocatedKind::Scalar(scalar) = &located.kind
        && scalar.decoded().starts_with('*')
    {
        source_map.insert(
            path,
            SchemaSpanKind::Atom,
            offset_span(&located.span, source_offset),
        );
        return Ok(());
    }
    match (definition, &located.kind) {
        (PropertyDef::Single(atom), _) => {
            record_atom_span(located, path, source_offset, source_map)?;
            project_atom_source(atom, located, path, source_offset, source_map)
        }
        (PropertyDef::Union(_), LocatedKind::Scalar(scalar)) => scan_definition_range(
            definition,
            scalar,
            path,
            0..scalar.decoded().len(),
            source_offset,
            source_map,
        ),
        (PropertyDef::Union(atoms), LocatedKind::Sequence(items)) if atoms.len() == items.len() => {
            for (index, (atom, item)) in atoms.iter().zip(items).enumerate() {
                let arm_path = path.union_arm(index);
                let span = offset_span(&item.span, source_offset);
                source_map.insert(&arm_path, SchemaSpanKind::UnionArm, span.clone());
                source_map.insert(&arm_path, SchemaSpanKind::Definition, span);
                record_atom_span(item, &arm_path, source_offset, source_map)?;
                project_atom_source(atom, item, &arm_path, source_offset, source_map)?;
            }
            Ok(())
        }
        _ => Err(projection_error()),
    }
}

fn record_atom_span(
    located: &LocatedValue,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    match &located.kind {
        LocatedKind::Scalar(_) => Ok(()),
        LocatedKind::Mapping(_) | LocatedKind::Sequence(_) => {
            source_map.insert(
                path,
                SchemaSpanKind::Atom,
                offset_span(&located.span, source_offset),
            );
            Ok(())
        }
    }
}

fn project_atom_source(
    atom: &PropertyAtom,
    located: &LocatedValue,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    match (&atom.ty, &located.kind) {
        (TypeExpr::InlineObject(shape), LocatedKind::Mapping(_)) =>
            project_shape_source(shape, located, path, source_offset, source_map),
        (TypeExpr::InlineObject(shape), LocatedKind::Scalar(scalar)) => {
            scan_expression_source(scalar, path, source_offset, source_map)?;
            project_inline_shape_source(shape, scalar, path, source_offset, source_map)
        }
        // The YAML null scalar is the `null` type; its whole scalar (empty
        // for `key:`) is the keyword.
        (TypeExpr::Primitive(SimplifiedType::Null), LocatedKind::Scalar(scalar))
            if is_yaml_null_spelling(scalar.decoded()) =>
        {
            let span = offset_span(&located.span, source_offset);
            source_map.insert(path, SchemaSpanKind::Atom, span.clone());
            source_map.insert(path, SchemaSpanKind::TypeKeyword, span);
            Ok(())
        }
        (_, LocatedKind::Scalar(scalar)) => {
            scan_expression_source(scalar, path, source_offset, source_map)
        }
        _ => Err(projection_error()),
    }
}

/// Whether `text` is a plain YAML 1.2 core-schema null scalar.
fn is_yaml_null_spelling(text: &str) -> bool {
    matches!(text, "" | "~" | "null" | "Null" | "NULL")
}

fn record_scalar_content(
    located: &LocatedValue,
    path: &SchemaSourcePath,
    kind: SchemaSpanKind,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let LocatedKind::Scalar(scalar) = &located.kind else {
        return Err(projection_error());
    };
    let span = scalar
        .project(0..scalar.decoded().len())
        .ok_or_else(projection_error)?;
    source_map.insert(path, kind, offset_span(&span, source_offset));
    Ok(())
}

fn scan_expression_source(
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let source = scalar.decoded();
    scan_expression_range(
        scalar,
        path,
        0..source.len(),
        source_offset,
        source_map,
    )
}

fn scan_expression_range(
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    range: Range<usize>,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let source = scalar.decoded();
    let arrow = find_top_level_arrow(source, range.clone());
    let expression = trim_local(source, range.start..arrow.unwrap_or(range.end));
    let atom = scalar.project(expression.clone()).ok_or_else(projection_error)?;
    source_map.insert(path, SchemaSpanKind::Atom, offset_span(&atom, source_offset));

    if source.as_bytes().get(expression.start) == Some(&b'{') {
        return scan_constraint_groups(scalar, path, expression, source_offset, source_map);
    }
    // A grouped union, `(A | B)[]`, names no single type and has no
    // constraint list of its own.
    if source.as_bytes().get(expression.start) == Some(&b'(') {
        return Ok(());
    }

    let name = leading_identifier(source, expression.clone()).ok_or_else(projection_error)?;
    if let Some(at) = find_top_level_byte(
        source,
        expression.clone(),
        ExpressionContext::Expression,
        b'@',
    ) {
        insert_projected(
            scalar,
            path,
            SchemaSpanKind::ImportName,
            name,
            source_offset,
            source_map,
        )?;
        let reference = trim_local(source, at + 1..expression.end);
        insert_projected(
            scalar,
            path,
            SchemaSpanKind::ImportReference,
            reference,
            source_offset,
            source_map,
        )?;
    } else {
        // A bare non-keyword name is a local reference and shares the span
        // kind of an explicit reference's name.
        let kind = if SimplifiedType::is_keyword(&source[name.clone()]) {
            SchemaSpanKind::TypeKeyword
        } else {
            SchemaSpanKind::ImportName
        };
        insert_projected(scalar, path, kind, name, source_offset, source_map)?;
    }
    scan_constraint_groups(scalar, path, expression, source_offset, source_map)
}

fn project_inline_shape_source(
    shape: &SchemaShape,
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let source = scalar.decoded();
    project_inline_shape_range(
        shape,
        scalar,
        path,
        0..source.len(),
        source_offset,
        source_map,
    )
}

fn project_inline_shape_range(
    shape: &SchemaShape,
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    range: Range<usize>,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let source = scalar.decoded();
    let expression_end = find_top_level_arrow(source, range.clone()).unwrap_or(range.end);
    let expression = trim_local(source, range.start..expression_end);
    let close = inline_object_end(source, expression.clone()).ok_or_else(projection_error)?;
    let body = expression.start + 1..close;
    for pair in split_top_level(source, body, ExpressionContext::ObjectBody, b',') {
        let pair = trim_local(source, pair);
        if pair.start == pair.end {
            continue;
        }
        let colon = find_top_level_byte(source, pair.clone(), ExpressionContext::ObjectBody, b':')
            .ok_or_else(projection_error)?;
        let key_span = trim_local(source, pair.start..colon);
        let key = source[key_span.clone()].to_string();
        let property_path = path.property(&key);
        insert_projected(
            scalar,
            &property_path,
            SchemaSpanKind::MappingKey,
            key_span,
            source_offset,
            source_map,
        )?;
        let definition_span = trim_local(source, colon + 1..pair.end);
        insert_projected(
            scalar,
            &property_path,
            SchemaSpanKind::Definition,
            definition_span.clone(),
            source_offset,
            source_map,
        )?;
        let definition = shape.properties.get(&key).or_else(|| {
            super::PatternKey::parse(&key).ok().and_then(|pattern_key| {
                shape
                    .pattern_keys
                    .iter()
                    .find(|pattern| pattern.key == pattern_key)
                    .map(|pattern| &pattern.def)
            })
        });
        scan_definition_range(
            definition.ok_or_else(projection_error)?,
            scalar,
            &property_path,
            definition_span,
            source_offset,
            source_map,
        )?;
    }
    Ok(())
}

/// Records the spans of the definition written in `range` of `scalar`: one
/// type expression, or a `|` union whose options are recorded as union arms.
/// A union whose written options do not line up with its parsed ones (a
/// parenthesized group joins its options to the union) records only its
/// whole span.
fn scan_definition_range(
    definition: &PropertyDef,
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    range: Range<usize>,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let source = scalar.decoded();
    let atoms = match definition {
        PropertyDef::Single(atom) => {
            return scan_atom_range(atom, scalar, path, range, source_offset, source_map);
        }
        PropertyDef::Union(atoms) => atoms,
    };
    let arms = split_top_level(source, range.clone(), ExpressionContext::Expression, b'|');
    if arms.len() != atoms.len() {
        return insert_projected(
            scalar,
            path,
            SchemaSpanKind::Atom,
            trim_local(source, range),
            source_offset,
            source_map,
        );
    }
    for (index, (atom, arm)) in atoms.iter().zip(arms).enumerate() {
        let arm = trim_local(source, arm);
        let arm_path = path.union_arm(index);
        insert_projected(scalar, &arm_path, SchemaSpanKind::UnionArm, arm.clone(), source_offset, source_map)?;
        insert_projected(scalar, &arm_path, SchemaSpanKind::Definition, arm.clone(), source_offset, source_map)?;
        scan_atom_range(atom, scalar, &arm_path, arm, source_offset, source_map)?;
    }
    Ok(())
}

fn scan_atom_range(
    atom: &PropertyAtom,
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    range: Range<usize>,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    scan_expression_range(scalar, path, range.clone(), source_offset, source_map)?;
    let source = scalar.decoded();
    if let TypeExpr::InlineObject(nested) = &atom.ty
        && source.as_bytes().get(trim_local(source, range.clone()).start) == Some(&b'{')
    {
        project_inline_shape_range(nested, scalar, path, range, source_offset, source_map)?;
    }
    Ok(())
}

/// Records the constraints and arguments of every `(…)` constraint list in
/// `range`, a type expression without its description. Lists inside an inline
/// object belong to its properties and are skipped here.
fn scan_constraint_groups(
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    range: Range<usize>,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let source = scalar.decoded();
    let mut groups = Vec::new();
    let mut open = None;
    scan_expression(source, range, ExpressionContext::Expression, |index, byte, level| {
        if level.objects > 0 {
            return false;
        }
        match (byte, level.groups) {
            (b'(', 0) => open = Some(index),
            (b')', 1) => groups.extend(open.take().map(|open| open..index)),
            _ => {}
        }
        false
    });
    if open.is_some() {
        return Err(projection_error());
    }
    for group in groups {
        let list = group.start + 1..group.end;
        for constraint in split_top_level(source, list, ExpressionContext::Arguments, b';') {
            let constraint = trim_local(source, constraint);
            if constraint.start == constraint.end {
                continue;
            }
            insert_projected(
                scalar,
                path,
                SchemaSpanKind::Constraint,
                constraint.clone(),
                source_offset,
                source_map,
            )?;
            let Some(arguments) = constraint_call_arguments(source, constraint)? else {
                continue;
            };
            for argument in split_top_level(source, arguments, ExpressionContext::Arguments, b',') {
                let argument = trim_local(source, argument);
                if argument.start < argument.end {
                    insert_projected(
                        scalar,
                        path,
                        SchemaSpanKind::Argument,
                        argument,
                        source_offset,
                        source_map,
                    )?;
                }
            }
        }
    }
    Ok(())
}

/// The interior of the argument list of the constraint call in `constraint`
/// (text inside a constraint list), or `None` for a bare keyword or positional
/// members such as `enum('a(b', c)`'s, whose quoted `(` is argument text.
fn constraint_call_arguments(
    source: &str,
    constraint: Range<usize>,
) -> Result<Option<Range<usize>>, SchemaError> {
    let mut open = None;
    let mut close = None;
    scan_expression(source, constraint, ExpressionContext::Arguments, |index, byte, level| {
        match (byte, level.groups) {
            (b'(', 1) if open.is_none() => open = Some(index),
            (b')', 2) if open.is_some() => {
                close = Some(index);
                return true;
            }
            _ => {}
        }
        false
    });
    match (open, close) {
        (None, _) => Ok(None),
        (Some(open), Some(close)) => Ok(Some(open + 1..close)),
        (Some(_), None) => Err(projection_error()),
    }
}

fn insert_projected(
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    kind: SchemaSpanKind,
    span: Range<usize>,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let span = scalar.project(span).ok_or_else(projection_error)?;
    source_map.insert(path, kind, offset_span(&span, source_offset));
    Ok(())
}

fn leading_identifier(source: &str, range: Range<usize>) -> Option<Range<usize>> {
    let mut end = range.start;
    while end < range.end {
        let byte = source.as_bytes()[end];
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') {
            end += 1;
        } else {
            break;
        }
    }
    (end > range.start).then_some(range.start..end)
}

/// The `->` opening the top-level description of the type expression in
/// `range`.
fn find_top_level_arrow(source: &str, range: Range<usize>) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut found = None;
    scan_expression(source, range, ExpressionContext::Expression, |index, byte, level| {
        if level.is_top() && byte == b'-' && bytes.get(index + 1) == Some(&b'>') {
            found = Some(index);
            return true;
        }
        false
    });
    found
}

/// The first `needle` read as structure outside every inline object and
/// constraint list in `range`.
fn find_top_level_byte(
    source: &str,
    range: Range<usize>,
    context: ExpressionContext,
    needle: u8,
) -> Option<usize> {
    let mut found = None;
    scan_expression(source, range, context, |index, byte, level| {
        if level.is_top() && byte == needle {
            found = Some(index);
            return true;
        }
        false
    });
    found
}

/// The `}` closing the inline object that opens at `range.start`.
fn inline_object_end(source: &str, range: Range<usize>) -> Option<usize> {
    let mut found = None;
    scan_expression(source, range, ExpressionContext::Expression, |index, byte, level| {
        if byte == b'}' && level.objects == 1 && level.groups == 0 {
            found = Some(index);
            return true;
        }
        false
    });
    found
}

/// Splits `range` at each `separator` read as structure at the level where
/// `range` starts: between properties for [`ExpressionContext::ObjectBody`],
/// between constraints or arguments for [`ExpressionContext::Arguments`].
/// Raw YAML uses [`split_flow_entries`].
fn split_top_level(
    source: &str,
    range: Range<usize>,
    context: ExpressionContext,
    separator: u8,
) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut start = range.start;
    scan_expression(source, range.clone(), context, |index, byte, level| {
        if byte == separator && level.is_top() {
            spans.push(start..index);
            start = index + 1;
        }
        false
    });
    spans.push(start..range.end);
    spans
}

/// Where a [`scan_expression`] range sits in the decoded type-expression
/// grammar, which decides where a description ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExpressionContext {
    /// A whole type expression. A top-level description runs to the end of
    /// the range, as the parser reads it.
    Expression,
    /// A whole type expression written as a plain arm of a raw YAML flow
    /// sequence, where YAML ends a top-level description at the next `,` or
    /// `]`.
    FlowArm,
    /// The body of an inline object, so a description ends at the object's
    /// next `,` or `}`.
    ObjectBody,
    /// The inside of a constraint or argument list.
    Arguments,
}

/// The structural nesting before a byte passed to [`scan_expression`],
/// relative to the start of the scanned range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExpressionLevel {
    /// Enclosing inline objects `{ … }`.
    pub(super) objects: usize,
    /// Enclosing constraint and argument lists `( … )`, counting the list an
    /// [`ExpressionContext::Arguments`] range starts inside.
    pub(super) groups: usize,
    base_groups: usize,
}

impl ExpressionLevel {
    /// Whether the byte is at the level where the scanned range starts.
    pub(super) fn is_top(self) -> bool {
        self.objects == 0 && self.groups == self.base_groups
    }
}

/// Calls `visit` with each byte of `range` that the type-expression grammar
/// reads as structure, and the nesting before it, until `visit` returns `true`.
///
/// The grammar has one lexical mode per kind of text, and this scanner follows
/// the same mode the parser is in rather than inferring one from delimiter
/// depth:
///
/// - **Type text** (`{`, `}`, `[]`, `(`, `@`, `->`, `,`, `:`, identifiers) is
///   visited. `{` opens an inline object and `(` a constraint list.
/// - **Argument lists**, inside a constraint list's `(…)`: `(`/`)` nest and a
///   quote opens a string the lexer reads with `\` escapes, so
///   `suggest('a)b', c)` holds two arguments. `[`, `{`, and the rest are
///   argument text. String contents are not visited.
/// - **Descriptions** after `->` are prose, never visited: quotes and brackets
///   in them mean nothing, so `(it's fine)` and `plain [x` hide no boundary.
///   Inside an inline object one ends where
///   [`grammar::inline_description_end`] ends it.
/// - **Imported file references** after `@` are opaque, never visited, and
///   end where [`grammar::file_reference_end`] ends them, so
///   `Name@./a{b.yaml -> d` has a top-level `->`.
/// - **Pattern keys** `<…>` are opaque to their first `>`.
///
/// The byte that ends a description or reference is visited as type text.
/// Scanning stops at an unterminated string or pattern key, and at an
/// unbalanced `)` in a description, which the parser rejects.
pub(super) fn scan_expression(
    source: &str,
    range: Range<usize>,
    context: ExpressionContext,
    mut visit: impl FnMut(usize, u8, ExpressionLevel) -> bool,
) {
    let bytes = source.as_bytes();
    let end = range.end;
    let base_groups = usize::from(context == ExpressionContext::Arguments);
    let mut level = ExpressionLevel {
        objects: 0,
        groups: base_groups,
        base_groups,
    };
    let mut index = range.start;
    while index < end {
        let byte = bytes[index];
        if level.groups > 0 {
            if matches!(byte, b'\'' | b'"') {
                let Some(next) = grammar::quoted_end(bytes, index, end) else {
                    return;
                };
                index = next;
                continue;
            }
            if visit(index, byte, level) {
                return;
            }
            match byte {
                b'(' => level.groups += 1,
                b')' => level.groups -= 1,
                _ => {}
            }
            index += 1;
            continue;
        }
        if visit(index, byte, level) {
            return;
        }
        index = match byte {
            b'(' => {
                level.groups = 1;
                index + 1
            }
            b'{' => {
                level.objects += 1;
                index + 1
            }
            b'}' => {
                level.objects = level.objects.saturating_sub(1);
                index + 1
            }
            b'<' => match grammar::pattern_key_end(bytes, index, end) {
                Some(next) => next,
                None => return,
            },
            b'@' => grammar::file_reference_end(bytes, index + 1, end),
            b'-' if bytes.get(index + 1) == Some(&b'>') => {
                let start = index + 2;
                if level.objects > 0 || context == ExpressionContext::ObjectBody {
                    match grammar::inline_description_end(bytes, start, end) {
                        Ok(next) => next,
                        Err(_) => return,
                    }
                } else if context == ExpressionContext::FlowArm {
                    (start..end)
                        .find(|&next| matches!(bytes[next], b',' | b']'))
                        .unwrap_or(end)
                } else {
                    end
                }
            }
            _ => index + 1,
        };
    }
}

fn trim_local(source: &str, mut range: Range<usize>) -> Range<usize> {
    while range.start < range.end && source.as_bytes()[range.start].is_ascii_whitespace() {
        range.start += 1;
    }
    while range.end > range.start && source.as_bytes()[range.end - 1].is_ascii_whitespace() {
        range.end -= 1;
    }
    range
}

fn offset_span(span: &Range<usize>, offset: usize) -> Range<usize> {
    offset + span.start..offset + span.end
}

/// Parses a SimplifiedSchema value and projects suggestion spans into its YAML
/// source.
///
/// `source_offset` is added after YAML scalar projection. Pass zero for a
/// standalone YAML buffer, or the frontmatter YAML block's document offset for
/// inline Markdown. The source is never line-ending-normalized, and must start
/// at a line boundary or be dedented (see [`locate_schema_value`]).
pub fn parse_yaml_schema_with_source(
    value: &YamlValue,
    yaml_source: &str,
    source_offset: usize,
) -> Result<SimplifiedSchema, SchemaError> {
    let mut schema = parse_yaml_schema(value)?;
    project_suggestion_spans(&mut schema, yaml_source, source_offset)?;
    Ok(schema)
}

/// Projects expression-relative suggestion spans through YAML scalar quoting
/// and escaping into byte ranges in the caller's source document.
///
/// Plain, single-quoted, and double-quoted YAML scalars are supported, including
/// a plain scalar continued onto more-indented lines, CRLF input, and UTF-8
/// text. A projection mismatch is a grammar error rather than a silently
/// approximate range.
///
/// Each suggestion-bearing atom takes the first later scalar in source order
/// that parses to the same atom. Properties are visited before pattern keys,
/// so an identical pattern-key definition authored before a property can take
/// the property's spans; the standalone loader projects through each
/// definition's located scalar instead.
pub fn project_suggestion_spans(
    schema: &mut SimplifiedSchema,
    yaml_source: &str,
    source_offset: usize,
) -> Result<(), SchemaError> {
    let scalars = scan_value_scalars(yaml_source);
    let mut projector = Projector {
        scalars: &scalars,
        next_scalar: 0,
        source_offset,
    };
    match schema {
        SimplifiedSchema::Single(shape) => projector.shape(shape),
        SimplifiedSchema::Union(arms) => {
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    projector.shape(shape)?;
                }
            }
            Ok(())
        }
    }
}

struct Projector<'a> {
    scalars: &'a [DecodedScalar],
    next_scalar: usize,
    source_offset: usize,
}

impl Projector<'_> {
    fn shape(&mut self, shape: &mut SchemaShape) -> Result<(), SchemaError> {
        for def in shape.properties.values_mut() {
            self.property(def)?;
        }
        for pattern in &mut shape.pattern_keys {
            self.property(&mut pattern.def)?;
        }
        Ok(())
    }

    fn property(&mut self, def: &mut PropertyDef) -> Result<(), SchemaError> {
        match def {
            PropertyDef::Single(atom) => self.atom(atom),
            PropertyDef::Union(atoms) => {
                for atom in atoms {
                    self.atom(atom)?;
                }
                Ok(())
            }
        }
    }

    fn atom(&mut self, atom: &mut PropertyAtom) -> Result<(), SchemaError> {
        if atom_has_suggestions(atom)
            && let Some((scalar_index, scalar)) = self.scalars[self.next_scalar..]
                .iter()
                .enumerate()
                .find(|(_, scalar)| scalar_matches_atom(scalar, atom))
                .map(|(relative, scalar)| (self.next_scalar + relative, scalar))
        {
            self.next_scalar = scalar_index + 1;
            return project_atom_suggestions(atom, scalar, self.source_offset);
        }

        if let TypeExpr::InlineObject(shape) = &mut atom.ty {
            self.shape(shape)?;
        }
        if atom.constraints.iter().any(|constraint| matches!(constraint, Constraint::Suggest(_))) {
            return Err(projection_error());
        }
        Ok(())
    }
}

fn atom_has_suggestions(atom: &PropertyAtom) -> bool {
    atom.constraints.iter().any(|constraint| matches!(constraint, Constraint::Suggest(_)))
        || match &atom.ty {
            TypeExpr::InlineObject(shape) => shape.properties.values().any(property_has_suggestions)
                || shape.pattern_keys.iter().any(|pattern| property_has_suggestions(&pattern.def)),
            TypeExpr::Union(arms) => arms.iter().any(atom_has_suggestions),
            TypeExpr::Primitive(_)
            | TypeExpr::Imported { .. }
            | TypeExpr::Tuple(_)
            | TypeExpr::Function(_)
            | TypeExpr::Ref(_) => false,
        }
}

fn property_has_suggestions(def: &PropertyDef) -> bool {
    match def {
        PropertyDef::Single(atom) => atom_has_suggestions(atom),
        PropertyDef::Union(atoms) => atoms.iter().any(atom_has_suggestions),
    }
}

fn scalar_matches_atom(scalar: &DecodedScalar, expected: &PropertyAtom) -> bool {
    super::grammar::parse_type_expr("<source>", scalar.decoded())
        .is_ok_and(|actual| actual == *expected)
}

fn project_atom_suggestions(
    atom: &mut PropertyAtom,
    scalar: &DecodedScalar,
    source_offset: usize,
) -> Result<(), SchemaError> {
    if let TypeExpr::InlineObject(shape) = &mut atom.ty {
        for def in shape.properties.values_mut() {
            project_property_suggestions(def, scalar, source_offset)?;
        }
        for pattern in &mut shape.pattern_keys {
            project_property_suggestions(&mut pattern.def, scalar, source_offset)?;
        }
    }
    if let Some(candidates) = atom.constraints.iter_mut().find_map(|constraint| {
        if let Constraint::Suggest(candidates) = constraint {
            Some(candidates)
        } else {
            None
        }
    }) {
        for candidate in candidates {
            let span = scalar.project(candidate.span.clone()).ok_or_else(projection_error)?;
            candidate.span = offset_span(&span, source_offset);
        }
    }
    Ok(())
}

fn project_property_suggestions(
    def: &mut PropertyDef,
    scalar: &DecodedScalar,
    source_offset: usize,
) -> Result<(), SchemaError> {
    match def {
        PropertyDef::Single(atom) => project_atom_suggestions(atom, scalar, source_offset),
        PropertyDef::Union(atoms) => {
            for atom in atoms {
                project_atom_suggestions(atom, scalar, source_offset)?;
            }
            Ok(())
        }
    }
}

fn projection_error() -> SchemaError {
    SchemaError::Grammar {
        property: "<source>".into(),
        message: "could not project SimplifiedSchema expression spans through YAML source".into(),
        span: 0..0,
    }
}

/// Every block mapping or sequence value in `source` that may be a type
/// expression, decoded, in source order.
///
/// A plain scalar continued onto more-indented lines is decoded whole and its
/// continuation lines are not read again as values of their own; flow
/// sequences and quoted scalars are read on their first line.
fn scan_value_scalars(source: &str) -> Vec<DecodedScalar> {
    let mut scalars = Vec::new();
    let mut line_start = 0;
    let mut consumed_to = 0;
    for line_with_ending in source.split_inclusive('\n') {
        let start = line_start;
        line_start += line_with_ending.len();
        if start < consumed_to {
            continue;
        }
        let line = line_with_ending
            .strip_suffix('\n')
            .unwrap_or(line_with_ending);
        let line = line.strip_suffix('\r').unwrap_or(line);
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // The value's start and the column of the mapping key or `-` holding
        // it, which a continuation line must be indented past.
        let value_at = if let Some(rest) = trimmed.strip_prefix('-') {
            if rest.starts_with(char::is_whitespace) {
                let leading = rest.len() - rest.trim_start().len();
                let item = rest.trim_start();
                let item_at = indent + 1 + leading;
                mapping_value_offset(item)
                    .map(|offset| {
                        let value = &item[offset..];
                        (item_at + offset + (value.len() - value.trim_start().len()), item_at)
                    })
                    .or(Some((item_at, indent)))
            } else {
                None
            }
        } else {
            mapping_value_offset(line).map(|offset| {
                let rest = &line[offset..];
                (offset + (rest.len() - rest.trim_start().len()), indent)
            })
        };
        if let Some((value_at, holder)) = value_at.filter(|(offset, _)| *offset < line.len())
            && let Some(end) = scan_value(source, &line[value_at..], start + value_at, holder, &mut scalars)
        {
            consumed_to = end;
        }
    }
    // `split_inclusive` produces no item for an empty trailing remainder, and
    // handles a final line without `\n` in the ordinary case above.
    scalars
}

/// Just past the key/value separator of a raw block YAML line, as found by
/// [`mapping_separator`].
fn mapping_value_offset(line: &str) -> Option<usize> {
    mapping_separator(line, false).map(|colon| colon + 1)
}

/// Decodes the value at `base`, whose first line is `raw`. Returns where a
/// plain scalar decoded through [`yaml_scalar::decode_scalar_node`] ends.
fn scan_value(
    source: &str,
    raw: &str,
    base: usize,
    holder: usize,
    out: &mut Vec<DecodedScalar>,
) -> Option<usize> {
    if raw.starts_with('[') {
        let mut offset = 1;
        while offset < raw.len() {
            offset += raw[offset..].len() - raw[offset..].trim_start().len();
            if raw[offset..].starts_with(']') {
                break;
            }
            if let Some((scalar, consumed)) = yaml_scalar::decode_scalar_at(&raw[offset..], base + offset) {
                out.push(scalar);
                offset += consumed;
            } else {
                break;
            }
            if let Some(comma) = raw[offset..].find(',') {
                offset += comma + 1;
            } else {
                break;
            }
        }
        return None;
    }
    let plain = !is_block_scalar_header(raw) && !raw.starts_with(['\'', '"', '{', '!', '&', '*']);
    if plain && let Some((scalar, end)) = yaml_scalar::decode_scalar_node(source, base, holder) {
        out.push(scalar);
        return Some(end);
    }
    if let Some((scalar, _)) = yaml_scalar::decode_scalar_at(raw, base) {
        out.push(scalar);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate_span(yaml: &str, offset: usize) -> std::ops::Range<usize> {
        let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
        let mut schema = parse_yaml_schema_with_source(&value, yaml, offset).unwrap();
        let SimplifiedSchema::Single(shape) = &mut schema else {
            panic!("expected single shape");
        };
        let PropertyDef::Single(atom) = &shape.properties["value"] else {
            panic!("expected single atom");
        };
        let Constraint::Suggest(candidates) = atom
            .constraints
            .iter()
            .find(|constraint| matches!(constraint, Constraint::Suggest(_)))
            .unwrap()
        else {
            unreachable!()
        };
        candidates[1].span.clone()
    }

    /// A [`SchemaConstraintOrigin`] address resolves to the authored text of
    /// that constraint in plain, quoted, and `$constraints` presentations;
    /// `min` never selects `min-keys`, and `occurrence` counts per keyword.
    #[test]
    fn constraint_span_resolves_an_origin_address_in_each_presentation() {
        let yaml = "plain: string(min(3); max(9))[](min(1))\n\
                    quoted: \"number(max(5))\"\n\
                    box:\n  $constraints:\n    min-keys: 3\n  a: string\n\
                    keyed: \"{ a: string }(min-keys(2))\"\n";
        let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
        let declared = parse_schema_declaration_with_source(&value, yaml, 0).unwrap();
        let map = &declared.source_map;
        let text = |path: &SchemaSourcePath, keyword: &str, occurrence: usize| {
            map.constraint_span(yaml, path, keyword, occurrence)
                .map(|span| &yaml[span])
        };
        let root = SchemaSourcePath::root();
        let plain = root.property("plain");
        assert_eq!(text(&plain, "min", 0), Some("min(3)"));
        assert_eq!(text(&plain, "min", 1), Some("min(1)"), "array-level group follows");
        assert_eq!(text(&plain, "max", 0), Some("max(9)"));
        assert_eq!(text(&plain, "min", 2), None);
        assert_eq!(text(&root.property("quoted"), "max", 0), Some("max(5)"));
        let block = root.property("box").property("$constraints").property("min-keys");
        assert_eq!(text(&block, "min-keys", 0), Some("min-keys: 3"));
        let keyed = root.property("keyed");
        assert_eq!(text(&keyed, "min-keys", 0), Some("min-keys(2)"));
        assert_eq!(text(&keyed, "min", 0), None, "`min` is a whole word");
    }

    #[test]
    fn schema_source_path_and_id_render_for_messages() {
        let root = SchemaSourcePath::root();
        assert_eq!(root.to_string(), "<root>");
        assert_eq!(root.property("a").union_arm(1).property("b").to_string(), "a[1].b");
        assert_eq!(SchemaSourceId::InlineDocument.to_string(), "the inline `$schema`");
        let file = std::path::PathBuf::from("types.yaml");
        assert_eq!(SchemaSourceId::File(file).to_string(), "`types.yaml`");
        let embedded = SchemaSourceId::Embedded("darkmatter/schemas/partials/fm.yaml".into());
        assert_eq!(embedded.to_string(), "embedded `darkmatter/schemas/partials/fm.yaml`");
    }

    #[test]
    fn mapping_separator_treats_a_quote_as_quoting_only_where_a_scalar_begins() {
        for (line, expected) in [
            ("it's: x", Some(4)),
            ("say \"hi: x", Some(7)),
            ("a 'b: c'", Some(4)),
            ("'a:b': c", Some(5)),
            ("'it''s: x': y", Some(10)),
            ("\"a\\\": b\": c", Some(8)),
            ("[a, 'b: c']: x", Some(11)),
            ("{k: 'v: w'}: x", Some(11)),
            ("'unterminated: x", None),
            ("plain scalar", None),
        ] {
            assert_eq!(mapping_separator(line, false), expected, "{line:?}");
        }
    }

    /// A `:` separates a key only as a YAML value indicator; inside a plain
    /// scalar it is content, and the quote after it stays content too.
    #[test]
    fn mapping_separator_skips_content_colons_inside_plain_scalars() {
        for (line, flow, expected) in [
            // Block context: only `: ` or a line-ending `:` separates.
            ("a:b: c", false, Some(3)),
            ("a:'b: c", false, Some(4)),
            ("a:\"b: c", false, Some(4)),
            ("url: http://x:8080", false, Some(3)),
            ("key:", false, Some(3)),
            ("key:value", false, None),
            ("'a':b", false, None),
            ("a #b: c", false, None),
            // Flow context adds a `:` right after a quoted or collection key.
            ("a:'b: \"v\"", true, Some(4)),
            ("a:\"b: 'v'", true, Some(4)),
            ("\"a\":b", true, Some(3)),
            ("'a' :b", true, Some(4)),
            ("[x]:b", true, Some(3)),
            ("a :b", true, None),
            ("http://x", true, None),
        ] {
            assert_eq!(mapping_separator(line, flow), expected, "{line:?} flow={flow}");
        }
    }

    /// Renders a located node as its authored scalars, `[a|b]` sequences, and
    /// `{key=value|…}` mappings, so one string pins structure and spans.
    fn located_shape(source: &str, node: &SchemaValueNode) -> String {
        let join = |parts: Vec<String>| parts.join("|");
        match &node.kind {
            SchemaValueKind::Scalar => source[node.span.clone()].to_string(),
            SchemaValueKind::Sequence(items) => format!(
                "[{}]",
                join(items.iter().map(|item| located_shape(source, item)).collect())
            ),
            SchemaValueKind::Mapping(entries) => format!(
                "{{{}}}",
                join(
                    entries
                        .iter()
                        .map(|entry| format!("{}={}", entry.key, located_shape(source, &entry.value)))
                        .collect()
                )
            ),
        }
    }

    #[test]
    fn raw_flow_collections_open_a_quote_only_where_a_scalar_begins() {
        for (yaml, expected) in [
            // Controls: ordinary plain siblings, genuinely quoted spellings,
            // and delimiters inside real quoted scalars.
            ("[ordinary, \"v\"]", "[ordinary|\"v\"]"),
            ("['don''t', \"v\"]", "['don''t'|\"v\"]"),
            ("{'don''t': \"v\"}", "{don't=\"v\"}"),
            ("[\"a, b\", 'c'']d', \"e\\\"]f\"]", "[\"a, b\"|'c'']d'|\"e\\\"]f\"]"),
            // A quote inside a plain scalar is content.
            ("[don't, \"v\"]", "[don't|\"v\"]"),
            ("[say \"hi, 'v']", "[say \"hi|'v']"),
            ("[don't, can't, \"v\"]", "[don't|can't|\"v\"]"),
            ("{don't: \"v\"}", "{don't=\"v\"}"),
            ("{say \"hi: 'v'}", "{say \"hi='v'}"),
            ("{a: don't, b: \"v\"}", "{a=don't|b=\"v\"}"),
            ("{a: say \"hi, b: 'v'}", "{a=say \"hi|b='v'}"),
            // Nested collections take the same rule.
            ("{list: [don't, \"v\"], z: 1}", "{list=[don't|\"v\"]|z=1}"),
            ("[{k: don't}, {k: 'v'}]", "[{k=don't}|{k='v'}]"),
            ("[don't, {can't: \"v\"}]", "[don't|{can't=\"v\"}]"),
            ("key: [don't, can't, \"v\"]", "{key=[don't|can't|\"v\"]}"),
            ("key: {a: say \"hi, b: 'v'}", "{key={a=say \"hi|b='v'}}"),
        ] {
            let node = locate_schema_value(yaml, 0)
                .unwrap_or_else(|| panic!("{yaml:?} was not located"));
            assert_eq!(located_shape(yaml, &node), expected, "{yaml:?}");
        }
    }

    /// Every row is first read by `serde_yaml_ng`, whose structure the
    /// located shape must match, under LF and CRLF.
    #[test]
    fn a_content_colon_does_not_reopen_quote_mode_in_raw_flow_yaml() {
        for (flow, expected) in [
            // Controls: ordinary and `don't` siblings, genuinely quoted
            // colons, and adjacent JSON-like keys, which do separate.
            ("[ordinary, \"v\"]", "[ordinary|\"v\"]"),
            ("[don't, \"v\"]", "[don't|\"v\"]"),
            ("['a:''b', \"v\"]", "['a:''b'|\"v\"]"),
            ("[\"a:b\", 'v']", "[\"a:b\"|'v']"),
            ("{\"a\":b, c: 'v'}", "{a=b|c='v'}"),
            ("{'a':'v'}", "{a='v'}"),
            ("[http://x, 'v']", "[http://x|'v']"),
            ("{a:b: c, d: 'v'}", "{a:b=c|d='v'}"),
            // A quote after a content colon is content.
            ("[a:'b, \"v\"]", "[a:'b|\"v\"]"),
            ("[a:\"b, 'v']", "[a:\"b|'v']"),
            ("[a:'b, c:'d, \"v\"]", "[a:'b|c:'d|\"v\"]"),
            ("[a:\"b, c:\"d, 'v']", "[a:\"b|c:\"d|'v']"),
            ("{a:'b: \"v\"}", "{a:'b=\"v\"}"),
            ("{a:\"b: 'v'}", "{a:\"b='v'}"),
            ("{a: a:'b, b: \"v\"}", "{a=a:'b|b=\"v\"}"),
            ("{a: a:\"b, b: 'v'}", "{a=a:\"b|b='v'}"),
            // Nested collections take the same rule.
            ("{list: [a:'b, \"v\"]}", "{list=[a:'b|\"v\"]}"),
            ("{list: [a:\"b, c:\"d, 'v'], z: 1}", "{list=[a:\"b|c:\"d|'v']|z=1}"),
            ("[{k: a:'b}, {k: 'v'}]", "[{k=a:'b}|{k='v'}]"),
            ("[a:'b, {c:'d: \"v\"}]", "[a:'b|{c:'d=\"v\"}]"),
        ] {
            let parsed: YamlValue = serde_yaml_ng::from_str(flow)
                .unwrap_or_else(|error| panic!("{flow:?} is not YAML: {error}"));
            for newline in ["\n", "\r\n"] {
                let yaml = format!("a:'b: {flow}{newline}q: {flow}{newline}");
                let node = locate_schema_value(&yaml, 0)
                    .unwrap_or_else(|| panic!("{yaml:?} was not located"));
                assert_eq!(
                    located_shape(&yaml, &node),
                    format!("{{a:'b={expected}|q={expected}}}"),
                    "{yaml:?}"
                );
                let SchemaValueKind::Mapping(entries) = &node.kind else {
                    unreachable!()
                };
                assert_eq!(entry_count(&entries[1].value), entry_count_of(&parsed), "{yaml:?}");
            }
            let end = flow_collection_end(flow, 0);
            assert_eq!(end, Some(flow.len() - 1), "{flow:?}");
        }
    }

    /// Parentheses are plain-scalar content in raw YAML: they neither hide a
    /// `,` between entries nor, when they balance across two entries, merge
    /// those entries.
    #[test]
    fn parentheses_are_content_in_raw_flow_yaml() {
        for (flow, expected) in [
            // Controls: no parenthesis, and parentheses inside quotes.
            ("[ordinary, \"v\"]", "[ordinary|\"v\"]"),
            ("['a(b', \"v\"]", "['a(b'|\"v\"]"),
            ("[\"a(b, c)d\", 'v']", "[\"a(b, c)d\"|'v']"),
            ("[a(b)c, \"v\"]", "[a(b)c|\"v\"]"),
            // Unbalanced and cross-entry balanced parentheses.
            ("[a(b, \"v\"]", "[a(b|\"v\"]"),
            ("[a)b, \"v\"]", "[a)b|\"v\"]"),
            ("[a(b, c)d, \"v\"]", "[a(b|c)d|\"v\"]"),
            ("{a: a(b, b: \"v\"}", "{a=a(b|b=\"v\"}"),
            ("{a(b: x, c)d: \"v\"}", "{a(b=x|c)d=\"v\"}"),
            // Nested collections take the same rule.
            ("{list: [a(b, \"v\"]}", "{list=[a(b|\"v\"]}"),
            ("[{k: a(b}, {k: c)d}, 'v']", "[{k=a(b}|{k=c)d}|'v']"),
            ("[[a(b, c)d], \"v\"]", "[[a(b|c)d]|\"v\"]"),
        ] {
            let parsed: YamlValue = serde_yaml_ng::from_str(flow)
                .unwrap_or_else(|error| panic!("{flow:?} is not YAML: {error}"));
            for newline in ["\n", "\r\n"] {
                let yaml = format!("p(x: {flow}{newline}q: {flow}{newline}");
                let node = locate_schema_value(&yaml, 0)
                    .unwrap_or_else(|| panic!("{yaml:?} was not located"));
                assert_eq!(
                    located_shape(&yaml, &node),
                    format!("{{p(x={expected}|q={expected}}}"),
                    "{yaml:?}"
                );
                let SchemaValueKind::Mapping(entries) = &node.kind else {
                    unreachable!()
                };
                assert_eq!(entry_count(&entries[1].value), entry_count_of(&parsed), "{yaml:?}");
            }
            assert_eq!(flow_collection_end(flow, 0), Some(flow.len() - 1), "{flow:?}");
        }
    }

    fn entry_count(node: &SchemaValueNode) -> usize {
        match &node.kind {
            SchemaValueKind::Scalar => 0,
            SchemaValueKind::Sequence(items) => items.len(),
            SchemaValueKind::Mapping(entries) => entries.len(),
        }
    }

    fn entry_count_of(value: &YamlValue) -> usize {
        match value {
            YamlValue::Sequence(items) => items.len(),
            YamlValue::Mapping(entries) => entries.len(),
            _ => 0,
        }
    }

    fn suggestion_candidates(yaml: &str, property: &str) -> Vec<String> {
        let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
        let schema = parse_yaml_schema_with_source(&value, yaml, 0)
            .unwrap_or_else(|error| panic!("{yaml:?}: {error}"));
        let SimplifiedSchema::Single(shape) = schema else {
            panic!("expected single shape");
        };
        let mut atom = match &shape.properties[property] {
            PropertyDef::Single(atom) => atom,
            PropertyDef::Union(_) => panic!("expected single atom"),
        };
        while let TypeExpr::InlineObject(nested) = &atom.ty {
            let PropertyDef::Single(inner) = nested.properties.values().next().unwrap() else {
                panic!("expected single nested atom");
            };
            atom = inner;
        }
        atom.constraints
            .iter()
            .find_map(|constraint| match constraint {
                Constraint::Suggest(candidates) => Some(candidates),
                _ => None,
            })
            .unwrap()
            .iter()
            .map(|candidate| yaml[candidate.span.clone()].to_string())
            .collect()
    }

    #[test]
    fn a_schema_key_holding_a_quote_inside_a_plain_scalar_projects_its_suggestions() {
        for yaml in ["it's: string(suggest(alpha, beta))\n", "say \"hi: string(suggest(alpha, beta))\n"] {
            let key = yaml.split(": ").next().unwrap();
            assert_eq!(suggestion_candidates(yaml, key), ["alpha", "beta"], "{yaml:?}");
        }
        let quoted = "'it''s': string(suggest(alpha, beta))\n";
        assert_eq!(suggestion_candidates(quoted, "it's"), ["alpha", "beta"]);
    }

    #[test]
    fn a_schema_key_holding_a_content_colon_projects_its_suggestions() {
        for (key, newline) in [
            ("a:'b", "\n"),
            ("a:\"b", "\n"),
            ("a:b", "\n"),
            ("a:'b", "\r\n"),
            ("a:\"b", "\r\n"),
        ] {
            let yaml = format!("{key}: string(suggest(alpha, beta)){newline}other: string{newline}");
            assert_eq!(suggestion_candidates(&yaml, key), ["alpha", "beta"], "{yaml:?}");
        }
    }

    /// The decoded expression grammar is not YAML: its quotes open anywhere,
    /// so a quoted argument after `(` keeps its `,` and `}`.
    #[test]
    fn expression_quotes_open_anywhere_in_the_decoded_grammar() {
        assert_eq!(
            suggestion_candidates("value: string(suggest('a,b', c))\n", "value"),
            ["'a,b'", "c"]
        );
        assert_eq!(
            suggestion_candidates("value: \"{ mode: string(suggest('x}y', z)) }\"\n", "value"),
            ["'x}y'", "z"]
        );
    }

    /// The decoded helpers follow the expression lexer's quoting: `\` escapes
    /// in either quote style, a quote opens a string only inside an argument
    /// list, and braces inside an argument list are argument text.
    #[test]
    fn expression_projection_follows_the_lexer_quote_rule() {
        let root = SchemaSourcePath::root();
        let rows: &[(&str, &str, &[&str], &[&str])] = &[
            // Controls.
            ("string(suggest('a,b', c); min(1))", "", &["suggest('a,b', c)", "min(1)"], &["'a,b'", "c", "1"]),
            ("string(suggest(\"a\\\")b\", c))", "", &["suggest(\"a\\\")b\", c)"], &["\"a\\\")b\"", "c"]),
            // A backslash escapes inside single quotes too.
            ("string(suggest('a\\')b', c); min(1))", "", &["suggest('a\\')b', c)", "min(1)"], &["'a\\')b'", "c", "1"]),
            // A quote outside parentheses is description prose.
            ("\"{ a: string -> it's, b: string(suggest(x, y)) }\"", "b", &["suggest(x, y)"], &["x", "y"]),
            ("\"{ a: string -> say \\\"hi, b: string(suggest(x, y)) }\"", "b", &["suggest(x, y)"], &["x", "y"]),
            ("'{ a: string(suggest(''p,q'', r)) -> it''s here, b: number(min(2)) }'", "b", &["min(2)"], &["2"]),
            // A brace inside an argument is argument text.
            ("\"{ a: string(pattern(^[}]$)), b: number(min(2)) }\"", "b", &["min(2)"], &["2"]),
        ];
        for (yaml, property, constraints, arguments) in rows {
            let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
            let projected = parse_property_definition_with_source("p", &value, yaml, 0)
                .unwrap_or_else(|error| panic!("{yaml:?}: {error:?}"));
            let path = if property.is_empty() { root.clone() } else { root.property(*property) };
            let texts = |kind| {
                projected
                    .source_map
                    .spans(&path, kind)
                    .iter()
                    .map(|span| &yaml[span.clone()])
                    .collect::<Vec<_>>()
            };
            assert_eq!(texts(SchemaSpanKind::Constraint), *constraints, "{yaml:?}");
            assert_eq!(texts(SchemaSpanKind::Argument), *arguments, "{yaml:?}");
        }
    }

    /// The suggestion projector reads semantic candidate spans and YAML
    /// decoding rather than expression structure, so description and filename
    /// punctuation beside a suggestion leaves its spans exact.
    #[test]
    fn suggestion_spans_ignore_description_and_file_reference_punctuation() {
        for yaml in [
            "value: \"{ a: string(suggest(p, q)) -> (it's fine), b: Name@./a(b.yaml }\"\n",
            "value: '{ a: string(suggest(p, q)) -> (say \"hi), b: string -> plain [x }'\n",
            "value: \"{ a: string(suggest(p, q)) -> ({x} it's), b: Name@./a{b.yaml -> d }\"\r\n",
        ] {
            assert_eq!(suggestion_candidates(yaml, "value"), ["p", "q"], "{yaml:?}");
        }
    }

    #[test]
    fn projects_plain_single_and_double_quoted_scalars() {
        for yaml in [
            "value: string(suggest(alpha, 'café'))\n",
            "value: 'string(suggest(alpha, ''café''))'\n",
            "value: \"string(suggest(alpha, 'caf\\u00e9'))\"\n",
        ] {
            let span = candidate_span(yaml, 0);
            assert!(yaml[span].contains("caf"), "{yaml:?}");
        }
    }

    #[test]
    fn projection_preserves_crlf_and_document_offset() {
        let yaml = "café: string\r\nvalue: string(suggest(alpha, beta))\r\n";
        let span = candidate_span(yaml, 100);
        assert_eq!(&yaml[span.start - 100..span.end - 100], "beta");
    }

    #[test]
    fn projects_nested_inline_object_candidates_through_containing_scalar() {
        let yaml = "value: \"{ mode: string(min(5); suggest(no, valid)) }\"\n";
        let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
        let schema = parse_yaml_schema_with_source(&value, yaml, 0).unwrap();
        let SimplifiedSchema::Single(shape) = schema else {
            panic!("expected single shape");
        };
        let PropertyDef::Single(value) = &shape.properties["value"] else {
            panic!("expected single value atom");
        };
        let TypeExpr::InlineObject(nested) = &value.ty else {
            panic!("expected inline object");
        };
        let PropertyDef::Single(mode) = &nested.properties["mode"] else {
            panic!("expected single mode atom");
        };
        let Constraint::Suggest(candidates) = mode
            .constraints
            .iter()
            .find(|constraint| matches!(constraint, Constraint::Suggest(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let expected = yaml.find("no").unwrap();
        assert_eq!(candidates[0].span, expected..expected + 2);
    }
}
