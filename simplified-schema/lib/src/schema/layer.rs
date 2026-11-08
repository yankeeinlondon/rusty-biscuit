//! One schema as a factory built it: the authored form, the expanded typed
//! form, the lowered JSON Schema, the file context it was bound to, and what
//! resolving it read.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileReferenceError, FileResolutionContext};
use serde_json::Value;
use serde_yaml_ng::Value as YamlValue;

use super::SimplifiedSchemaError;
use crate::embedded::ShippedSources;
use crate::resolve::buffers::{self, SourceBuffers};
use crate::resolve::{self, InlineOwner, ReferenceStack, ResolvedSchema};
use crate::simplified::standalone::visit_def;
use crate::simplified::{
    Constraint, DefinitionTable, LOCAL_REFERENCE, PropertyAtom, PropertyDef, SchemaArm, SchemaCategory, SchemaDeclaration,
    SchemaRoot, SchemaShape, Slot, StandaloneSchemaEnvelope, TypeExpr, parse_keyed_declaration, project_keyed_declaration_spans,
    parse_standalone_schema_document, to_json_schema_with_definitions,
};
use crate::{SchemaAdvisory, SchemaError};

/// How in-memory text names itself in errors.
pub(super) const CONTENT: &str = "content";

/// One schema layer.
#[derive(Debug)]
pub(super) struct Layer {
    pub(super) authored: Authored,
    /// The declaration with its named types expanded, for queries. `None`
    /// for raw JSON Schema and for a whole-file reference an in-memory schema
    /// cannot follow.
    pub(super) typed: Option<Typed>,
    /// The lowered JSON Schema, or why there is none.
    pub(super) lowered: Result<Arc<Value>, Arc<SchemaError>>,
    /// The file context of a file-backed schema; `None` for content.
    pub(super) binding: Option<Binding>,
    pub(super) provenance: SchemaProvenance,
    /// How to build this layer again under slot values; set only on a
    /// template layer.
    pub(super) rebuild: Option<Rebuild>,
}

/// A template layer's own construction, run again by
/// [`super::SimplifiedSchema::resolve_slots`] inside a slot-value scope,
/// with `buffers` (the open buffers of the first build and every schema text
/// it read) in force, so the rebuild reads exactly what the first build did.
#[derive(Clone)]
pub(super) struct Rebuild {
    pub(super) build: Arc<dyn Fn() -> Result<Layer, SimplifiedSchemaError> + Send + Sync>,
    pub(super) buffers: SourceBuffers,
}

impl std::fmt::Debug for Rebuild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rebuild").finish_non_exhaustive()
    }
}

/// A layer constructor: the text, the file it came from, and its binding.
type Build = fn(&str, Option<PathBuf>, Option<Binding>) -> Result<Layer, SimplifiedSchemaError>;

/// Runs `build` and, when the layer is a template, keeps how to run it
/// again: the text (copied only then), the layer's own path and binding, and
/// the schema texts the build read. A build inside a slot-value scope is a
/// rebuild and keeps nothing.
fn rebuildable(
    text: &str,
    path: Option<PathBuf>,
    binding: Option<Binding>,
    build: Build,
) -> Result<Layer, SimplifiedSchemaError> {
    if crate::slots::active_values().is_some() {
        return build(text, path, binding);
    }
    let mut buffers = buffers::current();
    let (layer, sources) = buffers::recording(|| build(text, path, binding));
    let mut layer = layer?;
    if !layer.pending_slots().is_empty() {
        for (path, text) in sources {
            buffers.insert(&path, text);
        }
        let text: Arc<str> = Arc::from(text);
        let path = layer.provenance.path.clone();
        let binding = layer.binding.clone();
        layer.rebuild = Some(Rebuild {
            build: Arc::new(move || build(&text, path.clone(), binding.clone())),
            buffers,
        });
    }
    Ok(layer)
}

/// The form the schema was written in.
#[derive(Debug)]
pub(super) enum Authored {
    Simplified(Box<AuthoredSchema>),
    JsonSchema,
}

/// A SimplifiedSchema document as authored, before named-type expansion.
#[derive(Debug)]
pub(super) struct AuthoredSchema {
    pub(super) declaration: SchemaDeclaration,
    /// A separate `types` table beside an exported `$schema`.
    pub(super) local_types: Option<SchemaShape>,
    pub(super) envelope: StandaloneSchemaEnvelope,
    pub(super) categories: Option<Vec<SchemaCategory>>,
}

/// The expanded declaration and the stored definitions its references
/// point into.
#[derive(Debug)]
pub(super) struct Typed {
    pub(super) root: SchemaRoot,
    pub(super) definitions: Arc<DefinitionTable>,
}

/// The directory and request context a file-backed schema resolves in.
#[derive(Debug, Clone)]
pub(super) struct Binding {
    pub(super) base_dir: PathBuf,
    pub(super) context: FileResolutionContext,
}

/// Where a schema came from and what resolving it read.
///
/// The dependency lists are the edges a cache keys on: a change to any of
/// these files can change the schema. `searched` also lists locations that
/// were probed and absent; a file appearing at one of them can change what
/// resolution selects.
#[derive(Debug, Clone, Default)]
pub struct SchemaProvenance {
    pub(super) path: Option<PathBuf>,
    pub(super) imports: Vec<PathBuf>,
    pub(super) examples: Vec<PathBuf>,
    pub(super) referenced_files: Vec<PathBuf>,
    pub(super) searched: Vec<PathBuf>,
    pub(super) advisories: Vec<SchemaAdvisory>,
}

#[allow(missing_docs)]
impl SchemaProvenance {
    /// The file the schema was read from; `None` for content.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Files `Name@file` imports read, sorted.
    pub fn imports(&self) -> &[PathBuf] {
        &self.imports
    }

    /// Files `example(...)` constraints read, sorted.
    pub fn examples(&self) -> &[PathBuf] {
        &self.examples
    }

    /// Schema files a whole-file reference or root-union arm named, sorted.
    pub fn referenced_files(&self) -> &[PathBuf] {
        &self.referenced_files
    }

    /// Every location probed while resolving, found or not, sorted.
    pub fn searched(&self) -> &[PathBuf] {
        &self.searched
    }

    pub fn advisories(&self) -> &[SchemaAdvisory] {
        &self.advisories
    }
}

/// The context of a schema with no file: no anchors, no home, no
/// environment, and a relative directory, so it fails validation before any
/// candidate is probed. Building it reads no process state.
pub(crate) fn content_context() -> FileResolutionContext {
    FileResolutionContext::from_snapshot(PathBuf::new(), None, HashMap::new())
}

/// Where a declaration was authored, which decides its named types'
/// namespaces.
#[derive(Clone)]
enum Site<'a> {
    /// A Markdown document's frontmatter `$schema`, as `yaml`, located in
    /// the document when its text proves where it is.
    Document { yaml: &'a YamlValue, located: Option<crate::simplified::LocatedSchemaValue> },
    /// A standalone schema document.
    File,
}

impl Layer {
    /// Content: Markdown when it opens with a frontmatter block, otherwise a
    /// YAML (or JSON) document.
    pub(super) fn from_content(text: &str) -> Result<Self, SimplifiedSchemaError> {
        rebuildable(text, None, None, |text, _, _| match biscuit_file::frontmatter::extract_frontmatter_block(text)? {
            Some(frontmatter) => Self::from_frontmatter(frontmatter.yaml, frontmatter.yaml_span.start, None, None),
            None => Self::build_yaml_document(text, None, None),
        })
    }

    /// A Markdown document: its frontmatter `$schema`; the body is ignored.
    #[cfg(feature = "file")]
    pub(super) fn from_markdown(
        text: &str,
        path: Option<PathBuf>,
        binding: Option<Binding>,
    ) -> Result<Self, SimplifiedSchemaError> {
        rebuildable(text, path, binding, Self::build_markdown)
    }

    #[cfg(feature = "file")]
    fn build_markdown(text: &str, path: Option<PathBuf>, binding: Option<Binding>) -> Result<Self, SimplifiedSchemaError> {
        match biscuit_file::frontmatter::extract_frontmatter_block(text)? {
            Some(frontmatter) => Self::from_frontmatter(frontmatter.yaml, frontmatter.yaml_span.start, path, binding),
            None => Err(missing_schema(path.as_deref(), "the document has no frontmatter block")),
        }
    }

    /// A frontmatter block's YAML, which must hold `$schema`. `yaml_offset`
    /// is where `yaml_text` starts in the Markdown document, so a rejected
    /// definition is located in the document as supplied.
    fn from_frontmatter(
        yaml_text: &str,
        yaml_offset: usize,
        path: Option<PathBuf>,
        binding: Option<Binding>,
    ) -> Result<Self, SimplifiedSchemaError> {
        let frontmatter = crate::numeric::load_admitted_yaml(yaml_text).map_err(|error| SchemaError::SchemaDocument {
            path: path.clone().unwrap_or_else(|| PathBuf::from(CONTENT)),
            // The parser's line and column count from the frontmatter's
            // first line; `span` is the same position in the document.
            span: error.location().map(|at| yaml_offset + at.index()..yaml_offset + at.index()),
            message: format!("frontmatter is not valid YAML: {error}"),
            source: None,
        })?;
        let Some(value) = frontmatter.as_mapping().and_then(|mapping| mapping.get("$schema")) else {
            return Err(missing_schema(path.as_deref(), "the frontmatter has no `$schema`"));
        };
        let mut declaration = parse_keyed_declaration(value, yaml_text, "$schema", yaml_offset).map_err(|failure| {
            failure.into_document_error(path.as_deref().unwrap_or(Path::new(CONTENT)), "invalid frontmatter `$schema`")
        })?;
        // Candidate and slot spans point into the Markdown document, as a
        // standalone document's point into its file.
        project_keyed_declaration_spans(&mut declaration, yaml_text, "$schema", yaml_offset)?;
        let authored = AuthoredSchema {
            declaration,
            local_types: None,
            envelope: StandaloneSchemaEnvelope::Pure,
            categories: None,
        };
        // A root union's inline arms are parsed again when one arm is a
        // file reference; each projects through the same located value.
        let located = crate::simplified::LocatedSchemaValue::locate(yaml_text, "$schema", yaml_offset, value);
        resolve_authored(authored, Site::Document { yaml: value, located }, path, binding)
    }

    /// A standalone schema document, or a raw JSON Schema written in YAML or
    /// JSON (a mapping whose `$schema` is a JSON Schema dialect URI).
    #[cfg(feature = "file")]
    pub(super) fn from_yaml_document(
        text: &str,
        path: Option<PathBuf>,
        binding: Option<Binding>,
    ) -> Result<Self, SimplifiedSchemaError> {
        rebuildable(text, path, binding, Self::build_yaml_document)
    }

    fn build_yaml_document(
        text: &str,
        path: Option<PathBuf>,
        binding: Option<Binding>,
    ) -> Result<Self, SimplifiedSchemaError> {
        let named = path.clone().unwrap_or_else(|| PathBuf::from(CONTENT));
        if let Some(document) = parse_standalone_schema_document(text, &named)? {
            let authored = AuthoredSchema {
                declaration: document.declaration,
                local_types: document.local_types,
                envelope: document.envelope,
                categories: document.categories,
            };
            return resolve_authored(authored, Site::File, path, binding);
        }
        // The standalone parse accepted the text as YAML, so this load
        // cannot fail on syntax; it can only classify.
        let yaml = crate::numeric::load_admitted_yaml(text).map_err(|error| SchemaError::SchemaDocument {
            path: named.clone(),
            message: format!("not valid YAML: {error}"),
            source: None,
            span: None,
        })?;
        let dialect = yaml
            .as_mapping()
            .and_then(|mapping| mapping.get("$schema"))
            .and_then(YamlValue::as_str)
            .is_some_and(is_json_schema_dialect);
        if !dialect {
            let reason = match &yaml {
                YamlValue::Mapping(_) => {
                    "no root `$schema` declaration, no `kind: schema` with `types`, and no JSON Schema dialect"
                        .to_string()
                }
                other => format!("a YAML {} is not a schema", resolve::describe_yaml(other)),
            };
            return Err(missing_schema(path.as_deref(), &reason));
        }
        let json = serde_json::to_value(yaml).map_err(|_| SchemaError::NotASchemaDocument {
            path: named,
            reason: crate::errors::NotSchemaReason::NotAMapping,
        })?;
        Ok(Self::json_schema(json, path, binding))
    }

    /// A raw JSON Schema file's bytes, through the resolver's JSON loader.
    #[cfg(feature = "file")]
    pub(super) fn from_json_schema_bytes(
        bytes: &[u8],
        path: PathBuf,
        binding: Option<Binding>,
    ) -> Result<Self, SimplifiedSchemaError> {
        let resolved = resolve::parse_raw_json_schema(&path, bytes)?;
        Ok(Self::json_schema(resolved.json_schema, Some(path), binding))
    }

    fn json_schema(json: Value, path: Option<PathBuf>, binding: Option<Binding>) -> Self {
        Self {
            authored: Authored::JsonSchema,
            typed: None,
            lowered: Ok(Arc::new(json)),
            binding,
            provenance: SchemaProvenance { path, ..SchemaProvenance::default() },
            rebuild: None,
        }
    }
}

/// `http(s)://json-schema.org/…`: a dialect identifier, never a file to open.
fn is_json_schema_dialect(uri: &str) -> bool {
    ["https://json-schema.org/", "http://json-schema.org/"]
        .iter()
        .any(|prefix| uri.starts_with(prefix))
}

fn missing_schema(path: Option<&Path>, reason: &str) -> SimplifiedSchemaError {
    SimplifiedSchemaError::MissingSchema {
        source_name: path.map_or_else(|| CONTENT.to_string(), |path| format!("`{}`", path.display())),
        reason: reason.to_string(),
    }
}

/// Expands and lowers `authored`.
///
/// Content (no `binding`) resolves only local named types, through a
/// context that reads nothing; a reference to another file is kept as
/// parsed and makes the schema unpreparable. A bound schema resolves through
/// its context exactly as the resolver resolves a document's `$schema` or a
/// referenced schema file.
///
/// A lowering failure (a function-typed property) is kept on the layer, so
/// the schema can still be inspected and exported; expansion failures are
/// malformed definitions and fail construction.
fn resolve_authored(
    authored: AuthoredSchema,
    site: Site<'_>,
    path: Option<PathBuf>,
    binding: Option<Binding>,
) -> Result<Layer, SimplifiedSchemaError> {
    let mut provenance = SchemaProvenance { path: path.clone(), ..SchemaProvenance::default() };
    let shipped = ShippedSources::NONE;
    let defining_file = match site {
        Site::File => path.as_deref(),
        Site::Document { .. } => None,
    };

    let Some(binding) = binding else {
        if let Some(reference) = external_reference(&authored) {
            let unavailable = SchemaError::Unresolved {
                reference: reference.clone(),
                source: FileReferenceError::ResolutionUnavailable { reference },
            };
            let typed = authored.schema().map(|root| Typed {
                root: root.clone(),
                definitions: Arc::default(),
            });
            return Ok(Layer {
                authored: Authored::Simplified(Box::new(authored)),
                typed,
                lowered: Err(Arc::new(unavailable)),
                binding: None,
                provenance,
                rebuild: None,
            });
        }
        let root = authored.schema().expect("a declaration with no external reference is inline");
        let context = content_context();
        let mut expansion = resolve::expand_named_types_searched(
            root,
            authored.local_types.as_ref(),
            Path::new(""),
            None,
            &[],
            &context,
            shipped,
            &mut BTreeSet::new(),
        )?;
        let lowered = lower_expanded(&mut expansion.schema, &mut expansion.definitions);
        return Ok(Layer {
            authored: Authored::Simplified(Box::new(authored)),
            typed: Some(Typed { root: expansion.schema, definitions: expansion.definitions }),
            lowered,
            binding: None,
            provenance,
            rebuild: None,
        });
    };

    let base_dir = &binding.base_dir;
    let context = &binding.context;
    let whole = match (&authored.declaration, site) {
        (SchemaDeclaration::Reference(reference), _) => {
            let mut stack = ReferenceStack::default();
            Some(resolve::resolve_reference_in_context(
                reference.file_reference().raw(),
                base_dir,
                &[],
                context,
                &mut stack,
                shipped,
            ))
        }
        (SchemaDeclaration::Schema(SchemaRoot::Union(arms)), site)
            if arms.iter().any(|arm| matches!(arm, SchemaArm::FileRef(_))) =>
        {
            let mut stack = ReferenceStack::default();
            Some(match site {
                Site::Document { yaml, located } => resolve::resolve_root_union(
                    yaml.as_sequence().expect("a parsed root union is a sequence"),
                    InlineOwner::Document,
                    located.as_ref(),
                    base_dir,
                    &[],
                    context,
                    &mut stack,
                    shipped,
                ),
                Site::File => resolve::resolve_standalone_root_union(
                    arms.clone(),
                    authored.local_types.as_ref(),
                    base_dir,
                    path.as_deref().expect("a standalone schema file has a path"),
                    &[],
                    context,
                    &mut stack,
                    shipped,
                ),
            })
        }
        _ => None,
    };
    // A whole-file reference or root-union arm that reaches a template is
    // still a valid schema: it is kept, unlowered, for explicit resolution.
    let whole = match whole {
        Some(Err(unresolved @ SchemaError::UnresolvedSlots(_))) => {
            let typed = authored.schema().map(|root| Typed { root: root.clone(), definitions: Arc::default() });
            return Ok(Layer {
                authored: Authored::Simplified(Box::new(authored)),
                typed,
                lowered: Err(Arc::new(unresolved)),
                binding: Some(binding),
                provenance,
                rebuild: None,
            });
        }
        whole => whole.transpose()?,
    };
    if let Some(resolved) = whole {
        let ResolvedSchema {
            simplified,
            definitions,
            json_schema,
            imports,
            examples,
            referenced_files,
            searched,
            advisories,
            ..
        } = resolved;
        provenance.imports = imports;
        provenance.examples = examples;
        provenance.referenced_files = referenced_files;
        provenance.searched = searched;
        provenance.advisories = advisories;
        return Ok(Layer {
            authored: Authored::Simplified(Box::new(authored)),
            typed: simplified.map(|root| Typed { root, definitions }),
            lowered: Ok(Arc::new(json_schema)),
            binding: Some(binding),
            provenance,
            rebuild: None,
        });
    }

    let root = authored.schema().expect("an inline declaration");
    let mut searched = BTreeSet::new();
    let mut expansion = resolve::expand_named_types_searched(
        root,
        authored.local_types.as_ref(),
        base_dir,
        defining_file,
        &[],
        context,
        shipped,
        &mut searched,
    )?;
    let lowered = match lower_expanded(&mut expansion.schema, &mut expansion.definitions) {
        Ok(json) => {
            let mut json = Arc::unwrap_or_clone(json);
            provenance.examples =
                resolve::resolve_document_examples(&mut json, base_dir, defining_file, &[], context, &mut searched)?;
            Ok(Arc::new(json))
        }
        Err(error) => Err(error),
    };
    provenance.imports = expansion.imports;
    provenance.searched = searched.into_iter().collect();
    Ok(Layer {
        authored: Authored::Simplified(Box::new(authored)),
        typed: Some(Typed { root: expansion.schema, definitions: expansion.definitions }),
        lowered,
        binding: Some(binding),
        provenance,
        rebuild: None,
    })
}

/// Lowers an expanded declaration, substituting slot values first when a
/// slot-value scope is running (a rebuild). A lowering failure is kept, not
/// raised: a template's is [`SchemaError::UnresolvedSlots`], a refused slot
/// value's [`SchemaError::SlotResult`].
fn lower_expanded(root: &mut SchemaRoot, definitions: &mut Arc<DefinitionTable>) -> Result<Arc<Value>, Arc<SchemaError>> {
    if crate::slots::has_dynamic(root, definitions) {
        crate::slots::apply_active(root, Arc::make_mut(definitions)).map_err(Arc::new)?;
    }
    to_json_schema_with_definitions(root, definitions).map(Arc::new).map_err(Arc::new)
}

impl AuthoredSchema {
    /// The inline declaration; `None` for a whole-file reference.
    pub(super) fn schema(&self) -> Option<&SchemaRoot> {
        match &self.declaration {
            SchemaDeclaration::Schema(root) => Some(root),
            SchemaDeclaration::Reference(_) => None,
        }
    }
}

/// The first reference `authored` makes to another file, as written: a
/// whole-file reference, a root-union file arm, a `Name@file` import, or an
/// `example(./file)`.
fn external_reference(authored: &AuthoredSchema) -> Option<String> {
    let root = match &authored.declaration {
        SchemaDeclaration::Reference(reference) => return Some(reference.file_reference().raw().to_string()),
        SchemaDeclaration::Schema(root) => root,
    };
    let mut shapes: Vec<&SchemaShape> = authored.local_types.iter().collect();
    match root {
        SchemaRoot::Single(shape) => shapes.push(shape),
        SchemaRoot::Union(arms) => {
            for arm in arms {
                match arm {
                    SchemaArm::Inline(shape) => shapes.push(shape),
                    SchemaArm::FileRef(reference) => return Some(reference.clone()),
                }
            }
        }
    }
    let mut found = None;
    for def in shapes
        .into_iter()
        .flat_map(|shape| shape.properties.values().chain(shape.pattern_keys.iter().map(|pattern| &pattern.def)))
    {
        visit_def(def, &mut |atom| {
            if found.is_some() {
                return;
            }
            if let TypeExpr::Imported { name, reference } = &atom.ty
                && reference != LOCAL_REFERENCE
            {
                found = Some(format!("{name}@{reference}"));
            }
            if let Some(Constraint::Example(references)) = atom
                .property_constraints()
                .find(|constraint| matches!(constraint, Constraint::Example(references) if !references.is_empty()))
            {
                found = Some(references[0].clone());
            }
        });
        if found.is_some() {
            break;
        }
    }
    found
}

/// Whether `typed` declares the top-level property `key`, in its own
/// properties or in any inline root-union arm's.
pub(super) fn declares(typed: &Typed, key: &str) -> bool {
    match &typed.root {
        SchemaRoot::Single(shape) => shape.properties.contains_key(key),
        SchemaRoot::Union(arms) => arms.iter().any(|arm| match arm {
            SchemaArm::Inline(shape) => shape.properties.contains_key(key),
            SchemaArm::FileRef(_) => false,
        }),
    }
}

/// Whether a lowered JSON Schema declares the top-level property `key`, in
/// its own properties or in any root-union arm's.
pub(super) fn json_declares(json: &Value, key: &str) -> bool {
    let declares = |object: &Value| object.get("properties").and_then(|properties| properties.get(key)).is_some();
    declares(json) || json.get("anyOf").and_then(Value::as_array).is_some_and(|arms| arms.iter().any(declares))
}

impl Layer {
    /// The unresolved `{{ … }}` slots of this layer, in declaration order.
    /// A layer whose resolution stopped at a template (a whole-file
    /// reference to one) has no typed form; its error lists them.
    pub(super) fn pending_slots(&self) -> Vec<Slot> {
        match (&self.typed, self.lowered.as_ref().err().map(|error| error.as_ref())) {
            (_, Some(SchemaError::UnresolvedSlots(unresolved))) => unresolved.slots.clone(),
            (Some(typed), _) => crate::slots::collect_slots(&typed.root, &typed.definitions),
            (None, _) => Vec::new(),
        }
    }

    /// The object named type `name` of this layer as a schema of its own:
    /// the expanded type for validation and queries, and, for export, the
    /// authored type with the whole authored table beside it, so the
    /// type's references to its siblings still resolve.
    pub(super) fn select(&self, name: &str) -> Result<Self, SimplifiedSchemaError> {
        let selection = |reason: &str| SimplifiedSchemaError::Selection { name: name.to_string(), reason: reason.to_string() };
        let (Authored::Simplified(authored), Some(typed)) = (&self.authored, &self.typed) else {
            return Err(selection("the schema has no SimplifiedSchema types"));
        };
        let table = match (&authored.local_types, authored.schema()) {
            (Some(types), _) => types,
            (None, Some(SchemaRoot::Single(shape))) => shape,
            _ => return Err(selection("the schema has no named types")),
        };
        let authored_def = table.properties.get(name).ok_or_else(|| selection("no such type"))?;
        // A separate `types` table is not expanded into the root; take the
        // root's expansion only when the table is the root itself.
        let expanded_def = match (&authored.local_types, &typed.root) {
            (None, SchemaRoot::Single(shape)) => shape.properties.get(name),
            _ => None,
        };
        let object = |def: &PropertyDef| match def {
            PropertyDef::Single(PropertyAtom { ty: TypeExpr::InlineObject(shape), is_array: false, .. }) => {
                Some(shape.clone())
            }
            _ => None,
        };
        let authored_shape = object(authored_def).ok_or_else(|| selection("not an object type"))?;
        let selected = AuthoredSchema {
            declaration: SchemaDeclaration::Schema(SchemaRoot::Single(authored_shape)),
            local_types: Some(table.clone()),
            envelope: StandaloneSchemaEnvelope::Pure,
            categories: None,
        };
        let (mut root, mut definitions) = match expanded_def.and_then(object) {
            Some(shape) => (SchemaRoot::Single(shape), Arc::clone(&typed.definitions)),
            // Expand the selection against the table it came from.
            None => {
                let context = self.binding.as_ref().map_or_else(content_context, |binding| binding.context.clone());
                let base_dir = self.binding.as_ref().map_or_else(PathBuf::new, |binding| binding.base_dir.clone());
                let root = selected.schema().expect("an inline selection");
                let expansion = resolve::expand_named_types_searched(
                    root,
                    selected.local_types.as_ref(),
                    &base_dir,
                    self.provenance.path.as_deref(),
                    &[],
                    &context,
                    ShippedSources::NONE,
                    &mut BTreeSet::new(),
                )?;
                (expansion.schema, expansion.definitions)
            }
        };
        let lowered = match &self.lowered {
            // An unpreparable source stays unpreparable for the same reason.
            Err(error) if self.binding.is_none() && !matches!(error.as_ref(), SchemaError::UnresolvedSlots(_)) => {
                Err(Arc::clone(error))
            }
            _ => lower_expanded(&mut root, &mut definitions),
        };
        let mut layer = Self {
            authored: Authored::Simplified(Box::new(selected)),
            typed: Some(Typed { root, definitions }),
            lowered,
            binding: self.binding.clone(),
            provenance: self.provenance.clone(),
            rebuild: None,
        };
        // A selection from a template rebuilds its source, then selects.
        if let Some(parent) = &self.rebuild
            && !layer.pending_slots().is_empty()
        {
            let (build, name) = (Arc::clone(&parent.build), name.to_string());
            layer.rebuild = Some(Rebuild {
                build: Arc::new(move || build()?.select(&name)),
                buffers: parent.buffers.clone(),
            });
        }
        Ok(layer)
    }
}
