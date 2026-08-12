//! Lazy-capturing `ctx.*` resolver for expression evaluation.
//!
//! Provides [`CtxLookup`], a standalone [`EvaluationLookup`] implementation
//! that resolves `ctx.*` paths via demand-driven runtime context capture.
//! This type was extracted from [`ShortcutLookup`](super::conditions::ShortcutLookup)
//! so that `ctx.*` resolution can be composed on top of any other lookup.

use super::EvaluationLookup;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Lazy-capturing `ctx.*` resolver, suitable for composing on top of any
/// `EvaluationLookup` that handles non-`ctx` paths.
#[derive(Debug)]
pub struct CtxLookup<'a> {
    work_dir: &'a Path,
    cache: RefCell<HashMap<String, Value>>,
    captured: RefCell<HashSet<super::super::context::capture::ContextGroup>>,
}

impl<'a> CtxLookup<'a> {
    /// Creates a new `CtxLookup` for the given working directory.
    pub fn new(work_dir: &'a Path) -> Self {
        Self {
            work_dir,
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
    pub fn resolve_ctx(&self, path: &str) -> Option<Value> {
        let ctx_key = path.strip_prefix("ctx.")?;

        // Check cache first
        if let Some(cached) = self.cache.borrow().get(ctx_key) {
            return Some(cached.clone());
        }

        // Determine which group this key belongs to
        if let Some(group) = super::super::context::capture::ContextGroup::for_key(ctx_key) {
            let need_capture = !self.captured.borrow().contains(&group);
            if need_capture {
                self.capture_group(group);
            }
        }

        self.cache.borrow().get(ctx_key).cloned()
    }

    /// Captures a single context group and merges its values into the cache.
    fn capture_group(&self, group: super::super::context::capture::ContextGroup) {
        let (values, _diagnostics, _timings, _environment) =
            super::super::context::capture::capture_runtime_context_for_groups(
                self.work_dir,
                &[group],
            );

        let mut cache = self.cache.borrow_mut();
        let mut captured = self.captured.borrow_mut();
        for (key, value) in values {
            cache.insert(key, value);
        }
        captured.insert(group);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::context::capture::ContextGroup;

    #[test]
    fn ctx_lookup_returns_none_for_non_ctx_paths() {
        let lookup = CtxLookup::new(Path::new("."));
        assert!(lookup.get("foo").is_none());
        assert!(lookup.get("env.HOME").is_none());
        assert!(lookup.get("git.branch").is_none());
    }

    #[test]
    fn ctx_lookup_resolves_today() {
        let lookup = CtxLookup::new(Path::new("."));
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
        let lookup = CtxLookup::new(Path::new("."));
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
        let lookup = CtxLookup::new(Path::new("."));
        assert!(
            lookup.get("ctx.zzz").is_none(),
            "Unknown ctx key should return None"
        );
    }

    #[test]
    fn ctx_lookup_bare_ctx_returns_none() {
        let lookup = CtxLookup::new(Path::new("."));
        assert!(
            lookup.get("ctx").is_none(),
            "Bare 'ctx' token should return None"
        );
    }

    /// `ctx.agent` and `ctx.model` resolve lazily and capture only the Agent
    /// group, without pulling in repo, OS, or hardware probes.
    #[test]
    #[serial_test::serial(env_agent_model)]
    fn ctx_lookup_resolves_agent_and_model() {
        let previous = std::env::var("AGENT").ok();
        let previous_model = std::env::var("MODEL").ok();
        unsafe {
            std::env::set_var("AGENT", "opencode");
            std::env::set_var("MODEL", "glm-5.2");
        }

        let lookup = CtxLookup::new(Path::new("."));
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

        unsafe {
            match previous {
                Some(v) => std::env::set_var("AGENT", v),
                None => std::env::remove_var("AGENT"),
            }
            match previous_model {
                Some(v) => std::env::set_var("MODEL", v),
                None => std::env::remove_var("MODEL"),
            }
        }
    }
}
