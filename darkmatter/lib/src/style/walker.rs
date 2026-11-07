//! Pass 1 of the style parser: walk the raw `serde_json::Value` map of the
//! `style:` value, canonicalizing every segment and comparing the canonical
//! path against the schema descriptor.
//!
//! Per-segment canonicalization means a document like
//! `style.block_quote.max_width` emits two `Deprecated` warnings (one per
//! snake-cased segment) rather than a single container warning plus an
//! `UnknownKey` for the child. The same property holds for the nested
//! `hyperlinks.local_style.*` and `images.local_style.*` paths.
//!
//! An unknown key carries a suggestion when its intended key is known: a
//! removed page spacing name (`page.left-margin` → `page.margin-left`) or a
//! snake-case spelling of a key that has no snake-case alias.

use crate::style::descriptor::{
    is_canonical_container, join_path, kebabify, leaf_for_key, suggested_key,
};
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
        if let Some(leaf) = leaf_for_key(&canonical_child, key) {
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
        let warning = StyleWarning::new(format!("style.{}", raw_child), StyleWarningKind::UnknownKey);
        warnings.push(match suggested_key(&canonical_child) {
            Some(key) => warning.with_suggestion(key),
            None => warning,
        });
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
        let w = warnings_for(json!({"page": {"max_width": "80ch"}}));
        assert_eq!(w, vec![deprecated("style.page.max_width", "page.max-width")]);
    }

    #[test]
    fn removed_page_spacing_names_suggest_css_order() {
        let w = warnings_for(json!({"page": {"left-margin": "2ch", "top_padding": 1}}));
        assert_eq!(
            w,
            vec![
                unknown("style.page.left-margin").with_suggestion("page.margin-left"),
                unknown("style.page.top_padding").with_suggestion("page.padding-top"),
            ]
        );
    }

    #[test]
    fn removed_page_spacing_name_at_top_level_suggests_top_level_key() {
        let w = warnings_for(json!({"right-margin": "2ch"}));
        assert_eq!(w, vec![unknown("style.right-margin").with_suggestion("margin-right")]);
    }

    #[test]
    fn snake_spelling_of_key_without_alias_is_unknown_with_suggestion() {
        let w = warnings_for(json!({"page": {"margin_left": "2ch"}, "code_block": {"line_numbers": true}}));
        assert_eq!(
            w,
            vec![
                unknown("style.page.margin_left").with_suggestion("page.margin-left"),
                deprecated("style.code_block", "code-block"),
                unknown("style.code_block.line_numbers").with_suggestion("code-block.line-numbers"),
            ]
        );
    }

    #[test]
    fn page_keys_and_shorthands_are_known_at_top_level_and_under_page() {
        let w = warnings_for(json!({
            "margin": 1, "mx": "2ch", "pl": 3, "max-width": "80ch", "theme": "github",
            "code": {"theme": "nord", "mode": "dark"},
            "page": {"padding": 1, "py": 2, "mr": "1ch", "theme": "github", "code": {"mode": "same"}}
        }));
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn removed_hr_keys_are_unknown() {
        let w = warnings_for(json!({"hr": {"margin": 1, "border": true, "emphasis": {}, "word-wrap": "wrap", "padding-left": 2}}));
        let paths: Vec<&str> = w.iter().map(|w| w.path.as_str()).collect();
        assert_eq!(paths, ["style.hr.margin", "style.hr.border", "style.hr.emphasis", "style.hr.word-wrap"]);
        assert!(w.iter().all(|w| w.kind == StyleWarningKind::UnknownKey && w.suggestion.is_none()));
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
