//! Subtree compose over a host binding session (DM2).
//!
//! Exposes a public entry point that interpolates a frontmatter *subtree*
//! (a JSON value) through the same interpolation core as main compose. This is
//! the primitive Claudine uses for both event-time resolution (C2) and
//! early-binding shell resolution (C3).
//!
//! ## Bindings
//!
//! Each [`SubtreeCompose::compose`] call associates one
//! [`EvaluationSession`]: the caller's runtime globals ([`RuntimeBinding`]:
//! eager, lazy, or unavailable) over Darkmatter's own seed state
//! (`ctx`/`env`/`doc`/`current`/`current_env` + read-side functions via
//! [`ResolutionContext`]). A root resolves as a reserved namespace, then a
//! registered global, then a document property; an absent document property is
//! `null`. A caller that declares its globals in a [`BindingView`] gets the
//! view's checks (every declared global supplied, no contradiction); without a
//! view each supplied global is simply available. A reserved namespace name is
//! never a valid global.
//!
//! ## Resolution Semantics
//!
//! Whole-value typing and mixed-string resolution rules match main compose
//! byte-for-byte: the subtree compose reuses [`Evaluator`]/[`interpolate_value`]
//! so a lifecycle string interpolated at event-time produces the same typed /
//! substituted result as the same string with the same data at compose-time.
//!
//! ## Expression Failures
//!
//! Like full-document composition, subtree compose fails on any `{{ … }}` that
//! cannot be parsed or evaluated: a malformed span, an unknown function, a
//! rejected argument, or a read of an unavailable global. That is ordinary
//! error propagation; an absent document property is not a failure.
//!
//! [`ResolutionContext`]: super::expression::ResolutionContext
//! [`Evaluator`]: super::interpolation::Evaluator
//! [`interpolate_value`]: super::interpolation::interpolate_value

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use super::context::effective_state::EffectiveState;
use super::expression::{
    Availability, BindingError, BindingView, EvaluationSession, ExpressionError,
    ResolutionContext, RuntimeBinding, ScopeId,
};
use super::interpolation::{Evaluator, ExpressionFailurePolicy, interpolate_value};
use crate::markdown::types::{MarkdownError, SourceRef};

/// A caller-supplied runtime global that owns its provider.
///
/// Eager globals carry a precomputed value; lazy globals carry a closure that
/// runs on first read and is cached for the session, so it sees the state at
/// the point of first reference exactly as `ctx` is at compose time; an
/// unavailable global fails every read with its reason.
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
pub type InjectedGlobal = RuntimeBinding<'static>;

/// The scope of the view synthesized for a caller that declares none.
const UNDECLARED_SCOPE: &str = "darkmatter.subtree";

/// Associates one evaluation session: `globals` over `base`, checked against
/// `view`.
///
/// Without a view, one is synthesized in which each supplied global is
/// available (or unavailable with its own reason), so only the root checks
/// apply: a reserved namespace or dotted name is still rejected. The session
/// is the lookup for one direct evaluation or one subtree composition; each
/// lazy global runs at most once in it.
///
/// ## Errors
///
/// Any [`BindingError`] configuration variant from
/// [`BindingView`] construction or [`EvaluationSession::associate`].
pub fn layered_session<'a>(
    base: &'a EffectiveState,
    globals: HashMap<String, RuntimeBinding<'a>>,
    view: Option<Arc<BindingView>>,
    resolution_context: Option<ResolutionContext>,
) -> Result<EvaluationSession<'a>, BindingError> {
    let view = match view {
        Some(view) => view,
        None => Arc::new(undeclared_view(&globals)?),
    };
    let session = EvaluationSession::associate(view, base, globals)?;
    Ok(match resolution_context {
        Some(context) => session.with_resolution_context(context),
        None => session,
    })
}

/// The view for a caller that declares nothing: every supplied global as it
/// was supplied.
fn undeclared_view(globals: &HashMap<String, RuntimeBinding<'_>>) -> Result<BindingView, BindingError> {
    globals
        .iter()
        .fold(BindingView::builder(ScopeId::new(UNDECLARED_SCOPE)), |builder, (root, binding)| {
            let availability = match binding {
                RuntimeBinding::Unavailable(reason) => Availability::Unavailable(reason.clone()),
                RuntimeBinding::Eager(_) | RuntimeBinding::Lazy(_) => Availability::Available,
            };
            builder.declare(root.clone(), "caller-supplied global", availability)
        })
        .build()
}

/// Builder for subtree compose (DM2).
///
/// Interpolates a frontmatter subtree (a JSON value) through the same
/// interpolation core as main compose, against one [`EvaluationSession`]. Reuses
/// [`Evaluator`]/[`interpolate_value`] so whole-value typing and mixed-string
/// *resolution* rules match main compose byte-for-byte.
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
    globals: HashMap<String, RuntimeBinding<'a>>,
    view: Option<Arc<BindingView>>,
    resolution_context: Option<ResolutionContext>,
}

impl<'a> SubtreeCompose<'a> {
    /// Creates a new subtree-compose builder with no globals and no view.
    pub fn new(value: &'a Value, base: &'a EffectiveState) -> Self {
        Self {
            value,
            base,
            globals: HashMap::new(),
            view: None,
            resolution_context: None,
        }
    }

    /// Adds a single runtime global.
    #[must_use]
    pub fn with_global(mut self, name: impl Into<String>, global: RuntimeBinding<'a>) -> Self {
        self.globals.insert(name.into(), global);
        self
    }

    /// Replaces the runtime-globals map.
    #[must_use]
    pub fn with_globals(mut self, globals: HashMap<String, RuntimeBinding<'a>>) -> Self {
        self.globals = globals;
        self
    }

    /// Checks the globals against `view` at association.
    #[must_use]
    pub fn with_binding_view(mut self, view: Arc<BindingView>) -> Self {
        self.view = Some(view);
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
    /// [`MarkdownError::Interpolation`] whose typed cause is
    /// [`ExpressionError::Binding`] when the globals fail association, before
    /// anything is evaluated; otherwise the error of the first span that fails
    /// to parse or evaluate.
    ///
    /// [`MarkdownError::Interpolation`]: crate::markdown::types::MarkdownError::Interpolation
    pub fn compose(self) -> Result<Value, MarkdownError> {
        let session =
            layered_session(self.base, self.globals, self.view, self.resolution_context)
                .map_err(association_error)?;
        compose_subtree_impl(self.value, &session)
    }
}

/// Convenience entry point: composes a subtree with the given globals.
///
/// Equivalent to `SubtreeCompose::new(value, base).with_globals(globals).compose()`.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::subtree::{compose_subtree, InjectedGlobal};
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
/// let result = compose_subtree(&json!("phase {{phase}} failed: {{err.msg}}"), &state, globals)
///     .unwrap();
/// assert_eq!(result, json!("phase 2 failed: boom"));
/// ```
pub fn compose_subtree<'a>(
    value: &'a Value,
    base: &'a EffectiveState,
    globals: HashMap<String, RuntimeBinding<'a>>,
) -> Result<Value, MarkdownError> {
    SubtreeCompose::new(value, base).with_globals(globals).compose()
}

/// A failed association, carried on the typed interpolation channel so a
/// caller downcasts the same [`BindingError`] it would get from evaluation.
fn association_error(error: BindingError) -> MarkdownError {
    let root = error.root().to_string();
    MarkdownError::Interpolation {
        key: None,
        expression: root.clone(),
        source: Box::new(SourceRef::Effective {
            rendered: root,
            origin_key: None,
        }),
        cause: Box::new(ExpressionError::Binding(Box::new(error))),
    }
}

/// Recursive subtree interpolation driver.
fn compose_subtree_impl(value: &Value, session: &EvaluationSession<'_>) -> Result<Value, MarkdownError> {
    match value {
        Value::String(s) => compose_string(s, session),
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                out.push(compose_subtree_impl(item, session)?);
            }
            Ok(Value::Array(out))
        }
        Value::Object(obj) => {
            let mut out = serde_json::Map::with_capacity(obj.len());
            for (k, v) in obj {
                out.insert(k.clone(), compose_subtree_impl(v, session)?);
            }
            Ok(Value::Object(out))
        }
        // Number, Bool, Null — pass through unchanged.
        other => Ok(other.clone()),
    }
}

/// Interpolates a single string value through the shared interpolation core.
fn compose_string(s: &str, session: &EvaluationSession<'_>) -> Result<Value, MarkdownError> {
    let evaluator = Evaluator::new(session);
    let (resolved, _count, _warnings) =
        interpolate_value(s, &evaluator, ExpressionFailurePolicy::Strict, "subtree-compose")?;
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::EffectiveStateBuilder;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
        )
        .unwrap();
        assert_eq!(result, json!("phase 2 failed: boom"));
    }

    #[test]
    fn dm2_resolves_injected_lazy_global() {
        let state = state_with(vec![("phase", json!(2))]);
        let mut globals = HashMap::new();
        globals.insert(
            "snapshot".to_string(),
            InjectedGlobal::lazy(|| json!({"today": "2026-06-24"})),
        );
        let result = compose_subtree(
            &json!("today is {{snapshot.today}}"),
            &state,
            globals,
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
            "snapshot".to_string(),
            InjectedGlobal::lazy(move || {
                count_for_closure.fetch_add(1, Ordering::SeqCst);
                json!({"phase": 1})
            }),
        );
        // String does NOT reference `snapshot`.
        let result = compose_subtree(
            &json!("no reference here"),
            &state,
            globals,
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
            "snapshot".to_string(),
            InjectedGlobal::lazy(move || {
                count_for_closure.fetch_add(1, Ordering::SeqCst);
                json!({"phase": 7})
            }),
        );
        // References `snapshot` twice — the closure must run at most once.
        let result = compose_subtree(
            &json!("{{snapshot.phase}} then {{snapshot.phase}}"),
            &state,
            globals,
        )
        .unwrap();
        assert_eq!(result, json!("7 then 7"));
        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "lazy global must run at most once"
        );
    }

    // ── Failures and absence ───────────────────────────────────────────

    #[test]
    fn an_absent_property_is_null_whole_value_and_empty_in_a_mixed_string() {
        let state = state_with(vec![("phase", json!(2))]);
        let whole = compose_subtree(&json!("{{spec_fil}}"), &state, HashMap::new()).unwrap();
        assert_eq!(whole, Value::Null);
        let mixed = compose_subtree(&json!("spec=[{{spec_fil}}]"), &state, HashMap::new()).unwrap();
        assert_eq!(mixed, json!("spec=[]"));
        let fallback =
            compose_subtree(&json!("{{ missing || 'default' }}"), &state, HashMap::new()).unwrap();
        assert_eq!(fallback, json!("default"));
        let ternary =
            compose_subtree(&json!("{{ missing ? 'a' : 'b' }}"), &state, HashMap::new()).unwrap();
        assert_eq!(ternary, json!("b"));
    }

    #[test]
    fn a_bare_name_never_reads_ctx() {
        let state = state_with(vec![]);
        let bare = compose_subtree(&json!("[{{today}}]"), &state, HashMap::new()).unwrap();
        assert_eq!(bare, json!("[]"));
        let namespaced = compose_subtree(&json!("{{ctx.today}}"), &state, HashMap::new()).unwrap();
        assert!(namespaced.as_str().is_some_and(|today| !today.is_empty()), "{namespaced}");
    }

    #[test]
    fn malformed_spans_and_unknown_functions_fail_in_any_position() {
        let state = state_with(vec![("phase", json!(2))]);
        for input in ["{{ > broken }}", "bad={{ > broken }}", "{{ bogus_fn(phase) }}", "x {{ bogus_fn(phase) }}"] {
            let error = compose_subtree(&json!(input), &state, HashMap::new())
                .expect_err(input);
            assert!(
                matches!(error, MarkdownError::Interpolation { .. }),
                "{input}: expected a typed interpolation error, got {error:?}"
            );
        }
    }

    #[test]
    fn an_unavailable_global_fails_and_never_reads_the_document() {
        use crate::markdown::compose::expression::{UnavailabilityReason, BindingError};

        let state = state_with(vec![("group", json!("document"))]);
        let error = SubtreeCompose::new(&json!("{{ group || 'x' }}"), &state)
            .with_global("group", InjectedGlobal::unavailable(UnavailabilityReason::new("host.outside-group")))
            .compose()
            .expect_err("an unavailable global is a typed error");
        let MarkdownError::Interpolation { cause, .. } = &error else {
            panic!("expected an interpolation error, got {error:?}");
        };
        assert!(matches!(
            cause.as_ref(),
            ExpressionError::Binding(binding) if matches!(binding.as_ref(), BindingError::Unavailable(_))
        ));
        let document = SubtreeCompose::new(&json!("{{ doc.group }}"), &state)
            .with_global("group", InjectedGlobal::unavailable(UnavailabilityReason::new("host.outside-group")))
            .compose()
            .unwrap();
        assert_eq!(document, json!("document"));
    }

    #[test]
    fn a_reserved_root_cannot_be_registered_and_no_provider_runs() {
        use crate::markdown::compose::expression::BindingError;

        let state = state_with(vec![]);
        for reserved in ["doc", "ctx", "env", "current", "current_env"] {
            let error = SubtreeCompose::new(&json!("{{ x }}"), &state)
                .with_global(reserved, InjectedGlobal::lazy(|| panic!("never evaluated")))
                .compose()
                .expect_err(reserved);
            let MarkdownError::Interpolation { cause, .. } = &error else {
                panic!("expected an interpolation error, got {error:?}");
            };
            assert!(
                matches!(
                    cause.as_ref(),
                    ExpressionError::Binding(binding)
                        if matches!(binding.as_ref(), BindingError::ReservedName { root } if root == reserved)
                ),
                "{reserved}: {cause:?}"
            );
        }
    }

    #[test]
    fn a_declared_view_is_enforced_at_association() {
        use crate::markdown::compose::expression::BindingError;

        let state = state_with(vec![]);
        let view = Arc::new(
            BindingView::builder(ScopeId::new("test.scope"))
                .declare("err", "the error", Availability::Available)
                .declare("timing", "the timing", Availability::Available)
                .build()
                .unwrap(),
        );
        let error = SubtreeCompose::new(&json!("{{ 1 }}"), &state)
            .with_binding_view(view.clone())
            .with_global("err", InjectedGlobal::eager(json!(null)))
            .compose()
            .expect_err("timing is declared but omitted");
        let MarkdownError::Interpolation { cause, .. } = &error else {
            panic!("expected an interpolation error, got {error:?}");
        };
        assert!(matches!(
            cause.as_ref(),
            ExpressionError::Binding(binding)
                if matches!(binding.as_ref(), BindingError::OmittedDeclaredGlobal { root, .. } if root == "timing")
        ));

        let resolved = SubtreeCompose::new(&json!("{{ err }}|{{ timing.ms }}"), &state)
            .with_binding_view(view)
            .with_global("err", InjectedGlobal::eager(json!(null)))
            .with_global("timing", InjectedGlobal::eager(json!({"ms": 5})))
            .compose()
            .unwrap();
        assert_eq!(resolved, json!("|5"));
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
        let result = compose_subtree(&input, &state, globals).unwrap();
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
        let result = compose_subtree(&input, &state, HashMap::new())
            .unwrap();
        assert_eq!(result, input);
    }

    // ── DM2: builder API ────────────────────────────────────────────────

    #[test]
    fn dm2_builder_api() {
        let state = state_with(vec![("phase", json!(2))]);
        let result = SubtreeCompose::new(&json!("{{phase}}"), &state)
            .with_global("err", InjectedGlobal::eager(json!({"msg": "x"})))
            .compose()
            .unwrap();
        assert_eq!(result, json!(2));
    }
}
