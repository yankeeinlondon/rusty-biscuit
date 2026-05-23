//! Pass 1 of the style parser: walk the raw `serde_json::Value` map of the
//! `style:` value, comparing every leaf path against the schema descriptor.
//!
//! Emits `UnknownKey` for paths the descriptor doesn't know, and
//! `Deprecated` for paths that matched a snake-case alias of a canonical
//! kebab-case key.

use crate::style::descriptor::canonicalize;
use crate::style::warning::{StyleWarning, StyleWarningKind};

/// Walk the raw style value and collect schema-validation warnings.
///
/// The path produced for each warning starts with `style.` so it's directly
/// usable by callers / users.
pub fn walk(value: &serde_json::Value) -> Vec<StyleWarning> {
    let mut warnings = Vec::new();
    walk_inner(value, "", &mut warnings);
    warnings
}

fn walk_inner(value: &serde_json::Value, path: &str, warnings: &mut Vec<StyleWarning>) {
    let serde_json::Value::Object(map) = value else {
        return; // leaves are checked at the parent level (by name).
    };

    for (key, child) in map {
        let child_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", path, key)
        };

        // If the path is a leaf in the schema, check for alias use.
        if let Some(leaf) = canonicalize(&child_path) {
            if leaf.alias == Some(child_path.as_str()) {
                warnings.push(StyleWarning::new(
                    format!("style.{}", child_path),
                    StyleWarningKind::Deprecated {
                        replacement: leaf.canonical.to_string(),
                    },
                ));
            }
            // Leaves do not need to recurse further into their value — the
            // typed deserializer will validate them.
            continue;
        }

        // Not a leaf — might be a container path (e.g. "page", "block-quote",
        // "block_quote"). Container "block_quote" is also a deprecated alias
        // because the canonical key is "block-quote"; treat the container's
        // own segment as if it were a leaf path of length 1.
        if is_known_container(&child_path) {
            walk_inner(child, &child_path, warnings);
            continue;
        }
        if let Some(canonical) = deprecated_container(&child_path) {
            warnings.push(StyleWarning::new(
                format!("style.{}", child_path),
                StyleWarningKind::Deprecated {
                    replacement: canonical.to_string(),
                },
            ));
            walk_inner(child, canonical, warnings);
            continue;
        }

        // Truly unknown.
        warnings.push(StyleWarning::new(
            format!("style.{}", child_path),
            StyleWarningKind::UnknownKey,
        ));
    }
}

/// Known canonical container paths (top-level buckets plus the nested
/// `local-style` paths). Anything else either matches a leaf in the
/// descriptor or is unknown.
fn is_known_container(path: &str) -> bool {
    matches!(
        path,
        "page"
            | "table"
            | "block-quote"
            | "hyperlinks"
            | "hyperlinks.local-style"
            | "images"
            | "images.local-style"
            | "hr"
            | "ul"
            | "ol"
            | "li"
            | "page.code"
    )
}

/// Map a deprecated container alias to its canonical container path.
fn deprecated_container(path: &str) -> Option<&'static str> {
    match path {
        "block_quote" => Some("block-quote"),
        "hyperlinks.local_style" => Some("hyperlinks.local-style"),
        "images.local_style" => Some("images.local-style"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn warnings_for(value: serde_json::Value) -> Vec<StyleWarning> {
        walk(&value)
    }

    #[test]
    fn empty_object_no_warnings() {
        assert!(warnings_for(json!({})).is_empty());
    }

    #[test]
    fn unknown_top_level_bucket() {
        let w = warnings_for(json!({"planet": {"x": 1}}));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.planet");
        assert_eq!(w[0].kind, StyleWarningKind::UnknownKey);
    }

    #[test]
    fn unknown_leaf_under_known_bucket() {
        let w = warnings_for(json!({"page": {"lft-margin": "2ch"}}));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.page.lft-margin");
        assert_eq!(w[0].kind, StyleWarningKind::UnknownKey);
    }

    #[test]
    fn snake_case_leaf_is_deprecated() {
        let w = warnings_for(json!({"page": {"left_margin": "2ch"}}));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.page.left_margin");
        assert_eq!(
            w[0].kind,
            StyleWarningKind::Deprecated {
                replacement: "page.left-margin".to_string()
            }
        );
    }

    #[test]
    fn block_quote_snake_container_is_deprecated() {
        let w = warnings_for(json!({"block_quote": {"max-width": "50%"}}));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.block_quote");
        assert!(matches!(
            w[0].kind,
            StyleWarningKind::Deprecated { ref replacement } if replacement == "block-quote"
        ));
    }

    #[test]
    fn flatten_typo_detected() {
        // `table` is structurally just `CommonStyle` — typos inside it must
        // be detected.
        let w = warnings_for(json!({"table": {"maxx-width": "50%"}}));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.table.maxx-width");
        assert_eq!(w[0].kind, StyleWarningKind::UnknownKey);
    }

    #[test]
    fn nested_local_style_typo_detected() {
        let w = warnings_for(json!({
            "hyperlinks": {
                "local-style": {"maxx-width": "50%"}
            }
        }));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.hyperlinks.local-style.maxx-width");
        assert_eq!(w[0].kind, StyleWarningKind::UnknownKey);
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
}
