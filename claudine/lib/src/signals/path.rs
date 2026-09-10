//! Restricted JSONPath walker: dot segments plus numeric bracket indices.
//!
//! This is the full grammar `match_path` / `ExtractionSpec::path` are allowed
//! to use (`error.responseBody.code`, `message.content[0].text`) — no
//! wildcards, filters, or recursive descent. The generator validates corpus
//! paths against the same subset.

use serde_json::Value;

/// Resolve `path` against `value`. A missing object key, out-of-range index,
/// or type mismatch (indexing a non-array, keying a non-object) yields `None`.
pub(crate) fn walk<'v>(value: &'v Value, path: &str) -> Option<&'v Value> {
    let mut current = value;
    for segment in path.split('.') {
        let (name, mut brackets) = match segment.find('[') {
            Some(idx) => (&segment[..idx], &segment[idx..]),
            None => (segment, ""),
        };
        if !name.is_empty() {
            current = current.as_object()?.get(name)?;
        }
        while let Some(rest) = brackets.strip_prefix('[') {
            let end = rest.find(']')?;
            let index: usize = rest[..end].parse().ok()?;
            current = current.as_array()?.get(index)?;
            brackets = &rest[end + 1..];
        }
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn walks_dot_segments() {
        let payload = json!({"error": {"responseBody": {"code": -32004}}});
        assert_eq!(
            walk(&payload, "error.responseBody.code"),
            Some(&json!(-32004))
        );
    }

    #[test]
    fn walks_bracket_indices() {
        let payload = json!({"message": {"content": [{"text": "hello"}]}});
        assert_eq!(walk(&payload, "message.content[0].text"), Some(&json!("hello")));
        assert_eq!(walk(&payload, "message.content[1].text"), None);
    }

    #[test]
    fn absent_segment_is_none() {
        let payload = json!({"a": {"b": 1}});
        assert_eq!(walk(&payload, "a.c"), None);
        assert_eq!(walk(&payload, "a.b.c"), None);
        assert_eq!(walk(&payload, "a[0]"), None);
    }
}
