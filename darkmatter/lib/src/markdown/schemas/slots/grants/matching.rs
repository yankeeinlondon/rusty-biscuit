//! The grants in force for one run, and how a read location is matched
//! against them.

use std::path::{Component, Path, PathBuf};

use super::{Grant, GrantDecision, GrantScope, GrantStoreWriteError, GrantTarget, GrantStore};
use crate::markdown::compose::expression::read_gate::ReadLocation;

/// The grants one run decides reads by: the persisted store, the grants
/// given for this run, and the command's `--allow host=HOST` values.
///
/// A host value allows every `http` and `https` URL on any port whose host
/// equals it (ASCII case-insensitively) for this run; it never denies.
///
/// Locations passed to [`decide`](Self::decide) must already be canonical
/// where they name an existing local path; the book compares paths by
/// component and never touches the filesystem.
#[derive(Debug, Clone, Default)]
pub struct GrantBook {
    store: GrantStore,
    once: Vec<Grant>,
    hosts: Vec<String>,
}

impl GrantBook {
    /// The book for a loaded store and the command's `--allow host=` values.
    pub fn new(store: GrantStore, hosts: Vec<String>) -> Self {
        Self { store, once: Vec::new(), hosts }
    }

    /// No grants, no store, and no hosts.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Adds a grant for this run only.
    pub fn add_once(&mut self, decision: GrantDecision, target: GrantTarget) {
        let grant = Grant { decision, target, scope: GrantScope::Once };
        if !self.once.contains(&grant) {
            self.once.push(grant);
        }
    }

    /// Writes a grant to the store file and applies it from now on.
    ///
    /// ## Errors
    ///
    /// Whatever [`GrantStore::append`] reports; the grant then does not apply.
    pub fn persist(&mut self, decision: GrantDecision, target: GrantTarget) -> Result<(), GrantStoreWriteError> {
        self.store.append(decision, target)
    }

    /// The store the persisted grants came from.
    pub fn store(&self) -> &GrantStore {
        &self.store
    }

    /// The persisted grants.
    pub fn persisted(&self) -> &[Grant] {
        self.store.grants()
    }

    /// The grants for this run only.
    pub fn once(&self) -> &[Grant] {
        &self.once
    }

    /// The command's `--allow host=HOST` values.
    pub fn hosts(&self) -> &[String] {
        &self.hosts
    }

    /// The decision for a read of `location`: `Deny` when any deny grant
    /// covers it (whatever allows it too), else `Allow` when an allow grant
    /// or host value covers it, else `None`.
    ///
    /// A glob is allowed only by a `dir` grant holding its literal leading
    /// folders (an absolute pattern without `.` or `..`), and is denied by any
    /// deny `file` or `dir` grant its matches could reach: one under the
    /// pattern's literal leading folders, or one holding them. A relative glob
    /// or one with `.`/`..` could reach anything, so every deny `file` or
    /// `dir` grant denies it.
    pub fn decide(&self, location: &ReadLocation) -> Option<GrantDecision> {
        let grants = || self.store.grants().iter().chain(&self.once);
        if grants().any(|grant| grant.decision == GrantDecision::Deny && deny_covers(&grant.target, location)) {
            return Some(GrantDecision::Deny);
        }
        let allowed = grants().any(|grant| grant.decision == GrantDecision::Allow && allow_covers(&grant.target, location))
            || self.host_allows(location);
        allowed.then_some(GrantDecision::Allow)
    }

    fn host_allows(&self, location: &ReadLocation) -> bool {
        let ReadLocation::Remote(url) = location else {
            return false;
        };
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some_and(|host| self.hosts.iter().any(|allowed| allowed.eq_ignore_ascii_case(host)))
    }
}

fn allow_covers(target: &GrantTarget, location: &ReadLocation) -> bool {
    match location {
        ReadLocation::Glob(pattern) => match (target, glob_region(pattern)) {
            (GrantTarget::Dir(dir), Some(region)) => region.starts_with(dir),
            _ => false,
        },
        _ => covers(target, location),
    }
}

fn deny_covers(target: &GrantTarget, location: &ReadLocation) -> bool {
    match location {
        ReadLocation::Glob(pattern) => match (target, glob_region(pattern)) {
            (GrantTarget::Dir(path) | GrantTarget::File(path), Some(region)) => {
                region.starts_with(path) || path.starts_with(&region)
            }
            (GrantTarget::Dir(_) | GrantTarget::File(_), None) => true,
            _ => false,
        },
        _ => covers(target, location),
    }
}

fn covers(target: &GrantTarget, location: &ReadLocation) -> bool {
    match (target, location) {
        (GrantTarget::File(file), ReadLocation::Local(path)) => path == file,
        (GrantTarget::Dir(dir), ReadLocation::Local(path)) => path.starts_with(dir),
        (GrantTarget::Url(granted), ReadLocation::Remote(url)) => url.as_str() == granted.as_str(),
        (GrantTarget::UrlAnyQuery(granted), ReadLocation::Remote(url)) => {
            url.scheme() == granted.scheme()
                && url.host_str() == granted.host_str()
                && url.port_or_known_default() == granted.port_or_known_default()
                && url.path() == granted.path()
        }
        (GrantTarget::Site { scheme, host, port }, ReadLocation::Remote(url)) => {
            url.scheme() == scheme
                && url.host_str().is_some_and(|url_host| url_host.eq_ignore_ascii_case(host))
                && url.port_or_known_default() == port.or_else(|| default_port(scheme))
        }
        _ => false,
    }
}

fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    }
}

/// The literal leading folders of an absolute glob pattern: its components
/// up to the first one holding a wildcard. `None` when the pattern is
/// relative or names `.` or `..`, so its reach is unknown.
fn glob_region(pattern: &str) -> Option<PathBuf> {
    let path = Path::new(pattern);
    if !path.is_absolute() {
        return None;
    }
    if path.components().any(|component| matches!(component, Component::CurDir | Component::ParentDir)) {
        return None;
    }
    let mut region = PathBuf::new();
    for component in path.components() {
        if component.as_os_str().to_string_lossy().contains(['*', '?', '[', ']', '{', '}']) {
            break;
        }
        region.push(component);
    }
    Some(region)
}
