use super::*;
use std::fs;
use tempfile::TempDir;

/// Write a minimal Cargo workspace so `sniff::detect_repo_structure`
/// recognizes the tempdir as a monorepo.
fn seed_cargo_workspace(root: &Path, members: &[&str]) {
    fs::create_dir_all(root.join(".git")).unwrap();
    let members_list = members
        .iter()
        .map(|m| format!("    \"{m}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    let root_manifest =
        format!("[workspace]\nresolver = \"2\"\nmembers = [\n{members_list}\n]\n");
    fs::write(root.join("Cargo.toml"), root_manifest).unwrap();

    for member in members {
        let member_dir = root.join(member);
        fs::create_dir_all(member_dir.join("src")).unwrap();
        let name = member.replace('/', "-");
        fs::write(
            member_dir.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
        )
        .unwrap();
        fs::write(member_dir.join("src").join("lib.rs"), "").unwrap();
    }
}

fn test_ctx(cwd: &Path) -> ScopeContext {
    ScopeContext::discover_from(cwd)
}

#[test]
fn scope_set_iter_preserves_priority_order() {
    let set = ScopeSet {
        repo: Some(Scope {
            kind: ScopeKind::RepoPrompts,
            path: PathBuf::from("/repo"),
            follow_links: true,
        }),
        package_area: Some(Scope {
            kind: ScopeKind::PackageAreaPrompts,
            path: PathBuf::from("/area"),
            follow_links: true,
        }),
        package: Some(Scope {
            kind: ScopeKind::PackagePrompts,
            path: PathBuf::from("/pkg"),
            follow_links: true,
        }),
        repo_claudine: Some(Scope {
            kind: ScopeKind::RepoClaudinePrompts,
            path: PathBuf::from("/repo/.claudine"),
            follow_links: true,
        }),
        user_claudine: Some(Scope {
            kind: ScopeKind::UserClaudinePrompts,
            path: PathBuf::from("/user/.claudine"),
            follow_links: true,
        }),
        extras: vec![Scope {
            kind: ScopeKind::RepoDocs,
            path: PathBuf::from("/repo/docs"),
            follow_links: true,
        }],
    };
    let paths: Vec<&PathBuf> = set.iter_scopes().map(|s| &s.path).collect();
    assert_eq!(
        paths,
        vec![
            &PathBuf::from("/repo"),
            &PathBuf::from("/area"),
            &PathBuf::from("/pkg"),
            &PathBuf::from("/repo/.claudine"),
            &PathBuf::from("/user/.claudine"),
            &PathBuf::from("/repo/docs"),
        ]
    );
}

#[test]
fn outside_repo_only_user_scope_is_set() {
    // Place the cwd at a depth far below any realistic enclosing git
    // repo so `find_enclosing_repo` returns `None`. On developer
    // machines the tempdir is typically under `/var/folders/...`
    // which is NOT inside a git repo, so the tempdir itself suffices.
    let tmp = TempDir::new().unwrap();
    let ctx = test_ctx(tmp.path());
    let set = resolve_compose_scopes(&ctx, ComposeMode::Compose);
    if ctx.git_root.is_none() {
        assert!(set.repo.is_none());
        assert!(set.repo_claudine.is_none());
    }
    assert!(set.package_area.is_none());
    assert!(set.package.is_none());
    assert!(set.extras.is_empty());
    // user_claudine depends on dirs::home_dir(); assert only when
    // $HOME is resolvable, otherwise the field is None.
    if dirs::home_dir().is_some() {
        assert!(set.user_claudine.is_some());
    }
}

#[test]
fn bare_git_checkout_still_sets_repo_scope() {
    // Plain `.git` with no Cargo workspace — `detect_repo_structure`
    // returns `None`, but the git-root fallback means `<root>/prompts`
    // still appears as a scope.
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let ctx = test_ctx(tmp.path());
    assert!(ctx.repo_info.is_none(), "no workspace tool — no RepoInfo");
    assert!(ctx.git_root.is_some(), "git root must be detected");
    let set = resolve_compose_scopes(&ctx, ComposeMode::Compose);
    assert!(
        set.repo
            .as_ref()
            .is_some_and(|s| s.path.ends_with("prompts")),
        "bare git checkout must populate repo prompt scope: {:?}",
        set.repo
    );
    assert!(
        set.repo_claudine
            .as_ref()
            .is_some_and(|s| s.path.ends_with(PathBuf::from(".claudine").join("prompts"))),
        "bare git checkout must populate .claudine prompt scope: {:?}",
        set.repo_claudine
    );
}

#[test]
fn cwd_at_repo_root_sets_repo_and_claudine_but_not_area_or_pkg() {
    let tmp = TempDir::new().unwrap();
    // Workspace with two members under a single area. cwd is the repo
    // root itself, which is outside every package and every area.
    seed_cargo_workspace(tmp.path(), &["claudine/lib", "claudine/cli"]);

    let ctx = test_ctx(tmp.path());
    let set = resolve_compose_scopes(&ctx, ComposeMode::Compose);

    assert!(
        set.repo
            .as_ref()
            .is_some_and(|s| s.path.ends_with("prompts")),
        "repo scope must be populated at the workspace root: {:?}",
        set.repo
    );
    assert!(
        set.package_area.is_none(),
        "cwd at the root is not inside any area: {:?}",
        set.package_area
    );
    assert!(
        set.package.is_none(),
        "cwd at the root is not inside any package: {:?}",
        set.package
    );
    assert!(
        set.repo_claudine
            .as_ref()
            .is_some_and(|s| s.path.ends_with(PathBuf::from(".claudine").join("prompts"))),
        ".claudine/prompts scope must be populated: {:?}",
        set.repo_claudine
    );
}

#[test]
fn cwd_inside_package_area_sets_area_scope() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["claudine/lib", "claudine/cli"]);

    // cwd is the area directory itself (not a specific package inside it).
    let area_dir = tmp.path().join("claudine");
    fs::create_dir_all(&area_dir).unwrap();
    let ctx = test_ctx(&area_dir);
    let set = resolve_compose_scopes(&ctx, ComposeMode::Compose);

    // Because cwd is not inside a specific package, package should be
    // None. package_area should resolve to "claudine".
    assert!(
        set.package_area
            .as_ref()
            .is_some_and(|s| s.path.ends_with(PathBuf::from("claudine").join("prompts"))),
        "expected claudine/prompts as the area scope; got {:?}",
        set.package_area
    );
    assert!(
        set.package.is_none(),
        "cwd at the area root is not inside any specific package: {:?}",
        set.package
    );
}

#[test]
fn cwd_inside_discrete_package_sets_both_area_and_package() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["claudine/lib", "claudine/cli"]);

    let pkg_dir = tmp.path().join("claudine").join("cli");
    let ctx = test_ctx(&pkg_dir);
    let set = resolve_compose_scopes(&ctx, ComposeMode::Compose);

    assert!(
        set.package_area
            .as_ref()
            .is_some_and(|s| s.path.ends_with(PathBuf::from("claudine").join("prompts"))),
        "expected claudine/prompts as the area scope; got {:?}",
        set.package_area
    );
    assert!(
        set.package.as_ref().is_some_and(|s| s
            .path
            .ends_with(PathBuf::from("claudine").join("cli").join("prompts"))),
        "expected claudine/cli/prompts as the package scope; got {:?}",
        set.package
    );
}

#[test]
fn inline_compose_adds_docs_and_skill_peer_extras() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);

    let ctx = test_ctx(tmp.path());
    let set = resolve_compose_scopes(&ctx, ComposeMode::InlineCompose);

    // Assert by suffix; the temp-dir path on macOS aliases
    // `/var/folders/...` ↔ `/private/var/folders/...` and some scopes
    // reference directories that don't exist on disk, so canonicalize
    // cannot be applied uniformly.
    assert!(
        set.extras.iter().any(|s| s.path.ends_with("docs")),
        "docs/ missing from inline-compose extras: {:?}",
        set.extras
    );
    for peer in SKILL_PEER_DIRS {
        let expected_tail = PathBuf::from(peer).join("skills");
        assert!(
            set.extras.iter().any(|s| s.path.ends_with(&expected_tail)),
            "skill peer {peer}/skills missing from inline-compose extras: {:?}",
            set.extras
        );
    }
}

#[test]
fn sequence_adds_docs_and_skill_peer_extras() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);

    let ctx = test_ctx(tmp.path());
    let set = resolve_compose_scopes(&ctx, ComposeMode::Sequence);
    assert!(
        set.extras
            .iter()
            .any(|s| s.path.ends_with("docs") && s.follow_links),
        "sequence mode must include docs/ extra"
    );
    for peer in SKILL_PEER_DIRS {
        let expected_tail = PathBuf::from(peer).join("skills");
        assert!(
            set.extras
                .iter()
                .any(|s| s.path.ends_with(&expected_tail) && !s.follow_links),
            "sequence mode must include {peer}/skills extra with follow_links=false"
        );
    }
}

#[test]
fn compose_mode_has_no_extras() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let ctx = test_ctx(tmp.path());
    let set = resolve_compose_scopes(&ctx, ComposeMode::Compose);
    assert!(
        set.extras.is_empty(),
        "compose mode must not emit docs/ or skill-peer extras: {:?}",
        set.extras
    );
}

#[test]
fn skill_peer_scopes_never_follow_symlinks() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let ctx = test_ctx(tmp.path());
    let set = resolve_compose_scopes(&ctx, ComposeMode::InlineCompose);
    for scope in &set.extras {
        if scope.path.ends_with("skills") {
            assert!(
                !scope.follow_links,
                "skill peer scope must not follow symlinks: {scope:?}"
            );
        }
    }
}

#[test]
fn docs_extra_follows_symlinks() {
    let tmp = TempDir::new().unwrap();
    seed_cargo_workspace(tmp.path(), &["a/lib"]);
    let ctx = test_ctx(tmp.path());
    let set = resolve_compose_scopes(&ctx, ComposeMode::InlineCompose);
    let docs = set
        .extras
        .iter()
        .find(|s| s.path.ends_with("docs"))
        .expect("docs extra missing");
    assert!(docs.follow_links, "docs/ must follow symlinks: {docs:?}");
}

#[test]
fn scope_context_discover_from_is_deterministic() {
    let tmp = TempDir::new().unwrap();
    let a = ScopeContext::discover_from(tmp.path());
    let b = ScopeContext::discover_from(tmp.path());
    assert_eq!(a.cwd, b.cwd);
    assert_eq!(a.repo_info.is_some(), b.repo_info.is_some());
}

#[test]
fn dedup_collapses_when_area_equals_repo_path() {
    // Synthetic ScopeSet with coinciding repo and area paths.
    let mut set = ScopeSet {
        repo: Some(Scope {
            kind: ScopeKind::RepoPrompts,
            path: PathBuf::from("/r/prompts"),
            follow_links: true,
        }),
        package_area: Some(Scope {
            kind: ScopeKind::PackageAreaPrompts,
            path: PathBuf::from("/r/prompts"),
            follow_links: true,
        }),
        ..Default::default()
    };
    dedup_scopes(&mut set);
    assert!(set.package_area.is_none());
}
