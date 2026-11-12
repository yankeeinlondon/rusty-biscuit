//! The production [`PushServices`]: the real repository, processes, and clock.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use biscuit_terminal::prelude::{Terminal, TerminalRenderable, UnorderedList};
use chrono::NaiveDate;
use gix::ObjectId;

use super::evidence::{SCOPE_NOTES_REF, ci_environment};
use super::git::{
    CommandOutput, CommandRunner, GitError, HeadIdentity, Remote, RemoteTipError, Repository,
    TransportCommand, attach_note,
};
use super::process::{CancelSignal, Cancellation, ChildCommand, Completed, ProcessError, Termination};
use super::python::PythonSelectionError;
use super::worktree::{CleanupFailure, CreateError, RuntimeWorktree};
use super::{BlockReason, Diagnostic, PushRequest, PushServices, PushSession, PythonCommand};

/// Services that read the host `ci-push` runs on. Diagnostics go to stderr
/// as they happen.
#[derive(Debug, Default)]
pub struct HostServices;

impl PushServices for HostServices {
    fn object_hash(&mut self, cwd: &Path) -> Result<gix::hash::Kind> {
        let repository = sniff::filesystem::git::GitRepo::discover(cwd)
            .with_context(|| format!("cannot open the Git repository at {}", cwd.display()))?
            .with_context(|| format!("{} is not inside a Git repository", cwd.display()))?;
        let root = repository.repo_root();
        let opened = gix::open(root)
            .with_context(|| format!("cannot open the Git repository at {}", root.display()))?;
        Ok(opened.object_hash())
    }

    fn emit(&mut self, diagnostic: &Diagnostic) {
        report(diagnostic);
    }

    fn session(
        &mut self,
        request: &PushRequest,
        body: &mut dyn FnMut(&mut dyn PushSession) -> Result<()>,
    ) -> Result<()> {
        let repository = Repository::discover(&request.cwd)
            .map_err(|error| anyhow::anyhow!("cannot open the checkout: {error}"))?;
        // An unreadable remote configuration leaves the identity unknown,
        // which every repository-scoped constraint record matches.
        let url = request.remote_url.clone().or_else(|| {
            sniff::filesystem::git::api::remote_url(repository.root(), &request.remote_name)
                .ok()
                .flatten()
                .filter(|url| !url.is_empty())
        });
        let mut runner = SupervisedRunner;
        let remote = Remote::new(&repository, &mut runner, request.remote_name.clone(), url.as_deref());
        let mut session = HostSession {
            repository: &repository,
            remote,
            url,
            worktree: None,
            python: None,
            terminal: None,
        };
        let result = body(&mut session);
        if let Err(failure) = session.release_tree() {
            eprintln!("{failure}");
        }
        result
    }
}

/// Runs Git's transport commands as owned, cancellable children.
struct SupervisedRunner;

impl CommandRunner for SupervisedRunner {
    fn run(&mut self, command: &TransportCommand) -> io::Result<CommandOutput> {
        let mut child = ChildCommand::new(command.program())
            .args(command.args())
            .repository_independent();
        if let Some(cwd) = command.cwd() {
            child = child.current_dir(cwd);
        }
        for (name, value) in TransportCommand::SET_ENV {
            child = child.env(name, value);
        }
        let completed = child.run().map_err(|error| error.source)?;
        Ok(CommandOutput {
            code: match completed.termination {
                Termination::Exited(status) => status.code(),
                Termination::TimedOut | Termination::Cancelled(_) => None,
            },
            stdout: completed.stdout,
            stderr: completed.stderr,
        })
    }
}

struct HostSession<'r> {
    repository: &'r Repository,
    remote: Remote<'r>,
    url: Option<String>,
    worktree: Option<RuntimeWorktree>,
    python: Option<PythonCommand>,
    terminal: Option<Terminal>,
}

impl HostSession<'_> {
    fn python_command(&self) -> Result<&PythonCommand, ProcessError> {
        self.python.as_ref().ok_or_else(|| ProcessError {
            program: "python".into(),
            source: io::Error::other("no Python interpreter was selected"),
        })
    }
}

impl PushSession for HostSession<'_> {
    fn root(&self) -> &Path {
        self.repository.root()
    }

    fn remote_url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    fn head(&mut self) -> Result<HeadIdentity, GitError> {
        self.repository.head()
    }

    fn is_dirty(&mut self) -> Result<bool, GitError> {
        self.repository.is_dirty()
    }

    fn select_python(&mut self) -> Result<(), PythonSelectionError> {
        if self.python.is_none() {
            self.python = Some(PythonCommand::select(std::env::var_os("PATH").as_deref())?);
        }
        Ok(())
    }

    fn remote_tip(&mut self, branch: &str) -> Result<ObjectId, RemoteTipError> {
        self.remote.branch_tip(branch)
    }

    fn ensure_commit(&mut self, commit: ObjectId, branch: &str) -> Result<bool, RemoteTipError> {
        self.remote.ensure_commit(commit, branch)
    }

    fn arguments_file(&mut self, base: ObjectId, head: ObjectId) -> Result<Vec<u8>, GitError> {
        Ok(self.repository.changed_paths(base, head)?.arguments_file())
    }

    fn list_pull_requests(&mut self, repository: &str, branch: &str) -> Result<Completed, ProcessError> {
        ChildCommand::new("gh")
            .args([
                "pr",
                "list",
                "--repo",
                repository,
                "--head",
                branch,
                "--state",
                "open",
                "--json",
                "number,baseRefName",
            ])
            .current_dir(self.repository.root())
            .env("GH_NO_UPDATE_NOTIFIER", "1")
            .repository_independent()
            .run()
    }

    fn materialize(&mut self, commit: ObjectId) -> Result<PathBuf, CreateError> {
        if let Err(cleanup) = self.release_tree() {
            return Err(CreateError {
                commit,
                message: "the previous temporary worktree could not be removed".to_owned(),
                cleanup: Some(cleanup),
            });
        }
        let worktree = RuntimeWorktree::create(self.repository.root(), commit)?;
        let path = worktree.path().to_path_buf();
        self.worktree = Some(worktree);
        Ok(path)
    }

    fn release_tree(&mut self) -> Result<(), CleanupFailure> {
        // Every child that ran in the tree has been reaped, and no repository
        // handle into it is held here, so Windows lets the directory go.
        match self.worktree.take() {
            Some(mut worktree) => worktree.remove(),
            None => Ok(()),
        }
    }

    fn python(&mut self, cwd: &Path, args: &[OsString]) -> Result<Completed, ProcessError> {
        self.python_command()?
            .command()
            .args(args)
            .current_dir(cwd)
            .repository_independent()
            .run()
    }

    fn render_plan(&mut self, plan: &[u8]) -> Result<String> {
        let terminal = self.terminal.get_or_insert_with(Terminal::new);
        crate::ci_plan_render::render_document(plan, terminal)
    }

    fn publish_scope_note(&mut self, receipt: &[u8], head: ObjectId) -> Result<(), String> {
        attach_note(&mut self.remote, SCOPE_NOTES_REF, receipt, head).map_err(|error| error.to_string())?;
        self.remote.push_ref(SCOPE_NOTES_REF).map_err(|error| error.to_string())
    }

    fn run_gates(
        &mut self,
        selection: &[String],
        variables: &[(OsString, OsString)],
        staging: &Path,
    ) -> Result<Completed, ProcessError> {
        let interpreter = self
            .python_command()?
            .gate_environment(staging)
            .map_err(|source| ProcessError {
                program: "just".into(),
                source,
            })?;
        let mut command = ChildCommand::new("just")
            .arg("pre-push")
            .args(selection)
            .current_dir(self.repository.root())
            .repository_independent()
            .clear_plan_input();
        for (name, value) in variables {
            command = command.env(name, value);
        }
        for (name, value) in interpreter {
            command = command.env(name, value);
        }
        command.inherit_output().run()
    }

    fn environment(&mut self) -> Option<String> {
        // A host whose OS cannot be read publishes no validation receipt.
        let os = sniff::os::detect_os_with_request(&sniff::request::OsRequest::summary()).ok()?;
        ci_environment(&os.os_type, &os.kernel).map(str::to_owned)
    }

    fn home(&self) -> Option<PathBuf> {
        let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        std::env::var_os(variable)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(dirs::home_dir)
    }

    fn variable(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }

    fn today(&self) -> NaiveDate {
        chrono::Local::now().date_naive()
    }

    fn cancelled(&self) -> Option<CancelSignal> {
        Cancellation::process().requested()
    }

    fn emit(&mut self, diagnostic: &Diagnostic) {
        report(diagnostic);
    }
}

/// Writes `diagnostic` to stderr. A constraint refusal lists its records
/// with the terminal's list component, so a long record wraps under its own
/// bullet; the records are added literally, never parsed as markup.
fn report(diagnostic: &Diagnostic) {
    if let Diagnostic::Blocked(BlockReason::Constraint { update, refusals }) = diagnostic {
        let list = UnorderedList::new(refusals.clone()).render(&Terminal::new());
        eprintln!(
            "Push blocked: a recorded execution constraint refuses {update}:\n{}\n{}",
            list.trim_end(),
            BlockReason::CONSTRAINT_ADVICE
        );
    } else {
        eprintln!("{diagnostic}");
    }
}
