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
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// An age in the units `wt` shows everywhere: `less than 1 min` below a
/// minute, then minutes below an hour, hours below two days, then days.
pub fn age_text(seconds: u64) -> String {
    let minutes = seconds / 60;
    match minutes {
        0 => "less than 1 min".to_string(),
        1..=59 => format!("{minutes} min"),
        60..=2879 => format!("{} h", minutes / 60),
        _ => format!("{} days", minutes / 1440),
    }
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
