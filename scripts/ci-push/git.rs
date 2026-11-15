//! Repository reads, the committed two-tree diff, scope-note writes, and the
//! remote transport boundary.
//!
//! Three Git implementations meet here, each where only it fits:
//!
//! - **gix** (through a handle opened from sniff's discovery) answers every
//!   read: HEAD identity, object peeling, ancestry, refs, notes, and the
//!   committed `base..head` tree diff. sniff's own `changed_paths` unions the
//!   checkout's status into that diff and loses rename classification, so it
//!   cannot stand in for `git diff --name-status BASE HEAD`.
//! - **git2** writes the scope note ([`attach_note`]): gix 0.90 has no notes
//!   write API. Runtime worktrees are git2 too, in [`super::worktree`].
//! - **The Git CLI** runs only `ls-remote`, `fetch`, and `push`
//!   ([`TransportCommand`]), because those must honor the user's own
//!   transport configuration (credential helpers, SSH config, `insteadOf`
//!   rewrites) exactly as `git push` itself does.

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use gix::bstr::{BStr, ByteSlice};
use gix::object::tree::diff::ChangeDetached;
use gix::ObjectId;
use sniff::filesystem::git::GitRepo;
use sniff::request::GitRequest;

/// A repository read failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitError {
    /// `path` is not inside a Git repository.
    NotARepository { path: PathBuf },
    /// The object is absent from the object database.
    MissingObject { id: ObjectId },
    /// The object exists but does not peel to a commit.
    NotACommit { id: ObjectId },
    /// HEAD names a branch that has no commit yet.
    UnbornHead,
    /// A changed path cannot cross the native Windows/Python boundary
    /// without lossy conversion. `path` is the lossy rendering, for the
    /// message only.
    NonPortablePath { path: String, revision: String },
    /// Any other repository failure, with the operation that hit it.
    Read { operation: String, message: String },
}

impl GitError {
    fn read(operation: impl Into<String>, error: impl fmt::Display) -> Self {
        GitError::Read {
            operation: operation.into(),
            message: error.to_string(),
        }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::NotARepository { path } => {
                write!(
                    formatter,
                    "{} is not inside a Git repository",
                    path.display()
                )
            }
            GitError::MissingObject { id } => {
                write!(formatter, "object {id} is not in the local object database")
            }
            GitError::NotACommit { id } => write!(formatter, "object {id} is not a commit"),
            GitError::UnbornHead => formatter.write_str("HEAD has no commit yet"),
            GitError::NonPortablePath { path, revision } => write!(
                formatter,
                "the path {path:?} changed in {revision} is not valid UTF-8, so it cannot be \
                 handed to the planner on Windows without corrupting it"
            ),
            GitError::Read { operation, message } => write!(formatter, "{operation}: {message}"),
        }
    }
}

impl std::error::Error for GitError {}

/// The checked-out commit of a checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadIdentity {
    pub commit: ObjectId,
    /// The full ref HEAD points at; `None` when detached.
    pub branch: Option<String>,
}

/// The path lists `affected_scope.py` takes for one `base..head` comparison,
/// in the order `git diff --name-status -z BASE HEAD --` reports them.
///
/// `deleted` is a subset of `changed`; `renamed_from` (a rename's source) is
/// disjoint from it. A copy contributes its destination only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangedPaths {
    changed: Vec<OsString>,
    deleted: Vec<OsString>,
    renamed_from: Vec<OsString>,
}

impl ChangedPaths {
    /// For the parity oracle, which parses Git's own output. Production paths
    /// come only from [`Repository::changed_paths`], whose [`native_path`]
    /// keeps every Windows path valid UTF-8; the private fields hold that.
    #[cfg(test)]
    pub(crate) fn new(
        changed: Vec<OsString>,
        deleted: Vec<OsString>,
        renamed_from: Vec<OsString>,
    ) -> Self {
        ChangedPaths {
            changed,
            deleted,
            renamed_from,
        }
    }

    pub fn changed(&self) -> &[OsString] {
        &self.changed
    }

    pub fn deleted(&self) -> &[OsString] {
        &self.deleted
    }

    pub fn renamed_from(&self) -> &[OsString] {
        &self.renamed_from
    }

    /// The planner's `--arguments-file` contents, byte-identical to what
    /// `scripts/ci/diff_scope.py` writes for the same diff:
    /// `--deleted` and `--renamed-from` pairs, then `--`, then the changed
    /// paths, each NUL-terminated.
    pub fn arguments_file(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut push = |part: &[u8]| {
            out.extend_from_slice(part);
            out.push(0);
        };
        for path in &self.deleted {
            push(b"--deleted");
            push(&os_bytes(path));
        }
        for path in &self.renamed_from {
            push(b"--renamed-from");
            push(&os_bytes(path));
        }
        push(b"--");
        for path in &self.changed {
            push(&os_bytes(path));
        }
        out
    }
}

#[cfg(unix)]
fn os_bytes(path: &OsStr) -> std::borrow::Cow<'_, [u8]> {
    use std::os::unix::ffi::OsStrExt;
    std::borrow::Cow::Borrowed(path.as_bytes())
}

#[cfg(not(unix))]
fn os_bytes(path: &OsStr) -> std::borrow::Cow<'_, [u8]> {
    // Lossless: `ChangedPaths` holds only paths `native_path` proved UTF-8.
    match path.to_string_lossy() {
        std::borrow::Cow::Borrowed(text) => std::borrow::Cow::Borrowed(text.as_bytes()),
        std::borrow::Cow::Owned(text) => std::borrow::Cow::Owned(text.into_bytes()),
    }
}

/// A repository path as the native string the planner receives.
fn native_path(bytes: &BStr, revision: &str) -> Result<OsString, GitError> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let _ = revision;
        Ok(OsStr::from_bytes(bytes).to_os_string())
    }
    #[cfg(not(unix))]
    {
        utf8_path(bytes, revision).map(OsString::from)
    }
}

/// The Windows rule, compiled everywhere so every host tests it.
#[cfg_attr(unix, allow(dead_code))]
pub(crate) fn utf8_path(bytes: &[u8], revision: &str) -> Result<String, GitError> {
    String::from_utf8(bytes.to_vec()).map_err(|_| GitError::NonPortablePath {
        path: String::from_utf8_lossy(bytes).into_owned(),
        revision: revision.to_owned(),
    })
}

/// One checkout, discovered by sniff and read through gix.
pub struct Repository {
    sniff: GitRepo,
    gix: gix::Repository,
}

impl fmt::Debug for Repository {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Repository")
            .field("root", &self.root())
            .finish_non_exhaustive()
    }
}

impl Repository {
    /// The repository containing `cwd`.
    pub fn discover(cwd: &Path) -> Result<Self, GitError> {
        let sniff = GitRepo::discover(cwd)
            .map_err(|error| {
                GitError::read(
                    format!("cannot discover a repository at {}", cwd.display()),
                    error,
                )
            })?
            .ok_or_else(|| GitError::NotARepository {
                path: cwd.to_path_buf(),
            })?;
        let root = sniff.repo_root();
        let gix = gix::open(root).map_err(|error| {
            GitError::read(
                format!("cannot open the repository at {}", root.display()),
                error,
            )
        })?;
        Ok(Repository { sniff, gix })
    }

    /// The checkout's working directory (the git directory when bare).
    pub fn root(&self) -> &Path {
        self.sniff.repo_root()
    }

    /// The git directory shared by every worktree of this repository.
    pub fn common_dir(&self) -> &Path {
        self.sniff.common_dir()
    }

    pub fn object_hash(&self) -> gix::hash::Kind {
        self.gix.object_hash()
    }

    pub fn head(&self) -> Result<HeadIdentity, GitError> {
        let head = self
            .gix
            .head()
            .map_err(|error| GitError::read("cannot read HEAD", error))?;
        let commit = head.id().ok_or(GitError::UnbornHead)?.detach();
        let branch = head.referent_name().map(|name| name.as_bstr().to_string());
        Ok(HeadIdentity { commit, branch })
    }

    /// Whether the checkout has staged, unstaged, or untracked (not ignored)
    /// changes: the same question as a non-empty `git status --porcelain`.
    pub fn is_dirty(&self) -> Result<bool, GitError> {
        let info = self
            .sniff
            .detect_with_request(&GitRequest::minimal())
            .map_err(|error| GitError::read("cannot read the checkout's status", error))?;
        info.status.map(|status| status.is_dirty).ok_or_else(|| {
            GitError::read(
                "cannot read the checkout's status",
                "sniff returned no status",
            )
        })
    }

    /// The commit `id` names, peeling tags.
    pub fn peel_to_commit(&self, id: ObjectId) -> Result<ObjectId, GitError> {
        let object = self
            .gix
            .try_find_object(id)
            .map_err(|error| GitError::read(format!("cannot read object {id}"), error))?
            .ok_or(GitError::MissingObject { id })?;
        object
            .peel_to_kind(gix::object::Kind::Commit)
            .map(|commit| commit.id)
            .map_err(|_| GitError::NotACommit { id })
    }

    /// Whether `id` names a commit present locally (`git cat-file -e ID^{commit}`).
    pub fn has_commit(&self, id: ObjectId) -> Result<bool, GitError> {
        match self.peel_to_commit(id) {
            Ok(_) => Ok(true),
            Err(GitError::MissingObject { .. } | GitError::NotACommit { .. }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Whether `base` is `head` or one of its ancestors.
    pub fn is_ancestor(&self, base: ObjectId, head: ObjectId) -> Result<bool, GitError> {
        let base = self.peel_to_commit(base)?;
        let head = self.peel_to_commit(head)?;
        if base == head {
            return Ok(true);
        }
        let merge_base = self.gix.merge_base(base, head).map_err(|error| {
            GitError::read(
                format!("cannot find the merge base of {base} and {head}"),
                error,
            )
        })?;
        Ok(merge_base.is_some_and(|id| id.detach() == base))
    }

    /// The object a full ref points at, unpeeled; `None` when absent.
    pub fn resolve_ref(&self, name: &str) -> Result<Option<ObjectId>, GitError> {
        let reference = self
            .gix
            .try_find_reference(name)
            .map_err(|error| GitError::read(format!("cannot read {name}"), error))?;
        reference
            .map(|mut reference| {
                reference
                    .peel_to_id()
                    .map(|id| id.detach())
                    .map_err(|error| GitError::read(format!("cannot resolve {name}"), error))
            })
            .transpose()
    }

    /// The note `notes_ref` attaches to `target`, as `git notes --ref
    /// NOTES_REF show TARGET` prints it; `None` when there is none. Reads
    /// every fan-out layout Git writes.
    pub fn read_note(
        &self,
        notes_ref: &str,
        target: ObjectId,
    ) -> Result<Option<Vec<u8>>, GitError> {
        let Some(notes_commit) = self.resolve_ref(notes_ref)? else {
            return Ok(None);
        };
        let unreadable = |error: &dyn fmt::Display| {
            GitError::read(format!("cannot read the tree of {notes_ref}"), error)
        };
        let mut tree = self
            .gix
            .find_object(notes_commit)
            .map_err(|error| unreadable(&error))?
            .peel_to_tree()
            .map_err(|error| unreadable(&error))?;
        let hex = target.to_string();
        let mut rest = hex.as_str();
        loop {
            if let Some(entry) = tree.find_entry(rest) {
                if entry.mode().is_blob() {
                    let blob = entry.object().map_err(|error| {
                        GitError::read(format!("cannot read the note of {target}"), error)
                    })?;
                    return Ok(Some(blob.detach().data));
                }
            }
            if rest.len() <= 2 {
                return Ok(None);
            }
            let (fanout, tail) = rest.split_at(2);
            let Some(entry) = tree
                .find_entry(fanout)
                .filter(|entry| entry.mode().is_tree())
            else {
                return Ok(None);
            };
            tree = entry
                .object()
                .map(|object| object.into_tree())
                .map_err(|error| GitError::read(format!("cannot read {notes_ref}"), error))?;
            rest = tail;
        }
    }

    /// The committed comparison `git diff --name-status -z BASE HEAD --`
    /// makes, with Git's default rename detection (renames at 50% similarity,
    /// empty files included, no copies, rename limit 1000) regardless of the
    /// user's `diff.renames`: the plan must be the one hosted CI computes with
    /// default settings.
    ///
    /// Similarity is gix's estimate, not Git's, so the two can disagree on a
    /// pair near the threshold, or on which of several identical deleted
    /// files a fewer number of identical additions pair with.
    pub fn changed_paths(&self, base: ObjectId, head: ObjectId) -> Result<ChangedPaths, GitError> {
        let revision = format!("{base}..{head}");
        let tree_of = |id: ObjectId| {
            let commit = self.peel_to_commit(id)?;
            let unreadable = |error: &dyn fmt::Display| {
                GitError::read(format!("cannot read the tree of {id}"), error)
            };
            self.gix
                .find_commit(commit)
                .map_err(|error| unreadable(&error))?
                .tree()
                .map_err(|error| unreadable(&error))
        };
        let old = tree_of(base)?;
        let new = tree_of(head)?;
        let rewrites = gix::diff::Rewrites {
            // Git pairs an empty deletion with an empty addition (`R100`).
            track_empty: true,
            ..gix::diff::Rewrites::default()
        };
        let options = gix::diff::Options::default().with_rewrites(Some(rewrites));
        let changes = self
            .gix
            .diff_tree_to_tree(&old, &new, options)
            .map_err(|error| GitError::read(format!("cannot diff {revision}"), error))?;
        classify(changes, &revision)
    }

    fn transport(
        &self,
        runner: &mut dyn CommandRunner,
        command: TransportCommand,
    ) -> io::Result<CommandOutput> {
        runner.run(&command.in_dir(self.root()))
    }
}

/// One change as `--name-status` reports it.
struct Record<'a> {
    destination: &'a BStr,
    kind: RecordKind<'a>,
}

enum RecordKind<'a> {
    Changed,
    Deleted,
    Renamed { source: &'a BStr },
}

fn classify(changes: Vec<ChangeDetached>, revision: &str) -> Result<ChangedPaths, GitError> {
    let mut records: Vec<Record<'_>> = Vec::with_capacity(changes.len());
    for change in &changes {
        // Directories are not paths `--name-status` prints; their files are
        // reported one by one (as additions, deletions, or rewrites).
        let record = match change {
            ChangeDetached::Addition {
                location,
                entry_mode,
                ..
            }
            | ChangeDetached::Modification {
                location,
                entry_mode,
                ..
            } => (!entry_mode.is_tree()).then_some(Record {
                destination: location.as_bstr(),
                kind: RecordKind::Changed,
            }),
            ChangeDetached::Deletion {
                location,
                entry_mode,
                ..
            } => (!entry_mode.is_tree()).then_some(Record {
                destination: location.as_bstr(),
                kind: RecordKind::Deleted,
            }),
            ChangeDetached::Rewrite {
                source_location,
                location,
                entry_mode,
                copy,
                ..
            } => (!entry_mode.is_tree()).then_some(Record {
                destination: location.as_bstr(),
                kind: if *copy {
                    RecordKind::Changed
                } else {
                    RecordKind::Renamed {
                        source: source_location.as_bstr(),
                    }
                },
            }),
        };
        records.extend(record);
    }
    // Git orders the queue by path, a rename at its destination; a stable
    // sort by bytes reproduces that order because Git's tree order of full
    // paths is their byte order.
    records.sort_by(|a, b| a.destination.cmp(b.destination));

    let mut paths = ChangedPaths::default();
    for record in records {
        let destination = native_path(record.destination, revision)?;
        match record.kind {
            RecordKind::Changed => {}
            RecordKind::Deleted => paths.deleted.push(destination.clone()),
            RecordKind::Renamed { source } => {
                paths.renamed_from.push(native_path(source, revision)?)
            }
        }
        paths.changed.push(destination);
    }
    Ok(paths)
}

/// How the open pull requests of a remote can be listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubRemote {
    /// A github.com remote, as `OWNER/REPO`.
    Repository(String),
    /// A remote on another host: it has no pull requests to list.
    NotGithub,
    /// A URL naming github.com in a form not read here; the caller must fail
    /// closed rather than guess.
    Unrecognized,
}

/// The `OWNER/REPO` of a github.com remote URL, accepting exactly the
/// spellings the shell hook's `github_repo_of` accepted.
pub fn github_repo_of(url: &str) -> GithubRemote {
    const PREFIXES: [&str; 5] = [
        "git@github.com:",
        "ssh://git@github.com/",
        "ssh://github.com/",
        "https://github.com/",
        "http://github.com/",
    ];
    let tail = PREFIXES
        .iter()
        .find_map(|prefix| url.strip_prefix(prefix))
        // `https://*@github.com/*`: the shell removed the shortest matching
        // prefix, which ends at the first `@github.com/`.
        .or_else(|| {
            url.strip_prefix("https://").and_then(|rest| {
                rest.find("@github.com/")
                    .map(|at| &rest[at + "@github.com/".len()..])
            })
        });
    let Some(tail) = tail else {
        return if url.contains("github.com") {
            GithubRemote::Unrecognized
        } else {
            GithubRemote::NotGithub
        };
    };
    let tail = tail.strip_suffix('/').unwrap_or(tail);
    let tail = tail.strip_suffix(".git").unwrap_or(tail);
    match tail.split_once('/') {
        Some((owner, repo)) if !owner.is_empty() && !repo.is_empty() && !repo.contains('/') => {
            GithubRemote::Repository(tail.to_owned())
        }
        _ => GithubRemote::Unrecognized,
    }
}

/// One Git CLI transport invocation. Only [`TransportCommand::ls_remote`],
/// [`TransportCommand::fetch`], and [`TransportCommand::push`] construct one,
/// so nothing else can reach the CLI through this boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportCommand {
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
}

impl TransportCommand {
    /// The variables a transport child must not inherit: they would pin it
    /// to whichever repository the hook's environment names instead of `cwd`.
    pub const REMOVED_ENV: [&'static str; 6] = [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_COMMON_DIR",
        "GIT_PREFIX",
    ];
    /// Variables set on a transport child: a credential prompt would wait on
    /// a terminal the hook does not own.
    pub const SET_ENV: [(&'static str, &'static str); 1] = [("GIT_TERMINAL_PROMPT", "0")];

    pub fn ls_remote(remote: &str, branch: &str) -> Self {
        Self::new([
            "ls-remote",
            "--exit-code",
            remote,
            &format!("refs/heads/{branch}"),
        ])
    }

    pub fn fetch(remote: &str, refspec: &str) -> Self {
        Self::new(["fetch", "--no-tags", remote, refspec])
    }

    pub fn push(remote: &str, refspec: &str) -> Self {
        Self::new(["push", "--no-verify", remote, refspec])
    }

    fn new<'a>(args: impl IntoIterator<Item = &'a str>) -> Self {
        TransportCommand {
            args: args.into_iter().map(OsString::from).collect(),
            cwd: None,
        }
    }

    fn in_dir(mut self, cwd: &Path) -> Self {
        self.cwd = Some(cwd.to_path_buf());
        self
    }

    /// Always `git`, resolved on `PATH`.
    pub fn program(&self) -> &'static str {
        "git"
    }

    /// The arguments after `git`.
    pub fn args(&self) -> &[OsString] {
        &self.args
    }

    /// The checkout the command runs in.
    pub fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }
}

/// A finished transport command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    /// The exit code; `None` when a signal ended the process.
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}

/// Runs a [`TransportCommand`] with null stdin, captured output,
/// [`TransportCommand::SET_ENV`] set and [`TransportCommand::REMOVED_ENV`]
/// removed. The orchestrator supplies a supervised implementation; tests
/// supply recording ones.
pub trait CommandRunner {
    fn run(&mut self, command: &TransportCommand) -> io::Result<CommandOutput>;
}

/// The unsupervised [`CommandRunner`]: a plain blocking child.
#[derive(Debug, Default, Clone, Copy)]
pub struct DirectRunner;

impl CommandRunner for DirectRunner {
    fn run(&mut self, command: &TransportCommand) -> io::Result<CommandOutput> {
        let mut child = Command::new(command.program());
        child.args(command.args()).stdin(Stdio::null());
        if let Some(cwd) = command.cwd() {
            child.current_dir(cwd);
        }
        for name in TransportCommand::REMOVED_ENV {
            child.env_remove(name);
        }
        child.envs(TransportCommand::SET_ENV);
        let output = child.output()?;
        Ok(CommandOutput {
            code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

/// Why a remote branch's tip cannot serve as a comparison base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteTipError {
    /// `ls-remote` advertised no such branch, or could not read the remote.
    Unreadable {
        branch: String,
        remote: String,
        url: String,
    },
    /// The advertised commit is still absent after fetching the branch.
    Unfetchable {
        branch: String,
        remote: String,
        tip: ObjectId,
    },
    /// `git` could not be started.
    Spawn {
        command: String,
        message: String,
    },
    Repository(GitError),
}

impl fmt::Display for RemoteTipError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RemoteTipError::Unreadable { branch, remote, url } => write!(
                formatter,
                "branch {branch} does not exist on {remote} ({url}), or the remote could not be read"
            ),
            RemoteTipError::Unfetchable { branch, remote, tip } => {
                write!(formatter, "branch {branch} on {remote} is at {tip}, which could not be fetched")
            }
            RemoteTipError::Spawn { command, message } => write!(formatter, "could not run `{command}`: {message}"),
            RemoteTipError::Repository(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RemoteTipError {}

/// The remote a push writes to, with every transport read made at most once
/// per branch for the invocation.
pub struct Remote<'r> {
    repository: &'r Repository,
    runner: &'r mut dyn CommandRunner,
    name: String,
    /// Git's second hook argument; empty when unknown, as the shell printed it.
    url: String,
    tips: HashMap<String, Result<ObjectId, RemoteTipError>>,
    fetched: HashSet<String>,
}

impl<'r> Remote<'r> {
    pub fn new(
        repository: &'r Repository,
        runner: &'r mut dyn CommandRunner,
        name: impl Into<String>,
        url: Option<&str>,
    ) -> Self {
        Remote {
            repository,
            runner,
            name: name.into(),
            url: url.unwrap_or_default().to_owned(),
            tips: HashMap::new(),
            fetched: HashSet::new(),
        }
    }

    /// The commit `branch` points at on the remote — the remote's, never a
    /// tracking ref, which is stale exactly when the target advanced — made
    /// present locally. The answer (success or failure) is cached per branch.
    pub fn branch_tip(&mut self, branch: &str) -> Result<ObjectId, RemoteTipError> {
        if let Some(cached) = self.tips.get(branch) {
            return cached.clone();
        }
        let tip = self.lookup_tip(branch);
        self.tips.insert(branch.to_owned(), tip.clone());
        tip
    }

    fn lookup_tip(&mut self, branch: &str) -> Result<ObjectId, RemoteTipError> {
        let command = TransportCommand::ls_remote(&self.name, branch);
        let output = self.run(command)?;
        let advertised = output
            .success()
            .then(|| parse_ls_remote(&output.stdout, self.repository.object_hash()))
            .flatten()
            .ok_or_else(|| RemoteTipError::Unreadable {
                branch: branch.to_owned(),
                remote: self.name.clone(),
                url: self.url.clone(),
            })?;
        if self.ensure_commit(advertised, branch)? {
            Ok(advertised)
        } else {
            Err(RemoteTipError::Unfetchable {
                branch: branch.to_owned(),
                remote: self.name.clone(),
                tip: advertised,
            })
        }
    }

    /// Whether `commit` is present locally, fetching `refs/heads/BRANCH`
    /// (at most once per branch for the invocation) when it is not. A failed
    /// fetch is not itself an error: the caller decides from the answer.
    pub fn ensure_commit(
        &mut self,
        commit: ObjectId,
        branch: &str,
    ) -> Result<bool, RemoteTipError> {
        if self
            .repository
            .has_commit(commit)
            .map_err(RemoteTipError::Repository)?
        {
            return Ok(true);
        }
        if self.fetched.insert(branch.to_owned()) {
            let command = TransportCommand::fetch(&self.name, &format!("refs/heads/{branch}"));
            self.run(command)?;
        }
        self.repository
            .has_commit(commit)
            .map_err(RemoteTipError::Repository)
    }

    /// Pushes the local ref `name` to the same name on the remote, as the
    /// shell's `push_note` did.
    pub fn push_ref(&mut self, name: &str) -> Result<(), TransportError> {
        let command = TransportCommand::push(&self.name, &format!("{name}:{name}"));
        let rendered = render(&command);
        let output = self.run(command).map_err(|error| TransportError {
            command: rendered.clone(),
            message: error.to_string(),
        })?;
        if output.success() {
            Ok(())
        } else {
            Err(TransportError {
                command: rendered,
                message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            })
        }
    }

    fn run(&mut self, command: TransportCommand) -> Result<CommandOutput, RemoteTipError> {
        let rendered = render(&command);
        self.repository
            .transport(self.runner, command)
            .map_err(|error| RemoteTipError::Spawn {
                command: rendered,
                message: error.to_string(),
            })
    }
}

fn render(command: &TransportCommand) -> String {
    let mut text = command.program().to_owned();
    for arg in command.args() {
        text.push(' ');
        text.push_str(&arg.to_string_lossy());
    }
    text
}

/// The first advertised object id of `ls-remote` output.
fn parse_ls_remote(stdout: &[u8], hash: gix::hash::Kind) -> Option<ObjectId> {
    let line = stdout.split(|byte| *byte == b'\n').next()?;
    let hex = line.split(|byte| *byte == b'\t').next()?;
    (hex.len() == hash.len_in_hex())
        .then(|| ObjectId::from_hex(hex).ok())
        .flatten()
}

/// A transport command failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportError {
    pub command: String,
    pub message: String,
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "`{}` failed: {}", self.command, self.message)
    }
}

impl std::error::Error for TransportError {}

/// A scope-note write failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteError {
    /// The document is empty once Git's note cleanup is applied; Git would
    /// remove the note instead of writing it.
    Empty,
    Write {
        notes_ref: String,
        message: String,
    },
}

impl fmt::Display for NoteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteError::Empty => formatter.write_str("the note document is empty"),
            NoteError::Write { notes_ref, message } => {
                write!(formatter, "cannot write a note on {notes_ref}: {message}")
            }
        }
    }
}

impl std::error::Error for NoteError {}

/// Attaches `document` to `target` on `notes_ref`, replacing any note there,
/// after refreshing `notes_ref` from the remote (`+REF:REF`) so the write
/// lands on top of what the remote has. A failed refresh is tolerated, as the
/// shell's `attach_note` tolerated it: the later push then fails instead.
///
/// The note is the document after Git's `notes add -F` cleanup (trailing
/// whitespace and surrounding blank lines removed, blank runs collapsed, one
/// final newline), so it is byte-identical to the shell hook's note.
pub fn attach_note(
    remote: &mut Remote<'_>,
    notes_ref: &str,
    document: &[u8],
    target: ObjectId,
) -> Result<(), NoteError> {
    let _ = remote.run(TransportCommand::fetch(
        &remote.name,
        &format!("+{notes_ref}:{notes_ref}"),
    ));
    let text = String::from_utf8(stripspace(document)).map_err(|_| NoteError::Write {
        notes_ref: notes_ref.to_owned(),
        message: "the document is not UTF-8".to_owned(),
    })?;
    if text.is_empty() {
        return Err(NoteError::Empty);
    }
    let write_error = |error: git2::Error| NoteError::Write {
        notes_ref: notes_ref.to_owned(),
        message: error.message().to_owned(),
    };
    // Opened per write so the ref the fetch just moved is read fresh, and
    // dropped before returning so no handle outlives the call.
    let repository = git2::Repository::open(remote.repository.root()).map_err(write_error)?;
    let signature = note_signature(&repository).map_err(write_error)?;
    let oid = git2::Oid::from_bytes(target.as_bytes()).map_err(write_error)?;
    repository
        .note(&signature, &signature, Some(notes_ref), oid, &text, true)
        .map_err(write_error)?;
    Ok(())
}

/// Git's committer identity: `GIT_COMMITTER_NAME`/`GIT_COMMITTER_EMAIL`
/// override `user.name`/`user.email`, as they do for `git notes`.
fn note_signature(repository: &git2::Repository) -> Result<git2::Signature<'static>, git2::Error> {
    let config = repository.config()?;
    let value = |variable: &str, key: &str| -> Result<String, git2::Error> {
        match std::env::var(variable) {
            Ok(value) if !value.is_empty() => Ok(value),
            _ => config.get_string(key),
        }
    };
    git2::Signature::now(
        &value("GIT_COMMITTER_NAME", "user.name")?,
        &value("GIT_COMMITTER_EMAIL", "user.email")?,
    )
}

/// Git's `stripspace` without comment stripping, which `git notes add -F`
/// applies to the file it reads.
fn stripspace(document: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(document.len() + 1);
    let mut pending_blank = false;
    for line in document.split(|byte| *byte == b'\n') {
        let end = line
            .iter()
            .rposition(|byte| !byte.is_ascii_whitespace())
            .map_or(0, |last| last + 1);
        let line = &line[..end];
        if line.is_empty() {
            pending_blank = !out.is_empty();
            continue;
        }
        if pending_blank {
            out.push(b'\n');
            pending_blank = false;
        }
        out.extend_from_slice(line);
        out.push(b'\n');
    }
    out
}
