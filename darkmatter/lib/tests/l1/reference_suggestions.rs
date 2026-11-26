//! The host reference builder (`findings::reference`): which outcome a
//! resolution becomes under a permission callback, what the missing-document
//! finding carries, which directories its suggestions come from, and the
//! bounded repository basename search hosts offer after an explicit miss.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileResolutionContext, ResolutionFailure, to_forward_slash_string};
use darkmatter::findings::reference::{
    BasenameBounds, DocumentCheck, DocumentConstruct, ListingMemo, basename_matches, check_document,
};
use darkmatter::findings::{Diagnostic, MISSING_DOCUMENT_CODE};
use tempfile::TempDir;

const PROMPT: DocumentConstruct<'static> = DocumentConstruct { construct: "prompt:", noun: "prompt file", launch_dir: None };

fn write(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "x").unwrap();
}

fn slash(path: &Path) -> String {
    to_forward_slash_string(path)
}

/// A repository whose `^` references search the package, the package area,
/// then the repository, launched in the package.
struct Scoped {
    _dir: TempDir,
    repo: PathBuf,
    area: PathBuf,
    package: PathBuf,
}

impl Scoped {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let repo = dir.path().to_path_buf();
        let area = repo.join("area");
        let package = area.join("package");
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::create_dir_all(&package).unwrap();
        Self { _dir: dir, repo, area, package }
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(&self.package, None, HashMap::new())
            .with_repository_root(&self.repo)
            .with_package_area(&self.area)
            .with_package_root(&self.package)
    }
}

fn admit_all(_: &Path) -> Result<(), String> {
    Ok(())
}

fn check(raw: &str, context: &FileResolutionContext, memo: &mut ListingMemo) -> DocumentCheck<String> {
    check_document(&FileReference::new(raw).unwrap(), context, PROMPT, memo, admit_all)
}

fn missing(outcome: DocumentCheck<String>) -> Diagnostic {
    match outcome {
        DocumentCheck::Missing(finding) => *finding,
        other => panic!("expected a missing document, got {other:?}"),
    }
}

#[test]
fn a_no_match_is_the_missing_document_finding_with_its_searched_locations_in_resolver_order() {
    let fixture = Scoped::new();
    let finding = missing(check("^notes.md", &fixture.context(), &mut ListingMemo::new()));
    assert_eq!(finding.code, MISSING_DOCUMENT_CODE);
    let details = finding.reference_details.unwrap();
    assert_eq!(details.reference.as_deref(), Some("^notes.md"));
    assert_eq!(details.failure.as_deref(), Some("no-match"));
    assert_eq!(details.resolved, None);
    let expected: Vec<String> =
        [&fixture.package, &fixture.area, &fixture.repo].iter().map(|dir| slash(&dir.join("notes.md"))).collect();
    assert_eq!(details.searched, Some(expected));
    assert_eq!(finding.notes, ["searched 3 locations"]);
}

#[test]
fn suggestions_follow_the_search_order_and_stop_at_three() {
    let fixture = Scoped::new();
    // Alphabetically the area's names come first; the package is searched first.
    write(&fixture.area.join("note.md"));
    write(&fixture.area.join("notes.mdx"));
    write(&fixture.package.join("notez.md"));
    write(&fixture.repo.join("notes.m"));
    let finding = missing(check("^notes.md", &fixture.context(), &mut ListingMemo::new()));
    let suggestions = finding.reference_details.unwrap().suggestions.unwrap();
    assert_eq!(
        suggestions,
        [
            slash(&fixture.package.join("notez.md")),
            slash(&fixture.area.join("note.md")),
            slash(&fixture.area.join("notes.mdx")),
        ]
    );
}

#[test]
fn a_refused_candidate_ends_the_search_before_it_is_touched() {
    let fixture = Scoped::new();
    // The first candidate exists: probing it would have found it.
    write(&fixture.package.join("notes.md"));
    write(&fixture.area.join("notes.md"));
    let asked = RefCell::new(Vec::new());
    let refuse_package = |path: &Path| {
        asked.borrow_mut().push(path.to_path_buf());
        if path.starts_with(&fixture.package) { Err("denied".to_string()) } else { Ok(()) }
    };
    let mut memo = ListingMemo::new();
    let outcome =
        check_document(&FileReference::new("^notes.md").unwrap(), &fixture.context(), PROMPT, &mut memo, refuse_package);
    assert!(matches!(outcome, DocumentCheck::Refused(ref refusal) if refusal == "denied"), "{outcome:?}");
    assert_eq!(asked.into_inner(), [fixture.package.join("notes.md")]);
    assert_eq!(memo.listed(), 0, "a refusal lists nothing");
}

#[test]
fn a_refusal_after_a_missing_candidate_asks_about_nothing_later() {
    let fixture = Scoped::new();
    write(&fixture.repo.join("notes.md"));
    let asked = RefCell::new(Vec::new());
    let refuse_area = |path: &Path| {
        asked.borrow_mut().push(path.to_path_buf());
        if path.starts_with(&fixture.package) { Ok(()) } else { Err(path.to_path_buf()) }
    };
    let outcome = check_document(
        &FileReference::new("^notes.md").unwrap(),
        &fixture.context(),
        PROMPT,
        &mut ListingMemo::new(),
        refuse_area,
    );
    assert!(matches!(outcome, DocumentCheck::Refused(ref path) if *path == fixture.area.join("notes.md")), "{outcome:?}");
    assert_eq!(asked.into_inner(), [fixture.package.join("notes.md"), fixture.area.join("notes.md")]);
}

#[test]
fn a_refused_directory_is_never_listed_or_suggested_from() {
    let fixture = Scoped::new();
    write(&fixture.package.join("notez.md"));
    write(&fixture.area.join("note.md"));
    let refuse_area_listing = |path: &Path| if path == fixture.area { Err("denied".to_string()) } else { Ok(()) };
    let mut memo = ListingMemo::new();
    let outcome = check_document(
        &FileReference::new("^notes.md").unwrap(),
        &fixture.context(),
        PROMPT,
        &mut memo,
        refuse_area_listing,
    );
    let suggestions = missing(outcome).reference_details.unwrap().suggestions.unwrap();
    assert_eq!(suggestions, [slash(&fixture.package.join("notez.md"))]);
    assert_eq!(memo.listed(), 2, "the package and the repository, never the area");
}

#[test]
fn a_directory_several_references_search_is_listed_once() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("docs/plan.md"));
    let context = FileResolutionContext::new(dir.path());
    let mut memo = ListingMemo::new();
    missing(check("./docs/plam.md", &context, &mut memo));
    missing(check("./docs/../docs/plab.md", &context, &mut memo));
    assert_eq!(memo.listed(), 1);
}

#[test]
fn a_recursive_reference_with_two_matches_is_found_not_missing_or_ambiguous() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("a/deep/notes.md"));
    write(&dir.path().join("b/notes.md"));
    let context = FileResolutionContext::new(dir.path());
    match check("%./notes.md", &context, &mut ListingMemo::new()) {
        DocumentCheck::Found(path) => assert_eq!(path, dir.path().join("b/notes.md")),
        other => panic!("the most local match wins: {other:?}"),
    }
}

#[test]
fn a_recursive_miss_lists_its_search_roots_and_suggests_from_them() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("notez.md"));
    let context = FileResolutionContext::new(dir.path());
    let details = missing(check("%./notes.md", &context, &mut ListingMemo::new())).reference_details.unwrap();
    assert_eq!(details.resolved, None);
    assert_eq!(details.searched, Some(vec![slash(dir.path())]));
    assert_eq!(details.suggestions, Some(vec![slash(&dir.path().join("notez.md"))]));
}

#[test]
fn an_unresolvable_reference_keeps_its_own_failure() {
    let dir = TempDir::new().unwrap();
    let context = FileResolutionContext::new(dir.path());
    match check("{{DM_UNSET_VARIABLE}}/notes.md", &context, &mut ListingMemo::new()) {
        DocumentCheck::Failed(resolution) => {
            assert!(matches!(
                resolution.outcome(),
                biscuit_file::DetailedOutcome::Failed(ResolutionFailure::MissingContext)
            ));
            assert!(resolution.error().is_some());
        }
        other => panic!("expected the missing-context failure, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn an_io_failure_is_not_a_missing_document() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("file.md"));
    let context = FileResolutionContext::new(dir.path());
    match check("./file.md/child.md", &context, &mut ListingMemo::new()) {
        DocumentCheck::Failed(resolution) => {
            assert!(matches!(resolution.outcome(), biscuit_file::DetailedOutcome::Failed(ResolutionFailure::Io)));
        }
        other => panic!("expected the I/O failure, got {other:?}"),
    }
}

#[test]
fn a_literal_glob_miss_carries_the_resolver_glob_hint() {
    let dir = TempDir::new().unwrap();
    let context = FileResolutionContext::new(dir.path());
    let expected = ResolutionFailure::NoMatch.glob_hint("./*.md");
    assert!(expected.is_some());
    match check("./*.md", &context, &mut ListingMemo::new()) {
        DocumentCheck::Missing(finding) => assert_eq!(finding.hint.as_deref(), expected),
        // Windows refuses `*` in a file name, so the probe is an I/O failure there.
        DocumentCheck::Failed(_) if cfg!(windows) => {}
        other => panic!("expected a missing document, got {other:?}"),
    }
}

/// Whether `directory`'s volume opens one entry under another case.
fn case_insensitive(directory: &Path) -> bool {
    let probe = directory.join("CaseProbe");
    fs::write(&probe, "").unwrap();
    let insensitive = directory.join("caseprobe").exists();
    fs::remove_file(probe).unwrap();
    insensitive
}

#[test]
fn a_case_only_name_is_suggested_only_when_it_is_a_different_file() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("Notes.md"));
    let context = FileResolutionContext::new(dir.path());
    let outcome = check("./notes.md", &context, &mut ListingMemo::new());
    if case_insensitive(dir.path()) {
        // The volume opens `Notes.md` for `notes.md`: the reference resolves,
        // and nothing is suggested in place of the file it names.
        assert!(matches!(outcome, DocumentCheck::Found(_)), "{outcome:?}");
    } else {
        let suggestions = missing(outcome).reference_details.unwrap().suggestions.unwrap();
        assert_eq!(suggestions, [slash(&dir.path().join("Notes.md"))]);
    }
}

fn bounds(entry_budget: usize, max_matches: usize) -> BasenameBounds {
    BasenameBounds { entry_budget, max_matches }
}

fn keep_all(_: &OsStr) -> bool {
    true
}

#[test]
fn basename_matches_are_exact_sorted_and_capped() {
    let dir = TempDir::new().unwrap();
    for directory in ["z", "a", "c", "b"] {
        write(&dir.path().join(directory).join("access.md"));
    }
    write(&dir.path().join("d/ACCESS.md"));
    write(&dir.path().join("e/access.md.bak"));
    let found = basename_matches(dir.path(), OsStr::new("access.md"), bounds(1_000, 3), keep_all);
    assert_eq!(found, ["a/access.md", "b/access.md", "c/access.md"]);
    let all = basename_matches(dir.path(), OsStr::new("access.md"), bounds(1_000, 10), keep_all);
    assert_eq!(all, ["a/access.md", "b/access.md", "c/access.md", "z/access.md"]);
}

#[test]
fn the_entry_budget_counts_the_root_and_every_entry_in_path_order() {
    let dir = TempDir::new().unwrap();
    for directory in ["d0", "d1", "d2"] {
        write(&dir.path().join(directory).join("access.md"));
    }
    // Root, d0, d0/access.md, d1, d1/access.md: five items reach two matches.
    let five = basename_matches(dir.path(), OsStr::new("access.md"), bounds(5, 10), keep_all);
    assert_eq!(five, ["d0/access.md", "d1/access.md"]);
    let four = basename_matches(dir.path(), OsStr::new("access.md"), bounds(4, 10), keep_all);
    assert_eq!(four, ["d0/access.md"]);
}

#[test]
fn the_search_honors_gitignore_and_the_host_subtree_filter() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join(".git")).unwrap();
    fs::write(dir.path().join(".gitignore"), "ignored/\n").unwrap();
    for directory in ["visible", "ignored", "_drafts", "target"] {
        write(&dir.path().join(directory).join("access.md"));
    }
    let host_filter = |name: &OsStr| name != "target" && !name.to_string_lossy().starts_with('_');
    let found = basename_matches(dir.path(), OsStr::new("access.md"), bounds(1_000, 5), host_filter);
    assert_eq!(found, ["visible/access.md"]);
}

#[test]
fn an_unusable_root_or_empty_name_finds_nothing() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("access.md"));
    assert!(basename_matches(&dir.path().join("missing"), OsStr::new("access.md"), bounds(100, 5), keep_all).is_empty());
    assert!(basename_matches(dir.path(), OsStr::new(""), bounds(100, 5), keep_all).is_empty());
    assert!(basename_matches(dir.path(), OsStr::new("access.md"), bounds(100, 0), keep_all).is_empty());
}

#[cfg(unix)]
#[test]
fn the_search_never_follows_a_directory_symlink() {
    let repo = TempDir::new().unwrap();
    let external = TempDir::new().unwrap();
    write(&external.path().join("access.md"));
    std::os::unix::fs::symlink(external.path(), repo.path().join("linked")).unwrap();
    assert!(basename_matches(repo.path(), OsStr::new("access.md"), bounds(100, 5), keep_all).is_empty());
}
