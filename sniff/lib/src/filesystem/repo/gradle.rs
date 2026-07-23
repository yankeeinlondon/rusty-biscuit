//! Gradle multi-project build (`settings.gradle[.kts]` `include`) detection.

use std::path::Path;

use crate::Result;
use crate::performance;
use crate::performance::counters;

use super::detection::{
    DetectorOutcome, probe_exists,
};
use super::seed::{PackageSeed, merge_seeds};
use super::standard::{MonorepoStandard, PackageProvenance};

pub(super) fn detect_gradle_workspace(root: &Path) -> Result<Option<DetectorOutcome>> {
    let settings = root.join("settings.gradle");
    let settings_kts = root.join("settings.gradle.kts");
    let settings_path = if probe_exists(&settings) {
        settings
    } else if probe_exists(&settings_kts) {
        settings_kts
    } else {
        return Ok(None);
    };

    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let content = std::fs::read_to_string(&settings_path)?;
    performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
    performance::increment_counter(counters::REPO_MANIFEST_PARSES, 1);
    let includes = parse_gradle_includes(&content);
    if includes.is_empty() {
        return Ok(None);
    }

    let mut seeds = Vec::new();
    for member in includes {
        let member_path = root.join(&member);
        if !probe_exists(&member_path) {
            continue;
        }
        seeds.push(PackageSeed::new(
            &member_path,
            root,
            MonorepoStandard::GradleMultiProject,
            PackageProvenance::Explicit,
        ));
    }

    if seeds.is_empty() {
        return Ok(None);
    }


    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::GradleMultiProject,
        root: root.to_path_buf(),
        seeds: merge_seeds(seeds),
    }))
}

/// Parse subproject directories from a `settings.gradle[.kts]` file's `include`
/// directives.
///
/// Gradle paths are colon-separated (`:app:core`) and map to nested directories
/// (`app/core`); a leading colon is the root and is stripped. Both the Groovy
/// (`include 'a', ':b'`) and Kotlin (`include("a", "b")`) call forms are handled.
/// `includeBuild` (composite builds) and `includeFlat` are deliberately ignored —
/// only direct `include` membership counts.
pub(super) fn parse_gradle_includes(content: &str) -> Vec<String> {
    let mut includes = Vec::new();

    for line in content.lines() {
        let trimmed = line.split("//").next().unwrap_or("").trim();
        let Some(rest) = trimmed.strip_prefix("include") else {
            continue;
        };
        // Require a boundary after `include` so `includeBuild` / `includeFlat`
        // do not match. A direct `include` is followed by whitespace, `(`, or a
        // quote.
        if !rest
            .chars()
            .next()
            .is_none_or(|c| c.is_whitespace() || c == '(' || c == '\'' || c == '"')
        {
            continue;
        }

        for raw in extract_quoted(rest) {
            if let Some(path) = gradle_path_to_relative(&raw) {
                includes.push(path);
            }
        }
    }

    includes
}

/// Collect the contents of every single- or double-quoted span in `s`.
fn extract_quoted(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' || c == '"' {
            let quote = c;
            let mut buf = String::new();
            for next in chars.by_ref() {
                if next == quote {
                    break;
                }
                buf.push(next);
            }
            out.push(buf);
        }
    }
    out
}

/// Translate a Gradle project path (`:app:core`) to a repo-relative directory
/// (`app/core`), or `None` when blank.
fn gradle_path_to_relative(raw: &str) -> Option<String> {
    let relative = raw.trim().trim_start_matches(':').replace(':', "/");
    if relative.is_empty() {
        None
    } else {
        Some(relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_includes_handles_groovy_comma_list() {
        let content = "rootProject.name = 'app'\ninclude 'core', ':api'\n";
        assert_eq!(parse_gradle_includes(content), vec!["core", "api"]);
    }

    #[test]
    fn parse_includes_handles_kotlin_call_form() {
        let content = "include(\"core\", \"api\")\n";
        assert_eq!(parse_gradle_includes(content), vec!["core", "api"]);
    }

    #[test]
    fn parse_includes_maps_colon_paths_to_directories() {
        let content = "include ':app:core'\n";
        assert_eq!(parse_gradle_includes(content), vec!["app/core"]);
    }

    #[test]
    fn parse_includes_ignores_include_build() {
        let content = "includeBuild '../shared'\ninclude 'core'\n";
        assert_eq!(parse_gradle_includes(content), vec!["core"]);
    }

    #[test]
    fn parse_includes_empty_when_absent() {
        assert!(parse_gradle_includes("rootProject.name = 'solo'\n").is_empty());
    }
}
