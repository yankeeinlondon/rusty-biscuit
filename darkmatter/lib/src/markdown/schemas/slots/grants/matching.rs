//! The approvals in force for one run, and how a read location is matched
//! against them.

use super::{Grant, GrantDecision, GrantScope, GrantTarget};
use crate::config::network::NetworkPermission;
use crate::markdown::compose::expression::read_gate::ReadLocation;

/// The approvals one run decides reads by beyond `permissions.file-read`:
/// the approvals given for this run, the command's `--allow host=HOST`
/// values, and the effective `network` permission.
///
/// A host value allows every `http` and `https` URL on any port whose host
/// equals it (ASCII case-insensitively) for this run; `network: allow`
/// allows every `http` and `https` URL. Neither ever denies, so a deny for
/// this run still wins.
///
/// Locations passed to [`decide`](Self::decide) must already be canonical
/// where they name an existing local path; the book compares paths by
/// component and never touches the filesystem.
#[derive(Debug, Clone, Default)]
pub struct GrantBook {
    once: Vec<Grant>,
    hosts: Vec<String>,
    network: NetworkPermission,
}

impl GrantBook {
    /// The book for the command's `--allow host=` values.
    pub fn new(hosts: Vec<String>) -> Self {
        Self { once: Vec::new(), hosts, network: NetworkPermission::Deny }
    }

    /// Decides URL reads under the effective `network` permission too.
    #[must_use]
    pub fn with_network(mut self, network: NetworkPermission) -> Self {
        self.network = network;
        self
    }

    /// The effective `network` permission.
    pub fn network(&self) -> &NetworkPermission {
        &self.network
    }

    /// No approvals and no hosts.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Applies an approval for the rest of this run; an "always allow"
    /// ([`GrantScope::Persisted`]) applies as an allow for this run. A grant
    /// naming a folder, a URL whatever its query, or a site is ignored.
    pub fn add_once(&mut self, decision: GrantDecision, target: GrantTarget) {
        if !target.is_approvable() {
            return;
        }
        let grant = Grant { decision, target, scope: GrantScope::Once };
        if !self.once.contains(&grant) {
            self.once.push(grant);
        }
    }

    /// The approvals for this run.
    pub fn once(&self) -> &[Grant] {
        &self.once
    }

    /// The command's `--allow host=HOST` values.
    pub fn hosts(&self) -> &[String] {
        &self.hosts
    }

    /// The decision this run's approvals make for `location`: `Deny` when a
    /// deny covers it (whatever allows it too), else `Allow` when an allow
    /// covers it, else `None`. Host values and the network permission are
    /// not consulted; see [`Self::network_allows`].
    pub fn decide(&self, location: &ReadLocation) -> Option<GrantDecision> {
        let covered = |decision| self.once.iter().any(|grant| grant.decision == decision && covers(&grant.target, location));
        if covered(GrantDecision::Deny) {
            Some(GrantDecision::Deny)
        } else if covered(GrantDecision::Allow) {
            Some(GrantDecision::Allow)
        } else {
            None
        }
    }

    /// Whether a host value or `network: allow` allows the `http`/`https`
    /// URL at `location`; never under an invalid network permission.
    pub fn network_allows(&self, location: &ReadLocation) -> bool {
        let ReadLocation::Remote(url) = location else {
            return false;
        };
        self.network.config_error().is_none()
            && matches!(url.scheme(), "http" | "https")
            && (self.network.allows_every_host()
                || url.host_str().is_some_and(|host| self.hosts.iter().any(|allowed| allowed.eq_ignore_ascii_case(host))))
    }
}

fn covers(target: &GrantTarget, location: &ReadLocation) -> bool {
    match (target, location) {
        (GrantTarget::File(file), ReadLocation::Local(path)) => path == file,
        (GrantTarget::Url(granted), ReadLocation::Remote(url)) => url.as_str() == granted.as_str(),
        _ => false,
    }
}
