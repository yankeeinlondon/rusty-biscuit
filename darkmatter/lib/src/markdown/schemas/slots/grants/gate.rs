//! The read gate a run's slot evaluations consult: the run's grants, the
//! automatic grant, and (for a read nobody approved up front) the approval
//! handler.
//!
//! For each read a slot is about to make, or made, in this order:
//!
//! 1. a deny grant covering it refuses it (`dm.schema.slot_denied`);
//! 2. an allow grant (persisted, for this run, or `--allow host=HOST`)
//!    allows it;
//! 3. the automatic grant ([`super::auto`]) allows it, decided once per run
//!    for each target, so a file the run itself later changes keeps the
//!    grant it had;
//! 4. the approval handler, when there is one, is asked, and its answer is
//!    applied as in pre-flight;
//! 5. otherwise the read needs approval (`dm.schema.slot_approval_required`),
//!    naming the grants that would allow it.
//!
//! A slot's URL reads go through the slots' own remote runtime
//! ([`SlotGrants::remote_runtime`]), whose host policy is exactly the hosts
//! the grants name; the request's document fetch policy is never widened.
//! The gate checks scheme, port, path, and query before the fetch, because a
//! fetch policy matches hosts only.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use biscuit_file::file_reference::fetch::{FetchPolicy, HostPattern};

use super::auto::{AutoVerdict, RepositoryGrounds, canonical_target};
use super::request::SlotGrantRequest;
use super::{Grant, GrantBook, GrantDecision, GrantScope, GrantStoreWriteError, GrantTarget, SlotGrantDecision};
use crate::markdown::compose::expression::functions::ReadKind;
use crate::markdown::compose::expression::read_gate::{GateRefusal, ReadGate, ReadLocation, ReadTarget};
use crate::markdown::compose::remote_fetch::RemoteFetchRuntime;
use crate::markdown::compose::shell_expansion::ShellApprovalHandler;
use crate::markdown::schemas::slots::evaluator::SlotGates;

/// An automatic grant decided this run, and whether the slot's own read
/// has been seen to reproduce the bytes it was granted for.
#[derive(Debug, Clone, Copy)]
struct AutoGrant {
    expected: Option<[u8; 32]>,
    verified: bool,
}

/// The key an automatic decision is kept under: whether the slot was
/// written inside the repository, the read's kind, and its canonical
/// location.
type AutoKey = (bool, ReadKind, ReadLocation);

/// The grants one run decides slot reads by.
pub(crate) struct SlotGrants {
    book: Mutex<GrantBook>,
    grounds: RepositoryGrounds,
    automatic: Mutex<HashMap<AutoKey, Option<AutoGrant>>>,
    handler: Option<Arc<dyn ShellApprovalHandler>>,
}

impl std::fmt::Debug for SlotGrants {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlotGrants")
            .field("book", &self.book)
            .field("grounds", &self.grounds)
            .field("handler", &self.handler.is_some())
            .finish_non_exhaustive()
    }
}

// The slot session evaluates through the run's gates; until it lands only
// pre-flight (which never asks through the gate) and tests use the rest.
#[cfg_attr(not(test), allow(dead_code))]
impl SlotGrants {
    /// Grants from `book`, automatic grants against the repository at
    /// `repository_root` (the request's captured observation), and
    /// `handler` for a read nothing else decides.
    pub(crate) fn new(
        book: GrantBook,
        repository_root: Option<&Path>,
        handler: Option<Arc<dyn ShellApprovalHandler>>,
    ) -> Self {
        Self {
            book: Mutex::new(book),
            grounds: RepositoryGrounds::new(repository_root),
            automatic: Mutex::new(HashMap::new()),
            handler,
        }
    }

    /// A copy of the grants in force now.
    pub(crate) fn book(&self) -> GrantBook {
        self.book.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Whether the run can persist a grant (there is a grant store).
    pub(crate) fn can_persist(&self) -> bool {
        self.book.lock().unwrap_or_else(PoisonError::into_inner).store().path().is_some()
    }

    /// The decision for `target` from grants and the automatic grant alone,
    /// never asking the handler: `Some(Ok)` allowed, `Some(Err(Denied))`
    /// denied, `None` undecided.
    pub(crate) fn decide_without_asking(&self, defining: &Path, target: &ReadTarget) -> Option<Result<(), GateRefusal>> {
        let target = canonical(target);
        match self.book.lock().unwrap_or_else(PoisonError::into_inner).decide(&target.location) {
            Some(GrantDecision::Deny) => return Some(Err(GateRefusal::Denied)),
            Some(GrantDecision::Allow) => return Some(Ok(())),
            None => {}
        }
        self.automatic_grant(defining, &target).then_some(Ok(()))
    }

    /// Applies a handler's `decision`, when it is one of `request`'s choices:
    /// a `Once` grant lasts for the run, a `Persisted` one is also written to
    /// the grant store. Returns whether a grant was applied.
    ///
    /// ## Errors
    ///
    /// The store write's failure; the grant then still applies for this run.
    pub(crate) fn apply(
        &self,
        request: &SlotGrantRequest,
        decision: &SlotGrantDecision,
    ) -> Result<bool, GrantStoreWriteError> {
        let Some(grant) = decision.grant().filter(|grant| request.choices.contains(grant)) else {
            return Ok(false);
        };
        let mut book = self.book.lock().unwrap_or_else(PoisonError::into_inner);
        let written = match grant.scope {
            GrantScope::Persisted => book.persist(grant.decision, grant.target.clone()),
            GrantScope::Once => Ok(()),
        };
        if grant.scope == GrantScope::Once || written.is_err() {
            book.add_once(grant.decision, grant.target);
        }
        written.map(|()| true)
    }

    /// One request per read the slots of `templates` name before evaluation
    /// (literal or pure targets) that no grant or automatic grant decides
    /// yet, in slot order, each target once. Reads nothing a slot names.
    pub(crate) fn undecided_requests(
        &self,
        templates: &[crate::markdown::schemas::PendingTemplate],
        context: &biscuit_file::FileResolutionContext,
    ) -> Vec<SlotGrantRequest> {
        use super::targets::{CollectedRead, collect_slot_targets};
        let mut requests: Vec<SlotGrantRequest> = Vec::new();
        for template in templates {
            for slot in collect_slot_targets(&template.slots, context) {
                for read in &slot.reads {
                    let CollectedRead::Known(target) = read else {
                        continue;
                    };
                    if self.decide_without_asking(&slot.defining, target).is_some() {
                        continue;
                    }
                    let asked = SlotGrantRequest::for_target(&canonical(target), slot.slot.origin.as_deref(), self.can_persist());
                    if !requests.iter().any(|known| known.kind == asked.kind && known.location == asked.location) {
                        requests.push(asked);
                    }
                }
            }
        }
        requests
    }

    /// Applies a handler's answers to `requests`, in order; a store write
    /// that fails leaves the grant for this run only.
    pub(crate) fn apply_all(&self, requests: &[SlotGrantRequest], decisions: &[SlotGrantDecision]) {
        for (asked, decision) in requests.iter().zip(decisions) {
            if let Err(error) = self.apply(asked, decision) {
                tracing::warn!(%error, "the schema read grant applies for this run only");
            }
        }
    }

    /// The fetch policy the slots' remote runtime uses: exactly the hosts
    /// the allow grants of URLs and sites name, and the command's
    /// `--allow host=HOST` values.
    pub(crate) fn fetch_policy(&self) -> FetchPolicy {
        let book = self.book.lock().unwrap_or_else(PoisonError::into_inner);
        let granted = book.persisted().iter().chain(book.once()).filter(|grant| grant.decision == GrantDecision::Allow);
        let mut hosts: Vec<String> = granted
            .filter_map(|grant| match &grant.target {
                GrantTarget::Url(url) | GrantTarget::UrlAnyQuery(url) => url.host_str().map(str::to_string),
                GrantTarget::Site { host, .. } => Some(host.clone()),
                GrantTarget::File(_) | GrantTarget::Dir(_) => None,
            })
            .chain(book.hosts().iter().cloned())
            .collect();
        hosts.sort();
        hosts.dedup();
        hosts.into_iter().fold(FetchPolicy::deny_all(), |policy, host| policy.allow(HostPattern::Exact(host)))
    }

    /// A remote runtime for slot reads under [`Self::fetch_policy`], with
    /// the command's concurrency, freshness, and transport cache when the
    /// command has a remote runtime (`document`), else the defaults with no
    /// cache. Build it after pre-flight's grants are applied; a host granted
    /// later in the run is fetched by a runtime built after that grant.
    pub(crate) fn remote_runtime(&self, document: Option<&RemoteFetchRuntime>) -> RemoteFetchRuntime {
        let policy = self.fetch_policy();
        match document {
            Some(document) => document.with_other_policy(policy),
            None => RemoteFetchRuntime::with_store(&Default::default(), None).with_other_policy(policy),
        }
    }

    /// Whether the automatic grant allows `target` (canonical), decided once
    /// per run; a post-read check also confirms the bytes read are the ones
    /// granted.
    fn automatic_grant(&self, defining: &Path, target: &ReadTarget) -> bool {
        let inside = self.grounds.root().is_some_and(|root| canonical_target(defining).starts_with(root));
        let key = (inside, target.kind, target.location.clone());
        let mut automatic = self.automatic.lock().unwrap_or_else(PoisonError::into_inner);
        let grant = match automatic.get(&key) {
            Some(decided) => *decided,
            None => {
                let decided = match self.grounds.judge(defining, target) {
                    AutoVerdict::Granted { expected } => Some(AutoGrant { expected, verified: false }),
                    AutoVerdict::Refused(refusal) => {
                        tracing::debug!(target = %target.location, ?refusal, "slot read not granted automatically");
                        None
                    }
                };
                // An absent target is judged as a probe; once it exists it is
                // a content read again.
                let absent_content = target.kind == ReadKind::Content
                    && matches!(decided, Some(AutoGrant { expected: None, .. }));
                if !absent_content {
                    automatic.insert(key.clone(), decided);
                }
                decided
            }
        };
        let Some(grant) = grant else {
            return false;
        };
        match (target.observed_hash, grant.expected) {
            (Some(observed), Some(expected)) if !grant.verified => {
                if observed == expected {
                    automatic.insert(key, Some(AutoGrant { verified: true, ..grant }));
                    true
                } else {
                    // The bytes read are not the committed ones the grant
                    // was for: the file is treated as dirty from now on.
                    automatic.insert(key, None);
                    false
                }
            }
            _ => true,
        }
    }

    /// Asks the handler about `target`, applying its answer; `true` when the
    /// read is now allowed.
    fn ask(&self, request: &SlotGrantRequest) -> Option<Result<(), GateRefusal>> {
        let handler = self.handler.as_ref()?;
        let decision = handler
            .approve_slot_reads(std::slice::from_ref(request))
            .ok()
            .and_then(|mut decisions| decisions.drain(..).next())
            .unwrap_or(SlotGrantDecision::Unanswered);
        if let Err(error) = self.apply(request, &decision) {
            tracing::warn!(%error, "the slot grant applies for this run only");
        }
        match self.book.lock().unwrap_or_else(PoisonError::into_inner).decide(&request.location) {
            Some(GrantDecision::Allow) => Some(Ok(())),
            Some(GrantDecision::Deny) => Some(Err(GateRefusal::Denied)),
            None => None,
        }
    }

    /// The full decision for one read of a slot written at `defining`
    /// (`schema` is its file, when it has one).
    fn check(&self, defining: &Path, schema: Option<&Path>, target: &ReadTarget) -> Result<(), GateRefusal> {
        let target = canonical(target);
        if let Some(decided) = self.decide_without_asking(defining, &target) {
            return decided;
        }
        let request = SlotGrantRequest::for_target(&target, schema, self.can_persist());
        if let Some(answered) = self.ask(&request) {
            return answered;
        }
        Err(GateRefusal::ApprovalRequired { needed: request.choices.into_iter().filter(is_allow).collect() })
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn is_allow(grant: &Grant) -> bool {
    grant.decision == GrantDecision::Allow
}

/// `target` with its local path canonicalized, as grants and the automatic
/// grant compare it.
pub(crate) fn canonical(target: &ReadTarget) -> ReadTarget {
    match &target.location {
        ReadLocation::Local(path) => {
            ReadTarget { location: ReadLocation::Local(canonical_target(path)), ..target.clone() }
        }
        _ => target.clone(),
    }
}

/// The gate for the slots defined at one place.
#[derive(Debug)]
#[cfg_attr(not(test), allow(dead_code))]
struct SlotReadGate {
    grants: Arc<SlotGrants>,
    defining: PathBuf,
    schema: Option<PathBuf>,
}

impl ReadGate for SlotReadGate {
    fn check(&self, target: &ReadTarget) -> Result<(), GateRefusal> {
        self.grants.check(&self.defining, self.schema.as_deref(), target)
    }
}

/// Every slot of a run asks the run's one [`SlotGrants`].
#[derive(Debug, Clone)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct RunGates(pub(crate) Arc<SlotGrants>);

impl SlotGates for RunGates {
    fn gate_for(&self, defining: &Path) -> Arc<dyn ReadGate> {
        let schema = defining.is_file().then(|| defining.to_path_buf());
        Arc::new(SlotReadGate { grants: Arc::clone(&self.0), defining: defining.to_path_buf(), schema })
    }
}
