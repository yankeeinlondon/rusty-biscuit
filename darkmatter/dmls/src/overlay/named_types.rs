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
    PropertyAtom, PropertyDef, SchemaArm, SchemaDeclaration, SchemaShape, SchemaSourceMap,
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
}

impl AuthoredDeclaration {
    /// The declaration a standalone schema document authors.
    pub fn standalone(model: &StandaloneSchemaDocument) -> Self {
        Self {
            declaration: model.declaration.clone(),
            source_map: model.source_map.clone(),
        }
    }

    /// The frontmatter `$schema` value of `text`, or `None` when the value is
    /// absent or cannot be projected (for example a block scalar).
    ///
    /// The value is re-parsed in place: the text before the value on its first
    /// line is replaced with spaces, so every column and byte offset matches
    /// the document and continuation lines keep their indentation.
    pub fn frontmatter(text: &str, ast: &FrontmatterAst) -> Option<Self> {
        let mut span = ast.schema_entry()?.value_span.clone();
        // A flow collection's recorded value span stops before its closing
        // bracket.
        if let Some(end) = flow_collection_end(text, span.start) {
            span.end = end + 1;
        }
        let line_start = text[..span.start].rfind('\n').map_or(0, |index| index + 1);
        let mut source = " ".repeat(span.start - line_start);
        source.push_str(text.get(span)?);
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&source).ok()?;
        let parsed = parse_schema_declaration_with_source(&yaml, &source, line_start).ok()?;
        Some(Self {
            declaration: parsed.value,
            source_map: parsed.source_map,
        })
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
    /// inline arm holding the reference (the resolver expands each arm with
    /// its own properties as `this`).
    pub fn local_namespace(&self, path: &SchemaSourcePath) -> Option<(&SchemaShape, SchemaSourcePath)> {
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
}

impl ExternalNamespace {
    /// The named types the file exports: the properties of a single mapping
    /// payload. A root union, a whole-file reference, or a raw JSON Schema
    /// exports none, as the resolver refuses to import from them.
    pub fn exports(&self) -> Option<&SchemaShape> {
        match &self.document.as_ref()?.declaration {
            SchemaDeclaration::Schema(SimplifiedSchema::Single(shape)) => Some(shape),
            _ => None,
        }
    }

    /// The exported definition of `name` with the span of its key in
    /// [`Self::text`].
    pub fn definition(&self, name: &str) -> Option<(&PropertyDef, Range<usize>)> {
        let definition = self.exports()?.properties.get(name)?;
        let key = self
            .document
            .as_ref()?
            .source_map
            .spans(&SchemaSourcePath::root().property(name), SchemaSpanKind::MappingKey)
            .first()?
            .clone();
        Some((definition, key))
    }
}

impl NamedTypeCatalog {
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
                let text: Arc<str> = Arc::from(std::fs::read_to_string(&path).ok()?);
                let document = parse_standalone_schema_document(&text, &path).ok().flatten();
                Some(ExternalNamespace { target, path, text, document })
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
        TypeExpr::Primitive(_) => {}
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
}
