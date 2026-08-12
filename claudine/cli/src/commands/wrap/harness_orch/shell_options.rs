use color_eyre::eyre::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) fn harness_policy_root(source_path: &Path, repo_root: Option<&Path>) -> Option<PathBuf> {
    let source_dir = source_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())?;

    if let Some(repo_root) = repo_root
        && source_path.starts_with(repo_root)
    {
        return Some(repo_root.to_path_buf());
    }

    if let Some(source_repo_root) = find_git_root(source_dir) {
        return Some(source_repo_root);
    }

    Some(source_dir.to_path_buf())
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start;

    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    None
}

#[allow(dead_code)]
pub(crate) fn build_harness_shell_options(
    source_path: &Path,
    repo_root: Option<&Path>,
) -> claudine::harness::ShellApprovalOptions {
    build_harness_shell_options_with_cache(source_path, repo_root, None)
}

/// Build shell approval policy from an already-derived source repository.
///
/// `None` is an observed non-repository source, so its directory is the policy
/// root. This path never searches ancestors for `.git`.
pub(crate) fn build_harness_shell_options_for_source(
    source_path: &Path,
    source_repo_root: Option<&Path>,
) -> claudine::harness::ShellApprovalOptions {
    build_harness_shell_options_for_source_with_cache(source_path, source_repo_root, None)
}

/// Build source-scoped shell policy while reusing an invocation approval cache.
pub(crate) fn build_harness_shell_options_for_source_with_cache(
    source_path: &Path,
    source_repo_root: Option<&Path>,
    shared_cache: Option<claudine::composition::SharedApprovalCache>,
) -> claudine::harness::ShellApprovalOptions {
    let policy_root = source_repo_root.map(Path::to_path_buf).or_else(|| {
        source_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .map(Path::to_path_buf)
    });
    build_harness_shell_options_for_policy_root(policy_root, shared_cache)
}

/// Build shell approval options, optionally reusing a shared approval
/// cache. Callers like the sequence orchestrator pass a shared cache so
/// that "allow once" approvals from earlier steps carry over to later
/// ones for the duration of the sequence run.
///
/// The interactive approval handler is installed whenever the process
/// can actually prompt — i.e. stdin and stderr are both TTYs. This is
/// independent of whether the spawned agent runs in interactive mode:
/// shell approval happens during preflight, before any agent is launched,
/// so there is no TTY contention. Non-TTY environments (CI, piped input)
/// get no handler and unapproved commands hard-fail as before.
pub(crate) fn build_harness_shell_options_with_cache(
    source_path: &Path,
    repo_root: Option<&Path>,
    shared_cache: Option<claudine::composition::SharedApprovalCache>,
) -> claudine::harness::ShellApprovalOptions {
    build_harness_shell_options_for_policy_root(
        harness_policy_root(source_path, repo_root),
        shared_cache,
    )
}

fn build_harness_shell_options_for_policy_root(
    policy_root: Option<PathBuf>,
    shared_cache: Option<claudine::composition::SharedApprovalCache>,
) -> claudine::harness::ShellApprovalOptions {
    let approval_handler: Option<
        std::sync::Arc<dyn darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler>,
    > = if darkmatter_cli::approval::can_prompt_interactively() {
        Some(std::sync::Arc::new(
            darkmatter_cli::approval::CliShellApprovalHandler,
        ))
    } else {
        None
    };
    let mut opts = claudine::harness::ShellApprovalOptions {
        policy_root,
        approval_handler,
        ..Default::default()
    };
    if let Some(cache) = shared_cache {
        opts.approval_cache = cache;
    }
    opts
}

/// Approval handler that auto-approves every command it is asked about.
///
/// Installed for composition preflight when `--yolo` is active so that
/// unapproved shell commands clear the gate without prompting. Blacklisted
/// commands are rejected upstream, before the handler is consulted, so YOLO
/// never widens the blacklist.
struct YoloApprovalHandler;

impl darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler for YoloApprovalHandler {
    fn approve(
        &self,
        _request: darkmatter::markdown::compose::shell_expansion::ShellApprovalRequest,
    ) -> Result<
        darkmatter::markdown::compose::shell_expansion::ShellApprovalDecision,
        darkmatter::markdown::compose::shell_expansion::ShellExpansionError,
    > {
        Ok(darkmatter::markdown::compose::shell_expansion::ShellApprovalDecision::AllowOnce)
    }
}

/// Apply composition `--dry-run` / `--yolo` overrides to freshly built
/// shell-approval options.
///
/// `dry_run` flips the non-TTY unapproved-command failure to the dry-run
/// CI gate message. `yolo` installs an auto-approving handler so every
/// (non-blacklisted) command clears preflight without prompting, matching
/// the spec guidance that `--yolo` auto-approves all shell commands.
pub(crate) fn apply_composition_shell_overrides(
    mut opts: claudine::harness::ShellApprovalOptions,
    dry_run: bool,
    yolo: bool,
) -> claudine::harness::ShellApprovalOptions {
    opts.dry_run = dry_run;
    if yolo {
        opts.approval_handler = Some(Arc::new(YoloApprovalHandler));
    }
    opts
}

type SharedApprovalHandler =
    Arc<dyn darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler>;

#[derive(Clone)]
pub(crate) struct CachedHarnessLoopContext {
    pub(crate) source_path: PathBuf,
    pub(crate) repo_root: Option<PathBuf>,
    pub(crate) shell_options: claudine::harness::ShellApprovalOptions,
    /// The approval handler the run launched with, retained across a freeze.
    ///
    /// `freeze_shell_approvals` drops the live handler to close the approval
    /// window for the *current* document. Without this copy the drop would be
    /// permanent for the whole invocation, which would silently deny a proxy
    /// target its own approval window — see [`Self::refresh`].
    thawed_handler: Option<SharedApprovalHandler>,
}

impl CachedHarnessLoopContext {
    pub(crate) fn with_shell_options(
        source_path: &Path,
        repo_root: Option<&Path>,
        shell_options: claudine::harness::ShellApprovalOptions,
    ) -> Self {
        Self {
            source_path: source_path.to_path_buf(),
            repo_root: repo_root.map(Path::to_path_buf),
            thawed_handler: shell_options.approval_handler.clone(),
            shell_options,
        }
    }

    /// Re-point the policy root at `source_path`, and — when the active
    /// document actually changed — reopen the approval window.
    ///
    /// The thaw is what makes a proxy target's shell approval equivalent to
    /// invoking that target directly (R9). A freeze is scoped to the document
    /// that earned it: the source's frozen cache must not decide, on the
    /// target's behalf, that a command the target alone declares is denied.
    /// Re-prompting here is free of TTY contention because the source's child
    /// process has already exited and the target has not launched yet.
    ///
    /// Only a document change thaws. Retry and resume re-read the *same*
    /// document mid-run, where the freeze is the point: their unchanged
    /// commands hit the invocation-wide cache and new ones stay denied rather
    /// than prompting behind a live agent.
    pub(crate) fn refresh(&mut self, source_path: &Path, repo_root: Option<&Path>) {
        let policy_root = harness_policy_root(source_path, repo_root);
        self.refresh_with_policy_root(source_path, repo_root, policy_root);
    }

    /// Refresh from definitive source evidence without searching for `.git`.
    pub(crate) fn refresh_for_source(
        &mut self,
        source_path: &Path,
        source_repo_root: Option<&Path>,
    ) {
        let policy_root = source_repo_root.map(Path::to_path_buf).or_else(|| {
            source_path
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .map(Path::to_path_buf)
        });
        self.refresh_with_policy_root(source_path, source_repo_root, policy_root);
    }

    fn refresh_with_policy_root(
        &mut self,
        source_path: &Path,
        repo_root: Option<&Path>,
        policy_root: Option<PathBuf>,
    ) {
        let repo_root = repo_root.map(Path::to_path_buf);
        if self.source_path != source_path || self.repo_root != repo_root {
            self.source_path = source_path.to_path_buf();
            self.repo_root = repo_root;
            self.shell_options.policy_root = policy_root;
            self.shell_options.approval_handler = self.thawed_handler.clone();
        }
    }

    pub(crate) fn shell_options(&self) -> &claudine::harness::ShellApprovalOptions {
        &self.shell_options
    }

    /// Strip the interactive approval handler so subsequent harness-loop
    /// iterations operate in deny-only mode.  Cached and whitelisted
    /// commands still pass; new uncached commands are denied without
    /// prompting.  This enforces the spec contract: "all shell approvals
    /// are resolved before the provider workflow begins."
    ///
    /// Scoped to the active document — [`Self::refresh`] reopens the window
    /// when the run hands off to a different one.
    pub(crate) fn freeze_shell_approvals(&mut self) {
        self.shell_options.approval_handler = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initialize_repository(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        let status = std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(path)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn source_policy_uses_explicit_same_repository_root() {
        let directory = tempfile::TempDir::new().unwrap();
        initialize_repository(directory.path());
        let source = directory.path().join("docs/CLAUDE.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        let invocation = claudine::invocation_context::InvocationContext::capture_at(
            directory.path(),
        );
        let source_context = invocation.derive_source(&source).unwrap();

        let options = build_harness_shell_options_for_source(
            &source,
            source_context.repository_root(),
        );

        assert_eq!(options.policy_root.as_deref(), Some(directory.path()));
    }

    #[test]
    fn source_policy_uses_explicit_sibling_repository_root() {
        let directory = tempfile::TempDir::new().unwrap();
        let launch = directory.path().join("launch");
        let sibling = directory.path().join("sibling");
        initialize_repository(&launch);
        initialize_repository(&sibling);
        let source = sibling.join("docs/CLAUDE.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        let invocation = claudine::invocation_context::InvocationContext::capture_at(&launch);
        let source_context = invocation.derive_source(&source).unwrap();

        let options = build_harness_shell_options_for_source(
            &source,
            source_context.repository_root(),
        );

        assert_eq!(options.policy_root.as_deref(), Some(sibling.as_path()));
        assert_ne!(options.policy_root.as_deref(), Some(launch.as_path()));
    }

    #[test]
    fn authoritative_non_repository_source_keeps_its_directory_as_policy_root() {
        let directory = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(directory.path().join(".git")).unwrap();
        let nested = directory.path().join("outside");
        std::fs::create_dir(&nested).unwrap();
        let source = nested.join("target.md");

        let options = build_harness_shell_options_for_source(&source, None);
        assert_eq!(options.policy_root.as_deref(), Some(nested.as_path()));

        let initial = directory.path().join("router.md");
        let mut cached = CachedHarnessLoopContext::with_shell_options(
            &initial,
            Some(directory.path()),
            build_harness_shell_options_for_source(&initial, Some(directory.path())),
        );
        cached.refresh_for_source(&source, None);
        assert_eq!(cached.shell_options.policy_root.as_deref(), Some(nested.as_path()));
    }
}
