//! The effective `network` permission of a captured configuration, and the
//! one fetch policy it projects into.
//!
//! Composition (`::file`, `::code`, read-side functions) and schema-slot URL
//! reads both take their host consent from [`NetworkPermission::fetch_policy`],
//! so the two surfaces never decide a URL differently under the same
//! configuration. A refused read names the remedy of the program composing
//! ([`NetworkRemedy`]), which a host without `md`'s per-run `--allow
//! host=HOST` installs once ([`install_network_remedy`]). User guide:
//! `darkmatter/docs/topics/configuration.md`.

use biscuit_file::file_reference::fetch::{FetchPolicy, HostPattern};

use super::load::{ConfigError, ConfigSnapshot, NetworkAccess};

/// What a request's configuration says about fetching URLs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum NetworkPermission {
    /// Every HTTP(S) host may be fetched.
    Allow,
    /// Only the per-run `--allow host=HOST` hosts may be fetched without an
    /// approval; any other URL needs one. A schema slot asks for it; a
    /// composition read, which has no prompt, and a dry run, which asks
    /// nothing, report the approval as needed.
    Ask,
    /// Only the per-run `--allow host=HOST` hosts may be fetched.
    #[default]
    Deny,
    /// The `permissions` block is invalid: no URL may be fetched, not even
    /// a per-run `--allow host=HOST` host, and every refused read reports
    /// this error.
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
                NetworkAccess::Ask => Self::Ask,
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

    /// Whether a URL no per-run host allows needs an approval (`ask`)
    /// rather than being refused (`deny`).
    #[must_use]
    pub const fn requires_approval(&self) -> bool {
        matches!(self, Self::Ask)
    }

    /// The configuration error that left the permission undecided.
    #[must_use]
    pub const fn config_error(&self) -> Option<&ConfigError> {
        match self {
            Self::Invalid(error) => Some(error),
            Self::Allow | Self::Ask | Self::Deny => None,
        }
    }

    /// The fetch policy for this permission plus the exact `hosts` a run
    /// allows on top: [`FetchPolicy::allow_all`] under [`Self::Allow`],
    /// deny-all widened by each host under [`Self::Ask`] and [`Self::Deny`]
    /// (which differ only in how a refused URL is reported), and deny-all under
    /// [`Self::Invalid`], whose configuration error outranks every per-run
    /// host.
    #[must_use]
    pub fn fetch_policy<I, S>(&self, hosts: I) -> FetchPolicy
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        match self {
            Self::Allow => return FetchPolicy::allow_all(),
            Self::Invalid(_) => return FetchPolicy::deny_all(),
            Self::Ask | Self::Deny => {}
        }
        hosts
            .into_iter()
            .fold(FetchPolicy::deny_all(), |policy, host| policy.allow(HostPattern::Exact(host.as_ref().to_string())))
    }
}

/// What a refused remote read tells its reader to do: the remedies the
/// composing program supports. Every network refusal's text (a slot read
/// under `deny`, a composition read under `ask`) is worded from the
/// installed remedy ([`network_remedy`]), never a fixed flag.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum NetworkRemedy {
    /// `md`'s: a per-run `--allow host=HOST`, or the `network` setting.
    #[default]
    PerRunHost,
    /// A host with no per-run host allowance (Claudine): the `network`
    /// setting alone.
    Setting,
}

static INSTALLED_REMEDY: std::sync::OnceLock<NetworkRemedy> = std::sync::OnceLock::new();

/// Installs the process's [`NetworkRemedy`]; a host calls this once at
/// startup, before composing. The first installation wins: a later call
/// returns `Err` with the remedy already installed.
///
/// ## Errors
///
/// The remedy installed earlier, when one was.
pub fn install_network_remedy(remedy: NetworkRemedy) -> Result<(), NetworkRemedy> {
    INSTALLED_REMEDY.set(remedy).map_err(|_| network_remedy())
}

/// The installed [`NetworkRemedy`]; [`NetworkRemedy::PerRunHost`] (`md`'s)
/// when no host installed one.
#[must_use]
pub fn network_remedy() -> NetworkRemedy {
    INSTALLED_REMEDY.get().copied().unwrap_or_default()
}

impl NetworkRemedy {
    /// The remedy clause for a URL `permissions.network: deny` refused.
    #[must_use]
    pub fn when_denied(self) -> &'static str {
        match self {
            Self::PerRunHost => "allow the host for this run (`--allow host=HOST`), or set `permissions.network` to `ask` or `allow`",
            Self::Setting => "set `permissions.network` to `ask` or `allow`",
        }
    }

    /// The remedy clause for `host`, which `permissions.network: ask` left
    /// needing an approval a composition does not ask for.
    #[must_use]
    pub fn when_approval_needed(self, host: &str) -> String {
        match self {
            Self::PerRunHost => format!("allow the host for this run (`--allow host={host}`) or set `permissions.network: allow`"),
            Self::Setting => "set `permissions.network: allow`".to_string(),
        }
    }

    /// [`Self::when_approval_needed`] as a status block's hint sentence.
    #[must_use]
    pub fn approval_hint(self, host: &str) -> String {
        match self {
            Self::PerRunHost => format!("Allow the host for this run (`--allow host={host}`), or set `permissions.network: allow`."),
            Self::Setting => "Set `permissions.network: allow`.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_remedy_names_only_what_its_host_supports() {
        assert!(NetworkRemedy::PerRunHost.when_denied().contains("--allow host=HOST"));
        assert!(NetworkRemedy::PerRunHost.when_approval_needed("a.test").contains("--allow host=a.test"));
        assert!(!NetworkRemedy::Setting.when_denied().contains("--allow"));
        assert!(!NetworkRemedy::Setting.when_approval_needed("a.test").contains("--allow"));
        assert!(NetworkRemedy::Setting.when_approval_needed("a.test").contains("permissions.network: allow"));
        assert!(NetworkRemedy::PerRunHost.approval_hint("a.test").contains("--allow host=a.test"));
        assert!(!NetworkRemedy::Setting.approval_hint("a.test").contains("--allow"));
    }

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

    #[test]
    fn ask_projects_like_deny_and_alone_requires_approval() {
        let ask = NetworkPermission::Ask.fetch_policy(["a.test"]);
        assert!(!ask.allows_every_host());
        assert!(ask.is_allowed("a.test"));
        assert!(!ask.is_allowed("b.test"));
        assert!(NetworkPermission::Ask.requires_approval());
        assert!(!NetworkPermission::Deny.requires_approval());
        assert!(!NetworkPermission::Allow.requires_approval());
        assert_eq!(NetworkPermission::Ask.config_error(), None);
    }

    #[test]
    fn invalid_projects_to_deny_all_whatever_the_run_allows() {
        use crate::config::load::{ConfigErrorKind, ConfigRevision};
        let error = ConfigError::unlocated(
            ConfigErrorKind::InvalidSetting,
            std::path::Path::new(".darkmatter.yaml"),
            ConfigRevision::User,
            "network: invalid".to_string(),
        );
        let invalid = NetworkPermission::Invalid(error).fetch_policy(["a.test"]);
        assert!(!invalid.allows_every_host());
        assert!(!invalid.is_allowed("a.test"));
    }
}
