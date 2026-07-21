use clap_complete::engine::CompletionCandidate;
use darkmatter::markdown::highlighting::{ColorMode, ThemePair};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Completes markdown files (`.md`, `.dm`) and directory paths.
pub fn complete_markdown_files(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_markdown_files_from(Path::new("."), current)
}

/// Completes compose positionals.
///
/// Tokens containing `=` are treated as shorthand setters, so file completion
/// is suppressed to avoid suggesting markdown paths for setter values.
pub fn complete_compose_args(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_compose_args_from(Path::new("."), current)
}

pub fn complete_compose_args_from(
    base_dir: &Path,
    current: &OsStr,
) -> Vec<CompletionCandidate> {
    if current.to_string_lossy().contains('=') {
        Vec::new()
    } else {
        complete_markdown_files_from(base_dir, current)
    }
}

pub fn complete_markdown_files_from(
    base_dir: &Path,
    current: &OsStr,
) -> Vec<CompletionCandidate> {
    let current_str = current.to_string_lossy();
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();

    let is_markdown = |p: &Path| {
        p.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("dm"))
            .unwrap_or(false)
    };

    if !current_str.is_empty() && "-".starts_with(current_str.as_ref()) {
        seen.insert("-".to_string());
        candidates.push(CompletionCandidate::new("-"));
    }

    let has_trailing_sep = current_str.ends_with('/') || current_str.ends_with('\\');
    let current_path = Path::new(current_str.as_ref());
    let (dir_part, file_prefix) = if current_str.is_empty() {
        (PathBuf::new(), String::new())
    } else if has_trailing_sep {
        (PathBuf::from(current_str.as_ref()), String::new())
    } else {
        let parent = current_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let prefix = current_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        (parent, prefix)
    };

    let search_dir = if dir_part.as_os_str().is_empty() {
        base_dir.to_path_buf()
    } else if dir_part.is_absolute() {
        dir_part.clone()
    } else {
        base_dir.join(&dir_part)
    };

    if let Ok(entries) = std::fs::read_dir(&search_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if !name.starts_with(&file_prefix) {
                continue;
            }

            let is_dir = path.is_dir();
            if !is_dir && !is_markdown(&path) {
                continue;
            }

            let mut display_path = if current_path.is_absolute() {
                path.to_string_lossy().to_string()
            } else {
                path.strip_prefix(base_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string()
            };

            if display_path.starts_with("./") {
                display_path = display_path.trim_start_matches("./").to_string();
            }

            if is_dir && !display_path.ends_with('/') {
                display_path.push('/');
            }

            if seen.insert(display_path.clone()) {
                candidates.push(CompletionCandidate::new(display_path));
            }
        }
    }

    candidates.sort_by(|a, b| a.get_value().cmp(b.get_value()));
    candidates
}

/// Completes supported list indentation widths.
pub fn complete_indent_values(current: &OsStr) -> Vec<CompletionCandidate> {
    let current_str = current.to_string_lossy();
    let mut candidates: Vec<_> = ["2", "4", "8"]
        .into_iter()
        .filter(|value| value.starts_with(current_str.as_ref()))
        .map(CompletionCandidate::new)
        .collect();
    candidates.sort_by(|a, b| a.get_value().cmp(b.get_value()));
    candidates
}

/// Completes common fixed-width wrapping targets.
pub fn complete_fixed_width_values(current: &OsStr) -> Vec<CompletionCandidate> {
    let current_str = current.to_string_lossy();
    let mut candidates: Vec<_> = ["40", "60", "80", "100", "120"]
        .into_iter()
        .filter(|value| value.starts_with(current_str.as_ref()))
        .map(CompletionCandidate::new)
        .collect();
    candidates.sort_by(|a, b| a.get_value().cmp(b.get_value()));
    candidates
}

/// Completes theme names for `--theme` / `--code-theme`.
///
/// Enumerates every available [`ThemePair`](ThemePair)
/// by its kebab-case name (the same set `--list-themes` prints), attaching each
/// theme's description as completion help. Without this completer the dynamic
/// completion engine has no value source for the theme flags, so `--theme <tab>`
/// offers nothing.
pub fn complete_theme_names(current: &OsStr) -> Vec<CompletionCandidate> {
    let current_str = current.to_string_lossy();
    ThemePair::all()
        .iter()
        .filter(|pair| pair.kebab_name().starts_with(current_str.as_ref()))
        .map(|pair| {
            CompletionCandidate::new(pair.kebab_name())
                .help(Some(pair.description(ColorMode::Dark).into()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn completion_values(candidates: Vec<CompletionCandidate>) -> Vec<String> {
        candidates
            .into_iter()
            .map(|candidate| candidate.get_value().to_string_lossy().into_owned())
            .collect()
    }

    fn normalize_path(path: &str) -> String {
        path.replace('\\', "/")
    }

    #[test]
    fn complete_indent_values_lists_valid_widths() {
        let values = completion_values(complete_indent_values(OsStr::new("")));
        assert_eq!(values, vec!["2", "4", "8"]);

        let values = completion_values(complete_indent_values(OsStr::new("4")));
        assert_eq!(values, vec!["4"]);

        let values = completion_values(complete_indent_values(OsStr::new("8")));
        assert_eq!(values, vec!["8"]);
    }

    #[test]
    fn complete_fixed_width_values_lists_common_widths() {
        let values = completion_values(complete_fixed_width_values(OsStr::new("")));
        assert_eq!(values, vec!["100", "120", "40", "60", "80"]);

        let values = completion_values(complete_fixed_width_values(OsStr::new("8")));
        assert_eq!(values, vec!["80"]);
    }

    #[test]
    fn complete_theme_names_lists_all_themes() {
        let values = completion_values(complete_theme_names(OsStr::new("")));
        let expected: Vec<String> = ThemePair::all()
            .iter()
            .map(|pair| pair.kebab_name().to_string())
            .collect();
        assert_eq!(values, expected);
        assert!(values.contains(&"dracula".to_string()));
        assert!(values.contains(&"nord".to_string()));

        let values = completion_values(complete_theme_names(OsStr::new("gru")));
        assert_eq!(values, vec!["gruvbox"]);

        assert!(completion_values(complete_theme_names(OsStr::new("zzz"))).is_empty());
    }

    #[test]
    fn complete_markdown_files_from_supports_nested_paths() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("README.md"), "# Root").unwrap();
        std::fs::write(temp_dir.path().join("notes.txt"), "ignore").unwrap();

        let docs_dir = temp_dir.path().join("docs");
        let deep_dir = docs_dir.join("deep");
        std::fs::create_dir_all(&deep_dir).unwrap();
        std::fs::write(docs_dir.join("guide.md"), "# Guide").unwrap();
        std::fs::write(deep_dir.join("nested.md"), "# Nested").unwrap();

        let root_values = completion_values(complete_markdown_files_from(
            temp_dir.path(),
            OsStr::new(""),
        ));
        let root_values: Vec<_> = root_values
            .into_iter()
            .map(|value| normalize_path(&value))
            .collect();
        assert!(root_values.contains(&"README.md".to_string()));
        assert!(root_values.contains(&"docs/".to_string()));
        assert!(!root_values.iter().any(|value| value.ends_with("notes.txt")));

        let docs_values = completion_values(complete_markdown_files_from(
            temp_dir.path(),
            OsStr::new("docs/"),
        ));
        let docs_values: Vec<_> = docs_values
            .into_iter()
            .map(|value| normalize_path(&value))
            .collect();
        assert!(docs_values.contains(&"docs/guide.md".to_string()));
        assert!(docs_values.contains(&"docs/deep/".to_string()));

        let deep_values = completion_values(complete_markdown_files_from(
            temp_dir.path(),
            OsStr::new("docs/deep/"),
        ));
        let deep_values: Vec<_> = deep_values
            .into_iter()
            .map(|value| normalize_path(&value))
            .collect();
        assert!(deep_values.contains(&"docs/deep/nested.md".to_string()));
    }

    #[test]
    fn compose_arg_completion_suggests_files_for_non_setter_tokens() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("README.md"), "# Root").unwrap();

        let values = completion_values(complete_compose_args_from(
            temp_dir.path(),
            OsStr::new("REA"),
        ));
        assert!(
            values
                .iter()
                .any(|value| normalize_path(value) == "README.md")
        );
    }

    #[test]
    fn compose_arg_completion_skips_file_suggestions_for_setters() {
        let values = completion_values(complete_compose_args(OsStr::new("name=Al")));
        assert!(values.is_empty());
    }
}
