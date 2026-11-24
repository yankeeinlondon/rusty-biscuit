//! Claudine's shipped artifact: the prepared form of its registration
//! inventory.
//!
//! Claudine compiles in the files under `claudine/registration/`
//! ([`crate::composition::inventory`]). Preparing them (resolving and
//! lowering each trigger payload, checking it merges and compiles, preparing
//! the namespace catalog and the YAML kind definitions) used to run on first
//! use in every process. The checked-in artifact,
//! `claudine/lib/generated/shipped.json`, holds the result; [`registry`]
//! loads it once per process through the shared mechanism
//! ([`simplified_schema::shipped`]) after proving it describes exactly the
//! compiled inventory, the current Simplified Schema and Darkmatter
//! artifacts it was built over, and this crate's [`PREPARATION_REVISION`].
//!
//! Each payload member is a section decoded on first read. A refused
//! artifact or section is never used; its consumer prepares from the
//! compiled text instead:
//!
//! | View | Consumer | Fallback |
//! | --- | --- | --- |
//! | [`Registry::registered`] | [`registered_payload`], served to [`claudine_hosts`](super::bundle::claudine_hosts)'s profile | registration prepares the trigger's payload |
//! | [`Registry::globals`] | [`claudine_globals`](super::globals::claudine_globals) | the namespace members prepared ([`prepare_namespace_catalog`]) |
//! | [`Registry::kinds`] | [`formal_sequence_kind`] | the `yaml-schema` members prepared ([`prepare_kinds`]) |
//!
//! Every record keeps its member's bundle path (`schemas/seq.yaml`) as its
//! provenance. Regenerate with `just shipped-schemas` from the repository
//! root after editing `claudine/registration/` or the code that prepares
//! it; see `claudine/docs/topics/schemas.md`.

use std::sync::OnceLock;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use darkmatter::markdown::schemas::catalog::{GlobalCatalog, GlobalCatalogRecord};
use darkmatter::markdown::schemas::shipped::{self as darkmatter_shipped, FreshDarkmatter};
use serde::{Deserialize, Serialize};
use simplified_schema::embedded::EmbeddedDocument;
use simplified_schema::shipped::{
    self as mechanism, Acceptance, AcceptedArtifact, ArtifactDescriptor, CanonicalDigestRecord, FreshLower, LowerArtifact,
    RecordError, Refusal, Section, Sections,
};
use simplified_schema::simplified::DefinitionTable;
use simplified_schema::triggers::assemble::ResolvedPayload;
use simplified_schema::SchemaShape;

use super::globals::{CLAUDINE_GLOBALS, HOST_CATALOG_IDENTITY, globals_from_sources, prepare_namespace_catalog};
use crate::composition::inventory::{self, MemberKind};

/// The owner name in Claudine's artifact header.
pub const OWNER: &str = "claudine";

/// Claudine's preparation revision. Increment it whenever code changes what
/// the shipped sources prepare to (a record's shape, or how the bundle or the
/// host catalog is prepared), then run `just shipped-schemas`.
pub const PREPARATION_REVISION: u32 = 7;

/// The artifact's repository-relative path.
pub const ARTIFACT_PATH: &str = "claudine/lib/generated/shipped.json";

/// The command that regenerates every shipped artifact, from the repository
/// root.
pub const REGENERATE_COMMAND: &str = "just shipped-schemas";

/// The checked-in artifact compiled into this build.
pub const ARTIFACT: &[u8] = include_bytes!("../../../generated/shipped.json");

/// Every compiled-in text Claudine's preparation reads, by virtual path:
/// every inventory member as `claudine/registration/{path}`. A change to any
/// member's bytes makes the artifact stale.
pub fn sources() -> &'static [EmbeddedDocument<'static>] {
    static PATHS: OnceLock<Vec<(String, &'static str)>> = OnceLock::new();
    static SOURCES: OnceLock<Vec<EmbeddedDocument<'static>>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        let paths = PATHS.get_or_init(|| {
            inventory::members().iter().map(|member| (source_identity(member.path), member.text)).collect()
        });
        paths.iter().map(|(path, text)| EmbeddedDocument { path, text }).collect()
    })
}

/// The source identity of inventory member `path` in the artifact header.
fn source_identity(path: &str) -> String {
    format!("{}/{path}", inventory::SOURCE_DIR)
}

/// What Claudine's artifact must describe over `sources` and the lower
/// artifact states `lower` (Simplified Schema's, then Darkmatter's).
pub fn descriptor_over<'a>(sources: &'a [EmbeddedDocument<'a>], lower: &'a [LowerArtifact]) -> ArtifactDescriptor<'a> {
    ArtifactDescriptor {
        owner: OWNER,
        preparation_revision: PREPARATION_REVISION,
        sources,
        dependencies: lower,
        configuration: &[],
    }
}

/// The artifact payload.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudinePayload {
    /// Each trigger member's payload as registration prepares it, in
    /// bundle-path order.
    pub registered: Vec<RegisteredTriggerRecord>,
    /// The host catalog, prepared from the inventory's namespace members.
    pub globals: GlobalCatalogRecord,
    /// The inventory's `yaml-schema` members, prepared, in bundle-path order.
    pub kind_definitions: Vec<KindRecord>,
}

/// One prepared `kind: yaml-schema` member: what a document of its kinds is
/// validated against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindRecord {
    /// The member's bundle path (`schemas/formal-sequence-file.yaml`), its
    /// provenance.
    pub path: String,
    /// The kinds its `$schema` claims, in first-declared order.
    pub kinds: Vec<String>,
    /// The lowered JSON Schema of its `$schema`, an object.
    pub json_schema: serde_json::Value,
}

/// One trigger member's registered payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredTriggerRecord {
    /// The member's bundle path (`schemas/composition.yaml`), its provenance.
    pub path: String,
    /// Its prepared payload; `null` for a matcher-only trigger.
    #[serde(deserialize_with = "simplified_schema::shipped::present")]
    pub payload: Option<RegisteredPayloadRecord>,
}

/// A registered trigger payload, resolved and checked inside the embedded
/// bundle: an object schema or a root union of object schemas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredPayloadRecord {
    /// The lowered JSON Schema.
    pub json_schema: serde_json::Value,
    /// The object payload's shape; `null` for a root union or a payload with
    /// no SimplifiedSchema projection.
    #[serde(deserialize_with = "simplified_schema::shipped::present")]
    pub shape: Option<SchemaShape>,
    /// A root union's inline arms; `null` for an object payload.
    #[serde(deserialize_with = "simplified_schema::shipped::present")]
    pub union_arms: Option<Vec<SchemaShape>>,
    /// The cyclic named types the payload reaches, in allocation order.
    pub definitions: DefinitionTable,
    /// The bundle file declaring each imported default, by property path.
    pub default_files: Vec<DefaultFileRecord>,
    /// The canonical digest of `json_schema`.
    pub digest: CanonicalDigestRecord,
}

/// One entry of a payload's imported-default files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultFileRecord {
    /// The object-property path of the defaulted property.
    pub property: Vec<String>,
    /// The declaring file's virtual path with `/` separators on every
    /// platform (`<claudine>/schemas/shared.yaml`).
    pub file: String,
}

/// `path` spelled with `/` separators, so the artifact's bytes do not depend
/// on the platform that generated it.
fn portable_path(path: &std::path::Path) -> Option<String> {
    let parts = path.components().map(|part| part.as_os_str().to_str()).collect::<Option<Vec<_>>>()?;
    Some(parts.join("/"))
}

/// A [`portable_path`] spelling as this platform's path: the value uncached
/// preparation produces.
fn native_path(portable: &str) -> PathBuf {
    portable.split('/').collect()
}

/// An accepted [`KindRecord`], ready to validate with.
#[derive(Debug)]
pub struct PreparedKind {
    /// [`KindRecord::path`].
    pub path: String,
    /// [`KindRecord::kinds`].
    pub kinds: Vec<String>,
    /// [`KindRecord::json_schema`], shared by every validator built from it.
    pub json_schema: std::sync::Arc<serde_json::Value>,
}

/// An accepted artifact's entries, one [`Section`] per payload member.
struct Prepared {
    registered: Section<HashMap<String, Option<ResolvedPayload>>>,
    globals: Section<GlobalCatalog>,
    kinds: Section<Vec<PreparedKind>>,
}

/// Claudine's loaded artifact, or why it was refused. Every view returns the
/// accepted entry, or its consumer's preparation from text when the artifact,
/// or that entry's section, was refused.
pub struct Registry {
    artifact: Result<AcceptedArtifact<Prepared>, Refusal>,
}

impl Registry {
    /// Loads `bytes` as Claudine's artifact over the process's Simplified
    /// Schema and Darkmatter artifacts.
    pub fn load(bytes: Option<&[u8]>) -> Self {
        Self::load_over(
            bytes,
            [
                simplified_schema::shipped::registry::registry().as_dependency(),
                darkmatter_shipped::registry().as_dependency(),
            ],
        )
    }

    /// Loads `bytes` as Claudine's artifact over the lower artifact states
    /// `lower`, Simplified Schema's then Darkmatter's (tests inject a refused
    /// or changed one). Nothing past the header and the payload's member list
    /// is decoded here, and nothing is prepared from text, even on a refusal.
    pub fn load_over(bytes: Option<&[u8]>, lower: [LowerArtifact; 2]) -> Self {
        let artifact = mechanism::accept_deferred(&descriptor_over(sources(), &lower), bytes, accept_sections);
        if let Err(refusal) = &artifact {
            tracing::debug!(owner = OWNER, %refusal, "shipped artifact refused; preparing compiled sources on use");
        }
        Self { artifact }
    }

    /// Why the artifact was refused, or `None` when its header and member
    /// list were accepted. A section can still be refused when first read
    /// ([`Self::check_all`]).
    pub fn refusal(&self) -> Option<&Refusal> {
        self.artifact.as_ref().err()
    }

    /// Reads every section and returns the first refusal: the artifact's, or
    /// the first section's in payload order.
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
            prepared.registered.get(accept_registered).err(),
            prepared.globals.get(accept_globals).err(),
            prepared.kinds.get(accept_kinds).err(),
        ];
        match refusals.into_iter().flatten().next() {
            Some(refusal) => Err(refusal.clone()),
            None => Ok(()),
        }
    }

    fn accepted(&self) -> Option<&Prepared> {
        self.artifact.as_ref().ok().map(AcceptedArtifact::payload)
    }

    /// The accepted registered payloads, by trigger bundle path (`None`
    /// for a matcher-only trigger); `None` when the artifact or the section
    /// was refused, so registration prepares each payload.
    pub fn registered(&self) -> Option<&HashMap<String, Option<ResolvedPayload>>> {
        self.accepted()?.registered.get(accept_registered).ok()
    }

    /// Claudine's host catalog.
    ///
    /// ## Panics
    ///
    /// When the accepted entry is unavailable and the namespace members do
    /// not prepare: a build defect a test catches.
    pub fn globals(&self) -> &GlobalCatalog {
        match self.accepted().map(|prepared| prepared.globals.get(accept_globals)) {
            Some(Ok(catalog)) => catalog,
            _ => globals_from_sources(),
        }
    }

    /// The inventory's prepared `yaml-schema` members.
    ///
    /// ## Panics
    ///
    /// When the accepted entry is unavailable and the members do not
    /// prepare: a build defect a test catches.
    pub fn kinds(&self) -> &[PreparedKind] {
        match self.accepted().map(|prepared| prepared.kinds.get(accept_kinds)) {
            Some(Ok(kinds)) => kinds,
            _ => kinds_from_sources(),
        }
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

/// The process registry's prepared payload of the trigger at bundle path
/// `path`, which [`claudine_hosts`](super::bundle::claudine_hosts) serves to
/// registration; `None` when the artifact or its section was refused, or for
/// a matcher-only trigger, so registration prepares it from text. The
/// artifact describes exactly the compiled inventory, so a served payload
/// was prepared from the bytes the profile verifies.
pub fn registered_payload(path: &str) -> Option<ResolvedPayload> {
    registry().registered()?.get(path)?.clone()
}

fn accept_sections(_: &Acceptance<'_>, sections: &mut Sections) -> Result<Prepared, RecordError> {
    Ok(Prepared {
        registered: sections.take("registered")?,
        globals: sections.take("globals")?,
        kinds: sections.take("kind_definitions")?,
    })
}

fn accept_registered(
    _: &Acceptance<'_>,
    records: Vec<RegisteredTriggerRecord>,
) -> Result<HashMap<String, Option<ResolvedPayload>>, RecordError> {
    let expected: Vec<String> = inventory::members_of(MemberKind::Trigger).map(|member| member.bundle_path()).collect();
    if records.len() != expected.len() {
        return Err(RecordError::new("", format!("{} triggers are recorded, {} are compiled in", records.len(), expected.len())));
    }
    records
        .into_iter()
        .zip(expected)
        .enumerate()
        .map(|(index, (record, path))| {
            let at = format!("/{index}");
            if record.path != path {
                return Err(RecordError::new(format!("{at}/path"), format!("`{}` is recorded where `{path}` belongs", record.path)));
            }
            let Some(payload) = record.payload else { return Ok((path, None)) };
            if !payload.json_schema.is_object() {
                return Err(RecordError::new(format!("{at}/payload/json_schema"), "a payload lowers to a JSON Schema object"));
            }
            if payload.union_arms.is_some() == payload.shape.is_some() {
                return Err(RecordError::new(
                    format!("{at}/payload"),
                    "a payload records an object shape or root-union arms, not both",
                ));
            }
            let digest = payload
                .digest
                .digest()
                .map_err(|message| RecordError::new(format!("{at}/payload/digest/revision"), message))?;
            let resolved = ResolvedPayload {
                json_schema: Arc::new(payload.json_schema),
                shape: payload.shape.map(Arc::new),
                union_arms: payload.union_arms.map(Arc::from),
                definitions: Arc::new(payload.definitions),
                dependencies: Vec::new(),
                searched: Vec::new(),
                advisories: Vec::new(),
                pending: None,
                digest: Some(digest),
                default_files: payload.default_files.into_iter().map(|entry| (entry.property, native_path(&entry.file))).collect(),
            };
            Ok((path, Some(resolved)))
        })
        .collect()
}

fn accept_globals(_: &Acceptance<'_>, record: GlobalCatalogRecord) -> Result<GlobalCatalog, RecordError> {
    if record.identity != HOST_CATALOG_IDENTITY {
        return Err(RecordError::new("/identity", format!("`{}` is not the host catalog", record.identity)));
    }
    GlobalCatalog::from_record(record, CLAUDINE_GLOBALS)
}

fn accept_kinds(_: &Acceptance<'_>, records: Vec<KindRecord>) -> Result<Vec<PreparedKind>, RecordError> {
    let expected: Vec<String> = inventory::members_of(MemberKind::YamlSchema).map(|member| member.bundle_path()).collect();
    if records.len() != expected.len() {
        return Err(RecordError::new("", format!("{} kinds are recorded, {} are compiled in", records.len(), expected.len())));
    }
    records
        .into_iter()
        .zip(expected)
        .enumerate()
        .map(|(index, (record, path))| {
            let at = format!("/{index}");
            if record.path != path {
                return Err(RecordError::new(format!("{at}/path"), format!("`{}` is recorded where `{path}` belongs", record.path)));
            }
            if !record.json_schema.is_object() {
                return Err(RecordError::new(format!("{at}/json_schema"), "a kind lowers to a JSON Schema object"));
            }
            if record.kinds.is_empty() {
                return Err(RecordError::new(format!("{at}/kinds"), "a kind definition claims at least one kind"));
            }
            Ok(PreparedKind { path: record.path, kinds: record.kinds, json_schema: std::sync::Arc::new(record.json_schema) })
        })
        .collect()
}

/// Prepares every `yaml-schema` member of the inventory from its embedded
/// bytes inside the inventory's bundle scope, as a reader of the published
/// bundle prepares it, and returns its record.
///
/// ## Errors
///
/// A message naming the first member that does not prepare or claims no
/// kind.
pub fn prepare_kinds() -> Result<Vec<KindRecord>, String> {
    use simplified_schema::yaml_documents::{RegisteredDefinition, RegisteredSource};
    let sources = inventory::bundle_sources();
    let context = inventory::bundle_context();
    inventory::members_of(MemberKind::YamlSchema)
        .map(|member| {
            let path = member.bundle_path();
            let location = sources.path_of(&path).ok_or_else(|| format!("`{path}` is not a bundle member"))?;
            let (resolved, _) = simplified_schema::resolve::bundle::with(sources, || {
                simplified_schema::resolve::prepare_yaml_schema_text(
                    &location,
                    member.text,
                    &[],
                    &context,
                    darkmatter::markdown::schemas::shipped_sources(),
                )
            });
            let resolved = resolved.map_err(|error| format!("`{path}` does not prepare: {error}"))?;
            let json_schema = resolved.json_schema.clone();
            let source = RegisteredSource { host: crate::composition::registration::HOST_ID.to_string(), path: path.clone(), file: location };
            let definition =
                RegisteredDefinition::new(source, member.text, resolved, path.clone()).map_err(|error| format!("`{path}` claims no kind: {error}"))?;
            Ok(KindRecord { kinds: definition.definition().kinds().to_vec(), path, json_schema })
        })
        .collect()
}

/// [`prepare_kinds`], once per process: the fallback when the shipped
/// artifact's entry was refused.
fn kinds_from_sources() -> &'static [PreparedKind] {
    static KINDS: OnceLock<Vec<PreparedKind>> = OnceLock::new();
    KINDS.get_or_init(|| {
        darkmatter::note_work!(ShippedRuntimePreparation);
        prepare_kinds()
            .unwrap_or_else(|error| panic!("Claudine's YAML kinds must prepare: {error}"))
            .into_iter()
            .map(|record| PreparedKind { path: record.path, kinds: record.kinds, json_schema: std::sync::Arc::new(record.json_schema) })
            .collect()
    })
}

/// The prepared kind that claims `kind: sequence`
/// (`schemas/formal-sequence-file.yaml`).
///
/// ## Panics
///
/// When no inventory member claims `sequence`: a build defect a test
/// catches.
pub fn formal_sequence_kind() -> &'static PreparedKind {
    registry()
        .kinds()
        .iter()
        .find(|kind| kind.kinds.iter().any(|claimed| claimed == "sequence"))
        .expect("Claudine's inventory defines `kind: sequence`")
}

/// Prepares the payload from the compiled sources over the freshly prepared
/// Simplified Schema data `lower`, and runs every check discovery and
/// catalog loading run. The trigger bundle consults no artifact, registry,
/// or identity-keyed memo. The namespace catalog and the YAML kinds resolve
/// inside the inventory's bundle scope, which has no uncached mode, so they
/// read the lower registries; a registry serves only an artifact whose
/// recorded sources match the compiled text.
///
/// ## Errors
///
/// A message naming the first source that does not prepare or check.
pub fn prepare_payload(_lower: &FreshLower) -> Result<ClaudinePayload, String> {
    let registered = prepare_registered()?;
    let catalog = prepare_namespace_catalog()?;
    let globals = GlobalCatalogRecord::of(HOST_CATALOG_IDENTITY, &catalog.catalog);
    GlobalCatalog::from_record(globals.clone(), CLAUDINE_GLOBALS)
        .map_err(|error| format!("the host catalog record does not rebuild: {}", error.message))?;
    let kind_definitions = prepare_kinds()?;
    Ok(ClaudinePayload { registered, globals, kind_definitions })
}

/// Prepares every trigger member of the inventory as registration prepares
/// it in process ([`darkmatter::config::registration::prepare_embedded_payload`]
/// over the compiled registration's embedded snapshot), and returns its
/// records in bundle-path order.
///
/// ## Errors
///
/// A message naming the first trigger that does not prepare, carries an
/// advisory (which the record does not keep), or names a file outside the
/// embedded bundle.
pub fn prepare_registered() -> Result<Vec<RegisteredTriggerRecord>, String> {
    let snapshot = super::bundle::snapshot_of(crate::composition::registration::RegistrationInventory::compiled_claudine())
        .map_err(|refusal| format!("the compiled registration does not verify: {}", refusal.message))?;
    let owned = darkmatter::config::registration::md_owned_names();
    inventory::members_of(MemberKind::Trigger)
        .map(|member| {
            let path = member.bundle_path();
            let matcher_only = simplified_schema::triggers::parse_trigger_envelope_from_str(member.text)
                .map_err(|error| format!("`{path}` is not a valid trigger: {error}"))?
                .is_some_and(|envelope| envelope.payload.is_none());
            if matcher_only {
                return Ok(RegisteredTriggerRecord { path, payload: None });
            }
            let payload = darkmatter::config::registration::prepare_embedded_payload(&snapshot, &path, owned)
                .map_err(|error| format!("`{path}` does not prepare: {error}"))?
                .ok_or_else(|| format!("`{path}` is not a schema trigger"))?;
            if !payload.advisories.is_empty() {
                return Err(format!("`{path}` prepares with advisories, which the artifact does not record: {:?}", payload.advisories));
            }
            let default_files = payload
                .default_files
                .iter()
                .map(|(property, file)| {
                    portable_path(file)
                        .map(|file| DefaultFileRecord { property: property.clone(), file })
                        .ok_or_else(|| format!("`{path}` names a default file that is not UTF-8"))
                })
                .collect::<Result<_, _>>()?;
            Ok(RegisteredTriggerRecord {
                path,
                payload: Some(RegisteredPayloadRecord {
                    digest: CanonicalDigestRecord::of(&payload.json_schema),
                    json_schema: (*payload.json_schema).clone(),
                    shape: payload.shape.as_deref().cloned(),
                    union_arms: payload.union_arms.as_deref().map(<[SchemaShape]>::to_vec),
                    definitions: (*payload.definitions).clone(),
                    default_files,
                }),
            })
        })
        .collect()
}

/// A fresh, uncached generation of Claudine's artifact.
pub struct FreshClaudine {
    artifact: Vec<u8>,
}

impl FreshClaudine {
    /// Generates Claudine's artifact over `lower` and `darkmatter`, the fresh
    /// lower preparations of the same generation.
    ///
    /// ## Errors
    ///
    /// A message when preparation or a generation check fails.
    pub fn prepare(lower: &FreshLower, darkmatter: &FreshDarkmatter) -> Result<Self, String> {
        darkmatter::note_work!(ArtifactGeneration);
        let payload = prepare_payload(lower)?;
        let dependencies = [lower.as_dependency(), darkmatter.as_dependency()];
        let artifact = mechanism::generate(&descriptor_over(sources(), &dependencies), &payload)?;
        Ok(Self { artifact })
    }

    /// The generated artifact.
    pub fn artifact(&self) -> &[u8] {
        &self.artifact
    }
}

impl std::fmt::Debug for FreshClaudine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("FreshClaudine").field("artifact_bytes", &self.artifact.len()).finish()
    }
}

/// Generates Claudine's artifact from current text along the whole
/// dependency chain, consulting no checked-in artifact or process registry.
///
/// ## Errors
///
/// A message when preparation or a generation check fails.
pub fn generate() -> Result<Vec<u8>, String> {
    let lower = FreshLower::prepare()?;
    let darkmatter = FreshDarkmatter::prepare(&lower)?;
    FreshClaudine::prepare(&lower, &darkmatter).map(|fresh| fresh.artifact)
}

/// Accepts `bytes` as Claudine's artifact over the process's lower
/// artifacts and reads every section, for generation commands that report
/// where an artifact is refused.
///
/// ## Errors
///
/// The [`Refusal`] of the first failed check.
pub fn check_artifact(bytes: &[u8]) -> Result<(), Refusal> {
    Registry::load(Some(bytes)).check_all()
}
