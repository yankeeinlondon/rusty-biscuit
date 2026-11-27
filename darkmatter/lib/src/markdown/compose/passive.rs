//! The passive session of a dry run
//! ([`ComposeOptions::with_dry_run`](super::ComposeOptions::with_dry_run)).
//!
//! A dry run composes a document without any effect: no command runs, no
//! login shell is probed, no packet is sent, no approval is asked for, and no
//! cache is read or written. Work that needs one of those effects is
//! *deferred*: its authored text stays in the output as written, and the
//! session records a [`Deferral`] naming what was skipped, why, and where.
//!
//! Deferral propagates through evaluation. A frontmatter value that needs a
//! command keeps its authored text and is registered here; any expression
//! that reads that text, directly or through another value that read it,
//! fails with [`ExpressionError::Unavailable`](super::expression::ExpressionError::Unavailable)
//! instead of producing a value. That failure travels through lookups,
//! conditions, collections, function arguments, and paths like any other
//! evaluation error, and the surface that started the evaluation keeps its
//! own authored text: a `{{ … }}` span stays as written, a `::block` whose
//! condition is unavailable stays whole with its directive lines, a
//! `::map`/`::filter` whose gate, source, callback, or option is unavailable
//! stays whole and inert, and a frontmatter value that read one is deferred
//! in turn. Independent spans,
//! siblings, and branches still evaluate.
//!
//! The session is shared by every clone of the request's options, so a
//! transcluded child, a schema slot, a callback, and a nested `as_markdown`
//! composition all see the same deferred values and report into one list.
//!
//! A session also holds the dry run's [`PassiveObservations`]: one in-memory
//! `load_data` memo and one remote-fetch runtime per fetch policy, which
//! every composition attached to the session reads through. A host that
//! checks one invocation in several passes, each with a session of its own
//! deferrals, gives every session after the first
//! [`PassiveSession::sharing_observations`], so a file or URL the passes
//! all read is observed once and every pass judges the same bytes. Local
//! source files (an included document, a `$schema` file) are observed the
//! same way, through [`PassiveObservations::local_reads`]. Nothing
//! observed is written to a cache.

use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::markdown::compose::expression::ExpressionError;

/// What a dry run could not do, and so left as written.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum DeferralReason {
    /// The value comes from a command, which a dry run never runs.
    Command {
        /// The command as written.
        command: String,
    },
    /// A function would start the login shell to answer (`has_alias`,
    /// `can_execute`, …).
    ShellProbe {
        /// The function's name.
        function: String,
    },
    /// A function would send a network packet (`ping`, `ping_under`).
    NetworkProbe {
        /// The function's name.
        function: String,
    },
    /// A function would query a provider's API (`pr`, `cicd`, …).
    ProviderQuery {
        /// The function's name.
        function: String,
    },
    /// A URL the captured `permissions.network: ask` leaves needing an
    /// approval this run did not give; a dry run asks nothing, so the read
    /// is not made and what depends on it is not checked.
    NetworkNeedsApproval {
        /// The URL that would have been fetched.
        url: String,
    },
    /// A read the real run would ask the user to approve; a dry run asks
    /// nothing.
    ReadNeedsApproval {
        /// What would have been read.
        target: String,
    },
    /// A required input the caller gave no value for; a dry run never asks
    /// for one, so what reads it stays as written.
    MissingInput {
        /// The input's name.
        name: String,
    },
    /// The value is one the run writes before this composition (a lifecycle
    /// `set` an earlier task or event makes, the outputs earlier tasks
    /// produce), declared with [`PassiveSession::defer_written`]: whatever it
    /// holds now is not what the run will read.
    WrittenAtLaunch {
        /// The value, as declared (`topic`, `seq.outputs`).
        name: String,
    },
    /// The file is one the run creates before this composition reads it
    /// (a root `initialize` action such as `ensure_file`), declared with
    /// [`PassiveSession::defer_created`]: it is absent now, so its content,
    /// and whether it exists, are known only at launch.
    CreatedAtLaunch {
        /// The created file, as the action wrote it (`generated/notes.md`).
        path: String,
    },
    /// The file exists, but the run appends to it before this composition
    /// reads it (a root `initialize` `append_line` or `append_jsonl`),
    /// declared with [`PassiveSession::declare_initializer_write`]: what it
    /// holds now is not what the run will read.
    ChangedAtLaunch {
        /// The changed file, as the action wrote it (`log.jsonl`).
        path: String,
    },
    /// The value reads another value that is itself deferred.
    DependsOn {
        /// The deferred value read, as it was named (`branch`, `fm.branch`).
        name: String,
    },
}

impl DeferralReason {
    /// A short phrase for a report row ("runs a command", …).
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Command { command } => format!("needs the output of `{command}`, which a dry run does not run"),
            Self::ShellProbe { function } => {
                format!("`{function}()` starts the login shell, which a dry run does not do")
            }
            Self::NetworkProbe { function } => {
                format!("`{function}()` sends network packets, which a dry run does not do")
            }
            Self::ProviderQuery { function } => {
                format!("`{function}()` queries a provider's API, which a dry run does not do")
            }
            Self::NetworkNeedsApproval { url } => {
                format!("fetching `{url}` needs network approval (`permissions.network: ask`), which a dry run does not ask for")
            }
            Self::ReadNeedsApproval { target } => {
                format!("reading {target} needs approval, which a dry run does not ask for")
            }
            Self::MissingInput { name } => format!("reads `{name}`, which has no value"),
            Self::WrittenAtLaunch { name } => {
                format!("reads `{name}`, which the run writes before this point, so it is known only at launch")
            }
            Self::CreatedAtLaunch { path } => {
                format!("reads `{path}`, which the run creates before this point, so it is known only at launch")
            }
            Self::ChangedAtLaunch { path } => {
                format!("reads `{path}`, which the run appends to before this point, so its content is known only at launch")
            }
            Self::DependsOn { name } => format!("reads `{name}`, which is not known until the document runs"),
        }
    }
}

/// Which authored construct a deferral left as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DeferredConstruct {
    /// A top-level frontmatter value.
    FrontmatterValue,
    /// A `{{ … }}` span in the body or in frontmatter text.
    Interpolation,
    /// A body `::shell` directive line.
    ShellDirective,
    /// A body `::shell-block` region.
    ShellBlock,
    /// A `::block when=…` region whose condition could not be decided.
    BlockCondition,
    /// A `$schema` template whose slots could not be resolved without
    /// asking; the schema's checks were not run.
    SchemaTemplate,
    /// A `::file`/`::code` directive, or a `prologue`/`epilogue` value,
    /// whose target was not read; its content is not in the output.
    Transclusion,
    /// A schema problem with a frontmatter value the run writes before this
    /// composition ([`DeferralReason::WrittenAtLaunch`]); the value is
    /// judged when it is known. `key` names the value.
    SchemaCheck,
    /// A `::map`/`::filter` region whose gate, source, callback, or option
    /// could not be evaluated. The whole region stays as written and inert:
    /// nothing inside it runs, and what it would contribute is unknown.
    /// `authored` is the region's text.
    Collection,
}

/// One thing a dry run left as written.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Deferral {
    /// The construct left as written.
    pub construct: DeferredConstruct,
    /// Why.
    pub reason: DeferralReason,
    /// The authored text of the construct (a `{{ … }}` span, a command line,
    /// a condition, a collection region), as written.
    pub authored: String,
    /// The top-level frontmatter key, for a frontmatter value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// The one-based line in the document, when the surface knows it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

#[derive(Debug, Default)]
struct State {
    /// Authored text of each deferred value, mapped to why it is deferred.
    values: BTreeMap<String, DeferralReason>,
    /// Top-level names with no value, mapped to why a read of one is
    /// unavailable.
    absent: BTreeMap<String, DeferralReason>,
    /// Values (a top-level name or a dotted member path) the run writes
    /// before this composition, mapped to why a read of one is unavailable
    /// whatever it holds now.
    written: BTreeMap<String, DeferralReason>,
    deferrals: Vec<Deferral>,
    seen: HashSet<Deferral>,
}

/// The invocation-local record of a dry run, shared by every clone of the
/// request; see the [module docs](self).
#[derive(Debug, Clone, Default)]
pub struct PassiveSession {
    inner: Arc<Mutex<State>>,
    observations: PassiveObservations,
    /// The files the run writes before this composition, shared by every
    /// session made with [`Self::sharing_observations`].
    created: Arc<Mutex<created::Initialized>>,
}

/// The sources a dry run has read, shared by every session made with
/// [`PassiveSession::sharing_observations`]: the `load_data` memo (local and
/// fetched sources alike), the remote-fetch runtimes, one per distinct
/// authorization, and the bytes of every local source file a reader opened
/// ([`Self::local_reads`]). In memory only, for the life of the invocation.
///
/// A runtime is shared only with a composition whose runtime would decide
/// every URL alike (the same fetch policy, `network` permission, and
/// transport settings), and each read still passes that runtime's policy
/// and any schema-slot gate before the memo is consulted, so a URL one pass
/// was allowed to fetch never serves a read another pass's policy refuses.
/// The same holds for local files: a reader judges its own permission for a
/// path (a schema slot's `file-read` grant, say) before it asks the local
/// reads, and each occurrence still evaluates what it read under its own
/// values.
#[derive(Debug, Clone, Default)]
pub struct PassiveObservations {
    inner: Arc<ObservationsInner>,
}

#[derive(Debug, Default)]
struct ObservationsInner {
    data: super::DataRuntime,
    remote: Mutex<Vec<super::remote_fetch::RemoteFetchRuntime>>,
    local: simplified_schema::resolve::buffers::SharedReads,
}

impl PassiveObservations {
    /// The shared `load_data` runtime.
    pub fn data(&self) -> super::DataRuntime {
        self.inner.data.clone()
    }

    /// The shared observation of local source files: a prompt, an included
    /// `::file`/`::code` target, a `prologue`/`epilogue`, a `$schema` file,
    /// or a host's referenced document, each opened once for every pass.
    pub fn local_reads(&self) -> &simplified_schema::resolve::buffers::SharedReads {
        &self.inner.local
    }

    /// The shared runtime that authorizes every URL as `candidate` does,
    /// keeping `candidate` as that runtime when none is held yet.
    pub(crate) fn remote(&self, candidate: super::remote_fetch::RemoteFetchRuntime) -> super::remote_fetch::RemoteFetchRuntime {
        let mut held = self.inner.remote.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(existing) = held.iter().find(|existing| existing.authorizes_like(&candidate)) {
            return existing.clone();
        }
        held.push(candidate.clone());
        candidate
    }

    /// Whether two handles share one set of observations.
    #[must_use]
    pub fn same_observations(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

/// The bytes of the local file at `path`, as a composition holding
/// `session` reads a source: through the session's
/// [`PassiveObservations::local_reads`], else the
/// [`SharedReads`](simplified_schema::resolve::buffers::SharedReads) in force
/// on this thread, else one open. A caller runs its own permission check
/// for `path` first.
pub(crate) fn read_local_source(
    session: Option<&PassiveSession>,
    path: &std::path::Path,
) -> std::io::Result<simplified_schema::resolve::buffers::ReadSource> {
    use simplified_schema::resolve::buffers;
    match session.map(|session| session.observations().local_reads().clone()).or_else(buffers::shared_reads) {
        Some(shared) => shared.read(path),
        None => buffers::open_source(path),
    }
}

impl PassiveSession {
    /// A new, empty session with observations of its own.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A new session with no deferrals or declared values, reading through
    /// this one's [`PassiveObservations`]: another pass of the same dry run.
    #[must_use]
    pub fn sharing_observations(&self) -> Self {
        Self { inner: Arc::default(), observations: self.observations.clone(), created: Arc::clone(&self.created) }
    }

    /// The sources this session's dry run has read.
    #[must_use]
    pub fn observations(&self) -> &PassiveObservations {
        &self.observations
    }

    /// Every deferral recorded so far, in the order recorded, each once.
    #[must_use]
    pub fn deferrals(&self) -> Vec<Deferral> {
        self.lock().deferrals.clone()
    }

    /// How many deferrals have been recorded; a mark for
    /// [`Self::deferrals_since`].
    #[must_use]
    pub fn mark(&self) -> usize {
        self.lock().deferrals.len()
    }

    /// The deferrals recorded after `mark`.
    #[must_use]
    pub fn deferrals_since(&self, mark: usize) -> Vec<Deferral> {
        self.lock().deferrals.iter().skip(mark).cloned().collect()
    }

    /// Whether two handles share one session.
    #[must_use]
    pub fn same_session(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// Why the value whose text is `text` is deferred, when it is one.
    #[must_use]
    pub fn deferred_value(&self, text: &str) -> Option<DeferralReason> {
        self.lock().values.get(text).cloned()
    }

    /// Declares that the top-level value `name` has no value in this dry run
    /// for `reason` (a missing input, a value only a run writes): a read of
    /// it, or below it, that finds nothing is unavailable instead of empty,
    /// and the schema stage does not report it missing.
    pub fn defer_absent(&self, name: &str, reason: DeferralReason) {
        self.lock().absent.entry(name.to_string()).or_insert(reason);
    }

    /// Why the top-level value `name` is declared absent, when it is.
    #[must_use]
    pub fn absent_value(&self, name: &str) -> Option<DeferralReason> {
        self.lock().absent.get(name).cloned()
    }

    /// Declares that the value `path` (a top-level name such as `topic`, or a
    /// member path such as `seq.outputs`) is written by the run before this
    /// composition for `reason`: a read of it, or below it, is unavailable
    /// whatever it holds now and however it is spelled (`seq['outputs']`,
    /// `(seq).outputs`, a computed key), except an explicit `fm.` read of a
    /// document property whose bare name a host global takes (`fm.seq` when
    /// the host binds `seq`). The schema stage sets aside its problems
    /// with a top-level one, recording a [`DeferredConstruct::SchemaCheck`]
    /// deferral instead.
    pub fn defer_written(&self, path: &str, reason: DeferralReason) {
        self.lock().written.entry(path.to_string()).or_insert(reason);
    }

    /// Why a read of `path` (`fm.` prefix allowed) reads a value declared
    /// written ([`Self::defer_written`]), when it does: the path is a
    /// declared one or below it.
    #[must_use]
    pub fn written_value(&self, path: &str) -> Option<DeferralReason> {
        let path = path.strip_prefix("fm.").unwrap_or(path);
        self.lock()
            .written
            .iter()
            .find(|(declared, _)| {
                path.strip_prefix(declared.as_str())
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('.') || rest.starts_with('['))
            })
            .map(|(_, reason)| reason.clone())
    }

    /// Declares that the run creates the file `path` before this
    /// composition reads it, for `reason`: [`Self::declare_initializer_write`]
    /// with [`InitializerChange::CreatesIfAbsent`].
    pub fn defer_created(&self, path: &std::path::Path, reason: DeferralReason) {
        self.declare_initializer_write(path, InitializerChange::CreatesIfAbsent, reason);
    }

    /// Declares that the run writes the file `path` before this composition
    /// reads it, with the effect `change`, for `reason`. Every reader asks
    /// [`Self::initializer_changes`] (content), [`Self::created_file`]
    /// (existence), or [`Self::changes_listing`] (a folder listing) before it
    /// treats what it finds now as a verdict. Every session made with
    /// [`Self::sharing_observations`] shares the declaration. `path` is a
    /// location resolved in the request's context (as
    /// [`CreatedFile::target`](crate::lifecycle::CreatedFile::target)
    /// returns it); a relative path declares nothing.
    pub fn declare_initializer_write(&self, path: &std::path::Path, change: InitializerChange, reason: DeferralReason) {
        let Some(identity) = created::CreatedIdentity::of(path) else { return };
        let Some(lexical) = created::lexically_absolute(path) else { return };
        let mut created = self.created.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(held) = created.files.iter_mut().find(|held| held.identity == identity) {
            // An append anywhere in the stack makes the file's content
            // change, whatever else creates it first.
            if change == InitializerChange::CreatesOrAppends && held.change == InitializerChange::CreatesIfAbsent {
                held.change = change;
                held.reason = reason;
            }
            return;
        }
        created.files.push(created::Declared { identity, lexical, change, reason });
    }

    /// Declares a write whose destination this dry run cannot resolve
    /// (a template reading a value known only when `initialize` runs): every
    /// location `pattern` matches is treated as possibly written, with the
    /// effect `change`. The run never writes outside its mutation root, so
    /// a pattern never reaches beyond it.
    pub fn declare_unresolved_write(&self, pattern: WritePattern, change: InitializerChange, reason: DeferralReason) {
        let mut created = self.created.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if !created.patterns.iter().any(|(held, _, _)| *held == pattern) {
            created.patterns.push((pattern, change, reason));
        }
    }

    /// Why a read of the absent file `path` is unavailable, when the run
    /// may create it first: `path` names the same entry as a declared file
    /// once both are created, judged by the identity of the nearest
    /// existing folder above each and the names below it, compared without
    /// case on a volume that opens one name under another case, or it
    /// matches an unresolved destination. `None` for a file that exists:
    /// no initializer effect removes one, so its existence is proven.
    #[must_use]
    pub fn created_file(&self, path: &std::path::Path) -> Option<DeferralReason> {
        let created = self.created.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if created.is_empty() || std::fs::symlink_metadata(path).is_ok() {
            return None;
        }
        if let Some(identity) = created::CreatedIdentity::of(path)
            && let Some(held) = created.files.iter().find(|held| held.identity == identity)
        {
            return Some(held.reason.clone());
        }
        created.pattern_matching(path, false)
    }

    /// The first of `candidates` (a reference's probed locations, in
    /// resolution order) the run creates first ([`Self::created_file`]).
    #[must_use]
    pub fn created_among<P: AsRef<std::path::Path>>(&self, candidates: impl IntoIterator<Item = P>) -> Option<DeferralReason> {
        if self.created.lock().unwrap_or_else(std::sync::PoisonError::into_inner).is_empty() {
            return None;
        }
        candidates.into_iter().find_map(|candidate| self.created_file(candidate.as_ref()))
    }

    /// Why a verdict about what the file at `path` holds is unavailable,
    /// when the run's initializer changes it first: an absent file the run
    /// may create ([`Self::created_file`]), or an existing one a declared
    /// append targets ([`InitializerChange::CreatesOrAppends`]). An existing
    /// file that only an `ensure_file` names is read as it is: that effect
    /// leaves an existing file unchanged. The one rule every content reader
    /// (transclusion, data, frontmatter, schema, graph) asks.
    #[must_use]
    pub fn initializer_changes(&self, path: &std::path::Path) -> Option<DeferralReason> {
        {
            let created = self.created.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if created.is_empty() {
                return None;
            }
            if std::fs::symlink_metadata(path).is_ok() {
                if let Some(identity) = created::CreatedIdentity::of(path)
                    && let Some(held) = created
                        .files
                        .iter()
                        .find(|held| held.identity == identity && held.change == InitializerChange::CreatesOrAppends)
                {
                    let path = match &held.reason {
                        DeferralReason::CreatedAtLaunch { path } | DeferralReason::ChangedAtLaunch { path } => path.clone(),
                        _ => path.display().to_string(),
                    };
                    return Some(DeferralReason::ChangedAtLaunch { path });
                }
                return created.pattern_matching(path, true);
            }
        }
        self.created_file(path)
    }

    /// The first of `candidates` whose content the run's initializer
    /// changes first ([`Self::initializer_changes`]).
    #[must_use]
    pub fn initializer_changes_among<P: AsRef<std::path::Path>>(&self, candidates: impl IntoIterator<Item = P>) -> Option<DeferralReason> {
        if self.created.lock().unwrap_or_else(std::sync::PoisonError::into_inner).is_empty() {
            return None;
        }
        candidates.into_iter().find_map(|candidate| self.initializer_changes(candidate.as_ref()))
    }

    /// Why a listing of the files below `folders` that `matches` selects is
    /// unavailable, when the run's initializer may add an entry to it first:
    /// a declared destination that is absent now and that `matches`
    /// selects, or an unresolved destination that may lie below one of
    /// `folders`. Appending to a file that exists already changes no listing.
    #[must_use]
    pub fn changes_listing(&self, folders: &[std::path::PathBuf], matches: impl Fn(&std::path::Path) -> bool) -> Option<DeferralReason> {
        let created = self.created.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if created.is_empty() {
            return None;
        }
        if let Some(held) = created.files.iter().find(|held| std::fs::symlink_metadata(&held.lexical).is_err() && matches(&held.lexical)) {
            return Some(held.reason.clone());
        }
        let folders: Vec<std::path::PathBuf> = folders.iter().filter_map(|folder| created::lexically_absolute(folder)).collect();
        created
            .patterns
            .iter()
            .find(|(pattern, _, _)| folders.iter().any(|folder| pattern.may_lie_below(folder)))
            .map(|(_, _, reason)| reason.clone())
    }

    /// Why a `{{ … }}` span `text` holds was left as written, when one was:
    /// the reason of the first such span recorded. A directive line holding
    /// one names no target until the value is known.
    #[must_use]
    pub fn deferred_span_in(&self, text: &str) -> Option<DeferralReason> {
        self.lock()
            .deferrals
            .iter()
            .find(|deferral| deferral.construct == DeferredConstruct::Interpolation && text.contains(&deferral.authored))
            .map(|deferral| deferral.reason.clone())
    }

    /// Why the `prologue`/`epilogue` value `text` is not known, when it is
    /// not: it is a deferred value itself, or holds a `{{ … }}` span left as
    /// written. Such a value names no target.
    #[must_use]
    pub fn deferred_reference(&self, text: &str) -> Option<DeferralReason> {
        self.deferred_value(text).or_else(|| self.deferred_span_in(text))
    }

    /// Records `deferral`, once.
    pub fn record(&self, deferral: Deferral) {
        let mut state = self.lock();
        if state.seen.insert(deferral.clone()) {
            state.deferrals.push(deferral);
        }
    }

    /// Records that the frontmatter value `key`, whose authored text stays
    /// `authored`, is deferred for `reason`; later reads of that text are
    /// unavailable.
    pub(crate) fn defer_value(&self, key: &str, authored: &str, reason: DeferralReason, line: Option<usize>) {
        self.lock().values.entry(authored.to_string()).or_insert_with(|| reason.clone());
        self.record(Deferral {
            construct: DeferredConstruct::FrontmatterValue,
            reason,
            authored: authored.to_string(),
            key: Some(key.to_string()),
            line,
        });
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

mod created {
    //! The identity of a file that does not exist yet.

    use std::ffi::OsString;
    use std::path::{Component, Path, PathBuf};

    use sniff::filesystem::query::FileIdentity;

    use super::{DeferralReason, InitializerChange, WritePattern};

    /// One resolved destination an initializer writes.
    #[derive(Debug)]
    pub(super) struct Declared {
        pub(super) identity: CreatedIdentity,
        /// The destination, absolute with `.` and `..` folded.
        pub(super) lexical: PathBuf,
        pub(super) change: InitializerChange,
        pub(super) reason: DeferralReason,
    }

    /// Everything the run's initializer is declared to write.
    #[derive(Debug, Default)]
    pub(super) struct Initialized {
        pub(super) files: Vec<Declared>,
        pub(super) patterns: Vec<(WritePattern, InitializerChange, DeferralReason)>,
    }

    impl Initialized {
        pub(super) fn is_empty(&self) -> bool {
            self.files.is_empty() && self.patterns.is_empty()
        }

        /// The reason of the first unresolved destination `path` matches:
        /// for an existing file (`exists`), only one that appends.
        pub(super) fn pattern_matching(&self, path: &Path, exists: bool) -> Option<DeferralReason> {
            let path = lexically_absolute(path)?;
            self.patterns
                .iter()
                .find(|(pattern, change, _)| (!exists || *change == InitializerChange::CreatesOrAppends) && pattern.matches(&path))
                .map(|(_, change, reason)| match (exists, change, reason) {
                    (true, InitializerChange::CreatesOrAppends, DeferralReason::CreatedAtLaunch { path }) => {
                        DeferralReason::ChangedAtLaunch { path: path.clone() }
                    }
                    _ => reason.clone(),
                })
        }
    }

    /// Where a file will be once created: the nearest existing folder above
    /// it, by its OS identity (so `/var` and `/private/var`, a symlinked
    /// folder, or a case-only spelling of an existing folder all agree), and
    /// the names below that folder, compared without case when the folder's
    /// volume opens one name under another case (APFS, NTFS).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) struct CreatedIdentity {
        base: FileIdentity,
        rest: Vec<OsString>,
    }

    impl CreatedIdentity {
        /// The identity of `path`; `None` when no folder above it can be
        /// identified, or when `path` is relative. Every location compared
        /// here was resolved in a request's context (a declared target as
        /// the effect resolves it, a reader's probed candidates), so it is
        /// absolute; a relative one has no base this session captured, and
        /// the process directory is never read in its place.
        pub(super) fn of(path: &Path) -> Option<Self> {
            let path = lexically_absolute(path)?;
            let mut base = path.as_path();
            let mut rest = Vec::new();
            loop {
                if std::fs::symlink_metadata(base).is_ok() {
                    break;
                }
                rest.push(base.file_name()?.to_os_string());
                base = base.parent()?;
            }
            rest.reverse();
            let identity = FileIdentity::of_path(base, true).ok()?;
            if folds_case(base) {
                rest = rest.iter().map(|name| OsString::from(name.to_string_lossy().to_lowercase())).collect();
            }
            Some(Self { base: identity, rest })
        }
    }

    /// The absolute `path` with `.` and `..` folded; `None` when relative.
    pub(super) fn lexically_absolute(path: &Path) -> Option<PathBuf> {
        if !path.is_absolute() {
            return None;
        }
        let mut clean = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    clean.pop();
                }
                other => clean.push(other.as_os_str()),
            }
        }
        Some(clean)
    }

    /// Whether the volume holding the existing `folder` opens a name under
    /// another case: the folder, or the nearest folder above it with a
    /// cased name, reached with that name's case swapped is the same
    /// object. A folder with no cased name above it reads as case-sensitive.
    pub(super) fn folds_case(folder: &Path) -> bool {
        let mut probe = folder;
        loop {
            let (Some(name), Some(parent)) = (probe.file_name().and_then(|name| name.to_str()), probe.parent()) else {
                return false;
            };
            let swapped: String =
                name.chars().map(|c| if c.is_lowercase() { c.to_uppercase().next().unwrap_or(c) } else { c.to_lowercase().next().unwrap_or(c) }).collect();
            if swapped != name {
                return match (FileIdentity::of_path(probe, false), FileIdentity::of_path(&parent.join(swapped), false)) {
                    (Ok(found), Ok(swapped)) => found == swapped,
                    _ => false,
                };
            }
            probe = parent;
        }
    }
}

/// What an initializer's write does to its destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InitializerChange {
    /// `ensure_file`: creates an absent file and leaves an existing one
    /// unchanged, so only an absent file's existence and content change.
    CreatesIfAbsent,
    /// `append_line`, `append_jsonl`: creates an absent file and appends to
    /// an existing one, so an existing file's content changes too.
    CreatesOrAppends,
}

/// The locations a write may reach when its destination cannot be resolved
/// before the run: the destination's literal text in order, with anything
/// between consecutive pieces (a value known only at launch). Matched
/// against an absolute location with `.` and `..` folded, by text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WritePattern {
    pieces: Vec<String>,
}

impl WritePattern {
    /// The pattern of `pieces`: the first is the location's start (always
    /// an absolute folder the write cannot leave, such as the mutation
    /// root), the last its end, and every piece follows the one before it.
    /// `None` without a first piece.
    #[must_use]
    pub fn new(pieces: Vec<String>) -> Option<Self> {
        if pieces.first().is_none_or(String::is_empty) {
            return None;
        }
        Some(Self { pieces })
    }

    /// Whether `path` (absolute, `.` and `..` folded) is a location the
    /// write may reach. Compared without case when the volume holding the
    /// pattern's start opens one name under another case (APFS, NTFS), as
    /// a declared file's identity is.
    #[must_use]
    pub fn matches(&self, path: &std::path::Path) -> bool {
        let fold = self.folds_case();
        let fit = |text: &str| {
            let text = separators_native(text);
            if fold { text.to_lowercase() } else { text }
        };
        let text = fit(&path.to_string_lossy());
        let pieces: Vec<String> = self.pieces.iter().map(|piece| fit(piece)).collect();
        let Some(mut rest) = text.strip_prefix(pieces[0].as_str()) else { return false };
        let Some((last, middle)) = pieces[1..].split_last() else { return true };
        for piece in middle {
            match rest.find(piece.as_str()) {
                Some(at) => rest = &rest[at + piece.len()..],
                None => return false,
            }
        }
        rest.ends_with(last.as_str())
    }

    /// Whether a location the write may reach lies below `folder`: the
    /// pattern's fixed start and `folder` are on one branch of the tree,
    /// compared without case where [`Self::matches`] is.
    #[must_use]
    pub fn may_lie_below(&self, folder: &std::path::Path) -> bool {
        // The fixed start may end inside a name (`logs/run-`), so compare
        // its folder.
        let start_folder = if self.pieces[0].ends_with(std::path::MAIN_SEPARATOR) {
            std::path::PathBuf::from(&self.pieces[0])
        } else {
            std::path::Path::new(&self.pieces[0]).parent().map_or_else(|| std::path::PathBuf::from(&self.pieces[0]), std::path::Path::to_path_buf)
        };
        let fold = self.folds_case();
        let fit = |path: &std::path::Path| {
            let text = separators_native(&path.to_string_lossy());
            std::path::PathBuf::from(if fold { text.to_lowercase() } else { text })
        };
        let (start_folder, folder) = (fit(&start_folder), fit(folder));
        folder.starts_with(&start_folder) || start_folder.starts_with(&folder)
    }

    /// Whether the volume holding the pattern's start (the nearest existing
    /// folder at or above it) opens one name under another case.
    fn folds_case(&self) -> bool {
        let mut probe = std::path::Path::new(&self.pieces[0]);
        loop {
            if std::fs::symlink_metadata(probe).is_ok() {
                return created::folds_case(probe);
            }
            match probe.parent() {
                Some(parent) => probe = parent,
                None => return false,
            }
        }
    }
}

/// `text` with every separator the OS accepts written as its own: on
/// Windows a location joined from `/`-spelled text (`docs/logs`) names the
/// same entry as its `\` spelling, so the two compare equal as text.
fn separators_native(text: &str) -> String {
    if cfg!(windows) { text.replace('/', "\\") } else { text.to_string() }
}

/// The deferral reason of reading `path`, whose value is `value`, in a dry
/// run: the value is the authored text of a deferred value. `None` outside a
/// dry run and for every other value.
pub(crate) fn unavailable_read(
    session: Option<&PassiveSession>,
    path: &str,
    value: Option<&serde_json::Value>,
) -> Option<DeferralReason> {
    let text = value?.as_str()?;
    session?.deferred_value(text)?;
    Some(DeferralReason::DependsOn { name: path.to_string() })
}

/// Refuses a read of `path`, which `lookup` resolved to `binding`, when it
/// reads a deferred value: the value itself, or a member below one.
///
/// ## Errors
///
/// [`ExpressionError::Unavailable`] for a read of a deferred value in a dry
/// run.
pub(crate) fn check_read<L: crate::markdown::compose::expression::EvaluationLookup + ?Sized>(
    lookup: &L,
    path: &str,
    binding: &crate::markdown::compose::expression::ResolvedBinding,
) -> Result<(), ExpressionError> {
    let Some(session) = session_of(lookup) else {
        return Ok(());
    };
    check_value(session, lookup, path, binding.value())?;
    if let Some(root) = dotted_root(path) {
        let root_path = if path.starts_with("fm.") { format!("fm.{root}") } else { root.to_string() };
        if let Ok(root_binding) = lookup.resolve(&root_path)
            && unavailable_read(Some(session), &root_path, root_binding.value()).is_some()
        {
            return Err(ExpressionError::Unavailable(Box::new(DeferralReason::DependsOn { name: root_path })));
        }
    }
    Ok(())
}

/// Refuses a member or index selection that selected `path` (the variable
/// it read, then each member and computed key: `seq['outputs']` and
/// `(seq).outputs` select `seq.outputs`) and found `value`, when the
/// selection reads a deferred value. The rule is [`check_read`]'s, applied
/// to what the selection selected, so a read's spelling never decides it.
///
/// ## Errors
///
/// [`ExpressionError::Unavailable`] for a selection of a deferred value in
/// a dry run.
pub(crate) fn check_selected<L: crate::markdown::compose::expression::EvaluationLookup + ?Sized>(
    lookup: &L,
    path: &str,
    value: &serde_json::Value,
) -> Result<(), ExpressionError> {
    match session_of(lookup) {
        Some(session) => check_value(session, lookup, path, Some(value)),
        None => Ok(()),
    }
}

/// Whether `lookup` evaluates in a dry run, where a read can be deferred.
pub(crate) fn is_active<L: crate::markdown::compose::expression::EvaluationLookup + ?Sized>(lookup: &L) -> bool {
    session_of(lookup).is_some()
}

fn session_of<L: crate::markdown::compose::expression::EvaluationLookup + ?Sized>(lookup: &L) -> Option<&PassiveSession> {
    lookup.resolution_context_ref().and_then(|context| context.passive.as_ref())
}

/// The checks of a read of `path` that found `value`: a deferred value's
/// text, a value the run writes, and a value declared absent.
fn check_value<L: crate::markdown::compose::expression::EvaluationLookup + ?Sized>(
    session: &PassiveSession,
    lookup: &L,
    path: &str,
    value: Option<&serde_json::Value>,
) -> Result<(), ExpressionError> {
    if let Some(reason) = unavailable_read(Some(session), path, value) {
        return Err(ExpressionError::Unavailable(Box::new(reason)));
    }
    if let Some(reason) = session.written_value(path)
        && !reads_shadowed_property(lookup, path)
    {
        return Err(ExpressionError::Unavailable(Box::new(reason)));
    }
    // A value declared absent has no binding to read; a read that finds one
    // (a default, a setter) is an ordinary read.
    let top = path.strip_prefix("fm.").unwrap_or(path);
    let top = top.split(['.', '[']).next().unwrap_or(top);
    if value.is_none_or(serde_json::Value::is_null)
        && let Some(reason) = session.absent_value(top)
    {
        return Err(ExpressionError::Unavailable(Box::new(reason)));
    }
    Ok(())
}

/// Whether `path` is an explicit `fm.` read of a document property whose
/// bare name a host global takes: `fm.seq.outputs` reads the authored
/// `seq:`, while a written path declared as `seq.outputs` names what bare
/// `seq.outputs` reads, the host's `seq`.
fn reads_shadowed_property<L: crate::markdown::compose::expression::EvaluationLookup + ?Sized>(
    lookup: &L,
    path: &str,
) -> bool {
    let Some(rest) = path.strip_prefix("fm.") else {
        return false;
    };
    let root = rest.split(['.', '[']).next().unwrap_or(rest);
    matches!(lookup.resolve(root), Ok(crate::markdown::compose::expression::ResolvedBinding::Global { .. }))
}

/// The top-level value a dotted `path` reads below (`fm.x.y` and `x.y` both
/// read below `x`); `None` for a path that names a top-level value itself.
pub(crate) fn dotted_root(path: &str) -> Option<&str> {
    let path = path.strip_prefix("fm.").unwrap_or(path);
    path.split_once('.').map(|(root, _)| root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unresolved_destination_matches_another_case_only_where_the_volume_folds_case() {
        let root = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(root.path()).expect("canonical root");
        let folder = root.join("Gen");
        std::fs::create_dir(&folder).expect("gen folder");
        let mut start = folder.to_string_lossy().to_string();
        start.push(std::path::MAIN_SEPARATOR);
        let pattern = WritePattern::new(vec![start, ".md".into()]).expect("pattern");
        let other_case = root.join("gen").join("notes.md");
        let folds = created::folds_case(&folder);

        assert!(pattern.matches(&folder.join("notes.md")));
        assert!(!pattern.matches(&root.join("other").join("notes.md")));
        assert_eq!(pattern.matches(&other_case), folds, "case folding of {}", folder.display());
        assert_eq!(pattern.may_lie_below(&root.join("gen")), folds);
        assert!(pattern.may_lie_below(&folder));
    }

    #[test]
    fn a_deferral_is_recorded_once_and_its_value_becomes_unavailable() {
        let session = PassiveSession::new();
        let reason = DeferralReason::Command { command: "git branch".into() };
        session.defer_value("branch", "$(git branch)", reason.clone(), Some(2));
        session.defer_value("branch", "$(git branch)", reason, Some(2));

        assert_eq!(session.deferrals().len(), 1);
        let shared = session.clone();
        assert!(shared.same_session(&session));
        assert_eq!(
            unavailable_read(Some(&shared), "fm.branch", Some(&serde_json::json!("$(git branch)"))),
            Some(DeferralReason::DependsOn { name: "fm.branch".into() })
        );
        assert_eq!(unavailable_read(Some(&shared), "title", Some(&serde_json::json!("x"))), None);
        assert_eq!(unavailable_read(None, "branch", Some(&serde_json::json!("$(git branch)"))), None);
    }

    #[test]
    fn a_written_value_covers_its_members_and_nothing_beside_it() {
        let session = PassiveSession::new();
        let reason = DeferralReason::WrittenAtLaunch { name: "seq.outputs".into() };
        session.defer_written("seq.outputs", reason.clone());

        for read in ["seq.outputs", "seq.outputs[0]", "seq.outputs.0", "fm.seq.outputs"] {
            assert_eq!(session.written_value(read), Some(reason.clone()), "{read}");
        }
        for read in ["seq", "seq.index", "seq.outputs_count", "outputs"] {
            assert_eq!(session.written_value(read), None, "{read}");
        }
    }

    #[test]
    fn a_dotted_read_names_its_top_level_value() {
        assert_eq!(dotted_root("fm.server.name"), Some("server"));
        assert_eq!(dotted_root("server.name"), Some("server"));
        assert_eq!(dotted_root("fm.server"), None);
        assert_eq!(dotted_root("server"), None);
    }
}
