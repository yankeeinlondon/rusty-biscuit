//! The user policy files under a policy root: where they are, and how one
//! approval or one runtime blacklist check loads them.

use std::path::Path;

use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::compose::shell_expansion::{
    ShellExpansionOptions, ShellPolicyPaths, ShellRuleSet, resolve_policy_paths, store,
};

use super::ShellApprovalOptions;
use crate::harness::error::HarnessError;

/// The shell policy files under `policy_root`.
///
/// Darkmatter reads an explicit policy root directly; the context it is
/// handed is anchored there and never consulted.
fn policy_paths_at(
    policy_root: &Path,
) -> Option<ShellPolicyPaths> {
    let shell_opts = ShellExpansionOptions {
        policy_root: Some(policy_root.to_path_buf()),
        ..Default::default()
    };
    let context = darkmatter::markdown::compose::build_resolution_context_with_catalog(
        &darkmatter::markdown::compose::RequestSnapshot::new(policy_root),
        None,
    )
    .ok()?;
    resolve_policy_paths(&shell_opts, &ComposeSource::File(policy_root.join("dummy")), &context).ok()
}

/// The user blacklist under `policy_root`, which a template-mode command's
/// filled arguments are checked against before it starts. Only the blacklist
/// file is read, and it is read again on every call, so an edit applies to
/// the next command. Empty without a policy root, when the root's policy
/// paths cannot be derived, or when the file is missing or unreadable; the
/// caller's built-in blacklist still applies. Approval denies an
/// underivable root instead.
pub fn user_blacklist(policy_root: Option<&Path>) -> ShellRuleSet {
    policy_root
        .and_then(policy_paths_at)
        .map(|paths| load_policy_file(&paths.blacklist))
        .unwrap_or_default()
}

/// One policy file's rules. A missing or unreadable file has none; a
/// malformed line is warned about and skipped by [`store::load_ruleset`],
/// and the file's other rules still load.
fn load_policy_file(path: &Path) -> ShellRuleSet {
    #[cfg(test)]
    let loaded = policy_reads::observe(path).and_then(|()| store::load_ruleset(path));
    #[cfg(not(test))]
    let loaded = store::load_ruleset(path);
    loaded.unwrap_or_default()
}

/// The user policy one approval reads: the rule files under the policy root,
/// or none when there is no root.
pub(super) struct SessionPolicy {
    pub(super) paths: Option<ShellPolicyPaths>,
    pub(super) whitelist: ShellRuleSet,
    pub(super) blacklist: ShellRuleSet,
}

impl SessionPolicy {
    /// Without a policy root there are no user policy files to consult: the
    /// built-in blacklist, the session cache, and the approval handler decide.
    pub(super) fn load(options: &ShellApprovalOptions, command: &str) -> Result<Self, HarnessError> {
        let paths = match &options.policy_root {
            Some(policy_root) => Some(policy_paths_at(policy_root).ok_or_else(|| {
                HarnessError::ShellCommandDenied {
                    command: command.to_string(),
                }
            })?),
            None => None,
        };
        let load = |path: Option<&Path>| path.map(load_policy_file).unwrap_or_default();
        Ok(Self {
            whitelist: load(paths.as_ref().map(|paths| paths.whitelist.as_path())),
            blacklist: load(paths.as_ref().map(|paths| paths.blacklist.as_path())),
            paths,
        })
    }
}

/// Records the policy files this thread loads and fails the loads a test
/// names, so a test proves which files a check read, and an unreadable file,
/// without relying on platform file permissions.
#[cfg(test)]
pub(crate) mod policy_reads {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    use darkmatter::markdown::compose::shell_expansion::ShellExpansionError;

    struct Observer {
        reads: Vec<PathBuf>,
        failing: Vec<PathBuf>,
    }

    thread_local! {
        static OBSERVER: RefCell<Option<Observer>> = const { RefCell::new(None) };
    }

    /// Observes this thread's policy loads until the guard drops. A load of
    /// a path in `failing` is recorded and then fails as unreadable.
    pub(crate) fn observe_this_thread(failing: &[PathBuf]) -> Guard {
        OBSERVER.with_borrow_mut(|observer| {
            *observer = Some(Observer { reads: Vec::new(), failing: failing.to_vec() });
        });
        Guard(())
    }

    pub(crate) struct Guard(());

    impl Guard {
        /// Every policy file loaded so far, in order, repeats included.
        pub(crate) fn reads(&self) -> Vec<PathBuf> {
            OBSERVER.with_borrow(|observer| observer.as_ref().map(|observer| observer.reads.clone()).unwrap_or_default())
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            OBSERVER.with_borrow_mut(|observer| *observer = None);
        }
    }

    pub(super) fn observe(path: &Path) -> Result<(), ShellExpansionError> {
        OBSERVER.with_borrow_mut(|observer| {
            let Some(observer) = observer else { return Ok(()) };
            observer.reads.push(path.to_path_buf());
            if observer.failing.iter().any(|failing| failing == path) {
                return Err(ShellExpansionError::PolicyIo {
                    path: path.to_path_buf(),
                    source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "injected read failure"),
                });
            }
            Ok(())
        })
    }
}
