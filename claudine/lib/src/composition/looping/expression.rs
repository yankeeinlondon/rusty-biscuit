//! Loop condition expression evaluation.

use std::path::Path;

use darkmatter::markdown::compose::expression::{
    EvaluationLookup, ResolutionContext, evaluate, is_truthy, parse_condition,
};
use serde_json::{Map, Value};

use super::super::error::{CompositionError, LoopExpressionCause};
use super::super::types::LoopCondition;

/// Ambient values injected into loop condition and prompt evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopAmbient {
    /// 1-based iteration index.
    pub iteration: usize,
    /// Whether this is the first iteration.
    pub is_first: bool,
    /// Whether this iteration is expected to be the last.
    pub is_last: bool,
    /// Captured output from the previous iteration.
    pub last_output: String,
    /// Exit code from the previous iteration.
    pub last_exit_code: i32,
}

impl LoopAmbient {
    /// Construct ambient values for an iteration.
    pub fn new(
        iteration: usize,
        is_first: bool,
        is_last: bool,
        last_output: impl Into<String>,
        last_exit_code: i32,
    ) -> Self {
        Self {
            iteration,
            is_first,
            is_last,
            last_output: last_output.into(),
            last_exit_code,
        }
    }
}

/// Darkmatter expression lookup for loop frontmatter plus ambient variables.
///
/// Resolution order:
///
/// 1. Reserved `doc` namespace (bare `doc` → the whole frontmatter object;
///    `doc.<path>` → dotted traversal into it). Intercepted first so a missing
///    `doc.<path>` never collapses into a same-named ambient/env/frontmatter key.
/// 2. `env.NAME`
/// 3. Ambient variables (`_loop_count`, `_loop_is_first`, `_loop_is_last`,
///    `_loop_last_output`, `_loop_last_exit_code`)
/// 4. Frontmatter properties, including nested object paths via `.`
///
/// When constructed with a base directory ([`with_base_dir`](Self::with_base_dir)),
/// the lookup exposes a [`ResolutionContext`] rooted at the prompt's parent so
/// read-side expression functions (`file_exists`, `absolute`, `relative`, …)
/// resolve implicit document-authored references repository-first, then against
/// the document directory. The probe re-runs each iteration while the request
/// snapshot and source remain fixed.
///
/// [`with_file_ref_fallback_dir`](Self::with_file_ref_fallback_dir) retains the
/// captured launch area as diagnostic metadata only. It is not a candidate for
/// these document-authored references; genuinely launch-relative top-level CLI
/// references are resolved before loop evaluation.
#[derive(Debug, Clone, Copy)]
pub struct LoopExpressionLookup<'a> {
    frontmatter: &'a Map<String, Value>,
    ambient: &'a LoopAmbient,
    base_dir: Option<&'a Path>,
    file_ref_fallback_dir: Option<&'a Path>,
    file_resolution_context: Option<&'a biscuit_file::FileResolutionContext>,
    source_path: Option<&'a Path>,
}

impl<'a> LoopExpressionLookup<'a> {
    /// Create a lookup over the current loop state.
    pub fn new(frontmatter: &'a Map<String, Value>, ambient: &'a LoopAmbient) -> Self {
        Self {
            frontmatter,
            ambient,
            base_dir: None,
            file_ref_fallback_dir: None,
            file_resolution_context: None,
            source_path: None,
        }
    }

    /// Root read-side expression functions at `base_dir` (typically the prompt
    /// document's parent directory). `None` leaves the lookup context-free.
    #[must_use]
    pub fn with_base_dir(mut self, base_dir: Option<&'a Path>) -> Self {
        self.base_dir = base_dir;
        self
    }

    /// Retain the captured launch area as `file_ref_fallback_dir` diagnostic
    /// metadata.
    ///
    /// References authored by the loop document resolve repository-first, then
    /// source-relative; this directory is never an additional candidate.
    #[must_use]
    pub fn with_file_ref_fallback_dir(mut self, fallback: Option<&'a Path>) -> Self {
        self.file_ref_fallback_dir = fallback;
        self
    }

    /// Reuse the request snapshot for references authored by `source_path`.
    #[must_use]
    pub fn with_file_resolution_context(
        mut self,
        context: Option<&'a biscuit_file::FileResolutionContext>,
        source_path: &'a Path,
    ) -> Self {
        self.file_resolution_context = context;
        self.source_path = Some(source_path);
        self
    }
}

impl EvaluationLookup for LoopExpressionLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        if path == "doc" || path.starts_with("doc.") {
            return resolve_doc(self.frontmatter, path);
        }

        if let Some(env_key) = path.strip_prefix("env.") {
            return resolve_env(env_key);
        }

        if let Some(value) = resolve_ambient(path, self.ambient) {
            return Some(value);
        }

        if let Some(value) = resolve_boolean_literal(path) {
            return Some(value);
        }

        resolve_frontmatter(self.frontmatter, path)
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        self.base_dir.map(|dir| match self.source_path {
            Some(source_path) => super::super::document_expression_resolution_context(
                source_path,
                None,
                self.file_resolution_context,
                self.file_ref_fallback_dir,
            ),
            None => {
                let ctx = ResolutionContext::new(dir.to_path_buf());
                match self.file_ref_fallback_dir {
                    Some(fallback) => ctx.with_file_ref_fallback_dir(fallback.to_path_buf()),
                    None => ctx,
                }
            }
        })
    }
}

/// Evaluate a loop condition and return whether the loop should continue.
///
/// `while` conditions continue when the expression is truthy. `until`
/// conditions continue while the expression is falsy.
///
/// ## Errors
///
/// Returns `LoopExpressionInvalid` when the condition cannot be parsed or
/// evaluated, carrying the typed failure as its `#[source]`.
pub fn evaluate_condition(
    condition: &LoopCondition,
    lookup: &LoopExpressionLookup<'_>,
) -> Result<bool, CompositionError> {
    let (kind, source) = match condition {
        LoopCondition::While(source) => ("while", source),
        LoopCondition::Until(source) => ("until", source),
    };

    let parsed = parse_condition(source).map_err(|error| {
        CompositionError::LoopExpressionInvalid {
            kind: kind.to_string(),
            condition: source.clone(),
            source: LoopExpressionCause::Parse(error),
        }
    })?;
    let value = evaluate(&parsed, lookup).map_err(|error| {
        CompositionError::LoopExpressionInvalid {
            kind: kind.to_string(),
            condition: source.clone(),
            source: LoopExpressionCause::Evaluate(Box::new(error)),
        }
    })?;
    let truthy = is_truthy(&value);

    Ok(match condition {
        LoopCondition::While(_) => truthy,
        LoopCondition::Until(_) => !truthy,
    })
}

/// Resolve the reserved `doc` namespace against the loop frontmatter, which is
/// the document's root object. Bare `doc` yields the whole object; `doc.<path>`
/// traverses it. A `doc.<path>` that does not resolve returns `None` and must
/// not fall back to another namespace.
fn resolve_doc(frontmatter: &Map<String, Value>, path: &str) -> Option<Value> {
    match path.strip_prefix("doc.") {
        Some(rest) => {
            let mut current = Value::Object(frontmatter.clone());
            for segment in rest.split('.') {
                current = match current {
                    Value::Object(map) => map.get(segment).cloned()?,
                    _ => return None,
                };
            }
            Some(current)
        }
        // path == "doc": the whole frontmatter object.
        None => Some(Value::Object(frontmatter.clone())),
    }
}

fn resolve_env(name: &str) -> Option<Value> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    std::env::var(trimmed).ok().map(Value::String)
}

fn resolve_ambient(path: &str, ambient: &LoopAmbient) -> Option<Value> {
    match path {
        "_loop_count" => Some(Value::Number(ambient.iteration.into())),
        "_loop_is_first" => Some(Value::Bool(ambient.is_first)),
        "_loop_is_last" => Some(Value::Bool(ambient.is_last)),
        "_loop_last_output" => Some(Value::String(ambient.last_output.clone())),
        "_loop_last_exit_code" => Some(Value::Number(ambient.last_exit_code.into())),
        _ => None,
    }
}

fn resolve_boolean_literal(path: &str) -> Option<Value> {
    match path {
        "true" => Some(Value::Bool(true)),
        "false" => Some(Value::Bool(false)),
        _ => None,
    }
}

fn resolve_frontmatter(frontmatter: &Map<String, Value>, path: &str) -> Option<Value> {
    if path.trim().is_empty() {
        return None;
    }

    if let Some(value) = frontmatter.get(path) {
        return Some(value.clone());
    }

    let mut parts = path.split('.');
    let head = parts.next()?;
    let mut current = frontmatter.get(head)?.clone();
    for part in parts {
        current = match current {
            Value::Object(map) => map.get(part).cloned()?,
            _ => return None,
        };
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::compose::expression::EvaluationLookup;
    use serde_json::json;

    mod resolution_context;

    fn map(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn ambient() -> LoopAmbient {
        LoopAmbient::new(1, true, false, "", 0)
    }

    #[test]
    fn resolves_ambient_variables() {
        let fm = map(json!({}));
        let ambient = LoopAmbient::new(3, false, true, "done", 7);
        let lookup = LoopExpressionLookup::new(&fm, &ambient);

        assert_eq!(lookup.get("_loop_count"), Some(json!(3)));
        assert_eq!(lookup.get("_loop_is_first"), Some(json!(false)));
        assert_eq!(lookup.get("_loop_is_last"), Some(json!(true)));
        assert_eq!(lookup.get("_loop_last_output"), Some(json!("done")));
        assert_eq!(lookup.get("_loop_last_exit_code"), Some(json!(7)));
    }

    #[test]
    fn ambient_variables_do_not_shadow_user_frontmatter() {
        // Loop ambients live under the `_loop_` namespace, so a user-defined
        // frontmatter property called `iteration` is fully visible and is
        // not silently overwritten by the loop counter.
        let fm = map(json!({"iteration": 99, "is_first": false}));
        let ambient = ambient();
        let lookup = LoopExpressionLookup::new(&fm, &ambient);

        assert_eq!(lookup.get("iteration"), Some(json!(99)));
        assert_eq!(lookup.get("is_first"), Some(json!(false)));
        assert_eq!(lookup.get("_loop_count"), Some(json!(1)));
        assert_eq!(lookup.get("_loop_is_first"), Some(json!(true)));
    }

    #[test]
    fn resolves_top_level_and_nested_frontmatter() {
        let fm = map(json!({
            "counter": 4,
            "stage": "review",
            "state": {"inner": {"done": true}}
        }));
        let ambient = ambient();
        let lookup = LoopExpressionLookup::new(&fm, &ambient);

        assert_eq!(lookup.get("counter"), Some(json!(4)));
        assert_eq!(lookup.get("stage"), Some(json!("review")));
        assert_eq!(lookup.get("state.inner.done"), Some(json!(true)));
    }

    #[test]
    fn resolves_environment_variables() {
        let fm = map(json!({}));
        let ambient = ambient();
        let lookup = LoopExpressionLookup::new(&fm, &ambient);

        assert_eq!(
            lookup.get("env.PATH").is_some(),
            std::env::var("PATH").is_ok()
        );
    }

    #[test]
    fn evaluates_while_condition() {
        let ambient = ambient();
        let fm = map(json!({"counter": 3}));
        let lookup = LoopExpressionLookup::new(&fm, &ambient);
        assert!(evaluate_condition(&LoopCondition::While("counter < 5".into()), &lookup).unwrap());

        let fm = map(json!({"counter": 5}));
        let lookup = LoopExpressionLookup::new(&fm, &ambient);
        assert!(!evaluate_condition(&LoopCondition::While("counter < 5".into()), &lookup).unwrap());
    }

    #[test]
    fn evaluates_until_condition_as_continue_decision() {
        let ambient = ambient();
        let fm = map(json!({"done": false}));
        let lookup = LoopExpressionLookup::new(&fm, &ambient);
        assert!(evaluate_condition(&LoopCondition::Until("done".into()), &lookup).unwrap());

        let fm = map(json!({"done": true}));
        let lookup = LoopExpressionLookup::new(&fm, &ambient);
        assert!(!evaluate_condition(&LoopCondition::Until("done".into()), &lookup).unwrap());
    }

    #[test]
    fn evaluates_ambient_and_env_conditions() {
        let fm = map(json!({"counter": 1}));
        let ambient = ambient();
        let lookup = LoopExpressionLookup::new(&fm, &ambient);

        assert!(
            evaluate_condition(&LoopCondition::While("_loop_count == 1".into()), &lookup).unwrap()
        );
        assert!(
            evaluate_condition(
                &LoopCondition::While("_loop_is_first == true".into()),
                &lookup
            )
            .unwrap()
        );
        assert_eq!(
            evaluate_condition(&LoopCondition::While("env.PATH != \"\"".into()), &lookup).unwrap(),
            std::env::var("PATH").is_ok_and(|value| !value.is_empty())
        );
    }

    #[test]
    fn resolves_doc_namespace_against_frontmatter() {
        let fm = map(json!({
            "build": "from-frontmatter",
            "config": {"retries": 3},
            "doc": {"child": "literal-doc"},
        }));
        let ambient = ambient();
        let lookup = LoopExpressionLookup::new(&fm, &ambient);

        // doc.<path> traverses the frontmatter object.
        assert_eq!(lookup.get("doc.build"), Some(json!("from-frontmatter")));
        assert_eq!(lookup.get("doc.config.retries"), Some(json!(3)));

        // bare doc returns the whole frontmatter object.
        let obj = lookup.get("doc").expect("bare doc resolves");
        assert!(obj.is_object());
        assert_eq!(obj.get("build"), Some(&json!("from-frontmatter")));

        // a literal property named `doc` is reached as doc.doc.
        assert_eq!(lookup.get("doc.doc.child"), Some(json!("literal-doc")));

        // missing doc.* values do not fall back to env/ambient.
        assert_eq!(lookup.get("doc.missing"), None);
    }

    #[test]
    fn read_side_functions_resolve_against_base_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("artifact"), "ready").unwrap();
        let fm = map(json!({}));
        let ambient = ambient();

        // Without a base dir, read-side functions have no resolution context.
        let context_free = LoopExpressionLookup::new(&fm, &ambient);
        assert!(context_free.resolution_context().is_none());

        // With a base dir, `file_exists` resolves against it.
        let lookup = LoopExpressionLookup::new(&fm, &ambient).with_base_dir(Some(dir.path()));
        assert!(lookup.resolution_context().is_some());

        // `until` continues while the expression is falsy. The artifact exists,
        // so `file_exists` is truthy and the loop should stop (continue=false).
        assert!(
            !evaluate_condition(
                &LoopCondition::Until("file_exists('artifact')".into()),
                &lookup
            )
            .unwrap()
        );
        // A missing artifact is falsy, so the `until` loop keeps going.
        assert!(
            evaluate_condition(
                &LoopCondition::Until("file_exists('missing')".into()),
                &lookup
            )
            .unwrap()
        );
    }

    /// The launch-area fallback (`file_ref_fallback_dir`) is diagnostic-only: a
    /// reference authored inside a document resolves against `base_dir`, not the
    /// launch area (D2). A file present ONLY under the launch area does not
    /// resolve even when the fallback is set, while a file under `base_dir`
    /// does — and the fallback is still carried on the resolution context for
    /// diagnostics.
    #[test]
    fn file_ref_fallback_dir_is_diagnostic_only_not_a_resolution_candidate() {
        let prompt_dir = tempfile::TempDir::new().unwrap();
        let launch_dir = tempfile::TempDir::new().unwrap();

        // One artifact under the launch area, one under the document dir.
        std::fs::write(launch_dir.path().join("spec.md"), "# spec\n").unwrap();
        std::fs::write(prompt_dir.path().join("local.md"), "# local\n").unwrap();

        let fm = map(json!({}));
        let ambient = ambient();

        let lookup = LoopExpressionLookup::new(&fm, &ambient)
            .with_base_dir(Some(prompt_dir.path()))
            .with_file_ref_fallback_dir(Some(launch_dir.path()));

        // A file present only under the launch area does NOT resolve, so the
        // `until` condition keeps going (file_exists is falsy).
        assert!(
            evaluate_condition(
                &LoopCondition::Until("file_exists('spec.md')".into()),
                &lookup
            )
            .unwrap(),
            "a launch-area-only file must not resolve; the fallback is diagnostic-only"
        );

        // A file under base_dir resolves as the source-local candidate, so the `until`
        // condition stops (file_exists is truthy).
        assert!(
            !evaluate_condition(
                &LoopCondition::Until("file_exists('local.md')".into()),
                &lookup
            )
            .unwrap(),
            "a file under base_dir resolves; `until` stops"
        );

        // The fallback is still propagated into the resolution context for
        // diagnostics, even though it is not a resolution candidate.
        let ctx = lookup
            .resolution_context()
            .expect("resolution context is present");
        assert_eq!(
            ctx.file_ref_fallback_dir,
            Some(launch_dir.path().to_path_buf())
        );
    }

    /// A condition that cannot be parsed reports the `while`/`until` keyword and
    /// the authored text, and keeps the Darkmatter `ParseError` recoverable
    /// through the chain (spec §L1) rather than flattening it into prose.
    #[test]
    fn parse_errors_carry_the_typed_parse_cause() {
        let fm = map(json!({}));
        let ambient = ambient();
        let lookup = LoopExpressionLookup::new(&fm, &ambient);
        let err = evaluate_condition(&LoopCondition::While("counter <".into()), &lookup)
            .expect_err("condition should fail to parse");

        let CompositionError::LoopExpressionInvalid {
            kind,
            condition,
            source,
        } = &err
        else {
            panic!("expected LoopExpressionInvalid, got {err:?}");
        };
        assert_eq!(kind, "while");
        assert_eq!(condition, "counter <");
        assert!(matches!(source, LoopExpressionCause::Parse(_)));
        assert!(err.to_string().contains("failed to parse loop.while"));

        // `LoopExpressionCause` is `#[error(transparent)]`, so it *replaces* the
        // concrete error rather than adding a hop to it: the stage is recovered
        // by downcasting to the cause enum, not by walking one level deeper.
        let cause = std::error::Error::source(&err).expect("the typed cause is on the chain");
        assert!(cause.downcast_ref::<LoopExpressionCause>().is_some());
    }

    /// The evaluate stage is discriminated from the parse stage by the cause's
    /// arm, and both project through the same variant.
    #[test]
    fn evaluate_errors_carry_the_typed_evaluate_cause() {
        let fm = map(json!({}));
        let ambient = ambient();
        let lookup = LoopExpressionLookup::new(&fm, &ambient);
        let err = evaluate_condition(&LoopCondition::Until("no_such_function()".into()), &lookup)
            .expect_err("condition should fail to evaluate");

        let CompositionError::LoopExpressionInvalid { kind, source, .. } = &err else {
            panic!("expected LoopExpressionInvalid, got {err:?}");
        };
        assert_eq!(kind, "until");
        assert!(matches!(source, LoopExpressionCause::Evaluate(_)));
        assert!(err.to_string().contains("failed to evaluate loop.until"));
    }
}
