use color_eyre::eyre::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) fn harness_policy_root(source_path: &Path, repo_root: Option<&Path>) -> Option<PathBuf> {
    let source_dir = source_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())?;

    if let Some(source_repo_root) = find_git_root(source_dir) {
        return Some(source_repo_root);
    }

    if let Some(repo_root) = repo_root
        && source_path.starts_with(repo_root)
    {
        return Some(repo_root.to_path_buf());
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

pub(crate) fn build_harness_shell_options(
    source_path: &Path,
    repo_root: Option<&Path>,
) -> claudine::harness::ShellApprovalOptions {
    build_harness_shell_options_with_cache(source_path, repo_root, None)
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
        policy_root: harness_policy_root(source_path, repo_root),
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

#[derive(Clone)]
pub(crate) struct CachedHarnessLoopContext {
    pub(crate) source_path: PathBuf,
    pub(crate) repo_root: Option<PathBuf>,
    pub(crate) shell_options: claudine::harness::ShellApprovalOptions,
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
            shell_options,
        }
    }

    pub(crate) fn refresh(&mut self, source_path: &Path, repo_root: Option<&Path>) {
        let repo_root = repo_root.map(Path::to_path_buf);
        if self.source_path != source_path || self.repo_root != repo_root {
            self.source_path = source_path.to_path_buf();
            self.repo_root = repo_root;
            self.shell_options.policy_root =
                harness_policy_root(&self.source_path, self.repo_root.as_deref());
        }
    }

    pub(crate) fn resolve_context(&self) -> claudine::harness::HarnessResolutionContext<'_> {
        claudine::harness::HarnessResolutionContext {
            source_path: &self.source_path,
            repo_root: self.repo_root.as_deref(),
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
    pub(crate) fn freeze_shell_approvals(&mut self) {
        self.shell_options.approval_handler = None;
    }
}
