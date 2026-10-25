//! Process lifetimes: merging observations and enriching identity safely.

use super::ancestry;
use super::backend::{LinkError, ProcessDetails, UsageBackend};
use super::budget::Budget;
use super::report::{
    Ancestor, AncestryEnd, Evidence, LimitationExample, LimitationKind, Limitations, ProcessRecord,
};
use crate::performance::{self, counters};
use std::collections::HashMap;
use std::ffi::OsString;

/// One process acquisition and the in-scope evidence it produced.
#[derive(Debug, Clone)]
pub(crate) struct Observation {
    pub(crate) pid: u32,
    pub(crate) creation_token: Option<u64>,
    pub(crate) retained_handle: bool,
    pub(crate) details: ProcessDetails,
    pub(crate) evidence: Vec<Evidence>,
}

impl Observation {
    fn lifetime_known(&self) -> bool {
        self.creation_token.is_some() || self.retained_handle
    }

    fn needs_enrichment(&self) -> bool {
        self.details.name.is_none() || self.details.executable.is_none() || self.details.user.is_none()
    }
}

fn fill_missing(target: &mut ProcessDetails, source: ProcessDetails) {
    target.name = target.name.take().or(source.name);
    target.executable = target.executable.take().or(source.executable);
    target.user = target.user.take().or(source.user);
    target.start_time = target.start_time.take().or(source.start_time);
}

/// Merges acquisitions that share a PID *and* a known creation token.
///
/// Different tokens are different lifetimes (a reused PID), and an
/// acquisition without a token can never be proven to match another, so both
/// stay separate records.
pub(crate) fn merge(observations: Vec<Observation>) -> Vec<Observation> {
    let mut merged: Vec<Observation> = Vec::new();
    let mut by_lifetime: HashMap<(u32, u64), usize> = HashMap::new();
    for observation in observations {
        let Some(token) = observation.creation_token else {
            merged.push(observation);
            continue;
        };
        match by_lifetime.get(&(observation.pid, token)) {
            Some(&index) => {
                let existing = &mut merged[index];
                existing.retained_handle |= observation.retained_handle;
                existing.evidence.extend(observation.evidence);
                fill_missing(&mut existing.details, observation.details);
            }
            None => {
                by_lifetime.insert((observation.pid, token), merged.len());
                merged.push(observation);
            }
        }
    }
    merged
}

/// Fills missing identity fields, only where the same lifetime can be proven.
///
/// Evidence is never removed: a process that exits, or whose PID is reused
/// between inspection and enrichment, keeps its evidence and gains a
/// limitation instead of a successor's name.
pub(crate) fn enrich<B: UsageBackend>(
    observations: &mut [Observation],
    backend: &mut B,
    budget: &Budget,
    limitations: &mut Limitations,
) {
    for observation in observations.iter_mut().filter(|o| o.needs_enrichment()) {
        let pid = observation.pid;
        if !observation.lifetime_known() {
            limitations.record(
                LimitationKind::IdentityUnavailable,
                "identity is not read for a process whose lifetime cannot be verified",
                1,
                Some(LimitationExample::pid(pid)),
            );
            continue;
        }
        if budget.expired() {
            limitations.record(
                LimitationKind::IdentityUnavailable,
                "the budget expired before process identity was read",
                1,
                Some(LimitationExample::pid(pid)),
            );
            continue;
        }
        performance::increment_counter(counters::QUERY_IDENTITY_ENRICHMENTS, 1);
        let details = match backend.details(pid) {
            Ok(details) => details,
            Err(error) => {
                limitations.record(
                    LimitationKind::IdentityUnavailable,
                    "process identity could not be read",
                    1,
                    Some(LimitationExample::pid(pid).with_detail(error)),
                );
                continue;
            }
        };
        // Read after `details`: an unchanged token proves the PID named the
        // inspected lifetime for the whole read.
        if observation.creation_token.is_some()
            && backend.creation_token(pid) != observation.creation_token
        {
            limitations.record(
                LimitationKind::ProcessDisappeared,
                "the process exited or its PID was reused before its identity was read",
                1,
                Some(LimitationExample::pid(pid)),
            );
            continue;
        }
        fill_missing(&mut observation.details, details);
    }
}

/// A process's parent, app name, and verified ancestors.
#[derive(Debug, Clone, Default)]
pub(crate) struct Context {
    parent_pid: Option<u32>,
    app_name: Option<OsString>,
    ancestors: Vec<Ancestor>,
    end: AncestryEnd,
}

impl Context {
    fn ended(end: AncestryEnd) -> Self {
        Self {
            end,
            ..Self::default()
        }
    }
}

/// Reads each observation's context, aligned with `observations`, under the
/// same proof of lifetime as [`enrich`].
pub(crate) fn contextualize<B: UsageBackend>(
    observations: &[Observation],
    backend: &mut B,
    budget: &Budget,
    limitations: &mut Limitations,
) -> Vec<Context> {
    let mut contexts = Vec::with_capacity(observations.len());
    for observation in observations {
        let pid = observation.pid;
        if !observation.lifetime_known() {
            limitations.record(
                LimitationKind::ContextUnavailable,
                "context is not read for a process whose lifetime cannot be verified",
                1,
                Some(LimitationExample::pid(pid)),
            );
            contexts.push(Context::ended(AncestryEnd::NotRead));
            continue;
        }
        if budget.expired() {
            limitations.record(
                LimitationKind::BudgetExhausted,
                "the budget expired before process context was read",
                1,
                Some(LimitationExample::pid(pid)),
            );
            contexts.push(Context::ended(AncestryEnd::NotRead));
            continue;
        }
        let mut reader = |pid| backend.link(pid);
        let own = match ancestry::read(&mut reader, pid) {
            Ok(own) => own,
            Err(LinkError::Unsupported) => {
                contexts.push(Context::ended(AncestryEnd::Unsupported));
                continue;
            }
            Err(LinkError::Gone) => {
                limitations.record(
                    LimitationKind::ProcessDisappeared,
                    "the process exited or its PID was reused before its context was read",
                    1,
                    Some(LimitationExample::pid(pid)),
                );
                contexts.push(Context::ended(AncestryEnd::NotRead));
                continue;
            }
            Err(LinkError::Unreadable(detail)) => {
                limitations.record(
                    LimitationKind::ContextUnavailable,
                    "process context could not be read",
                    1,
                    Some(LimitationExample::pid(pid).with_detail(detail)),
                );
                contexts.push(Context::ended(AncestryEnd::Unreadable));
                continue;
            }
        };
        // Read after the link: an unchanged token proves the PID named the
        // inspected lifetime for the whole read.
        if observation.creation_token.is_some()
            && backend.creation_token(pid) != observation.creation_token
        {
            limitations.record(
                LimitationKind::ProcessDisappeared,
                "the process exited or its PID was reused before its context was read",
                1,
                Some(LimitationExample::pid(pid)),
            );
            contexts.push(Context::ended(AncestryEnd::NotRead));
            continue;
        }
        let mut reader = |pid| backend.link(pid);
        let (ancestors, end) = ancestry::chain(pid, &own, budget, limitations, &mut reader);
        contexts.push(Context {
            parent_pid: Some(own.parent_pid),
            app_name: own.app_name,
            ancestors,
            end,
        });
    }
    contexts
}

/// Converts finished observations and their [`contextualize`]d contexts to
/// sorted report records.
pub(crate) fn into_records(observations: Vec<Observation>, contexts: Vec<Context>) -> Vec<ProcessRecord> {
    let mut contexts = contexts.into_iter();
    let mut records: Vec<ProcessRecord> = observations
        .into_iter()
        .map(|observation| {
            let context = contexts.next().unwrap_or_default();
            let identity_uncertain = !observation.lifetime_known();
            let mut evidence = observation.evidence;
            evidence.sort_by(|a, b| evidence_key(a).cmp(&evidence_key(b)));
            evidence.dedup();
            ProcessRecord {
                pid: observation.pid,
                creation_token: observation.creation_token,
                start_time: observation.details.start_time,
                identity_uncertain,
                name: observation.details.name.map(Into::into),
                executable: observation.details.executable.map(Into::into),
                user: observation.details.user,
                parent_pid: context.parent_pid,
                app_name: context.app_name.map(Into::into),
                ancestors: context.ancestors,
                ancestry_end: context.end,
                evidence,
            }
        })
        .collect();
    records.sort_by_key(|r| (r.pid, r.creation_token));
    records
}

fn evidence_key(
    evidence: &Evidence,
) -> (
    Option<&std::path::Path>,
    super::report::EvidenceKind,
    super::report::Mechanism,
    Option<u64>,
    Option<i64>,
) {
    (
        evidence.matched_paths.first().map(|p| p.as_path()),
        evidence.kind,
        evidence.mechanism,
        evidence.descriptor,
        evidence.watch.as_ref().and_then(|w| w.watch_id),
    )
}
