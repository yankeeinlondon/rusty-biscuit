//! The file-tree model on [`FileResolutionContext`]: how the tree root
//! (`base_dir`) is selected, the boundary it enforces on relative references
//! (as written and where they really land), the reader opt-in, and how
//! derivations keep or replace the tree.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{
    BaseDirOrigin, DetailedOutcome, FileReference, FileReferenceError, FileResolutionContext,
    PackageAreaFallback, RepositoryScopeCatalog, ResolutionFailure,
};
use tempfile::TempDir;

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

/// A context that reads no ambient HOME or environment.
fn snapshot(cwd: &Path, home: Option<&Path>, env: &[(&str, &str)]) -> FileResolutionContext {
    let env: HashMap<String, String> =
        env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    FileResolutionContext::from_snapshot(cwd, home.map(Path::to_path_buf), env)
}

fn reference(raw: &str) -> FileReference {
    FileReference::new(raw).unwrap()
}

/// A directory link that needs no privilege: a symlink on Unix, a junction on
/// Windows (ruling R7).
fn link_dir(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    {
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "mklink /J failed");
    }
}

fn assert_tree_escape(result: Result<Option<PathBuf>, FileReferenceError>, base_dir: &Path) {
    match result {
        Err(FileReferenceError::RelativeTreeEscape { base_dir: actual, .. }) => {
            assert_eq!(actual, base_dir, "the error names the tree root");
        }
        other => panic!("expected RelativeTreeEscape, got {other:?}"),
    }
}

// --- Selection precedence -------------------------------------------------

#[test]
fn without_any_tree_input_base_dir_is_a_fallback_to_cwd() {
    let temp = TempDir::new().unwrap();
    let ctx = snapshot(temp.path(), None, &[]);

    assert_eq!(ctx.base_dir(), temp.path());
    assert_eq!(ctx.base_dir_origin(), &BaseDirOrigin::Fallback);
    assert!(!ctx.base_dir_is_boundary());
    assert_eq!(ctx.repository_root(), None);
}

#[test]
fn repository_root_is_the_tree_root_and_an_equal_explicit_root_is_accepted() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let docs = repo.join("docs");

    let ctx = snapshot(&docs, None, &[]).with_repository_root(&repo);
    assert_eq!(ctx.cwd(), docs.as_path());
    assert_eq!(ctx.base_dir(), repo.as_path());
    assert_eq!(ctx.base_dir_origin(), &BaseDirOrigin::Repository);
    assert_eq!(ctx.repository_root(), Some(repo.as_path()));

    // Builder order does not matter, and a differently spelled equal root is
    // still equal.
    for ctx in [
        snapshot(&docs, None, &[]).with_base_dir(&repo).with_repository_root(&repo),
        snapshot(&docs, None, &[]).with_repository_root(&repo).with_base_dir(repo.join("docs/..")),
    ] {
        ctx.validate().unwrap();
        assert_eq!(ctx.base_dir_origin(), &BaseDirOrigin::Repository);
        assert_eq!(ctx.base_dir(), repo.as_path());
    }
}

#[test]
fn an_explicit_root_other_than_the_repository_root_is_a_configuration_error() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let docs = repo.join("docs");
    write(&docs.join("x.md"), "x");

    for ctx in [
        snapshot(&docs, None, &[]).with_repository_root(&repo).with_base_dir(&docs),
        snapshot(&docs, None, &[]).with_base_dir(temp.path()).with_repository_root(&repo),
    ] {
        assert!(matches!(
            ctx.validate().unwrap_err(),
            FileReferenceError::BaseDirNotRepositoryRoot { .. }
        ));
        // The repository still outranks the explicit root.
        assert_eq!(ctx.base_dir(), repo.as_path());

        let detailed = reference("./x.md").resolve_detailed(&ctx);
        assert!(matches!(
            detailed.outcome(),
            DetailedOutcome::Failed(ResolutionFailure::MissingContext)
        ));
        assert!(matches!(
            detailed.error(),
            Some(FileReferenceError::BaseDirNotRepositoryRoot { .. })
        ));
        assert!(matches!(
            FileReference::complete_partial_in_context("x", &ctx),
            Err(FileReferenceError::BaseDirNotRepositoryRoot { .. })
        ));
    }
}

#[test]
fn an_explicit_root_outranks_a_containing_vault_and_is_a_boundary_even_at_cwd() {
    let temp = TempDir::new().unwrap();
    let vault = temp.path().join("vault");
    let notes = vault.join("notes");

    let ctx = snapshot(&notes, None, &[]).add_vault(&vault).with_base_dir(temp.path());
    assert_eq!(ctx.base_dir(), temp.path());
    assert_eq!(ctx.base_dir_origin(), &BaseDirOrigin::Explicit);

    let at_cwd = snapshot(&notes, None, &[]).with_base_dir(&notes);
    assert_eq!(at_cwd.base_dir(), at_cwd.cwd());
    assert!(at_cwd.base_dir_is_boundary(), "an explicit root equal to cwd still enforces");
}

#[test]
fn the_deepest_containing_vault_wins_and_configured_roots_win_depth_ties() {
    let temp = TempDir::new().unwrap();
    let outer = temp.path().join("vault");
    let inner = outer.join("team");
    let notes = inner.join("notes");

    for ctx in [
        snapshot(&notes, None, &[]).add_vault(&outer).add_vault(&inner),
        snapshot(&notes, None, &[]).add_vault(&inner).add_vault(&outer),
    ] {
        assert_eq!(ctx.base_dir(), inner.as_path());
        assert_eq!(ctx.base_dir_origin(), &BaseDirOrigin::Vault);
    }

    // A captured `VAULT` root takes part too, after the configured roots.
    let vault_env = std::env::join_paths([&inner]).unwrap();
    let ctx = snapshot(&notes, None, &[("VAULT", vault_env.to_str().unwrap())]).add_vault(&outer);
    assert_eq!(ctx.base_dir(), inner.as_path(), "a deeper captured VAULT root wins");

    // Equal depth (one directory, two spellings): configuration order decides,
    // and the configured spelling comes before the captured one.
    let respelled = inner.join("..").join("team");
    let respelled_env = std::env::join_paths([&respelled]).unwrap();
    let ctx = snapshot(&notes, None, &[("VAULT", respelled_env.to_str().unwrap())])
        .add_vault(&inner);
    assert_eq!(ctx.base_dir(), inner.as_path());
    let ctx = snapshot(&notes, None, &[]).add_vault(&respelled).add_vault(&inner);
    assert_eq!(ctx.base_dir(), respelled.as_path());

    // A vault that does not contain cwd supplies nothing.
    let elsewhere = snapshot(temp.path(), None, &[]).add_vault(&inner);
    assert_eq!(elsewhere.base_dir_origin(), &BaseDirOrigin::Fallback);
}

#[test]
fn a_home_anchored_opening_reference_makes_home_the_tree_root() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    let downloads = home.join("Downloads");
    write(&downloads.join("a.md"), "a");
    write(&home.join("b.md"), "b");
    write(&temp.path().join("outside.md"), "outside");
    let launch = snapshot(temp.path(), Some(&home), &[]);

    let opened = reference("~/Downloads/a.md");
    let child = launch.for_source_reference(&opened, downloads.join("a.md"));

    child.validate().unwrap();
    assert_eq!(child.cwd(), downloads.as_path());
    assert_eq!(child.base_dir(), home.as_path());
    assert_eq!(child.base_dir_origin(), &BaseDirOrigin::Home);
    assert_eq!(
        reference("../b.md").resolve_in_context(&child).unwrap(),
        Some(home.join("b.md")),
        "a parent link inside the home tree resolves"
    );
    assert_tree_escape(reference("../../outside.md").resolve_in_context(&child), &home);
}

#[test]
fn an_environment_anchored_opening_reference_makes_the_variable_the_tree_root() {
    let temp = TempDir::new().unwrap();
    let notes = temp.path().join("notes");
    let inbox = notes.join("inbox");
    write(&inbox.join("a.md"), "a");
    write(&temp.path().join("outside.md"), "outside");
    let launch = snapshot(temp.path(), None, &[("NOTES", notes.to_str().unwrap())]);

    let child = launch.for_source_reference(&reference("{{NOTES}}/inbox/a.md"), inbox.join("a.md"));

    assert_eq!(child.base_dir(), notes.as_path());
    assert_eq!(
        child.base_dir_origin(),
        &BaseDirOrigin::Environment {
            name: "NOTES".to_string()
        }
    );
    assert_tree_escape(reference("../../outside.md").resolve_in_context(&child), &notes);
}

#[test]
fn unset_relative_foreign_or_non_containing_anchors_supply_no_tree_root() {
    let temp = TempDir::new().unwrap();
    let notes = temp.path().join("notes");
    let source = notes.join("inbox/a.md");
    write(&source, "a");
    #[cfg(not(windows))]
    let foreign = r"C:\notes";
    #[cfg(windows)]
    let foreign = "/notes";
    let other = temp.path().join("other");

    let cases: [(&str, Vec<(&str, &str)>); 4] = [
        ("unset", vec![]),
        ("relative", vec![("NOTES", "notes")]),
        ("foreign host", vec![("NOTES", foreign)]),
        ("not containing", vec![("NOTES", other.to_str().unwrap())]),
    ];
    for (label, env) in cases {
        let launch = snapshot(temp.path(), None, &env);
        let child = launch.for_source_reference(&reference("{{NOTES}}/inbox/a.md"), &source);
        assert_eq!(child.base_dir_origin(), &BaseDirOrigin::Fallback, "{label}");
        assert_eq!(child.base_dir(), child.cwd(), "{label}");
    }

    // `~` with no captured home supplies nothing either.
    let launch = snapshot(temp.path(), None, &[]);
    let child = launch.for_source_reference(&reference("~/notes/inbox/a.md"), &source);
    assert_eq!(child.base_dir_origin(), &BaseDirOrigin::Fallback);
}

#[test]
fn a_containing_vault_outranks_the_opening_anchor() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    let vault = home.join("vault");
    let source = vault.join("notes/a.md");
    write(&source, "a");
    let launch = snapshot(temp.path(), Some(&home), &[]).add_vault(&vault);

    let child = launch.for_source_reference(&reference("~/vault/notes/a.md"), &source);
    assert_eq!(child.base_dir(), vault.as_path());
    assert_eq!(child.base_dir_origin(), &BaseDirOrigin::Vault);
}

#[test]
fn an_anchor_in_a_link_does_not_replace_a_tree_that_already_contains_the_document() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    let docs = home.join("docs");
    let source = docs.join("notes/a.md");
    write(&source, "a");
    let launch = snapshot(&docs, Some(&home), &[]).with_base_dir(&docs);

    let child = launch.for_source_reference(&reference("~/docs/notes/a.md"), &source);
    assert_eq!(child.base_dir(), docs.as_path());
    assert_eq!(child.base_dir_origin(), &BaseDirOrigin::Explicit);
}

// --- The relative boundary -------------------------------------------------

#[test]
fn relative_references_that_leave_a_repository_are_rejected() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let docs = repo.join("docs");
    write(&temp.path().join("outside.md"), "outside");
    write(&repo.join("x.md"), "inside");
    fs::create_dir_all(&docs).unwrap();
    let ctx = snapshot(&docs, None, &[]).with_repository_root(&repo);

    // Explicit and bare escapes, through every API.
    for raw in ["./../../outside.md", "../../outside.md", "a/../../../outside.md"] {
        let file_ref = reference(raw);
        assert_tree_escape(file_ref.resolve_in_context(&ctx), &repo);
        let detailed = file_ref.resolve_detailed(&ctx);
        assert!(matches!(
            detailed.outcome(),
            DetailedOutcome::Failed(ResolutionFailure::InvalidReference)
        ));
        match detailed.error() {
            Some(FileReferenceError::RelativeTreeEscape {
                base_dir,
                candidate,
                reference,
            }) => {
                assert_eq!(base_dir, &repo);
                assert_eq!(candidate, &temp.path().join("outside.md"));
                assert_eq!(reference, raw);
            }
            other => panic!("expected RelativeTreeEscape, got {other:?}"),
        }
        assert!(detailed.candidates().is_empty(), "no candidate is probed");
        assert!(matches!(
            file_ref.candidate_plan(&ctx),
            Err(FileReferenceError::RelativeTreeEscape { .. })
        ));
    }

    let repo_root_ctx = snapshot(&repo, None, &[]).with_repository_root(&repo);
    assert_tree_escape(reference("a/../../outside.md").resolve_in_context(&repo_root_ctx), &repo);

    // A parent link that stays inside the repository still resolves.
    assert_eq!(
        reference("../x.md").resolve_in_context(&ctx).unwrap(),
        Some(repo.join("x.md"))
    );
}

#[test]
fn an_escaping_repository_fallback_candidate_is_an_error_not_a_skipped_root() {
    // From `repo/docs`, the bare `a/../../x.md` is `repo/x.md` relative to
    // cwd but leaves the repository from the repository-root fallback. The
    // whole reference is rejected before either candidate is probed.
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let docs = repo.join("docs");
    write(&repo.join("x.md"), "x");
    let ctx = snapshot(&docs, None, &[]).with_repository_root(&repo);

    assert_tree_escape(reference("a/../../x.md").resolve_in_context(&ctx), &repo);
}

#[test]
fn an_explicit_root_bounds_relative_references_outside_a_repository() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path().join("docs");
    let notes = docs.join("notes");
    write(&docs.join("index.md"), "index");
    write(&temp.path().join("outside.md"), "outside");
    fs::create_dir_all(&notes).unwrap();
    let ctx = snapshot(&notes, None, &[]).with_base_dir(&docs);

    ctx.validate().unwrap();
    assert_eq!(ctx.repository_root(), None);
    assert_eq!(
        reference("../index.md").resolve_in_context(&ctx).unwrap(),
        Some(docs.join("index.md"))
    );
    assert_tree_escape(reference("../../outside.md").resolve_in_context(&ctx), &docs);
    assert_tree_escape(reference("./../../outside.md").resolve_in_context(&ctx), &docs);
}

#[test]
fn a_fallback_tree_root_does_not_reject_relative_references() {
    let temp = TempDir::new().unwrap();
    let notes = temp.path().join("notes");
    fs::create_dir_all(&notes).unwrap();
    write(&temp.path().join("outside.md"), "outside");
    let ctx = snapshot(&notes, None, &[]);

    assert_eq!(
        reference("../outside.md").resolve_in_context(&ctx).unwrap(),
        Some(temp.path().join("outside.md"))
    );
    assert!(FileReference::complete_partial_in_context("a/../../", &ctx).is_ok());

    // The ambient compatibility method carries no tree either.
    assert_eq!(
        reference("../outside.md").resolve_from(&notes).unwrap(),
        Some(temp.path().join("outside.md"))
    );
}

#[test]
fn the_reader_opt_in_permits_escaping_targets_and_survives_derivation() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let docs = repo.join("docs");
    write(&temp.path().join("outside.md"), "outside");
    write(&docs.join("guide/page.md"), "page");
    let ctx = snapshot(&docs, None, &[])
        .with_repository_root(&repo)
        .allow_external_relative();

    assert!(ctx.external_relative_allowed());
    assert_eq!(
        reference("../../outside.md").resolve_in_context(&ctx).unwrap(),
        Some(temp.path().join("outside.md"))
    );

    let child = ctx.for_source(docs.join("guide/page.md"));
    assert!(child.external_relative_allowed(), "the opt-in is copied to children");
    assert_eq!(
        reference("../../../outside.md").resolve_in_context(&child).unwrap(),
        Some(temp.path().join("outside.md"))
    );
    assert!(FileReference::complete_partial_in_context("a/../../../", &child).is_ok());

    // It does not exempt an invalid document cwd, nor relax `&`.
    let escaped = ctx.for_cwd(temp.path());
    assert!(matches!(
        escaped.validate().unwrap_err(),
        FileReferenceError::RepositoryRootNotContainingSource { .. }
    ));
    assert!(matches!(
        reference("&../outside.md").resolve_in_context(&ctx),
        Err(FileReferenceError::RepositoryEscape { .. })
    ));
}

#[test]
fn an_absolute_environment_expansion_is_not_held_to_the_boundary() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let config = temp.path().join("config");
    write(&config.join("x.json"), "{}");
    let ctx = snapshot(&repo, None, &[("CONFIG_DIR", config.to_str().unwrap())])
        .with_repository_root(&repo);

    assert_eq!(
        reference("{{CONFIG_DIR}}/x.json").resolve_in_context(&ctx).unwrap(),
        Some(config.join("x.json"))
    );
}

#[test]
fn repository_sigils_stay_repository_only_with_an_explicit_root() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path().join("docs");
    write(&docs.join("x.md"), "x");
    let ctx = snapshot(&docs, None, &[]).with_base_dir(&docs);

    for raw in ["&x.md", "^x.md"] {
        assert!(matches!(
            reference(raw).resolve_in_context(&ctx),
            Err(FileReferenceError::OutsideRepository { .. })
        ));
    }
}

#[test]
fn recursive_relative_searches_cannot_start_outside_the_tree() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    write(&temp.path().join("sibling/x.md"), "x");
    let ctx = snapshot(&repo, None, &[]).with_repository_root(&repo);

    let file_ref = reference("%a/../../sibling/x.md");
    assert!(matches!(
        file_ref.candidate_plan(&ctx),
        Err(FileReferenceError::RelativeTreeEscape { .. })
    ));
    assert_tree_escape(file_ref.resolve_in_context(&ctx), &repo);
}

#[test]
fn completion_does_not_offer_escaping_relative_roots() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let docs = repo.join("docs");
    write(&docs.join("guide.md"), "guide");
    let ctx = snapshot(&docs, None, &[]).with_repository_root(&repo);

    assert!(matches!(
        FileReference::complete_partial_in_context("a/../../../", &ctx),
        Err(FileReferenceError::RelativeTreeEscape { .. })
    ));
    let completion = FileReference::complete_partial_in_context("gu", &ctx)
        .unwrap()
        .unwrap();
    assert_eq!(completion.roots(), [docs.clone(), repo.clone()]);
}

// --- Where a link really lands ---------------------------------------------

#[test]
fn the_boundary_follows_directory_links_to_where_they_land() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let docs = repo.join("docs");
    let outside = temp.path().join("team-docs");
    write(&docs.join("v2/x.md"), "in tree");
    write(&outside.join("x.md"), "outside");
    link_dir(&docs.join("v2"), &docs.join("current"));
    link_dir(&outside, &docs.join("shared"));
    let ctx = snapshot(&docs, None, &[]).with_repository_root(&repo);

    // In-tree link: allowed.
    assert_eq!(
        reference("./current/x.md").resolve_in_context(&ctx).unwrap(),
        Some(docs.join("current/x.md"))
    );
    // Out-of-tree link: rejected for an existing target, for a target not yet
    // created (via its deepest existing ancestor), and in completion.
    assert_tree_escape(reference("./shared/x.md").resolve_in_context(&ctx), &repo);
    assert_tree_escape(reference("shared/x.md").resolve_in_context(&ctx), &repo);
    assert_tree_escape(reference("./shared/missing/new.md").resolve_in_context(&ctx), &repo);
    assert!(matches!(
        FileReference::complete_partial_in_context("shared/x", &ctx),
        Err(FileReferenceError::RelativeTreeEscape { .. })
    ));
    // The lexical plan cannot see the link, so it does not reject it.
    assert!(reference("./shared/x.md").candidate_plan(&ctx).is_ok());

    // The reader opt-in permits the escape; a fallback tree never checks.
    let opted_in = ctx.clone().allow_external_relative();
    assert_eq!(
        reference("./shared/x.md").resolve_in_context(&opted_in).unwrap(),
        Some(docs.join("shared/x.md"))
    );
    let fallback = snapshot(&docs, None, &[]);
    assert_eq!(
        reference("./shared/x.md").resolve_in_context(&fallback).unwrap(),
        Some(docs.join("shared/x.md"))
    );
}

// --- Derivation ------------------------------------------------------------

#[test]
fn normal_derivation_keeps_the_tree_its_origin_and_the_launch_scope() {
    let temp = TempDir::new().unwrap();
    let docs = temp.path().join("docs");
    let notes = docs.join("notes");
    let ctx = snapshot(&docs, None, &[]).with_base_dir(&docs);

    for child in [ctx.for_source(notes.join("a.md")), ctx.for_cwd(&notes)] {
        child.validate().unwrap();
        assert_eq!(child.cwd(), notes.as_path());
        assert_eq!(child.base_dir(), docs.as_path());
        assert_eq!(child.base_dir_origin(), &BaseDirOrigin::Explicit);
        assert_eq!(child.launch_magic_scope(), ctx.launch_magic_scope());
    }

    // Re-deriving at its own cwd is a no-op on the tree.
    let same = ctx.for_cwd(ctx.cwd());
    assert_eq!(same.base_dir(), ctx.base_dir());
    assert_eq!(same.base_dir_origin(), ctx.base_dir_origin());

    // A normal derivation that leaves the tree is invalid, not re-rooted.
    let escaped = ctx.for_source(temp.path().join("a.md"));
    assert_eq!(escaped.base_dir(), docs.as_path());
    assert!(matches!(
        escaped.validate().unwrap_err(),
        FileReferenceError::CwdOutsideBaseDir { .. }
    ));
}

#[test]
fn derivation_from_a_fallback_tree_selects_a_tree_for_the_new_document() {
    let temp = TempDir::new().unwrap();
    let vault = temp.path().join("vault");
    let ctx = snapshot(temp.path(), None, &[]).add_vault(&vault);
    assert_eq!(ctx.base_dir_origin(), &BaseDirOrigin::Fallback);

    let in_vault = ctx.for_source(vault.join("notes/a.md"));
    assert_eq!(in_vault.base_dir(), vault.as_path());
    assert_eq!(in_vault.base_dir_origin(), &BaseDirOrigin::Vault);

    let plain = ctx.for_cwd(temp.path().join("misc"));
    assert_eq!(plain.base_dir(), plain.cwd(), "a fallback root follows cwd");
}

#[test]
fn trusted_external_derivation_drops_source_anchors_but_keeps_the_launch_scope() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let area = repo.join("area");
    let package = area.join("pkg");
    let home = temp.path().join("home");
    let prompts = home.join(".claudine/prompts");
    write(&prompts.join("x.md"), "x");
    write(&home.join("escape.md"), "inside home");
    write(&temp.path().join("outside.md"), "outside");
    let catalog = RepositoryScopeCatalog::new(
        repo.clone(),
        vec![area.clone()],
        vec![package.clone()],
        PackageAreaFallback::None,
    )
    .unwrap();
    let request = snapshot(&package, Some(&home), &[])
        .with_repository_scope_catalog(catalog)
        .with_base_dir(&repo);
    request.validate().unwrap();

    let external = request
        .for_trusted_external_source_reference(&reference("~/.claudine/prompts/x.md"), prompts.join("x.md"));
    external.validate().unwrap();
    assert_eq!(external.repository_root(), None);
    assert_eq!(external.package_root(), None);
    assert_eq!(external.package_area(), None);
    assert_eq!(external.base_dir(), home.as_path());
    assert_eq!(external.base_dir_origin(), &BaseDirOrigin::Home);
    assert_eq!(external.launch_magic_scope(), request.launch_magic_scope());
    assert!(matches!(
        reference("&x.md").resolve_in_context(&external),
        Err(FileReferenceError::OutsideRepository { .. })
    ));
    assert_eq!(
        reference("../../escape.md").resolve_in_context(&external).unwrap(),
        Some(home.join("escape.md"))
    );
    assert_tree_escape(reference("../../../outside.md").resolve_in_context(&external), &home);

    // Without an anchor the external tree falls back to the new cwd, and the
    // originating explicit root does not carry over.
    let plain = request.for_trusted_external_source(prompts.join("x.md"));
    assert_eq!(plain.base_dir_origin(), &BaseDirOrigin::Fallback);
    assert_eq!(plain.base_dir(), prompts.as_path());

    // A caller that knows the destination's topology supplies it explicitly.
    let supplied = request.for_trusted_external_cwd(&prompts).with_base_dir(&home);
    supplied.validate().unwrap();
    assert_eq!(supplied.base_dir_origin(), &BaseDirOrigin::Explicit);
}

#[test]
fn trusted_external_derivation_uses_a_catalog_repository_and_never_discovers_one() {
    let temp = TempDir::new().unwrap();
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    let request = snapshot(&first, None, &[]).with_repository_root(&first);
    // This crate's directory sits inside a real git worktree; the explicit
    // context must not discover it.
    let worktree_dir = biscuit_test_harness::manifest_dir!();
    let external = request.for_trusted_external_cwd(&worktree_dir);
    external.validate().unwrap();
    assert_eq!(external.repository_root(), None);
    assert_eq!(external.base_dir_origin(), &BaseDirOrigin::Fallback);

    // A catalog that does contain the destination supplies its repository.
    let catalog =
        RepositoryScopeCatalog::new(second.clone(), Vec::new(), Vec::new(), PackageAreaFallback::None)
            .unwrap();
    let request = snapshot(&first, None, &[]).with_repository_scope_catalog(catalog);
    assert_eq!(request.repository_root(), None, "the catalog does not contain first");
    let external = request.for_trusted_external_cwd(second.join("tools"));
    assert_eq!(external.repository_root(), Some(second.as_path()));
    assert_eq!(external.base_dir_origin(), &BaseDirOrigin::Repository);
}

#[test]
fn a_tree_root_that_does_not_exist_is_checked_lexically() {
    // Nothing inside a missing root can exist, so the real-landing step has
    // nothing to add: an in-tree miss stays a clean miss, not an I/O error.
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("gone/repo");
    let ctx = snapshot(&repo.join("docs"), None, &[]).with_repository_root(&repo);

    assert_eq!(reference("../x.md").resolve_in_context(&ctx).unwrap(), None);
    assert_tree_escape(reference("../../x.md").resolve_in_context(&ctx), &repo);
}
