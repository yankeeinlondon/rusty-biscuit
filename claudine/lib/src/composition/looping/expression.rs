//! Loop condition expression evaluation.

use std::collections::HashMap;
use std::path::Path;

use darkmatter::markdown::compose::expression::{
    EvaluationLookup, ExpressionError, ResolutionContext, ResolvedBinding, evaluate,
    fm_namespace_path, is_reserved_namespace, is_truthy, parse_condition,
    resolve_fm_namespace_in_object,
};
use serde_json::{Map, Value};

use super::super::error::{CompositionError, LoopExpressionCause};
use super::super::lifecycle::bindings::{removed_loop_grammar, removed_loop_read};
use super::super::types::{AmbientVariable, LoopCondition};

/// The values of the `loop` global for one iteration.
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

    /// The `loop` global's runtime value:
    /// `{count, is_first, is_last, last_output, last_exit_code}`.
    pub fn to_value(&self) -> Value {
        let mut object = Map::new();
        for member in AmbientVariable::ALL {
            object.insert(member.as_str().to_string(), self.member(member));
        }
        Value::Object(object)
    }

    fn member(&self, member: AmbientVariable) -> Value {
        match member {
            AmbientVariable::Iteration => Value::Number(self.iteration.into()),
            AmbientVariable::IsFirst => Value::Bool(self.is_first),
            AmbientVariable::IsLast => Value::Bool(self.is_last),
            AmbientVariable::LastOutput => Value::String(self.last_output.clone()),
            AmbientVariable::LastExitCode => Value::Number(self.last_exit_code.into()),
        }
    }
}

/// Darkmatter expression lookup for loop frontmatter plus the `loop` global.
///
/// Resolution order:
///
/// 1. Reserved `fm` namespace (bare `fm` → the whole frontmatter object;
///    `fm.<path>` → dotted traversal into it). Intercepted first so a missing
///    `fm.<path>` never collapses into a same-named ambient/env/frontmatter key.
///    `fm.loop` is therefore the `loop:` configuration block.
/// 2. Prepared `ctx.NAME` and `env.NAME` from the document epoch, when supplied
/// 3. Ambient `env.NAME` compatibility lookup
/// 4. The `loop` global: bare `loop` is [`LoopAmbient::to_value`], and
///    `loop.<member>` one of its members. A missing member is `null`; it
///    never falls through to frontmatter.
/// 5. Frontmatter properties, including nested object paths via `.`
///
/// The lookup resolves file references through the request's
/// file-resolution context, derived for the document at `source_path`, so
/// read-side expression functions (`file_exists`, `absolute`, `relative`, …)
/// resolve implicit document-authored references against the document
/// directory before the repository fallback. The probe re-runs each iteration
/// while the request context and source remain fixed. `env.NAME` without a
/// prepared context reads the same context's environment.
///
/// [`with_file_ref_fallback_dir`](Self::with_file_ref_fallback_dir) retains the
/// captured launch area as diagnostic metadata only. It is not a candidate for
/// these document-authored references; genuinely launch-relative top-level CLI
/// references are resolved before loop evaluation.
#[derive(Debug, Clone, Copy)]
pub struct LoopExpressionLookup<'a> {
    frontmatter: &'a Map<String, Value>,
    runtime_state: Option<&'a crate::composition::RuntimeState>,
    ambient: &'a LoopAmbient,
    file_ref_fallback_dir: Option<&'a Path>,
    file_resolution_context: &'a biscuit_file::FileResolutionContext,
    source_path: &'a Path,
    prepared_context: Option<&'a darkmatter::markdown::compose::ComposeContext>,
}

impl<'a> LoopExpressionLookup<'a> {
    /// Create a lookup over the current loop state of the document at
    /// `source_path`, resolving its references through `file_resolution_context`.
    pub fn new(
        frontmatter: &'a Map<String, Value>,
        ambient: &'a LoopAmbient,
        file_resolution_context: &'a biscuit_file::FileResolutionContext,
        source_path: &'a Path,
    ) -> Self {
        Self {
            frontmatter,
            runtime_state: None,
            ambient,
            file_ref_fallback_dir: None,
            file_resolution_context,
            source_path,
            prepared_context: None,
        }
    }

    /// Read the invocation's sequence root and committed outputs in conditions.
    pub fn with_runtime_state(mut self, runtime: Option<&'a crate::composition::RuntimeState>) -> Self {
        self.runtime_state = runtime;
        self
    }

    /// Retain the captured launch area as `file_ref_fallback_dir` diagnostic
    /// metadata.
    ///
    /// References authored by the loop document resolve source-relative, then
    /// against the repository; this directory is never an additional candidate.
    #[must_use]
    pub fn with_file_ref_fallback_dir(mut self, fallback: Option<&'a Path>) -> Self {
        self.file_ref_fallback_dir = fallback;
        self
    }

    /// Reuse the active document epoch for prepared `ctx.*` and target-adjusted
    /// `env.*` lookups.
    #[must_use]
    pub fn with_prepared_context(
        mut self,
        context: Option<&'a darkmatter::markdown::compose::ComposeContext>,
    ) -> Self {
        self.prepared_context = context;
        self
    }
}

impl EvaluationLookup for LoopExpressionLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        if fm_namespace_path(path).is_some() {
            return resolve_fm_namespace_in_object(path, self.frontmatter);
        }

        if path == "seq" || path.starts_with("seq.") {
            let value = self.runtime_state.map_or_else(||
                super::super::lifecycle::bindings::SequenceBinding::standalone(Vec::new()).to_value(),
                crate::composition::RuntimeState::sequence_value);
            if path == "seq" { return Some(value); }
            let mut result = &value;
            for member in path[4..].split('.') { result = result.get(member)?; }
            return Some(result.clone());
        }

        // A reserved namespace is never a document property, so a miss inside
        // one stays a miss rather than reading a same-named frontmatter key.
        if let Some(name) = path.strip_prefix("ctx.") {
            return self
                .prepared_context
                .and_then(|context| context.get(name))
                .cloned();
        }

        if let Some(env_key) = path.strip_prefix("env.") {
            if let Some(value) = self
                .prepared_context
                .and_then(|context| env_var(context.env(), env_key))
            {
                return Some(Value::String(value.clone()));
            }
            return resolve_env(self.file_resolution_context, env_key);
        }

        if is_reserved_namespace(path.split('.').next().unwrap_or(path)) {
            return None;
        }

        if let Some(member) = loop_member(path) {
            return member.map_or_else(
                || Some(self.ambient.to_value()),
                |member| self.ambient.to_value().get(member).cloned(),
            );
        }

        if let Some(value) = resolve_boolean_literal(path) {
            return Some(value);
        }

        resolve_frontmatter(self.frontmatter, path)
    }

    fn resolve(&self, path: &str) -> Result<ResolvedBinding, ExpressionError> {
        Ok(ResolvedBinding::classify(path, self.get(path)))
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        // `None` makes a read-side function fail with Darkmatter's
        // "requires a document resolution context" error, so an invalid
        // request context surfaces at the first file function.
        super::super::document_expression_resolution_context(
            self.source_path,
            self.prepared_context,
            self.file_resolution_context,
            self.file_ref_fallback_dir,
        )
        .inspect_err(|error| tracing::warn!(%error, "loop expression context is invalid"))
        .ok()
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
/// evaluated, carrying the typed failure as its `#[source]`, and
/// `RemovedGrammar` when it reads a retired `_loop_*` name.
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
    if let Some(retired) = removed_loop_read(&parsed) {
        return Err(removed_loop_grammar(
            lookup.source_path,
            format!("loop.{kind}"),
            retired,
        ));
    }
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

fn resolve_env(context: &biscuit_file::FileResolutionContext, name: &str) -> Option<Value> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    env_var(context.env(), trimmed).cloned().map(Value::String)
}

/// `name`'s value in `env`. Windows variable names are case-insensitive, so
/// `env.PATH` finds the `Path` Windows reports.
fn env_var<'a>(env: &'a HashMap<String, String>, name: &str) -> Option<&'a String> {
    env.get(name).or_else(|| {
        cfg!(windows)
            .then(|| env.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)))
            .flatten()
            .map(|(_, value)| value)
    })
}

/// `Some(None)` for bare `loop`, `Some(Some(rest))` for `loop.<rest>`, and
/// `None` for any other path.
fn loop_member(path: &str) -> Option<Option<&str>> {
    match path.strip_prefix(AmbientVariable::ROOT)? {
        "" => Some(None),
        rest => rest.strip_prefix('.').map(Some),
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
mod tests;
