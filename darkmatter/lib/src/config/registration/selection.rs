//! Host selection: which host, if any, a document belongs to.
//!
//! [`RegisteredHosts`] holds one request's loaded registrations, grouped by
//! [`agreement`] once. [`RegisteredHosts::identify`] matches every usable
//! host's registered triggers against a document's authored frontmatter and
//! path, with ordinary trigger semantics; the same match decides which
//! triggers' payloads apply. A document no host's trigger matches is an `md`
//! document: nothing in a schema root ever names a host. Selection is
//! passive: it reads no file, writes nothing, fetches nothing, and prompts
//! for nothing.
//!
//! User-facing guide: `darkmatter/docs/topics/host-registration.md`
//! ("Selection").

use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use serde_json::Value;
use simplified_schema::triggers::PathSubject;
use simplified_schema::yaml_documents::{RegisteredDefinition, RegisteredKinds};

use super::{
    HOST_AMBIGUOUS_CODE, HostClaim, LoadedRegistrations, RegisteredHostProfile, RegistrationDiagnostic, agreement,
    load_registrations, md_owned_names,
};
use crate::config::load::ConfigSnapshot;
use crate::markdown::compose::expression::Severity;

/// The host a document is analyzed as.
#[derive(Debug, Clone)]
pub enum HostSelection {
    /// No host: the document is an `md` document. Also the result when a
    /// caller supplied its own executable host options, which registration
    /// never replaces.
    Md,
    /// Exactly one registered host's triggers matched the document: its
    /// matched triggers, globals, functions, and lifecycle apply together.
    Registered(Arc<RegisteredHostProfile>),
    /// More than one host's triggers matched the document, or those of a host
    /// whose registrations conflict did. The document is analyzed as `md`.
    /// Holds the host identifiers, in registration order.
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
    /// For [`HostSelection::Registered`], the indices into the profile's
    /// [`triggers`](RegisteredHostProfile::triggers) that matched, ascending
    /// (bundle-path order); empty otherwise.
    pub matched: Vec<usize>,
    /// `dm.host.ambiguous` when two hosts identified the document; the
    /// conflicting claims' `dm.host.registration` warnings when a conflicted
    /// host did. The same conflict warnings are also part of
    /// [`RegisteredHosts::diagnostics`], so a caller reporting both should
    /// report each distinct record once.
    pub diagnostics: Vec<RegistrationDiagnostic>,
}

impl Selected {
    /// No host: an `md` document.
    #[must_use]
    pub const fn md() -> Self {
        Self { selection: HostSelection::Md, matched: Vec::new(), diagnostics: Vec::new() }
    }

    /// The selected profile's matched triggers, in bundle-path order; empty
    /// unless one host was selected.
    pub fn matched_triggers(&self) -> impl Iterator<Item = &super::RegisteredTrigger> {
        let triggers = self.selection.registered().map_or(&[][..], |profile| profile.triggers());
        self.matched.iter().filter_map(move |index| triggers.get(*index))
    }
}

/// One request's registrations, ready for selection: loaded once
/// ([`load_registrations`]), grouped by host once ([`agreement`]).
#[derive(Debug)]
pub struct RegisteredHosts {
    loaded: Arc<LoadedRegistrations>,
    claims: Vec<HostClaim>,
    kinds: RegisteredKinds,
}

impl RegisteredHosts {
    /// Groups `loaded` for selection.
    #[must_use]
    pub fn new(loaded: Arc<LoadedRegistrations>) -> Self {
        let claims = agreement(&loaded.profiles);
        // A conflicting host's definitions stay indexed: its claims compete,
        // so a kind only it defines is a selection failure, never silently
        // one release's definition.
        let definitions: Vec<RegisteredDefinition> = claims
            .iter()
            .flat_map(|claim| match claim {
                HostClaim::Agreed { profile, .. } => std::slice::from_ref(profile),
                HostClaim::Conflict { claims, .. } => claims.as_slice(),
            })
            .flat_map(|profile| profile.yaml_kinds().iter().cloned())
            .collect();
        Self { loaded, claims, kinds: RegisteredKinds::new(definitions) }
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

    /// Every usable registered host's `kind: yaml-schema` definitions,
    /// indexed by the kinds they claim: the registered tier of YAML kind
    /// selection
    /// ([`select_with_registered`](simplified_schema::yaml_documents::select_with_registered)).
    #[must_use]
    pub const fn yaml_kinds(&self) -> &RegisteredKinds {
        &self.kinds
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

    /// Which registered host a document with `frontmatter` (authored, before
    /// defaults or coercion; a `$schema` control key is ignored), the file
    /// `document` (when it has one) judged in its own `context`, belongs to:
    /// the host one of whose triggers matches it, with ordinary trigger
    /// semantics (a `literal` value must be equal; a `$path` test reads bare
    /// and `./` patterns from `context`'s tree root,
    /// [`FileResolutionContext::base_dir`], as for a user-level schema root).
    /// [`Selected::matched`] lists the selected host's matched triggers.
    #[must_use]
    pub fn identify(&self, frontmatter: &Value, document: Option<&Path>, context: &FileResolutionContext) -> Selected {
        let frontmatter: Cow<'_, Value> = match frontmatter.as_object() {
            Some(map) if map.contains_key("$schema") => {
                let mut map = map.clone();
                map.remove("$schema");
                Cow::Owned(Value::Object(map))
            }
            _ => Cow::Borrowed(frontmatter),
        };
        let subject = match document {
            Some(document) => PathSubject::new(document, context).with_pattern_cwd(context.base_dir()),
            None => PathSubject::detached(),
        };
        let matched_by = |profile: &RegisteredHostProfile| -> Vec<usize> {
            profile
                .triggers()
                .iter()
                .enumerate()
                .filter(|(_, trigger)| trigger.trigger.envelope.conditions.matches(&frontmatter, &subject))
                .map(|(index, _)| index)
                .collect()
        };
        let mut hosts: Vec<String> = Vec::new();
        let mut agreed: Vec<(&Arc<RegisteredHostProfile>, Vec<usize>)> = Vec::new();
        let mut diagnostics = Vec::new();
        for claim in &self.claims {
            match claim {
                HostClaim::Agreed { profile, .. } => {
                    let matched = matched_by(profile);
                    if !matched.is_empty() {
                        hosts.push(profile.host().to_string());
                        agreed.push((profile, matched));
                    }
                }
                HostClaim::Conflict { host, claims } => {
                    if claims.iter().any(|claim| !matched_by(claim).is_empty()) {
                        hosts.push(host.clone());
                        diagnostics.extend(claim.diagnostics());
                    }
                }
            }
        }
        if hosts.is_empty() {
            return Selected::md();
        }
        if diagnostics.is_empty()
            && let [(profile, matched)] = agreed.as_mut_slice()
        {
            return Selected {
                selection: HostSelection::Registered(Arc::clone(profile)),
                matched: std::mem::take(matched),
                diagnostics: Vec::new(),
            };
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
                line_prefix: None,
                entry,
                member: None,
                related: Vec::new(),
            });
        }
        Selected { selection: HostSelection::Ambiguous(hosts), matched: Vec::new(), diagnostics }
    }

    fn first_entry_of(&self, hosts: &[String]) -> Option<super::Registration> {
        self.loaded
            .profiles
            .iter()
            .find(|profile| hosts.iter().any(|host| host == profile.host()))
            .and_then(|profile| profile.entries().first().cloned())
    }
}

/// Selects the host of a document: [`RegisteredHosts::identify`] when
/// `registered` is given, else [`HostSelection::Md`].
#[must_use]
pub fn select(
    registered: Option<&RegisteredHosts>,
    frontmatter: &Value,
    document: Option<&Path>,
    context: &FileResolutionContext,
) -> Selected {
    registered.map_or_else(Selected::md, |registered| registered.identify(frontmatter, document, context))
}
