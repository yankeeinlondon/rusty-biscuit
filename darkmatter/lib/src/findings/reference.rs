//! The missing-document finding: a confirmed no-match of a document-valued
//! host reference, built from biscuit-file's detailed resolution.
//!
//! The host supplies the construct it wrote (`prompt:`) and the noun a
//! person calls the target (`prompt file`); this module supplies everything
//! the resolver proved: the written reference, the directory it was resolved
//! against, the candidate it resolved to, every location it checked, and up
//! to three nearby files. A suggestion is only shown, never selected.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{DetailedOutcome, DetailedResolution, ResolutionFailure, to_forward_slash_string};

use super::{Diagnostic, MISSING_DOCUMENT_CODE, ReferenceDetails, Severity};

/// The most nearby files a finding suggests.
pub const MAX_SUGGESTIONS: usize = 3;

/// The entries of each directory a suggestion search listed, shared across
/// one invocation so a directory several references search is read once.
#[derive(Debug, Default)]
pub struct ListingMemo {
    listings: HashMap<PathBuf, Vec<String>>,
}

impl ListingMemo {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        Self::default()
    }

    /// The file names in `directory`, read at most once; empty when it
    /// cannot be read.
    pub fn file_names(&mut self, directory: &Path) -> &[String] {
        self.listings.entry(directory.to_path_buf()).or_insert_with(|| {
            let mut names: Vec<String> = std::fs::read_dir(directory)
                .map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
                        .filter_map(|entry| entry.file_name().into_string().ok())
                        .collect()
                })
                .unwrap_or_default();
            names.sort();
            names
        })
    }

    /// How many directories have been listed.
    pub fn listed(&self) -> usize {
        self.listings.len()
    }
}

/// What a host calls the reference a finding is about.
#[derive(Debug, Clone, Copy)]
pub struct DocumentConstruct<'a> {
    /// The authored construct, such as `prompt:`.
    pub construct: &'a str,
    /// What a person calls its target, such as `prompt file`.
    pub noun: &'a str,
    /// The launch directory: a path the hint names is written relative to
    /// it, with `/`, when it lies under it. The record's path fields stay
    /// absolute.
    pub launch_dir: Option<&'a Path>,
}

/// The [`MISSING_DOCUMENT_CODE`] error for `resolution`, or `None` unless
/// its outcome is a confirmed no-match: an ambiguous, refused, invalid, or
/// unreadable reference keeps its own typed failure.
pub fn missing_document(
    construct: DocumentConstruct<'_>,
    resolution: &DetailedResolution,
    memo: &mut ListingMemo,
) -> Option<Diagnostic> {
    if !matches!(resolution.outcome(), DetailedOutcome::Failed(ResolutionFailure::NoMatch)) {
        return None;
    }
    let searched: Vec<PathBuf> = resolution.probed_paths().collect();
    let resolved = (searched.len() == 1).then(|| searched[0].clone());
    let base = resolution.cwd();
    let suggestions = suggestions(resolution.raw(), &searched, memo);

    let hint = match (&resolved, resolution.glob_hint()) {
        (_, Some(glob)) => glob.to_string(),
        (Some(path), None) => format!(
            "create {} before a real run, or correct the reference",
            shown(path, construct.launch_dir)
        ),
        (None, None) => "correct the reference, or create the file in one of the searched locations".to_string(),
    };
    let mut finding = Diagnostic::actionable(
        MISSING_DOCUMENT_CODE,
        Severity::Error,
        format!("the {} does not exist", construct.noun),
        hint,
    )
    .with_reference_details(ReferenceDetails {
        reason: Some("file not found".to_string()),
        construct: Some(construct.construct.to_string()),
        reference: Some(resolution.raw().to_string()),
        failure: Some("no-match".to_string()),
        included_via: Some(Vec::new()),
        base: Some(to_forward_slash_string(base)),
        resolved: resolved.as_deref().map(to_forward_slash_string),
        searched: (searched.len() > 1).then(|| searched.iter().map(|path| to_forward_slash_string(path)).collect()),
        suggestions: (!suggestions.is_empty()).then_some(suggestions),
    });
    if resolved.is_none() && searched.len() > 1 {
        finding = finding.with_note(format!("searched {} locations", searched.len()));
    }
    Some(finding)
}

fn shown(path: &Path, launch_dir: Option<&Path>) -> String {
    match launch_dir.and_then(|base| path.strip_prefix(base).ok()) {
        Some(relative) if !relative.as_os_str().is_empty() => to_forward_slash_string(relative),
        _ => to_forward_slash_string(path),
    }
}

/// Up to [`MAX_SUGGESTIONS`] existing files near the reference's file name,
/// drawn from the directories the resolver searched, in search order. A
/// name that differs from the reference's only by case is never offered.
fn suggestions(raw: &str, searched: &[PathBuf], memo: &mut ListingMemo) -> Vec<String> {
    let wanted = Path::new(raw).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    if wanted.is_empty() {
        return Vec::new();
    }
    let mut directories: Vec<PathBuf> = Vec::new();
    for path in searched {
        if let Some(parent) = path.parent()
            && !directories.iter().any(|held| held == parent)
        {
            directories.push(parent.to_path_buf());
        }
    }
    let mut found = Vec::new();
    for directory in directories {
        let names: Vec<String> = memo
            .file_names(&directory)
            .iter()
            .filter(|name| !name.eq_ignore_ascii_case(&wanted))
            .cloned()
            .collect();
        for name in crate::catalog::suggest_strings(&names, &wanted, MAX_SUGGESTIONS) {
            if found.len() == MAX_SUGGESTIONS {
                return found;
            }
            found.push(to_forward_slash_string(&directory.join(name)));
        }
    }
    found
}
