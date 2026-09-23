//! Nested composition behind `as_markdown(content)`.
//!
//! A nested child is not a new request: it reuses the calling pipeline's
//! `ComposeOptions` (consent, remote policy, cache policy, captured context),
//! the request's context epoch, the shell runtime, and the transclusion
//! ancestry. Its node counts against the same `max_transclusion_depth` budget
//! as `::file`, so mixed `as_markdown`/transclusion recursion stops with the
//! existing typed error (decisions D8). See
//! `features/2026-09-09-more-context/spec.md` (R11, R20) and plan Phase 8.
//!
//! Expression handlers are synchronous, and so is the compose pipeline
//! (transclusion concurrency is rayon, not an async runtime), so the child runs
//! on the calling thread exactly as a transcluded child does inside the
//! transclusion engine.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::context::options::SourceDerivation;
use super::expression::ExpressionError;
use super::shell_expansion::types::PipelineRuntime;
use super::{ComposeOptions, ComposeReport, ComposeSource, ContextRequirements, transclusion};
use crate::markdown::Markdown;
use crate::markdown::types::{MarkdownError, MarkdownResult};

const FUNCTION: &str = "as_markdown";

/// What `as_markdown` may do on the surface that evaluates it.
#[derive(Clone, Default)]
pub(crate) enum NestedComposeSlot {
    /// No compose request backs this evaluation (passive validation, host
    /// adapters, bare `ResolutionContext`s); the call is a compose error.
    #[default]
    Unavailable,
    /// Shell-command discovery: record the evaluated content for the static
    /// preflight walk and return `""` without composing anything.
    Discover(Arc<Mutex<Vec<String>>>),
    /// A running compose pipeline: compose the content as a nested child.
    Active(NestedCompose),
}

impl std::fmt::Debug for NestedComposeSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unavailable => "Unavailable",
            Self::Discover(_) => "Discover",
            Self::Active(_) => "Active",
        })
    }
}

impl NestedComposeSlot {
    /// A fresh discovery sink for one document of the preflight walk.
    pub(crate) fn discover() -> Self {
        Self::Discover(Arc::default())
    }

    /// Drains the content recorded by [`Self::Discover`].
    pub(crate) fn take_discovered(&self) -> Vec<String> {
        match self {
            Self::Discover(sink) => std::mem::take(&mut *lock(sink)),
            _ => Vec::new(),
        }
    }

    pub(crate) fn is_discovery(&self) -> bool {
        matches!(self, Self::Discover(_))
    }

    /// Evaluates `as_markdown(content)` on this surface.
    pub(crate) fn evaluate(&self, content: &str) -> Result<String, ExpressionError> {
        match self {
            Self::Unavailable => Err(violation(
                "is only available while a document is being composed".to_string(),
            )),
            Self::Discover(sink) => {
                lock(sink).push(content.to_string());
                Ok(String::new())
            }
            Self::Active(nested) => nested.compose(content),
        }
    }
}

/// Handle to the calling document's pipeline, installed once per document.
#[derive(Clone)]
pub(crate) struct NestedCompose {
    scope: Arc<Scope>,
}

struct Scope {
    /// The calling pipeline's options, without this handle (no `Arc` cycle).
    options: ComposeOptions,
    /// The calling pipeline's runtime, forked while its node is on the stack.
    runtime: Mutex<PipelineRuntime>,
    /// The root document as it entered the pipeline, when this document owns
    /// the pre-approved gate. Frontmatter interpolation runs before that gate,
    /// so a nested call there validates the whole request first.
    root_gate: Option<Markdown>,
    outcome: Mutex<Outcome>,
}

#[derive(Default)]
struct Outcome {
    reports: Vec<ComposeReport>,
    deepest_seen: usize,
    context_groups: Option<ContextRequirements>,
    /// The first cycle, depth-limit, or pre-approved gate failure below this
    /// document. It replaces the expression error the call site reported, so
    /// the request fails with the same typed error a `::file` recursion or the
    /// root gate produces.
    restored: Option<MarkdownError>,
}

/// Unique per process so an `as_markdown` node never forms a cycle by itself
/// and repeated self-composition ends at the depth limit.
static NEXT_NODE: AtomicU64 = AtomicU64::new(0);

impl NestedCompose {
    /// Installs a handle for the document whose node `runtime` has just
    /// entered. Discovery options keep their discovery slot.
    pub(crate) fn install(
        options: &mut ComposeOptions,
        runtime: &PipelineRuntime,
        document: &Markdown,
    ) -> Option<Self> {
        if options.nested_compose.is_discovery() {
            return None;
        }
        let mut snapshot = options.clone();
        snapshot.nested_compose = NestedComposeSlot::Unavailable;
        let root_gate = super::pipeline::preflight_gate_applies(options, runtime).then(|| document.clone());
        let nested = Self {
            scope: Arc::new(Scope {
                options: snapshot,
                runtime: Mutex::new(runtime.clone_for_child()),
                root_gate,
                outcome: Mutex::default(),
            }),
        };
        options.nested_compose = NestedComposeSlot::Active(nested.clone());
        Some(nested)
    }

    /// Folds every nested child's report, depth, and context groups into the
    /// calling document once its stages finish.
    pub(crate) fn finish(
        self,
        result: MarkdownResult<ComposeReport>,
        runtime: &mut PipelineRuntime,
    ) -> MarkdownResult<ComposeReport> {
        let outcome = std::mem::take(&mut *lock(&self.scope.outcome));
        runtime.transclusion.deepest_seen = runtime.transclusion.deepest_seen.max(outcome.deepest_seen);
        if let Some(groups) = &outcome.context_groups {
            runtime.record_context_groups(groups);
        }
        if let Some(error) = outcome.restored {
            return Err(error);
        }
        let mut report = result?;
        for child in outcome.reports {
            report.merge(child);
        }
        Ok(report)
    }

    fn compose(&self, content: &str) -> Result<String, ExpressionError> {
        let source_context = biscuit_terminal::errors::SourceContext::new(
            PathBuf::from("as_markdown()"),
            PathBuf::from("as_markdown()"),
            content,
        );
        let (frontmatter, body) = crate::markdown::frontmatter::parse_frontmatter(content, source_context)
            .map_err(|error| violation(format!("content has invalid frontmatter: {error}")))?;
        let authored_keys: Option<HashSet<String>> = (frontmatter.raw_source().is_some()
            || !frontmatter.is_empty())
        .then(|| frontmatter.as_map().keys().cloned().collect());
        let mut child = Markdown::with_frontmatter(frontmatter, body);

        let mut runtime = lock(&self.scope.runtime).clone_for_child();
        if let Some(root) = &self.scope.root_gate
            && !runtime.preflight_validated().load(Ordering::Acquire)
        {
            if let Err(error) = super::preflight::validate_pre_approved(root, &self.scope.options) {
                let message = format!("the pre-approved command gate failed: {error}");
                lock(&self.scope.outcome).restored.get_or_insert(error);
                return Err(violation(message));
            }
            runtime.preflight_validated().store(true, Ordering::Release);
        }
        let mut options = self.scope.options.clone();
        // Relative references resolve against the root document (R11), not
        // the transcluded document that happens to call the function.
        if let Some((source, derivation)) = runtime.root_source.clone() {
            options.source = source;
            options.source_derivation = derivation;
        }
        // `--set` overrides target the root document's frontmatter, and the
        // preflight graph's edges belong to the caller, not this content.
        options.set_overrides = None;
        options.preflight_graph = None;
        let context = runtime.context_epoch.context_for_source(
            options.context(),
            &ContextRequirements::for_document(&child),
            options.context_authority(),
        );
        runtime.record_context_groups(context.capture_requirements());
        let options = options.with_request_context(context);

        let node = format!("as_markdown#{}", NEXT_NODE.fetch_add(1, Ordering::Relaxed));
        let result = child.run_compose_pipeline_node(options, &mut runtime, Some((node, PathBuf::from("as_markdown()"))));

        let mut outcome = lock(&self.scope.outcome);
        outcome.deepest_seen = outcome.deepest_seen.max(runtime.transclusion.deepest_seen);
        outcome.context_groups = Some(match outcome.context_groups.take() {
            Some(groups) => groups.union(runtime.context_groups()),
            None => runtime.context_groups().clone(),
        });
        let mut report = match result {
            Ok(report) => report,
            Err(error) => {
                let message = format!("nested composition failed: {error}");
                if is_structural(&error) && outcome.restored.is_none() {
                    outcome.restored = Some(error);
                }
                return Err(violation(message));
            }
        };
        for warning in &mut report.warnings {
            warning.stage = format!("as_markdown > {}", warning.stage);
        }
        outcome.reports.push(report);
        drop(outcome);

        // Composition merges request state into frontmatter; only the keys the
        // content authored belong to its serialization.
        match authored_keys {
            Some(keys) => child.frontmatter_mut().as_map_mut().retain(|key, _| keys.contains(key)),
            None => child.frontmatter_mut().as_map_mut().clear(),
        }
        Ok(child.as_string())
    }
}

fn is_structural(error: &MarkdownError) -> bool {
    matches!(
        error,
        MarkdownError::Transclusion(inner)
            if matches!(
                inner.as_ref(),
                transclusion::TransclusionError::CycleDetected { .. }
                    | transclusion::TransclusionError::MaxDepthExceeded { .. }
            )
    )
}

fn violation(message: String) -> ExpressionError {
    ExpressionError::ContractViolation {
        function: FUNCTION.to_string(),
        message,
    }
}

/// The guarded data is plain accumulation, so a panic elsewhere cannot leave
/// it logically inconsistent; recover rather than propagate the poison.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Root source recorded on the request runtime for nested children.
pub(crate) type RootSource = (ComposeSource, SourceDerivation);
