//! The missing-document finding: a confirmed no-match of a document-valued
//! host reference, built from biscuit-file's detailed resolution.
//!
//! The host supplies the construct it wrote (`prompt:`) and the noun a
//! person calls the target (`prompt file`); this module supplies everything
//! the resolver proved: the written reference, the directory it was resolved
//! against, the candidate it resolved to, every location it checked, and up
//! to three nearby files. A suggestion is only shown, never selected.
//!
//! [`check_document`] is the host entry point: it resolves through the
//! host's permission callback, so a refused location ends the search as a
//! typed refusal and is never probed or listed, and only a confirmed
//! no-match becomes the finding. [`basename_matches`] is the bounded
//! repository-wide search a host offers when an explicit reference missed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{
    DetailedOutcome, DetailedResolution, FileReference, FileResolutionContext, PathIdentity, ProbeDisposition,
    ResolutionFailure, to_forward_slash_string,
};

use super::{Diagnostic, MISSING_DOCUMENT_CODE, ReferenceDetails, Severity};

mod basename;

pub use basename::{BasenameBounds, basename_matches};

/// The most nearby files a finding suggests.
pub const MAX_SUGGESTIONS: usize = 3;

/// The file entries of each directory a suggestion search listed, keyed by
/// the directory's [`PathIdentity`] and shared across one invocation, so a
/// directory several references search is read once. A directory is listed
/// only after the caller's permission callback admitted it.
#[derive(Debug, Default)]
pub struct ListingMemo {
    listings: HashMap<PathIdentity, Vec<String>>,
}

impl ListingMemo {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        Self::default()
    }

    /// The file names in `directory`, sorted, read at most once; empty when
    /// it cannot be read.
    pub fn file_names(&mut self, directory: &Path) -> &[String] {
        self.listings.entry(PathIdentity::new(directory)).or_insert_with(|| {
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

/// How a document-valued host reference resolved under a permission
/// callback. Only [`Self::NoMatch`] becomes a missing-document finding;
/// every other failure keeps its own typed cause.
#[derive(Debug)]
pub enum DocumentResolution<E> {
    /// The reference names this file.
    Found(PathBuf),
    /// No candidate holds a file: the evidence a missing-document finding
    /// and a host's typed no-match are built from.
    NoMatch(Box<DetailedResolution>),
    /// The callback refused a location. The search stopped there: that
    /// location was not touched, and no later one was asked about.
    Refused(E),
    /// An invalid reference, a missing context anchor, an I/O failure, or a
    /// remote URL, with its typed error in [`DetailedResolution::error`].
    Failed(Box<DetailedResolution>),
}

/// [`DocumentResolution`] with the no-match already turned into its
/// [`MISSING_DOCUMENT_CODE`] finding.
#[derive(Debug)]
pub enum DocumentCheck<E> {
    /// The reference names this file.
    Found(PathBuf),
    /// The missing-document finding.
    Missing(Box<Diagnostic>),
    /// As [`DocumentResolution::Refused`].
    Refused(E),
    /// As [`DocumentResolution::Failed`].
    Failed(Box<DetailedResolution>),
}

/// Resolves `reference` in `context` through
/// [`FileReference::resolve_detailed_admitted`], asking `admit` about each
/// location immediately before it is touched.
///
/// A recursive (`%`) reference takes the most local match, so two files it
/// could name are a [`DocumentResolution::Found`], never an ambiguity.
pub fn resolve_document<E>(
    reference: &FileReference,
    context: &FileResolutionContext,
    admit: impl FnMut(&Path) -> Result<(), E>,
) -> DocumentResolution<E> {
    let resolution = match reference.resolve_detailed_admitted(context, admit) {
        Ok(resolution) => resolution,
        Err(refusal) => return DocumentResolution::Refused(refusal),
    };
    match resolution.outcome() {
        DetailedOutcome::Matched(path) => DocumentResolution::Found(path.clone()),
        DetailedOutcome::Failed(ResolutionFailure::NoMatch) => DocumentResolution::NoMatch(Box::new(resolution)),
        DetailedOutcome::Failed(_) => DocumentResolution::Failed(Box::new(resolution)),
    }
}

/// [`resolve_document`], then the missing-document finding for a no-match.
/// The same `admit` decides which searched directories the suggestion
/// search may list, so a refused directory is never read or suggested from.
pub fn check_document<E>(
    reference: &FileReference,
    context: &FileResolutionContext,
    construct: DocumentConstruct<'_>,
    memo: &mut ListingMemo,
    mut admit: impl FnMut(&Path) -> Result<(), E>,
) -> DocumentCheck<E> {
    match resolve_document(reference, context, &mut admit) {
        DocumentResolution::Found(path) => DocumentCheck::Found(path),
        DocumentResolution::NoMatch(resolution) => {
            let finding = missing_document_admitted(construct, &resolution, memo, |directory| admit(directory).is_ok());
            DocumentCheck::Missing(Box::new(finding.expect("a no-match always builds its finding")))
        }
        DocumentResolution::Refused(refusal) => DocumentCheck::Refused(refusal),
        DocumentResolution::Failed(resolution) => DocumentCheck::Failed(resolution),
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
/// unreadable reference keeps its own typed failure. Every searched
/// directory may be listed for suggestions; [`check_document`] lists only
/// the ones its callback admits.
pub fn missing_document(
    construct: DocumentConstruct<'_>,
    resolution: &DetailedResolution,
    memo: &mut ListingMemo,
) -> Option<Diagnostic> {
    missing_document_admitted(construct, resolution, memo, |_| true)
}

fn missing_document_admitted(
    construct: DocumentConstruct<'_>,
    resolution: &DetailedResolution,
    memo: &mut ListingMemo,
    may_list: impl FnMut(&Path) -> bool,
) -> Option<Diagnostic> {
    if !matches!(resolution.outcome(), DetailedOutcome::Failed(ResolutionFailure::NoMatch)) {
        return None;
    }
    let locations = searched_locations(resolution);
    let resolved = match locations.as_slice() {
        [Location::File(path)] => Some(path.clone()),
        _ => None,
    };
    let searched: Vec<PathBuf> = locations.iter().map(|location| location.path().to_path_buf()).collect();
    let base = resolution.cwd();
    let suggestions = suggestions(resolution.raw(), &locations, memo, may_list);

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
        reports_inclusion: true,
        base: Some(to_forward_slash_string(base)),
        resolved: resolved.as_deref().map(to_forward_slash_string),
        searched: (resolved.is_none() && !searched.is_empty())
            .then(|| searched.iter().map(|path| to_forward_slash_string(path)).collect()),
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

/// A location the resolver searched, in its order.
enum Location {
    /// A direct candidate probed as a file.
    File(PathBuf),
    /// A recursive (`%`) search root.
    Root(PathBuf),
}

impl Location {
    fn path(&self) -> &Path {
        match self {
            Self::File(path) | Self::Root(path) => path,
        }
    }

    /// The directory a suggestion search lists for this location.
    fn directory(&self) -> Option<&Path> {
        match self {
            Self::File(path) => path.parent(),
            Self::Root(path) => Some(path),
        }
    }
}

/// Every candidate of `resolution` in resolver order: the file probes as
/// [`DetailedResolution::probed_paths`] spells them, and the search roots
/// made absolute against its `cwd`.
fn searched_locations(resolution: &DetailedResolution) -> Vec<Location> {
    let mut files = resolution.probed_paths();
    resolution
        .candidates()
        .iter()
        .filter_map(|probed| match probed.disposition() {
            ProbeDisposition::SearchRoot => Some(Location::Root(resolution.cwd().join(probed.candidate().path()))),
            _ => files.next().map(Location::File),
        })
        .collect()
}

/// Up to [`MAX_SUGGESTIONS`] existing files near the reference's file name,
/// drawn from the directories the resolver searched, in search order, each
/// listed only when `may_list` admits it. A name that differs from the
/// reference's only by case is offered unless it is the same directory
/// entry as the wanted path.
fn suggestions(
    raw: &str,
    locations: &[Location],
    memo: &mut ListingMemo,
    mut may_list: impl FnMut(&Path) -> bool,
) -> Vec<String> {
    let wanted = Path::new(raw).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    if wanted.is_empty() {
        return Vec::new();
    }
    let mut directories: Vec<PathIdentity> = Vec::new();
    let mut found = Vec::new();
    for directory in locations.iter().filter_map(Location::directory) {
        let identity = PathIdentity::new(directory);
        if directories.contains(&identity) {
            continue;
        }
        directories.push(identity);
        if !may_list(directory) {
            continue;
        }
        let listed = memo.file_names(directory);
        let names: Vec<String> = listed
            .iter()
            .filter(|name| **name != wanted && !names_wanted_entry(directory, listed, name, &wanted))
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

/// Whether `name`, listed in `directory`, is the directory entry the wanted
/// name reaches on this volume: a case-only spelling of `wanted`, no entry
/// spelled exactly `wanted` beside it (two listed entries are two files,
/// even hard-linked), and both spellings opening the same object, which a
/// case-insensitive volume (APFS, NTFS) does and a case-sensitive one never
/// does.
fn names_wanted_entry(directory: &Path, listed: &[String], name: &str, wanted: &str) -> bool {
    use sniff::filesystem::query::FileIdentity;
    if name.to_lowercase() != wanted.to_lowercase() || listed.iter().any(|entry| entry == wanted) {
        return false;
    }
    match (
        FileIdentity::of_path(&directory.join(name), false),
        FileIdentity::of_path(&directory.join(wanted), false),
    ) {
        (Ok(listed), Ok(reached)) => listed == reached,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether `directory`'s volume opens one entry under another case.
    fn case_insensitive(directory: &Path) -> bool {
        let probe = directory.join("CaseProbe");
        std::fs::write(&probe, "").unwrap();
        let insensitive = directory.join("caseprobe").exists();
        std::fs::remove_file(probe).unwrap();
        insensitive
    }

    #[test]
    fn a_case_only_name_is_the_wanted_entry_only_where_the_volume_opens_it() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("Notes.md"), "").unwrap();
        let listed = vec!["Notes.md".to_string()];
        let same = names_wanted_entry(dir.path(), &listed, "Notes.md", "notes.md");
        assert_eq!(same, case_insensitive(dir.path()));
        // An unrelated name is never the wanted entry.
        assert!(!names_wanted_entry(dir.path(), &listed, "Notes.md", "other.md"));
    }

    #[test]
    fn two_listed_spellings_are_two_entries_even_when_linked() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("Notes.md"), "").unwrap();
        if case_insensitive(dir.path()) || std::fs::hard_link(dir.path().join("Notes.md"), dir.path().join("notes.md")).is_err() {
            return;
        }
        let listed = vec!["Notes.md".to_string(), "notes.md".to_string()];
        assert!(!names_wanted_entry(dir.path(), &listed, "Notes.md", "notes.md"));
    }
}
