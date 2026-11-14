//! The effective `network` permission of a captured configuration, and the
//! one fetch policy it projects into.
//!
//! Composition (`::file`, `::code`, read-side functions) and schema-slot URL
//! reads both take their host consent from [`NetworkPermission::fetch_policy`],
//! so the two surfaces never decide a URL differently under the same
//! configuration. User guide: `darkmatter/docs/topics/configuration.md`.

use biscuit_file::file_reference::fetch::{FetchPolicy, HostPattern};

use super::load::{ConfigError, ConfigSnapshot, NetworkAccess};

/// What a request's configuration says about fetching URLs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum NetworkPermission {
    /// Every HTTP(S) host may be fetched.
    Allow,
    /// Only the per-run `--allow host=HOST` hosts may be fetched.
    #[default]
    Deny,
    /// The `permissions` block is invalid, so no blanket consent exists;
    /// a read of a host no `--allow host=HOST` names reports this error.
    Invalid(ConfigError),
}

impl NetworkPermission {
    /// The permission `snapshot` grants; no snapshot is [`Self::Deny`].
    #[must_use]
    pub fn from_snapshot(snapshot: Option<&ConfigSnapshot>) -> Self {
        match snapshot.map(ConfigSnapshot::policy) {
            None => Self::Deny,
            Some(Ok(policy)) => match policy.network() {
                NetworkAccess::Allow => Self::Allow,
                NetworkAccess::Deny => Self::Deny,
            },
            Some(Err(error)) => Self::Invalid(error),
        }
    }

    /// Whether every host may be fetched without a per-run host.
    #[must_use]
    pub const fn allows_every_host(&self) -> bool {
        matches!(self, Self::Allow)
    }

    /// The configuration error that left the permission undecided.
    #[must_use]
    pub const fn config_error(&self) -> Option<&ConfigError> {
        match self {
            Self::Invalid(error) => Some(error),
            Self::Allow | Self::Deny => None,
        }
    }

    /// The fetch policy for this permission plus the exact `hosts` a run
    /// allows on top: [`FetchPolicy::allow_all`] under [`Self::Allow`], else
    /// deny-all widened by each host.
    #[must_use]
    pub fn fetch_policy<I, S>(&self, hosts: I) -> FetchPolicy
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if self.allows_every_host() {
            return FetchPolicy::allow_all();
        }
        hosts
            .into_iter()
            .fold(FetchPolicy::deny_all(), |policy, host| policy.allow(HostPattern::Exact(host.as_ref().to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_snapshot_denies() {
        assert_eq!(NetworkPermission::from_snapshot(None), NetworkPermission::Deny);
    }

    #[test]
    fn allow_projects_to_a_blanket_policy_and_deny_to_exact_hosts() {
        let allow = NetworkPermission::Allow.fetch_policy(["a.test"]);
        assert!(allow.allows_every_host());

        let deny = NetworkPermission::Deny.fetch_policy(["a.test"]);
        assert!(!deny.allows_every_host());
        assert!(deny.is_allowed("A.test"));
        assert!(!deny.is_allowed("b.test"));
    }
}
