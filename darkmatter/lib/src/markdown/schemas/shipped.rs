//! Darkmatter's shipped artifact: the prepared form of its compiled-in
//! schema sources.
//!
//! Darkmatter compiles in its global catalog with its partials and Content
//! Policy's editor schema ([`embedded::SHIPPED`]), the YAML document-kind
//! definitions (`schema-trigger.yaml` and the library's envelope
//! definitions), the side-effect catalog (`mutations.yaml`), and the
//! expression catalog's file-form examples. Preparing them (resolving the
//! catalog, lowering the baseline, loading the function catalogs) used to run
//! on first use in every process. The checked-in artifact,
//! `darkmatter/lib/generated/shipped.json`, holds the result; [`registry`]
//! loads it once per process through the shared mechanism
//! ([`simplified_schema::shipped`]) after proving it describes exactly the
//! compiled text, the current Simplified Schema artifact it was built over,
//! and this crate's [`PREPARATION_REVISION`].
//!
//! Loading checks the header and the payload's member list only. Each
//! payload member is a section, decoded and validated the first time a
//! consumer reads it ([`simplified_schema::shipped::Section`]), so a process
//! pays only for what it reads. A refused artifact is never used; a section
//! refused on first read is not used either. Each consumer then prepares its
//! own item from the compiled text on first use, as it always did:
//!
//! | View | Consumer | Fallback |
//! | --- | --- | --- |
//! | [`Registry::global_catalog`] | [`GlobalCatalog::darkmatter`] | the catalog resolved from [`embedded::SHIPPED`] |
//! | baseline | the runtime baseline in [`super`] | the catalog's `fm`, lowered |
//! | [`Registry::inline_baseline`] | [`super::darkmatter_base_schema`] | the `fm` declaration resolved inline |
//! | kind definitions | [`super::KindRegistry`] | each definition prepared from its text |
//! | [`Registry::function_catalog`] | `expression_function_catalog` | `functions.yaml` loaded |
//! | [`Registry::effect_catalog`] | `effect_function_catalog` | `mutations.yaml` loaded |
//! | [`NAMED_TYPES`] | `Name@darkmatter:<stem>` references | the partial parsed |
//!
//! Only an accepted baseline skips the baseline check
//! (`validate_baseline_schema`): generation ran it. Regenerate with
//! `just shipped-schemas` after editing a shipped source or the code that
//! prepares one; see `simplified-schema/docs/topics/shipped-artifacts.md`.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use simplified_schema::embedded::EmbeddedDocument;
use simplified_schema::shipped::{
    self as mechanism, Acceptance, AcceptedArtifact, ArtifactDescriptor, FreshLower, LowerArtifact, NamedTypeTableRecord,
    PreparationMode, PreparedNamedTypes, PreparedSourcesDescriptor, RecordError, Refusal, Section, Sections,
};
use simplified_schema::{DefinitionTable, SchemaRoot, SchemaSourceId};

use super::catalog::{DARKMATTER_CATALOG_PATH, DARKMATTER_GLOBALS, GlobalCatalog, GlobalCatalogRecord};
use super::embedded::{self, SHIPPED};
use super::yaml_documents::KindRegistry;
use super::yaml_documents::registry::ShippedDefinition;
use crate::markdown::compose::expression::FunctionCatalog;
use crate::markdown::compose::expression::catalog as functions;

/// The owner name in Darkmatter's artifact header.
pub const OWNER: &str = "darkmatter";

/// Darkmatter's preparation revision. Increment it whenever code changes
/// what the shipped sources prepare to (catalog resolution, baseline
/// lowering, kind-definition or function-catalog loading, a record's shape),
/// then run `just shipped-schemas`.
pub const PREPARATION_REVISION: u32 = 1;

/// The artifact's repository-relative path.
pub const ARTIFACT_PATH: &str = "darkmatter/lib/generated/shipped.json";

/// The command that regenerates every shipped artifact, from the repository
/// root.
pub const REGENERATE_COMMAND: &str = "just shipped-schemas";

/// The checked-in artifact compiled into this build.
pub const ARTIFACT: &[u8] = include_bytes!("../../../generated/shipped.json");

/// Where the reserved namespace's partials live; a shipped document under
/// this folder is served as `Name@darkmatter:<stem>`.
const PARTIALS: &str = "darkmatter/schemas/partials/";

/// Every compiled-in text Darkmatter's preparation reads, by virtual path:
/// [`embedded::SHIPPED`], the kind definitions (the library's envelope
/// definitions and `schema-trigger.yaml`), `mutations.yaml`, and the
/// expression catalog's file-form examples under
/// `darkmatter/schemas/partials/function-examples/`.
pub fn sources() -> &'static [EmbeddedDocument<'static>] {
    static EXAMPLE_PATHS: OnceLock<Vec<String>> = OnceLock::new();
    static SOURCES: OnceLock<Vec<EmbeddedDocument<'static>>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        let example_paths =
            EXAMPLE_PATHS.get_or_init(|| functions::EXAMPLE_FILES.iter().map(|(key, _)| format!("{PARTIALS}{key}")).collect());
        let mut documents = SHIPPED.to_vec();
        documents.extend_from_slice(embedded::shipped_kind_definitions());
        documents.push(EmbeddedDocument {
            path: crate::effects::types::MUTATIONS_PATH,
            text: crate::effects::types::MUTATIONS_SOURCE,
        });
        documents.extend(
            example_paths.iter().zip(functions::EXAMPLE_FILES).map(|(path, (_, text))| EmbeddedDocument { path, text }),
        );
        documents
    })
}

/// What Darkmatter's artifact must describe over `sources` and the
/// Simplified Schema artifact state `lower`.
pub fn descriptor_over<'a>(sources: &'a [EmbeddedDocument<'a>], lower: &'a [LowerArtifact]) -> ArtifactDescriptor<'a> {
    ArtifactDescriptor {
        owner: OWNER,
        preparation_revision: PREPARATION_REVISION,
        sources,
        dependencies: lower,
        configuration: &[],
    }
}

/// The artifact payload. Members keep authored order (catalog globals,
/// definitions, categories, overloads, named types); nothing is sorted.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarkmatterPayload {
    /// The named types the reserved `darkmatter:` namespace serves.
    pub named_types: NamedTypesRecord,
    /// The global catalog, `darkmatter/schemas/darkmatter.yaml`, resolved.
    pub catalog: GlobalCatalogRecord,
    /// The runtime baseline (the catalog's `fm`) lowered to JSON Schema; its
    /// typed form is the catalog record's `fm`.
    pub baseline: Value,
    /// The baseline with every acyclic type expanded at each use.
    pub inline_baseline: SchemaRoot,
    /// Each shipped YAML document-kind definition, in source-table order.
    pub kind_definitions: Vec<KindDefinitionRecord>,
    /// The expression-function catalog (`functions.yaml`).
    pub functions: FunctionCatalog,
    /// The side-effect catalog (`mutations.yaml`).
    pub effects: FunctionCatalog,
}

/// The named-type tables of [`embedded::SHIPPED`]'s partials, and the
/// documents (every [`embedded::SHIPPED`] path, in table order) they were
/// prepared from.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedTypesRecord {
    pub inventory: Vec<String>,
    pub tables: Vec<NamedTypeTableRecord>,
}

/// One shipped `kind: yaml-schema` definition, prepared: the exported
/// `$schema` resolved (the parts of a `ResolvedSchema` an embedded
/// resolution fills) and the kinds it claims.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindDefinitionRecord {
    /// The definition's repository-relative path.
    pub identity: String,
    #[serde(deserialize_with = "simplified_schema::shipped::present")]
    pub simplified: Option<SchemaRoot>,
    pub definitions: DefinitionTable,
    pub json_schema: Value,
    pub origin: OriginRecord,
    /// The kinds it claims, in first-declared order.
    pub kinds: Vec<String>,
}

/// A `SchemaOrigin` on the wire; a host file is not representable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginRecord {
    pub kind: OriginKindRecord,
    #[serde(deserialize_with = "simplified_schema::shipped::present")]
    pub source: Option<SchemaSourceId>,
}

/// A `SchemaOriginKind` on the wire.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OriginKindRecord {
    Document,
    ReferencedFile,
    Baseline,
    Trigger,
}

impl OriginRecord {
    fn of(origin: &simplified_schema::SchemaOrigin) -> Self {
        use simplified_schema::SchemaOriginKind as Kind;
        let kind = match origin.kind {
            Kind::Document => OriginKindRecord::Document,
            Kind::ReferencedFile => OriginKindRecord::ReferencedFile,
            Kind::Baseline => OriginKindRecord::Baseline,
            Kind::Trigger => OriginKindRecord::Trigger,
        };
        Self { kind, source: origin.source.clone() }
    }

    pub(crate) fn origin(&self) -> simplified_schema::SchemaOrigin {
        use simplified_schema::SchemaOriginKind as Kind;
        let kind = match self.kind {
            OriginKindRecord::Document => Kind::Document,
            OriginKindRecord::ReferencedFile => Kind::ReferencedFile,
            OriginKindRecord::Baseline => Kind::Baseline,
            OriginKindRecord::Trigger => Kind::Trigger,
        };
        simplified_schema::SchemaOrigin { kind, source: self.source.clone() }
    }
}

/// The runtime baseline: its typed form with every import expanded, the
/// shared definitions its references point into, and the pair lowered to
/// JSON Schema.
pub(crate) struct ShippedBaseline {
    pub(crate) schema: SchemaRoot,
    pub(crate) definitions: Arc<DefinitionTable>,
    pub(crate) json_schema: Arc<Value>,
}

/// An accepted artifact's entries, one [`Section`] per payload member. A
/// process decodes only what it reads: `md compose` on a plain document
/// reads the catalog and the baseline, never the function catalog.
pub struct Prepared {
    named_types: Section<PreparedNamedTypes>,
    catalog: Section<GlobalCatalog>,
    baseline: Section<ShippedBaseline>,
    inline_baseline: Section<SchemaRoot>,
    kind_definitions: Section<Vec<ShippedDefinition>>,
    functions: Section<FunctionCatalog>,
    effects: Section<FunctionCatalog>,
}

impl std::fmt::Debug for Prepared {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Prepared").finish_non_exhaustive()
    }
}

/// Darkmatter's loaded artifact, or why it was refused. Every view returns
/// the accepted entry, or the consumer's own preparation from text when the
/// artifact, or that entry's section, was refused.
pub struct Registry {
    artifact: Result<AcceptedArtifact<Prepared>, Refusal>,
    /// [`KindRegistry::shipped`], built on first use from this registry's
    /// kind definitions.
    kinds: OnceLock<Arc<KindRegistry>>,
}

impl Registry {
    /// Loads `bytes` as Darkmatter's artifact over the process's Simplified
    /// Schema artifact.
    pub fn load(bytes: Option<&[u8]>) -> Self {
        Self::load_over(bytes, simplified_schema::shipped::registry::registry().as_dependency())
    }

    /// Loads `bytes` as Darkmatter's artifact over the Simplified Schema
    /// artifact state `lower` (tests inject a refused or changed one).
    /// Nothing is decoded past the header and the payload's member list
    /// here, and nothing is prepared from text, even on a refusal: each view
    /// decodes its section, or prepares its own item, when first read.
    pub fn load_over(bytes: Option<&[u8]>, lower: LowerArtifact) -> Self {
        let lower = [lower];
        let artifact = mechanism::accept_deferred(&descriptor_over(sources(), &lower), bytes, accept_sections);
        if let Err(refusal) = &artifact {
            tracing::debug!(owner = OWNER, %refusal, "shipped artifact refused; preparing compiled sources on use");
        }
        Self { artifact, kinds: OnceLock::new() }
    }

    /// Why the artifact was refused, or `None` when its header and member
    /// list were accepted. A section can still be refused when first read
    /// ([`Self::check_all`]).
    pub fn refusal(&self) -> Option<&Refusal> {
        self.artifact.as_ref().err()
    }

    /// Reads every section and returns the first refusal: the artifact's,
    /// or the first section's in payload order. Generation commands and
    /// tests use it; a process reads only the sections it needs.
    ///
    /// ## Errors
    ///
    /// The first [`Refusal`].
    pub fn check_all(&self) -> Result<(), Refusal> {
        let prepared = match &self.artifact {
            Ok(accepted) => accepted.payload(),
            Err(refusal) => return Err(refusal.clone()),
        };
        let refusals = [
            prepared.named_types.get(accept_named_types).err(),
            prepared.catalog.get(accept_catalog).err(),
            self.accepted_baseline().err(),
            prepared.inline_baseline.get(accept_inline_baseline).err(),
            prepared.kind_definitions.get(accept_kind_definitions).err(),
            prepared.functions.get(accept_function_catalog).err(),
            prepared.effects.get(accept_function_catalog).err(),
        ];
        match refusals.into_iter().flatten().next() {
            Some(refusal) => Err(refusal.clone()),
            None => Ok(()),
        }
    }

    /// This artifact's state as a dependency of a higher artifact
    /// (Claudine's). A higher artifact records the digest of these bytes as
    /// fresh generation produced them, so a section that would be refused
    /// cannot match it.
    pub fn as_dependency(&self) -> LowerArtifact {
        match &self.artifact {
            Ok(accepted) => accepted.as_dependency(),
            Err(_) => LowerArtifact::Refused { owner: OWNER.to_string() },
        }
    }

    fn accepted(&self) -> Option<&Prepared> {
        self.artifact.as_ref().ok().map(AcceptedArtifact::payload)
    }

    fn accepted_catalog(&self) -> Option<&GlobalCatalog> {
        self.accepted()?.catalog.get(accept_catalog).ok()
    }

    /// The accepted baseline: its lowered JSON from its own section, its
    /// typed form from the accepted catalog's `fm`. Refused when either
    /// section is.
    fn accepted_baseline(&self) -> Result<&ShippedBaseline, &Refusal> {
        let Some(prepared) = self.accepted() else {
            return Err(self.refusal().expect("a refused artifact"));
        };
        let catalog = self.accepted_catalog();
        prepared.baseline.get(|_, baseline: Value| accept_baseline(baseline, catalog))
    }

    /// Darkmatter's global catalog.
    pub fn global_catalog(&self) -> &GlobalCatalog {
        match self.accepted_catalog() {
            Some(catalog) => catalog,
            None => GlobalCatalog::darkmatter_from_sources(),
        }
    }

    /// The runtime baseline.
    pub(crate) fn baseline(&self) -> &ShippedBaseline {
        match self.accepted_baseline() {
            Ok(baseline) => baseline,
            Err(_) => super::baseline_from_sources(),
        }
    }

    /// Whether the baseline came from the accepted artifact, whose
    /// generation already ran the baseline check.
    pub fn baseline_is_accepted(&self) -> bool {
        self.accepted_baseline().is_ok()
    }

    /// The runtime baseline lowered to JSON Schema.
    pub fn baseline_json_schema(&self) -> &Value {
        &self.baseline().json_schema
    }

    /// The baseline with every acyclic type expanded
    /// ([`super::darkmatter_base_schema`]).
    pub fn inline_baseline(&self) -> &SchemaRoot {
        match self.accepted().map(|prepared| prepared.inline_baseline.get(accept_inline_baseline)) {
            Some(Ok(schema)) => schema,
            _ => super::inline_baseline_from_sources(),
        }
    }

    /// The shipped YAML document-kind definitions, in source-table order.
    pub(crate) fn kind_definitions(&self) -> &[ShippedDefinition] {
        match self.accepted().map(|prepared| prepared.kind_definitions.get(accept_kind_definitions)) {
            Some(Ok(definitions)) => definitions,
            _ => super::yaml_documents::registry::shipped_definitions_from_sources(),
        }
    }

    /// The registry of the shipped kind definitions alone
    /// ([`KindRegistry::shipped`]), built once from [`Self::kind_definitions`].
    pub fn shipped_kind_registry(&self) -> Arc<KindRegistry> {
        Arc::clone(self.kinds.get_or_init(|| Arc::new(KindRegistry::shipped_from(self.kind_definitions()))))
    }

    /// The expression-function catalog.
    pub fn function_catalog(&self) -> &FunctionCatalog {
        match self.accepted().map(|prepared| prepared.functions.get(accept_function_catalog)) {
            Some(Ok(catalog)) => catalog,
            _ => functions::expression_function_catalog_from_sources(),
        }
    }

    /// The side-effect catalog.
    pub fn effect_catalog(&self) -> &FunctionCatalog {
        match self.accepted().map(|prepared| prepared.effects.get(accept_function_catalog)) {
            Some(Ok(catalog)) => catalog,
            _ => crate::effects::types::effect_function_catalog_from_sources(),
        }
    }

    /// The accepted named-type tables of the reserved namespace; `None` when
    /// the artifact or the section was refused, so the resolver parses the
    /// partials.
    pub fn named_types(&self) -> Option<&PreparedNamedTypes> {
        self.accepted()?.named_types.get(accept_named_types).ok()
    }
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Registry").field("refusal", &self.refusal()).finish_non_exhaustive()
    }
}

/// The process registry, loaded from [`ARTIFACT`] on first use.
pub fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| Registry::load(Some(ARTIFACT)))
}

/// The process registry's named-type tables, attached to
/// [`embedded::SHIPPED_SOURCES`]. Served only while the documents a
/// `ShippedSources` carries are exactly the ones they were prepared from.
pub static NAMED_TYPES: PreparedSourcesDescriptor =
    PreparedSourcesDescriptor { owner: OWNER, provider: || registry().named_types() };

/// Takes each payload member as its [`Section`] (the member set is
/// [`DarkmatterPayload`]'s). Nothing is decoded here.
fn accept_sections(_: &Acceptance<'_>, sections: &mut Sections) -> Result<Prepared, RecordError> {
    Ok(Prepared {
        named_types: sections.take("named_types")?,
        catalog: sections.take("catalog")?,
        baseline: sections.take("baseline")?,
        inline_baseline: sections.take("inline_baseline")?,
        kind_definitions: sections.take("kind_definitions")?,
        functions: sections.take("functions")?,
        effects: sections.take("effects")?,
    })
}

fn accept_named_types(acceptance: &Acceptance<'_>, named_types: NamedTypesRecord) -> Result<PreparedNamedTypes, RecordError> {
    let expected: Vec<&str> = SHIPPED.iter().map(|document| document.path).collect();
    if named_types.inventory != expected {
        return Err(RecordError::new("/inventory", "the inventory is not the shipped document table"));
    }
    PreparedNamedTypes::from_records(acceptance, "", &named_types.inventory, named_types.tables)
}

fn accept_catalog(_: &Acceptance<'_>, catalog: GlobalCatalogRecord) -> Result<GlobalCatalog, RecordError> {
    if catalog.identity != DARKMATTER_CATALOG_PATH {
        return Err(RecordError::new("/identity", format!("`{}` is not the global catalog", catalog.identity)));
    }
    GlobalCatalog::from_record(catalog, DARKMATTER_GLOBALS)
}

fn accept_baseline(baseline: Value, catalog: Option<&GlobalCatalog>) -> Result<ShippedBaseline, RecordError> {
    if !baseline.is_object() {
        return Err(RecordError::new("", "the baseline lowers to a JSON Schema object"));
    }
    let catalog = catalog.ok_or_else(|| RecordError::new("", "the catalog section it is typed by was refused"))?;
    let (schema, definitions) =
        catalog.baseline().ok_or_else(|| RecordError::new("", "the catalog declares no `fm` shape"))?;
    Ok(ShippedBaseline { schema, definitions, json_schema: Arc::new(baseline) })
}

fn accept_inline_baseline(_: &Acceptance<'_>, inline_baseline: SchemaRoot) -> Result<SchemaRoot, RecordError> {
    if !matches!(inline_baseline, SchemaRoot::Single(_)) {
        return Err(RecordError::new("", "the inline baseline is a single object shape"));
    }
    Ok(inline_baseline)
}

fn accept_kind_definitions(
    _: &Acceptance<'_>,
    kind_definitions: Vec<KindDefinitionRecord>,
) -> Result<Vec<ShippedDefinition>, RecordError> {
    let documents = embedded::shipped_kind_definitions();
    if kind_definitions.len() != documents.len() {
        return Err(RecordError::new(
            "",
            format!("{} definitions are recorded, {} are compiled in", kind_definitions.len(), documents.len()),
        ));
    }
    kind_definitions
        .into_iter()
        .zip(documents)
        .enumerate()
        .map(|(index, (record, document))| {
            let at = format!("/{index}");
            if record.identity != document.path {
                return Err(RecordError::new(
                    format!("{at}/identity"),
                    format!("`{}` is recorded where `{}` belongs", record.identity, document.path),
                ));
            }
            if !record.json_schema.is_object() {
                return Err(RecordError::new(format!("{at}/json_schema"), "a definition lowers to a JSON Schema object"));
            }
            if record.kinds.is_empty() {
                return Err(RecordError::new(format!("{at}/kinds"), "a shipped definition claims at least one kind"));
            }
            Ok(ShippedDefinition::from_record(document.path, record))
        })
        .collect()
}

fn accept_function_catalog(_: &Acceptance<'_>, catalog: FunctionCatalog) -> Result<FunctionCatalog, RecordError> {
    catalog.check_record()?;
    Ok(catalog)
}

/// Prepares the payload from the compiled sources over the freshly prepared
/// Simplified Schema data `fresh`, consulting no artifact, registry, or
/// identity memo, and runs every generation check: the baseline check, and
/// that each catalog, definition, and function catalog prepares.
///
/// ## Errors
///
/// A message naming the first source that does not prepare or check.
pub fn prepare_payload(fresh: &FreshLower) -> Result<DarkmatterPayload, String> {
    let mode = PreparationMode::Uncached(fresh);
    if embedded::find(SHIPPED, functions::CATALOG_PATH).map(|document| document.text) != Some(functions::CATALOG_SOURCE) {
        return Err(format!("`{}` is compiled in twice with different text", functions::CATALOG_PATH));
    }

    let tables = SHIPPED
        .iter()
        .filter(|document| document.path.starts_with(PARTIALS))
        .map(|document| {
            NamedTypeTableRecord::prepare(document, mode).map_err(|error| format!("`{}`: {error}", document.path))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let named_types = NamedTypesRecord { inventory: SHIPPED.iter().map(|document| document.path.to_string()).collect(), tables };

    let catalog = GlobalCatalogRecord::prepare(DARKMATTER_CATALOG_PATH, SHIPPED, DARKMATTER_GLOBALS, mode)
        .map_err(|error| error.to_string())?;
    let (schema, definitions) = GlobalCatalog::from_record(catalog.clone(), DARKMATTER_GLOBALS)
        .map_err(|error| format!("the global catalog record does not rebuild: {}", error.message))?
        .baseline()
        .ok_or("the global catalog declares no `fm` shape")?;
    let baseline = simplified_schema::simplified::to_json_schema_with_definitions(&schema, &definitions)
        .map_err(|error| format!("the baseline does not lower: {error}"))?;
    crate::note_work!(BaselineCheck);
    simplified_schema::resolve::validate_baseline_schema(&baseline).map_err(|error| error.to_string())?;
    let inline_baseline = super::catalog::darkmatter_inline_baseline_with_mode(mode)
        .map_err(|reason| format!("the inline baseline does not resolve: {reason}"))?;

    let documents = embedded::shipped_kind_definitions();
    let kind_definitions = documents
        .iter()
        .map(|document| {
            let definition = super::yaml_documents::registry::prepare_shipped(document.path, documents, mode)
                .map_err(|error| format!("`{}`: {error}", document.path))?;
            kind_definition_record(document.path, definition.resolved(), definition.kinds())
        })
        .collect::<Result<Vec<_>, _>>()?;

    let functions_catalog = functions::load_catalog_with_mode(
        functions::CATALOG_SOURCE,
        functions::CATALOG_PATH,
        functions::EXAMPLE_FILES,
        mode,
    )
    .map_err(|error| error.to_string())?;
    let effects = functions::load_catalog_with_mode(
        crate::effects::types::MUTATIONS_SOURCE,
        crate::effects::types::MUTATIONS_PATH,
        &[],
        mode,
    )
    .map_err(|error| error.to_string())?;
    for (name, catalog) in [("functions", &functions_catalog), ("effects", &effects)] {
        catalog.check_record().map_err(|error| format!("the {name} catalog breaks `{}`: {}", error.pointer, error.message))?;
    }

    Ok(DarkmatterPayload {
        named_types,
        catalog,
        baseline,
        inline_baseline,
        kind_definitions,
        functions: functions_catalog,
        effects,
    })
}

/// The record of one prepared definition. Every path list of an embedded
/// resolution must be empty: a shipped artifact names no host file, so a
/// non-empty one means the preparation reached outside the compiled table.
fn kind_definition_record(
    path: &str,
    resolved: &simplified_schema::resolve::ResolvedSchema,
    kinds: &[String],
) -> Result<KindDefinitionRecord, String> {
    let lists = [
        ("imports", resolved.imports.len()),
        ("examples", resolved.examples.len()),
        ("referenced_files", resolved.referenced_files.len()),
        ("searched", resolved.searched.len()),
        ("advisories", resolved.advisories.len()),
    ];
    if let Some((list, _)) = lists.iter().find(|(_, length)| *length > 0) {
        return Err(format!("`{path}`'s preparation lists host `{list}`, which a shipped artifact cannot hold"));
    }
    Ok(KindDefinitionRecord {
        identity: path.to_string(),
        simplified: resolved.simplified.clone(),
        definitions: resolved.definitions.as_ref().clone(),
        json_schema: resolved.json_schema.clone(),
        origin: OriginRecord::of(&resolved.origin),
        kinds: kinds.to_vec(),
    })
}

/// A fresh, uncached generation of Darkmatter's artifact, kept for the
/// artifacts built over it (Claudine's): the generated bytes and their
/// fingerprint as a dependency.
pub struct FreshDarkmatter {
    artifact: Vec<u8>,
}

impl FreshDarkmatter {
    /// Generates Darkmatter's artifact over `lower`, the fresh Simplified
    /// Schema preparation of the same generation.
    ///
    /// ## Errors
    ///
    /// A message when preparation or a generation check fails.
    pub fn prepare(lower: &FreshLower) -> Result<Self, String> {
        crate::note_work!(ArtifactGeneration);
        let payload = prepare_payload(lower)?;
        let dependencies = [lower.as_dependency()];
        let artifact = mechanism::generate(&descriptor_over(sources(), &dependencies), &payload)?;
        Ok(Self { artifact })
    }

    /// The generated artifact.
    pub fn artifact(&self) -> &[u8] {
        &self.artifact
    }

    /// The generated artifact as a dependency of a higher artifact.
    pub fn as_dependency(&self) -> LowerArtifact {
        LowerArtifact::Current(mechanism::DependencyFingerprint::of_bytes(OWNER, PREPARATION_REVISION, &self.artifact))
    }
}

impl std::fmt::Debug for FreshDarkmatter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("FreshDarkmatter").field("artifact_bytes", &self.artifact.len()).finish()
    }
}

/// Generates Darkmatter's artifact from current text along the whole
/// dependency chain, consulting no checked-in artifact or process registry.
///
/// ## Errors
///
/// A message when preparation or a generation check fails.
pub fn generate() -> Result<Vec<u8>, String> {
    let lower = FreshLower::prepare()?;
    FreshDarkmatter::prepare(&lower).map(|fresh| fresh.artifact)
}

/// Accepts `bytes` as Darkmatter's artifact over the process's Simplified
/// Schema artifact, for generation commands that report where an artifact is
/// refused.
///
/// ## Errors
///
/// The [`Refusal`] of the first failed check.
pub fn check_artifact(bytes: &[u8]) -> Result<(), Refusal> {
    Registry::load(Some(bytes)).check_all()
}
