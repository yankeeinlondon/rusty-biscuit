/// Convert a branch name to a filesystem-safe dasherized directory name.
///
/// ## Examples
///
/// ```
/// use worktree::util::dasherize;
///
/// assert_eq!(dasherize("feature/my-thing"), "feature-my-thing");
/// assert_eq!(dasherize("fix/JIRA-123/description"), "fix-jira-123-description");
/// assert_eq!(dasherize("simple"), "simple");
/// ```
pub fn dasherize(branch: &str) -> String {
    branch
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dasherize_slash_separated() {
        assert_eq!(dasherize("feature/my-thing"), "feature-my-thing");
    }

    #[test]
    fn dasherize_nested_slashes() {
        assert_eq!(
            dasherize("fix/JIRA-123/description"),
            "fix-jira-123-description"
        );
    }

    #[test]
    fn dasherize_simple_name() {
        assert_eq!(dasherize("main"), "main");
    }

    #[test]
    fn dasherize_uppercase() {
        assert_eq!(dasherize("Feature/MyBranch"), "feature-mybranch");
    }

    #[test]
    fn dasherize_multiple_separators() {
        assert_eq!(dasherize("a//b--c__d"), "a-b-c-d");
    }
}
