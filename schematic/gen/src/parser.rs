//! Path parameter extraction from endpoint paths.
//!
//! Extracts parameter names from URL path templates that use `{param}` syntax.

/// Extracts parameter names from a path template.
///
/// ## Examples
///
/// ```
/// use schematic_gen::parser::extract_path_params;
///
/// assert_eq!(extract_path_params("/models"), vec![] as Vec<&str>);
/// assert_eq!(extract_path_params("/models/{model}"), vec!["model"]);
/// assert_eq!(
///     extract_path_params("/threads/{thread_id}/messages/{message_id}"),
///     vec!["thread_id", "message_id"]
/// );
/// ```
pub fn extract_path_params(path: &str) -> Vec<&str> {
    let mut params = Vec::new();
    let mut pos = 0;

    for (idx, c) in path.char_indices() {
        if c == '{' {
            pos = idx + 1; // Start after '{'
        } else if c == '}' && pos > 0 {
            let param = &path[pos..idx];
            if !param.is_empty() {
                params.push(param);
            }
            pos = 0;
        }
    }

    params
}

/// Substitutes path parameters with their values.
///
/// ## Examples
///
/// ```
/// use schematic_gen::parser::substitute_path_params;
///
/// let path = substitute_path_params(
///     "/models/{model}",
///     &[("model", "gpt-4")]
/// );
/// assert_eq!(path, "/models/gpt-4");
/// ```
pub fn substitute_path_params(path: &str, params: &[(&str, &str)]) -> String {
    let mut result = path.to_string();
    for (name, value) in params {
        let placeholder = format!("{{{}}}", name);
        result = result.replace(&placeholder, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_no_params() {
        assert_eq!(extract_path_params("/models"), Vec::<&str>::new());
        assert_eq!(extract_path_params("/v1/models"), Vec::<&str>::new());
        assert_eq!(extract_path_params("/"), Vec::<&str>::new());
    }

    #[test]
    fn extract_single_param() {
        assert_eq!(extract_path_params("/models/{model}"), vec!["model"]);
        assert_eq!(extract_path_params("/{id}"), vec!["id"]);
        assert_eq!(
            extract_path_params("/users/{user_id}/posts"),
            vec!["user_id"]
        );
    }

    #[test]
    fn extract_multiple_params() {
        assert_eq!(
            extract_path_params("/threads/{thread_id}/messages/{message_id}"),
            vec!["thread_id", "message_id"]
        );
        assert_eq!(
            extract_path_params("/orgs/{org}/repos/{repo}/issues/{issue}"),
            vec!["org", "repo", "issue"]
        );
    }

    #[test]
    fn extract_consecutive_params() {
        assert_eq!(extract_path_params("/{a}/{b}"), vec!["a", "b"]);
    }

    #[test]
    fn substitute_single_param() {
        assert_eq!(
            substitute_path_params("/models/{model}", &[("model", "gpt-4")]),
            "/models/gpt-4"
        );
    }

    #[test]
    fn substitute_multiple_params() {
        assert_eq!(
            substitute_path_params(
                "/threads/{thread_id}/messages/{message_id}",
                &[("thread_id", "abc123"), ("message_id", "xyz789")]
            ),
            "/threads/abc123/messages/xyz789"
        );
    }

    #[test]
    fn substitute_no_params() {
        assert_eq!(substitute_path_params("/models", &[]), "/models");
    }

    #[test]
    fn substitute_missing_param_unchanged() {
        // If a param isn't provided, the placeholder remains
        assert_eq!(
            substitute_path_params("/models/{model}", &[]),
            "/models/{model}"
        );
    }
}
