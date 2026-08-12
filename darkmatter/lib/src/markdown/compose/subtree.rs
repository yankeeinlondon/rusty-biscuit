//! Subtree compose with an injectable layered lookup (DM2).
//!
//! Exposes a public entry point that interpolates a frontmatter *subtree*
//! (a JSON value) through the same interpolation core as main compose, driven
//! by a caller-supplied layered lookup. This is the primitive Claudine uses
//! for both event-time resolution (C2) and early-binding shell resolution (C3).
//!
//! ## Layered Lookup
//!
//! [`LayeredLookup`] layers caller-injected named globals
//! ([`InjectedGlobal`]) over Darkmatter's own seed state
//! (`ctx`/`env`/`doc` + read-side functions via [`ResolutionContext`]).
//! Injected globals may be **eager** (a precomputed value) or **lazy** (a
//! closure evaluated on first access and memoized, so it sees the state at the
//! point of first reference exactly as `ctx` is at compose time). The API
//! takes arbitrary named globals — not three hardcoded names — so future
//! late-binding globals need no further Darkmatter change.
//!
//! ## Resolution Semantics
//!
//! Whole-value typing and mixed-string resolution rules match main compose
//! byte-for-byte: the subtree compose reuses [`Evaluator`]/[`interpolate_value`]
//! so a lifecycle string interpolated at event-time produces the same typed /
//! substituted result as the same string with the same data at compose-time.
//! Strictness is an **orthogonal** mode flag (see [`SubtreeStrictness`]): it
//! changes what happens on *failure* (typed error vs lenient empty), not how a
//! successful resolution is typed or substituted.
//!
//! [`ResolutionContext`]: super::expression::ResolutionContext
//! [`Evaluator`]: super::interpolation::Evaluator
//! [`interpolate_value`]: super::interpolation::interpolate_value

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use super::context::effective_state::EffectiveState;
use super::expression::{
    EvaluationLookup, Expr, ExpressionFinder, ResolutionContext, parse,
};
use super::interpolation::{Evaluator, interpolate_value};
use crate::markdown::types::MarkdownError;

/// A caller-injected named global available to subtree compose (DM2).
///
/// Layers over Darkmatter's own seed state (`ctx`/`env`/`doc` + read-side
/// functions via [`ResolutionContext`]). Eager globals carry a precomputed
/// value; lazy globals carry a closure that runs on first access and is
/// memoized, so it sees the state at the point of first reference exactly as
/// `ctx` is at compose time.
///
/// The API takes arbitrary named globals — not three hardcoded names — so
/// future late-binding globals need no further Darkmatter change.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::subtree::InjectedGlobal;
/// use serde_json::json;
///
/// let eager = InjectedGlobal::eager(json!({"msg": "boom"}));
/// let lazy = InjectedGlobal::lazy(|| json!({"phase": 2}));
/// ```
///
/// [`ResolutionContext`]: super::expression::ResolutionContext
#[derive(Clone)]
pub enum InjectedGlobal {
    /// A value computed before subtree compose was called.
    Eager(Value),

    /// A closure evaluated on first access within a subtree-compose call.
    ///
    /// Memoized per [`LayeredLookup`] instance: the closure runs at most once
    /// per subtree compose, only when its name is referenced — never eagerly
    /// at subtree-compose entry.
    Lazy(Arc<dyn Fn() -> Value + Send + Sync>),
}

impl InjectedGlobal {
    /// Creates an eager global from a precomputed value.
    pub fn eager(value: Value) -> Self {
        Self::Eager(value)
    }

    /// Creates a lazy global from a closure.
    ///
    /// The closure runs on first access (via `{{ name }}` or `{{ name.path }}`)
    /// during subtree compose and is memoized, so subsequent references within
    /// the same subtree compose reuse the cached value.
    pub fn lazy<F>(f: F) -> Self
    where
        F: Fn() -> Value + Send + Sync + 'static,
    {
        Self::Lazy(Arc::new(f))
    }
}

impl std::fmt::Debug for InjectedGlobal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Eager(v) => f.debug_tuple("Eager").field(v).finish(),
            Self::Lazy(_) => f.debug_tuple("Lazy").field(&"Fn(..)").finish(),
        }
    }
}

/// Strictness mode for subtree compose (DM2).
///
/// Orthogonal to resolution semantics: it changes what happens on *failure*
/// (typed error vs lenient empty), not how a successful resolution is typed or
/// substituted.
///
/// ## When to use which
///
/// - **Strict** — Claudine uses strict mode for lifecycle communication/action
///   text so parse failures, fatal evaluation failures, and unresolved roots
///   become typed errors before any side effect is dispatched.
/// - **Lenient** — Darkmatter's existing body and mixed-string use cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubtreeStrictness {
    /// Lenient: malformed spans and unknown roots produce a degraded/empty
    /// string (existing body/mixed-string behavior). Unknown functions remain
    /// fatal (they can never resolve, so leaving the literal `{{ … }}` text
    /// would leak downstream).
    #[default]
    Lenient,

    /// Strict: malformed spans, unknown functions, and unknown roots return a
    /// typed error instead of a degraded string.
    ///
    /// A reference whose root is a *known* surface — a declared frontmatter
    /// key, `ctx`/`env`/`doc`, or an in-scope injected global — that resolves
    /// to `null`/empty still renders empty. Strictness targets *unknown* roots
    /// and malformed/illegal expressions only.
    Strict,
}

/// A lookup that layers caller-injected named globals over Darkmatter's own
/// seed state (DM2).
///
/// Injected globals take precedence over (shadow) frontmatter keys with the
/// same root name; resolution then falls back to the base [`EffectiveState`]
/// (`ctx.*`, `env.*`, `doc.*`, frontmatter keys, read-side functions via the
/// optional [`ResolutionContext`]).
///
/// Lazy globals are memoized per `LayeredLookup` instance: the closure runs at
/// most once and only when its name is referenced — never eagerly at
/// subtree-compose entry.
///
/// [`ResolutionContext`]: super::expression::ResolutionContext
pub struct LayeredLookup<'a> {
    state: &'a EffectiveState,
    globals: &'a HashMap<String, InjectedGlobal>,
    cache: Mutex<HashMap<String, Value>>,
    resolution_context: Option<ResolutionContext>,
}

impl<'a> std::fmt::Debug for LayeredLookup<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayeredLookup")
            .field("state", &self.state)
            .field("globals", &self.globals)
            .field("resolution_context", &self.resolution_context)
            .finish()
    }
}

impl<'a> LayeredLookup<'a> {
    /// Creates a new layered lookup.
    ///
    /// ## Arguments
    ///
    /// - `state` — the base seed state (frontmatter + `ctx.*` + `env.*` +
    ///   `doc.*`).
    /// - `globals` — caller-injected named globals layered over `state`.
    /// - `resolution_context` — enables read-side expression functions
    ///   (`file_exists`, `frontmatter`, `absolute`, …). Pass `None` for
    ///   context-free callers.
    pub fn new(
        state: &'a EffectiveState,
        globals: &'a HashMap<String, InjectedGlobal>,
        resolution_context: Option<ResolutionContext>,
    ) -> Self {
        Self {
            state,
            globals,
            cache: Mutex::new(HashMap::new()),
            resolution_context,
        }
    }

    /// Resolves an injected global value (eager or memoized lazy).
    ///
    /// Lazy globals are memoized: the closure runs at most once per
    /// `LayeredLookup` instance.
    fn resolve_global(&self, name: &str, global: &InjectedGlobal) -> Option<Value> {
        match global {
            InjectedGlobal::Eager(v) => Some(v.clone()),
            InjectedGlobal::Lazy(f) => {
                let mut cache = self.cache.lock().ok()?;
                if let Some(cached) = cache.get(name) {
                    return Some(cached.clone());
                }
                let value = f();
                cache.insert(name.to_string(), value.clone());
                Some(value)
            }
        }
    }
}

impl<'a> EvaluationLookup for LayeredLookup<'a> {
    fn get(&self, path: &str) -> Option<Value> {
        let root = path.split('.').next().unwrap_or(path);
        if let Some(global) = self.globals.get(root) {
            let value = self.resolve_global(root, global)?;
            if path == root {
                return Some(value);
            }
            return walk_dotted_path(&value, &path[root.len()..]);
        }
        self.state.get(path)
    }

    fn get_string(&self, path: &str) -> String {
        match self.get(path) {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s,
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(v) => v.to_string(),
        }
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        self.resolution_context.clone()
    }

    fn resolution_context_ref(&self) -> Option<&ResolutionContext> {
        // Borrowed path (Finding 12) — avoids cloning the context per read-side
        // function call during subtree evaluation.
        self.resolution_context.as_ref()
    }

    fn is_valid_context_variable(&self, name: &str) -> bool {
        self.state.is_valid_context_variable(name)
    }

    fn context_variable_names(&self) -> &[&'static str] {
        self.state.context_variable_names()
    }

    fn is_known_variable_root(&self, root: &str) -> bool {
        if self.globals.contains_key(root) {
            return true;
        }
        if root == "ctx" || root == "env" || root == "doc" {
            return true;
        }
        self.state.data().contains_key(root)
    }
}

/// Walks a dotted path (`.a.b` or `a.b`) through a JSON value.
fn walk_dotted_path(value: &Value, path: &str) -> Option<Value> {
    let trimmed = path.strip_prefix('.').unwrap_or(path);
    let mut current = value;
    for segment in trimmed.split('.') {
        if segment.is_empty() {
            continue;
        }
        current = match current {
            Value::Object(map) => map.get(segment)?,
            _ => return None,
        };
    }
    Some(current.clone())
}

/// Builder for subtree compose (DM2).
///
/// Interpolates a frontmatter subtree (a JSON value) through the same
/// interpolation core as main compose, driven by a caller-supplied layered
/// lookup. Reuses [`Evaluator`]/[`interpolate_value`] so whole-value typing and
/// mixed-string *resolution* rules match main compose byte-for-byte.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::subtree::{InjectedGlobal, SubtreeCompose};
/// use darkmatter::markdown::compose::EffectiveStateBuilder;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let mut fm = HashMap::new();
/// fm.insert("phase".to_string(), json!(2));
/// let state = EffectiveStateBuilder::new()
///     .with_frontmatter(fm)
///     .build()
///     .unwrap();
///
/// let result = SubtreeCompose::new(&json!("phase {{phase}} failed: {{err.msg}}"), &state)
///     .with_global("err", InjectedGlobal::eager(json!({"msg": "boom"})))
///     .strict()
///     .compose()
///     .unwrap();
/// assert_eq!(result, json!("phase 2 failed: boom"));
/// ```
///
/// [`Evaluator`]: super::interpolation::Evaluator
/// [`interpolate_value`]: super::interpolation::interpolate_value
#[derive(Debug)]
pub struct SubtreeCompose<'a> {
    value: &'a Value,
    base: &'a EffectiveState,
    globals: HashMap<String, InjectedGlobal>,
    strictness: SubtreeStrictness,
    resolution_context: Option<ResolutionContext>,
}

impl<'a> SubtreeCompose<'a> {
    /// Creates a new subtree-compose builder with no injected globals and
    /// default (lenient) strictness.
    pub fn new(value: &'a Value, base: &'a EffectiveState) -> Self {
        Self {
            value,
            base,
            globals: HashMap::new(),
            strictness: SubtreeStrictness::default(),
            resolution_context: None,
        }
    }

    /// Adds a single injected global.
    #[must_use]
    pub fn with_global(mut self, name: impl Into<String>, global: InjectedGlobal) -> Self {
        self.globals.insert(name.into(), global);
        self
    }

    /// Replaces the injected-globals map.
    #[must_use]
    pub fn with_globals(mut self, globals: HashMap<String, InjectedGlobal>) -> Self {
        self.globals = globals;
        self
    }

    /// Sets the strictness mode (default: lenient).
    #[must_use]
    pub fn with_strictness(mut self, strictness: SubtreeStrictness) -> Self {
        self.strictness = strictness;
        self
    }

    /// Shortcut for `with_strictness(SubtreeStrictness::Strict)`.
    #[must_use]
    pub fn strict(mut self) -> Self {
        self.strictness = SubtreeStrictness::Strict;
        self
    }

    /// Attaches a [`ResolutionContext`] so the read-side expression functions
    /// (`file_exists`, `frontmatter`, `absolute`, …) resolve during subtree
    /// compose. Pass this when the subtree references filesystem functions.
    ///
    /// [`ResolutionContext`]: super::expression::ResolutionContext
    #[must_use]
    pub fn with_resolution_context(mut self, ctx: ResolutionContext) -> Self {
        self.resolution_context = Some(ctx);
        self
    }

    /// Runs subtree compose, returning the interpolated value.
    ///
    /// ## Errors
    ///
    /// Returns [`MarkdownError::Transform`] when:
    /// - strict mode rejects a malformed span, unknown function, or unknown
    ///   root;
    /// - a whole-value `{{ expr }}` fails to parse or evaluate (fatal in both
    ///   lenient and strict modes);
    /// - an unknown function is referenced (fatal in both modes).
    ///
    /// [`MarkdownError::Transform`]: crate::markdown::types::MarkdownError::Transform
    pub fn compose(self) -> Result<Value, MarkdownError> {
        let lookup = LayeredLookup::new(self.base, &self.globals, self.resolution_context);
        compose_subtree_impl(self.value, &lookup, self.strictness)
    }
}

/// Convenience entry point: composes a subtree with the given injected globals
/// and strictness.
///
/// Equivalent to `SubtreeCompose::new(value, base).with_globals(globals).with_strictness(strictness).compose()`.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::subtree::{
///     compose_subtree, InjectedGlobal, SubtreeStrictness,
/// };
/// use darkmatter::markdown::compose::EffectiveStateBuilder;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let state = EffectiveStateBuilder::new()
///     .with_frontmatter([("phase".to_string(), json!(2))].into())
///     .build()
///     .unwrap();
///
/// let mut globals = HashMap::new();
/// globals.insert("err".to_string(), InjectedGlobal::eager(json!({"msg": "boom"})));
///
/// let result = compose_subtree(
///     &json!("phase {{phase}} failed: {{err.msg}}"),
///     &state,
///     globals,
///     SubtreeStrictness::Strict,
/// ).unwrap();
/// assert_eq!(result, json!("phase 2 failed: boom"));
/// ```
pub fn compose_subtree(
    value: &Value,
    base: &EffectiveState,
    globals: HashMap<String, InjectedGlobal>,
    strictness: SubtreeStrictness,
) -> Result<Value, MarkdownError> {
    SubtreeCompose::new(value, base)
        .with_globals(globals)
        .with_strictness(strictness)
        .compose()
}

/// Recursive subtree interpolation driver.
fn compose_subtree_impl(
    value: &Value,
    lookup: &LayeredLookup<'_>,
    strictness: SubtreeStrictness,
) -> Result<Value, MarkdownError> {
    match value {
        Value::String(s) => compose_string(s, lookup, strictness),
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                out.push(compose_subtree_impl(item, lookup, strictness)?);
            }
            Ok(Value::Array(out))
        }
        Value::Object(obj) => {
            let mut out = serde_json::Map::with_capacity(obj.len());
            for (k, v) in obj {
                out.insert(k.clone(), compose_subtree_impl(v, lookup, strictness)?);
            }
            Ok(Value::Object(out))
        }
        // Number, Bool, Null — pass through unchanged.
        other => Ok(other.clone()),
    }
}

/// Interpolates a single string value through the shared interpolation core.
fn compose_string(
    s: &str,
    lookup: &LayeredLookup<'_>,
    strictness: SubtreeStrictness,
) -> Result<Value, MarkdownError> {
    if matches!(strictness, SubtreeStrictness::Strict) {
        validate_strict_roots(s, lookup)?;
    }
    let fail_fast = matches!(strictness, SubtreeStrictness::Strict);
    let evaluator = Evaluator::new(lookup);
    let (resolved, _count, _warnings) =
        interpolate_value(s, &evaluator, fail_fast, "subtree-compose")?;
    Ok(resolved)
}

/// Strict-mode pre-pass: parses every `{{ }}` span and rejects unknown roots.
///
/// A whole-value parse failure is already fatal in `interpolate_value`; this
/// pass surfaces mixed-string parse failures (which lenient mode would degrade
/// to warnings) and unknown-root references (which lenient mode resolves to
/// empty) as typed errors.
///
/// Root collection follows expression short-circuit semantics (see
/// [`collect_variable_roots`]), so a root that `evaluate` would never reach —
/// inside a `||` fallback or an unchosen ternary branch — is not rejected. This
/// keeps the documented `{{ maybe || 'default' }}` migration path valid in
/// strict mode.
fn validate_strict_roots(s: &str, lookup: &LayeredLookup<'_>) -> Result<(), MarkdownError> {
    for loc in ExpressionFinder::find_all_plain(s) {
        let expr = parse(&loc.expression).map_err(|e| {
            MarkdownError::Transform(format!(
                "subtree-compose strict mode: failed to parse '{{{{ {} }}}}': {e}",
                loc.expression
            ))
        })?;
        let mut roots = Vec::new();
        collect_variable_roots(&expr, &mut roots);
        for root in roots {
            if !lookup.is_known_variable_root(&root) {
                return Err(MarkdownError::Transform(format!(
                    "subtree-compose strict mode: unknown root '{root}' in \
                     '{{{{ {} }}}}'",
                    loc.expression
                )));
            }
        }
    }
    Ok(())
}

/// Walks an `Expr` AST pushing the first dotted segment of every `Variable`
/// node's path onto `roots`.
///
/// Unlike the frontmatter dependency walker, this does not skip `ctx.*` or
/// `env.*`: their root (`ctx`, `env`) is a known namespace, and strict-mode
/// checks the *root* only (a `ctx.typo` has a known root and is allowed; a
/// future claudine-side scan polices the `typo` segment).
///
/// The walk follows expression-evaluation (short-circuit) semantics so the
/// strict pre-pass mirrors what `evaluate` actually reaches at runtime — the
/// same tolerance claudine's `find_undefined_top_level_variable` applies:
///
/// - `Expr::Fallback { .. }` is fully tolerant — no roots inside a fallback
///   subtree are collected. The `||` construct exists precisely to tolerate an
///   undefined/optional operand (`{{ maybe || 'default' }}`), so a miss inside
///   it is intentional, not a leak.
/// - `Expr::Ternary` descends into `condition` only — the condition is always
///   evaluated, but the branches intentionally tolerate undefined operands.
/// - Every other node (variable, unary, paren, member access, comparison,
///   binary, index, function-call args) is descended, so an unknown root buried
///   in e.g. `parent_dir(missing)` is still rejected.
fn collect_variable_roots(expr: &Expr, roots: &mut Vec<String>) {
    match expr {
        Expr::Variable(path) => {
            let root = path.split('.').next().unwrap_or(path);
            if !root.is_empty() {
                roots.push(root.to_string());
            }
        }
        Expr::StringLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::BoolLiteral(_) => {}
        Expr::UnaryNot(inner)
        | Expr::UnaryMinus(inner)
        | Expr::Paren(inner)
        | Expr::MemberAccess { base: inner, .. } => collect_variable_roots(inner, roots),
        Expr::Fallback { .. } => {}
        Expr::Ternary { condition, .. } => {
            collect_variable_roots(condition, roots);
        }
        Expr::Comparison { left, right, .. } | Expr::Binary { left, right, .. } => {
            collect_variable_roots(left, roots);
            collect_variable_roots(right, roots);
        }
        Expr::Index { base, index } => {
            collect_variable_roots(base, roots);
            collect_variable_roots(index, roots);
        }
        Expr::FunctionCall { args, .. } => {
            for arg in args {
                collect_variable_roots(arg, roots);
            }
        }
        Expr::ArrayLiteral(items) => {
            for item in items {
                collect_variable_roots(item, roots);
            }
        }
        Expr::ObjectLiteral(entries) => {
            for (_, value) in entries {
                collect_variable_roots(value, roots);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::EffectiveStateBuilder;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn state_with(fm: Vec<(&str, Value)>) -> EffectiveState {
        let fm: HashMap<String, Value> = fm
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        // No fixture here reads `ctx.*`, so the builder's full-capture default
        // would walk the working tree once per state for values nothing reads.
        EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(crate::markdown::compose::ComposeContext::capture_minimal())
            .build()
            .unwrap()
    }

    // ── DM2: injected eager + lazy globals resolve ───────────────────────

    #[test]
    fn dm2_resolves_injected_eager_global() {
        let state = state_with(vec![("phase", json!(2))]);
        let mut globals = HashMap::new();
        globals.insert(
            "err".to_string(),
            InjectedGlobal::eager(json!({"msg": "boom"})),
        );
        let result = compose_subtree(
            &json!("phase {{phase}} failed: {{err.msg}}"),
            &state,
            globals,
            SubtreeStrictness::Lenient,
        )
        .unwrap();
        assert_eq!(result, json!("phase 2 failed: boom"));
    }

    #[test]
    fn dm2_resolves_injected_lazy_global() {
        let state = state_with(vec![("phase", json!(2))]);
        let mut globals = HashMap::new();
        globals.insert(
            "current".to_string(),
            InjectedGlobal::lazy(|| json!({"ctx": {"today": "2026-06-24"}})),
        );
        let result = compose_subtree(
            &json!("today is {{current.ctx.today}}"),
            &state,
            globals,
            SubtreeStrictness::Lenient,
        )
        .unwrap();
        assert_eq!(result, json!("today is 2026-06-24"));
    }

    #[test]
    fn dm2_injected_global_shadows_frontmatter_key() {
        let state = state_with(vec![("phase", json!(2))]);
        let mut globals = HashMap::new();
        globals.insert("phase".to_string(), InjectedGlobal::eager(json!(99)));
        let result = compose_subtree(
            &json!("{{phase}}"),
            &state,
            globals,
            SubtreeStrictness::Lenient,
        )
        .unwrap();
        assert_eq!(result, json!(99));
    }

    // ── DM2: layered seed state still resolves ───────────────────────────

    #[test]
    fn dm2_layered_seed_state_resolves() {
        let state = state_with(vec![
            ("phase", json!(3)),
            (
                "config",
                json!({"artifact": {"path": "/tmp/out"}}),
            ),
        ]);
        let result = compose_subtree(
            &json!("artifact={{config.artifact.path}} phase={{phase}}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Lenient,
        )
        .unwrap();
        assert_eq!(result, json!("artifact=/tmp/out phase=3"));
    }

    // ── DM2: whole-value typing matches main compose ─────────────────────

    #[test]
    fn dm2_whole_value_typed_result_matches_main_compose() {
        let state = state_with(vec![("flag", json!(false))]);
        // Whole-value single span: typed result preserved.
        let result = compose_subtree(
            &json!("{{flag}}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Lenient,
        )
        .unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn dm2_mixed_string_result_matches_main_compose() {
        let state = state_with(vec![("flag", json!(true))]);
        // Mixed string: concatenated as a string.
        let result = compose_subtree(
            &json!("flag={{flag}}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Lenient,
        )
        .unwrap();
        assert_eq!(result, json!("flag=true"));
    }

    // ── DM2: lazy global evaluated only when referenced ──────────────────

    #[test]
    fn dm2_lazy_global_evaluated_only_when_referenced() {
        let state = state_with(vec![]);
        let count = Arc::new(AtomicUsize::new(0));
        let count_for_closure = count.clone();
        let mut globals = HashMap::new();
        globals.insert(
            "current".to_string(),
            InjectedGlobal::lazy(move || {
                count_for_closure.fetch_add(1, Ordering::SeqCst);
                json!({"phase": 1})
            }),
        );
        // String does NOT reference `current`.
        let result = compose_subtree(
            &json!("no reference here"),
            &state,
            globals,
            SubtreeStrictness::Lenient,
        )
        .unwrap();
        assert_eq!(result, json!("no reference here"));
        assert_eq!(count.load(Ordering::SeqCst), 0, "lazy global must not run");
    }

    #[test]
    fn dm2_lazy_global_evaluated_at_most_once() {
        let state = state_with(vec![]);
        let count = Arc::new(AtomicUsize::new(0));
        let count_for_closure = count.clone();
        let mut globals = HashMap::new();
        globals.insert(
            "current".to_string(),
            InjectedGlobal::lazy(move || {
                count_for_closure.fetch_add(1, Ordering::SeqCst);
                json!({"phase": 7})
            }),
        );
        // References `current` twice — the closure must run at most once.
        let result = compose_subtree(
            &json!("{{current.phase}} then {{current.phase}}"),
            &state,
            globals,
            SubtreeStrictness::Lenient,
        )
        .unwrap();
        assert_eq!(result, json!("7 then 7"));
        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "lazy global must run at most once"
        );
    }

    // ── DM2: strict mode ────────────────────────────────────────────────

    #[test]
    fn dm2_strict_rejects_unknown_root() {
        let state = state_with(vec![("phase", json!(2))]);
        let result = compose_subtree(
            &json!("{{spec_fil}}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Strict,
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("unknown root") && err.contains("spec_fil"),
            "error should name the unknown root: {err}"
        );
    }

    #[test]
    fn dm2_strict_known_but_empty_renders_empty() {
        let state = state_with(vec![("spec_file", json!(null))]);
        let result = compose_subtree(
            &json!("spec={{spec_file}}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Strict,
        )
        .unwrap();
        assert_eq!(result, json!("spec="));
    }

    #[test]
    fn dm2_strict_rejects_malformed_span() {
        let state = state_with(vec![]);
        let result = compose_subtree(
            &json!("{{ > broken }}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Strict,
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("failed to parse"),
            "error should be a parse failure: {err}"
        );
    }

    #[test]
    fn dm2_strict_rejects_unknown_function() {
        let state = state_with(vec![("phase", json!(2))]);
        let result = compose_subtree(
            &json!("{{ bogus_fn(phase) }}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Strict,
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Unknown function") || err.to_lowercase().contains("bogus_fn"),
            "error should name the unknown function: {err}"
        );
    }

    #[test]
    fn dm2_strict_allows_known_root_resolving_empty() {
        // A declared frontmatter key referenced through doc. namespace is
        // known even when it resolves empty.
        let state = state_with(vec![("phase", json!(2))]);
        let result = compose_subtree(
            &json!("{{ phase }}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Strict,
        )
        .unwrap();
        assert_eq!(result, json!(2));
    }

    #[test]
    fn dm2_strict_accepts_fallback_for_unknown_optional_root() {
        // The `||` construct is the documented migration path for an unknown
        // optional name: strict mode must accept it and resolve to the fallback.
        let state = state_with(vec![]);
        let result = compose_subtree(
            &json!("{{ missing || 'default' }}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Strict,
        )
        .unwrap();
        assert_eq!(result, json!("default"));
    }

    #[test]
    fn dm2_strict_accepts_ternary_unknown_in_unchosen_branch() {
        // Known truthy condition picks the then-branch; an unknown root in the
        // unchosen else-branch is tolerated (the branches are never both run).
        let state = state_with(vec![("phase", json!(2))]);
        let result = compose_subtree(
            &json!("{{ phase ? 'has-phase' : missing_else }}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Strict,
        )
        .unwrap();
        assert_eq!(result, json!("has-phase"));
    }

    #[test]
    fn dm2_strict_rejects_unknown_root_in_ternary_condition() {
        // The condition is always evaluated, so an unknown root there is still
        // a leak and must be rejected.
        let state = state_with(vec![]);
        let result = compose_subtree(
            &json!("{{ unknown_cond ? 'a' : 'b' }}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Strict,
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("unknown root") && err.contains("unknown_cond"),
            "ternary condition root must still be rejected: {err}"
        );
    }

    #[test]
    fn dm2_strict_rejects_unknown_root_in_function_arg() {
        // Function-call args are still descended: an unknown root buried in
        // `parent_dir(missing_thing)` is not fallback/ternary-guarded.
        let state = state_with(vec![]);
        let result = compose_subtree(
            &json!("{{ parent_dir(missing_thing) }}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Strict,
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("unknown root") && err.contains("missing_thing"),
            "function-call arg root must still be rejected: {err}"
        );
    }

    #[test]
    fn dm2_lenient_tolerates_unknown_root() {
        let state = state_with(vec![]);
        let result = compose_subtree(
            &json!("value={{unknown_thing}}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Lenient,
        )
        .unwrap();
        assert_eq!(result, json!("value="));
    }

    #[test]
    fn dm2_lenient_tolerates_malformed_span_in_mixed_string() {
        let state = state_with(vec![]);
        let result = compose_subtree(
            &json!("bad={{ > broken }}"),
            &state,
            HashMap::new(),
            SubtreeStrictness::Lenient,
        )
        .unwrap();
        // Lenient: the malformed span degrades to its original text.
        let out = result.as_str().unwrap();
        assert!(out.starts_with("bad="));
    }

    // ── DM2: subtree (object/array) recursion ────────────────────────────

    #[test]
    fn dm2_composes_object_subtree() {
        let state = state_with(vec![("phase", json!(2))]);
        let mut globals = HashMap::new();
        globals.insert(
            "err".to_string(),
            InjectedGlobal::eager(json!({"msg": "boom"})),
        );
        let input = json!({
            "message": "phase {{phase}}: {{err.msg}}",
            "details": {
                "error": "{{err.msg}}",
                "phase": "{{phase}}"
            },
            "tags": ["{{err.msg}}", "{{phase}}"]
        });
        let result = compose_subtree(&input, &state, globals, SubtreeStrictness::Strict).unwrap();
        assert_eq!(
            result,
            json!({
                "message": "phase 2: boom",
                "details": {
                    "error": "boom",
                    // Whole-value single span preserves type.
                    "phase": 2
                },
                "tags": ["boom", 2]
            })
        );
    }

    #[test]
    fn dm2_passthrough_non_string_scalars() {
        let state = state_with(vec![]);
        let input = json!({
            "count": 42,
            "active": true,
            "missing": null
        });
        let result = compose_subtree(&input, &state, HashMap::new(), SubtreeStrictness::Strict)
            .unwrap();
        assert_eq!(result, input);
    }

    // ── DM2: builder API ────────────────────────────────────────────────

    #[test]
    fn dm2_builder_api() {
        let state = state_with(vec![("phase", json!(2))]);
        let result = SubtreeCompose::new(&json!("{{phase}}"), &state)
            .with_global("err", InjectedGlobal::eager(json!({"msg": "x"})))
            .strict()
            .compose()
            .unwrap();
        assert_eq!(result, json!(2));
    }
}
