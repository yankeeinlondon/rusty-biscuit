//! Host registration: loading the manifests that `register` entries name
//! into passive host profiles.
//!
//! A `register` entry of `.darkmatter.yaml` names a **host manifest**
//! (`kind: darkmatter-host`, format 3, [`manifest`]), one file a host program
//! publishes that lists every file of its bundle with its BLAKE3 digest:
//! schema triggers, YAML kind definitions, namespace schemas, type
//! libraries, and expression catalogs under `files`, and its lifecycle
//! artifact; each expression catalog's facts companion is derived from the
//! catalog's name and digested too. [`load_registrations`] turns each entry
//! into a [`RegisteredHostProfile`], or into warnings when anything about it
//! is unusable:
//!
//! - [`bundle::load_snapshot`] checks the manifest's format version and
//!   validates it against the shipped closed envelope, then reads every
//!   member once and checks its digest; a manifest of any other format is
//!   refused with an update-and-republish warning;
//! - every member is classified by its own `kind` and prepared from the
//!   verified bytes alone ([`simplified_schema::resolve::bundle`]): an import
//!   outside the listed members, an escape, or a remote reference is refused;
//! - the namespace files form the host's global catalog ([`namespaces`]),
//!   which the lifecycle artifact must be versioned against;
//! - each expression catalog loads with the authored catalog reader and its
//!   verified companion into an analysis-only function set; every function
//!   must be complete, and no canonical name or alias may be claimed by two
//!   of the host's catalogs (a `kind: mutation-schema` file is refused:
//!   hosts define no actions);
//! - a profile is published only when all of it loads: no partial globals,
//!   functions, lifecycle events, trigger payloads, or YAML kinds are ever
//!   exposed.
//!
//! Registration is passive: it writes nothing, fetches nothing, prompts for
//! nothing, installs no function handler, and runs no command. A profile is
//! analysis data, never runtime authority: its function sets are entered
//! only for a document selected for its host, and a call that reaches
//! evaluation fails because only the host process holds the handler.
//!
//! Which host a document belongs to is [`select`]ed over [`RegisteredHosts`]
//! (built on [`agreement`]): a host one of whose triggers matches the
//! document's authored frontmatter and path; the selected host's matched
//! triggers layer into the effective schema through
//! [`DarkmatterSchemas::with_registered_hosts`](crate::markdown::schemas::DarkmatterSchemas::with_registered_hosts).
//!
//! User-facing guide: `darkmatter/docs/topics/host-registration.md`.

// A refusal is a located message plus its related locations, built once
// per refused member and never on a hot path; boxing every `Err` would only
// add noise at each construction site.
#![allow(clippy::result_large_err)]

pub mod bundle;
mod locate;
pub mod manifest;
pub mod merge;
pub mod namespaces;
mod selection;

pub use selection::{HostSelection, RegisteredHosts, Selected, select};

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use biscuit_file::{FileReference, FileReferenceKind, FileResolutionContext, PathIdentity, canonicalize_simplified};
use serde_json::json;
use simplified_schema::SchemaRoot;
use simplified_schema::member_envelope::{MemberKind, admit_member_text};
use simplified_schema::resolve::bundle::{self as resolve_bundle, BundleSources};
use simplified_schema::triggers::assemble::{ResolvedPayload, resolve_registered_trigger_payload};
use simplified_schema::triggers::{
    IdentifyingTrigger, LoadedTrigger, TriggerEnvelope, admit_identifying_trigger, parse_trigger_envelope_from_str,
};
use simplified_schema::yaml_documents::{RegisteredDefinition, RegisteredSource};

use self::bundle::{BundleCompanion, BundleMember, VerifiedSnapshot};
use self::namespaces::{NamespaceMember, PreparedCatalog};
use super::load::{ConfigSnapshot, Registration, SourcePosition, line_column};
use crate::lifecycle::host_profile::{self, HostLifecycleProfile};
use crate::markdown::compose::expression::host::FactsArtifact;
use crate::markdown::compose::expression::host::artifact::analysis_only_sited;
use crate::markdown::compose::expression::{
    Availability, BindingView, HostFunctionCatalog, HostFunctions, HostGlobals, RuntimeBinding, ScopeId, Severity,
    UnavailabilityReason, is_reserved_namespace,
};
use crate::markdown::schemas::catalog::{DARKMATTER_GLOBALS, GlobalCatalog};

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
    /// The line's text before [`Self::column`], when [`Self::line`] is
    /// known: an editor counts the column in its own position encoding from
    /// it (see [`SourcePosition::line_prefix`]).
    pub line_prefix: Option<String>,
    /// The `register` entry refused.
    pub entry: Registration,
    /// The manifest field of the failing member (`lifecycle`, `files[1]`;
    /// a facts companion's is its catalog's `files` entry); `None` when the
    /// manifest itself failed.
    pub member: Option<String>,
    /// Other locations the problem involves: the earlier declaration a
    /// conflicting name or alias collides with, or the built-in a catalog
    /// name reuses.
    pub related: Vec<RegistrationRelated>,
}

/// A secondary location of a [`RegistrationDiagnostic`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationRelated {
    /// What the location is, as an editor labels it.
    pub message: String,
    #[allow(missing_docs)]
    pub file: PathBuf,
    /// 1-based line in [`Self::file`], when known.
    pub line: Option<usize>,
    /// 1-based character column, when [`Self::line`] is known.
    pub column: Option<usize>,
    /// The line's text before [`Self::column`], when [`Self::line`] is
    /// known.
    pub line_prefix: Option<String>,
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

/// A registered trigger file: its loaded envelope, resolved payload, and
/// admitted identifying conditions.
///
/// The trigger's payload resolves from its own file inside the bundle, so
/// relative imports (`./action.yaml`) are anchored there. Its `pattern_cwd`
/// is the tree root of the context the registration was loaded in; a caller
/// layering it for a document in another tree sets that document's tree
/// root, as for a user-level schema root.
#[derive(Debug, Clone)]
pub struct RegisteredTrigger {
    /// The loaded trigger, ready for a
    /// [`TriggerRegistry`](simplified_schema::triggers::TriggerRegistry).
    pub trigger: LoadedTrigger,
    /// Its payload, resolved and checked at load; empty for a matcher-only
    /// trigger, which identifies the host and contributes nothing.
    pub payload: ResolvedPayload,
    /// Its bundle-relative path (`schemas/composition.yaml`): its identity
    /// wherever the bundle is installed.
    pub bundle_path: String,
    /// Where its text came from: the installed file, or for an embedded
    /// bundle ([`load_embedded`]) the virtual `<name>/schemas/…` path, which
    /// names no host file. Diagnostics and inspection name it.
    pub location: PathBuf,
    /// Its conditions as admitted against the names `md` owns: every
    /// matching alternative requires a key no `md` document has.
    pub identifying: IdentifyingTrigger,
}

impl RegisteredTrigger {
    /// The installed trigger file.
    ///
    /// ## Panics
    ///
    /// For a trigger of an embedded bundle ([`load_embedded`]), whose source
    /// is [`TriggerSource::Embedded`](simplified_schema::triggers::TriggerSource::Embedded)
    /// and names no host file. Code that may meet one reads
    /// [`Self::location`] or the trigger's source instead.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.trigger.source.path().expect("an installed registered trigger is a file")
    }

    /// Whether this trigger came from an embedded bundle.
    #[must_use]
    pub const fn is_embedded(&self) -> bool {
        matches!(self.trigger.source, simplified_schema::triggers::TriggerSource::Embedded { .. })
    }
}

/// One `kind: expression-schema` member: the analysis-only set its catalog
/// and derived facts companion give.
#[derive(Debug, Clone)]
pub struct HostFunctionSet {
    /// The catalog's bundle-relative path (`schemas/project.yaml`).
    pub bundle_path: String,
    /// The function catalog file.
    pub catalog: PathBuf,
    /// The facts companion file.
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
    snapshot: Arc<VerifiedSnapshot>,
    triggers: Vec<RegisteredTrigger>,
    kinds: Vec<RegisteredDefinition>,
    libraries: Vec<String>,
    catalog: PreparedCatalog,
    lifecycle_path: PathBuf,
    lifecycle: Arc<HostLifecycleProfile>,
    functions: Vec<HostFunctionSet>,
    candidates: Vec<PathBuf>,
    /// The checked host layer of each matched set seen so far: the profile
    /// is one revision, so the sorted matched indices are the whole key.
    checked: std::sync::Mutex<Vec<(Vec<usize>, CheckedResult)>>,
}

/// A memoized [`merge::check`] outcome.
type CheckedResult = Result<Arc<merge::CheckedLayer>, merge::TriggerConflict>;

#[allow(missing_docs)]
impl RegisteredHostProfile {
    /// The checked host layer of the triggers at `matched` (indices into
    /// [`Self::triggers`], bundle-path order) that carry a payload, built
    /// once per matched set. Every such payload must be concrete: a caller
    /// with a template payload resolves it and calls [`merge::check`]
    /// itself, uncached, since the result depends on the resolved values.
    ///
    /// ## Errors
    ///
    /// The set's [`merge::TriggerConflict`], memoized like a success.
    pub fn checked_layer(&self, matched: &[usize]) -> CheckedResult {
        let mut key: Vec<usize> = matched.to_vec();
        key.sort_unstable();
        key.dedup();
        if let Some((_, result)) = self.checked.lock().expect("checked layers poisoned").iter().find(|(seen, _)| *seen == key) {
            return result.clone();
        }
        let payloads: Vec<merge::MatchedPayload<'_>> = key
            .iter()
            .filter_map(|&index| Some((index, self.triggers.get(index)?)))
            .filter(|(_, trigger)| trigger.trigger.envelope.payload.is_some())
            .map(|(index, trigger)| merge::MatchedPayload { trigger: index, file: &trigger.location, payload: &trigger.payload })
            .collect();
        let result = merge::check(&payloads).map(Arc::new);
        self.checked.lock().expect("checked layers poisoned").push((key, result.clone()));
        result
    }

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

    /// The verified snapshot the profile was prepared from.
    #[must_use]
    pub const fn snapshot(&self) -> &Arc<VerifiedSnapshot> {
        &self.snapshot
    }

    /// The `files` that are schema triggers, in bundle-path order (the
    /// order of `files` has no effect).
    #[must_use]
    pub fn triggers(&self) -> &[RegisteredTrigger] {
        &self.triggers
    }

    /// The `files` that are `kind: yaml-schema` definitions, prepared, in
    /// bundle-path order.
    #[must_use]
    pub fn yaml_kinds(&self) -> &[RegisteredDefinition] {
        &self.kinds
    }

    /// The bundle-relative paths of the `files` that are types-only
    /// libraries, sorted.
    #[must_use]
    pub fn libraries(&self) -> &[String] {
        &self.libraries
    }

    /// The host's global catalog: one global per namespace file.
    #[must_use]
    pub const fn globals(&self) -> &Arc<GlobalCatalog> {
        &self.catalog.catalog
    }

    /// The namespace files the catalog was prepared from, in bundle-path
    /// order: where each global is declared.
    #[must_use]
    pub fn namespaces(&self) -> &[NamespaceMember] {
        &self.catalog.namespaces
    }

    /// The namespace file that declares global `name`.
    #[must_use]
    pub fn namespace(&self, name: &str) -> Option<&NamespaceMember> {
        self.catalog.namespaces.iter().find(|member| member.namespace == name)
    }

    /// The `catalog_hash` the lifecycle artifact was checked against
    /// ([`namespaces::namespace_catalog_hash`]).
    #[must_use]
    pub fn catalog_hash(&self) -> &str {
        &self.catalog.hash
    }

    #[must_use]
    pub fn lifecycle_path(&self) -> &Path {
        &self.lifecycle_path
    }

    #[must_use]
    pub const fn lifecycle(&self) -> &Arc<HostLifecycleProfile> {
        &self.lifecycle
    }

    /// The expression catalogs' analysis-only sets, in bundle-path order.
    /// No two share a canonical name or alias (case-insensitively).
    #[must_use]
    pub fn functions(&self) -> &[HostFunctionSet] {
        &self.functions
    }

    /// Every file the load read (the manifest and each member), once each,
    /// in read order, with the hash of the bytes read.
    #[must_use]
    pub fn dependencies(&self) -> &[HostDependency] {
        self.snapshot.dependencies()
    }

    /// Every location the load looked at, present or not: the manifest, each
    /// member, and each unlisted path a member's reference resolved to. What
    /// a watcher observes so a republish is noticed.
    #[must_use]
    pub fn candidates(&self) -> &[PathBuf] {
        &self.candidates
    }

    /// The content revision ([`VerifiedSnapshot::revision`]): the same for
    /// two installs of one release wherever they are, and independent of the
    /// order of `files`; any member byte changes it.
    #[must_use]
    pub fn revision(&self) -> &str {
        self.snapshot.revision()
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
            .globals()
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
            line_prefix: None,
            entry: self.entries.first().cloned().unwrap_or_else(|| Registration {
                reference: self.manifest.path.display().to_string(),
                index: 0,
                declared_in: self.manifest.path.clone(),
                revision: super::load::ConfigRevision::User,
                at: None,
            }),
            member: None,
            related: Vec::new(),
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
    /// Every location a refused entry's load read or looked at, present or
    /// not (its manifest, every member of every attempt, and each unlisted
    /// path a member's reference resolved to): what a watcher observes so
    /// the creation or repair of any of them loads the host.
    pub watches: Vec<PathBuf>,
}

/// Loads every valid `register` entry of `snapshot` (user entries first,
/// then the repository's). An invalid entry is already a finding of the
/// snapshot and is not reported again.
///
/// `context` is the request's captured context; each entry resolves in a
/// context derived from it and anchored at the file that declares it.
/// `owned` holds the top-level names Darkmatter owns, which no registered
/// trigger may rely on alone ([`md_owned_names`]).
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
    let mut manifests: Vec<(PathIdentity, Option<RegisteredHostProfile>)> = Vec::new();
    let mut loaded = LoadedRegistrations::default();
    for entry in entries {
        let path = match locate_manifest(entry, context) {
            Ok(path) => path,
            Err(refusal) => {
                loaded.diagnostics.push(refusal.into_diagnostic(entry));
                continue;
            }
        };
        let identity = PathIdentity::new(&path);
        if let Some((_, profile)) = manifests.iter_mut().find(|(seen, _)| *seen == identity) {
            if let Some(profile) = profile {
                profile.entries.push(entry.clone());
            }
            continue;
        }
        match load_manifest(&path, context, owned) {
            Ok(mut profile) => {
                profile.entries.push(entry.clone());
                manifests.push((identity, Some(profile)));
            }
            Err(refused) => {
                loaded.diagnostics.extend(refused.refusals.into_iter().map(|refusal| refusal.into_diagnostic(entry)));
                for watch in refused.watches {
                    if !loaded.watches.contains(&watch) {
                        loaded.watches.push(watch);
                    }
                }
                manifests.push((identity, None));
            }
        }
    }
    loaded.profiles = manifests.into_iter().filter_map(|(_, profile)| profile.map(Arc::new)).collect();
    loaded
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
    /// manifest's `host`, naming the others; none for an agreed claim. The
    /// text scopes the conflict as selection does: no claim's triggers
    /// apply to Markdown, while [`RegisteredHosts`] still indexes every
    /// claim's YAML kinds.
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
                    at: claim.snapshot.host_position().cloned(),
                    member: None,
                    related: Vec::new(),
                    message: format!(
                        "claims host `{host}`, which {} also claims with different content; neither's triggers apply to Markdown documents, while each one's YAML kinds stay available and a kind both define differently cannot be selected",
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
            if group.iter().all(|claim| claim.revision() == group[0].revision()) {
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
    at: Option<SourcePosition>,
    member: Option<String>,
    message: String,
    related: Vec<RegistrationRelated>,
}

impl Refusal {
    fn into_diagnostic(self, entry: &Registration) -> RegistrationDiagnostic {
        Labeled { refusal: self, label: None }.into_diagnostic(entry)
    }
}

/// A refusal and the display label of the member it is reported under
/// ([`member_label`]), once [`related`] attributed it.
struct Labeled {
    refusal: Refusal,
    label: Option<String>,
}

impl Labeled {
    fn into_diagnostic(self, entry: &Registration) -> RegistrationDiagnostic {
        let Self { refusal, label } = self;
        let member = match (&refusal.member, &label) {
            (Some(member), Some(label)) => format!(" at `{member}` (`{label}`)"),
            (Some(member), None) => format!(" at `{member}`"),
            (None, _) => String::new(),
        };
        RegistrationDiagnostic {
            code: refusal.code,
            severity: Severity::Warning,
            message: format!(
                "host registration `{}` (entry {} of `register` in `{}`) is not loaded{member}: {}",
                entry.reference,
                // `index` is the list offset; a person counts from 1.
                entry.index + 1,
                entry.declared_in.display(),
                refusal.message
            ),
            file: refusal.file,
            line: refusal.at.as_ref().map(|at| at.line),
            column: refusal.at.as_ref().map(|at| at.column),
            line_prefix: refusal.at.map(|at| at.line_prefix),
            entry: entry.clone(),
            member: refusal.member,
            related: refusal.related,
        }
    }
}

/// A context anchored at `file`, for the references it holds. Registration
/// is passive and grants nothing, so a member may sit outside the tree.
fn anchored_at(context: &FileResolutionContext, file: &Path) -> FileResolutionContext {
    context.for_trusted_external_source(file).allow_external_relative()
}

/// The canonical path of the manifest `entry` names.
fn locate_manifest(entry: &Registration, context: &FileResolutionContext) -> Result<PathBuf, Refusal> {
    let refuse = |message: String| Refusal {
        code: HOST_REGISTRATION_CODE,
        file: entry.declared_in.clone(),
        at: entry.at.clone(),
        member: None,
        message,
        related: Vec::new(),
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

/// [`manifest_identity`] for an entry whose manifest may be missing: the
/// identity of the file the entry would open, from the first location its
/// resolution probes in the context captured for `declared_in`. `None` for a
/// malformed or remote reference, or one that names no location.
pub(crate) fn manifest_location_identity(
    reference: &str,
    declared_in: &Path,
    context: &FileResolutionContext,
) -> Option<PathIdentity> {
    let parsed = FileReference::new(reference).ok()?;
    if parsed.class().kind == FileReferenceKind::Url {
        return None;
    }
    let detailed = parsed.resolve_detailed(&anchored_at(context, declared_in));
    let location = match detailed.matched_path() {
        Some(path) => path.to_path_buf(),
        None => detailed.probed_paths().next()?,
    };
    Some(location_identity(&location))
}

/// Whether the `register` entry `reference`, written in `declared_in`, names
/// the file at `location`, whether or not either exists: the entry is
/// compared by the location it would open, resolved from `declared_in`'s
/// folder in `context`, so every spelling of the same file matches. This is the rule
/// [`replace_manifest_registration`](super::init::replace_manifest_registration)
/// uses to find the entries of a host's previous manifest, so a host can
/// tell which entries that writer would remove.
#[must_use]
pub fn entry_names_location(reference: &str, declared_in: &Path, context: &FileResolutionContext, location: &Path) -> bool {
    manifest_location_identity(reference, declared_in, context).is_some_and(|identity| identity == location_identity(location))
}

/// The identity of `path` whether or not it exists: its nearest existing
/// ancestor canonicalized, with the missing rest appended, so a missing file
/// under a linked folder compares equal to its other spellings. Equals
/// [`manifest_identity`]'s for an existing file.
pub(crate) fn location_identity(path: &Path) -> PathIdentity {
    let mut missing = Vec::new();
    let mut existing = path;
    loop {
        if let Ok(canonical) = canonicalize_simplified(existing) {
            let full = missing.iter().rev().fold(canonical, |path: PathBuf, name| path.join(name));
            return PathIdentity::new(&full);
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name.to_os_string());
                existing = parent;
            }
            _ => return PathIdentity::new(path),
        }
    }
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

/// Why a manifest published no profile: every refusal, and what a watcher
/// observes to notice its repair.
struct Refused {
    refusals: Vec<Labeled>,
    watches: Vec<PathBuf>,
}

impl Refused {
    fn one(refusal: Refusal, watches: Vec<PathBuf>) -> Self {
        Self { refusals: vec![Labeled { refusal, label: None }], watches }
    }
}

/// A manifest field written from segments: `files[1]`, `digests.<key>`.
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

/// The profile of the manifest at `path`, or every reason it cannot be one.
fn load_manifest(path: &Path, context: &FileResolutionContext, owned: &BTreeSet<String>) -> Result<RegisteredHostProfile, Refused> {
    let snapshot = bundle::load_snapshot(path, context).map_err(|refusal| {
        let mut watches = refusal.candidates.clone();
        watches.extend(refusal.dependencies.iter().map(|dependency| dependency.path.clone()));
        let refusal = Refusal {
            code: refusal.code,
            at: refusal.position(),
            file: refusal.file,
            member: refusal.member,
            message: refusal.message,
            related: Vec::new(),
        };
        Refused::one(refusal, dedup(watches))
    })?;
    profile_of(snapshot, context, owned)
}

/// The profile of the installed bundle `snapshot`, prepared in `context`
/// exactly as [`load_registrations`] prepares a `register` entry's verified
/// snapshot: every member classified and prepared, every declared type
/// checked, and the catalog and every artifact (the lifecycle profile, each
/// expression catalog's companion) checked against the members. A diagnostic that has
/// already read the bundle ([`bundle::verify_read`]) learns whether
/// Darkmatter would load it without loading it through a `register` entry.
///
/// `context` is the request's captured context, as for
/// [`load_registrations`]; each member resolves in a context derived from
/// it and anchored at the member.
///
/// ## Errors
///
/// Every refusal, located as for a `register` entry; an embedded snapshot
/// is refused ([`load_embedded`] prepares it).
pub fn load_verified(
    snapshot: VerifiedSnapshot,
    context: &FileResolutionContext,
    owned: &BTreeSet<String>,
) -> Result<RegisteredHostProfile, Vec<bundle::SnapshotRefusal>> {
    let as_refusal = |Labeled { refusal, label }: Labeled| bundle::SnapshotRefusal {
        label,
        ..bundle::SnapshotRefusal::of(refusal, None, Vec::new(), Vec::new())
    };
    if snapshot.embedded_bundle().is_some() {
        let refusal = Refusal {
            code: HOST_REGISTRATION_CODE,
            file: snapshot.manifest().to_path_buf(),
            at: None,
            member: None,
            message: "the snapshot is embedded; load it with `load_embedded`".to_string(),
            related: Vec::new(),
        };
        return Err(vec![as_refusal(Labeled { refusal, label: None })]);
    }
    profile_of(snapshot, context, owned).map_err(|refused| refused.refusals.into_iter().map(as_refusal).collect())
}

/// The profile of the embedded bundle `snapshot`
/// ([`VerifiedSnapshot::from_embedded`]): every member classified and
/// prepared from its verified bytes inside its virtual root, exactly as an
/// installed bundle's members are, with no disk read. Each trigger's source
/// is [`TriggerSource::Embedded`](simplified_schema::triggers::TriggerSource::Embedded)
/// named by its bundle path (`embedded:claudine/schemas/composition.yaml`),
/// so origins and traces never name a host file. A host compiles its own
/// bundle in this way to judge documents in process without an installed
/// registration.
///
/// `prepared` serves payloads the host prepared ahead of time, by trigger
/// bundle path; a trigger it serves is not resolved again. The host
/// vouches that each served payload was prepared from that trigger's
/// verified bytes, with every check registration runs ([`prepare_embedded_payload`]).
///
/// ## Errors
///
/// Every refusal, as for an installed manifest; a snapshot that is not
/// embedded is refused.
pub fn load_embedded(
    snapshot: VerifiedSnapshot,
    owned: &BTreeSet<String>,
    prepared: &dyn Fn(&str) -> Option<ResolvedPayload>,
) -> Result<RegisteredHostProfile, Vec<bundle::SnapshotRefusal>> {
    let as_refusal = |Labeled { refusal, label }: Labeled| bundle::SnapshotRefusal {
        label,
        ..bundle::SnapshotRefusal::of(refusal, None, Vec::new(), Vec::new())
    };
    if snapshot.embedded_bundle().is_none() {
        let refusal = Refusal {
            code: HOST_REGISTRATION_CODE,
            file: snapshot.manifest().to_path_buf(),
            at: None,
            member: None,
            message: "the snapshot was read from disk; load it through its `register` entry".to_string(),
            related: Vec::new(),
        };
        return Err(vec![as_refusal(Labeled { refusal, label: None })]);
    }
    let context = embedded_context(&snapshot);
    profile_with(snapshot, &context, owned, prepared)
        .map_err(|refused| refused.refusals.into_iter().map(as_refusal).collect())
}

/// The context an embedded bundle prepares in: detached at its virtual
/// root, with no home and no environment, so nothing outside the bundle's
/// verified bytes can be reached through it.
fn embedded_context(snapshot: &VerifiedSnapshot) -> FileResolutionContext {
    FileResolutionContext::from_snapshot(snapshot.bundle_root(), None, std::collections::HashMap::new())
}

/// The embedded trigger at bundle path `path` of `snapshot`
/// ([`VerifiedSnapshot::from_embedded`]), its payload resolved and checked
/// as [`load_embedded`] resolves it: what a host records ahead of time and
/// later serves through `load_embedded`'s `prepared`. `Ok(None)` when the
/// member is not a schema trigger.
///
/// ## Errors
///
/// Why the member does not load as a registered trigger.
pub fn prepare_embedded_payload(
    snapshot: &VerifiedSnapshot,
    path: &str,
    owned: &BTreeSet<String>,
) -> Result<Option<ResolvedPayload>, String> {
    let name = snapshot.embedded_bundle().ok_or_else(|| "the snapshot is not embedded".to_string())?;
    let member = snapshot.files().iter().find(|member| member.path == path).ok_or_else(|| format!("`{path}` is not a member"))?;
    let text = member_text(snapshot, member).ok_or_else(|| format!("`{path}` is not UTF-8 text"))?;
    if !matches!(member_kind(text, &member.location).map_err(|refusal| refusal.message)?, MemberKind::Trigger) {
        return Ok(None);
    }
    let sources = BundleSources::embedded(name, snapshot.bytes().clone()).map_err(|error| error.to_string())?.with_revision(snapshot.revision());
    let context = embedded_context(snapshot);
    let anchor = anchored_at(&context, &member.location);
    let texts = |location: &Path| verified_text(snapshot, location);
    let (loaded, _) = resolve_bundle::with(&sources, || {
        load_trigger(member, &anchor, &context, owned, Some(name), &|_| None, &texts)
    });
    let trigger = loaded.map_err(|refusal| refusal.message)?;
    declared_types(&sources, member, text, &anchor, &texts, &mut Vec::new()).map_err(|refusal| refusal.message)?;
    Ok(Some(trigger.payload))
}

/// The profile of `snapshot`, prepared in `context`.
fn profile_of(snapshot: VerifiedSnapshot, context: &FileResolutionContext, owned: &BTreeSet<String>) -> Result<RegisteredHostProfile, Refused> {
    profile_with(snapshot, context, owned, &|_| None)
}

fn profile_with(
    snapshot: VerifiedSnapshot,
    context: &FileResolutionContext,
    owned: &BTreeSet<String>,
    prepared: &dyn Fn(&str) -> Option<ResolvedPayload>,
) -> Result<RegisteredHostProfile, Refused> {
    let manifest_hash = snapshot.manifest_hash().to_string();
    let path = snapshot.manifest().to_path_buf();
    let snapshot = Arc::new(snapshot);
    let members = prepare_members(&snapshot, context, owned, prepared);
    let mut candidates: Vec<PathBuf> = snapshot.candidates().to_vec();
    candidates.extend(members.missing.iter().cloned());
    let candidates = dedup(candidates);
    if !members.refusals.is_empty() {
        let refusals = members.refusals.into_iter().map(|refusal| related(&snapshot, refusal)).collect();
        return Err(Refused { refusals, watches: candidates });
    }
    let Members { triggers, kinds, libraries, catalog, lifecycle, functions, .. } = members;
    let (Some(catalog), Some((lifecycle_path, lifecycle))) = (catalog, lifecycle) else {
        unreachable!("a preparation without refusals has a catalog and a lifecycle profile");
    };
    Ok(RegisteredHostProfile {
        host: snapshot.host().to_string(),
        version: snapshot.version().to_string(),
        manifest: ManifestIdentity { path, content_hash: manifest_hash },
        entries: Vec::new(),
        triggers,
        kinds,
        libraries,
        catalog,
        lifecycle_path,
        lifecycle: Arc::new(lifecycle),
        functions,
        candidates,
        snapshot,
        checked: std::sync::Mutex::default(),
    })
}

fn dedup(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen: Vec<PathBuf> = Vec::with_capacity(paths.len());
    for path in paths {
        if !seen.contains(&path) {
            seen.push(path);
        }
    }
    seen
}

/// Everything a verified snapshot's members prepare into, with every
/// refusal found.
#[derive(Default)]
struct Members {
    triggers: Vec<RegisteredTrigger>,
    kinds: Vec<RegisteredDefinition>,
    libraries: Vec<String>,
    catalog: Option<PreparedCatalog>,
    lifecycle: Option<(PathBuf, HostLifecycleProfile)>,
    functions: Vec<HostFunctionSet>,
    /// Unlisted locations the members' references resolved to.
    missing: Vec<PathBuf>,
    refusals: Vec<Refusal>,
}

/// A `files` member's display `name`, for the registration-wide uniqueness
/// check.
struct Label {
    name: String,
    member: String,
    file: PathBuf,
    at: Option<SourcePosition>,
}

/// Classifies and prepares every member of `snapshot` from its verified
/// bytes alone.
fn prepare_members(
    snapshot: &VerifiedSnapshot,
    context: &FileResolutionContext,
    owned: &BTreeSet<String>,
    prepared: &dyn Fn(&str) -> Option<ResolvedPayload>,
) -> Members {
    let mut members = Members::default();
    let embedded = snapshot.embedded_bundle();
    let sources = match embedded {
        Some(name) => BundleSources::embedded(name, snapshot.bytes().clone()),
        None => BundleSources::on_disk(snapshot.bundle_root(), snapshot.bytes().clone()),
    };
    let sources = match sources {
        Ok(sources) => sources.with_revision(snapshot.revision()),
        Err(error) => {
            members.refusals.push(Refusal {
                code: HOST_REGISTRATION_CODE,
                file: snapshot.manifest().to_path_buf(),
                at: None,
                member: None,
                message: error.to_string(),
                related: Vec::new(),
            });
            return members;
        }
    };
    let mut namespaces: Vec<&str> = Vec::new();
    // Set when a member that is or might be a namespace was refused: the
    // catalog would lack it, so no lifecycle artifact is checked against it.
    let mut catalog_incomplete = false;
    let mut labels: Vec<Label> = Vec::new();
    let mut catalogs: Vec<&BundleMember> = Vec::new();
    let texts = |location: &Path| verified_text(snapshot, location);
    let mut files: Vec<&BundleMember> = snapshot.files().iter().collect();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    for member in files {
        let refuse = |at: Option<SourcePosition>, message: String| Refusal {
            code: HOST_REGISTRATION_CODE,
            file: member.location.clone(),
            at,
            member: Some(member.field.clone()),
            message,
            related: Vec::new(),
        };
        let Some(text) = member_text(snapshot, member) else {
            // Reported at its manifest entry ([`related`]). Its kind is
            // unknown, so it might have been a namespace.
            catalog_incomplete = true;
            members.refusals.push(refuse(None, "the file is not UTF-8 text".to_string()));
            continue;
        };
        let anchor = anchored_at(context, &member.location);
        match member_kind(text, &member.location) {
            Ok(MemberKind::Trigger) => {
                let (loaded, reads) = resolve_bundle::with(&sources, || {
                    load_trigger(member, &anchor, context, owned, embedded, prepared, &texts)
                });
                members.missing.extend(reads.missing);
                match loaded.and_then(|trigger| declared_types(&sources, member, text, &anchor, &texts, &mut members.missing).map(|()| trigger)) {
                    Ok(trigger) => {
                        if let Some(name) = &trigger.trigger.envelope.name {
                            labels.push(label(name, member, text));
                        }
                        members.triggers.push(trigger);
                    }
                    Err(refusal) => members.refusals.push(refusal),
                }
            }
            Ok(MemberKind::YamlSchema) => {
                let (prepared, reads) = resolve_bundle::with(&sources, || {
                    simplified_schema::resolve::prepare_yaml_schema_text(
                        &member.location,
                        text,
                        &[],
                        &anchor,
                        crate::markdown::schemas::embedded::SHIPPED_SOURCES,
                    )
                });
                members.missing.extend(reads.missing);
                let source = RegisteredSource {
                    host: snapshot.host().to_string(),
                    path: member.path.clone(),
                    file: member.location.clone(),
                };
                let sameness = format!("{}\0{}", snapshot.revision(), member.path);
                match prepared.and_then(|resolved| RegisteredDefinition::new(source, text, resolved, sameness)) {
                    Ok(definition) => match declared_types(&sources, member, text, &anchor, &texts, &mut members.missing) {
                        Ok(()) => members.kinds.push(definition),
                        Err(refusal) => members.refusals.push(refusal),
                    },
                    Err(error) => {
                        let site = locate::schema_error(&error, &member.location, &texts);
                        members.refusals.push(Refusal {
                            file: site.file,
                            at: site.at,
                            ..refuse(None, format!("the YAML kind definition does not load: {error}"))
                        });
                    }
                }
            }
            Ok(MemberKind::Namespace) => namespaces.push(member.path.as_str()),
            Ok(MemberKind::ExpressionSchema) => catalogs.push(member),
            Ok(MemberKind::Library) => match library(text, &member.location) {
                Ok(name) => {
                    if let Some(name) = name {
                        labels.push(label(&name, member, text));
                    }
                    match declared_types(&sources, member, text, &anchor, &texts, &mut members.missing) {
                        Ok(()) => members.libraries.push(member.path.clone()),
                        Err(refusal) => members.refusals.push(refusal),
                    }
                }
                Err((at, message)) => members.refusals.push(refuse(at, message)),
            },
            Err(refusal) => {
                // A member refused before its kind was known (not YAML, no
                // or an unknown `kind`) might have been a namespace.
                catalog_incomplete |= refusal.kind.is_none_or(|kind| kind == MemberKind::Namespace);
                members.refusals.push(refuse(refusal.at, refusal.message));
            }
        }
    }
    members.refusals.extend(duplicate_labels(&labels));

    match namespaces::prepare_catalog(&sources, snapshot.bytes(), &namespaces, context) {
        Ok(_) if catalog_incomplete => {}
        Ok(catalog) => {
            // A namespace's unused declarations are checked once its payload
            // prepared, and before the catalog or the lifecycle artifact
            // checked against it is used.
            let refused = members.refusals.len();
            for member in snapshot.files().iter().filter(|member| namespaces.contains(&member.path.as_str())) {
                let text = member_text(snapshot, member).unwrap_or_default();
                let anchor = anchored_at(context, &member.location);
                if let Err(refusal) = declared_types(&sources, member, text, &anchor, &texts, &mut members.missing) {
                    members.refusals.push(refusal);
                }
            }
            if members.refusals.len() == refused {
                members.catalog = Some(catalog);
            }
        }
        Err(refusals) => members.refusals.extend(refusals.into_iter().map(|refusal| Refusal {
            code: HOST_REGISTRATION_CODE,
            member: snapshot.files().iter().find(|member| member.path == refusal.path).map(|member| member.field.clone()),
            file: refusal.location,
            at: refusal.at,
            message: refusal.message,
            related: Vec::new(),
        })),
    }
    if let Some(catalog) = &members.catalog {
        match load_lifecycle(snapshot, catalog) {
            Ok(lifecycle) => members.lifecycle = Some(lifecycle),
            Err(refusal) => members.refusals.push(refusal),
        }
    }
    let mut sets: Vec<(HostFunctionSet, &BundleCompanion)> = Vec::new();
    for catalog in catalogs {
        let Some(pair) = snapshot.companions().iter().find(|pair| pair.catalog.path == catalog.path) else {
            // The snapshot derives a companion for every catalog it classified.
            members.refusals.push(Refusal {
                code: HOST_REGISTRATION_CODE,
                file: snapshot.manifest().to_path_buf(),
                at: catalog.at.clone(),
                member: Some(catalog.field.clone()),
                message: format!("`{}` has no verified facts companion", catalog.reference),
                related: Vec::new(),
            });
            continue;
        };
        match load_functions(snapshot, pair) {
            Ok(set) => sets.push((set, pair)),
            Err(refusal) => members.refusals.push(refusal),
        }
    }
    members.refusals.extend(name_collisions(snapshot, &sets));
    members.functions = sets.into_iter().map(|(set, _)| set).collect();
    members
}

/// One name a host's catalogs claim: a canonical name, or an alias its facts
/// companion records, and where it is written.
struct Claim<'a> {
    folded: String,
    spelled: String,
    alias_of: Option<String>,
    pair: &'a BundleCompanion,
    file: PathBuf,
    at: Option<SourcePosition>,
}

/// Every canonical name and alias claimed by two of the host's catalogs,
/// compared case-insensitively over the catalogs in bundle-path order: each
/// later claimant is refused at its declaration (a catalog key, or an alias
/// in its companion), with the earliest claim as a related location. A name
/// repeated inside one catalog is that catalog's own failure, judged when it
/// loads; another host's catalogs are never compared.
fn name_collisions(snapshot: &VerifiedSnapshot, sets: &[(HostFunctionSet, &BundleCompanion)]) -> Vec<Refusal> {
    let mut claims: Vec<Claim<'_>> = Vec::new();
    for (set, pair) in sets {
        let catalog_text = member_text(snapshot, &pair.catalog).unwrap_or_default();
        let facts_text = member_text(snapshot, &pair.facts).unwrap_or_default();
        let types = simplified_schema::yaml_documents::load_yaml_document(catalog_text, &pair.catalog.location).ok();
        for name in set.functions.function_names() {
            let at = types.as_ref().and_then(|loaded| {
                loaded
                    .byte_range(&simplified_schema::yaml_documents::locating_problem(&["types".to_string(), name.to_string()]))
                    .map(|range| line_column(catalog_text, range.start))
            });
            claims.push(Claim {
                folded: name.to_ascii_lowercase(),
                spelled: name.to_string(),
                alias_of: None,
                pair,
                file: pair.catalog.location.clone(),
                at,
            });
            for (index, alias) in set.functions.aliases(name).unwrap_or_default().iter().enumerate() {
                let pointer = locate::artifact_pointer(&["functions".to_string(), name.to_string(), "aliases".to_string(), index.to_string()]);
                let at = biscuit_file::data_locations::locate_json(facts_text, &pointer).map(|offset| line_column(facts_text, offset));
                claims.push(Claim {
                    folded: alias.to_ascii_lowercase(),
                    spelled: alias.clone(),
                    alias_of: Some(name.to_string()),
                    pair,
                    file: pair.facts.location.clone(),
                    at,
                });
            }
        }
    }
    let describe = |claim: &Claim<'_>| match &claim.alias_of {
        Some(function) => format!("alias `{}` of `{function}` in `{}`", claim.spelled, claim.pair.facts.path),
        None => format!("function `{}` in `{}`", claim.spelled, claim.pair.catalog.path),
    };
    let mut refusals = Vec::new();
    for (index, claim) in claims.iter().enumerate() {
        let Some(first) = claims[..index]
            .iter()
            .find(|earlier| earlier.folded == claim.folded && !std::ptr::eq(earlier.pair, claim.pair))
        else {
            continue;
        };
        refusals.push(Refusal {
            code: HOST_REGISTRATION_CODE,
            file: claim.file.clone(),
            at: claim.at.clone(),
            member: Some(claim.pair.catalog.field.clone()),
            message: format!(
                "the {} claims the name `{}`, which the {} already claims; a host's expression catalogs share one \
                 namespace, compared without letter case",
                describe(claim),
                claim.folded,
                describe(first)
            ),
            related: vec![RegistrationRelated {
                message: format!("the earlier {}", describe(first)),
                file: first.file.clone(),
                line: first.at.as_ref().map(|at| at.line),
                column: first.at.as_ref().map(|at| at.column),
                line_prefix: first.at.as_ref().map(|at| at.line_prefix.clone()),
            }],
        });
    }
    refusals
}

/// The bundle boundary applied to every declaration in `member`'s `types`
/// table, used by its export or not
/// ([`simplified_schema::resolve::check_declared_types`] inside the bundle's
/// verified bytes): refused at the authored import or example reference
/// that leaves the bundle or names no listed member, so the whole host is
/// refused with it.
fn declared_types(
    sources: &BundleSources,
    member: &BundleMember,
    text: &str,
    anchor: &FileResolutionContext,
    texts: &locate::Texts<'_>,
    missing: &mut Vec<PathBuf>,
) -> Result<(), Refusal> {
    let (checked, reads) = resolve_bundle::with(sources, || {
        simplified_schema::resolve::check_declared_types(text, &member.location, anchor, crate::markdown::schemas::embedded::SHIPPED_SOURCES)
    });
    missing.extend(reads.missing);
    checked.map(drop).map_err(|error| {
        let site = locate::declared_type_error(&error, &member.location, texts);
        Refusal {
            code: HOST_REGISTRATION_CODE,
            file: site.file,
            at: site.at,
            member: Some(member.field.clone()),
            message: format!("a declared type does not resolve: {error}"),
            related: Vec::new(),
        }
    })
}

/// Every member: `files`, the lifecycle artifact, then each catalog's facts
/// companion.
fn listed(snapshot: &VerifiedSnapshot) -> impl Iterator<Item = &BundleMember> {
    snapshot.members()
}

/// The verified text of the member read from `location`.
fn verified_text<'a>(snapshot: &'a VerifiedSnapshot, location: &Path) -> Option<&'a str> {
    listed(snapshot).find(|member| member.location == location).and_then(|member| member_text(snapshot, member))
}

/// `refusal` reported where its problem is: a problem located in another
/// member (a library a trigger imports) is that member's refusal, and its
/// message names the member that imported it (a facts companion is its
/// catalog's entry, never an import of it). An unlocated problem in a
/// member that is not text is reported at that member's manifest entry.
/// The member the refusal is reported under, and an importer it names,
/// are shown by their display labels when they declare one
/// ([`member_label`]).
fn related(snapshot: &VerifiedSnapshot, refusal: Refusal) -> Labeled {
    let Some(importer) = refusal.member.as_deref().and_then(|field| listed(snapshot).find(|member| member.field == field)) else {
        return Labeled { refusal, label: None };
    };
    let Some(failed) = listed(snapshot).find(|member| member.location == refusal.file) else {
        return Labeled { refusal, label: member_label(snapshot, importer) };
    };
    // A facts companion shares its catalog's `files` entry: a problem in it
    // is that entry's own, not one the catalog imported.
    let refusal = if importer.field == failed.field {
        refusal
    } else {
        let named = member_label(snapshot, importer).unwrap_or_else(|| importer.path.clone());
        Refusal {
            member: Some(failed.field.clone()),
            message: format!("imported by `{}` (`{named}`): {}", importer.field, refusal.message),
            ..refusal
        }
    };
    let label = member_label(snapshot, failed);
    if refusal.at.is_none() && member_text(snapshot, failed).is_none() {
        return Labeled { refusal: Refusal { file: snapshot.manifest().to_path_buf(), at: failed.at.clone(), ..refusal }, label };
    }
    Labeled { refusal, label }
}

/// `member` as a diagnostic names it beside its path, `name (bundle path)`,
/// when its verified text is a trigger or types-only library declaring a
/// valid `name` ([`simplified_schema::member_envelope::declared_name`]);
/// `None` otherwise, and the path alone is shown.
fn member_label(snapshot: &VerifiedSnapshot, member: &BundleMember) -> Option<String> {
    let name = simplified_schema::member_envelope::declared_name(member_text(snapshot, member)?)?;
    Some(format!("{name} ({})", member.path))
}

fn member_text<'a>(snapshot: &'a VerifiedSnapshot, member: &BundleMember) -> Option<&'a str> {
    std::str::from_utf8(snapshot.member_bytes(&member.path)?).ok()
}

/// What a `files` member is, by its own `kind`, once its envelope passed
/// the structural member admission Claudine's build runs too
/// ([`simplified_schema::member_envelope`]); refused at the value of the
/// field it names, or at the document's mapping when that field is absent.
fn member_kind(text: &str, location: &Path) -> Result<MemberKind, MemberKindRefusal> {
    let loaded = simplified_schema::yaml_documents::load_yaml_document(text, location).map_err(|error| MemberKindRefusal {
        kind: None,
        at: locate::schema_error(&error, location, &|path: &Path| (path == location).then_some(text)).at,
        message: format!("the file is not one YAML mapping: {error}"),
    })?;
    admit_member_text(text).map(|admitted| admitted.kind).map_err(|refusal| {
        let segments = match refusal.field {
            Some(_) if refusal.absent => Some(Vec::new()),
            Some(field) => Some(vec![field]),
            None => None,
        };
        let at = segments.and_then(|segments| {
            loaded
                .byte_range(&simplified_schema::yaml_documents::locating_problem(&segments))
                .map(|range| line_column(text, range.start))
        });
        MemberKindRefusal { kind: refusal.kind, at, message: refusal.message }
    })
}

/// A member [`member_kind`] refused: the kind its `kind` named, when it
/// named one, and where and why.
struct MemberKindRefusal {
    kind: Option<MemberKind>,
    at: Option<SourcePosition>,
    message: String,
}

/// A refusal's position in the member it is about, and why.
type LocatedMessage = (Option<SourcePosition>, String);

/// The display name of the types-only library `text`, once it proved to be
/// one: `kind: schema` with a non-empty `types` table and no `$schema`; else
/// where and why it is not.
fn library(text: &str, location: &Path) -> Result<Option<String>, LocatedMessage> {
    let at = |field: &str| locate::field(text, location, &[field.to_string()]);
    let document = simplified_schema::parse_standalone_schema_document(text, location)
        .map_err(|error| {
            let site = locate::schema_error(&error, location, &|path: &Path| (path == location).then_some(text));
            (site.at, format!("the type library is invalid: {error}"))
        })?
        .ok_or_else(|| (None, "the file is not a schema document".to_string()))?;
    if document.exports_schema {
        return Err((
            at("$schema"),
            "a registered `kind: schema` file is a types-only library; it may not declare `$schema`".to_string(),
        ));
    }
    if document.named_types().is_none_or(|types| types.properties.is_empty() && types.pattern_keys.is_empty()) {
        return Err((at("types"), "a registered type library must declare a non-empty `types` table".to_string()));
    }
    Ok(document.name)
}

fn label(name: &str, member: &BundleMember, text: &str) -> Label {
    let at = simplified_schema::yaml_documents::load_yaml_document(text, &member.location).ok().and_then(|loaded| {
        loaded.byte_range(&simplified_schema::yaml_documents::locating_problem(&["name".to_string()])).map(|range| line_column(text, range.start))
    });
    Label { name: name.to_string(), member: member.field.clone(), file: member.location.clone(), at }
}

/// One refusal per file whose `name` another file of the registration also
/// declares, at that file's `name`.
fn duplicate_labels(labels: &[Label]) -> Vec<Refusal> {
    labels
        .iter()
        .filter_map(|label| {
            let others: Vec<String> = labels
                .iter()
                .filter(|other| other.name == label.name && other.file != label.file)
                .map(|other| format!("`{}`", other.file.display()))
                .collect();
            (!others.is_empty()).then(|| Refusal {
                code: HOST_REGISTRATION_CODE,
                file: label.file.clone(),
                at: label.at.clone(),
                member: Some(label.member.clone()),
                message: format!("the name `{}` is also declared by {}; names are unique within a registration", label.name, others.join(", ")),
                related: Vec::new(),
            })
        })
        .collect()
}

/// The lifecycle artifact, checked against `catalog`.
fn load_lifecycle(snapshot: &VerifiedSnapshot, catalog: &PreparedCatalog) -> Result<(PathBuf, HostLifecycleProfile), Refusal> {
    let member = snapshot.lifecycle();
    let refuse = |message: String| Refusal {
        code: HOST_REGISTRATION_CODE,
        file: member.location.clone(),
        at: None,
        member: Some(member.field.clone()),
        message,
        related: Vec::new(),
    };
    let artifact = match member_text(snapshot, member) {
        Some(text) => FactsArtifact::Text(text),
        None => FactsArtifact::Unreadable("the file is not UTF-8 text"),
    };
    let lifecycle =
        HostLifecycleProfile::from_artifact_located(Arc::clone(&catalog.catalog), &catalog.hash, &member.location, artifact)
            .map_err(|(message, site)| Refusal {
                at: member_text(snapshot, member).and_then(|text| locate::artifact(text, site.as_ref())),
                ..refuse(message)
            })?;
    // The reader checks a root's syntax and reserved names, and `host_globals`
    // only when `global_catalog` attaches the catalog; registration admits a
    // root only when Darkmatter or one of the host's namespaces declares it.
    if let Some(text) = member_text(snapshot, member)
        && let Some((segments, root)) = host_profile::undeclared_root(text, &catalog.catalog)
    {
        let at = simplified_schema::yaml_documents::load_yaml_document(text, &member.location).ok().and_then(|loaded| {
            loaded
                .byte_range(&simplified_schema::yaml_documents::locating_problem(&segments))
                .map(|range| line_column(text, range.start))
        });
        return Err(Refusal {
            at,
            message: format!(
                "the lifecycle artifact `{}` binds `{root}` at `{}`, which neither Darkmatter nor any namespace of the host's catalog declares",
                member.location.display(),
                simplified_schema::yaml_documents::pointer_string(&segments)
            ),
            ..refuse(String::new())
        });
    }
    let recorded = lifecycle.registry().host();
    if recorded != snapshot.host() {
        return Err(Refusal {
            code: HOST_REGISTRATION_CODE,
            file: snapshot.manifest().to_path_buf(),
            at: snapshot.host_position().cloned(),
            member: Some("host".to_string()),
            message: format!(
                "the manifest names host `{}`, but its lifecycle artifact `{}` records host `{recorded}`",
                snapshot.host(),
                member.location.display()
            ),
            related: Vec::new(),
        });
    }
    Ok((member.location.clone(), lifecycle))
}

/// One expression catalog's analysis-only set, from verified bytes: the
/// catalog read by the authored catalog reader, its facts from the derived
/// companion, every function complete.
fn load_functions(snapshot: &VerifiedSnapshot, pair: &BundleCompanion) -> Result<HostFunctionSet, Refusal> {
    let refuse = |member: &BundleMember, message: String| Refusal {
        code: HOST_REGISTRATION_CODE,
        file: member.location.clone(),
        at: None,
        member: Some(member.field.clone()),
        message,
        related: Vec::new(),
    };
    let text = member_text(snapshot, &pair.catalog)
        .ok_or_else(|| refuse(&pair.catalog, "the expression catalog is not UTF-8 text".to_string()))?;
    let catalog = HostFunctionCatalog::load(text, &pair.catalog.location).map_err(|error| Refusal {
        at: error.span().map(|span| line_column(text, span.start)),
        related: error.related().iter().map(|related| catalog_related(snapshot, related)).collect(),
        ..refuse(&pair.catalog, format!("the expression catalog does not load: {error}"))
    })?;
    let artifact = match member_text(snapshot, &pair.facts) {
        Some(facts) => FactsArtifact::Text(facts),
        None => FactsArtifact::Unreadable("the file is not UTF-8 text"),
    };
    let (set, sites) = analysis_only_sited(catalog, &pair.facts.location, artifact)
        .map_err(|error| refuse(&pair.catalog, error.to_string()))?;
    let incomplete = set.incomplete_functions();
    if let Some((function, _)) = incomplete.first() {
        let reasons: Vec<String> = incomplete.iter().map(|(name, reason)| format!("`{name}`: {reason}")).collect();
        // Reported where the reader judged the first incomplete function.
        return Err(Refusal {
            at: member_text(snapshot, &pair.facts)
                .and_then(|facts| locate::artifact(facts, sites.get(*function))),
            ..refuse(&pair.facts, format!("its functions are incomplete: {}", reasons.join("; ")))
        });
    }
    Ok(HostFunctionSet {
        bundle_path: pair.catalog.path.clone(),
        catalog: pair.catalog.location.clone(),
        facts: pair.facts.location.clone(),
        functions: set,
    })
}

/// A catalog failure's related location, positioned when its file is a
/// verified member (a built-in catalog's identity has no text here).
fn catalog_related(snapshot: &VerifiedSnapshot, related: &simplified_schema::errors::CatalogRelated) -> RegistrationRelated {
    let at = related
        .span
        .as_ref()
        .and_then(|span| verified_text(snapshot, &related.file).map(|text| line_column(text, span.start)));
    RegistrationRelated {
        message: related.message.clone(),
        file: related.file.clone(),
        line: at.as_ref().map(|at| at.line),
        column: at.as_ref().map(|at| at.column),
        line_prefix: at.map(|at| at.line_prefix),
    }
}

/// The registered trigger `member`, read from `texts` (which serve every
/// verified member, so a refusal in a file it imports is located there),
/// its payload resolved from its own folder inside the bundle and its
/// conditions admitted against `owned`.
fn load_trigger(
    member: &BundleMember,
    anchor: &FileResolutionContext,
    context: &FileResolutionContext,
    owned: &BTreeSet<String>,
    embedded: Option<&str>,
    prepared: &dyn Fn(&str) -> Option<ResolvedPayload>,
    texts: &locate::Texts<'_>,
) -> Result<RegisteredTrigger, Refusal> {
    let path = &member.location;
    let text = texts(path).unwrap_or_default();
    let refuse = |code: &'static str, at: Option<SourcePosition>, message: String| Refusal {
        code,
        file: path.clone(),
        at,
        member: Some(member.field.clone()),
        message,
        related: Vec::new(),
    };
    let text: Arc<str> = Arc::from(text);
    let envelope: TriggerEnvelope = match parse_trigger_envelope_from_str(&text) {
        Ok(Some(envelope)) => envelope,
        Ok(None) => return Err(refuse(HOST_REGISTRATION_CODE, None, "the file is not a schema trigger (`kind: schema-trigger`)".to_string())),
        Err(error) => {
            let at = locate::schema_error(&error, path, &|location: &Path| (location == path).then_some(&*text)).at;
            return Err(refuse(HOST_REGISTRATION_CODE, at, format!("the trigger is invalid: {error}")));
        }
    };
    let identifying = admit_identifying_trigger(&envelope.conditions, owned).map_err(|refusal| {
        let at = simplified_schema::yaml_documents::load_yaml_document(&text, path).ok().and_then(|loaded| {
            loaded
                .byte_range(&simplified_schema::yaml_documents::locating_problem(&["match".to_string()]))
                .map(|range| line_column(&text, range.start))
        });
        refuse(
            HOST_IDENTIFY_TOO_BROAD_CODE,
            at,
            format!("the trigger could match a document `md` owns, so it cannot identify the host: {refusal}"),
        )
    })?;
    let mut trigger = LoadedTrigger::from_file(path.clone(), Arc::clone(&text), envelope, context.base_dir().to_path_buf());
    // A registered trigger is its own source: its payload resolves from its
    // file, never through the document's schema roots. It may be a root
    // union, which the host's checked layer owns.
    let mut payload = match prepared(&member.path) {
        Some(payload) => payload,
        None => resolve_registered_trigger_payload(&trigger, anchor, crate::markdown::schemas::embedded::SHIPPED_SOURCES).map_err(
            |error| {
                let site = locate::schema_error(&error, path, texts);
                Refusal {
                    file: site.file,
                    ..refuse(HOST_REGISTRATION_CODE, site.at, format!("the trigger's payload does not load: {error}"))
                }
            },
        )?,
    };
    // An embedded trigger resolved from its virtual path; from here on it is
    // known by its bundle identity, never as a file.
    // Its payload's dependency and probe paths are virtual too: nothing on
    // disk can change an embedded payload, so none is kept.
    if let Some(bundle) = embedded {
        trigger.source = simplified_schema::triggers::TriggerSource::Embedded { bundle: bundle.to_string(), name: member.path.clone() };
        payload.dependencies.clear();
        payload.searched.clear();
    }
    Ok(RegisteredTrigger { trigger, payload, bundle_path: member.path.clone(), location: path.clone(), identifying })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_name_list_items_by_index() {
        assert_eq!(field(&["lifecycle"]), "lifecycle");
        assert_eq!(field(&["files", "1"]), "files[1]");
        assert_eq!(field(&["digests", "./a.yaml"]), "digests../a.yaml");
    }

    #[test]
    fn a_profile_is_shareable_across_threads() {
        fn shareable<T: Send + Sync>() {}
        shareable::<RegisteredHostProfile>();
        shareable::<LoadedRegistrations>();
    }
}
