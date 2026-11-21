//! Whole-value frontmatter properties the slot worker observes, and the
//! bounded preview hover shows of an observed value.
//!
//! A top-level property whose whole value is one `{{ … }}` expression that
//! calls `load_data` without a literal contract is *eligible*: its type is
//! known only from the data it loads, so the slot worker evaluates it in the
//! background under the editor's read policy
//! ([`SlotSession::observe_properties`](darkmatter::markdown::schemas::slots::SlotSession::observe_properties))
//! and judges the value against the destination schema. Every other
//! whole-value property is passed too, so an eligible one can read it. See
//! `docs/topics/schemas/schema-slots.md` ("In the editor").

use darkmatter::markdown::schemas::slots::observe::{PropertySite, observes_property};
use serde_json::Value;
use simplified_schema::span::SourceSpan;

use super::expressions::{frontmatter_interpolations, parse};
use super::frontmatter::{FmPathSegment, FrontmatterAst};

/// One whole-value top-level property and where its expression is written.
#[derive(Debug, Clone)]
pub(crate) struct LocatedSite {
    pub site: PropertySite,
    /// The authored `{{ … }}` range.
    pub span: SourceSpan,
}

/// Every whole-value top-level `{{ … }}` property of `text`, in document
/// order, with eligibility marked. Passive: a scan and a parse.
pub(crate) fn located_sites(text: &str, ast: &FrontmatterAst) -> Vec<LocatedSite> {
    frontmatter_interpolations(text, ast)
        .into_iter()
        .filter(|interpolation| interpolation.whole_value)
        .filter_map(|interpolation| {
            let path = ast.path_at(interpolation.index);
            let [FmPathSegment::Key(key)] = path.as_slice() else {
                return None;
            };
            let eligible = parse(&interpolation.text).is_ok_and(|expr| observes_property(&expr));
            Some(LocatedSite {
                site: PropertySite { key: (*key).to_string(), expression: interpolation.text.clone(), eligible },
                span: interpolation.outer_span(),
            })
        })
        .collect()
}

/// [`located_sites`] of `text`, parsing its frontmatter; empty when none is
/// eligible (nothing then needs observing).
pub(crate) fn sites_to_observe(text: &str) -> Vec<PropertySite> {
    let Some(ast) = FrontmatterAst::parse(text).and_then(|parse| parse.ast) else {
        return Vec::new();
    };
    let sites: Vec<PropertySite> = located_sites(text, &ast).into_iter().map(|located| located.site).collect();
    if sites.iter().any(|site| site.eligible) { sites } else { Vec::new() }
}

/// How many entries of one container a preview shows.
pub(crate) const PREVIEW_ENTRIES: usize = 8;
/// How deep a preview follows containers.
pub(crate) const PREVIEW_DEPTH: usize = 3;
/// How many characters of one string a preview shows.
pub(crate) const PREVIEW_STRING_CHARS: usize = 120;

/// A bounded, JSON-like preview of `value`: at most [`PREVIEW_ENTRIES`]
/// entries per container (with the count left out), [`PREVIEW_DEPTH`]
/// levels, and [`PREVIEW_STRING_CHARS`] characters per string. Built
/// directly from the bounded parts, never by formatting the whole value.
pub(crate) fn preview(value: &Value) -> String {
    let mut out = String::new();
    write_preview(value, 0, &mut out);
    out
}

fn write_preview(value: &Value, depth: usize, out: &mut String) {
    match value {
        Value::String(text) => {
            let mut chars = text.chars();
            let kept: String = chars.by_ref().take(PREVIEW_STRING_CHARS).collect();
            let rest = chars.count();
            out.push_str(&serde_json::to_string(&kept).unwrap_or_default());
            if rest > 0 {
                out.push_str(&format!("…(+{rest} chars)"));
            }
        }
        Value::Array(items) => {
            if depth >= PREVIEW_DEPTH {
                out.push_str(&format!("[…{} items]", items.len()));
                return;
            }
            out.push('[');
            for (index, item) in items.iter().take(PREVIEW_ENTRIES).enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                write_preview(item, depth + 1, out);
            }
            if items.len() > PREVIEW_ENTRIES {
                out.push_str(&format!(", …{} more", items.len() - PREVIEW_ENTRIES));
            }
            out.push(']');
        }
        Value::Object(members) => {
            if depth >= PREVIEW_DEPTH {
                out.push_str(&format!("{{…{} fields}}", members.len()));
                return;
            }
            out.push('{');
            for (index, (key, member)) in members.iter().take(PREVIEW_ENTRIES).enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                out.push_str(&serde_json::to_string(key).unwrap_or_default());
                out.push_str(": ");
                write_preview(member, depth + 1, out);
            }
            if members.len() > PREVIEW_ENTRIES {
                out.push_str(&format!(", …{} more", members.len() - PREVIEW_ENTRIES));
            }
            out.push('}');
        }
        scalar => out.push_str(&scalar.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_top_level_uncontracted_load_is_eligible() {
        let text = "---\ndirect: \"{{ load_data('d.yaml') }}\"\ntyped: \"{{ load_data('d.yaml', 'string[]') }}\"\nnested:\n  inner: \"{{ load_data('d.yaml') }}\"\nembedded: \"x {{ load_data('d.yaml') }}\"\n---\n";
        let ast = FrontmatterAst::parse(text).and_then(|parse| parse.ast).unwrap();
        let sites = located_sites(text, &ast);
        let found: Vec<(&str, bool)> = sites.iter().map(|located| (located.site.key.as_str(), located.site.eligible)).collect();
        assert_eq!(found, [("direct", true), ("typed", false)]);
        assert_eq!(&text[sites[0].span.clone()], "{{ load_data('d.yaml') }}");
        assert_eq!(sites_to_observe(text).len(), 2);
        assert!(sites_to_observe("---\ntyped: \"{{ load_data('d.yaml', 'string[]') }}\"\n---\n").is_empty());
    }

    #[test]
    fn the_preview_is_bounded_by_entries_depth_and_string_length() {
        let many: Vec<Value> = (0..20).map(Value::from).collect();
        let shown = preview(&Value::Array(many));
        assert_eq!(shown, "[0, 1, 2, 3, 4, 5, 6, 7, …12 more]");
        let deep = serde_json::json!({ "a": { "b": { "c": { "d": 1 } } } });
        assert_eq!(preview(&deep), "{\"a\": {\"b\": {\"c\": {…1 fields}}}}");
        let long = Value::String("x".repeat(130));
        let shown = preview(&long);
        assert!(shown.ends_with("…(+10 chars)"), "{shown}");
        assert_eq!(shown.matches('x').count(), PREVIEW_STRING_CHARS);
        let rows: Vec<Value> = (0..3).map(|index| serde_json::json!({ "name": format!("n{index}") })).collect();
        assert_eq!(preview(&Value::Array(rows)), "[{\"name\": \"n0\"}, {\"name\": \"n1\"}, {\"name\": \"n2\"}]");
    }
}
