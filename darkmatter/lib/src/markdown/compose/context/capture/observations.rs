//! Structured request observations that parameterized expression functions
//! read after capture.
//!
//! `ctx.*` projects JSON values; `package()`, `package_area()`, `ipv4()`,
//! `ipv6()`, and `has_agentic_cli()` need the observations behind them
//! (package roots with names, scoped interface addresses, and installed
//! agentic CLIs). These are retained from the same capture as the `Repo`,
//! `Network`, and `Agent` groups so a function never rediscovers topology,
//! re-enumerates interfaces, or rescans `PATH`.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use sniff::filesystem::repo::RepoInfo;
use sniff::network::ScopedIpAddr;
use sniff::programs::{ExecutableIndex, InstalledAiClients};

use super::super::repository_scope::{PackageLookup, RepositoryObservation};
use super::{ContextGroup, ContextRequirements};

/// Observations retained for expression functions.
///
/// Each part is `None` until its owning group (`Repo`, `Network`, `Agent`) is
/// captured, so a function can distinguish a request that never planned the
/// capture from a capture that observed nothing.
#[derive(Debug, Clone, Default)]
pub(crate) struct CapturedObservations {
    packages: Option<Arc<PackageLookup>>,
    /// The root and topology the `Repo` capture observed, behind `packages`.
    repository: Option<Arc<RepositoryObservation>>,
    addresses: Option<Arc<[ScopedIpAddr]>>,
    /// Scanned from `PATH` on first use rather than at capture, so `ctx.agent`
    /// never pays for it; every clone shares the one scan, so all calls in a
    /// request observe the same answer.
    agentic_clis: Option<Arc<OnceLock<InstalledAiClients>>>,
}

impl CapturedObservations {
    pub(super) fn with_packages(mut self, root: Option<&Path>, repo: Option<&RepoInfo>) -> Self {
        self.packages = Some(Arc::new(PackageLookup::new(root, repo)));
        self.repository = Some(Arc::new(RepositoryObservation {
            root: root.map(Path::to_path_buf),
            info: repo.cloned(),
        }));
        self
    }

    /// `addresses` is `None` when enumeration failed; the capture already
    /// recorded that diagnostic, so the observation is an empty address set.
    pub(super) fn with_addresses(mut self, addresses: Option<&[ScopedIpAddr]>) -> Self {
        self.addresses = Some(addresses.unwrap_or_default().into());
        self
    }

    pub(super) fn with_agentic_clis(mut self) -> Self {
        self.agentic_clis = Some(Arc::default());
        self
    }

    #[cfg(test)]
    pub(crate) fn for_test_agentic_clis(installed: InstalledAiClients) -> Self {
        let observed = OnceLock::new();
        let _ = observed.set(installed);
        Self { agentic_clis: Some(Arc::new(observed)), ..Self::default() }
    }

    #[cfg(test)]
    pub(crate) fn for_test_addresses(addresses: &[ScopedIpAddr]) -> Self {
        Self::default().with_addresses(Some(addresses))
    }

    #[cfg(test)]
    pub(crate) fn for_test_packages(root: &Path, repo: &RepoInfo) -> Self {
        Self::default().with_packages(Some(root), Some(repo))
    }

    /// The package lookup, or `None` when the `Repo` group was not captured.
    pub(crate) fn packages(&self) -> Option<&PackageLookup> {
        self.packages.as_deref()
    }

    /// The repository observation, or `None` when the `Repo` group was not
    /// captured.
    pub(crate) fn repository(&self) -> Option<&Arc<RepositoryObservation>> {
        self.repository.as_ref()
    }

    /// Stable, deduplicated host addresses, or `None` when the `Network` group
    /// was not captured.
    pub(crate) fn addresses(&self) -> Option<&[ScopedIpAddr]> {
        self.addresses.as_deref()
    }

    /// Installed agentic CLIs on the `PATH` index, or `None` when the `Agent`
    /// group was not captured.
    pub(crate) fn agentic_clis(&self) -> Option<&InstalledAiClients> {
        self.agentic_clis.as_deref().map(|installed| {
            installed.get_or_init(|| InstalledAiClients::new_with_index(&ExecutableIndex::build_path_only()))
        })
    }

    /// Fills each part owned by one of `groups` that this set lacks from
    /// `other`, never replacing a captured part.
    pub(crate) fn fill_from(&mut self, other: &Self, groups: &ContextRequirements) {
        if self.packages.is_none() && groups.contains(ContextGroup::Repo) {
            self.packages.clone_from(&other.packages);
            self.repository.clone_from(&other.repository);
        }
        if self.addresses.is_none() && groups.contains(ContextGroup::Network) {
            self.addresses.clone_from(&other.addresses);
        }
        if self.agentic_clis.is_none() && groups.contains(ContextGroup::Agent) {
            self.agentic_clis.clone_from(&other.agentic_clis);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_halves_are_never_replaced_by_a_later_fill() {
        let address: ScopedIpAddr = "192.168.10.5".parse().unwrap();
        let mut first = CapturedObservations::default().with_addresses(Some(std::slice::from_ref(&address)));
        let second = CapturedObservations::default()
            .with_addresses(Some(&[]))
            .with_packages(None, None);
        first.fill_from(&second, &ContextRequirements::from_groups([ContextGroup::Network]));
        assert_eq!(first.addresses(), Some(&[address.clone()][..]));
        assert!(first.packages().is_none(), "Repo was not among the filled groups");
        first.fill_from(&second, &ContextRequirements::all());
        assert_eq!(first.addresses(), Some(&[address][..]));
        assert!(first.packages().is_some());
    }
}
