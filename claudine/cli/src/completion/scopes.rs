//! "High-profile" directory resolution for composition command completion.
//!
//! Phase 2 of the `2026-04-24-improved-shell-completions` feature. This
//! module turns a cwd into an ordered [`ScopeSet`] — the handful of
//! directories the completion engine will walk when deciding which
//! composition files to offer. Everything filesystem-specific lives here so
//! the composition pipeline (Phase 3) can work against a pure data
//! structure.
//!
//! The resolver leans on [`sniff::filesystem::repo::detect_repo_structure`]
//! for monorepo shape discovery. `sniff` can shell out to `cargo metadata`
//! on first call, so the plan deliberately threads a single [`RepoInfo`]
//! through [`ScopeContext`] and never re-detects mid-completion.
//!
//! Scopes carry a `follow_links` flag so the walker (`walker.rs`) can apply
//! the per-scope symlink policy from the spec: generic scopes follow
//! symlinks, agent-skill peer directories do not (Claudine's linker
//! symlinks the same skill body across multiple provider dirs, and
//! following those symlinks would produce duplicate candidates).

// Phase 2 scaffolding. Public items below are consumed by the composition
// and setter-value completers wired up in Phases 3 and 4; today they are
// exercised only through this module's own `#[cfg(test)]` coverage.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use sniff::filesystem::repo::{RepoInfo, detect_repo_structure};

/// The composition command the scopes are being resolved for.
///
/// Each variant maps to a different scope table: `Compose` sees only the
/// shared prompt scopes; `InlineCompose` and `Sequence` additionally walk
/// `<repo>/docs/` and the agent-skill peer directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComposeMode {
    /// `claudine compose <FILE>` — plain markdown, no `prompt` frontmatter.
    Compose,
    /// `claudine inline-compose <FILE>` — markdown with `prompt` frontmatter.
    InlineCompose,
    /// `claudine sequence <FILE>` — markdown or YAML with `sequence` key.
    Sequence,
}

/// A single scope root paired with its symlink policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Scope {
    /// Directory root the walker will descend into.
    pub(crate) path: PathBuf,
    /// Whether symlinks encountered while walking this scope are followed.
    ///
    /// Set to `false` for agent-skill peer directories so Claudine-linked
    /// skills do not surface multiple times (one per provider CLI dir).
    pub(crate) follow_links: bool,
}

/// Ordered scope set for a composition command.
///
/// Iteration order matches the priority ordering used when rendering
/// candidates in later phases: repo first (most specific), then area,
/// package, repo-scope `.claudine/`, user-scope `.claudine/`, then extras.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ScopeSet {
    /// `<repo>/prompts/` when cwd is inside a detected repo.
    pub(crate) repo: Option<Scope>,
    /// `<repo>/<area>/prompts/` when cwd resolves to a non-root package area.
    pub(crate) package_area: Option<Scope>,
    /// `<pkg>/prompts/` when cwd is inside a discrete package.
    pub(crate) package: Option<Scope>,
    /// `<repo>/.claudine/prompts/` when cwd is inside a detected repo.
    pub(crate) repo_claudine: Option<Scope>,
    /// `~/.claudine/prompts/` when `$HOME` resolves.
    pub(crate) user_claudine: Option<Scope>,
    /// Mode-specific extras (e.g. `<repo>/docs/`, agent-skill peers).
    pub(crate) extras: Vec<Scope>,
}

impl ScopeSet {
    /// Flatten the scope set into an iteration order suitable for a walker.
    ///
    /// The order is: repo → package_area → package → repo_claudine →
    /// user_claudine → extras (in the order they were pushed). This is the
    /// priority used for rendering in Phase 3; earlier scopes win on
    /// dedup.
    pub(crate) fn iter_scopes(&self) -> impl Iterator<Item = &Scope> {
        self.repo
            .iter()
            .chain(self.package_area.iter())
            .chain(self.package.iter())
            .chain(self.repo_claudine.iter())
            .chain(self.user_claudine.iter())
            .chain(self.extras.iter())
    }
}

/// Context required to resolve scopes for one `__complete` invocation.
///
/// Built once per completion run; threaded through scope resolution so
/// `sniff::detect_repo_structure` runs at most once.
#[derive(Debug, Clone)]
pub(crate) struct ScopeContext {
    /// The current working directory the shell invoked completion from.
    pub(crate) cwd: PathBuf,
    /// User `$HOME`, if resolvable.
    pub(crate) home: Option<PathBuf>,
    /// `sniff`-derived repo shape (monorepo packages, areas, root).
    ///
    /// `None` means cwd is not inside any detected repository.
    pub(crate) repo_info: Option<RepoInfo>,
    /// Enclosing `.git` directory / file, if any.
    ///
    /// Recorded independently of [`repo_info`] because plain git
    /// checkouts (no Cargo workspace, no npm monorepo, etc.) still return
    /// `None` from `detect_repo_structure` yet are a real context for
    /// completion — we want `<git-root>/prompts/` surfaced even without a
    /// workspace manifest.
    pub(crate) git_root: Option<PathBuf>,
}

impl ScopeContext {
    /// Build a context by inspecting the filesystem.
    ///
    /// Invokes `sniff::detect_repo_structure` exactly once. Errors are
    /// swallowed into `None` — completion is best-effort and must never
    /// fail loudly because `sniff` had trouble with an unusual checkout.
    pub(crate) fn discover() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::discover_from(&cwd)
    }

    /// Like [`discover`](Self::discover) but treats `cwd` as given. Used by
    /// tests.
    ///
    /// `sniff::detect_repo_structure` runs against the detected repo root,
    /// not `cwd` itself — `sniff` only reads manifests at the exact path
    /// passed in, so a cwd nested inside a workspace must first be lifted
    /// to the enclosing repo root. Walking upward once here keeps the
    /// `sniff` call single-invocation per completion run.
    pub(crate) fn discover_from(cwd: &Path) -> Self {
        let repo_root = find_enclosing_repo(cwd);
        let repo_info = repo_root
            .as_deref()
            .and_then(|root| detect_repo_structure(root).ok().flatten());
        Self {
            cwd: cwd.to_path_buf(),
            home: dirs::home_dir(),
            repo_info,
            git_root: repo_root,
        }
    }
}

/// Walk upward from `start` until a `.git` entry is found.
///
/// Both a `.git` directory (standard checkout) and a `.git` file (worktree
/// pointer) count as repo roots. The file variant matters to Claudine
/// development specifically — the project is often edited from a git
/// worktree, where `.git` is a pointer file rather than a directory.
fn find_enclosing_repo(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        let dot_git = ancestor.join(".git");
        if dot_git.is_dir() || dot_git.is_file() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

/// Agent-skill peer directory names. Each is joined with `skills/` under
/// the repo root when `inline-compose` or `sequence` scopes are resolved.
///
/// Only **repo-local** skill directories are walked — the spec excludes
/// user-global skill directories from the composition scope set.
const SKILL_PEER_DIRS: &[&str] = &[
    ".claude",
    ".codex",
    ".gemini",
    ".opencode",
    ".goose",
    ".qwen",
    ".kimi",
];

/// Prompt-directory leaf name.
const PROMPTS_DIR: &str = "prompts";

/// Resolve the scope set for a composition command.
///
/// Never touches the filesystem beyond what [`ScopeContext::discover`]
/// already did; returns directory paths whether they exist or not. The
/// walker (`walker.rs`) checks existence when it tries to descend.
///
/// The `"root"` pseudo-area produced by `sniff` for top-level workspace
/// packages is elided — there is no `<repo>/root/prompts/` directory, and
/// resolving one would duplicate the repo-scope entry.
pub(crate) fn resolve_compose_scopes(ctx: &ScopeContext, mode: ComposeMode) -> ScopeSet {
    let mut set = ScopeSet::default();

    // Pick the best "repo root" for scope resolution: the workspace root
    // from `sniff` when it exists, otherwise the enclosing git root. Spec
    // §5.8 says repo-scoped paths should still apply to plain git
    // checkouts; the fallback keeps completion useful there.
    let effective_repo_root: Option<&Path> = ctx
        .repo_info
        .as_ref()
        .map(|info| info.root.as_path())
        .or(ctx.git_root.as_deref());

    if let Some(root) = effective_repo_root {
        set.repo = Some(Scope {
            path: root.join(PROMPTS_DIR),
            follow_links: true,
        });
        set.repo_claudine = Some(Scope {
            path: root.join(".claudine").join(PROMPTS_DIR),
            follow_links: true,
        });
    }

    if let Some(info) = &ctx.repo_info {
        if let Some(area) = info.package_area_for_dir(&ctx.cwd)
            && area != "root"
        {
            set.package_area = Some(Scope {
                path: info.root.join(area).join(PROMPTS_DIR),
                follow_links: true,
            });
        }

        if let Some(pkg) = info.package_for_dir(&ctx.cwd) {
            set.package = Some(Scope {
                path: pkg.path.join(PROMPTS_DIR),
                follow_links: true,
            });
        }
    }

    if let Some(home) = &ctx.home {
        set.user_claudine = Some(Scope {
            path: home.join(".claudine").join(PROMPTS_DIR),
            follow_links: true,
        });
    }

    match mode {
        ComposeMode::Compose => {}
        ComposeMode::InlineCompose | ComposeMode::Sequence => {
            if let Some(root) = effective_repo_root {
                set.extras.push(Scope {
                    path: root.join("docs"),
                    follow_links: true,
                });

                for peer in SKILL_PEER_DIRS {
                    set.extras.push(Scope {
                        path: root.join(peer).join("skills"),
                        follow_links: false,
                    });
                }
            }
        }
    }

    dedup_scopes(&mut set);
    set
}

/// Collapse duplicate scope paths that can arise when cwd is at the repo
/// root and the area/package resolve to the same directory as the repo
/// entry.
fn dedup_scopes(set: &mut ScopeSet) {
    let repo_path = set.repo.as_ref().map(|s| s.path.clone());

    if let (Some(repo), Some(area)) = (repo_path.as_ref(), set.package_area.as_ref())
        && area.path == *repo
    {
        set.package_area = None;
    }

    if let (Some(repo), Some(pkg)) = (repo_path.as_ref(), set.package.as_ref())
        && pkg.path == *repo
    {
        set.package = None;
    }

    if let (Some(area), Some(pkg)) = (set.package_area.as_ref(), set.package.as_ref())
        && area.path == pkg.path
    {
        set.package = None;
    }
}

#[cfg(test)]
mod tests {
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
                path: PathBuf::from("/repo"),
                follow_links: true,
            }),
            package_area: Some(Scope {
                path: PathBuf::from("/area"),
                follow_links: true,
            }),
            package: Some(Scope {
                path: PathBuf::from("/pkg"),
                follow_links: true,
            }),
            repo_claudine: Some(Scope {
                path: PathBuf::from("/repo/.claudine"),
                follow_links: true,
            }),
            user_claudine: Some(Scope {
                path: PathBuf::from("/user/.claudine"),
                follow_links: true,
            }),
            extras: vec![Scope {
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
                path: PathBuf::from("/r/prompts"),
                follow_links: true,
            }),
            package_area: Some(Scope {
                path: PathBuf::from("/r/prompts"),
                follow_links: true,
            }),
            ..Default::default()
        };
        dedup_scopes(&mut set);
        assert!(set.package_area.is_none());
    }
}
