//! Expression evaluation for dynamic sequence sources.
//!
//! Three surfaces in the source layer evaluate Darkmatter expressions, and all
//! three go through this module so they share one lookup contract:
//!
//! - a whole-value `{{ … }}` sequence source, resolved against the invoking
//!   document's frontmatter plus `ctx.*`/`env.*`;
//! - a `::template(expr)` operator, resolved per item;
//! - a formal sequence document's `template:` values, also per item.
//!
//! In the per-item surfaces the item's own top-level fields are the document
//! layer over the invoking document's frontmatter, so
//! `template(color + '-is-great')` reads the item's `color` even when the
//! invoking document defines one too. Reserved namespaces resolve first, so an
//! item field can never stand in for `ctx`, `env`, or `doc`.

use std::path::Path;
use std::sync::Arc;

use darkmatter::markdown::compose::expression::{
    AuthoredMode, BindingView, CtxLookup, EvaluationLookup, EvaluationSession, ExpressionError,
    ParseMode, ResolutionContext, ResolvedBinding, ScopeId, evaluate_prepared,
    is_reserved_namespace, prepare_value,
};
use darkmatter::markdown::MarkdownError;
use serde_json::{Map, Value};

use super::super::error::{CompositionError, SequenceExpressionCause};

/// Lookup for sequence-source expressions.
///
/// Resolution order: the reserved namespaces (`ctx.*` captured on demand,
/// `env.NAME`, `doc.*`), then the document layer — item fields (when evaluating
/// per item) over the invoking document's frontmatter. A bare name never reads
/// `ctx`, and no context group is captured unless an expression reads one.
/// `current` and `current_env` have no event here and read as `null`.
pub struct SourceExpressionLookup<'a> {
    item: Option<&'a Map<String, Value>>,
    frontmatter: &'a Map<String, Value>,
    ctx: CtxLookup<'a>,
    base_dir: &'a Path,
    file_resolution_context: Option<&'a biscuit_file::FileResolutionContext>,
    source_path: Option<&'a Path>,
}

impl<'a> SourceExpressionLookup<'a> {
    /// Build a lookup rooted at `base_dir` — the directory of the document that
    /// authored the expression, so read-side functions such as `file_exists`
    /// resolve document-relative rather than against the process CWD.
    pub fn new(frontmatter: &'a Map<String, Value>, base_dir: &'a Path) -> Self {
        Self {
            item: None,
            frontmatter,
            ctx: CtxLookup::new(base_dir),
            base_dir,
            file_resolution_context: None,
            source_path: None,
        }
    }

    /// Lay one item's top-level fields over the document's frontmatter.
    #[must_use]
    pub fn with_item(mut self, item: &'a Map<String, Value>) -> Self {
        self.item = Some(item);
        self
    }

    /// Reuse the request snapshot for expressions authored by `source_path`.
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

    /// A document-layer path: the item's field when it has one, else the
    /// invoking document's.
    fn document(&self, path: &str) -> Option<Value> {
        self.item
            .and_then(|item| resolve_path(item, path))
            .or_else(|| resolve_path(self.frontmatter, path))
    }

    /// The whole document layer, for a bare `doc`.
    fn document_object(&self) -> Value {
        let mut merged = self.frontmatter.clone();
        if let Some(item) = self.item {
            merged.extend(item.iter().map(|(key, value)| (key.clone(), value.clone())));
        }
        Value::Object(merged)
    }
}

impl EvaluationLookup for SourceExpressionLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        let root = path.split('.').next().unwrap_or(path);
        match root {
            "ctx" => self.ctx.resolve_ctx(path),
            "env" => path
                .strip_prefix("env.")
                .and_then(|key| std::env::var(key).ok().map(Value::String)),
            "doc" => match path.strip_prefix("doc.") {
                Some(rest) => self.document(rest),
                None => Some(self.document_object()),
            },
            root if is_reserved_namespace(root) => None,
            _ => self.document(path),
        }
    }

    fn resolve(&self, path: &str) -> Result<ResolvedBinding, ExpressionError> {
        Ok(ResolvedBinding::classify(path, self.get(path)))
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        Some(match self.source_path {
            Some(source_path) => super::super::document_expression_resolution_context(
                source_path,
                None,
                self.file_resolution_context,
                None,
            ),
            None => ResolutionContext::new(self.base_dir.to_path_buf()),
        })
    }
}

/// Walk a dotted path into a map.
fn resolve_path(map: &Map<String, Value>, path: &str) -> Option<Value> {
    let mut segments = path.split('.');
    let mut current = map.get(segments.next()?)?;
    for segment in segments {
        current = current.get(segment)?;
    }
    Some(current.clone())
}

/// Evaluate one whole expression, preserving its typed result.
///
/// ## Errors
///
/// Returns [`CompositionError::SequenceExpressionFailed`] with the typed parse
/// or evaluation cause.
pub fn evaluate_whole<L: EvaluationLookup>(
    expression: &str,
    lookup: &L,
) -> Result<Value, CompositionError> {
    evaluate_authored(
        expression,
        AuthoredMode::Expression(ParseMode::Interpolation),
        lookup,
    )
}

/// Render a string that may contain `{{ … }}` spans.
///
/// A string that is exactly one span keeps its typed value (so a template can
/// carry a number or a list); anything else is interpolated into text,
/// following composition's whole-value, mixed-string, and escape rules.
///
/// ## Errors
///
/// Returns [`CompositionError::SequenceExpressionFailed`] for the first span
/// that fails to parse or evaluate.
pub fn render_interpolated<L: EvaluationLookup>(
    raw: &str,
    lookup: &L,
) -> Result<Value, CompositionError> {
    evaluate_authored(raw, AuthoredMode::InterpolatedValue, lookup)
}

/// The sequence source layer declares no globals.
fn source_view() -> Arc<BindingView> {
    Arc::new(
        BindingView::builder(ScopeId::new("claudine.sequence-source"))
            .build()
            .expect("an empty view is valid"),
    )
}

fn evaluate_authored<L: EvaluationLookup>(
    text: &str,
    mode: AuthoredMode,
    lookup: &L,
) -> Result<Value, CompositionError> {
    let failed = |source| CompositionError::SequenceExpressionFailed {
        expression: text.to_string(),
        source,
    };
    let prepared = prepare_value(&Value::String(text.to_string()), mode)
        .map_err(|error| failed(SequenceExpressionCause::Prepare(error)))?;
    let session = EvaluationSession::associate(source_view(), lookup, [])
        .map_err(|error| failed(SequenceExpressionCause::Evaluate(Box::new(ExpressionError::Binding(Box::new(error))))))?;
    evaluate_prepared(&prepared, &session).map_err(|error| {
        failed(match error {
            MarkdownError::Interpolation { cause, .. } => SequenceExpressionCause::Evaluate(cause),
            other => SequenceExpressionCause::Compose(Box::new(other)),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_expression_lookup_reuses_request_resolution_inputs() {
        let request = tempfile::tempdir().unwrap();
        let source_path = request.path().join("sequence.md");
        let home = request.path().join("home");
        let magic = request.path().join("magic");
        let package = request.path().join("package");
        for dir in [&home, &magic, &package] {
            std::fs::create_dir_all(dir).unwrap();
        }
        for path in [
            request.path().join("env.flag"),
            home.join("home.flag"),
            magic.join("magic.flag"),
            package.join("package.flag"),
        ] {
            std::fs::write(path, "ready").unwrap();
        }
        let mut env = std::collections::HashMap::new();
        env.insert(
            "CLAUDINE_SEQUENCE_ROOT".to_string(),
            request.path().display().to_string(),
        );
        let snapshot = biscuit_file::FileResolutionContext::from_snapshot(
            request.path(),
            Some(home),
            env,
        )
        .with_repository_root(request.path())
        .with_package_area(package)
        .add_magic_path(magic, biscuit_file::PathPosition::Start);
        let frontmatter = Map::new();
        let lookup = SourceExpressionLookup::new(&frontmatter, request.path())
            .with_file_resolution_context(Some(&snapshot), &source_path);

        for expression in [
            "file_exists('{{CLAUDINE_SEQUENCE_ROOT}}/env.flag')",
            "file_exists('~/home.flag')",
            "file_exists('@magic.flag')",
            "file_exists('^package.flag')",
        ] {
            assert_eq!(evaluate_whole(expression, &lookup).unwrap(), Value::Bool(true));
        }
    }
}
