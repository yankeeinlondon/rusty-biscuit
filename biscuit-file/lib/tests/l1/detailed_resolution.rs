//! Integration tests for the detailed resolver (`resolve_detailed`),
//! candidate plan (`candidate_plan`), probe dispositions, and root provenance.
//!
//! These exercise the Phase 3 machinery: the detailed outcome retains the
//! ordered candidate/root plan with per-candidate probe disposition on both
//! success and no-match, the legacy convenience projection agrees with it, and
//! I/O probe failures stay typed and identify the failing candidate rather than
//! collapsing into "not found".

use biscuit_test_harness::manifest_dir;

use std::fs;
use std::path::Path;

use biscuit_file::{
    DetailedOutcome, FileReference, FileReferenceError, FileResolutionContext, ProbeDisposition,
    ResolutionFailure, RootProvenance,
};
use tempfile::TempDir;

/// Initialize a fresh git repository at `path` using gix (no system git).
fn git_init(path: &Path) {
    gix::init(path).expect("git init failed");
}

/// Build a context anchored at `base`, supplying `repo` as the repository root.
fn ctx_with_repo(base: &Path, repo: &Path) -> FileResolutionContext {
    FileResolutionContext::new(base.to_path_buf()).with_repository_root(repo.to_path_buf())
}

fn candidate_paths(detailed: &biscuit_file::DetailedResolution) -> Vec<std::path::PathBuf> {
    detailed
        .candidates()
        .iter()
        .map(|c| c.candidate().path().to_path_buf())
        .collect()
}

/// Canonicalize into the spelling the resolver reports.
///
/// `std::fs::canonicalize` returns a `\\?\` verbatim path on Windows; the
/// resolver reduces every anchor to the legacy form before joining, so
/// expectations must be built in that same form.
fn canonical(path: &Path) -> std::path::PathBuf {
    dunce::canonicalize(path).expect("canonicalize")
}

#[test]
fn detailed_success_retains_ordered_candidates_and_provenance() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    git_init(&repo);

    // File exists in the base sub-directory, so the base-first search stops there.
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);
    fs::write(base.join("notes.md"), b"local").unwrap();

    let ctx = ctx_with_repo(&base, &repo);
    let detailed = FileReference::new("notes.md").unwrap().resolve_detailed(&ctx);

    // The winner is the source-local candidate.
    assert_eq!(
        detailed.matched_path(),
        Some(base.join("notes.md").as_path()),
        "implicit reference selects the source candidate first",
    );
    assert!(matches!(detailed.outcome(), DetailedOutcome::Matched(_)));
    assert_eq!(detailed.repository_root(), Some(repo.as_path()));
    assert!(detailed.error().is_none());

    // Resolution stops after the first match.
    let candidates = detailed.candidates();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].candidate().provenance(), RootProvenance::Source);
    assert_eq!(candidates[0].disposition(), ProbeDisposition::Matched);
    assert_eq!(candidates[0].candidate().path(), base.join("notes.md"));
}

#[test]
fn detailed_no_match_retains_candidates_and_repository_root() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    git_init(&repo);

    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);

    let ctx = ctx_with_repo(&base, &repo);
    let detailed = FileReference::new("missing.md")
        .unwrap()
        .resolve_detailed(&ctx);

    // NoMatch is a typed outcome, not a discarded Ok(None).
    assert!(
        matches!(
            detailed.outcome(),
            DetailedOutcome::Failed(ResolutionFailure::NoMatch)
        ),
        "no-match must be a typed Failed(NoMatch), got {:?}",
        detailed.outcome()
    );
    assert!(detailed.matched_path().is_none());
    assert!(detailed.error().is_none(), "NoMatch has no underlying error");
    assert_eq!(detailed.repository_root(), Some(repo.as_path()));

    // Both candidates were probed and both missed, in base-first order.
    let candidates = detailed.candidates();
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0].candidate().provenance(),
        RootProvenance::Source
    );
    assert_eq!(candidates[0].disposition(), ProbeDisposition::Missing);
    assert_eq!(candidates[1].candidate().provenance(), RootProvenance::Repository);
    assert_eq!(candidates[1].disposition(), ProbeDisposition::Missing);
}

#[test]
fn candidate_plan_matches_the_probed_order() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    git_init(&repo);

    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);

    let file_ref = FileReference::new("missing.md").unwrap();
    let ctx = ctx_with_repo(&base, &repo);

    let plan = file_ref.candidate_plan(&ctx).unwrap();
    let detailed = file_ref.resolve_detailed(&ctx);

    // The plan a consumer inspects is exactly what execution probes, in order.
    let plan_paths: Vec<_> = plan.iter().map(|c| c.path().to_path_buf()).collect();
    assert_eq!(plan_paths, candidate_paths(&detailed));
    assert_eq!(plan[0].provenance(), RootProvenance::Source);
    assert_eq!(plan[1].provenance(), RootProvenance::Repository);
}

#[test]
fn explicit_relative_plan_is_exactly_one_source_candidate() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    git_init(&repo);

    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);

    let ctx = ctx_with_repo(&base, &repo);
    let plan = FileReference::new("./x.md")
        .unwrap()
        .candidate_plan(&ctx)
        .unwrap();

    assert_eq!(plan.len(), 1, "explicit relative has no fallback");
    assert_eq!(plan[0].provenance(), RootProvenance::Source);
    assert_eq!(plan[0].path(), base.join("x.md"));
}

#[test]
fn rooted_magic_payloads_never_reach_planning_or_resolution() {
    let tmp = TempDir::new().unwrap();
    let base = canonical(tmp.path());
    let ctx = FileResolutionContext::new(&base)
        .with_repository_root(&base)
        .with_home_dir(&base);

    for raw in [
        "@//etc/hosts",
        "%@///etc/hosts",
        r"@C:\Windows\win.ini",
        r"%@/C:\Windows\win.ini",
        r"@\Windows\win.ini",
        r"%@/\\server\share\file.md",
    ] {
        let plan = FileReference::new(raw).and_then(|file_ref| file_ref.candidate_plan(&ctx));
        assert!(
            matches!(plan, Err(FileReferenceError::InvalidSyntax(_))),
            "candidate planning must reject rooted magic payload: {raw:?}",
        );

        let resolution =
            FileReference::new(raw).and_then(|file_ref| file_ref.resolve_in_context(&ctx));
        assert!(
            matches!(resolution, Err(FileReferenceError::InvalidSyntax(_))),
            "resolution must reject rooted magic payload: {raw:?}",
        );
    }
}

#[test]
fn legacy_projection_agrees_with_detailed_outcome() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    git_init(&repo);
    fs::write(repo.join("hit.md"), b"x").unwrap();
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);

    // Match case: convenience Ok(Some) equals the detailed matched path.
    let hit = FileReference::new("hit.md").unwrap();
    let ctx = ctx_with_repo(&base, &repo);
    let projected = hit.resolve_in_context(&ctx).unwrap();
    assert_eq!(
        projected.as_deref(),
        hit.resolve_detailed(&ctx).matched_path(),
    );

    // No-match case: convenience Ok(None), detailed Failed(NoMatch).
    let miss = FileReference::new("nope.md").unwrap();
    assert_eq!(miss.resolve_in_context(&ctx).unwrap(), None);
    assert!(matches!(
        miss.resolve_detailed(&ctx).outcome(),
        DetailedOutcome::Failed(ResolutionFailure::NoMatch)
    ));
}

#[test]
fn non_file_candidate_advances_the_search() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    git_init(&repo);

    // A directory named like the target sits at the base; the regular file sits
    // at the repository root. The directory must not win.
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    fs::create_dir_all(base.join("thing.md")).unwrap();
    let base = canonical(&base);
    fs::write(repo.join("thing.md"), b"real").unwrap();

    let ctx = ctx_with_repo(&base, &repo);
    let detailed = FileReference::new("thing.md")
        .unwrap()
        .resolve_detailed(&ctx);

    assert_eq!(detailed.matched_path(), Some(repo.join("thing.md").as_path()));
    let candidates = detailed.candidates();
    assert_eq!(
        candidates[0].candidate().provenance(),
        RootProvenance::Source
    );
    assert_eq!(
        candidates[0].disposition(),
        ProbeDisposition::NonFile,
        "the source directory candidate is a non-file that advances the search",
    );
    assert_eq!(candidates[1].candidate().provenance(), RootProvenance::Repository);
    assert_eq!(candidates[1].disposition(), ProbeDisposition::Matched);
}

#[cfg(unix)]
#[test]
fn io_probe_failure_stops_with_typed_error_identifying_candidate() {
    // Traversing *through* a regular file component yields ENOTDIR, which is a
    // non-`NotFound` I/O failure. It must stop the search with a typed
    // `FileReferenceError::Io` naming the candidate, not be misread as absence.
    let tmp = TempDir::new().unwrap();
    let base = canonical(tmp.path());

    // `blocker` is a regular file; the candidate path descends into it.
    fs::write(base.join("blocker"), b"not a directory").unwrap();

    // Anchor the repository root at the base so the implicit reference yields a
    // single candidate (`<base>/blocker/x.md`).
    let ctx = ctx_with_repo(&base, &base);
    let file_ref = FileReference::new("blocker/x.md").unwrap();
    let detailed = file_ref.resolve_detailed(&ctx);

    assert!(
        matches!(
            detailed.outcome(),
            DetailedOutcome::Failed(ResolutionFailure::Io)
        ),
        "an I/O probe failure must classify as Io, got {:?}",
        detailed.outcome()
    );

    let candidate_path = base.join("blocker/x.md");
    match detailed.error() {
        Some(FileReferenceError::Io { path, .. }) => {
            assert_eq!(path, &candidate_path, "the Io error identifies the candidate");
        }
        other => panic!("expected a typed Io error, got {other:?}"),
    }

    let last = detailed.candidates().last().expect("one candidate probed");
    assert!(
        matches!(last.disposition(), ProbeDisposition::Io(_)),
        "the failing candidate's disposition is Io, got {:?}",
        last.disposition(),
    );
    assert_eq!(last.candidate().path(), candidate_path);

    // The legacy convenience projection surfaces the same typed error, not
    // `Ok(None)`.
    let err = file_ref.resolve_in_context(&ctx).unwrap_err();
    assert!(matches!(err, FileReferenceError::Io { .. }));
}

#[cfg(unix)]
#[test]
fn direct_symlink_to_regular_file_is_selected() {
    use std::os::unix::fs::symlink;

    let tmp = TempDir::new().unwrap();
    let base = canonical(tmp.path());

    fs::write(base.join("target.md"), b"real").unwrap();
    symlink(base.join("target.md"), base.join("link.md")).unwrap();

    let ctx = ctx_with_repo(&base, &base);
    let detailed = FileReference::new("./link.md")
        .unwrap()
        .resolve_detailed(&ctx);

    // The symlink path itself is selected (metadata follows it to confirm it is
    // a regular file); the target path is not substituted.
    assert_eq!(detailed.matched_path(), Some(base.join("link.md").as_path()));
    assert_eq!(
        detailed.candidates()[0].disposition(),
        ProbeDisposition::Matched
    );
}

#[test]
fn supplied_repository_root_anchors_and_suppresses_ancestor_discovery() {
    // A supplied (e.g. linked-worktree) root is used verbatim: resolution must
    // not walk up to an enclosing git root. `outer` is a real repo; the
    // supplied root is `outer/linked`, which shadows a same-named file at
    // `outer`.
    let tmp = TempDir::new().unwrap();
    let outer = canonical(tmp.path());
    git_init(&outer);
    fs::write(outer.join("x.md"), b"outer").unwrap();

    let linked = outer.join("linked");
    fs::create_dir_all(&linked).unwrap();
    fs::write(linked.join("x.md"), b"linked").unwrap();
    let sub = linked.join("sub");
    fs::create_dir_all(&sub).unwrap();
    let sub = canonical(&sub);
    let linked = canonical(&linked);

    let ctx = ctx_with_repo(&sub, &linked);
    let detailed = FileReference::new("x.md").unwrap().resolve_detailed(&ctx);

    assert_eq!(
        detailed.matched_path(),
        Some(linked.join("x.md").as_path()),
        "the supplied root anchors resolution; the ancestor repo is not consulted",
    );
    assert_eq!(detailed.repository_root(), Some(linked.as_path()));
    // Only the supplied-root and source candidates exist -- never `outer`.
    for probed in detailed.candidates() {
        assert!(
            probed.candidate().path().starts_with(&linked),
            "no candidate may escape the supplied root: {:?}",
            probed.candidate().path(),
        );
    }
}

/// The explicit path performs no live git discovery: an implicit reference whose
/// base sits inside a real git worktree, but whose context supplies no
/// repository root, yields a single base-only candidate -- never the repository
/// candidate a `find_git_root` fallback would have injected (D2/D10).
#[test]
fn explicit_implicit_reference_does_not_discover_repository_root() {
    // The base is inside a real git repository (this crate's own worktree). The
    // legacy fallback discovered that root live; the explicit context must not.
    let base = manifest_dir!();
    let ctx = FileResolutionContext::new(&base);
    let plan = FileReference::new("Cargo.toml")
        .unwrap()
        .candidate_plan(&ctx)
        .unwrap();

    assert_eq!(
        plan.len(),
        1,
        "no repository root was supplied, so only the base candidate exists: {plan:?}",
    );
    assert_eq!(plan[0].provenance(), RootProvenance::Source);
    assert_eq!(plan[0].path(), base.join("Cargo.toml"));
}

#[test]
fn recursive_reference_records_search_roots_with_provenance() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    git_init(&repo);

    fs::create_dir_all(repo.join("a/b")).unwrap();
    fs::write(repo.join("a/b/deep.md"), b"deep").unwrap();
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);

    let ctx = ctx_with_repo(&base, &repo);
    let detailed = FileReference::new("%deep.md").unwrap().resolve_detailed(&ctx);

    assert_eq!(
        detailed.matched_path(),
        Some(repo.join("a/b/deep.md").as_path()),
        "recursive search finds the file under the repository root",
    );
    // The diagnostics-visible candidates for a recursive reference are its
    // search roots, marked SearchRoot, carrying provenance.
    let roots = detailed.candidates();
    assert!(roots
        .iter()
        .all(|r| matches!(r.disposition(), ProbeDisposition::SearchRoot)));
    assert!(
        roots
            .iter()
            .any(|r| r.candidate().provenance() == RootProvenance::Repository),
        "the repository root must appear as a recursive search root",
    );
}

#[test]
fn invalid_context_is_a_typed_missing_context_failure() {
    // A repository root that does not contain the base is rejected before any
    // probing, as a typed missing-context failure.
    let tmp = TempDir::new().unwrap();
    let root = canonical(tmp.path());
    let outside = root.join("outside");
    let repo = root.join("repo");
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(&repo).unwrap();

    let ctx = FileResolutionContext::new(outside).with_repository_root(repo);
    let detailed = FileReference::new("x.md").unwrap().resolve_detailed(&ctx);

    assert!(matches!(
        detailed.outcome(),
        DetailedOutcome::Failed(ResolutionFailure::MissingContext)
    ));
    assert!(matches!(
        detailed.error(),
        Some(FileReferenceError::RepositoryRootNotContainingSource { .. })
    ));
    assert!(
        detailed.candidates().is_empty(),
        "no candidate is probed when the context is invalid",
    );
}

#[test]
fn probed_paths_lists_missing_candidates_through_the_match_lexically() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    git_init(&repo);

    // `pkg/shared/` does not exist, so the source candidate is missing and the
    // repository candidate wins.
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);
    fs::create_dir_all(repo.join("shared")).unwrap();
    fs::write(repo.join("shared/types.yaml"), b"x").unwrap();

    let ctx = ctx_with_repo(&base, &repo);
    let detailed = FileReference::new("shared/../shared/types.yaml")
        .unwrap()
        .resolve_detailed(&ctx);

    let probed: Vec<_> = detailed.probed_paths().collect();
    assert_eq!(
        probed,
        vec![base.join("shared").join("types.yaml"), repo.join("shared").join("types.yaml")],
        "dot segments are collapsed without touching the absent folder",
    );
    assert_eq!(probed.last().map(|p| p.as_path()), detailed.matched_path());
}

#[test]
fn probed_paths_lists_every_candidate_of_a_no_match() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    git_init(&repo);
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);

    let ctx = ctx_with_repo(&base, &repo);
    let implicit = FileReference::new("missing.md").unwrap().resolve_detailed(&ctx);
    assert_eq!(
        implicit.probed_paths().collect::<Vec<_>>(),
        vec![base.join("missing.md"), repo.join("missing.md")],
    );

    let explicit = FileReference::new("./missing.md").unwrap().resolve_detailed(&ctx);
    assert_eq!(
        explicit.probed_paths().collect::<Vec<_>>(),
        vec![base.join("missing.md")],
        "an explicit relative reference probes its one location",
    );
}

#[test]
fn probed_paths_omits_recursive_search_roots() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    git_init(&repo);
    fs::create_dir_all(repo.join("a")).unwrap();
    fs::write(repo.join("a/deep.md"), b"deep").unwrap();

    let ctx = ctx_with_repo(&repo, &repo);
    let detailed = FileReference::new("%deep.md").unwrap().resolve_detailed(&ctx);

    assert!(detailed.matched_path().is_some());
    assert!(!detailed.candidates().is_empty());
    assert_eq!(detailed.probed_paths().count(), 0, "search roots are not file probes");
}

/// `resolve_detailed_admitted` asks about each candidate immediately before
/// probing it and stops at the first match, so a later candidate is never
/// asked about; the outcome equals `resolve_detailed`'s.
#[test]
fn admitted_resolution_asks_only_about_candidates_the_search_reaches() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);
    fs::write(base.join("notes.md"), b"local").unwrap();
    fs::write(repo.join("notes.md"), b"repo").unwrap();
    let ctx = ctx_with_repo(&base, &repo);
    let reference = FileReference::new("notes.md").unwrap();
    let mut asked = Vec::new();

    let detailed = reference
        .resolve_detailed_admitted(&ctx, |path| {
            asked.push(path.to_path_buf());
            Ok::<(), ()>(())
        })
        .expect("every location is admitted");

    assert_eq!(asked, vec![base.join("notes.md")]);
    assert_eq!(detailed.matched_path(), reference.resolve_detailed(&ctx).matched_path());
    assert_eq!(detailed.probed_paths().collect::<Vec<_>>(), asked);
}

/// A refusal stops the search before the refused candidate is probed and is
/// returned unchanged; a later candidate is neither asked about nor probed.
#[test]
fn admitted_resolution_stops_at_a_refused_candidate() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);
    fs::write(repo.join("notes.md"), b"repo").unwrap();
    let ctx = ctx_with_repo(&base, &repo);
    let reference = FileReference::new("notes.md").unwrap();
    let mut asked = Vec::new();

    let refusal = reference
        .resolve_detailed_admitted(&ctx, |path| {
            asked.push(path.to_path_buf());
            if path.starts_with(&base) { Ok(()) } else { Err("refused") }
        })
        .expect_err("the repository candidate is refused");

    assert_eq!(refusal, "refused");
    assert_eq!(asked, vec![base.join("notes.md"), repo.join("notes.md")]);
}

/// `present` declares the first candidate, which holds nothing on disk, a
/// match: the search stops there, recording it as the matched candidate,
/// and the later candidate holding a file is neither admitted nor probed.
#[test]
fn a_present_location_at_the_first_candidate_ends_the_search_there() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);
    fs::write(repo.join("notes.md"), b"repo").unwrap();
    let ctx = ctx_with_repo(&base, &repo);
    let reference = FileReference::new("notes.md").unwrap();
    let (mut admitted, mut offered) = (Vec::new(), Vec::new());

    let detailed = reference
        .resolve_detailed_with_presence(
            &ctx,
            |path| {
                admitted.push(path.to_path_buf());
                Ok::<(), ()>(())
            },
            |path| {
                offered.push(path.to_path_buf());
                path == base.join("notes.md")
            },
        )
        .expect("every location is admitted");

    let local = base.join("notes.md");
    assert_eq!(detailed.matched_path(), Some(local.as_path()));
    assert_eq!(admitted, vec![local.clone()]);
    assert_eq!(offered, vec![local.clone()]);
    assert_eq!(detailed.probed_paths().collect::<Vec<_>>(), vec![local]);
    let dispositions: Vec<_> = detailed.candidates().iter().map(|c| c.disposition()).collect();
    assert_eq!(dispositions, vec![ProbeDisposition::Matched]);
}

/// A file on disk wins at its own candidate without `present` being asked,
/// and a candidate `present` declines advances the search as a miss.
#[test]
fn a_file_on_disk_wins_at_its_candidate_and_a_declined_location_is_a_miss() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);
    fs::write(base.join("notes.md"), b"local").unwrap();
    fs::write(repo.join("other.md"), b"repo").unwrap();
    let ctx = ctx_with_repo(&base, &repo);
    let mut offered = Vec::new();

    let on_disk = FileReference::new("notes.md")
        .unwrap()
        .resolve_detailed_with_presence(&ctx, |_| Ok::<(), ()>(()), |path| {
            offered.push(path.to_path_buf());
            true
        })
        .unwrap();
    assert_eq!(on_disk.matched_path(), Some(base.join("notes.md").as_path()));
    assert!(offered.is_empty(), "{offered:?}");

    let declined = FileReference::new("other.md")
        .unwrap()
        .resolve_detailed_with_presence(&ctx, |_| Ok::<(), ()>(()), |path| {
            offered.push(path.to_path_buf());
            false
        })
        .unwrap();
    assert_eq!(declined.matched_path(), Some(repo.join("other.md").as_path()));
    assert_eq!(offered, vec![base.join("other.md")]);
}

/// A candidate that fails its containment check is never offered to
/// `present`: the typed boundary failure stands.
#[test]
fn a_candidate_failing_containment_is_never_offered_to_present() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);
    fs::create_dir_all(base.join("docs")).unwrap();
    let ctx = FileResolutionContext::new(base.join("docs")).with_base_dir(&base);
    let reference = FileReference::new("../../outside.md").unwrap();
    let static_error = reference.resolve_detailed(&ctx).error().map(ToString::to_string);
    let mut offered = Vec::new();

    let detailed = reference
        .resolve_detailed_with_presence(&ctx, |_| Ok::<(), ()>(()), |path| {
            offered.push(path.to_path_buf());
            true
        })
        .unwrap();

    assert!(offered.is_empty(), "{offered:?}");
    assert!(matches!(detailed.error(), Some(FileReferenceError::RelativeTreeEscape { .. })), "{:?}", detailed.outcome());
    assert_eq!(detailed.error().map(ToString::to_string), static_error);
}

/// Without a `present` predicate (`resolve_detailed_admitted`), a location
/// holding nothing on disk is a miss, exactly as `resolve_detailed` reports.
#[test]
fn admitted_resolution_without_presence_matches_the_plain_resolution() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);
    let ctx = ctx_with_repo(&base, &repo);
    let reference = FileReference::new("notes.md").unwrap();

    let admitted = reference.resolve_detailed_admitted(&ctx, |_| Ok::<(), ()>(())).unwrap();
    let plain = reference.resolve_detailed(&ctx);

    assert!(matches!(admitted.outcome(), DetailedOutcome::Failed(ResolutionFailure::NoMatch)));
    assert!(matches!(plain.outcome(), DetailedOutcome::Failed(ResolutionFailure::NoMatch)));
    assert_eq!(admitted.probed_paths().collect::<Vec<_>>(), plain.probed_paths().collect::<Vec<_>>());
    assert_eq!(admitted.probed_paths().collect::<Vec<_>>(), vec![base.join("notes.md"), repo.join("notes.md")]);
}
