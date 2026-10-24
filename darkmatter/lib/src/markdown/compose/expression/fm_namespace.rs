//! Shared resolution for the reserved `fm` frontmatter namespace.
//!
//! `fm` is a reserved expression root alongside `ctx.*` (run/context) and
//! `env.*` (environment). Every [`EvaluationLookup`](super::EvaluationLookup)
//! that exposes a document's frontmatter routes `fm` lookups through these
//! helpers so the behavior is identical across surfaces:
//!
//! - bare `fm` → the whole root frontmatter object (a snapshot);
//! - `fm.<path>` → dotted traversal into that object;
//! - a property literally named `fm` is reached as `fm.fm`.
//!
//! The namespace is reserved: callers must intercept it *before* normal key
//! lookup and *before* any legacy unprefixed fallback to `ctx.*`, so a missing
//! `fm.<path>` never collapses into `ctx.<path>` and bare `fm` never becomes
//! `ctx.fm`.

use serde_json::{Map, Value};

use super::ast::Expr;

/// The reserved expression root that names the document's own frontmatter.
pub const FRONTMATTER_ROOT: &str = "fm";

/// What a path addresses inside the reserved [`FRONTMATTER_ROOT`] namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmPath<'a> {
    /// Bare `fm`: the whole frontmatter object.
    Whole,
    /// `fm.<rest>`: a dotted path into the frontmatter.
    Member(&'a str),
}

/// Classifies `path` against the `fm` namespace; `None` when the path is
/// outside it (including look-alikes such as `fmt` or `ctx.fm`).
pub fn fm_namespace_path(path: &str) -> Option<FmPath<'_>> {
    let rest = path.strip_prefix(FRONTMATTER_ROOT)?;
    if rest.is_empty() {
        return Some(FmPath::Whole);
    }
    rest.strip_prefix('.').map(FmPath::Member)
}

/// Returns `true` when `path` addresses the reserved `fm` namespace — either
/// bare `fm` or a `fm.<path>` traversal.
pub(crate) fn is_fm_namespace(path: &str) -> bool {
    fm_namespace_path(path).is_some()
}

/// Strips a leading `fm.` so a namespaced read names the same frontmatter key
/// as its bare form; other paths, including bare `fm`, are returned unchanged.
pub(crate) fn strip_fm_prefix(path: &str) -> &str {
    match fm_namespace_path(path) {
        Some(FmPath::Member(rest)) => rest,
        _ => path,
    }
}

/// The frontmatter key a literal index on bare `fm` names: `fm['a']` → `a`,
/// `fm['release.channel']` → the single key `release.channel`. `None` for any
/// other index shape, including a computed index such as `fm[name]`.
pub(crate) fn literal_index_key<'e>(base: &Expr, index: &'e Expr) -> Option<&'e str> {
    match (base, index) {
        (Expr::Variable(root), Expr::StringLiteral(key)) if root == FRONTMATTER_ROOT && !key.is_empty() => {
            Some(key)
        }
        _ => None,
    }
}

/// Resolves an `fm`-namespace `path` against the document's `root` frontmatter
/// object.
///
/// Call only when [`is_fm_namespace`] is true. A `fm.<path>` that does not
/// resolve returns `None`; the caller must not fall back to another namespace
/// in that case.
pub(crate) fn resolve_fm_namespace(path: &str, root: &Value) -> Option<Value> {
    match fm_namespace_path(path)? {
        FmPath::Member(rest) => traverse(root.as_object()?, rest),
        FmPath::Whole => Some(root.clone()),
    }
}

/// Resolves an `fm`-namespace `path` against a frontmatter mapping, cloning
/// only the selected value for a dotted read. `None` when `path` is outside the
/// namespace or the member is absent.
pub fn resolve_fm_namespace_in_object(path: &str, frontmatter: &Map<String, Value>) -> Option<Value> {
    match fm_namespace_path(path)? {
        FmPath::Member(rest) => traverse(frontmatter, rest),
        FmPath::Whole => Some(Value::Object(frontmatter.clone())),
    }
}

fn traverse(map: &Map<String, Value>, path: &str) -> Option<Value> {
    let mut segments = path.split('.');
    let first = map.get(segments.next()?)?;
    descend(first, segments)
}

fn descend<'a>(mut current: &Value, segments: impl Iterator<Item = &'a str>) -> Option<Value> {
    for segment in segments {
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
            "fm": { "child": "literal-fm" },
        })
    }

    #[test]
    fn detects_fm_namespace() {
        assert!(is_fm_namespace("fm"));
        assert!(is_fm_namespace("fm.build"));
        assert!(is_fm_namespace("fm.config.retries"));
        assert!(!is_fm_namespace("fmt"));
        assert!(!is_fm_namespace("doc"));
        assert!(!is_fm_namespace("doc.build"));
        assert!(!is_fm_namespace("build"));
        assert!(!is_fm_namespace("ctx.fm"));
    }

    #[test]
    fn strip_fm_prefix_keeps_bare_root_and_other_paths() {
        assert_eq!(strip_fm_prefix("fm.a.b"), "a.b");
        assert_eq!(strip_fm_prefix("fm"), "fm");
        assert_eq!(strip_fm_prefix("doc.a"), "doc.a");
        assert_eq!(strip_fm_prefix("fmt.a"), "fmt.a");
    }

    #[test]
    fn bare_fm_returns_whole_object() {
        let resolved = resolve_fm_namespace("fm", &root()).unwrap();
        assert_eq!(resolved, root());
    }

    #[test]
    fn dotted_path_traverses_root() {
        assert_eq!(resolve_fm_namespace("fm.build", &root()), Some(json!("from-frontmatter")));
        assert_eq!(resolve_fm_namespace("fm.config.retries", &root()), Some(json!(3)));
    }

    #[test]
    fn literal_fm_property_reached_via_fm_fm() {
        assert_eq!(resolve_fm_namespace("fm.fm", &root()), Some(json!({ "child": "literal-fm" })));
        assert_eq!(resolve_fm_namespace("fm.fm.child", &root()), Some(json!("literal-fm")));
    }

    #[test]
    fn missing_path_returns_none() {
        assert_eq!(resolve_fm_namespace("fm.missing", &root()), None);
        assert_eq!(resolve_fm_namespace("fm.build.nope", &root()), None);
    }

    #[test]
    fn object_variant_agrees_with_value_variant() {
        let object = root().as_object().cloned().unwrap();
        for path in ["fm", "fm.build", "fm.config.retries", "fm.fm.child", "fm.missing", "fm.build.nope"] {
            let expected = resolve_fm_namespace(path, &root());
            assert_eq!(resolve_fm_namespace_in_object(path, &object), expected, "{path}");
        }
        assert_eq!(resolve_fm_namespace_in_object("doc.build", &object), None);
    }
}
