//! Who may grow a request's runtime context, and the request-scoped epoch
//! every source in one composition reads from.
//!
//! A `ComposeContext` handed to [`ComposeOptions`](super::options::ComposeOptions)
//! is the request's context authority. When a transcluded source names a
//! `ctx.*` group the context has not captured, [`ContextAuthority`] decides
//! whether the context may grow (from evidence retained for this request) or
//! the source fails with `ContextNotCaptured`. The growth itself happens once
//! per request in [`RequestContextEpoch`], so every source that reads a group
//! reads the same captured projection.

use std::sync::{Arc, RwLock};

use super::capture::ContextRequirements;
use super::runtime::ComposeContext;

/// A caller-provided, same-request extension of a composition's context.
///
/// Implementations must populate only groups `context` is missing, from
/// evidence retained for the request that produced `context` — never from a
/// fresh point-of-use discovery that re-anchors or re-reads the environment.
/// [`ComposeContext::extend_with_evidence`] is the intended building block.
pub trait ContextExtension: Send + Sync + std::fmt::Debug {
    /// Extends `context` with the groups of `required` it is missing.
    ///
    /// Returns `true` when at least one group was populated.
    fn extend(&self, context: &mut ComposeContext, required: &ContextRequirements) -> bool;
}

/// Whether composition may grow the request context when a source names a
/// group it has not captured.
#[derive(Debug, Clone, Default)]
pub enum ContextAuthority {
    /// The context is frozen: a source reading an uncaptured group fails with
    /// `ContextNotCaptured`. The default for
    /// [`ComposeOptions::new_with_context`](super::options::ComposeOptions::new_with_context),
    /// so a pinned snapshot is never silently augmented from the host.
    #[default]
    CallerSupplied,
    /// Darkmatter grows the context by discovering the missing groups at the
    /// context's retained anchor ([`ComposeContext::extend_ambient`]). The
    /// default for [`ComposeOptions::new`](super::options::ComposeOptions::new).
    DarkmatterOwned,
    /// The caller grows the context from its own retained request evidence.
    CallerExtended(Arc<dyn ContextExtension>),
}

impl ContextAuthority {
    /// Whether this authority may grow a context at all.
    pub fn is_extendable(&self) -> bool {
        !matches!(self, Self::CallerSupplied)
    }

    /// Extends `context` with the groups of `required` it is missing, when
    /// this authority permits growth.
    pub(crate) fn extend(&self, context: &mut ComposeContext, required: &ContextRequirements) -> bool {
        match self {
            Self::CallerSupplied => false,
            Self::DarkmatterOwned => context.extend_ambient(required),
            Self::CallerExtended(extension) => extension.extend(context, required),
        }
    }

    /// Stable discriminant for option fingerprints. A caller extension is
    /// identified by kind only: its evidence is the context it extends.
    pub(crate) fn fingerprint_tag(&self) -> u8 {
        match self {
            Self::CallerSupplied => 0,
            Self::DarkmatterOwned => 1,
            Self::CallerExtended(_) => 2,
        }
    }
}

/// The one runtime context shared by every source composed in a request.
///
/// Seeded by the root document's context and grown monotonically: a group is
/// captured at most once, under the write lock, and never overwritten. A
/// source's own context is its parent's context plus the groups the source
/// names, adopted from this epoch — so the set of groups a source sees depends
/// only on its path from the root, not on the order concurrent siblings
/// resolved in, which keeps each source's cache identity stable.
#[derive(Debug, Default)]
pub(crate) struct RequestContextEpoch {
    shared: RwLock<Option<ComposeContext>>,
}

impl RequestContextEpoch {
    /// Installs `root` as the request context unless one is already seeded.
    pub(crate) fn seed(&self, root: &ComposeContext) {
        let mut shared = self.shared.write().unwrap_or_else(|poison| poison.into_inner());
        if shared.is_none() {
            *shared = Some(root.clone());
        }
    }

    /// The context a source composed under `parent` must use.
    ///
    /// Groups of `required` that `parent` lacks are adopted from the request
    /// context, which `authority` first grows when it permits. Groups neither
    /// can supply stay missing, and the source's checked lookup reports them.
    pub(crate) fn context_for_source(
        &self,
        parent: &ComposeContext,
        required: &ContextRequirements,
        authority: &ContextAuthority,
    ) -> ComposeContext {
        if parent.missing_requirements(required).iter().next().is_none() {
            return parent.clone();
        }
        let request = self.ensure(parent, required, authority);
        let mut context = parent.clone();
        context.adopt_groups(&request, required);
        context
    }

    /// The request context, grown to cover `required` when `authority`
    /// permits.
    pub(crate) fn ensure(
        &self,
        seed: &ComposeContext,
        required: &ContextRequirements,
        authority: &ContextAuthority,
    ) -> ComposeContext {
        {
            let shared = self.shared.read().unwrap_or_else(|poison| poison.into_inner());
            if let Some(request) = shared.as_ref()
                && (!authority.is_extendable()
                    || request.missing_requirements(required).iter().next().is_none())
            {
                return request.clone();
            }
        }
        let mut shared = self.shared.write().unwrap_or_else(|poison| poison.into_inner());
        let request = shared.get_or_insert_with(|| seed.clone());
        // Re-checked under the write lock: a concurrent sibling may have
        // captured the group between the read above and this point.
        authority.extend(request, required);
        request.clone()
    }
}

/// A shared handle to one request's context epoch.
pub(crate) type SharedRequestContextEpoch = Arc<RequestContextEpoch>;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use rayon::prelude::*;

    use super::*;
    use crate::markdown::compose::cache::hashing::context_hash;
    use crate::markdown::compose::expression::ExpressionError;
    use crate::markdown::compose::shell_expansion::types::PipelineRuntime;
    use crate::markdown::compose::{CacheAccessMode, ComposeOptions, ContextCaptureEvidence, ContextGroup};

    /// Supplied-evidence extension projecting `request-<n>` as the repository
    /// root of its `n`th capture, so a repeated capture is observable.
    #[derive(Debug, Default)]
    struct Counting {
        captures: AtomicUsize,
    }

    impl ContextExtension for Counting {
        fn extend(&self, context: &mut ComposeContext, required: &ContextRequirements) -> bool {
            if context.missing_requirements(required).iter().next().is_none() {
                return false;
            }
            let capture = self.captures.fetch_add(1, Ordering::SeqCst) + 1;
            let evidence = ContextCaptureEvidence::new(HashMap::new())
                .with_git(None)
                .with_repository(Some(PathBuf::from(format!("request-{capture}"))), None)
                .with_os(None);
            context.extend_with_evidence(required, &evidence)
        }
    }

    fn root() -> ComposeContext {
        ComposeContext::capture_with_evidence(
            std::path::Path::new("does-not-exist"),
            &ContextRequirements::for_content(""),
            &ContextCaptureEvidence::new(HashMap::new()),
        )
    }

    fn seeded(root: &ComposeContext) -> RequestContextEpoch {
        let epoch = RequestContextEpoch::default();
        epoch.seed(root);
        epoch
    }

    #[test]
    fn concurrent_sources_capture_a_group_once_and_read_one_projection() {
        let root = root();
        let epoch = seeded(&root);
        let counting = Arc::new(Counting::default());
        let authority = ContextAuthority::CallerExtended(counting.clone());
        let required = ContextRequirements::for_content("{{ ctx.repo_root }}");

        let observed: Vec<_> = (0..32)
            .into_par_iter()
            .map(|_| {
                let context = epoch.context_for_source(&root, &required, &authority);
                context.get("repo_root").cloned().expect("repo_root projected")
            })
            .collect();

        assert_eq!(counting.captures.load(Ordering::SeqCst), 1, "one capture per request");
        assert!(observed.iter().all(|value| value == &observed[0]), "{observed:?}");
        assert!(
            !root.capture_requirements().contains(ContextGroup::Repo),
            "growth is copy-on-write: the parent snapshot is unchanged"
        );
    }

    #[test]
    fn a_source_sees_its_parent_groups_plus_its_own_whatever_the_request_holds() {
        let root = root();
        // A fresh extension per request, so both requests project `request-1`.
        let authority = || ContextAuthority::CallerExtended(Arc::new(Counting::default()));
        let repo = ContextRequirements::for_content("{{ ctx.repo_root }}");
        let os = ContextRequirements::for_content("{{ ctx.os }}");

        let quiet = seeded(&root);
        let alone = quiet.context_for_source(&root, &repo, &authority());

        let busy = seeded(&root);
        let sibling = busy.context_for_source(&root, &os, &authority());
        assert!(sibling.capture_requirements().contains(ContextGroup::Os));
        let after_sibling = busy.context_for_source(&root, &repo, &authority());

        assert!(!after_sibling.capture_requirements().contains(ContextGroup::Os));
        assert_eq!(
            context_hash(&after_sibling),
            context_hash(&alone),
            "a sibling's growth must not change another source's identity"
        );
        assert!(
            root.is_same_snapshot(&quiet.context_for_source(
                &root,
                &ContextRequirements::for_content(""),
                &authority()
            )),
            "a source naming nothing new reuses its parent snapshot"
        );
    }

    #[test]
    fn a_frozen_authority_never_grows_the_request_context() {
        let root = root();
        let epoch = seeded(&root);
        let required = ContextRequirements::for_content("{{ ctx.repo_root }}");

        let context = epoch.context_for_source(&root, &required, &ContextAuthority::CallerSupplied);
        let request = epoch.ensure(&root, &required, &ContextAuthority::CallerSupplied);

        assert!(!context.capture_requirements().contains(ContextGroup::Repo));
        assert!(!request.capture_requirements().contains(ContextGroup::Repo));
        assert!(request.values().get("repo_root").is_none(), "no fallback capture ran");
    }

    /// Mutation guard for the root handoff: the same pipeline
    /// `run_compose_pipeline` drives, minus the root context extension, fails
    /// with the named missing capture rather than rendering an empty value.
    #[test]
    fn a_pipeline_without_the_root_extension_fails_with_the_named_group() {
        let options = ComposeOptions::new().with_context_authority(ContextAuthority::DarkmatterOwned);
        let mut runtime = PipelineRuntime::new(16, CacheAccessMode::Off, None);
        runtime.context_epoch.seed(options.context());
        let mut markdown: crate::markdown::Markdown = "os={{ ctx.os }}\n".into();

        let error = markdown
            .run_compose_pipeline_internal(options, &mut runtime)
            .expect_err("no root extension, no OS group");

        assert!(
            matches!(
                error.missing_runtime_context(),
                Some(ExpressionError::ContextNotCaptured { group: ContextGroup::Os, .. })
            ),
            "{error:?}"
        );
    }
}
