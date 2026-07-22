//! Shared resolution for the reserved `doc` frontmatter namespace.
//!
//! `doc` is a reserved expression namespace alongside `ctx.*` (run/context) and
//! `env.*` (environment). Every [`EvaluationLookup`](super::EvaluationLookup)
//! that exposes a document's frontmatter routes `doc` lookups through these
//! helpers so the behavior is identical across surfaces:
//!
//! - bare `doc` → the whole root frontmatter object (a snapshot);
//! - `doc.<path>` → dotted traversal into that object;
//! - a property literally named `doc` is reached as `doc.doc`.
//!
//! The namespace is reserved: callers must intercept it *before* normal key
//! lookup and *before* any legacy unprefixed fallback to `ctx.*`, so a missing
//! `doc.<path>` never collapses into `ctx.<path>` and bare `doc` never becomes
//! `ctx.doc`.

use serde_json::Value;
use std::collections::HashMap;

/// Returns `true` when `path` addresses the reserved `doc` namespace — either
/// bare `doc` or a `doc.<path>` traversal.
pub(crate) fn is_doc_namespace(path: &str) -> bool {
    path == "doc" || path.starts_with("doc.")
}

/// Resolves a `doc`-namespace `path` against the document's `root` frontmatter
/// object.
///
/// Call only when [`is_doc_namespace`] is true. Bare `doc` yields the whole
/// `root` object; `doc.<path>` performs dotted traversal. A `doc.<path>` that
/// does not resolve returns `None`; the caller must not fall back to another
/// namespace in that case.
pub(crate) fn resolve_doc_namespace(path: &str, root: &Value) -> Option<Value> {
    match path.strip_prefix("doc.") {
        Some(rest) => doc_nested(root, rest),
        // path == "doc": the whole frontmatter object.
        None => Some(root.clone()),
    }
}

/// Map-based variant of [`resolve_doc_namespace`] that resolves against the
/// effective-state `data` map directly.
///
/// Byte-identical to building a `Value::Object` from `data` and calling
/// [`resolve_doc_namespace`], but a `doc.<path>` traversal borrows into the map
/// and clones only the resolved leaf instead of deep-cloning the entire state
/// per lookup (F30). Bare `doc` still materializes the whole object snapshot.
pub(crate) fn resolve_doc_namespace_in_map(
    path: &str,
    data: &HashMap<String, Value>,
) -> Option<Value> {
    match path.strip_prefix("doc.") {
        Some(rest) => {
            let mut segments = rest.split('.');
            let mut current = data.get(segments.next()?)?;
            for segment in segments {
                match current {
                    Value::Object(map) => current = map.get(segment)?,
                    _ => return None,
                }
            }
            Some(current.clone())
        }
        // path == "doc": the whole frontmatter object (a snapshot).
        None => Some(Value::Object(data.clone().into_iter().collect())),
    }
}

/// Walks a dotted path through a JSON object, returning the leaf value.
fn doc_nested(root: &Value, path: &str) -> Option<Value> {
    let mut current = root;
    for segment in path.split('.') {
        match current {
            Value::Object(map) => current = map.get(segment)?,
            _ => return None,
        }
    }
    Some(current.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn root() -> Value {
        json!({
            "build": "from-frontmatter",
            "config": { "retries": 3 },
            "doc": { "child": "literal-doc" },
        })
    }

    #[test]
    fn detects_doc_namespace() {
        assert!(is_doc_namespace("doc"));
        assert!(is_doc_namespace("doc.build"));
        assert!(is_doc_namespace("doc.config.retries"));
        assert!(!is_doc_namespace("document"));
        assert!(!is_doc_namespace("build"));
        assert!(!is_doc_namespace("ctx.doc"));
    }

    #[test]
    fn bare_doc_returns_whole_object() {
        let resolved = resolve_doc_namespace("doc", &root()).unwrap();
        assert!(resolved.is_object());
        assert_eq!(resolved.get("build"), Some(&json!("from-frontmatter")));
    }

    #[test]
    fn dotted_path_traverses_root() {
        assert_eq!(
            resolve_doc_namespace("doc.build", &root()),
            Some(json!("from-frontmatter"))
        );
        assert_eq!(
            resolve_doc_namespace("doc.config.retries", &root()),
            Some(json!(3))
        );
    }

    #[test]
    fn literal_doc_property_reached_via_doc_doc() {
        assert_eq!(
            resolve_doc_namespace("doc.doc", &root()),
            Some(json!({ "child": "literal-doc" }))
        );
        assert_eq!(
            resolve_doc_namespace("doc.doc.child", &root()),
            Some(json!("literal-doc"))
        );
    }

    #[test]
    fn missing_path_returns_none() {
        assert_eq!(resolve_doc_namespace("doc.missing", &root()), None);
        assert_eq!(resolve_doc_namespace("doc.build.nope", &root()), None);
    }
}
