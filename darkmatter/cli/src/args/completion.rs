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

            let selected = if current_path.is_absolute() {
                path.as_path()
            } else {
                path.strip_prefix(base_dir).unwrap_or(&path)
            };
            let display_path = candidate_value(selected, is_dir);

            if seen.insert(display_path.clone()) {
                candidates.push(CompletionCandidate::new(display_path));
            }
        }
    }

    candidates.sort_by(|a, b| a.get_value().cmp(b.get_value()));
    candidates
}

/// Renders one completion candidate from the path the completer selected.
///
/// ## Notes
///
/// A candidate never mixes spelling conventions. When
/// [`biscuit_file::try_portable_string`] declines — a UNC, device-namespace, or
/// irreducible verbatim path has no faithful `/`-separated spelling — the value
/// keeps its native `\` separators, so the directory marker must be native too.
/// A `\\server\share\docs/` candidate reads back as neither convention, and the
/// shell would offer it as a literal completion value.
fn candidate_value(selected: &Path, is_dir: bool) -> String {
    let portable = biscuit_file::try_portable_string(selected);
    let is_portable = portable.is_some();
    let mut value = portable.unwrap_or_else(|| biscuit_file::to_portable_string(selected));

    if is_portable && value.starts_with("./") {
        value = value.trim_start_matches("./").to_string();
    }

    if is_dir {
        let separator = if is_portable {
            '/'
        } else {
            std::path::MAIN_SEPARATOR
        };
        if !value.ends_with(separator) {
            value.push(separator);
        }
    }

    value
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
        assert!(root_values.contains(&"README.md".to_string()));
        assert!(root_values.contains(&"docs/".to_string()));
        assert!(!root_values.iter().any(|value| value.ends_with("notes.txt")));

        let docs_values = completion_values(complete_markdown_files_from(
            temp_dir.path(),
            OsStr::new("docs/"),
        ));
        assert!(docs_values.contains(&"docs/guide.md".to_string()));
        assert!(docs_values.contains(&"docs/deep/".to_string()));

        let deep_values = completion_values(complete_markdown_files_from(
            temp_dir.path(),
            OsStr::new("docs/deep/"),
        ));
        assert!(deep_values.contains(&"docs/deep/nested.md".to_string()));
    }

    /// A UNC directory has no faithful portable spelling, so its value and its
    /// directory marker both stay native. The mixed `\\server\share\docs/` form
    /// is exactly what the decline signal exists to prevent.
    #[test]
    #[cfg(windows)]
    fn unc_directory_candidate_stays_native() {
        let value = candidate_value(Path::new(r"\\server\share\docs"), true);
        assert_eq!(value, r"\\server\share\docs\");
        assert!(
            !value.contains('/'),
            "candidate mixed separator conventions: {value:?}"
        );
    }

    /// The same rule, reached through the enumerating entry point rather than
    /// the renderer, because selection sits between the two: `is_absolute`
    /// decides whether a candidate keeps the enumerated path or is stripped
    /// back to a base-relative one, and only the first branch can produce a
    /// declined spelling at all.
    ///
    /// A trailing dot is the cheapest local decline available. It is creatable
    /// only through the verbatim namespace and is precisely what makes the
    /// legacy spelling unfaithful, so no SMB share has to be reachable for the
    /// native-fallback path to be exercised.
    #[test]
    #[cfg(windows)]
    fn declined_directory_completes_without_mixing_separators() {
        let temp_dir = tempfile::tempdir().unwrap();
        let verbatim_root = std::fs::canonicalize(temp_dir.path()).unwrap();
        let declined_dir = verbatim_root.join("trailing.");
        std::fs::create_dir(&declined_dir).unwrap();
        std::fs::create_dir(declined_dir.join("nested")).unwrap();
        std::fs::write(declined_dir.join("guide.md"), "# Guide").unwrap();
        assert!(
            biscuit_file::try_portable_string(&declined_dir).is_none(),
            "fixture must be a path with no faithful portable spelling"
        );

        let current = format!("{}\\", declined_dir.display());
        let values = completion_values(complete_markdown_files_from(
            temp_dir.path(),
            OsStr::new(&current),
        ));

        assert!(
            values.contains(&format!("{current}guide.md")),
            "expected the native file candidate, got: {values:?}"
        );
        assert!(
            values.contains(&format!("{current}nested\\")),
            "expected a natively terminated directory candidate, got: {values:?}"
        );
        assert!(
            values.iter().all(|value| !value.contains('/')),
            "a candidate mixed separator conventions: {values:?}"
        );

        // The legacy removal `TempDir` performs on drop cannot delete a
        // trailing-dot name, so the verbatim spelling has to do it here.
        std::fs::remove_dir_all(&declined_dir).unwrap();
    }

    #[test]
    fn compose_arg_completion_suggests_files_for_non_setter_tokens() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("README.md"), "# Root").unwrap();

        let values = completion_values(complete_compose_args_from(
            temp_dir.path(),
            OsStr::new("REA"),
        ));
        assert!(values.iter().any(|value| value == "README.md"));
    }

    #[test]
    fn compose_arg_completion_skips_file_suggestions_for_setters() {
        let values = completion_values(complete_compose_args(OsStr::new("name=Al")));
        assert!(values.is_empty());
    }
}
