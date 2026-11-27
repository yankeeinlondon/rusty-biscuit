//! The read gate a run's slot evaluations consult: the captured
//! `permissions`, this run's approvals, and (for a read nothing decides) the
//! approval handler.
//!
//! For each read a slot is about to make, or made, judged from the document
//! doing the read (the slot's defining schema file, or for an inline
//! `$schema` the request's document):
//!
//! 1. invalid `permissions` refuse every read with the configuration's own
//!    error, never allowing anything;
//! 2. a listing (a folder or a glob) is allowed: it needs no approval of its
//!    own, and returns only the files [`SlotGrants::admits_listed`] admits;
//! 3. a deny for this run refuses it (`dm.schema.slot_denied`), and an allow
//!    for this run allows it, whatever `permissions` says (so an approval
//!    overrides a `!` exclusion);
//! 4. a file `permissions.file-read` admits, or a URL the network
//!    permission or `--allow host=HOST` allows, is allowed; any other URL
//!    under a captured `network: deny` is refused
//!    (`dm.schema.slot_network_denied`), and under `network: ask` goes on;
//! 5. the approval handler, when there is one, is asked: allow for this run,
//!    "always allow" (a file, saved to the user's `permissions.file-read`
//!    and allowed for this run whether or not the save succeeds), or deny
//!    for this run;
//! 6. otherwise the read needs approval (`dm.schema.slot_approval_required`),
//!    naming the approvals that would allow it.
//!
//! The captured configuration is never reloaded mid-run: a saved approval
//! takes effect for this run as a current-run approval. A save that fails
//! (an exclusion still matches, the file is outside the home, there is no
//! home, the write fails) is reported as a `dm.config.not_saved` warning.
//!
//! A slot's URL reads go through the slots' own remote runtime
//! ([`SlotGrants::remote_runtime`]), whose host policy is the network
//! permission's projection over the hosts the URL approvals name and the
//! `--allow host=HOST` values; the request's document fetch policy is never
//! widened. The gate checks scheme, port, path, and query before the fetch,
//! because a fetch policy matches hosts only.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use biscuit_file::FileResolutionContext;
use biscuit_file::file_reference::fetch::FetchPolicy;

use super::request::SlotGrantRequest;
use super::{Grant, GrantBook, GrantDecision, GrantScope, GrantTarget, SlotGrantDecision};
use crate::config::load::{CONFIG_NOT_SAVED_CODE, ConfigError};
use crate::config::permissions::PermissionPolicy;
use crate::config::permissions::persist::{PersistentApproval, save_user_approval};
use crate::markdown::compose::ComposeWarning;
use crate::markdown::compose::expression::functions::ReadKind;
use crate::markdown::compose::expression::read_gate::{GateRefusal, ReadGate, ReadLocation, ReadTarget};
use crate::markdown::compose::remote_fetch::RemoteFetchRuntime;
use crate::markdown::compose::shell_expansion::ShellApprovalHandler;
use crate::markdown::schemas::slots::evaluator::SlotGates;

/// The stage a slot's `dm.config.not_saved` warning reports under.
const NOT_SAVED_STAGE: &str = "schema-slots";

/// What the run's configuration says about file reads.
#[derive(Debug, Clone)]
pub(crate) enum SlotPermissions {
    /// No configuration was captured (a bare [`SlotSession::new`](crate::markdown::schemas::slots::SlotSession::new)):
    /// nothing is pre-approved and nothing can be saved.
    Unconfigured,
    /// The captured permission policy.
    Captured(Arc<PermissionPolicy>),
    /// The `permissions` block is invalid: every read is refused with it.
    Invalid(ConfigError),
}

impl SlotPermissions {
    /// The permissions `snapshot` captured.
    pub(crate) fn of(snapshot: &crate::config::load::ConfigSnapshot) -> Self {
        match snapshot.policy() {
            Ok(policy) => Self::Captured(Arc::clone(policy)),
            Err(error) => Self::Invalid(error),
        }
    }
}

/// The `dm.config.not_saved` warnings a run's "always allow" answers left,
/// shared between the run's grants and the session that reports them.
pub(crate) type NotSavedWarnings = Arc<Mutex<Vec<ComposeWarning>>>;

/// The approvals and permissions one run decides slot reads by.
pub(crate) struct SlotGrants {
    book: Mutex<GrantBook>,
    permissions: SlotPermissions,
    handler: Option<Arc<dyn ShellApprovalHandler>>,
    not_saved: NotSavedWarnings,
}

impl std::fmt::Debug for SlotGrants {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlotGrants")
            .field("book", &self.book)
            .field("permissions", &self.permissions)
            .field("handler", &self.handler.is_some())
            .finish_non_exhaustive()
    }
}

impl SlotGrants {
    /// Approvals from `book`, pre-approval from `permissions`, `handler` for
    /// a read nothing else decides, and `not_saved` collecting the warnings
    /// of approvals that could not be saved.
    pub(crate) fn new(
        book: GrantBook,
        permissions: SlotPermissions,
        handler: Option<Arc<dyn ShellApprovalHandler>>,
        not_saved: NotSavedWarnings,
    ) -> Self {
        Self { book: Mutex::new(book), permissions, handler, not_saved }
    }

    /// A copy of the approvals in force now.
    pub(crate) fn book(&self) -> GrantBook {
        self.book.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Whether an "always allow" can be offered: the permissions are valid,
    /// so an approval can be saved to them.
    pub(crate) fn can_persist(&self) -> bool {
        matches!(self.permissions, SlotPermissions::Captured(_))
    }

    /// The decision for `target`, read by the document `reading`, from the
    /// permissions and this run's approvals alone, never asking anyone:
    /// `Some(Ok)` allowed, `Some(Err)` refused, `None` undecided.
    pub(crate) fn decide_without_asking(
        &self,
        reading: &FileResolutionContext,
        target: &ReadTarget,
    ) -> Option<Result<(), GateRefusal>> {
        if let SlotPermissions::Invalid(error) = &self.permissions {
            return Some(Err(GateRefusal::InvalidPermissions(error.clone())));
        }
        let target = canonical(target);
        let path = match &target.location {
            ReadLocation::Glob(_) => return Some(Ok(())),
            ReadLocation::Local(_) if target.kind == ReadKind::Listing => return Some(Ok(())),
            ReadLocation::Local(path) => Some(path),
            ReadLocation::Remote(_) => None,
        };
        let book = self.book.lock().unwrap_or_else(PoisonError::into_inner);
        // An invalid network permission refuses a URL before any approval.
        if path.is_none()
            && let Some(error) = book.network().config_error()
        {
            return Some(Err(GateRefusal::InvalidPermissions(error.clone())));
        }
        match book.decide(&target.location) {
            Some(GrantDecision::Deny) => return Some(Err(GateRefusal::Denied)),
            Some(GrantDecision::Allow) => return Some(Ok(())),
            None => {}
        }
        match path {
            Some(path) => match &self.permissions {
                SlotPermissions::Captured(policy) => policy.file_read(path, reading).is_preapproved().then_some(Ok(())),
                SlotPermissions::Unconfigured | SlotPermissions::Invalid(_) => None,
            },
            None if book.network_allows(&target.location) => Some(Ok(())),
            // A captured `network: deny` refuses outright; `ask` (and a
            // session with no configuration) leaves the URL to approval.
            None if matches!(self.permissions, SlotPermissions::Captured(_))
                && matches!(book.network(), crate::config::network::NetworkPermission::Deny) =>
            {
                Some(Err(GateRefusal::NetworkDenied))
            }
            None => None,
        }
    }

    /// Whether a local `target`, compared as written, could be allowed
    /// without asking: an allow for this run naming exactly its lexical path,
    /// or a `permissions.file-read` allow matching it lexically with no
    /// lexical exclusion. Never touches the filesystem and grants nothing;
    /// see [`ReadGate::admits_lexically`]. A URL, a listing, and a glob are
    /// left to [`Self::decide_without_asking`].
    pub(crate) fn may_admit_lexically(&self, reading: &FileResolutionContext, target: &ReadTarget) -> bool {
        let ReadLocation::Local(path) = &target.location else {
            return true;
        };
        if target.kind == ReadKind::Listing {
            return true;
        }
        let policy = match &self.permissions {
            SlotPermissions::Invalid(_) => return false,
            SlotPermissions::Captured(policy) => Some(policy),
            SlotPermissions::Unconfigured => None,
        };
        let absolute = if path.is_absolute() { path.clone() } else { reading.cwd().join(path) };
        let lexical = crate::config::permissions::lexically_normal(&absolute);
        // The same name under its folder's canonical spelling (macOS `/var`
        // is `/private/var`), where approvals are recorded. Only the folder
        // is resolved, never the target itself.
        let spellings: Vec<PathBuf> = match (lexical.parent(), lexical.file_name()) {
            (Some(parent), Some(name)) if !parent.as_os_str().is_empty() => {
                let folder = canonical_target(parent).join(name);
                if folder == lexical { vec![lexical] } else { vec![lexical, folder] }
            }
            _ => vec![lexical],
        };
        let book = self.book.lock().unwrap_or_else(PoisonError::into_inner);
        if spellings.iter().any(|spelling| book.decide(&ReadLocation::Local(spelling.clone())) == Some(GrantDecision::Allow)) {
            return true;
        }
        policy.is_some_and(|policy| spellings.iter().any(|spelling| policy.may_allow_file_read_lexically(spelling, reading)))
    }

    /// Whether a file a listing returned may reach a slot reading from
    /// `reading`: only when the permissions or this run's approvals allow it
    /// without asking.
    pub(crate) fn admits_listed(&self, reading: &FileResolutionContext, target: &ReadTarget) -> bool {
        matches!(self.decide_without_asking(reading, target), Some(Ok(())))
    }

    /// Applies a handler's `decision`, when it is one of `request`'s choices:
    /// it lasts for the run, and an "always allow" is also saved to the
    /// user's `permissions.file-read` (judged from `reading`). Returns
    /// whether an approval was applied.
    pub(crate) fn apply(&self, request: &SlotGrantRequest, decision: &SlotGrantDecision, reading: &FileResolutionContext) -> bool {
        let Some(grant) = decision.grant().filter(|grant| request.choices.contains(grant)) else {
            return false;
        };
        self.apply_grant(&grant, reading);
        true
    }

    /// Applies `grant` for the rest of the run, saving an "always allow"
    /// first. A grant that is not [applicable](Grant::is_applicable) is
    /// ignored.
    pub(crate) fn apply_grant(&self, grant: &Grant, reading: &FileResolutionContext) {
        if !grant.is_applicable() {
            return;
        }
        if grant.scope == GrantScope::Persisted
            && let GrantTarget::File(path) = &grant.target
        {
            self.save(path, reading);
        }
        self.book.lock().unwrap_or_else(PoisonError::into_inner).add_once(grant.decision, grant.target.clone());
    }

    /// Saves an "always allow" of `path` to the user's permissions; see
    /// [`save_file_approval`].
    fn save(&self, path: &Path, reading: &FileResolutionContext) {
        save_file_approval(&self.permissions, path, reading, &self.not_saved);
    }

    /// One request per read the slots of `templates` name before evaluation
    /// (literal or pure targets) that nothing decides yet, in slot order,
    /// each target once, with the context of the document doing the read.
    /// Reads nothing a slot names. A listing is never asked about.
    pub(crate) fn undecided_requests(
        &self,
        templates: &[crate::markdown::schemas::PendingTemplate],
        context: &FileResolutionContext,
    ) -> Vec<(SlotGrantRequest, FileResolutionContext)> {
        use super::targets::{CollectedRead, collect_slot_targets};
        let mut requests: Vec<(SlotGrantRequest, FileResolutionContext)> = Vec::new();
        for template in templates {
            for slot in collect_slot_targets(&template.slots, context) {
                let reading = reading_context(context, &slot.defining);
                for read in &slot.reads {
                    let CollectedRead::Known(target) = read else {
                        continue;
                    };
                    if self.decide_without_asking(&reading, target).is_some() {
                        continue;
                    }
                    let asked = SlotGrantRequest::for_target(&canonical(target), slot.slot.origin.as_deref(), self.can_persist());
                    if asked.choices.is_empty() {
                        continue;
                    }
                    if !requests.iter().any(|(known, _)| known.kind == asked.kind && known.location == asked.location) {
                        requests.push((asked, reading.clone()));
                    }
                }
            }
        }
        requests
    }

    /// Applies a handler's answers to `requests`, in order.
    pub(crate) fn apply_all(&self, requests: &[(SlotGrantRequest, FileResolutionContext)], decisions: &[SlotGrantDecision]) {
        for ((asked, reading), decision) in requests.iter().zip(decisions) {
            self.apply(asked, decision, reading);
        }
    }

    /// The fetch policy the slots' remote runtime uses: the network
    /// permission's [`fetch_policy`](crate::config::network::NetworkPermission::fetch_policy)
    /// over the hosts this run's URL allows name and the command's
    /// `--allow host=HOST` values.
    pub(crate) fn fetch_policy(&self) -> FetchPolicy {
        let book = self.book.lock().unwrap_or_else(PoisonError::into_inner);
        let mut hosts: Vec<String> = book
            .once()
            .iter()
            .filter(|grant| grant.decision == GrantDecision::Allow)
            .filter_map(|grant| match &grant.target {
                GrantTarget::Url(url) => url.host_str().map(str::to_string),
                _ => None,
            })
            .chain(book.hosts().iter().cloned())
            .collect();
        hosts.sort();
        hosts.dedup();
        book.network().fetch_policy(hosts)
    }

    /// A remote runtime for slot reads under [`Self::fetch_policy`], with
    /// the command's concurrency, freshness, and transport cache when the
    /// command has a remote runtime (`document`), else the defaults with no
    /// cache. Build it after pre-flight's approvals are applied; a URL
    /// approved later in the run is fetched by a runtime built after that
    /// approval.
    pub(crate) fn remote_runtime(&self, document: Option<&RemoteFetchRuntime>) -> RemoteFetchRuntime {
        let policy = self.fetch_policy();
        match document {
            Some(document) => document.with_other_policy(policy),
            None => RemoteFetchRuntime::with_store(&Default::default(), None).with_other_policy(policy),
        }
    }

    /// Asks the handler about `request`, applying its answer; `None` when
    /// there is no handler or the answer decided nothing.
    fn ask(&self, request: &SlotGrantRequest, reading: &FileResolutionContext) -> Option<Result<(), GateRefusal>> {
        let handler = self.handler.as_ref()?;
        let decision = handler
            .approve_slot_reads(std::slice::from_ref(request))
            .ok()
            .and_then(|mut decisions| decisions.drain(..).next())
            .unwrap_or(SlotGrantDecision::Unanswered);
        self.apply(request, &decision, reading);
        match self.book.lock().unwrap_or_else(PoisonError::into_inner).decide(&request.location) {
            Some(GrantDecision::Allow) => Some(Ok(())),
            Some(GrantDecision::Deny) => Some(Err(GateRefusal::Denied)),
            None => None,
        }
    }

    /// The full decision for one read by the document `reading` (`schema` is
    /// the slot's schema file, when it has one).
    fn check(&self, reading: &FileResolutionContext, schema: Option<&Path>, target: &ReadTarget) -> Result<(), GateRefusal> {
        let target = canonical(target);
        if let Some(decided) = self.decide_without_asking(reading, &target) {
            return decided;
        }
        let request = SlotGrantRequest::for_target(&target, schema, self.can_persist());
        if let Some(answered) = self.ask(&request, reading) {
            return answered;
        }
        Err(GateRefusal::ApprovalRequired {
            needed: request.choices.into_iter().filter(|grant| grant.decision == GrantDecision::Allow).collect(),
        })
    }
}

/// Pre-flight's half of slot approval: asks `handler` once, in one batch,
/// about every read the slots of `templates` name before evaluation that
/// neither the request's `permissions` nor its network consent decide, and
/// returns the approvals it gave (an "always allow" kept as
/// [`GrantScope::Persisted`], for the session that receives them to save)
/// and how many reads were asked about.
///
/// Passive: nothing is resolved and nothing a slot names is read. Without a
/// handler nothing is asked. Invalid `permissions` ask nothing; each read
/// then fails with the configuration's error when its slot resolves.
///
/// ## Errors
///
/// The handler's own failure.
pub(crate) fn preflight_slot_grants(
    request: &crate::markdown::compose::ComposeRequest,
    templates: &[crate::markdown::schemas::PendingTemplate],
    handler: Option<&dyn ShellApprovalHandler>,
) -> Result<(Vec<Grant>, usize), crate::markdown::compose::ShellExpansionError> {
    let options = request.options();
    let hosts = options.remote_read_config.http_hosts().cloned().collect();
    let book = GrantBook::new(hosts).with_network(options.network_permission());
    let permissions = SlotPermissions::of(request.config_snapshot());
    let grants = SlotGrants::new(book, permissions, None, NotSavedWarnings::default());
    let requests = grants.undecided_requests(templates, request.resolution_context());
    let Some(handler) = handler.filter(|_| !requests.is_empty()) else {
        return Ok((Vec::new(), requests.len()));
    };
    let asked: Vec<SlotGrantRequest> = requests.iter().map(|(request, _)| request.clone()).collect();
    let decisions = handler.approve_slot_reads(&asked)?;
    let given = asked
        .iter()
        .zip(&decisions)
        .filter_map(|(request, decision)| decision.grant().filter(|grant| request.choices.contains(grant)))
        .collect();
    Ok((given, asked.len()))
}

/// Saves an "always allow" of the file `path`, read by the document
/// `reading`, to the user's `permissions.file-read` as an exact portable
/// entry. A save that fails (an exclusion still matches, the file is
/// outside the home, there is no home, the write fails) leaves a
/// `dm.config.not_saved` warning in `not_saved`; without captured
/// permissions nothing is saved.
pub(crate) fn save_file_approval(
    permissions: &SlotPermissions,
    path: &Path,
    reading: &FileResolutionContext,
    not_saved: &NotSavedWarnings,
) {
    let SlotPermissions::Captured(policy) = permissions else {
        return;
    };
    let approval = PersistentApproval::FileRead { target: path.to_path_buf(), reading: Box::new(reading.clone()) };
    match save_user_approval(policy, &approval) {
        Ok(saved) => tracing::info!(file = %saved.file.display(), entry = %saved.entry, "schema-slot read approval saved"),
        Err(error) => {
            let mut warning = ComposeWarning::new(
                NOT_SAVED_STAGE,
                format!("always allowing `{}`: {error}", biscuit_file::to_forward_slash_string(path)),
            );
            warning.code = Some(CONFIG_NOT_SAVED_CODE.to_string());
            warning.path.clone_from(&error.file);
            not_saved.lock().unwrap_or_else(PoisonError::into_inner).push(warning);
        }
    }
}

/// The context of the document a slot defined at `defining` reads from:
/// the request context derived for its schema file, or the request context
/// itself for an inline `$schema` (whose `defining` is a folder).
pub(crate) fn reading_context(context: &FileResolutionContext, defining: &Path) -> FileResolutionContext {
    super::super::evaluator::slot_context(context, defining.is_file().then_some(defining))
}

/// `target` with its local path canonicalized, as approvals and permissions
/// compare it.
pub(crate) fn canonical(target: &ReadTarget) -> ReadTarget {
    match &target.location {
        ReadLocation::Local(path) => {
            ReadTarget { location: ReadLocation::Local(canonical_target(path)), ..target.clone() }
        }
        _ => target.clone(),
    }
}

/// `path` canonicalized for comparison: an existing path through
/// [`biscuit_file::canonicalize_simplified`] (symlinks resolved), a
/// nonexistent one by its canonical parent and its file name, and the path
/// itself when neither exists.
pub(crate) fn canonical_target(path: &Path) -> PathBuf {
    if let Ok(canonical) = biscuit_file::canonicalize_simplified(path) {
        return canonical;
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) if !parent.as_os_str().is_empty() => canonical_target(parent).join(name),
        _ => path.to_path_buf(),
    }
}

/// The gate for the slots defined at one place.
#[derive(Debug)]
struct SlotReadGate {
    grants: Arc<SlotGrants>,
    reading: FileResolutionContext,
    schema: Option<PathBuf>,
}

impl ReadGate for SlotReadGate {
    fn check(&self, target: &ReadTarget) -> Result<(), GateRefusal> {
        self.grants.check(&self.reading, self.schema.as_deref(), target)
    }

    fn admits_listed(&self, target: &ReadTarget) -> bool {
        self.grants.admits_listed(&self.reading, target)
    }
}

/// Every slot of a resolution asks the run's one [`SlotGrants`], judged from
/// the document it reads from in the resolution's request `context`.
#[derive(Debug, Clone)]
pub(crate) struct RunGates {
    pub(crate) grants: Arc<SlotGrants>,
    pub(crate) context: FileResolutionContext,
}

impl SlotGates for RunGates {
    fn gate_for(&self, defining: &Path) -> Arc<dyn ReadGate> {
        let schema = defining.is_file().then(|| defining.to_path_buf());
        Arc::new(SlotReadGate {
            grants: Arc::clone(&self.grants),
            reading: reading_context(&self.context, defining),
            schema,
        })
    }
}
