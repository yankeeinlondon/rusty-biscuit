//! Pass 1 of the style parser: walk the raw `serde_json::Value` map of the
//! `style:` value, canonicalizing every segment and comparing the canonical
//! path against the schema descriptor.
//!
//! Per-segment canonicalization means a document like
//! `style.block_quote.max_width` emits two `Deprecated` warnings (one per
//! snake-cased segment) rather than a single container warning plus an
//! `UnknownKey` for the child. The same property holds for the nested
//! `hyperlinks.local_style.*` and `images.local_style.*` paths.

use crate::style::descriptor::{is_canonical_container, join_path, kebabify, leaf_for_canonical};
use crate::style::warning::{StyleWarning, StyleWarningKind};

/// Walk the raw style value and collect schema-validation warnings.
///
/// All warning paths start with `style.`. The path reflects what the user
/// wrote (so the warning is greppable in the source); the `replacement`
/// field on `Deprecated` carries the canonical kebab-case form.
pub fn walk(value: &serde_json::Value) -> Vec<StyleWarning> {
    let mut warnings = Vec::new();
    walk_inner(value, "", "", &mut warnings);
    warnings
}

fn walk_inner(
    value: &serde_json::Value,
    raw_path: &str,
    canonical_path: &str,
    warnings: &mut Vec<StyleWarning>,
) {
    let serde_json::Value::Object(map) = value else {
        return; // primitives are validated by the pre-validator, not here.
    };

    for (key, child) in map {
        let canonical_segment = kebabify(key);
        let was_alias = key != &canonical_segment;

        let raw_child = join_path(raw_path, key);
        let canonical_child = join_path(canonical_path, &canonical_segment);

        // Leaf?
        if let Some(leaf) = leaf_for_canonical(&canonical_child) {
            if was_alias {
                warnings.push(StyleWarning::new(
                    format!("style.{}", raw_child),
                    StyleWarningKind::Deprecated {
                        replacement: leaf.canonical.to_string(),
                    },
                ));
            }
            // Leaves do not recurse — the typed validator handles their value.
            continue;
        }

        // Container?
        if is_canonical_container(&canonical_child) {
            if was_alias {
                warnings.push(StyleWarning::new(
                    format!("style.{}", raw_child),
                    StyleWarningKind::Deprecated {
                        replacement: canonical_child.clone(),
                    },
                ));
            }
            walk_inner(child, &raw_child, &canonical_child, warnings);
            continue;
        }

        // Truly unknown — reported at the raw path the user wrote.
        warnings.push(StyleWarning::new(
            format!("style.{}", raw_child),
            StyleWarningKind::UnknownKey,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn warnings_for(value: serde_json::Value) -> Vec<StyleWarning> {
        walk(&value)
    }

    fn deprecated(path: &str, replacement: &str) -> StyleWarning {
        StyleWarning::new(
            path,
            StyleWarningKind::Deprecated {
                replacement: replacement.to_string(),
            },
        )
    }

    fn unknown(path: &str) -> StyleWarning {
        StyleWarning::new(path, StyleWarningKind::UnknownKey)
    }

    #[test]
    fn empty_object_no_warnings() {
        assert!(warnings_for(json!({})).is_empty());
    }

    #[test]
    fn unknown_top_level_bucket() {
        let w = warnings_for(json!({"planet": {"x": 1}}));
        assert_eq!(w, vec![unknown("style.planet")]);
    }

    #[test]
    fn unknown_leaf_under_known_bucket() {
        let w = warnings_for(json!({"page": {"lft-margin": "2ch"}}));
        assert_eq!(w, vec![unknown("style.page.lft-margin")]);
    }

    #[test]
    fn snake_case_leaf_is_deprecated() {
        let w = warnings_for(json!({"page": {"left_margin": "2ch"}}));
        assert_eq!(
            w,
            vec![deprecated("style.page.left_margin", "page.left-margin")]
        );
    }

    #[test]
    fn block_quote_snake_container_is_deprecated() {
        let w = warnings_for(json!({"block_quote": {"max-width": "50%"}}));
        // Only the container is deprecated — `max-width` is canonical.
        assert_eq!(w, vec![deprecated("style.block_quote", "block-quote")]);
    }

    #[test]
    fn block_quote_max_width_emits_two_deprecated() {
        // Regression for `review-1.md` Finding 1: a deprecated container
        // plus a deprecated child must emit *two* `Deprecated` warnings,
        // not one `Deprecated` + one `UnknownKey`.
        let w = warnings_for(json!({"block_quote": {"max_width": "50%"}}));
        assert_eq!(
            w,
            vec![
                deprecated("style.block_quote", "block-quote"),
                deprecated("style.block_quote.max_width", "block-quote.max-width",),
            ]
        );
    }

    #[test]
    fn hyperlinks_local_style_max_width_emits_two_deprecated() {
        let w = warnings_for(json!({
            "hyperlinks": {"local_style": {"max_width": "50%"}}
        }));
        assert_eq!(
            w,
            vec![
                deprecated("style.hyperlinks.local_style", "hyperlinks.local-style",),
                deprecated(
                    "style.hyperlinks.local_style.max_width",
                    "hyperlinks.local-style.max-width",
                ),
            ]
        );
    }

    #[test]
    fn images_local_style_max_width_emits_two_deprecated() {
        let w = warnings_for(json!({
            "images": {"local_style": {"max_width": "50%"}}
        }));
        assert_eq!(
            w,
            vec![
                deprecated("style.images.local_style", "images.local-style"),
                deprecated(
                    "style.images.local_style.max_width",
                    "images.local-style.max-width",
                ),
            ]
        );
    }

    #[test]
    fn flatten_typo_detected() {
        // `table` is structurally just `CommonStyle` — typos inside it must
        // be detected.
        let w = warnings_for(json!({"table": {"maxx-width": "50%"}}));
        assert_eq!(w, vec![unknown("style.table.maxx-width")]);
    }

    #[test]
    fn nested_local_style_typo_detected() {
        let w = warnings_for(json!({
            "hyperlinks": {"local-style": {"maxx-width": "50%"}}
        }));
        assert_eq!(w, vec![unknown("style.hyperlinks.local-style.maxx-width")]);
    }

    #[test]
    fn multiple_unknown_keys_emit_distinct_warnings() {
        let w = warnings_for(json!({
            "page":  {"lft-margin": "2ch"},
            "table": {"maxx-width": "50%"},
            "ul":    {"left-mrgin": "4ch"}
        }));
        assert_eq!(w.len(), 3);
        let paths: Vec<&str> = w.iter().map(|w| w.path.as_str()).collect();
        assert!(paths.contains(&"style.page.lft-margin"));
        assert!(paths.contains(&"style.table.maxx-width"));
        assert!(paths.contains(&"style.ul.left-mrgin"));
    }

    #[test]
    fn primitive_at_root_emits_no_warning() {
        // Defensive: a string at the root shouldn't panic — pre-validate
        // would catch the structural error first, but the walker must be
        // robust on its own.
        assert!(warnings_for(json!("oops")).is_empty());
        assert!(warnings_for(json!(42)).is_empty());
        assert!(warnings_for(json!(null)).is_empty());
    }
}
