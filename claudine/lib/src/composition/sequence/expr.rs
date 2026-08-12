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
//! In the per-item surfaces the item's own top-level fields shadow globals, so
//! `template(color + '-is-great')` reads the item's `color` even when the
//! invoking document defines one too.

use std::path::Path;

use darkmatter::markdown::compose::expression::{
    CtxLookup, EvaluationLookup, ExpressionFinder, ResolutionContext, evaluate, parse,
    scalar_string,
};
use serde_json::{Map, Value};

use super::super::error::{CompositionError, SequenceExpressionCause};

/// Lookup for sequence-source expressions.
///
/// Resolution order: item fields (when evaluating per item) → `env.NAME` →
/// document frontmatter → `ctx.*`. `ctx` is consulted last and captured on
/// demand, so a document property named like a context key keeps winning and no
/// context group is captured unless an expression actually reads one.
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

    /// Shadow the globals with one item's top-level fields.
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
}

impl EvaluationLookup for SourceExpressionLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        if let Some(item) = self.item
            && let Some(value) = resolve_path(item, path)
        {
            return Some(value);
        }

        if let Some(key) = path.strip_prefix("env.") {
            return std::env::var(key).ok().map(Value::String);
        }

        if let Some(value) = resolve_path(self.frontmatter, path) {
            return Some(value);
        }

        self.ctx.resolve_ctx(path)
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
/// Returns [`CompositionError::SequenceExpressionFailed`] with the parse or
/// evaluation message.
pub fn evaluate_whole<L: EvaluationLookup>(
    expression: &str,
    lookup: &L,
) -> Result<Value, CompositionError> {
    let parsed = parse(expression).map_err(|e| CompositionError::SequenceExpressionFailed {
        expression: expression.to_string(),
        source: SequenceExpressionCause::Parse(e),
    })?;
    evaluate(&parsed, lookup).map_err(|e| CompositionError::SequenceExpressionFailed {
        expression: expression.to_string(),
        source: SequenceExpressionCause::Evaluate(Box::new(e)),
    })
}

/// Render a string that may contain `{{ … }}` spans.
///
/// A string that is exactly one span keeps its typed value (so a template can
/// carry a number or a list); anything else is interpolated into text.
///
/// ## Errors
///
/// Returns [`CompositionError::SequenceExpressionFailed`] for the first span
/// that fails to parse or evaluate.
pub fn render_interpolated<L: EvaluationLookup>(
    raw: &str,
    lookup: &L,
) -> Result<Value, CompositionError> {
    let locations = ExpressionFinder::find_all_plain(raw);
    if locations.is_empty() {
        return Ok(Value::String(raw.to_string()));
    }

    let single_span =
        locations.len() == 1 && locations[0].start == 0 && locations[0].end == raw.len();

    let mut output = String::with_capacity(raw.len());
    let mut cursor = 0usize;

    for location in &locations {
        output.push_str(&raw[cursor..location.start]);
        let value = evaluate_whole(location.expression.trim(), lookup)?;
        if single_span {
            return Ok(value);
        }
        output.push_str(&scalar_string(&value));
        cursor = location.end;
    }

    output.push_str(&raw[cursor..]);
    Ok(Value::String(output))
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
            "file_exists('!package.flag')",
        ] {
            assert_eq!(evaluate_whole(expression, &lookup).unwrap(), Value::Bool(true));
        }
    }
}
