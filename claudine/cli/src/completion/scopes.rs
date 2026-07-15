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

/// Semantic category for a completion scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeKind {
    /// `<repo>/prompts/`.
    RepoPrompts,
    /// `<package-area>/prompts/`.
    PackageAreaPrompts,
    /// `<package>/prompts/`.
    PackagePrompts,
    /// `<repo>/.claudine/prompts/`.
    RepoClaudinePrompts,
    /// `~/.claudine/prompts/`.
    UserClaudinePrompts,
    /// Repo-local document-style scope such as `<repo>/docs/`.
    RepoDocs,
    /// Repo-local agent skill scope.
    AgentSkills,
    /// Repo-wide directory walk scope.
    RepoDirWalk,
    /// Committed directory selected by the user.
    CommittedDir,
}

/// A single scope root paired with explicit rendering semantics and symlink policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Scope {
    /// Semantic category used by renderers.
    pub(crate) kind: ScopeKind,
    /// Directory root the walker will descend into.
    pub(crate) path: PathBuf,
    /// Whether symlinks encountered while walking this scope are followed.
    ///
    /// Set to `false` for agent-skill peer directories so Claudine-linked
    /// skills do not surface multiple times (one per provider CLI dir).
    pub(crate) follow_links: bool,
}

/// Return the repo root that completion should treat as authoritative.
pub(crate) fn effective_repo_root(ctx: &ScopeContext) -> Option<&Path> {
    ctx.repo_info
        .as_ref()
        .map(|info| info.root.as_path())
        .or(ctx.git_root.as_deref())
}

/// Root directory that frontmatter **property-value** file completion walks
/// and renders paths against — always the invoking `cwd`, never the repo root.
///
/// A frontmatter file reference (a `name=value` setter, a `$schema`
/// `file`/`file[]` property) resolves at runtime against the **launch area**:
/// the directory the user was in when they started the composition, captured
/// as `LaunchWorkspaceContext.launch_cwd` and threaded into the read-side
/// resolver as `file_ref_fallback_dir`. Both completion surfaces observe that
/// same launch area as their `cwd`: the `claudine __complete` process is never
/// `chdir`'d by the wrapper, and the runtime missing-property chooser runs
/// *before* `switch_process_cwd`. Anchoring value completion here keeps every
/// offered path byte-identical to what the runtime resolver accepts. Anchoring
/// at the repo root (the previous behavior) offered repo-relative paths that
/// resolved to a non-existent `<launch_cwd>/<repo-relative>` at runtime.
///
/// This deliberately differs from [`effective_repo_root`], which still governs
/// the prompt-*file* positional scopes (a prompt may legitimately live anywhere
/// in the repo's prompt roots).
pub(crate) fn property_value_root(ctx: &ScopeContext) -> &Path {
    &ctx.cwd
}

/// Case-insensitive substring match of a candidate path against a query.
///
/// `query_lower` must already be lowercased by the caller (it is compared
/// against the lowercased full path string). An empty query matches every
/// path. Shared by the ENTER-path operation-file autocomplete and the
/// provided-partial `file(match)` resolver so both apply the exact same
/// substring predicate.
pub(crate) fn path_matches_query(path: &Path, query_lower: &str) -> bool {
    if query_lower.is_empty() {
        return true;
    }
    path.to_str()
        .map(|s| s.to_ascii_lowercase().contains(query_lower))
        .unwrap_or(false)
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

    /// Iteration order for magic-path resolution (spec §5.5).
    ///
    /// Repo-local scopes — including mode-specific `docs/` and skill
    /// extras — precede `user_claudine`. This is stricter than
    /// [`iter_scopes`](Self::iter_scopes) so project-specific prompts win
    /// over global prompts in the `@` search priority. Used exclusively
    /// by the magic-path pipeline; the non-magic pipeline still uses
    /// [`iter_scopes`](Self::iter_scopes) to preserve its existing contract.
    pub(crate) fn iter_magic_scopes(&self) -> impl Iterator<Item = &Scope> {
        self.repo
            .iter()
            .chain(self.package_area.iter())
            .chain(self.package.iter())
            .chain(self.repo_claudine.iter())
            .chain(self.extras.iter())
            .chain(self.user_claudine.iter())
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
    if let Some(root) = effective_repo_root(ctx) {
        set.repo = Some(Scope {
            kind: ScopeKind::RepoPrompts,
            path: root.join(PROMPTS_DIR),
            follow_links: true,
        });
        set.repo_claudine = Some(Scope {
            kind: ScopeKind::RepoClaudinePrompts,
            path: root.join(".claudine").join(PROMPTS_DIR),
            follow_links: true,
        });
    }

    if let Some(info) = &ctx.repo_info {
        if let Some(area) = info.package_area_for_dir(&ctx.cwd)
            && area != "root"
        {
            set.package_area = Some(Scope {
                kind: ScopeKind::PackageAreaPrompts,
                path: info.root.join(area).join(PROMPTS_DIR),
                follow_links: true,
            });
        }

        if let Some(pkg) = info.package_for_dir(&ctx.cwd) {
            set.package = Some(Scope {
                kind: ScopeKind::PackagePrompts,
                path: pkg.path.join(PROMPTS_DIR),
                follow_links: true,
            });
        }
    }

    if let Some(home) = &ctx.home {
        set.user_claudine = Some(Scope {
            kind: ScopeKind::UserClaudinePrompts,
            path: home.join(".claudine").join(PROMPTS_DIR),
            follow_links: true,
        });
    }

    match mode {
        ComposeMode::Compose => {}
        ComposeMode::InlineCompose | ComposeMode::Sequence => {
            if let Some(root) = effective_repo_root(ctx) {
                set.extras.push(Scope {
                    kind: ScopeKind::RepoDocs,
                    path: root.join("docs"),
                    follow_links: true,
                });

                for peer in SKILL_PEER_DIRS {
                    set.extras.push(Scope {
                        kind: ScopeKind::AgentSkills,
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

/// Single-scope resolver for the repo / CWD directory walk.
///
/// Separate from [`resolve_compose_scopes`] because the repo-wide
/// directory walk added in review-1 finding #2 is independent of the
/// high-profile file scope set (spec §5.3). The walk root is, in order
/// of preference: the workspace root from `sniff`, the enclosing git
/// root, and finally the cwd itself when no repository is detected.
///
/// Symlinks are followed by default — directory candidates are
/// rendered by name only and dedup by canonical path on the consumer
/// side, so following a generic project-root symlink is safe.
pub(crate) fn resolve_repo_dir_walk_root(ctx: &ScopeContext) -> Scope {
    let root = effective_repo_root(ctx)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| ctx.cwd.clone());
    Scope {
        kind: ScopeKind::RepoDirWalk,
        path: root,
        follow_links: true,
    }
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
mod tests;
