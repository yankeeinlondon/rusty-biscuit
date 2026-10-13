//! Directory scanning for source files.

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;

use crate::{ProgrammingLanguage, TreeHuggerError, file_identity};

/// Directories that hold test *data* rather than compiled source. Diagnostics on
/// these are meaningless (a fixture's unused import is the point), so a default
/// whole-package scan skips them. An explicit path/glob argument re-includes a
/// targeted fixture file via last-match-wins.
pub const DEFAULT_EXCLUDE_GLOBS: &[&str] = &[
    "**/fixtures/**",
    "**/__fixtures__/**",
    "**/snapshots/**",
    "**/testdata/**",
];

/// Resolves a positional input token to a concrete existing file, relative to
/// the process working directory.
///
/// ## Returns
/// Returns the path when `input` names an existing file and is not a glob;
/// returns `None` for glob patterns or non-files so they flow to the scanner.
pub fn resolve_explicit_file(input: &str) -> Option<PathBuf> {
    if input.contains(['*', '?', '[']) {
        return None;
    }

    let candidate = PathBuf::from(input);
    candidate.is_file().then_some(candidate)
}

/// Collect all supported source files under `root` matching the given filters.
///
/// * `inputs` – positional file/glob filters; empty means scan the whole tree.
/// * `excluded_files` – glob patterns to exclude.
/// * `language` – when `Some`, only files of that language are returned; when
///   `None`, any supported programming language is eligible.
///
/// Honors `.gitignore`, excludes fixture directories by default, and sorts the
/// result deterministically. Every file is returned as its [`file_identity`],
/// so a file named explicitly and also found by the walk, or named twice
/// under different spellings, is listed once.
pub fn collect_files(
    root: &Path,
    inputs: &[String],
    excluded_files: &[String],
    language: Option<ProgrammingLanguage>,
) -> Result<Vec<PathBuf>, TreeHuggerError> {
    collect_selected_files(root, inputs, &[], excluded_files, language)
}

/// [`collect_files`] plus `explicit_files`: existing files the caller already
/// resolved (such as `hug --file`). They are listed whatever their extension,
/// `.gitignore` or the default fixture exclusions say, but `excluded_files`
/// still removes them, and they never widen the scan: with no `inputs`, only
/// they are listed.
pub fn collect_selected_files(
    root: &Path,
    inputs: &[String],
    explicit_files: &[PathBuf],
    excluded_files: &[String],
    language: Option<ProgrammingLanguage>,
) -> Result<Vec<PathBuf>, TreeHuggerError> {
    let mut files = explicit_files.to_vec();
    let mut glob_inputs = Vec::new();

    // A forced `--language` lets us parse an explicitly named file regardless of
    // its extension (extensionless scripts, ambiguous names, or extensions that
    // map to a different language). Such files are honored verbatim and bypass
    // the extension-based scan filter below.
    for input in inputs {
        if language.is_some()
            && let Some(path) = resolve_explicit_file(input)
        {
            files.push(path);
        } else {
            glob_inputs.push(input.clone());
        }
    }

    // Apply `--exclude-files` to explicitly resolved files so exclusion is
    // consistent with the directory/glob scan below.
    if !files.is_empty() && !excluded_files.is_empty() {
        let mut exclude_builder = OverrideBuilder::new(root);
        for excluded_file in excluded_files {
            exclude_builder.add(excluded_file)?;
        }
        let exclude_matcher = exclude_builder.build()?;
        let cwd = std::env::current_dir().ok();
        files.retain(|file| {
            let absolute = match (&cwd, file.is_absolute()) {
                (Some(cwd), false) => cwd.join(file),
                _ => file.clone(),
            };
            !exclude_matcher.matched(&absolute, false).is_whitelist()
        });
    }

    // Walk the tree when scanning the whole package (no inputs and no explicit
    // files) or when some inputs were glob patterns rather than concrete files.
    if (inputs.is_empty() && explicit_files.is_empty()) || !glob_inputs.is_empty() {
        let mut overrides = OverrideBuilder::new(root);
        // Default excludes go first so a later explicit glob/path re-includes a
        // targeted fixture file (gitignore last-match-wins).
        for pattern in DEFAULT_EXCLUDE_GLOBS {
            overrides.add(&format!("!{pattern}"))?;
        }
        for input in &glob_inputs {
            overrides.add(input)?;
        }
        for excluded_file in excluded_files {
            overrides.add(&format!("!{excluded_file}"))?;
        }

        let overrides = overrides.build()?;
        let mut builder = WalkBuilder::new(root);
        builder.standard_filters(true).hidden(false).overrides(overrides);
        // The walker still decides every entry it reaches; literal targets
        // only stop it entering directories no target lies under.
        if let Some(targets) = literal_targets(&glob_inputs) {
            let root = root.to_path_buf();
            builder.filter_entry(move |entry| match entry.path().strip_prefix(&root) {
                Ok(relative) => targets.iter().any(|target| target.starts_with(relative)),
                Err(_) => true,
            });
        }
        let walker = builder.build();

        for entry in walker {
            let entry = entry.map_err(TreeHuggerError::Ignore)?;

            let is_file = entry
                .file_type()
                .map(|file| file.is_file())
                .unwrap_or(false);

            if !is_file {
                continue;
            }

            match language {
                Some(lang) if ProgrammingLanguage::from_path(entry.path()) != Some(lang) => {
                    continue;
                }
                None if ProgrammingLanguage::from_path(entry.path()).is_none() => continue,
                _ => {}
            }

            files.push(entry.into_path());
        }
    }

    let mut files: Vec<PathBuf> = files.iter().map(|file| file_identity(file)).collect();
    files.sort();
    files.dedup();

    if files.is_empty() {
        return Err(TreeHuggerError::NoSourceFiles {
            path: root.to_path_buf(),
        });
    }

    Ok(files)
}

/// The root-relative paths `patterns` can whitelist, when every pattern is a
/// literal one: such a pattern matches exactly the one path it spells, so no
/// file the walker would select lies outside these paths.
///
/// `None` (walk everything) for no patterns, and for any pattern whose
/// override meaning is wider or uncertain: glob syntax or escapes, negation,
/// comments, anchoring, trailing slashes or whitespace, `.`/`..`/empty
/// components, a drive colon, and a bare name (which matches at any depth).
fn literal_targets(patterns: &[String]) -> Option<Vec<PathBuf>> {
    if patterns.is_empty() {
        return None;
    }
    patterns
        .iter()
        .map(|pattern| {
            let literal = pattern.contains('/')
                && !pattern.contains(['*', '?', '[', ']', '{', '}', '\\', ':'])
                && !pattern.starts_with(['!', '#', '/'])
                && pattern.trim() == pattern
                && pattern.split('/').all(|part| !matches!(part, "" | "." | ".."));
            literal.then(|| pattern.split('/').collect())
        })
        .collect()
}
