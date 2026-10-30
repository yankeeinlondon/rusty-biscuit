//! Named-type references in an authored schema declaration.
//!
//! A reference is written as a bare `Name` (local), `Name@this` (local,
//! explicit), or `Name@./types.yaml` (another schema file). The library's
//! source map already records every reference name as a
//! [`SchemaSpanKind::ImportName`] span, so the editor finds references through
//! that map and never scans the text with its own grammar.
//!
//! Explicit cross-file targets are read once, when the overlay is assembled,
//! into a [`NamedTypeCatalog`]. Completion, hover, and go-to-definition read
//! the catalog, so a request performs no file I/O, and the catalog's files are
//! dependency edges: editing an imported file refreshes the assistance built
//! from it.

use std::collections::BTreeSet;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::resolve::named_type_reference_path;
use darkmatter::markdown::schemas::{
    LocatedDefinitionFailure, PropertyAtom, PropertyDef, SchemaArm, SchemaDeclaration, SchemaError,
    SchemaShape, SchemaSourceMap, SourceAware,
    SchemaSourcePath,
    SchemaSourcePathSegment, SchemaSpanKind, SimplifiedSchema, StandaloneSchemaDocument, TypeExpr, flow_collection_end,
    parse_property_definition, parse_schema_declaration_with_source, parse_standalone_schema_document,
};

use super::FrontmatterAst;

/// One parsed schema declaration with spans in its document's byte offsets.
#[derive(Debug, Clone)]
pub struct AuthoredDeclaration {
    /// The parsed declaration.
    pub declaration: SchemaDeclaration,
    /// Authored spans, in offsets of the document that holds the declaration.
    pub source_map: SchemaSourceMap,
    local_types: Option<SchemaShape>,
}

/// The authored bytes of the frontmatter `$schema` value.
fn frontmatter_value_span(text: &str, ast: &FrontmatterAst) -> Option<std::ops::Range<usize>> {
    let mut span = ast.schema_entry()?.value_span.clone();
    // A flow collection's recorded value span stops before its closing
    // bracket.
    if let Some(end) = flow_collection_end(text, span.start) {
        span.end = end + 1;
    }
    Some(span)
}

/// The frontmatter `$schema` value of `text` through the source-aware
/// declaration parser, re-parsed in place: the text before the value on its
/// first line is replaced with spaces, so every column and byte offset matches
/// the document and continuation lines keep their indentation.
fn parse_frontmatter_declaration(
    text: &str,
    ast: &FrontmatterAst,
) -> Option<Result<SourceAware<SchemaDeclaration>, LocatedDefinitionFailure>> {
    let span = frontmatter_value_span(text, ast)?;
    let line_start = text[..span.start].rfind('\n').map_or(0, |index| index + 1);
    let mut source = " ".repeat(span.start - line_start);
    source.push_str(text.get(span)?);
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&source).ok()?;
    Some(parse_schema_declaration_with_source(&yaml, &source, line_start))
}

impl AuthoredDeclaration {
    /// The declaration a standalone schema document authors.
    pub fn standalone(model: &StandaloneSchemaDocument) -> Self {
        Self {
            declaration: model.declaration.clone(),
            source_map: model.resolution_source_map(),
            local_types: model.local_types.clone(),
        }
    }

    /// The frontmatter `$schema` value of `text`, or `None` when the value is
    /// absent, invalid, or cannot be projected (for example a block scalar).
    pub fn frontmatter(text: &str, ast: &FrontmatterAst) -> Option<Self> {
        let parsed = parse_frontmatter_declaration(text, ast)?.ok()?;
        Some(Self {
            declaration: parsed.value,
            source_map: parsed.source_map,
            local_types: None,
        })
    }

    /// Why the frontmatter `$schema` value of `text` is not a declaration,
    /// located in `text`'s bytes; `None` when it parses, is absent, or is not
    /// YAML on its own.
    pub fn frontmatter_failure(text: &str, ast: &FrontmatterAst) -> Option<LocatedDefinitionFailure> {
        parse_frontmatter_declaration(text, ast)?.err()
    }

    /// [`Self::frontmatter`] for a request at `offset`: `None`, without
    /// parsing, unless `offset` lies within the `$schema` value (end
    /// inclusive, like [`Self::reference_at`]), since no span of the
    /// declaration lies elsewhere.
    pub fn frontmatter_at(text: &str, ast: &FrontmatterAst, offset: usize) -> Option<Self> {
        let span = frontmatter_value_span(text, ast)?;
        if offset < span.start || offset > span.end {
            return None;
        }
        Self::frontmatter(text, ast)
    }

    /// The named-type reference whose name contains `offset` (end inclusive,
    /// so a cursor just after the name still selects it).
    pub fn reference_at(&self, text: &str, offset: usize) -> Option<NamedReference> {
        self.source_map
            .entries()
            .filter(|(_, kind, _)| *kind == SchemaSpanKind::ImportName)
            .find_map(|(path, _, spans)| {
                let span = spans.iter().find(|span| span.start <= offset && offset <= span.end)?;
                Some(NamedReference::at(text, &self.source_map, path, span.clone()))
            })
    }

    /// The local namespace a reference at `path` resolves in, with the path
    /// of that namespace's root: the top-level shape, or for a root union the
    /// inline arm holding the reference. An explicit local type table
    /// supplies the namespace for every reference in the document.
    pub fn local_namespace(&self, path: &SchemaSourcePath) -> Option<(&SchemaShape, SchemaSourcePath)> {
        if let Some(types) = &self.local_types {
            return Some((types, SchemaSourcePath::root().property("types")));
        }
        let SchemaDeclaration::Schema(schema) = &self.declaration else {
            return None;
        };
        match schema {
            SimplifiedSchema::Single(shape) => Some((shape, SchemaSourcePath::root())),
            SimplifiedSchema::Union(arms) => {
                let Some(SchemaSourcePathSegment::UnionArm(index)) = path.segments().first() else {
                    return None;
                };
                match arms.get(*index)? {
                    SchemaArm::Inline(shape) => Some((shape, SchemaSourcePath::root().union_arm(*index))),
                    SchemaArm::FileRef(_) => None,
                }
            }
        }
    }
}

/// One authored named-type reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedReference {
    /// The referenced type name.
    pub name: String,
    /// The authored target (`./types.yaml`), or `None` for a local reference,
    /// bare or `@this`.
    pub target: Option<String>,
    /// Span of the name in the document.
    pub name_span: Range<usize>,
    /// Structural path of the atom holding the reference.
    pub path: SchemaSourcePath,
}

impl NamedReference {
    fn at(text: &str, source_map: &SchemaSourceMap, path: &SchemaSourcePath, span: Range<usize>) -> Self {
        let target = source_map
            .spans(path, SchemaSpanKind::ImportReference)
            .first()
            .and_then(|reference| text.get(reference.clone()))
            .map(str::trim)
            .filter(|reference| *reference != "this")
            .map(str::to_string);
        Self {
            name: text.get(span.clone()).unwrap_or_default().to_string(),
            target,
            name_span: span,
            path: path.clone(),
        }
    }
}

/// The schema files a document's explicit named-type references import, read
/// when the overlay was assembled.
#[derive(Debug, Default)]
pub struct NamedTypeCatalog {
    externals: Vec<ExternalNamespace>,
}

/// One imported schema file, keyed by the target spelling that selected it.
#[derive(Debug)]
pub struct ExternalNamespace {
    /// The authored target (`./types.yaml`).
    pub target: String,
    /// The file the resolver selects for `target`, canonicalized.
    pub path: PathBuf,
    /// The file's text when it was read.
    pub text: Arc<str>,
    /// The parsed file, or `None` when it is not a standalone schema document.
    pub document: Option<StandaloneSchemaDocument>,
    /// Why the file is not usable as a schema document: its text is not YAML,
    /// or its recognized envelope is malformed.
    pub error: Option<Arc<SchemaError>>,
}

impl ExternalNamespace {
    /// The named types the file exports: the properties of a single mapping
    /// payload when there is no explicit `types` table. A root union or
    /// whole-file reference without `types`, and raw JSON Schema, export none.
    pub fn exports(&self) -> Option<&SchemaShape> {
        self.document.as_ref()?.named_types()
    }

    /// The exported definition of `name` with the span of its key in
    /// [`Self::text`].
    pub fn definition(&self, name: &str) -> Option<(&PropertyDef, Range<usize>)> {
        let definition = self.exports()?.properties.get(name)?;
        let key = self
            .document
            .as_ref()?
            .named_type_source_map()
            .spans(&SchemaSourcePath::root().property(name), SchemaSpanKind::MappingKey)
            .first()?
            .clone();
        Some((definition, key))
    }
}

impl NamedTypeCatalog {
    /// Collects imports from both the export and the reusable type table.
    pub fn standalone(
        model: &StandaloneSchemaDocument,
        base_dir: &Path,
        schema_roots: &[PathBuf],
        context: &FileResolutionContext,
    ) -> Self {
        let mut targets = BTreeSet::new();
        if let Some(schema) = model.schema() {
            collect_schema_targets(schema, &mut targets);
        }
        if let Some(types) = &model.local_types {
            collect_schema_targets(&SimplifiedSchema::Single(types.clone()), &mut targets);
        }
        Self::read(targets, base_dir, schema_roots, context)
    }

    /// Reads the file of every distinct explicit target `declaration`
    /// references, resolved as the named-type resolver resolves it from
    /// `base_dir`. A target that does not resolve or cannot be read is left
    /// out; resolution reports it as an error of its own.
    pub fn build(
        declaration: &SchemaDeclaration,
        base_dir: &Path,
        schema_roots: &[PathBuf],
        context: &FileResolutionContext,
    ) -> Self {
        let mut targets = BTreeSet::new();
        if let SchemaDeclaration::Schema(schema) = declaration {
            collect_schema_targets(schema, &mut targets);
        }
        Self::read(targets, base_dir, schema_roots, context)
    }

    /// Like [`Self::build`] for a `$schema` value that may be mid-edit: each
    /// property definition is parsed on its own, so one unfinished property
    /// does not hide the targets of the others.
    pub fn build_tolerant(
        value: &serde_yaml_ng::Value,
        base_dir: &Path,
        schema_roots: &[PathBuf],
        context: &FileResolutionContext,
    ) -> Self {
        let mut targets = BTreeSet::new();
        collect_yaml_targets(value, &mut targets);
        Self::read(targets, base_dir, schema_roots, context)
    }

    fn read(
        targets: BTreeSet<String>,
        base_dir: &Path,
        schema_roots: &[PathBuf],
        context: &FileResolutionContext,
    ) -> Self {
        let externals = targets
            .into_iter()
            .filter_map(|target| {
                let path = named_type_reference_path(&target, base_dir, schema_roots, context)
                    .ok()
                    .flatten()?;
                let text: Arc<str> = Arc::from(crate::work_counters::read_to_string(&path).ok()?);
                let (document, error) = match parse_standalone_schema_document(&text, &path) {
                    Ok(document) => (document, None),
                    Err(error) => (None, Some(Arc::new(error))),
                };
                Some(ExternalNamespace { target, path, text, document, error })
            })
            .collect();
        Self { externals }
    }

    /// The imported file for an authored target.
    pub fn get(&self, target: &str) -> Option<&ExternalNamespace> {
        self.externals.iter().find(|external| external.target == target.trim())
    }

    /// Every imported file, in target order.
    pub fn externals(&self) -> &[ExternalNamespace] {
        &self.externals
    }

    /// The files the catalog read: dependency edges for the overlay cache.
    pub fn dependencies(&self) -> impl Iterator<Item = &PathBuf> {
        self.externals.iter().map(|external| &external.path)
    }
}

fn collect_yaml_targets(value: &serde_yaml_ng::Value, targets: &mut BTreeSet<String>) {
    match value {
        serde_yaml_ng::Value::Mapping(mapping) => {
            for (key, definition) in mapping {
                if let Some(key) = key.as_str()
                    && let Ok(definition) = parse_property_definition(key, definition)
                {
                    collect_definition_targets(&definition, targets);
                }
            }
        }
        // Root-union arms.
        serde_yaml_ng::Value::Sequence(arms) => {
            for arm in arms.iter().filter(|arm| arm.is_mapping()) {
                collect_yaml_targets(arm, targets);
            }
        }
        _ => {}
    }
}

fn collect_schema_targets(schema: &SimplifiedSchema, targets: &mut BTreeSet<String>) {
    match schema {
        SimplifiedSchema::Single(shape) => collect_shape_targets(shape, targets),
        SimplifiedSchema::Union(arms) => {
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    collect_shape_targets(shape, targets);
                }
            }
        }
    }
}

fn collect_shape_targets(shape: &SchemaShape, targets: &mut BTreeSet<String>) {
    let definitions = shape
        .properties
        .values()
        .chain(shape.pattern_keys.iter().map(|pattern| &pattern.def));
    for definition in definitions {
        collect_definition_targets(definition, targets);
    }
}

fn collect_definition_targets(definition: &PropertyDef, targets: &mut BTreeSet<String>) {
    let atoms = match definition {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(atoms) => atoms.as_slice(),
    };
    for atom in atoms {
        collect_atom_targets(atom, targets);
    }
}

fn collect_atom_targets(atom: &PropertyAtom, targets: &mut BTreeSet<String>) {
    match &atom.ty {
        TypeExpr::Imported { .. } if atom.ty.is_local_reference() => {}
        TypeExpr::Imported { reference, .. } => {
            targets.insert(reference.trim().to_string());
        }
        TypeExpr::InlineObject(nested) => collect_shape_targets(nested, targets),
        TypeExpr::Union(arms) => {
            for arm in arms {
                collect_atom_targets(arm, targets);
            }
        }
        TypeExpr::Tuple(tuple) => {
            tuple.any_atom(|atom| {
                collect_atom_targets(atom, targets);
                false
            });
        }
        // A function signature is description-only; a document schema that
        // reaches one fails to load before its references matter.
        TypeExpr::Primitive(_) | TypeExpr::Function(_) | TypeExpr::Ref(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frontmatter(text: &str) -> AuthoredDeclaration {
        let ast = FrontmatterAst::parse(text).and_then(|parse| parse.ast).expect("frontmatter");
        AuthoredDeclaration::frontmatter(text, &ast).expect("projected declaration")
    }

    #[test]
    fn frontmatter_projection_keeps_document_offsets_on_every_line() {
        let text = "---\n$schema:\n  base: string\n  alias: base\n  other: 'base(min(2))'\n---\n";
        let declaration = frontmatter(text);
        let alias = text.find("alias: base").unwrap() + "alias: ".len();
        let reference = declaration.reference_at(text, alias + 1).expect("bare reference");
        assert_eq!(reference.name, "base");
        assert_eq!(reference.name_span, alias..alias + 4);
        assert_eq!(reference.target, None);
        let quoted = text.find("'base(").unwrap() + 1;
        let reference = declaration.reference_at(text, quoted).expect("quoted reference");
        assert_eq!(reference.name_span, quoted..quoted + 4);
        assert_eq!(reference.path, SchemaSourcePath::root().property("other"));
    }

    #[test]
    fn flow_projection_and_explicit_targets_are_recovered() {
        let text = "---\n$schema: { a: 'T@./types.yaml', b: 'U@this' }\n---\n";
        let declaration = frontmatter(text);
        let t = text.find("T@").unwrap();
        let reference = declaration.reference_at(text, t).expect("explicit reference");
        assert_eq!(reference.target.as_deref(), Some("./types.yaml"));
        let u = text.find("U@").unwrap();
        assert_eq!(declaration.reference_at(text, u).unwrap().target, None, "`@this` is local");
    }

    /// A request outside the `$schema` value never parses the declaration,
    /// and one at either end of it (the end inclusive, as a reference's is)
    /// gets the same declaration [`AuthoredDeclaration::frontmatter`] does.
    #[test]
    fn a_request_parses_the_declaration_only_inside_the_schema_value() {
        let text = "---\ntitle: x\n$schema:\n  base: string\n  alias: base\nafter: 1\n---\n";
        let ast = FrontmatterAst::parse(text).and_then(|parse| parse.ast).expect("frontmatter");
        let value = ast.schema_entry().expect("a `$schema` entry").value_span.clone();
        let at = |offset| AuthoredDeclaration::frontmatter_at(text, &ast, offset);
        assert!(at(text.find("title").unwrap()).is_none(), "a key before the declaration");
        assert!(at(text.find("$schema").unwrap()).is_none(), "the `$schema` key itself");
        assert!(at(text.find("after").unwrap() + 1).is_none(), "a key after the declaration");
        assert!(at(value.start).is_some());
        assert!(at(value.end).is_some(), "the end is inclusive");
        let alias_end = text.find("alias: base").unwrap() + "alias: base".len();
        let declaration = at(alias_end).expect("a cursor just after the last reference");
        assert_eq!(declaration.reference_at(text, alias_end).expect("the reference").name, "base");

        // A flow value's span runs through its closing bracket.
        let text = "---\n$schema: { a: 'T@./types.yaml' }\nafter: 1\n---\n";
        let ast = FrontmatterAst::parse(text).and_then(|parse| parse.ast).expect("frontmatter");
        let close = text.find('}').unwrap();
        assert!(AuthoredDeclaration::frontmatter_at(text, &ast, close + 1).is_some());
        assert!(AuthoredDeclaration::frontmatter_at(text, &ast, close + 2).is_none(), "the next line");
    }
}
