//! Host registration: loading the manifests that `register` entries name
//! into passive host profiles.
//!
//! A `register` entry of `.darkmatter.yaml` names a **host manifest**
//! (`kind: darkmatter-host`, [`manifest`]), one file a host program installs
//! that names its identifying trigger, its triggers, its global catalog, its
//! lifecycle artifact, and its function catalogs. [`load_registrations`]
//! turns each entry into a [`RegisteredHostProfile`], or into one warning
//! when anything about it is unusable:
//!
//! - the manifest is validated against the shipped closed envelope before a
//!   member is read;
//! - every member resolves from the manifest's directory as a local
//!   [`FileReference`]; a remote member is refused without being fetched;
//! - every member and every file it imports is read once, inside one
//!   retained read set, and recorded with its content hash;
//! - a profile is published only when all of it loads: no partial globals,
//!   functions, lifecycle events, or trigger payloads are ever exposed.
//!
//! Registration is passive: it writes nothing, fetches nothing, prompts for
//! nothing, installs no function handler, and runs no command. A profile is
//! analysis data, never runtime authority.
//!
//! Which host a document belongs to is [`select`]ed over [`RegisteredHosts`]
//! (built on [`agreement`]); a selected host's triggers layer into the
//! effective schema through
//! [`DarkmatterSchemas::with_registered_hosts`](crate::markdown::schemas::DarkmatterSchemas::with_registered_hosts).
//!
//! User-facing guide: `darkmatter/docs/topics/host-registration.md`.

pub mod manifest;
mod selection;

pub use selection::{HostSelection, LegacyHost, RegisteredHosts, Selected, select};

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use biscuit_file::{FileReference, FileReferenceKind, FileResolutionContext, PathIdentity, canonicalize_simplified};
use serde_json::{Value, json};
use simplified_schema::SchemaRoot;
use simplified_schema::resolve::buffers;
use simplified_schema::triggers::assemble::{ResolvedPayload, resolve_trigger_payload};
use simplified_schema::triggers::{
    IdentifyingTrigger, LoadedTrigger, TriggerEnvelope, admit_identifying_trigger, parse_trigger_envelope_from_str,
};

use super::load::{ConfigSnapshot, Registration, line_column};
use super::migration_receipt::content_hash;
use crate::lifecycle::host_profile::HostLifecycleProfile;
use crate::markdown::compose::expression::host::FactsArtifact;
use crate::markdown::compose::expression::{
    Availability, BindingView, HostFunctions, HostGlobals, RuntimeBinding, ScopeId, Severity, UnavailabilityReason,
    is_reserved_namespace,
};
use crate::markdown::schemas::catalog::{DARKMATTER_GLOBALS, GlobalCatalog, host_function_set_at};

/// A `register` entry failed to load (a missing or invalid manifest or
/// member, a refused lifecycle or facts artifact, a host mismatch), or two
/// manifests claim one host with different content. Analysis continues
/// without that host.
pub const HOST_REGISTRATION_CODE: &str = "dm.host.registration";

/// An identifying trigger could match on Darkmatter-owned keys alone.
pub const HOST_IDENTIFY_TOO_BROAD_CODE: &str = simplified_schema::triggers::IDENTIFY_TOO_BROAD_CODE;

/// Two registered hosts identify the same document ([`select`]).
pub const HOST_AMBIGUOUS_CODE: &str = "dm.host.ambiguous";

/// `md compose` composes a document a registered host identifies
/// ([`RegisteredHostProfile::foreign_document_notice`]); a
/// [`Severity::Info`] notice, once per root request.
pub const HOST_FOREIGN_DOCUMENT_CODE: &str = "dm.host.foreign_document";

/// Why a registered host's global has no value when a process other than the
/// host composes a document the host identifies
/// ([`RegisteredHostProfile::analysis_globals`]).
pub const HOST_RUNTIME_ONLY_CODE: &str = "dm.host.runtime_only";

/// One refused registration (or same-host conflict): always a
/// [`Severity::Warning`], located in the file where the problem is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationDiagnostic {
    /// [`HOST_REGISTRATION_CODE`] or [`HOST_IDENTIFY_TOO_BROAD_CODE`].
    pub code: &'static str,
    #[allow(missing_docs)]
    pub severity: Severity,
    /// What happened, naming the entry and, when one failed, the member.
    pub message: String,
    /// The file the problem is in: the declaring configuration file when
    /// the manifest itself cannot be found, the manifest for its own shape or
    /// a member that cannot be resolved, else the member file.
    pub file: PathBuf,
    /// 1-based line in [`Self::file`], when known.
    pub line: Option<usize>,
    /// 1-based character column, when [`Self::line`] is known.
    pub column: Option<usize>,
    /// The `register` entry refused.
    pub entry: Registration,
    /// The manifest field of the failing member (`lifecycle`, `triggers[1]`,
    /// `functions[0].facts`); `None` when the manifest itself failed.
    pub member: Option<String>,
}

/// One file a profile read, with the BLAKE3 hash of the bytes read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostDependency {
    #[allow(missing_docs)]
    pub path: PathBuf,
    /// [`content_hash`] of the bytes read.
    pub content_hash: String,
}

/// A manifest's identity: two entries naming the same canonical path are
/// one manifest, loaded once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestIdentity {
    /// [`canonicalize_simplified`] of the resolved manifest path.
    pub path: PathBuf,
    /// [`content_hash`] of the manifest bytes.
    pub content_hash: String,
}

/// A registered trigger file: its loaded envelope and resolved payload.
///
/// The trigger's payload resolves from its own file, so relative imports
/// (`./action.yaml`) are anchored there. Its `pattern_cwd` is the tree root
/// of the context the registration was loaded in; a caller layering it for
/// a document in another tree sets that document's tree root, as for a
/// user-level schema root.
#[derive(Debug, Clone)]
pub struct RegisteredTrigger {
    /// The loaded trigger, ready for a
    /// [`TriggerRegistry`](simplified_schema::triggers::TriggerRegistry).
    pub trigger: LoadedTrigger,
    /// Its payload, resolved and checked at load.
    pub payload: ResolvedPayload,
}

impl RegisteredTrigger {
    /// The trigger file.
    ///
    /// ## Panics
    ///
    /// Never: a registered trigger is always a file trigger.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.trigger.source.path().expect("a registered trigger is a file")
    }
}

/// One `functions` pair: the analysis-only set its catalog and facts give.
#[derive(Debug, Clone)]
pub struct HostFunctionSet {
    /// The function catalog file.
    pub catalog: PathBuf,
    /// The facts artifact file.
    pub facts: PathBuf,
    /// Every function complete; no handler is installed.
    pub functions: HostFunctions,
}

/// A registered host: everything its manifest names, loaded together.
/// Immutable once published; share it with `Arc`.
#[derive(Debug)]
pub struct RegisteredHostProfile {
    host: String,
    version: String,
    manifest: ManifestIdentity,
    entries: Vec<Registration>,
    identify: RegisteredTrigger,
    identifying: IdentifyingTrigger,
    triggers: Vec<RegisteredTrigger>,
    globals_path: PathBuf,
    globals: Arc<GlobalCatalog>,
    lifecycle_path: PathBuf,
    lifecycle: Arc<HostLifecycleProfile>,
    functions: Vec<HostFunctionSet>,
    dependencies: Vec<HostDependency>,
    revision: String,
}

#[allow(missing_docs)]
impl RegisteredHostProfile {
    /// The manifest's `host`, which the lifecycle artifact records too.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// The manifest's `version`: the host release, not the format.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub const fn manifest(&self) -> &ManifestIdentity {
        &self.manifest
    }

    /// Every `register` entry naming this manifest, in entry order.
    #[must_use]
    pub fn entries(&self) -> &[Registration] {
        &self.entries
    }

    /// The identifying trigger file, as loaded.
    #[must_use]
    pub const fn identify_trigger(&self) -> &RegisteredTrigger {
        &self.identify
    }

    /// The identifying trigger's admitted conditions, for
    /// [`identifies`](simplified_schema::triggers::identifies).
    #[must_use]
    pub const fn identifying(&self) -> &IdentifyingTrigger {
        &self.identifying
    }

    /// The `triggers`, in manifest order.
    #[must_use]
    pub fn triggers(&self) -> &[RegisteredTrigger] {
        &self.triggers
    }

    #[must_use]
    pub fn globals_path(&self) -> &Path {
        &self.globals_path
    }

    #[must_use]
    pub const fn globals(&self) -> &Arc<GlobalCatalog> {
        &self.globals
    }

    #[must_use]
    pub fn lifecycle_path(&self) -> &Path {
        &self.lifecycle_path
    }

    #[must_use]
    pub const fn lifecycle(&self) -> &Arc<HostLifecycleProfile> {
        &self.lifecycle
    }

    /// The `functions` sets, in manifest order.
    #[must_use]
    pub fn functions(&self) -> &[HostFunctionSet] {
        &self.functions
    }

    /// Every file the load read (the manifest, each member, and each file a
    /// member imports), once each, in read order.
    #[must_use]
    pub fn dependencies(&self) -> &[HostDependency] {
        &self.dependencies
    }

    /// The content revision: a BLAKE3 digest over the manifest's meaning
    /// (its accepted value, not its comments or layout) and every other
    /// dependency's content hash, keyed by its route from the manifest's
    /// directory. Two manifests laid out alike with the same content agree
    /// wherever they are installed; any change to a member or import changes
    /// it.
    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// The globals a process other than this host declares while composing a
    /// document the host identifies: every root of the host's global catalog
    /// and lifecycle registry, each [`Availability::Unavailable`] with
    /// [`HOST_RUNTIME_ONLY_CODE`]. Analysis therefore knows the roots (a read
    /// of `seq` is no undeclared document property), and a runtime read fails
    /// as any unavailable global does; no value is ever manufactured.
    ///
    /// A root `md` owns (a baseline property, a Darkmatter global, a reserved
    /// namespace) or that an `md` event declares itself (`err`, `timing`) is
    /// left out, so registration never changes what such a name means.
    /// `None` when no root remains.
    #[must_use]
    pub fn analysis_globals(&self) -> Option<HostGlobals> {
        let md = crate::lifecycle::md_host::registry();
        let event_roots: BTreeSet<String> = crate::lifecycle::registry::CoreEvent::ALL
            .into_iter()
            .filter_map(|event| md.id(event.darkmatter_name()))
            .flat_map(|id| md.globals(id).globals().map(|global| global.root().to_string()).collect::<Vec<_>>())
            .collect();
        let mut roots: Vec<(String, String)> = Vec::new();
        let declared = self
            .globals
            .globals()
            .map(|global| (global.name().to_string(), global.description().unwrap_or_default().to_string()))
            .chain(self.lifecycle.registry().host_globals().iter().map(|root| (root.to_string(), String::new())));
        for (root, description) in declared {
            let owned = is_reserved_namespace(&root) || md_owned_names().contains(&root) || event_roots.contains(&root);
            if !owned && !roots.iter().any(|(seen, _)| *seen == root) {
                roots.push((root, description));
            }
        }
        if roots.is_empty() {
            return None;
        }
        let reason = UnavailabilityReason::new(HOST_RUNTIME_ONLY_CODE).with_parameter("host", json!(self.host));
        let mut view = BindingView::builder(ScopeId::new(format!("{}.analysis", self.host)));
        for (root, description) in &roots {
            view = view.declare(root.clone(), description.clone(), Availability::Unavailable(reason.clone()));
        }
        let view = Arc::new(view.build().ok()?);
        let runtime = roots.into_iter().map(|(root, _)| (root, RuntimeBinding::unavailable(reason.clone())));
        HostGlobals::associate(view, runtime).ok()
    }

    /// The [`HOST_FOREIGN_DOCUMENT_CODE`] notice for `document`, which this
    /// host identifies and `md compose` composes: it names the host, the
    /// events `md` runs (`shared_events`), and the host's own command.
    #[must_use]
    pub fn foreign_document_notice(&self, document: &Path, shared_events: &[&str]) -> RegistrationDiagnostic {
        let host = &self.host;
        let events: Vec<String> = shared_events.iter().map(|event| format!("`{event}`")).collect();
        let events = match events.as_slice() {
            [] => String::from("none of its lifecycle events"),
            [only] => format!("only its {only} event"),
            [rest @ .., last] => format!("only its {} and {last} events", rest.join(", ")),
        };
        RegistrationDiagnostic {
            code: HOST_FOREIGN_DOCUMENT_CODE,
            severity: Severity::Info,
            message: format!(
                "this is a `{host}` document: `md compose` checks it against `{host}`'s schemas, composes its body, \
                 and runs {events}; run `{host} compose` to run it as `{host}` does"
            ),
            file: document.to_path_buf(),
            line: None,
            column: None,
            entry: self.entries.first().cloned().unwrap_or_else(|| Registration {
                reference: self.manifest.path.display().to_string(),
                index: 0,
                declared_in: self.manifest.path.clone(),
                revision: super::load::ConfigRevision::User,
                at: None,
            }),
            member: None,
        }
    }
}

/// What [`load_registrations`] loaded.
#[derive(Debug, Default)]
pub struct LoadedRegistrations {
    /// One profile per distinct manifest that loaded, in entry order.
    pub profiles: Vec<Arc<RegisteredHostProfile>>,
    /// One or more warnings per refused entry, in entry order.
    pub diagnostics: Vec<RegistrationDiagnostic>,
}

/// Loads every valid `register` entry of `snapshot` (user entries first,
/// then the repository's). An invalid entry is already a finding of the
/// snapshot and is not reported again.
///
/// `context` is the request's captured context; each entry resolves in a
/// context derived from it and anchored at the file that declares it.
/// `owned` holds the top-level names Darkmatter owns, which an identifying
/// trigger may not rely on alone ([`md_owned_names`]).
#[must_use]
pub fn load_registrations(
    snapshot: &ConfigSnapshot,
    context: &FileResolutionContext,
    owned: &BTreeSet<String>,
) -> LoadedRegistrations {
    load_entries(&snapshot.registrations().entries, context, owned)
}

/// [`load_registrations`] over `entries`. Entries resolving to one canonical
/// manifest load it once; a refused manifest is reported once, for the first
/// entry naming it.
#[must_use]
pub fn load_entries(
    entries: &[Registration],
    context: &FileResolutionContext,
    owned: &BTreeSet<String>,
) -> LoadedRegistrations {
    buffers::with_reads_retained(|| {
        let mut manifests: Vec<(PathIdentity, Option<RegisteredHostProfile>)> = Vec::new();
        let mut diagnostics = Vec::new();
        for entry in entries {
            let path = match locate_manifest(entry, context) {
                Ok(path) => path,
                Err(refusal) => {
                    diagnostics.push(refusal.into_diagnostic(entry));
                    continue;
                }
            };
            let identity = PathIdentity::new(&path);
            if let Some((_, loaded)) = manifests.iter_mut().find(|(seen, _)| *seen == identity) {
                if let Some(profile) = loaded {
                    profile.entries.push(entry.clone());
                }
                continue;
            }
            match load_manifest(&path, context, owned) {
                Ok(mut profile) => {
                    profile.entries.push(entry.clone());
                    manifests.push((identity, Some(profile)));
                }
                Err(refusal) => {
                    diagnostics.push(refusal.into_diagnostic(entry));
                    manifests.push((identity, None));
                }
            }
        }
        let profiles = manifests.into_iter().filter_map(|(_, profile)| profile.map(Arc::new)).collect();
        LoadedRegistrations { profiles, diagnostics }
    })
}

/// The top-level names the `md` host owns: the resolved baseline's
/// properties, `md`'s lifecycle event keys (Darkmatter's own spellings,
/// `composed` and `failed` included), and Darkmatter's globals. Computed
/// once from those sources.
#[must_use]
pub fn md_owned_names() -> &'static BTreeSet<String> {
    static NAMES: OnceLock<BTreeSet<String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut names = BTreeSet::new();
        match GlobalCatalog::darkmatter().baseline() {
            Some((SchemaRoot::Single(shape), _)) => names.extend(shape.properties.keys().cloned()),
            Some((SchemaRoot::Union(arms), _)) => {
                for arm in arms {
                    if let simplified_schema::SchemaArm::Inline(shape) = arm {
                        names.extend(shape.properties.keys().cloned());
                    }
                }
            }
            None => {}
        }
        names.extend(crate::lifecycle::md_host::registry().authored_event_keys().into_iter().map(str::to_string));
        names.extend(crate::lifecycle::registry::CoreEvent::ALL.into_iter().map(|event| event.darkmatter_name().to_string()));
        names.extend(DARKMATTER_GLOBALS.iter().map(|global| global.name.to_string()));
        names
    })
}

// ── same-host agreement ─────────────────────────────────────────────────────

/// How the loaded profiles claiming one `host` agree.
#[derive(Debug, Clone)]
pub enum HostClaim {
    /// Every claim has the same [`RegisteredHostProfile::revision`]: the
    /// first stands for all.
    Agreed {
        /// The first claim, in entry order.
        profile: Arc<RegisteredHostProfile>,
        /// The later claims it collapsed.
        collapsed: Vec<Arc<RegisteredHostProfile>>,
    },
    /// Two or more revisions claim the host: none may win selection.
    Conflict {
        #[allow(missing_docs)]
        host: String,
        /// Every claim, in entry order.
        claims: Vec<Arc<RegisteredHostProfile>>,
    },
}

impl HostClaim {
    /// The claimed host.
    #[must_use]
    pub fn host(&self) -> &str {
        match self {
            Self::Agreed { profile, .. } => profile.host(),
            Self::Conflict { host, .. } => host,
        }
    }

    /// One [`HOST_REGISTRATION_CODE`] warning per conflicting claim, at its
    /// manifest, naming the others; none for an agreed claim.
    #[must_use]
    pub fn diagnostics(&self) -> Vec<RegistrationDiagnostic> {
        let Self::Conflict { host, claims } = self else {
            return Vec::new();
        };
        claims
            .iter()
            // A published profile always has the entry that loaded it.
            .filter_map(|claim| {
                let entry = claim.entries.first()?;
                let others: Vec<String> = claims
                    .iter()
                    .filter(|other| !Arc::ptr_eq(other, claim))
                    .map(|other| format!("`{}`", other.manifest.path.display()))
                    .collect();
                let refusal = Refusal {
                    code: HOST_REGISTRATION_CODE,
                    file: claim.manifest.path.clone(),
                    at: None,
                    member: None,
                    message: format!(
                        "claims host `{host}`, which {} also claims with different content; neither is used",
                        others.join(", ")
                    ),
                };
                Some(refusal.into_diagnostic(entry))
            })
            .collect()
    }
}

/// Groups `profiles` by host, in first-claim order: equal revisions
/// collapse into one [`HostClaim::Agreed`], different ones are a
/// [`HostClaim::Conflict`].
#[must_use]
pub fn agreement(profiles: &[Arc<RegisteredHostProfile>]) -> Vec<HostClaim> {
    let mut groups: Vec<Vec<Arc<RegisteredHostProfile>>> = Vec::new();
    for profile in profiles {
        match groups.iter_mut().find(|group| group[0].host == profile.host) {
            Some(group) => group.push(Arc::clone(profile)),
            None => groups.push(vec![Arc::clone(profile)]),
        }
    }
    groups
        .into_iter()
        .map(|mut group| {
            if group.iter().all(|claim| claim.revision == group[0].revision) {
                let profile = group.remove(0);
                HostClaim::Agreed { profile, collapsed: group }
            } else {
                HostClaim::Conflict { host: group[0].host.clone(), claims: group }
            }
        })
        .collect()
}

// ── loading ─────────────────────────────────────────────────────────────────

/// Why one entry was refused, before it is attached to the entry.
struct Refusal {
    code: &'static str,
    file: PathBuf,
    at: Option<(usize, usize)>,
    member: Option<String>,
    message: String,
}

impl Refusal {
    fn into_diagnostic(self, entry: &Registration) -> RegistrationDiagnostic {
        let member = self.member.as_ref().map_or_else(String::new, |member| format!(" at `{member}`"));
        RegistrationDiagnostic {
            code: self.code,
            severity: Severity::Warning,
            message: format!(
                "host registration `{}` (entry {} of `register` in `{}`) is not loaded{member}: {}",
                entry.reference,
                entry.index,
                entry.declared_in.display(),
                self.message
            ),
            file: self.file,
            line: self.at.map(|(line, _)| line),
            column: self.at.map(|(_, column)| column),
            entry: entry.clone(),
            member: self.member,
        }
    }
}

/// A context anchored at `file`, for the references it holds. Registration
/// is passive and grants nothing, so a member may sit outside the tree.
fn anchored_at(context: &FileResolutionContext, file: &Path) -> FileResolutionContext {
    context.for_trusted_external_source(file).allow_external_relative()
}

/// Every location the members of the manifest at `path` are probed at,
/// whether or not a file is there now: what a caller watching a refused
/// registration (a member missing, unreadable, or invalid) must observe so
/// the member's creation or repair loads it. Reads only the manifest; empty
/// when the manifest cannot be read or is not a valid manifest (its own
/// refusal names it).
#[must_use]
pub fn manifest_member_candidates(path: &Path, context: &FileResolutionContext) -> Vec<PathBuf> {
    let Ok(bytes) = buffers::read(path) else {
        return Vec::new();
    };
    let Ok(read) = manifest::read(path, bytes) else {
        return Vec::new();
    };
    let anchor = anchored_at(context, path);
    let manifest = &read.manifest;
    [&manifest.identify, &manifest.globals, &manifest.lifecycle]
        .into_iter()
        .chain(&manifest.triggers)
        .chain(manifest.functions.iter().flat_map(|pair| [&pair.catalog, &pair.facts]))
        .filter_map(|reference| FileReference::new(reference).ok())
        .flat_map(|reference| reference.resolve_detailed(&anchor).probed_paths().collect::<Vec<_>>())
        .collect()
}

/// The canonical path of the manifest `entry` names.
fn locate_manifest(entry: &Registration, context: &FileResolutionContext) -> Result<PathBuf, Refusal> {
    let refuse = |message: String| Refusal {
        code: HOST_REGISTRATION_CODE,
        file: entry.declared_in.clone(),
        at: entry.at,
        member: None,
        message,
    };
    let path = resolve_local(&entry.reference, &anchored_at(context, &entry.declared_in)).map_err(refuse)?;
    canonicalize_simplified(&path).map_err(|error| refuse(format!("the manifest `{}` cannot be resolved: {error}", path.display())))
}

/// The identity of the existing manifest the `register` entry `reference`
/// names when written in `declared_in`: the canonical file the loader would
/// open for it. Two entries naming one manifest (`~/…` and an absolute path,
/// or entries in the user and repository files) share an identity and load
/// once.
///
/// ## Errors
///
/// Why the entry names no existing local file: a malformed or remote
/// reference, or a missing file.
pub fn manifest_identity(
    reference: &str,
    declared_in: &Path,
    context: &FileResolutionContext,
) -> Result<PathIdentity, String> {
    let path = resolve_local(reference, &anchored_at(context, declared_in))?;
    let canonical = canonicalize_simplified(&path)
        .map_err(|error| format!("the manifest `{}` cannot be resolved: {error}", path.display()))?;
    Ok(PathIdentity::new(&canonical))
}

/// The existing local file `reference` names in `context`.
fn resolve_local(reference: &str, context: &FileResolutionContext) -> Result<PathBuf, String> {
    let parsed = FileReference::new(reference).map_err(|error| format!("`{reference}` is not a file reference: {error}"))?;
    if parsed.class().kind == FileReferenceKind::Url {
        return Err(format!("`{reference}` is remote; registration reads only local files and fetches nothing"));
    }
    match parsed.resolve_in_context(context) {
        Ok(Some(path)) => Ok(path),
        Ok(None) => Err(format!("`{reference}` names no existing file")),
        Err(error) => Err(format!("`{reference}` cannot be resolved: {error}")),
    }
}

/// Everything a manifest names, loaded, before its reads are recorded.
struct Members {
    read: manifest::ReadManifest,
    identify: RegisteredTrigger,
    identifying: IdentifyingTrigger,
    triggers: Vec<RegisteredTrigger>,
    globals_path: PathBuf,
    globals: Arc<GlobalCatalog>,
    lifecycle_path: PathBuf,
    lifecycle: Arc<HostLifecycleProfile>,
    functions: Vec<HostFunctionSet>,
}

/// The profile of the manifest at `path`, or the first reason it cannot be
/// one.
fn load_manifest(
    path: &Path,
    context: &FileResolutionContext,
    owned: &BTreeSet<String>,
) -> Result<RegisteredHostProfile, Refusal> {
    let (members, reads) = buffers::recording(|| load_members(path, context, owned));
    let members = members?;
    let dependencies: Vec<HostDependency> = reads
        .iter()
        .map(|(path, text)| HostDependency { path: path.clone(), content_hash: content_hash(text.as_bytes()) })
        .collect();
    let Some(manifest_hash) = dependencies
        .iter()
        .find(|dependency| dependency.path == path)
        .map(|dependency| dependency.content_hash.clone())
    else {
        return Err(Refusal {
            code: HOST_REGISTRATION_CODE,
            file: path.to_path_buf(),
            at: None,
            member: None,
            message: "the manifest's read was not recorded".to_string(),
        });
    };
    let revision = revision(members.read.meaning(), path, &dependencies);
    let Members { read, identify, identifying, triggers, globals_path, globals, lifecycle_path, lifecycle, functions } =
        members;
    Ok(RegisteredHostProfile {
        host: read.manifest.host,
        version: read.manifest.version,
        manifest: ManifestIdentity { path: path.to_path_buf(), content_hash: manifest_hash },
        entries: Vec::new(),
        identify,
        identifying,
        triggers,
        globals_path,
        globals,
        lifecycle_path,
        lifecycle,
        functions,
        dependencies,
        revision,
    })
}

/// See [`RegisteredHostProfile::revision`].
fn revision(meaning: &Value, manifest: &Path, dependencies: &[HostDependency]) -> String {
    let directory = PathIdentity::new(manifest.parent().unwrap_or(manifest));
    let mut members: Vec<Value> = dependencies
        .iter()
        .filter(|dependency| dependency.path != manifest)
        .map(|dependency| {
            let route = match PathIdentity::new(&dependency.path).relative_from(&directory) {
                Some(route) => json!({
                    "up": route.parent_hops(),
                    "names": route.forward().iter().map(|name| name.to_string_lossy().into_owned()).collect::<Vec<_>>(),
                }),
                None => json!({ "absolute": biscuit_file::to_portable_string(&dependency.path) }),
            };
            json!({ "route": route, "content_hash": dependency.content_hash })
        })
        .collect();
    members.sort_by_cached_key(|member| member.to_string());
    biscuit_hash::blake3_hash_json(&json!({ "manifest": meaning, "dependencies": members }))
}

fn load_members(path: &Path, context: &FileResolutionContext, owned: &BTreeSet<String>) -> Result<Members, Refusal> {
    let bytes = buffers::read(path).map_err(|error| Refusal {
        code: HOST_REGISTRATION_CODE,
        file: path.to_path_buf(),
        at: None,
        member: None,
        message: format!("the manifest cannot be read: {error}"),
    })?;
    let read = manifest::read(path, bytes)?;
    let anchor = anchored_at(context, path);
    let manifest = &read.manifest;

    let member = |segments: &[&str], reference: &str| -> Result<PathBuf, Refusal> {
        resolve_local(reference, &anchor).map_err(|message| Refusal {
            code: HOST_REGISTRATION_CODE,
            file: path.to_path_buf(),
            at: read.locate(segments),
            member: Some(field(segments)),
            message,
        })
    };

    let identify_path = member(&["identify"], &manifest.identify)?;
    let (identify, identifying) = load_identify(&identify_path, context, owned)?;

    let mut triggers = Vec::with_capacity(manifest.triggers.len());
    for (index, reference) in manifest.triggers.iter().enumerate() {
        let index = index.to_string();
        let segments = ["triggers", index.as_str()];
        let trigger_path = member(&segments, reference)?;
        let (trigger, _) = load_trigger(&trigger_path, context, &field(&segments))?;
        triggers.push(trigger);
    }

    let globals_path = member(&["globals"], &manifest.globals)?;
    let refuse_at = |file: &Path, member: &str, message: String| Refusal {
        code: HOST_REGISTRATION_CODE,
        file: file.to_path_buf(),
        at: None,
        member: Some(member.to_string()),
        message,
    };
    let globals_text = buffers::read_to_string(&globals_path)
        .map_err(|error| refuse_at(&globals_path, "globals", format!("the global catalog cannot be read: {error}")))?;
    let globals = GlobalCatalog::from_host_text(&globals_path, &globals_text, &anchored_at(context, &globals_path))
        .map_err(|error| refuse_at(&globals_path, "globals", error.to_string()))?;
    let globals = Arc::new(globals);

    let lifecycle_path = member(&["lifecycle"], &manifest.lifecycle)?;
    let lifecycle_read = buffers::read(&lifecycle_path);
    let lifecycle_text = companion(&lifecycle_read);
    let lifecycle =
        HostLifecycleProfile::from_artifact_at(Arc::clone(&globals), &globals_text, &lifecycle_path, lifecycle_text.artifact())
            .map_err(|reason| refuse_at(&lifecycle_path, "lifecycle", reason))?;
    let recorded = lifecycle.registry().host();
    if recorded != manifest.host {
        return Err(Refusal {
            code: HOST_REGISTRATION_CODE,
            file: path.to_path_buf(),
            at: read.locate(&["host"]),
            member: Some("host".to_string()),
            message: format!(
                "the manifest names host `{}`, but its lifecycle artifact `{}` records host `{recorded}`",
                manifest.host,
                lifecycle_path.display()
            ),
        });
    }

    let mut functions = Vec::with_capacity(manifest.functions.len());
    for (index, pair) in manifest.functions.iter().enumerate() {
        let index = index.to_string();
        let catalog_field = ["functions", index.as_str(), "catalog"];
        let facts_field = ["functions", index.as_str(), "facts"];
        let catalog = member(&catalog_field, &pair.catalog)?;
        let facts = member(&facts_field, &pair.facts)?;
        let text = buffers::read_to_string(&catalog).map_err(|error| {
            refuse_at(&catalog, &field(&catalog_field), format!("the function catalog cannot be read: {error}"))
        })?;
        let set = host_function_set_at(&catalog, &text, &facts, buffers::read(&facts))
            .map_err(|error| refuse_at(&catalog, &field(&catalog_field), error.message))?;
        let incomplete = set.incomplete_functions();
        if !incomplete.is_empty() {
            let reasons: Vec<String> =
                incomplete.iter().map(|(name, reason)| format!("`{name}`: {reason}")).collect();
            return Err(refuse_at(&facts, &field(&facts_field), format!("its functions are incomplete: {}", reasons.join("; "))));
        }
        functions.push(HostFunctionSet { catalog, facts, functions: set });
    }

    Ok(Members {
        identify,
        identifying,
        triggers,
        globals_path,
        globals,
        lifecycle_path,
        lifecycle: Arc::new(lifecycle),
        functions,
        read,
    })
}

/// A companion artifact's read as the profile readers receive it.
enum Companion {
    Text(String),
    Unreadable(String),
}

impl Companion {
    fn artifact(&self) -> FactsArtifact<'_> {
        match self {
            Self::Text(text) => FactsArtifact::Text(text),
            Self::Unreadable(error) => FactsArtifact::Unreadable(error),
        }
    }
}

fn companion(read: &std::io::Result<Vec<u8>>) -> Companion {
    match read {
        Err(error) => Companion::Unreadable(error.to_string()),
        Ok(bytes) => match std::str::from_utf8(bytes) {
            Ok(text) => Companion::Text(text.to_string()),
            Err(_) => Companion::Unreadable("the file is not UTF-8 text".to_string()),
        },
    }
}

/// `identify[...]` written from segments: `triggers[1]`,
/// `functions[0].facts`.
fn field(segments: &[&str]) -> String {
    let mut text = String::new();
    for segment in segments {
        if segment.bytes().all(|byte| byte.is_ascii_digit()) && !segment.is_empty() {
            text.push_str(&format!("[{segment}]"));
        } else {
            if !text.is_empty() {
                text.push('.');
            }
            text.push_str(segment);
        }
    }
    text
}

/// The identifying trigger at `path`, admitted against `owned`.
fn load_identify(
    path: &Path,
    context: &FileResolutionContext,
    owned: &BTreeSet<String>,
) -> Result<(RegisteredTrigger, IdentifyingTrigger), Refusal> {
    let (trigger, text) = load_trigger(path, context, "identify")?;
    let identifying = admit_identifying_trigger(&trigger.trigger.envelope.conditions, owned).map_err(|refusal| {
        // Only the position: the envelope already parsed this text.
        let at = simplified_schema::yaml_documents::load_yaml_document(&text, path).ok().and_then(|loaded| {
            loaded
                .byte_range(&simplified_schema::yaml_documents::locating_problem(&["match".to_string()]))
                .map(|range| line_column(&text, range.start))
        });
        Refusal {
            code: HOST_IDENTIFY_TOO_BROAD_CODE,
            file: path.to_path_buf(),
            at,
            member: Some("identify".to_string()),
            message: format!("its identifying trigger is too broad: {refusal}"),
        }
    })?;
    Ok((trigger, identifying))
}

/// The schema trigger at `path` with its payload resolved from its own
/// folder, and the text read.
fn load_trigger(path: &Path, context: &FileResolutionContext, member: &str) -> Result<(RegisteredTrigger, Arc<str>), Refusal> {
    let refuse = |message: String| Refusal {
        code: HOST_REGISTRATION_CODE,
        file: path.to_path_buf(),
        at: None,
        member: Some(member.to_string()),
        message,
    };
    let text: Arc<str> = buffers::read_to_string(path)
        .map_err(|error| refuse(format!("the trigger cannot be read: {error}")))?
        .into();
    let envelope: TriggerEnvelope = match parse_trigger_envelope_from_str(&text) {
        Ok(Some(envelope)) => envelope,
        Ok(None) => return Err(refuse("the file is not a schema trigger (`kind: schema-trigger`)".to_string())),
        Err(error) => return Err(refuse(format!("the trigger is invalid: {error}"))),
    };
    let anchor = anchored_at(context, path);
    let trigger = LoadedTrigger::from_file(path.to_path_buf(), Arc::clone(&text), envelope, context.base_dir().to_path_buf());
    // A registered trigger is its own source: its payload resolves from its
    // file, never through the document's schema roots.
    let payload = resolve_trigger_payload(&trigger, &[], &anchor, crate::markdown::schemas::embedded::SHIPPED_SOURCES)
        .map_err(|error| refuse(format!("the trigger's payload does not load: {error}")))?;
    Ok((RegisteredTrigger { trigger, payload }, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_name_list_items_by_index() {
        assert_eq!(field(&["identify"]), "identify");
        assert_eq!(field(&["triggers", "1"]), "triggers[1]");
        assert_eq!(field(&["functions", "0", "facts"]), "functions[0].facts");
    }

    #[test]
    fn a_profile_is_shareable_across_threads() {
        fn shareable<T: Send + Sync>() {}
        shareable::<RegisteredHostProfile>();
        shareable::<LoadedRegistrations>();
    }
}
