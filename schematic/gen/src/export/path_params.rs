//! Path parameter and folder key extraction for export.

/// Strips RFC 6570 reserved-expansion markers from a path template.
///
/// A reserved param `{+name}` is a runtime encoding hint only; exported
/// artifacts must render it as the valid `{name}` template. This rewrites
/// every `{+name}` to `{name}` and leaves plain params untouched.
///
/// ## Examples
///
/// ```
/// use schematic_gen::export::strip_reserved_markers;
///
/// assert_eq!(strip_reserved_markers("/models/{+repo_id}"), "/models/{repo_id}");
/// assert_eq!(strip_reserved_markers("/models/{model}"), "/models/{model}");
/// ```
pub fn strip_reserved_markers(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        result.push(c);
        if c == '{' && chars.peek() == Some(&'+') {
            chars.next();
        }
    }
    result
}

/// Extracts a folder key from a URL path for Postman folder grouping.
///
/// Algorithm:
/// 1. Strip leading `/`
/// 2. Ignore path variables `{...}`
/// 3. Ignore version prefixes (`v1`, `v2`, etc.)
/// 4. First remaining segment = folder key
/// 5. No stable segment → `None` (collection root)
///
/// ## Examples
///
/// ```
/// use schematic_gen::export::extract_folder_key;
///
/// assert_eq!(extract_folder_key("/models"), Some("models".to_string()));
/// assert_eq!(extract_folder_key("/repos/{owner}/{repo}/issues"), Some("repos".to_string()));
/// assert_eq!(extract_folder_key("/v1/audio/speech"), Some("audio".to_string()));
/// assert_eq!(extract_folder_key("/"), None);
/// assert_eq!(extract_folder_key("/{id}"), None);
/// ```
pub fn extract_folder_key(path: &str) -> Option<String> {
    let trimmed = path.strip_prefix('/').unwrap_or(path);

    for segment in trimmed.split('/') {
        // Skip empty segments
        if segment.is_empty() {
            continue;
        }
        // Skip path variables
        if segment.starts_with('{') && segment.ends_with('}') {
            continue;
        }
        // Skip version prefixes (v1, v2, etc.)
        if is_version_prefix(segment) {
            continue;
        }
        return Some(segment.to_string());
    }

    None
}

/// Returns true if the segment looks like a version prefix (v1, v2, etc.).
fn is_version_prefix(segment: &str) -> bool {
    if let Some(rest) = segment.strip_prefix('v') {
        rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_key_simple_path() {
        assert_eq!(extract_folder_key("/models"), Some("models".to_string()));
    }

    #[test]
    fn folder_key_with_path_params() {
        assert_eq!(
            extract_folder_key("/repos/{owner}/{repo}/issues"),
            Some("repos".to_string())
        );
    }

    #[test]
    fn folder_key_with_version_prefix() {
        assert_eq!(
            extract_folder_key("/v1/audio/speech"),
            Some("audio".to_string())
        );
    }

    #[test]
    fn folder_key_root_path() {
        assert_eq!(extract_folder_key("/"), None);
    }

    #[test]
    fn folder_key_only_path_params() {
        assert_eq!(extract_folder_key("/{id}"), None);
    }

    #[test]
    fn folder_key_only_version() {
        assert_eq!(extract_folder_key("/v1"), None);
    }

    #[test]
    fn folder_key_nested_path() {
        assert_eq!(
            extract_folder_key("/v2/threads/{thread_id}/messages"),
            Some("threads".to_string())
        );
    }

    #[test]
    fn folder_key_no_leading_slash() {
        assert_eq!(extract_folder_key("models"), Some("models".to_string()));
    }

    #[test]
    fn strip_reserved_markers_rewrites_plus_form() {
        assert_eq!(
            strip_reserved_markers("/models/{+repo_id}"),
            "/models/{repo_id}"
        );
        assert_eq!(
            strip_reserved_markers("/repos/{owner}/{repo}/contents/{+path}"),
            "/repos/{owner}/{repo}/contents/{path}"
        );
    }

    #[test]
    fn strip_reserved_markers_leaves_plain_form() {
        assert_eq!(strip_reserved_markers("/models/{model}"), "/models/{model}");
        assert_eq!(strip_reserved_markers("/models"), "/models");
    }

    #[test]
    fn version_prefix_detection() {
        assert!(is_version_prefix("v1"));
        assert!(is_version_prefix("v2"));
        assert!(is_version_prefix("v10"));
        assert!(!is_version_prefix("v"));
        assert!(!is_version_prefix("va"));
        assert!(!is_version_prefix("version1"));
        assert!(!is_version_prefix("models"));
    }
}
