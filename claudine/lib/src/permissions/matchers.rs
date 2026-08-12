use std::path::Path;

use crate::path_semantics::{is_exact_or_descendant, normalize, segments};

/// Checks whether a path matches a pattern.
///
/// Supports:
/// - exact path match
/// - prefix match (pattern ends with `/*` or pattern is a parent directory)
/// - simple glob with `*` as a single-segment wildcard
/// - `**` as a multi-segment wildcard
pub fn path_matches(path: &Path, pattern: &str) -> bool {
    let path_str = path.to_string_lossy();
    let path_str = normalize(&path_str);
    let pattern = normalize(pattern.trim());

    if pattern == "*" || pattern == "**" {
        return true;
    }

    if path_str == pattern {
        return true;
    }

    if let Some(prefix) = pattern.strip_suffix("/*") {
        return path_str.as_ref() != prefix && is_exact_or_descendant(&path_str, prefix);
    }

    if pattern.ends_with('/') {
        let prefix = pattern.trim_end_matches('/');
        return is_exact_or_descendant(&path_str, prefix);
    }

    if pattern.contains('*') {
        return glob_matches(&path_str, &pattern);
    }

    is_exact_or_descendant(&path_str, &pattern)
}

/// Checks whether a command matches a pattern.
///
/// Matches against both the raw command string and the extracted executable.
pub fn command_matches(raw: &str, executable: Option<&str>, pattern: &str) -> bool {
    let pattern = pattern.trim();

    if pattern == "*" {
        return true;
    }

    // Exact raw match
    if raw == pattern {
        return true;
    }

    // Executable match
    if let Some(exe) = executable
        && exe == pattern
    {
        return true;
    }

    // Prefix match on raw command
    if raw.starts_with(pattern) {
        return true;
    }

    // Simple wildcard pattern
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let (prefix, suffix) = (parts[0], parts[1]);
            return raw.starts_with(prefix) && raw.ends_with(suffix);
        }
    }

    false
}

/// Checks whether a domain matches a pattern.
///
/// Supports:
/// - exact match
/// - wildcard match (e.g., `*.example.com` matches `sub.example.com`)
/// - universal wildcard `*`
pub fn domain_matches(domain: &str, pattern: &str) -> bool {
    let domain = domain.to_lowercase();
    let pattern = pattern.to_lowercase();

    // Exact match
    if domain == pattern {
        return true;
    }

    // Wildcard match: *.example.com
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return domain.ends_with(&format!(".{suffix}")) || domain == suffix;
    }

    // Universal wildcard
    if pattern == "*" {
        return true;
    }

    false
}

/// Simple glob matching for path patterns.
///
/// Supports `*` as a single-segment wildcard (does not cross `/` boundaries)
/// and `**` as a multi-segment wildcard.
fn glob_matches(text: &str, pattern: &str) -> bool {
    // Handle ** (matches any number of path segments)
    if pattern.contains("**") {
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0].trim_end_matches('/');
            let suffix = parts[1].trim_start_matches('/');

            if !prefix.is_empty() && !is_exact_or_descendant(text, prefix) {
                return false;
            }
            if suffix.is_empty() {
                return true;
            }
            // Handle suffix containing single-star wildcards (e.g., "*.txt")
            if suffix.contains('*') {
                let suffix_parts: Vec<&str> = suffix.split('*').collect();
                if suffix_parts.len() == 2 {
                    let (before, after) = (suffix_parts[0], suffix_parts[1]);
                    // Find the filename portion of text (last segment)
                    if let Some(filename) = segments(text).next_back() {
                        let before_ok = before.is_empty() || filename.starts_with(before);
                        let after_ok = after.is_empty() || filename.ends_with(after);
                        return before_ok && after_ok;
                    }
                }
            }
            return text.ends_with(suffix);
        }
    }

    // Handle single * (matches within one segment)
    let pattern_segments: Vec<&str> = segments(pattern).collect();
    let text_segments: Vec<&str> = segments(text).collect();

    if pattern_segments.len() != text_segments.len() {
        return false;
    }

    pattern_segments
        .iter()
        .zip(text_segments.iter())
        .all(|(p, t)| *p == "*" || p == t)
}

/// Classification of a path relative to known directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathClassification {
    /// Path is part of a provider-owned configuration area.
    ProviderConfig,
    /// Path is within the workspace (repo root or cwd).
    Workspace,
    /// Path is within the user's home directory but outside workspace.
    Home,
    /// Path is in a temporary directory.
    Temp,
    /// Path is a system path.
    System,
    /// Path is external to all known scopes.
    External,
}

/// Classifies a path relative to known directories.
pub fn classify_path(
    path: &Path,
    workspace: Option<&Path>,
    home: Option<&Path>,
) -> PathClassification {
    classify_path_with_config(path, workspace, home, &[])
}

/// Classifies a path relative to known directories and provider config roots.
pub fn classify_path_with_config(
    path: &Path,
    workspace: Option<&Path>,
    home: Option<&Path>,
    provider_config_roots: &[&Path],
) -> PathClassification {
    if provider_config_roots
        .iter()
        .any(|root| path.starts_with(root))
    {
        return PathClassification::ProviderConfig;
    }

    if let Some(ws) = workspace
        && path.starts_with(ws)
    {
        return PathClassification::Workspace;
    }

    let temp = std::env::temp_dir();
    if path.starts_with(&temp) {
        return PathClassification::Temp;
    }

    if let Some(h) = home
        && path.starts_with(h)
    {
        return PathClassification::Home;
    }

    if path.starts_with("/usr") || path.starts_with("/etc") || path.starts_with("/System") {
        return PathClassification::System;
    }

    PathClassification::External
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn path_exact_match() {
        assert!(path_matches(Path::new("/foo/bar.txt"), "/foo/bar.txt"));
        assert!(!path_matches(Path::new("/foo/bar.txt"), "/foo/baz.txt"));
    }

    #[test]
    fn path_prefix_match_with_star() {
        assert!(path_matches(Path::new("/foo/bar/baz.txt"), "/foo/bar/*"));
        assert!(!path_matches(Path::new("/foo/barbaz.txt"), "/foo/bar/*"));
    }

    #[test]
    fn path_prefix_match_implicit() {
        assert!(path_matches(Path::new("/foo/bar/baz.txt"), "/foo/bar"));
        assert!(!path_matches(Path::new("/foo/barbaz.txt"), "/foo/bar"));
    }

    #[test]
    fn path_glob_single_star() {
        assert!(path_matches(
            Path::new("/foo/bar/baz.txt"),
            "/foo/*/baz.txt",
        ));
        assert!(!path_matches(
            Path::new("/foo/bar/qux/baz.txt"),
            "/foo/*/baz.txt",
        ));
    }

    #[test]
    fn path_double_star_match() {
        assert!(path_matches(
            Path::new("/foo/bar/baz/qux.txt"),
            "/foo/**/*.txt",
        ));
    }

    #[test]
    fn path_separator_spellings_are_interchangeable() {
        assert!(path_matches(
            Path::new(r"C:\proj\src\main.rs"),
            r"C:\proj\src\main.rs",
        ));
        assert!(path_matches(
            Path::new(r"C:\proj\src\main.rs"),
            "C:/proj/src/main.rs",
        ));
        assert!(path_matches(
            Path::new("C:/proj/src/main.rs"),
            r"C:\proj\src\main.rs",
        ));
    }

    #[test]
    fn portable_directory_matching_covers_every_pattern_form() {
        let child = Path::new(r"C:\proj\src\main.rs");
        assert!(path_matches(child, r"C:\proj\src\main.rs"));
        assert!(path_matches(child, r"C:\proj\*"));
        assert!(path_matches(child, r"C:\proj\"));
        assert!(path_matches(child, r"C:\proj"));
        assert!(path_matches(child, r"C:\proj\*\main.rs"));
        assert!(path_matches(child, r"C:\proj\**\*.rs"));
    }

    #[test]
    fn directory_prefix_requires_a_segment_boundary() {
        assert!(!path_matches(Path::new(r"C:\proj2\file.txt"), r"C:\proj"));
    }

    #[test]
    fn command_exact_match() {
        assert!(command_matches("git push", Some("git"), "git push"));
        assert!(command_matches("git push", Some("git"), "git"));
    }

    #[test]
    fn command_prefix_match() {
        assert!(command_matches("git push origin", Some("git"), "git push"));
    }

    #[test]
    fn command_no_match() {
        assert!(!command_matches("npm install", Some("npm"), "git"));
    }

    #[test]
    fn domain_exact_match() {
        assert!(domain_matches("example.com", "example.com"));
        assert!(!domain_matches("example.com", "other.com"));
    }

    #[test]
    fn domain_wildcard_match() {
        assert!(domain_matches("sub.example.com", "*.example.com"));
        assert!(domain_matches("example.com", "*.example.com"));
        assert!(!domain_matches("example.org", "*.example.com"));
    }

    #[test]
    fn domain_universal_wildcard() {
        assert!(domain_matches("anything.com", "*"));
    }

    #[test]
    fn domain_case_insensitive() {
        assert!(domain_matches("Example.COM", "example.com"));
    }

    #[test]
    fn classify_path_workspace() {
        let ws = PathBuf::from("/projects/myapp");
        assert_eq!(
            classify_path(Path::new("/projects/myapp/src/main.rs"), Some(&ws), None,),
            PathClassification::Workspace,
        );
    }

    #[test]
    fn classify_path_provider_config() {
        let home = PathBuf::from("/Users/test");
        let config_root = home.join(".claude");
        assert_eq!(
            classify_path_with_config(
                Path::new("/Users/test/.claude/settings.json"),
                None,
                Some(&home),
                &[&config_root],
            ),
            PathClassification::ProviderConfig,
        );
    }

    #[test]
    fn classify_path_system() {
        assert_eq!(
            classify_path(Path::new("/etc/hosts"), None, None),
            PathClassification::System,
        );
    }

    #[test]
    fn classify_path_external() {
        assert_eq!(
            classify_path(Path::new("/mnt/data/file.txt"), None, None),
            PathClassification::External,
        );
    }
}
