//! Host selection: which host, if any, a document belongs to.
//!
//! [`RegisteredHosts`] holds one request's loaded registrations, grouped by
//! [`agreement`] once. [`RegisteredHosts::identify`] runs every usable
//! host's identifying trigger against a document; [`select`] adds the legacy
//! root-catalog fallback for a document no registered host identifies.
//! Both are passive: they read no configuration, write nothing, fetch
//! nothing, and prompt for nothing. Only the legacy view reads files: the
//! host catalogs in the document's schema roots.
//!
//! User-facing guide: `darkmatter/docs/topics/host-registration.md`
//! ("Selection").

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, PathIdentity, canonicalize_simplified};
use serde_json::Value;
use simplified_schema::triggers::{PathSubject, identifies};

use super::{
    HOST_AMBIGUOUS_CODE, HostClaim, LoadedRegistrations, RegisteredHostProfile, RegistrationDiagnostic, agreement,
    load_registrations, md_owned_names,
};
use crate::config::load::ConfigSnapshot;
use crate::lifecycle::host_profile::HostLifecycleProfile;
use crate::markdown::compose::expression::{HostFunctions, Severity};
use crate::markdown::schemas::catalog::{
    CatalogError, GlobalCatalog, HostFunctionCatalogError, host_catalog_files, host_function_catalog_files,
    host_lifecycle_profiles,
};

/// The host a document is analyzed as.
#[derive(Debug, Clone)]
pub enum HostSelection {
    /// No host: the document is an `md` document. Also the result when a
    /// caller supplied its own executable host options, which registration
    /// never replaces.
    Md,
    /// Exactly one registered host identified the document: its triggers,
    /// globals, functions, and lifecycle apply together.
    Registered(Arc<RegisteredHostProfile>),
    /// No registered host identified the document, and the document's schema
    /// roots hold a host catalog of a host that is not registered.
    Legacy(Arc<LegacyHost>),
    /// More than one host identified the document, or a host whose
    /// registrations conflict did. The document is analyzed as `md`, with no
    /// legacy fallback. Holds the host identifiers, in registration order.
    Ambiguous(Vec<String>),
}

impl HostSelection {
    /// The selected registered profile, when one was selected.
    #[must_use]
    pub fn registered(&self) -> Option<&Arc<RegisteredHostProfile>> {
        match self {
            Self::Registered(profile) => Some(profile),
            _ => None,
        }
    }
}

/// A selection with the warnings it produced for this document.
#[derive(Debug, Clone)]
pub struct Selected {
    #[allow(missing_docs)]
    pub selection: HostSelection,
    /// `dm.host.ambiguous` when two hosts identified the document; the
    /// conflicting claims' `dm.host.registration` warnings when a conflicted
    /// host did. The same conflict warnings are also part of
    /// [`RegisteredHosts::diagnostics`], so a caller reporting both should
    /// report each distinct record once.
    pub diagnostics: Vec<RegistrationDiagnostic>,
}

impl Selected {
    const fn quiet(selection: HostSelection) -> Self {
        Self { selection, diagnostics: Vec::new() }
    }
}

/// One request's registrations, ready for selection: loaded once
/// ([`load_registrations`]), grouped by host once ([`agreement`]).
#[derive(Debug)]
pub struct RegisteredHosts {
    loaded: Arc<LoadedRegistrations>,
    claims: Vec<HostClaim>,
}

impl RegisteredHosts {
    /// Groups `loaded` for selection.
    #[must_use]
    pub fn new(loaded: Arc<LoadedRegistrations>) -> Self {
        let claims = agreement(&loaded.profiles);
        Self { loaded, claims }
    }

    /// Loads `snapshot`'s registrations in `context` ([`load_registrations`]
    /// with [`md_owned_names`]) and groups them.
    #[must_use]
    pub fn load(snapshot: &ConfigSnapshot, context: &FileResolutionContext) -> Self {
        Self::new(Arc::new(load_registrations(snapshot, context, md_owned_names())))
    }

    /// The loaded registrations.
    #[must_use]
    pub fn loaded(&self) -> &Arc<LoadedRegistrations> {
        &self.loaded
    }

    /// The per-host claims, in first-claim order.
    #[must_use]
    pub fn claims(&self) -> &[HostClaim] {
        &self.claims
    }

    /// Whether nothing is registered (or nothing loaded).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.loaded.profiles.is_empty()
    }

    /// The request-level warnings: every refused entry's, then one per claim
    /// of each conflicting host. Independent of any document.
    #[must_use]
    pub fn diagnostics(&self) -> Vec<RegistrationDiagnostic> {
        let mut diagnostics = self.loaded.diagnostics.clone();
        diagnostics.extend(self.claims.iter().flat_map(HostClaim::diagnostics));
        diagnostics
    }

    /// Whether `host` is a registered (installed) host, usable or not.
    #[must_use]
    pub fn is_installed(&self, host: &str) -> bool {
        self.claims.iter().any(|claim| claim.host() == host)
    }

    /// Which registered host identifies a document with `frontmatter`, the
    /// file `document` (when it has one) judged in its own `context`: never
    /// [`HostSelection::Legacy`]. `$path` predicates read bare and `./`
    /// patterns from `context`'s tree root ([`FileResolutionContext::base_dir`]),
    /// as for a user-level schema root.
    #[must_use]
    pub fn identify(&self, frontmatter: &Value, document: Option<&Path>, context: &FileResolutionContext) -> Selected {
        let subject = || match document {
            Some(document) => PathSubject::new(document, context).with_pattern_cwd(context.base_dir()),
            None => PathSubject::detached(),
        };
        let identifies_document =
            |profile: &RegisteredHostProfile| identifies(profile.identifying(), frontmatter, &subject());
        let mut hosts: Vec<String> = Vec::new();
        let mut agreed: Vec<&Arc<RegisteredHostProfile>> = Vec::new();
        let mut diagnostics = Vec::new();
        for claim in &self.claims {
            match claim {
                HostClaim::Agreed { profile, .. } => {
                    if identifies_document(profile) {
                        hosts.push(profile.host().to_string());
                        agreed.push(profile);
                    }
                }
                HostClaim::Conflict { host, claims } => {
                    if claims.iter().any(|claim| identifies_document(claim)) {
                        hosts.push(host.clone());
                        diagnostics.extend(claim.diagnostics());
                    }
                }
            }
        }
        if hosts.is_empty() {
            return Selected::quiet(HostSelection::Md);
        }
        if diagnostics.is_empty() && agreed.len() == 1 {
            return Selected::quiet(HostSelection::Registered(Arc::clone(agreed[0])));
        }
        if hosts.len() > 1
            && let Some(entry) = self.first_entry_of(&hosts)
        {
            let named: Vec<String> = hosts.iter().map(|host| format!("`{host}`")).collect();
            let file = document.map_or_else(|| entry.declared_in.clone(), Path::to_path_buf);
            diagnostics.push(RegistrationDiagnostic {
                code: HOST_AMBIGUOUS_CODE,
                severity: Severity::Warning,
                message: format!(
                    "hosts {} all identify this document; it is analyzed as an `md` document and no host's schemas apply",
                    named.join(", ")
                ),
                file,
                line: None,
                column: None,
                entry,
                member: None,
            });
        }
        Selected { selection: HostSelection::Ambiguous(hosts), diagnostics }
    }

    fn first_entry_of(&self, hosts: &[String]) -> Option<super::Registration> {
        self.loaded
            .profiles
            .iter()
            .find(|profile| hosts.iter().any(|host| host == profile.host()))
            .and_then(|profile| profile.entries().first().cloned())
    }

    /// The files of every registered profile's global and function catalogs,
    /// which the legacy view never serves.
    fn registered_catalog_files(&self) -> HashSet<PathIdentity> {
        self.loaded
            .profiles
            .iter()
            .flat_map(|profile| {
                std::iter::once(profile.globals_path().to_path_buf())
                    .chain(profile.functions().iter().map(|set| set.catalog.clone()))
            })
            .map(|path| identity_of(&path))
            .collect()
    }
}

/// Selects the host of a document: [`RegisteredHosts::identify`] when
/// `registered` is given, then, only when no registered host identified it,
/// the legacy root-catalog view ([`LegacyHost::discover`]) without the
/// catalogs of any registered host. [`HostSelection::Md`] when the legacy
/// view is empty.
#[must_use]
pub fn select(
    registered: Option<&RegisteredHosts>,
    frontmatter: &Value,
    document: Option<&Path>,
    context: &FileResolutionContext,
) -> Selected {
    if let Some(registered) = registered {
        let identified = registered.identify(frontmatter, document, context);
        if !matches!(identified.selection, HostSelection::Md) {
            return identified;
        }
    }
    let legacy = LegacyHost::discover(context, registered);
    if legacy.is_empty() {
        Selected::quiet(HostSelection::Md)
    } else {
        Selected::quiet(HostSelection::Legacy(Arc::new(legacy)))
    }
}

/// The compatibility host of a document no registered host identifies: the
/// host catalogs in its schema roots, as passive readers found them before
/// registration, gathered in one place. Catalogs that belong to a
/// registered host are left out.
#[derive(Debug, Default)]
pub struct LegacyHost {
    globals: Vec<(PathBuf, Arc<GlobalCatalog>)>,
    catalog_errors: Vec<CatalogError>,
    functions: Vec<(PathBuf, HostFunctions)>,
    function_errors: Vec<HostFunctionCatalogError>,
    lifecycle: Option<Arc<HostLifecycleProfile>>,
    incomplete: Vec<String>,
}

#[allow(missing_docs)]
impl LegacyHost {
    /// The legacy view of `context`'s schema roots. A root host catalog is
    /// left out when its lifecycle artifact records a host `registered`
    /// knows, or when it is a registered profile's catalog file; a function
    /// catalog is left out when it sits beside such a catalog or is a
    /// registered profile's function catalog. The lifecycle profile is the
    /// first usable one that is not left out.
    #[must_use]
    pub fn discover(context: &FileResolutionContext, registered: Option<&RegisteredHosts>) -> Self {
        let mut excluded = registered.map(RegisteredHosts::registered_catalog_files).unwrap_or_default();
        let mut excluded_folders: HashSet<PathIdentity> = HashSet::new();
        let mut lifecycle = None;
        let mut incomplete = Vec::new();
        for discovery in host_lifecycle_profiles(context) {
            let catalog = identity_of(&discovery.catalog);
            let installed = discovery
                .profile
                .as_ref()
                .is_ok_and(|profile| registered.is_some_and(|hosts| hosts.is_installed(profile.registry().host())));
            if installed || excluded.contains(&catalog) {
                excluded.insert(catalog);
                if let Some(folder) = discovery.catalog.parent() {
                    excluded_folders.insert(identity_of(folder));
                }
                continue;
            }
            if lifecycle.is_some() {
                continue;
            }
            match discovery.profile {
                Ok(profile) => lifecycle = Some(profile),
                Err(reason) => incomplete.push(reason),
            }
        }
        if lifecycle.is_some() {
            incomplete.clear();
        }

        let mut host = Self { lifecycle, incomplete, ..Self::default() };
        for (path, catalog) in host_catalog_files(context) {
            if excluded.contains(&identity_of(&path)) {
                continue;
            }
            match catalog {
                Ok(catalog) => host.globals.push((path, Arc::new(catalog))),
                Err(error) => host.catalog_errors.push(error),
            }
        }
        for (path, set) in host_function_catalog_files(context) {
            let beside_excluded = path.parent().is_some_and(|folder| excluded_folders.contains(&identity_of(folder)));
            if beside_excluded || excluded.contains(&identity_of(&path)) {
                continue;
            }
            match set {
                Ok(set) => host.functions.push((path, set)),
                Err(error) => host.function_errors.push(error),
            }
        }
        host
    }

    /// Whether the view holds nothing: no catalog, function set, lifecycle
    /// profile, or reported failure.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.globals.is_empty()
            && self.catalog_errors.is_empty()
            && self.functions.is_empty()
            && self.function_errors.is_empty()
            && self.lifecycle.is_none()
            && self.incomplete.is_empty()
    }

    /// The host global catalogs, each with its file, in root order.
    #[must_use]
    pub fn globals(&self) -> &[(PathBuf, Arc<GlobalCatalog>)] {
        &self.globals
    }

    /// The host catalogs that did not load.
    #[must_use]
    pub fn catalog_errors(&self) -> &[CatalogError] {
        &self.catalog_errors
    }

    /// The analysis-only function sets, each with its catalog file.
    #[must_use]
    pub fn functions(&self) -> &[(PathBuf, HostFunctions)] {
        &self.functions
    }

    #[must_use]
    pub fn function_errors(&self) -> &[HostFunctionCatalogError] {
        &self.function_errors
    }

    /// The first usable lifecycle profile.
    #[must_use]
    pub const fn lifecycle(&self) -> Option<&Arc<HostLifecycleProfile>> {
        self.lifecycle.as_ref()
    }

    /// Why each considered catalog's lifecycle artifact was not used, when
    /// none was.
    #[must_use]
    pub fn incomplete(&self) -> &[String] {
        &self.incomplete
    }
}

/// `path`'s identity for comparison: canonical when it exists.
fn identity_of(path: &Path) -> PathIdentity {
    PathIdentity::new(&canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf()))
}
