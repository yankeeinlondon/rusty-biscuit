use std::path::{Path, PathBuf};
use std::sync::Arc;

use claudine::harness::CapturedPermissions;

/// Shell approval options deciding with `permissions`, the configuration
/// captured for the source document (see
/// [`CapturedPermissions::for_source`]).
pub(crate) fn build_harness_shell_options(permissions: CapturedPermissions) -> claudine::harness::ShellApprovalOptions {
    build_harness_shell_options_with_cache(permissions, None)
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
    permissions: CapturedPermissions,
    shared_cache: Option<claudine::composition::SharedApprovalCache>,
) -> claudine::harness::ShellApprovalOptions {
    let approval_handler = prompt_handler(darkmatter_cli::approval::can_prompt_interactively());
    let mut opts = claudine::harness::ShellApprovalOptions {
        permissions,
        approval_handler,
        ..Default::default()
    };
    if let Some(cache) = shared_cache {
        opts.approval_cache = cache;
    }
    opts
}

/// The installed approval prompt: `md`'s own, which shows a template's
/// placeholders and only the choices a request offers. Without a terminal
/// there is no prompt, and an unapproved command fails as approval
/// unavailable.
fn prompt_handler(can_prompt: bool) -> Option<SharedApprovalHandler> {
    can_prompt.then(|| Arc::new(darkmatter_cli::approval::CliShellApprovalHandler) as SharedApprovalHandler)
}

/// Approval handler that auto-approves every command it is asked about.
///
/// Installed for composition preflight when `--yolo` is active so that
/// unapproved shell commands clear the gate without prompting. Commands the
/// built-in blacklist matches are refused upstream, before the handler is
/// consulted, so YOLO never overrides it.
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

/// Apply the composition `--yolo` override to freshly built shell-approval
/// options: an auto-approving handler, so every (non-blacklisted) command
/// clears preflight without prompting.
pub(crate) fn apply_composition_shell_overrides(
    mut opts: claudine::harness::ShellApprovalOptions,
    yolo: bool,
) -> claudine::harness::ShellApprovalOptions {
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

    /// Re-point the options at `source_path` and the `permissions` captured
    /// for it, and — when the active document actually changed — reopen the
    /// approval window.
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
    pub(crate) fn refresh(&mut self, source_path: &Path, repo_root: Option<&Path>, permissions: CapturedPermissions) {
        let repo_root = repo_root.map(Path::to_path_buf);
        if self.source_path != source_path || self.repo_root != repo_root {
            self.source_path = source_path.to_path_buf();
            self.repo_root = repo_root;
            self.shell_options.permissions = permissions;
            self.shell_options.approval_handler = self.thawed_handler.clone();
        }
    }

    pub(crate) fn shell_options(&self) -> &claudine::harness::ShellApprovalOptions {
        &self.shell_options
    }

    /// Strip the interactive approval handler so subsequent harness-loop
    /// iterations operate in deny-only mode.  Cached and pre-approved
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

/// Permissions captured from a temporary home, so a test never reads or
/// writes the process's home.
#[cfg(test)]
pub(crate) mod test_permissions {
    use std::path::Path;
    use std::sync::Arc;

    use claudine::harness::CapturedPermissions;

    /// The configuration of a user whose home is `home`, with `rules` as the
    /// `permissions.shell-commands` of `~/.darkmatter.yaml` (no file is
    /// written when there are none), and no repository.
    pub(crate) fn user_config(home: &Path, rules: &[&str]) -> Arc<darkmatter::config::load::ConfigSnapshot> {
        std::fs::create_dir_all(home).unwrap();
        if !rules.is_empty() {
            let mut text = String::from("permissions:\n  shell-commands:\n");
            for rule in rules {
                text.push_str(&format!("    - {}\n", serde_json::to_string(rule).unwrap()));
            }
            std::fs::write(home.join(".darkmatter.yaml"), text).unwrap();
        }
        let context = biscuit_file::FileResolutionContext::from_snapshot(home, Some(home.to_path_buf()), Default::default());
        Arc::new(
            darkmatter::config::load::ConfigSnapshot::load(&context, None, darkmatter::config::load::ConfigLoadMode::Passive)
                .unwrap(),
        )
    }

    /// [`user_config`]'s evaluated permissions.
    pub(crate) fn user_permissions(home: &Path, rules: &[&str]) -> CapturedPermissions {
        CapturedPermissions::of_snapshot(&user_config(home, rules))
    }

    /// The `permissions.shell-commands` entries saved under `home`, as written.
    pub(crate) fn saved_rules(home: &Path) -> Vec<String> {
        let snapshot = user_config(home, &[]);
        let policy = snapshot.policy().unwrap();
        policy.user().permissions().shell_commands.value.iter().map(|entry| entry.authored.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `git init`s `path` and commits `files` (relative path, text).
    fn committed_repository(path: &Path, files: &[(&str, &str)]) {
        std::fs::create_dir_all(path).unwrap();
        for (relative, text) in files {
            let file = path.join(relative);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        }
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(["-c", "user.name=test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false"])
                .args(args)
                .current_dir(path)
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?}");
        };
        git(&["init", "--quiet"]);
        git(&["add", "-A"]);
        git(&["commit", "--quiet", "--allow-empty", "-m", "fixture"]);
    }

    /// Whether `options` pre-approve `command` without asking.
    fn preapproved(options: &claudine::harness::ShellApprovalOptions, command: &str) -> bool {
        claudine::harness::validate_and_approve_command(command, options).is_ok()
    }

    #[test]
    fn each_source_decides_with_its_own_repository_permissions() {
        let directory = tempfile::TempDir::new().unwrap();
        let home = directory.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let launch = directory.path().join("launch");
        let sibling = directory.path().join("sibling");
        committed_repository(&launch, &[("docs/CLAUDE.md", "launch\n")]);
        committed_repository(
            &sibling,
            &[(".darkmatter.yaml", "permissions:\n  shell-commands:\n    - prefix ls\n"), ("docs/CLAUDE.md", "sibling\n")],
        );
        let snapshot = crate::request::snapshot().clone().with_home(Some(home));
        let invocation = claudine::invocation_context::InvocationContext::capture_at(&snapshot, &launch).unwrap();
        let options_for = |source: &Path| {
            let source_context = invocation.derive_source(source).unwrap();
            build_harness_shell_options(CapturedPermissions::for_source(&invocation, source_context.file_resolution_context()))
        };

        let sibling_options = options_for(&sibling.join("docs/CLAUDE.md"));
        let launch_options = options_for(&launch.join("docs/CLAUDE.md"));

        assert!(preapproved(&sibling_options, "ls -la"));
        assert!(!preapproved(&launch_options, "ls -la"));
    }

    #[test]
    fn a_refresh_to_another_document_takes_that_documents_permissions() {
        let directory = tempfile::TempDir::new().unwrap();
        let home = directory.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(".darkmatter.yaml"), "permissions:\n  shell-commands:\n    - prefix ls\n").unwrap();
        let allowing = super::test_permissions::user_permissions(&home, &[]);
        let initial = directory.path().join("router.md");
        let target = directory.path().join("target.md");
        let mut cached = CachedHarnessLoopContext::with_shell_options(
            &initial,
            None,
            build_harness_shell_options(CapturedPermissions::Absent),
        );
        assert!(!preapproved(cached.shell_options(), "ls"));

        cached.refresh(&target, None, allowing);

        assert!(preapproved(cached.shell_options(), "ls"));
    }

    // ── The installed prompt, answered with scripted input ───────────────

    use darkmatter::markdown::compose::shell_expansion::template::DeferredShellRoots;
    use darkmatter::markdown::compose::shell_expansion::{
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
    };
    use std::sync::Mutex;

    /// The prompt Claudine installs, reading one scripted line per request.
    struct ScriptedPrompt {
        answers: Mutex<Vec<&'static str>>,
        transcript: Mutex<String>,
    }

    impl ScriptedPrompt {
        fn new(answers: &[&'static str]) -> Arc<Self> {
            Arc::new(Self {
                answers: Mutex::new(answers.iter().rev().copied().collect()),
                transcript: Mutex::new(String::new()),
            })
        }

        fn transcript(&self) -> String {
            let raw = self.transcript.lock().unwrap();
            let mut plain = String::with_capacity(raw.len());
            let mut chars = raw.chars();
            while let Some(c) = chars.next() {
                if c == '\x1b' {
                    chars.by_ref().find(|inner| *inner == 'm');
                } else {
                    plain.push(c);
                }
            }
            plain
        }
    }

    impl ShellApprovalHandler for ScriptedPrompt {
        fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
            let answer = self.answers.lock().unwrap().pop().expect("a scripted answer");
            let mut output = Vec::new();
            let decision = darkmatter_cli::approval::approve_with_io(
                &request,
                &mut std::io::Cursor::new(format!("{answer}\n")),
                &mut output,
            );
            self.transcript.lock().unwrap().push_str(&String::from_utf8(output).unwrap());
            decision
        }
    }

    fn user_config(dir: &Path) -> Arc<darkmatter::config::load::ConfigSnapshot> {
        super::test_permissions::user_config(&dir.join("home"), &[])
    }

    fn saved_rules(dir: &Path) -> Vec<String> {
        super::test_permissions::saved_rules(&dir.join("home"))
    }

    fn scripted_options(dir: &Path, prompt: &Arc<ScriptedPrompt>) -> claudine::harness::ShellApprovalOptions {
        let mut options = build_harness_shell_options(CapturedPermissions::of_snapshot(&user_config(dir)));
        options.approval_handler = Some(Arc::clone(prompt) as SharedApprovalHandler);
        options
    }

    fn compose_request(dir: &Path, late: bool) -> darkmatter::markdown::compose::ComposeRequest {
        let mut options = darkmatter::markdown::compose::ComposeOptions::new();
        if late {
            options = options.with_deferred_shell_roots(DeferredShellRoots::new(["seq"]));
        }
        darkmatter::markdown::compose::ComposeRequest::with_context(options, crate::request::test_context_at(dir))
            .unwrap()
    }

    #[test]
    fn without_a_terminal_no_prompt_is_installed() {
        assert!(prompt_handler(false).is_none());
        assert!(prompt_handler(true).is_some());
    }

    #[test]
    fn a_single_document_command_is_shown_resolved_and_saved_as_its_shape() {
        let dir = tempfile::TempDir::new().unwrap();
        let prompt = ScriptedPrompt::new(&["2"]);
        let markdown: darkmatter::markdown::Markdown =
            "---\nout: dist\n---\n::shell just clean --out {{ out }}/build\n".into();

        let result = claudine::composition::preflight::resolve_shell_approvals(
            Some(&markdown),
            Some(&compose_request(dir.path(), false)),
            &scripted_options(dir.path(), &prompt),
            None,
            None,
        )
        .unwrap();

        let transcript = prompt.transcript();
        assert!(transcript.contains("Command:  just clean --out {{ out: string }}/build"), "{transcript}");
        assert!(transcript.contains("Resolved: just clean --out dist/build"), "{transcript}");
        assert!(transcript.contains("1 │ Allow this exact command"), "{transcript}");
        assert!(transcript.contains("2 │ Allow this command shape"), "{transcript}");
        assert_eq!(saved_rules(dir.path()), ["template just clean --out {{ string }}/build"]);
        assert!(!result.approved_commands.contains_exact("just clean --out dist/build"));
        assert_eq!(result.approved_commands.templates().count(), 1);
    }

    #[test]
    fn a_late_command_offers_no_exact_choice_and_refuses_an_unlisted_number() {
        let dir = tempfile::TempDir::new().unwrap();
        // Four choices are listed, so "5" is refused before "4" denies.
        let prompt = ScriptedPrompt::new(&["5\n4"]);
        let markdown: darkmatter::markdown::Markdown =
            "::shell just clean --out {{ last(seq.outputs) }}/build && git add {{ last(seq.outputs) }}\n".into();
        let candidates = markdown.compose_preflight(&compose_request(dir.path(), true)).unwrap().candidates;

        let error = claudine::harness::shell::validate_and_approve_template_site(
            &candidates[0],
            &scripted_options(dir.path(), &prompt),
        )
        .expect_err("denied");

        assert!(matches!(error, claudine::harness::HarnessError::ShellCommandDenied { .. }), "{error:?}");
        let transcript = prompt.transcript();
        assert!(
            transcript.contains("Command:  just clean --out {{ last(seq.outputs): string }}/build && git add"),
            "{transcript}"
        );
        assert!(!transcript.contains("exact command"), "{transcript}");
        assert!(!transcript.contains("Resolved:"), "{transcript}");
        assert!(transcript.contains("saves \"prefix just\", \"prefix git\" to your shell-commands"), "{transcript}");
        assert!(transcript.contains("Invalid choice. Please enter 1-4."), "{transcript}");
        assert!(!transcript.contains("blacklist"), "{transcript}");
        assert!(saved_rules(dir.path()).is_empty());
    }
}
