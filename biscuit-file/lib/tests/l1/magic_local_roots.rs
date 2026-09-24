//! Local-before-home ordering for `@` (magic) references (R1–R3).
//!
//! These exercise the request-scoped launch `@` scope and the tiered `@`
//! root chain: the launch tree resolves before every home-based root,
//! whether or not the launch directory sits below `$HOME`; the launch
//! directory itself is the local root when no repository exists; the launch
//! scope survives source derivation; and completion enumerates the same
//! ordered chain execution probes.
//!
//! The synthetic cases build contexts from `from_snapshot` with
//! platform-literal absolute paths, so they need no filesystem, no ambient
//! CWD, and no ambient HOME. The first-match cases stage real files under a
//! `TempDir` to prove the ordered plan actually changes the winner.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{
    CandidatePlanOrder, CompletionEntryForm, FileReference, FileResolutionContext, MagicPathTier,
    PathPosition, RootProvenance,
};
use tempfile::TempDir;

/// An absolute host path built from a POSIX-style `tail`.
///
/// Windows has no drive-less absolute path (`/a/b` is rooted but not
/// absolute), so the literal root is selected per platform; the synthetic
/// layouts below then exist identically on every OS.
fn abs(tail: &str) -> PathBuf {
    #[cfg(windows)]
    let root = Path::new(r"C:\");
    #[cfg(not(windows))]
    let root = Path::new("/");
    root.join(tail)
}

/// A synthetic request context: no ambient reads, no filesystem.
fn snapshot(base_dir: &Path, home: &Path) -> FileResolutionContext {
    FileResolutionContext::from_snapshot(base_dir, Some(home.to_path_buf()), HashMap::new())
}

fn plan_paths(reference: &str, ctx: &FileResolutionContext) -> Vec<PathBuf> {
    FileReference::new(reference)
        .unwrap()
        .candidate_plan(ctx)
        .unwrap()
        .iter()
        .map(|candidate| candidate.path().to_path_buf())
        .collect()
}

fn plan_provenance(reference: &str, ctx: &FileResolutionContext) -> Vec<RootProvenance> {
    FileReference::new(reference)
        .unwrap()
        .candidate_plan(ctx)
        .unwrap()
        .iter()
        .map(|candidate| candidate.provenance())
        .collect()
}

/// R2 headline order: every anchor present, one local and one user root at
/// each registration position. No user-tier candidate may precede any
/// local-tier candidate, and `PathPosition` keeps its meaning inside its
/// tier.
#[test]
fn tier_order_with_every_anchor_present() {
    let home = abs("h");
    let repo = abs("h/config/sh");
    let area = abs("h/config/sh/claudine");
    let package = abs("h/config/sh/claudine/lib");
    let launch = package.join("src");
    let local_pre = repo.join("local-pre");
    let local_app = repo.join("local-app");
    let user_pre = abs("opt/user-pre");
    let user_app = abs("opt/user-app");
    let ctx = snapshot(&launch, &home)
        .with_repository_root(&repo)
        .with_package_area(&area)
        .with_package_root(&package)
        .add_magic_path(&local_pre, PathPosition::Start)
        .add_magic_path(&user_pre, PathPosition::Start)
        .add_magic_path(&local_app, PathPosition::End)
        .add_magic_path(&user_app, PathPosition::End);

    let plan = FileReference::new("@x.md").unwrap().candidate_plan(&ctx).unwrap();
    let expected = vec![
        (local_pre.join("x.md"), RootProvenance::Magic),
        (package.join("x.md"), RootProvenance::PackageRoot),
        (area.join("x.md"), RootProvenance::PackageArea),
        (repo.join("x.md"), RootProvenance::Repository),
        (local_app.join("x.md"), RootProvenance::Magic),
        (user_pre.join("x.md"), RootProvenance::Magic),
        (home.join("x.md"), RootProvenance::Home),
        (user_app.join("x.md"), RootProvenance::Magic),
    ];
    let actual: Vec<(PathBuf, RootProvenance)> = plan
        .iter()
        .map(|c| (c.path().to_path_buf(), c.provenance()))
        .collect();
    assert_eq!(actual, expected);
}

/// The reported case: a repository nested inside `$HOME`. The repository
/// root and its local configured roots all precede the home candidate.
#[test]
fn repository_nested_in_home_precedes_home() {
    let home = abs("h");
    let repo = abs("h/config/sh");
    let ctx = snapshot(&repo, &home)
        .with_repository_root(&repo)
        .add_magic_path(repo.join(".claudine/prompts"), PathPosition::Start)
        .add_magic_path(home.join(".claudine/prompts"), PathPosition::Start);

    assert_eq!(
        plan_paths("@x.md", &ctx),
        vec![
            repo.join(".claudine/prompts/x.md"),
            repo.join("x.md"),
            home.join(".claudine/prompts/x.md"),
            home.join("x.md"),
        ],
        "a configured root inside the repository is local; one under home is user",
    );
}

/// Defect 4: with no repository, the launch directory itself is the local
/// root — as a first-class `LocalRoot`-provenance candidate — and precedes
/// the home tier.
#[test]
fn no_repository_local_root_is_the_request_directory() {
    let home = abs("h");
    let launch = abs("h/scratch");
    let ctx = snapshot(&launch, &home)
        .add_magic_path(home.join(".claudine/prompts"), PathPosition::Start);

    assert_eq!(
        plan_paths("@x.md", &ctx),
        vec![
            launch.join("x.md"),
            home.join(".claudine/prompts/x.md"),
            home.join("x.md"),
        ],
        "the launch directory is searched before every home-based root",
    );
    assert_eq!(
        plan_provenance("@x.md", &ctx),
        vec![
            RootProvenance::LocalRoot,
            RootProvenance::Magic,
            RootProvenance::Home,
        ],
    );
}

/// R2 containment rule: a configured root inside the local root is local;
/// `/h/.claudine` (under home but outside the local tree) and an external
/// root such as `/opt/configs` are user tier.
#[test]
fn tier_is_decided_by_containment_in_the_local_root() {
    let home = abs("h");
    let repo = abs("h/config/sh");
    let ctx = snapshot(&repo, &home)
        .with_repository_root(&repo)
        .add_magic_path(repo.join(".claudine"), PathPosition::End)
        .add_magic_path(home.join(".claudine"), PathPosition::Start)
        .add_magic_path(abs("opt/configs"), PathPosition::Start);

    assert_eq!(
        plan_paths("@x.md", &ctx),
        vec![
            repo.join("x.md"),
            repo.join(".claudine/x.md"),
            home.join(".claudine/x.md"),
            abs("opt/configs/x.md"),
            home.join("x.md"),
        ],
        "`<repo>/.claudine` is local; `<home>/.claudine` and `/opt/configs` are user",
    );
}

/// Launch directory equal to `$HOME`, no repository: containment would put
/// the home prompt directory in the local tier, so the explicit user-tier
/// override must keep it after the local files (ruling 1).
#[test]
fn launch_equals_home_user_override_wins_over_inference() {
    let home = abs("h");
    let ctx = snapshot(&home, &home)
        .add_magic_path(home.join("prompts"), PathPosition::Start)
        .add_magic_path_with_tier(
            home.join(".claudine/prompts"),
            PathPosition::Start,
            MagicPathTier::User,
        )
        .add_magic_path_with_tier(home.join(".claudine"), PathPosition::End, MagicPathTier::User);

    assert_eq!(
        plan_paths("@x.md", &ctx),
        vec![
            home.join("prompts/x.md"),
            home.join("x.md"),
            home.join(".claudine/prompts/x.md"),
            home.join(".claudine/x.md"),
        ],
        "inferred roots inside the local root stay local; explicit user roots follow",
    );
    assert_eq!(
        plan_provenance("@x.md", &ctx),
        vec![
            RootProvenance::Magic,
            RootProvenance::LocalRoot,
            RootProvenance::Magic,
            RootProvenance::Magic,
        ],
        "the intrinsic home root dedupes into the LocalRoot entry",
    );
}

/// `$HOME` itself is a repository: same overlap, with the local root
/// carrying `Repository` provenance.
#[test]
fn repository_equals_home_user_override_wins_over_inference() {
    let home = abs("h");
    let ctx = snapshot(&home, &home)
        .with_repository_root(&home)
        .add_magic_path(home.join("prompts"), PathPosition::Start)
        .add_magic_path_with_tier(
            home.join(".claudine/prompts"),
            PathPosition::Start,
            MagicPathTier::User,
        );

    assert_eq!(
        plan_paths("@x.md", &ctx),
        vec![
            home.join("prompts/x.md"),
            home.join("x.md"),
            home.join(".claudine/prompts/x.md"),
        ],
    );
    assert_eq!(
        plan_provenance("@x.md", &ctx),
        vec![RootProvenance::Magic, RootProvenance::Repository, RootProvenance::Magic],
    );
}

/// Ruling 2: a context derived for a trusted external source keeps the
/// launch `@` scope, while `./`-relative and bare references re-anchor on
/// the external source directory.
#[test]
fn trusted_external_source_keeps_the_launch_scope() {
    let home = abs("h");
    let launch = abs("h/scratch");
    let external_prompt = abs("h/.claudine/prompts/c.md");
    let ctx = snapshot(&launch, &home);
    let derived = ctx.for_trusted_external_source(&external_prompt);

    assert_eq!(
        plan_paths("@x.md", &derived),
        vec![launch.join("x.md"), home.join("x.md")],
        "nested `@` searches the launch tree first",
    );
    assert_eq!(
        plan_provenance("@x.md", &derived),
        vec![RootProvenance::LocalRoot, RootProvenance::Home],
    );
    assert_eq!(
        plan_paths("./x.md", &derived),
        vec![abs("h/.claudine/prompts/x.md")],
        "explicit-relative references keep their source anchor",
    );
    assert_eq!(
        plan_provenance("./x.md", &derived),
        vec![RootProvenance::Source],
    );
    assert_eq!(
        plan_paths("x.md", &derived),
        vec![abs("h/.claudine/prompts/x.md")],
        "bare implicit references keep their source anchor",
    );
    assert_eq!(plan_provenance("x.md", &derived), vec![RootProvenance::Source]);
}

/// A relative configured root resolves — and is tier-classified — against
/// the captured request directory, even on a context derived for a source
/// with a different authoring base.
#[test]
fn relative_configured_root_anchors_to_the_captured_request_directory() {
    let home = abs("h");
    let launch = abs("h/scratch");
    let ctx = snapshot(&launch, &home).add_magic_path("prompts", PathPosition::Start);

    assert_eq!(
        plan_paths("@x.md", &ctx),
        vec![launch.join("prompts/x.md"), launch.join("x.md"), home.join("x.md")],
        "the relative root joins onto the request directory and is local-tier",
    );

    let derived = ctx.for_trusted_external_source(abs("h/.claudine/prompts/c.md"));
    assert_eq!(
        plan_paths("@x.md", &derived),
        vec![launch.join("prompts/x.md"), launch.join("x.md"), home.join("x.md")],
        "derivation must not re-anchor the relative root onto the source base",
    );
}

/// The ambient `resolve_from(base)` form interprets a relative configured
/// root against `base` — not the process working directory (R2 behavior
/// change). The assertion pins the exact absolute winner, so the previous
/// CWD-relative probing cannot satisfy it.
#[test]
fn ambient_resolve_from_interprets_relative_roots_against_base() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("scratch");
    fs::create_dir_all(base.join("prompts")).unwrap();
    fs::write(base.join("prompts/x.md"), b"local").unwrap();

    let resolved = FileReference::new("@x.md")
        .unwrap()
        .add_magic_path("prompts", PathPosition::Start)
        .resolve_from(&base)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(base.join("prompts/x.md").as_path()),
        "a relative configured root follows the resolve_from base",
    );
}

/// Recursive `%@` traversal walks the same chain: local roots before user
/// roots, and a configured root equal to an intrinsic root is searched once
/// with the first-seen (configured) provenance.
#[test]
fn recursive_magic_walks_local_roots_once_in_tier_order() {
    let home = abs("h");
    let repo = abs("h/config/sh");
    let ctx = snapshot(&repo, &home)
        .with_repository_root(&repo)
        .add_magic_path(&repo, PathPosition::Start)
        .add_magic_path(abs("opt/user"), PathPosition::Start);

    let plan = FileReference::new("%@x.md").unwrap().candidate_plan(&ctx).unwrap();
    let roots: Vec<(PathBuf, RootProvenance)> = plan
        .iter()
        .map(|c| (c.path().to_path_buf(), c.provenance()))
        .collect();
    assert_eq!(
        roots,
        vec![
            (repo.clone(), RootProvenance::Magic),
            (abs("opt/user"), RootProvenance::Magic),
            (home.clone(), RootProvenance::Home),
        ],
        "the repository root is searched once, keeping its configured provenance",
    );
}

/// R3: for every supported completion entry form, the roots completion
/// enumerates for a partial token equal the candidate plan's ordered roots
/// for the corresponding complete token — including configured local and
/// user tiers. The `&`/`^` forms validate repository containment through a
/// real canonicalization, so the layout uses real (empty) directories.
#[test]
fn completion_roots_match_the_chain_for_every_entry_form() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let repo = home.join("config/sh");
    let launch = repo.join("docs");
    fs::create_dir_all(&launch).unwrap();
    let ctx = FileResolutionContext::from_snapshot(&launch, Some(home.clone()), HashMap::new())
        .with_repository_root(&repo)
        .add_magic_path(repo.join("prompts"), PathPosition::Start)
        .add_magic_path(home.join(".claudine"), PathPosition::End);

    let cases = [
        ("@docs/pl", "@docs/plan.md", CompletionEntryForm::Magic),
        ("&docs/pl", "&docs/plan.md", CompletionEntryForm::RepositoryRoot),
        ("^docs/pl", "^docs/plan.md", CompletionEntryForm::RepositoryScoped),
        ("docs/pl", "docs/plan.md", CompletionEntryForm::ImplicitRelative),
    ];
    for (token, executable, form) in cases {
        let completion = FileReference::complete_partial_in_context(token, &ctx)
            .unwrap()
            .unwrap_or_else(|| panic!("supported form for `{token}`"));
        let plan = FileReference::new(executable).unwrap().candidate_plan(&ctx).unwrap();
        let expected: Vec<PathBuf> = plan
            .iter()
            .map(|candidate| candidate.path().parent().unwrap().to_path_buf())
            .collect();
        assert_eq!(completion.entry_form(), form, "form for `{token}`");
        assert_eq!(completion.roots(), expected.as_slice(), "roots for `{token}`");
    }
}

/// R3 parity for the no-repository layout: completion's local root is the
/// launch directory, matching the plan's `LocalRoot` candidate.
#[test]
fn completion_without_repository_enumerates_the_launch_directory_first() {
    let home = abs("h");
    let launch = abs("h/scratch");
    let ctx = snapshot(&launch, &home)
        .add_magic_path(home.join(".claudine/prompts"), PathPosition::Start);

    let completion = FileReference::complete_partial_in_context("@prompts/p", &ctx)
        .unwrap()
        .expect("magic form is supported");
    assert_eq!(
        completion.roots(),
        &[
            launch.join("prompts"),
            home.join(".claudine/prompts/prompts"),
            home.join("prompts"),
        ],
    );
    assert_eq!(
        plan_paths("@prompts/plan.md", &ctx),
        vec![
            launch.join("prompts/plan.md"),
            home.join(".claudine/prompts/prompts/plan.md"),
            home.join("prompts/plan.md"),
        ],
        "the completion roots are exactly the plan candidate parents",
    );
}

/// R4: the exposed ordered root list is the chain itself — normalized,
/// deduplicated, provenance-tagged — with configured roots distinguishable
/// from intrinsic ones.
#[test]
fn magic_search_roots_exposes_the_ordered_chain() {
    let home = abs("h");
    let launch = abs("h/scratch");
    let ctx = snapshot(&launch, &home)
        .add_magic_path(&launch, PathPosition::Start)
        .add_magic_path(home.join(".claudine/prompts"), PathPosition::Start);

    let roots: Vec<(PathBuf, RootProvenance)> = ctx
        .magic_search_roots()
        .iter()
        .map(|root| (root.path().to_path_buf(), root.provenance()))
        .collect();
    assert_eq!(
        roots,
        vec![
            (launch.clone(), RootProvenance::Magic),
            (home.join(".claudine/prompts"), RootProvenance::Magic),
            (home.clone(), RootProvenance::Home),
        ],
        "a configured root equal to the local root dedupes to its Magic provenance",
    );
}

/// `LocalRoot` must not be boosted by `AuthoringBaseFirst`, which exists to
/// lift the *authoring base* of bare references — the request directory of
/// an `@` chain is a different anchor.
#[test]
fn authoring_base_first_does_not_boost_the_local_root() {
    let home = abs("h");
    let launch = abs("h/scratch");
    let ctx = snapshot(&launch, &home);

    let file_ref = FileReference::new("@x.md").unwrap();
    let resolution_order = file_ref.candidate_plan(&ctx).unwrap();
    let base_first = file_ref
        .candidate_plan_with_order(&ctx, CandidatePlanOrder::AuthoringBaseFirst)
        .unwrap();
    assert_eq!(
        resolution_order.iter().map(|c| c.path()).collect::<Vec<_>>(),
        base_first.iter().map(|c| c.path()).collect::<Vec<_>>(),
        "`@` order is unchanged by the authoring-base-first policy",
    );

    let bare = FileReference::new("x.md").unwrap();
    let base_first_bare = bare
        .candidate_plan_with_order(&ctx, CandidatePlanOrder::AuthoringBaseFirst)
        .unwrap();
    assert_eq!(base_first_bare[0].provenance(), RootProvenance::Source);
}

/// Ruling 2's cross-repository case, through the seeding API a request uses
/// when it rebuilds its context around a source from another repository: a
/// fresh source-anchored context carrying the launch `@` scope searches the
/// launch tree for `@` while `&`, `^`, and bare references anchor on the
/// source's own repository.
#[test]
fn seeded_launch_scope_keeps_at_local_while_sigils_follow_the_source() {
    let home = abs("h");
    let launch_repo = abs("h/scratch-repo");
    let launch_dir = launch_repo.join("work");
    let source_repo = abs("other/repo");
    let source_dir = source_repo.join("prompts");

    let launch = snapshot(&launch_dir, &home).with_repository_root(&launch_repo);
    let source = snapshot(&source_dir, &home)
        .with_repository_root(&source_repo)
        .with_launch_magic_scope(launch.launch_magic_scope().clone());

    assert_eq!(
        plan_paths("@x.md", &source),
        vec![launch_repo.join("x.md"), home.join("x.md")],
        "nested `@` keeps the launch tree; the source repository is never searched",
    );
    assert_eq!(
        plan_provenance("@x.md", &source),
        vec![RootProvenance::Repository, RootProvenance::Home],
    );
    assert_eq!(
        plan_paths("&x.md", &source),
        vec![source_repo.join("x.md")],
        "`&` keeps its source-specific repository anchor",
    );
    assert_eq!(
        plan_paths("x.md", &source),
        vec![source_dir.join("x.md"), source_repo.join("x.md")],
        "bare references keep their source-relative anchors",
    );
}

/// First-match regressions against the reported failures, staged with real
/// files so the ordered plan provably changes the winner (Defect 2 rows and
/// Defect 4).
#[test]
fn local_first_match_wins_over_user_tier_with_duplicate_files() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let scratch = home.join("scratch");
    let user_prompts = home.join(".claudine/prompts");
    let home_prompts = home.join("prompts");
    let scratch_prompts = scratch.join("prompts");
    for dir in [&scratch, &user_prompts, &home_prompts, &scratch_prompts] {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(scratch.join("x.md"), b"local").unwrap();
    fs::write(user_prompts.join("x.md"), b"user").unwrap();
    fs::create_dir_all(scratch.join("prompts")).unwrap();
    fs::write(scratch.join("prompts/x.md"), b"local prompts").unwrap();
    fs::write(home.join("prompts/x.md"), b"home prompts").unwrap();

    let ctx = FileResolutionContext::from_snapshot(
        &scratch,
        Some(home.clone()),
        HashMap::new(),
    )
    .add_magic_path(&user_prompts, PathPosition::Start);

    // Defect 2, row 1 shape: a user-tier prepend must not outrank the local
    // tree even though the local file is not in any configured root.
    assert_eq!(
        FileReference::new("@x.md")
            .unwrap()
            .resolve_in_context(&ctx)
            .unwrap()
            .as_deref(),
        Some(scratch.join("x.md").as_path()),
    );

    // Defect 4 shape (plain directory, path-shaped form): the launch
    // directory's own `prompts/` wins over the home tree.
    assert_eq!(
        FileReference::new("@prompts/x.md")
            .unwrap()
            .resolve_in_context(&ctx)
            .unwrap()
            .as_deref(),
        Some(scratch.join("prompts/x.md").as_path()),
    );
}

/// Defect 2, row 2: a local-tier *append* (`<repo>/.claudine`) must still
/// beat the home intrinsic root for the path-shaped form.
#[test]
fn local_append_root_beats_home_for_path_shaped_magic() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let repo = home.join("config/sh");
    let claudine_prompts = repo.join(".claudine/prompts");
    fs::create_dir_all(&claudine_prompts).unwrap();
    fs::create_dir_all(home.join("prompts")).unwrap();
    fs::write(claudine_prompts.join("x.md"), b"repository").unwrap();
    fs::write(home.join("prompts/x.md"), b"home").unwrap();

    let launch = repo.join("docs");
    fs::create_dir_all(&launch).unwrap();
    let ctx = FileResolutionContext::from_snapshot(&launch, Some(home.clone()), HashMap::new())
        .with_repository_root(&repo)
        .add_magic_path(repo.join(".claudine"), PathPosition::End);

    assert_eq!(
        FileReference::new("@prompts/x.md")
            .unwrap()
            .resolve_in_context(&ctx)
            .unwrap()
            .as_deref(),
        Some(claudine_prompts.join("x.md").as_path()),
        "the local append root precedes the home intrinsic root",
    );
}
