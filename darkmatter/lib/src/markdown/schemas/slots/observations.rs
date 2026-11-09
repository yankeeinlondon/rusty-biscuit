//! The structured observations a session's slots read, captured when a slot
//! that needs one first runs.
//!
//! `package()`, `package_area()`, `ipv4()`, `ipv6()`, and `has_agentic_cli()`
//! read the `Repo`, `Network`, and `Agent` observations behind their capture
//! groups. A document's capture covers only the groups its own text names,
//! so a slot written in a referenced schema, an imported type, a trigger
//! payload, or a `load_data` contract would otherwise find its group absent.
//! Instead each slot's own calls decide what is captured, once per session,
//! from the command's request: a pure or file-only schema captures nothing,
//! and no document text decides whether a schema function works.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use biscuit_file::FileResolutionContext;

use crate::markdown::compose::ComposeContext;
use crate::markdown::compose::context::RepositoryObservation;
use crate::markdown::compose::context::authority::ContextAuthority;
use crate::markdown::compose::context::capture::{
    CapturedObservations, ContextGroup, ContextRequirements, capture_runtime_context_for_groups,
};

/// The groups that own a structured observation.
const OBSERVED_GROUPS: [ContextGroup; 3] = [ContextGroup::Repo, ContextGroup::Network, ContextGroup::Agent];

/// Where a session captures a group none of its observations holds yet.
#[derive(Debug, Clone)]
enum Source {
    /// Nowhere: what was supplied is all there is.
    Supplied,
    /// A composition request: its repository observation for `Repo`, and its
    /// context, grown as its authority permits, for the other groups. A
    /// frozen (caller-supplied) context grows nothing, so the slot fails as
    /// any read of an uncaptured group does.
    Request { context: ComposeContext, authority: ContextAuthority, repository: Option<Arc<RepositoryObservation>> },
    /// A command that validates without composing: captured with the
    /// context's own environment, the repository at the context's
    /// repository root (none when the context found none, never a discovery
    /// elsewhere).
    Context { base_dir: PathBuf, environment: HashMap<String, String>, repository_root: Option<PathBuf> },
}

/// A session's observations; see the module documentation.
#[derive(Debug)]
pub(crate) struct SessionObservations {
    observed: Mutex<CapturedObservations>,
    source: Source,
}

impl SessionObservations {
    /// Exactly `observations`; a missing group stays missing.
    pub(crate) fn supplied(observations: CapturedObservations) -> Self {
        Self { observed: Mutex::new(observations), source: Source::Supplied }
    }

    /// A composition request's: what its context captured, growing as its
    /// `authority` permits.
    pub(crate) fn for_request(
        context: &ComposeContext,
        authority: &ContextAuthority,
        repository: Option<Arc<RepositoryObservation>>,
    ) -> Self {
        Self {
            observed: Mutex::new(context.observations().clone()),
            source: Source::Request { context: context.clone(), authority: authority.clone(), repository },
        }
    }

    /// A validating command's, captured from `context` on demand.
    pub(crate) fn for_context(context: &FileResolutionContext) -> Self {
        Self {
            observed: Mutex::new(CapturedObservations::default()),
            source: Source::Context {
                base_dir: context.base_dir().to_path_buf(),
                environment: context.env().clone(),
                repository_root: context.repository_root().map(std::path::Path::to_path_buf),
            },
        }
    }

    /// The observations, with every group of `groups` that owns one
    /// captured first if it was not yet. Each group is captured at most once
    /// per session, so every slot (and every re-check of a cached result)
    /// in the command reads the same answer.
    pub(crate) fn covering(&self, groups: impl IntoIterator<Item = ContextGroup>) -> CapturedObservations {
        let mut observed = self.observed.lock().unwrap_or_else(PoisonError::into_inner);
        let mut missing: Vec<ContextGroup> = Vec::new();
        for group in groups {
            if OBSERVED_GROUPS.contains(&group) && !observed.has(group) && !missing.contains(&group) {
                missing.push(group);
            }
        }
        if !missing.is_empty() {
            let captured = self.source.capture(&missing);
            observed.fill_from(&captured, &ContextRequirements::from_groups(missing));
        }
        observed.clone()
    }
}

impl Source {
    /// Observations for `missing`; a group this source cannot capture is
    /// left absent.
    fn capture(&self, missing: &[ContextGroup]) -> CapturedObservations {
        match self {
            Self::Supplied => CapturedObservations::default(),
            Self::Request { context, authority, repository } => {
                let mut captured = CapturedObservations::default();
                let mut rest: Vec<ContextGroup> = missing.to_vec();
                if let Some(repository) = repository
                    && missing.contains(&ContextGroup::Repo)
                {
                    captured = captured.with_repository(repository);
                    rest.retain(|group| *group != ContextGroup::Repo);
                }
                if !rest.is_empty() {
                    let required = ContextRequirements::from_groups(rest);
                    let mut grown = context.clone();
                    authority.extend(&mut grown, &required);
                    captured.fill_from(grown.observations(), &required);
                }
                captured
            }
            Self::Context { base_dir, environment, repository_root } => {
                let mut groups: Vec<ContextGroup> = missing.to_vec();
                let mut captured = CapturedObservations::default();
                if repository_root.is_none() && groups.contains(&ContextGroup::Repo) {
                    captured = captured.with_repository(&RepositoryObservation::default());
                    groups.retain(|group| *group != ContextGroup::Repo);
                }
                if !groups.is_empty() {
                    let anchor = repository_root.as_deref().unwrap_or(base_dir);
                    let (_, _, _, _, observations) = capture_runtime_context_for_groups(anchor, &groups, environment);
                    captured.fill_from(&observations, &ContextRequirements::from_groups(groups));
                }
                captured
            }
        }
    }
}
