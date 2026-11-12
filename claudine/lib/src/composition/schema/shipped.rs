//! Claudine's shipped artifact: the prepared form of its compiled-in schema
//! bundle and host catalog.
//!
//! Claudine compiles in the files under `claudine/schemas/`
//! ([`CLAUDINE_SCHEMA_FILES`](super::bundle::CLAUDINE_SCHEMA_FILES)) and its
//! host catalog ([`HOST_CATALOG_TEXT`]). Preparing them (resolving and
//! lowering each trigger payload, checking it merges and compiles, and
//! loading the catalog) used to run on first use in every process. The
//! checked-in artifact, `claudine/lib/generated/shipped.json`, holds the
//! result; [`registry`] loads it once per process through the shared
//! mechanism ([`simplified_schema::shipped`]) after proving it describes
//! exactly the compiled text, the current Simplified Schema and Darkmatter
//! artifacts it was built over, and this crate's [`PREPARATION_REVISION`].
//!
//! Each payload member is a section decoded on first read. A refused
//! artifact or section is never used; its consumer prepares from the
//! compiled text instead, as it always did:
//!
//! | View | Consumer | Fallback |
//! | --- | --- | --- |
//! | [`Registry::bundle`] | [`BUNDLE`], attached by [`claudine_schema_bundle`](super::bundle::claudine_schema_bundle) | trigger discovery prepares each document |
//! | [`Registry::globals`] | [`claudine_globals`](super::globals::claudine_globals) | the catalog loaded from [`HOST_CATALOG_TEXT`] |
//!
//! Discovery's selection rules do not change: a same-name file in a schema
//! root still replaces a bundle document and is prepared at run time.
//! Regenerate with `just shipped-schemas` from the repository root after
//! editing `claudine/schemas/` or the code that prepares it; see
//! `claudine/docs/topics/schemas.md`.

use std::sync::OnceLock;

use darkmatter::markdown::schemas::catalog::{GlobalCatalog, GlobalCatalogRecord};
use darkmatter::markdown::schemas::shipped::{self as darkmatter_shipped, FreshDarkmatter};
use darkmatter::markdown::schemas::triggers::{
    PreparedBundleDescriptor, PreparedTriggerBundle, TriggerBundleRecord, prepare_trigger_bundle,
};
use serde::{Deserialize, Serialize};
use simplified_schema::embedded::EmbeddedDocument;
use simplified_schema::shipped::{
    self as mechanism, Acceptance, AcceptedArtifact, ArtifactDescriptor, FreshLower, LowerArtifact, PreparationMode,
    RecordError, Refusal, Section, Sections,
};

use super::bundle::{CLAUDINE_SCHEMA_BUNDLE_NAME, source_bundle};
use super::globals::{CLAUDINE_GLOBALS, HOST_CATALOG_PATH, HOST_CATALOG_TEXT, globals_from_sources};

/// The owner name in Claudine's artifact header.
pub const OWNER: &str = "claudine";

/// Claudine's preparation revision. Increment it whenever code changes what
/// the shipped sources prepare to (a record's shape, or how the bundle or the
/// host catalog is prepared), then run `just shipped-schemas`.
pub const PREPARATION_REVISION: u32 = 1;

/// The artifact's repository-relative path.
pub const ARTIFACT_PATH: &str = "claudine/lib/generated/shipped.json";

/// The command that regenerates every shipped artifact, from the repository
/// root.
pub const REGENERATE_COMMAND: &str = "just shipped-schemas";

/// The checked-in artifact compiled into this build.
pub const ARTIFACT: &[u8] = include_bytes!("../../../generated/shipped.json");

/// Every compiled-in text Claudine's preparation reads, by virtual path: each
/// bundle document as `claudine/{file}` (the identity a
/// [`PreparedTriggerBundle`] matches), then the host catalog at
/// [`HOST_CATALOG_PATH`], whose nested path is never mistaken for a bundle
/// document.
pub fn sources() -> &'static [EmbeddedDocument<'static>] {
    static PATHS: OnceLock<Vec<(String, &'static str)>> = OnceLock::new();
    static SOURCES: OnceLock<Vec<EmbeddedDocument<'static>>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        let paths = PATHS.get_or_init(|| source_bundle().documents());
        let mut documents: Vec<EmbeddedDocument<'static>> =
            paths.iter().map(|(path, text)| EmbeddedDocument { path, text }).collect();
        documents.push(EmbeddedDocument { path: HOST_CATALOG_PATH, text: HOST_CATALOG_TEXT });
        documents
    })
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
    /// What trigger discovery derives from the bundle, in sorted file order.
    pub bundle: TriggerBundleRecord,
    /// The host catalog, `claudine/schemas/globals.yaml`, resolved.
    pub globals: GlobalCatalogRecord,
}

/// An accepted artifact's entries, one [`Section`] per payload member.
struct Prepared {
    bundle: Section<PreparedTriggerBundle>,
    globals: Section<GlobalCatalog>,
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
        let refusals = [prepared.bundle.get(accept_bundle).err(), prepared.globals.get(accept_globals).err()];
        match refusals.into_iter().flatten().next() {
            Some(refusal) => Err(refusal.clone()),
            None => Ok(()),
        }
    }

    fn accepted(&self) -> Option<&Prepared> {
        self.artifact.as_ref().ok().map(AcceptedArtifact::payload)
    }

    /// The accepted prepared bundle; `None` when the artifact or the section
    /// was refused, so discovery prepares each bundle document.
    pub fn bundle(&self) -> Option<&PreparedTriggerBundle> {
        self.accepted()?.bundle.get(accept_bundle).ok()
    }

    /// Claudine's host catalog.
    ///
    /// ## Panics
    ///
    /// When the accepted entry is unavailable and the compiled-in catalog
    /// does not load: a build defect a test catches.
    pub fn globals(&self) -> &GlobalCatalog {
        match self.accepted().map(|prepared| prepared.globals.get(accept_globals)) {
            Some(Ok(catalog)) => catalog,
            _ => globals_from_sources(),
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

/// The process registry's prepared bundle, attached to
/// [`claudine_schema_bundle`](super::bundle::claudine_schema_bundle). Served
/// only while the supplied bundle has Claudine's name and exactly the files
/// it was prepared from.
pub static BUNDLE: PreparedBundleDescriptor = PreparedBundleDescriptor { owner: OWNER, provider: || registry().bundle() };

fn accept_sections(_: &Acceptance<'_>, sections: &mut Sections) -> Result<Prepared, RecordError> {
    Ok(Prepared { bundle: sections.take("bundle")?, globals: sections.take("globals")? })
}

fn accept_bundle(acceptance: &Acceptance<'_>, record: TriggerBundleRecord) -> Result<PreparedTriggerBundle, RecordError> {
    if record.bundle != CLAUDINE_SCHEMA_BUNDLE_NAME {
        return Err(RecordError::new("/bundle", format!("`{}` is not Claudine's bundle", record.bundle)));
    }
    PreparedTriggerBundle::from_record(acceptance, "", record)
}

fn accept_globals(_: &Acceptance<'_>, record: GlobalCatalogRecord) -> Result<GlobalCatalog, RecordError> {
    if record.identity != HOST_CATALOG_PATH {
        return Err(RecordError::new("/identity", format!("`{}` is not the host catalog", record.identity)));
    }
    GlobalCatalog::from_record(record, CLAUDINE_GLOBALS)
}

/// Prepares the payload from the compiled sources over the freshly prepared
/// Simplified Schema data `lower`, consulting no artifact, registry, or memo,
/// and runs every check discovery and catalog loading run.
///
/// ## Errors
///
/// A message naming the first source that does not prepare or check.
pub fn prepare_payload(lower: &FreshLower) -> Result<ClaudinePayload, String> {
    let mode = PreparationMode::Uncached(lower);
    let bundle = prepare_trigger_bundle(source_bundle(), mode).map_err(|error| error.to_string())?;
    let catalog = [EmbeddedDocument { path: HOST_CATALOG_PATH, text: HOST_CATALOG_TEXT }];
    let globals = GlobalCatalogRecord::prepare(HOST_CATALOG_PATH, &catalog, CLAUDINE_GLOBALS, mode)
        .map_err(|error| error.to_string())?;
    GlobalCatalog::from_record(globals.clone(), CLAUDINE_GLOBALS)
        .map_err(|error| format!("the host catalog record does not rebuild: {}", error.message))?;
    Ok(ClaudinePayload { bundle, globals })
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
