//! Lazy-capturing `ctx.*` resolver for expression evaluation.
//!
//! Provides [`CtxLookup`], a standalone [`EvaluationLookup`] implementation
//! that resolves `ctx.*` paths via demand-driven runtime context capture.
//! This type was extracted from [`ShortcutLookup`](super::conditions::ShortcutLookup)
//! so that `ctx.*` resolution can be composed on top of any other lookup.

use super::{EvaluationLookup, ExpressionError};
use crate::markdown::compose::context::checked::CtxLookupOutcome;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Lazy-capturing `ctx.*` resolver, suitable for composing on top of any
/// `EvaluationLookup` that handles non-`ctx` paths.
#[derive(Debug)]
pub struct CtxLookup<'a> {
    work_dir: &'a Path,
    environment: &'a HashMap<String, String>,
    cache: RefCell<HashMap<String, Value>>,
    captured: RefCell<HashSet<super::super::context::capture::ContextGroup>>,
}

impl<'a> CtxLookup<'a> {
    /// Creates a new `CtxLookup` for the given working directory and request
    /// environment (the source of the `Agent` group's `agent`/`model`).
    pub fn new(work_dir: &'a Path, environment: &'a HashMap<String, String>) -> Self {
        Self {
            work_dir,
            environment,
            cache: RefCell::new(HashMap::new()),
            captured: RefCell::new(HashSet::new()),
        }
    }

    /// Returns `Some(value)` for `ctx.<key>` paths whose context group can
    /// be captured; `None` otherwise. Non-`ctx` paths always return `None`.
    ///
    /// The `path` argument is expected to include the `ctx.` prefix (e.g.
    /// `"ctx.today"`). The bare `"ctx"` token is also accepted and returns
    /// `None` because there is no single value associated with the `ctx`
    /// namespace itself.
    ///
    /// A capture that fails to project a cataloged key also returns `None`
    /// here; [`resolve_ctx_checked`](Self::resolve_ctx_checked) reports it.
    pub fn resolve_ctx(&self, path: &str) -> Option<Value> {
        self.resolve_ctx_checked(path).ok().flatten()
    }

    /// Checked twin of [`resolve_ctx`](Self::resolve_ctx).
    ///
    /// The group is captured on demand, so a known key is never
    /// "not captured" here.
    ///
    /// ## Errors
    ///
    /// Returns [`ExpressionError::ContextProjectionInvariant`] when the
    /// just-captured group did not project `path`'s cataloged key.
    pub fn resolve_ctx_checked(&self, path: &str) -> Result<Option<Value>, ExpressionError> {
        let Some(ctx_key) = path.strip_prefix("ctx.") else {
            return Ok(None);
        };

        let outcome = CtxLookupOutcome::classify(
            ctx_key,
            |group| {
                if !self.captured.borrow().contains(&group) {
                    self.capture_group(group);
                }
                true
            },
            |key| self.cache.borrow().get(key).cloned(),
        );
        // Every group this lookup reaches is captured, so `NotCaptured` is unreachable.
        outcome.into_checked(|| None)
    }

    /// Captures a single context group and merges its values into the cache.
    fn capture_group(&self, group: super::super::context::capture::ContextGroup) {
        let (values, _diagnostics, _timings, _environment, _observations) =
            super::super::context::capture::capture_runtime_context_for_groups(
                self.work_dir,
                &[group],
                self.environment,
            );

        let mut cache = self.cache.borrow_mut();
        let mut captured = self.captured.borrow_mut();
        for (key, value) in values {
            cache.insert(key, value);
        }
        captured.insert(group);
    }

    /// Marks `group` captured without projecting any of its keys: the
    /// malformed capture behind `ContextProjectionInvariant`.
    #[cfg(test)]
    pub(crate) fn mark_captured_without_projection(
        &self,
        group: super::super::context::capture::ContextGroup,
    ) {
        self.captured.borrow_mut().insert(group);
    }

    /// Returns the set of context groups that have been captured so far.
    #[cfg(test)]
    pub(crate) fn captured_groups(&self) -> Vec<super::super::context::capture::ContextGroup> {
        self.captured.borrow().iter().cloned().collect()
    }
}

impl<'a> EvaluationLookup for CtxLookup<'a> {
    fn get(&self, path: &str) -> Option<Value> {
        if path == "ctx" || path.starts_with("ctx.") {
            return self.resolve_ctx(path);
        }
        None
    }

    fn get_checked(&self, path: &str) -> Result<Option<Value>, ExpressionError> {
        if path == "ctx" || path.starts_with("ctx.") {
            return self.resolve_ctx_checked(path);
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::context::capture::ContextGroup;

    #[test]
    fn ctx_lookup_returns_none_for_non_ctx_paths() {
        let environment = HashMap::new();
        let lookup = CtxLookup::new(Path::new("."), &environment);
        assert!(lookup.get("foo").is_none());
        assert!(lookup.get("env.HOME").is_none());
        assert!(lookup.get("git.branch").is_none());
    }

    #[test]
    fn ctx_lookup_resolves_today() {
        let environment = HashMap::new();
        let lookup = CtxLookup::new(Path::new("."), &environment);
        let value = lookup.get("ctx.today");
        assert!(value.is_some(), "ctx.today should resolve to a value");
        if let Some(Value::String(s)) = value {
            assert!(!s.is_empty(), "ctx.today should be non-empty");
        } else {
            panic!("ctx.today should return a Value::String, got {:?}", value);
        }

        let captured = lookup.captured_groups();
        assert_eq!(
            captured.len(),
            1,
            "Exactly one group should be captured, got {:?}",
            captured
        );
        assert!(captured.contains(&ContextGroup::DateTime));
    }

    #[test]
    fn ctx_lookup_caches_repeated_lookups() {
        let environment = HashMap::new();
        let lookup = CtxLookup::new(Path::new("."), &environment);
        // First call triggers capture
        let _ = lookup.get("ctx.today");
        let captured_after_first = lookup.captured_groups().len();

        // Second call should not re-capture
        let _ = lookup.get("ctx.today");
        let captured_after_second = lookup.captured_groups().len();

        assert_eq!(
            captured_after_first, captured_after_second,
            "Repeated ctx.today lookups should not re-capture"
        );
    }

    #[test]
    fn ctx_lookup_unknown_ctx_key_returns_none() {
        let environment = HashMap::new();
        let lookup = CtxLookup::new(Path::new("."), &environment);
        assert!(
            lookup.get("ctx.zzz").is_none(),
            "Unknown ctx key should return None"
        );
    }

    #[test]
    fn ctx_lookup_bare_ctx_returns_none() {
        let environment = HashMap::new();
        let lookup = CtxLookup::new(Path::new("."), &environment);
        assert!(
            lookup.get("ctx").is_none(),
            "Bare 'ctx' token should return None"
        );
    }

    #[test]
    fn a_capture_missing_a_cataloged_key_is_an_internal_invariant() {
        let environment = HashMap::new();
        let lookup = CtxLookup::new(Path::new("."), &environment);
        lookup.mark_captured_without_projection(ContextGroup::Os);

        assert!(matches!(
            lookup.get_checked("ctx.os"),
            Err(ExpressionError::ContextProjectionInvariant { ref key, group: ContextGroup::Os })
                if key == "os"
        ));
        // Evaluation surfaces the typed cause instead of a silent `null`.
        let expr = crate::markdown::compose::expression::parse("ctx.os").unwrap();
        assert!(matches!(
            crate::markdown::compose::expression::evaluate(&expr, &lookup),
            Err(ExpressionError::ContextProjectionInvariant { .. })
        ));
        // The unchecked accessor keeps its `Option` contract, and the group is
        // not re-captured behind the invariant's back.
        assert!(lookup.resolve_ctx("ctx.os").is_none());
        assert_eq!(lookup.captured_groups(), vec![ContextGroup::Os]);
    }

    #[test]
    fn checked_resolution_captures_only_the_groups_it_reaches() {
        let environment = HashMap::new();
        let lookup = CtxLookup::new(Path::new("."), &environment);

        assert!(lookup.get_checked("ctx.zzz").unwrap().is_none());
        assert!(lookup.get_checked("ctx").unwrap().is_none());
        assert!(lookup.get_checked("env.HOME").unwrap().is_none());
        assert!(lookup.captured_groups().is_empty(), "unknown keys capture nothing");

        assert!(lookup.get_checked("ctx.today").unwrap().is_some());
        assert_eq!(lookup.captured_groups(), vec![ContextGroup::DateTime]);
    }

    /// `ctx.agent` and `ctx.model` resolve lazily from the lookup's
    /// environment and capture only the Agent group, without pulling in repo,
    /// OS, or hardware probes.
    #[test]
    fn ctx_lookup_resolves_agent_and_model() {
        let environment = HashMap::from([
            ("AGENT".to_string(), "opencode".to_string()),
            ("MODEL".to_string(), "glm-5.2".to_string()),
        ]);
        let lookup = CtxLookup::new(Path::new("."), &environment);
        assert_eq!(lookup.get("ctx.agent"), Some(Value::String("opencode".to_string())));
        assert_eq!(lookup.get("ctx.model"), Some(Value::String("glm-5.2".to_string())));

        let captured = lookup.captured_groups();
        assert_eq!(
            captured.len(),
            1,
            "Only the Agent group should be captured, got {:?}",
            captured
        );
        assert!(captured.contains(&ContextGroup::Agent));
    }
}
