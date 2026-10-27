//! Parsed reads of the retired sequence globals, with presence-based escape
//! for ordinary document properties (including an authored or supplied null).
//!
//! The reads are Darkmatter's ([`static_access_reads`]); this module only
//! matches their roots against [`RETIRED_SEQUENCE_NAMES`].

use std::path::Path;

use darkmatter::markdown::compose::expression::{
    Expr, ExpressionFinder, parse, parse_condition, static_access_reads,
};
use serde_json::Value;

use super::{AuthoredDocument, super::LifecycleConfig};
use crate::composition::error::CompositionError;

/// Section 7's retired roots and their replacements. Member-specific `state`
/// replacements are resolved from the parsed path.
pub const RETIRED_SEQUENCE_NAMES: &[(&str, &str)] = &[
    ("state", "seq"), ("previous", "seq.previous"),
    ("previous_state", "seq.previous"), ("next", "seq.next"),
    ("next_state", "seq.next"), ("outputs", "seq.outputs"),
    ("sequence_id", "seq.run_id"), ("step", "seq.index"),
    ("total_steps", "seq.count"), ("is_first", "seq.is_first"),
    ("is_last", "seq.is_last"),
];

/// Which [`RETIRED_SEQUENCE_NAMES`] `values` declares, one bit per name in
/// that order: the only part of a `supplied` overlay the retired-read checks
/// read, so a check that passed under one set passes under it again.
pub fn retired_names_present(values: &serde_json::Map<String, Value>) -> u16 {
    RETIRED_SEQUENCE_NAMES.iter().enumerate()
        .filter(|(_, (name, _))| values.contains_key(*name))
        .fold(0, |present, (bit, _)| present | (1 << bit))
}

/// The first retired sequence read of `expr` whose root `declared` does not
/// claim, as `(removed, replacement)`, in source order.
///
/// `removed` is the read's static path
/// ([`static_path`](darkmatter::markdown::compose::expression::StaticAccessRead::static_path)):
/// a string key extends it like a member, which matters for
/// `state['sequence_id']`, where the member changes the replacement; a
/// numeric or computed index ends it.
fn read(expr: &Expr, declared: &impl Fn(&str) -> bool) -> Option<(String, String)> {
    static_access_reads(expr).iter().find_map(|access| {
        let path = access.static_path();
        let (root, suffix) = path.split_once('.').unwrap_or((&path, ""));
        if declared(root) { return None; }
        let (_, replacement) = RETIRED_SEQUENCE_NAMES.iter().find(|(old, _)| *old == root)?;
        let replacement = if root == "state" && suffix == "sequence_id" {
            "seq.run_id".to_string()
        } else if suffix.is_empty() { replacement.to_string() }
        else { format!("{replacement}.{suffix}") };
        Some((path.clone(), replacement))
    })
}

fn reject(source: &Path, path: String, found: Option<(String, String)>) -> Result<(), CompositionError> {
    if let Some((removed, replacement)) = found {
        return Err(CompositionError::RemovedGrammar {
            source_path: source.to_path_buf(), path, removed,
            replacement: format!("use `{replacement}`"),
        });
    }
    Ok(())
}

/// Reject retired roots in `lifecycle`'s events alone: the part of
/// [`reject_removed_sequence_reads`] that reads `frontmatter` only for which
/// names it declares, so a caller that has already checked the document's
/// frontmatter does not scan it again.
///
/// # Errors
/// Returns `RemovedGrammar` with the authored surface and replacement.
pub fn reject_removed_lifecycle_reads(
    source: &Path,
    frontmatter: &Value,
    supplied: Option<&Value>,
    lifecycle: &LifecycleConfig,
) -> Result<(), CompositionError> {
    let declared = |key: &str| frontmatter.get(key).is_some()
        || supplied.is_some_and(|values| values.get(key).is_some());
    let text_read = |text: &str| ExpressionFinder::find_all_plain(text).iter()
        .find_map(|span| parse(span.expression.trim()).ok().and_then(|expr| read(&expr, &declared)));
    for signal in super::super::LifecycleSignal::ALL {
        let mut found = None;
        darkmatter::lifecycle::walk_stack(lifecycle.stack(signal), &mut |path, surface| {
            if found.is_none() {
                let retired = match surface.expr {
                    Expr::StringLiteral(text) if !surface.predicate => text_read(text),
                    expr => read(expr, &declared),
                };
                found = retired.map(|retired| (path.to_string(), retired));
            }
        });
        if let Some((path, found)) = found { reject(source, path, Some(found))?; }
    }
    Ok(())
}

/// Reject retired roots on all authored expression surfaces before effects.
/// `supplied` is the reading document's caller/property overlay, not generated
/// sequence state. Presence, rather than truthiness, preserves authored data.
///
/// # Errors
/// Returns `RemovedGrammar` with the authored surface and replacement.
pub fn reject_removed_sequence_reads(
    source: &Path,
    document: AuthoredDocument<'_>,
    supplied: Option<&Value>,
    lifecycle: Option<&LifecycleConfig>,
) -> Result<(), CompositionError> {
    let declared = |key: &str| document.frontmatter.get(key).is_some()
        || supplied.is_some_and(|values| values.get(key).is_some());
    let text_read = |text: &str| ExpressionFinder::find_all_plain(text).iter()
        .find_map(|span| parse(span.expression.trim()).ok().and_then(|expr| read(&expr, &declared)));
    fn strings(
        source: &Path, value: &Value, path: String,
        text_read: &impl Fn(&str) -> Option<(String, String)>,
    ) -> Result<(), CompositionError> {
        match value {
            Value::String(text) => reject(source, path, text_read(text)),
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() { strings(source, item, format!("{path}[{i}]"), text_read)?; }
                Ok(())
            }
            Value::Object(map) => {
                for (key, item) in map { strings(source, item, format!("{path}.{key}"), text_read)?; }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    strings(source, document.frontmatter, "frontmatter".into(), &text_read)?;
    for condition in ["while", "until"] {
        if let Some(text) = document.frontmatter.get("loop").and_then(|v| v.get(condition)).and_then(Value::as_str) {
            reject(source, format!("loop.{condition}"), parse_condition(text).ok().and_then(|expr| read(&expr, &declared)))?;
        }
    }
    if let Some(lifecycle) = lifecycle {
        reject_removed_lifecycle_reads(source, document.frontmatter, supplied, lifecycle)?;
    }
    if document.body.contains("::") {
        let context = biscuit_terminal::errors::SourceContext::new(
            source.to_path_buf(), source.to_path_buf(), document.body.to_string());
        let condition_read = |text: &str| parse_condition(text).ok().and_then(|expr| read(&expr, &declared));
        if let Ok(directives) = darkmatter::markdown::compose::transclusion::parse_directives(document.body, context.clone()) {
            for directive in directives {
                if let Some(condition) = directive.options.when_expr {
                    reject(source, format!("body:{}.when", directive.line + document.body_line_offset), condition_read(&condition))?;
                }
            }
        }
        fn page_conditions<'a>(
            regions: &'a [darkmatter::markdown::compose::page_blocks::PageBlockRegion],
            conditions: &mut Vec<(usize, &'a str)>,
        ) {
            for region in regions {
                if let Some(condition) = &region.options.when_expr { conditions.push((region.start_line, condition)); }
                page_conditions(&region.children, conditions);
            }
        }
        if let Ok(regions) = darkmatter::markdown::compose::page_blocks::parser::parse_page_blocks(document.body, context) {
            let mut conditions = Vec::new();
            page_conditions(&regions, &mut conditions);
            for (line, condition) in conditions {
                reject(source, format!("body:{}.when", line + document.body_line_offset), condition_read(condition))?;
            }
        }
    }
    let spans = if document.frontmatter.get("interpolate_code_blocks").and_then(Value::as_bool) == Some(true) {
        ExpressionFinder::find_all_plain(document.body)
    } else { ExpressionFinder::new(document.body).find_all() };
    for span in spans {
        let found = parse(span.expression.trim()).ok().and_then(|expr| read(&expr, &declared));
        let line = document.body[..span.start].matches('\n').count() + 1 + document.body_line_offset;
        reject(source, format!("body:{line}"), found)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
