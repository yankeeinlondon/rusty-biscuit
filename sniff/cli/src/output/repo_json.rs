//! Repo-action-aware JSON builders for `sniff repo` subcommands.
//!
//! This module dispatches on `RepoAction` so each `--json` subcommand can
//! return a focused serializer instead of falling through to the full
//! `RepoInfo` blob. Phase 1 wires the routing seam; later phases populate
//! per-action arms (git-status, packages, deps, locators, booleans).
//!
//! ## Notes
//!
//! - Bare `sniff repo --json` no longer reaches this builder: `commands.rs`
//!   intercepts `RepoAction::Default` and assembles the scope-complete
//!   aggregate via [`build_aggregate_value`]. The `None` / unspecialized
//!   fallback below (`serde_json::to_value(&fs.repo)`) is retained for
//!   defensive routing and keeps `sniff repo structure --json` unchanged.
//! - Builders return a plain `serde_json::Value`. Performance attachment
//!   happens once in `print_json` via `attach_performance`.
//! - Locator and boolean families need to influence the process exit code
//!   after JSON has been printed. They go through [`build_with_outcome`]
//!   instead, which returns a [`BuildOutcome`] carrying both the JSON value
//!   and an optional explicit `exit_code`. `commands.rs` is responsible for
//!   honoring that exit code after `attach_performance` + `println!`.

use std::path::PathBuf;

use serde::Serialize;
use serde_json::{Map, Value, json};
use sniff::SniffResult;
use sniff::filesystem::blast_radius::{ChangeScope, ChangedPathKind};
use sniff::filesystem::git::{BranchInfo, FileChange, GitConfig};
use sniff::filesystem::repo::types::RepoInfo;
use sniff::filesystem::repo::{
    ExternalDependencyFilter, Package, PathAttribution, RepoAggregate, attribute_paths, scope_paths,
};

use crate::args::RepoAction;
use crate::output::filesystem;
use crate::output::recent_commits::{RecentCommitsMode, commit_family_value};

/// Result returned by [`build_with_outcome`] for repo-action JSON.
///
/// `value` is the JSON payload to be printed (after `attach_performance`).
/// `exit_code`, when `Some`, instructs the caller in `commands.rs` to
/// `std::process::exit(code)` once stdout has been flushed.
///
/// The `None` exit code is the common case — most subcommands are
/// pure-data and exit `0` after `main` returns normally.
pub(crate) struct BuildOutcome {
    pub(crate) value: Value,
    pub(crate) exit_code: Option<i32>,
}

impl BuildOutcome {
    fn pure(value: Value) -> Self {
        Self {
            value,
            exit_code: None,
        }
    }

    fn with_exit(value: Value, exit_code: i32) -> Self {
        Self {
            value,
            exit_code: Some(exit_code),
        }
    }
}

/// Build the JSON value for a `sniff repo` invocation, dispatching on
/// `RepoAction` so each subcommand returns its own focused shape.
///
/// Thin wrapper over [`build_with_outcome`] that drops the `exit_code`
/// when callers don't care about it (e.g. tests focused on shape).
///
/// ## Returns
///
/// A `serde_json::Value` that mirrors the text-mode output of the given
/// `RepoAction`. When `repo_action` is `None` or set to a variant that has
/// not yet been specialized, this falls back to the full `RepoInfo` blob.
/// (Bare `sniff repo --json` does not pass through here — see the module
/// note; it is handled as the aggregate in `commands.rs`.)
#[cfg(test)]
pub(crate) fn build(
    result: &SniffResult,
    repo_action: Option<&RepoAction>,
    base_dir: Option<&std::path::Path>,
) -> Value {
    build_with_outcome(result, repo_action, base_dir).value
}

/// Build the JSON value plus an optional explicit exit code.
///
/// Locator and boolean subcommands need to influence the process exit
/// code after the JSON has been emitted (mirroring text-mode exit-code
/// semantics). They use this entry point; `commands.rs` honors
/// `BuildOutcome::exit_code` after printing.
pub(crate) fn build_with_outcome(
    result: &SniffResult,
    repo_action: Option<&RepoAction>,
    base_dir: Option<&std::path::Path>,
) -> BuildOutcome {
    match repo_action {
        // Unspecialized / defensive fallback — the full RepoInfo blob. Bare
        // `sniff repo --json` is routed to the aggregate in `commands.rs` and
        // does not reach this arm.
        None => BuildOutcome::pure(fallback_repo_value(result)),
        // `sniff repo structure --json` honors `--filter` so JSON consumers
        // get the same scoped package list as text mode. `--latest-versions`
        // enrichment is applied in `commands.rs` *before* this builder is
        // called, so `repo.packages` already carries the enriched
        // `DependencyEntry` fields (`latest_version`, `is_updatable`,
        // `has_major_update`); we serialize the repo as-is.
        Some(RepoAction::Structure {
            filter,
            package,
            package_area,
            ..
        }) => BuildOutcome::pure(structure_value(
            result,
            filter,
            package.as_deref(),
            package_area.as_deref(),
        )),
        // `git-status --json` returns the focused `GitInfo` object.
        // Package scoping is performed in `commands.rs` between detection
        // and serialization, so we just serialize whatever git data lives
        // on `result.filesystem` at this point.
        Some(RepoAction::GitStatus { .. }) => BuildOutcome::pure(git_status_value(result)),
        // Phase 3: dirty / staged / unstaged package + package-area families
        // emit `{ scope, kind, names }` so JSON consumers can scope
        // automation by lifecycle (dirty/staged/unstaged) and
        // granularity (packages/areas).
        Some(RepoAction::DirtyPackages {
            filter,
            package,
            package_area,
        }) => BuildOutcome::pure(package_family_value(
            "dirty",
            "packages",
            filesystem::select_dirty_package_names(
                result,
                filter,
                package.as_deref(),
                package_area.as_deref(),
            ),
        )),
        Some(RepoAction::DirtyPackageAreas {
            filter,
            package,
            package_area,
        }) => BuildOutcome::pure(package_family_value(
            "dirty",
            "package_areas",
            filesystem::select_dirty_package_area_names(
                result,
                filter,
                package.as_deref(),
                package_area.as_deref(),
            ),
        )),
        Some(RepoAction::StagedPackages {
            filter,
            package,
            package_area,
        }) => BuildOutcome::pure(package_family_value(
            "staged",
            "packages",
            filesystem::select_staged_package_names(
                result,
                filter,
                package.as_deref(),
                package_area.as_deref(),
            ),
        )),
        Some(RepoAction::StagedPackageAreas {
            filter,
            package,
            package_area,
        }) => BuildOutcome::pure(package_family_value(
            "staged",
            "package_areas",
            filesystem::select_staged_package_area_names(
                result,
                filter,
                package.as_deref(),
                package_area.as_deref(),
            ),
        )),
        Some(RepoAction::UnstagedPackages {
            filter,
            package,
            package_area,
        }) => BuildOutcome::pure(package_family_value(
            "unstaged",
            "packages",
            filesystem::select_unstaged_package_names(
                result,
                filter,
                package.as_deref(),
                package_area.as_deref(),
            ),
        )),
        Some(RepoAction::UnstagedPackageAreas {
            filter,
            package,
            package_area,
        }) => BuildOutcome::pure(package_family_value(
            "unstaged",
            "package_areas",
            filesystem::select_unstaged_package_area_names(
                result,
                filter,
                package.as_deref(),
                package_area.as_deref(),
            ),
        )),
        // Phase 4: locator family — `{ root }` / `{ name }`.
        //
        // Text mode `exit(1)` when the path/name resolves to empty. JSON
        // mirrors that exit-code semantics so scripts can branch on `$?`
        // without parsing the JSON. We still emit the (empty) JSON object
        // so consumers always see a stable shape.
        Some(RepoAction::PackageRoot) => {
            locator_root_outcome(filesystem::render_repo_package_root(result, base_dir))
        }
        Some(RepoAction::PackageAreaRoot) => {
            locator_root_outcome(filesystem::render_repo_package_area_root(result, base_dir))
        }
        // Phase 4: boolean family — `{ dirty }` / `{ has_source_code_changes }`.
        //
        // `has_merge_conflict` is built directly in `commands.rs` since the
        // detection runs before the heavy `detect_with_plan` pass. The JSON
        // shape there matches the contract documented here.
        Some(RepoAction::IsCurrentPackageAreaDirty) => {
            let dirty =
                filesystem::current_package_area_is_dirty(result, base_dir).unwrap_or(false);
            BuildOutcome::with_exit(json!({ "dirty": dirty }), if dirty { 0 } else { 1 })
        }
        Some(RepoAction::PackageAreaHasSourceCodeChanges) => {
            let has = filesystem::package_area_source_code_change_count(result, base_dir)
                .map(|(b, _, _)| b)
                .unwrap_or(false);
            BuildOutcome::with_exit(
                json!({ "has_source_code_changes": has }),
                if has { 0 } else { 1 },
            )
        }
        // `repo language --json` emits `{ "language": "Rust" }` (or
        // `{ "language": null }` when no primary language can be detected).
        // Exit code mirrors the text path: 0 on success, 1 on null, so scripts
        // can branch on `$?` without parsing the JSON body.
        Some(RepoAction::Worktree {
            no_error,
            on_error: _,
        }) => {
            // This should not normally be reached because Worktree is handled
            // as an early return in commands.rs, but we handle it here for
            // completeness in the JSON builder.
            let dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let name = sniff::filesystem::git::get_current_worktree_name(&dir)
                .ok()
                .flatten();
            worktree_outcome(name.as_deref(), *no_error)
        }
        Some(RepoAction::Language { breakdown: false }) => {
            let name = filesystem::primary_language_name(result);
            let exit_code = if name.is_none() { Some(1) } else { None };
            BuildOutcome {
                value: json!({ "language": name }),
                exit_code,
            }
        }
        Some(RepoAction::Language { breakdown: true }) => {
            // Breakdown mode: emit the full language breakdown like the old `sniff language --json`
            if let Some(ref fs) = result.filesystem {
                BuildOutcome::pure(serde_json::to_value(&fs.languages).unwrap_or(Value::Null))
            } else {
                BuildOutcome::pure(json!({}))
            }
        }
        // Phase 5: `package-dependencies --json` emits a hand-built per-package object so
        // future fields on `Package` don't leak into the public contract.
        // The `ui` flag is text-only and is intentionally ignored in JSON.
        Some(RepoAction::PackageDependencies {
            filter,
            ui: _,
            svg: _,
            package,
            package_area,
            width: _,
            orientation: _,
        }) => BuildOutcome::pure(build_deps_value(
            result,
            filter,
            package.as_deref(),
            package_area.as_deref(),
        )),
        // All other actions fall through to today's behavior. Later phases
        // (6) replace these fall-throughs with focused builders for the
        // commit families.
        _ => BuildOutcome::pure(fallback_repo_value(result)),
    }
}

/// Build the JSON outcome for a locator (path / area-root) command.
///
/// When `rendered` is empty the text path exits with status `1`; JSON
/// mirrors that by setting the exit code while still emitting an
/// `{ "root": "" }` object so consumers see a stable shape.
fn locator_root_outcome(rendered: String) -> BuildOutcome {
    let exit_code = if rendered.is_empty() { Some(1) } else { None };
    let value = json!({ "root": rendered });
    BuildOutcome { value, exit_code }
}

/// Build the JSON outcome for a `{ name }` locator command (`package` /
/// `package-area`).
///
/// Used by `commands.rs` from inside the early-return arms for `Package`
/// and `PackageArea` so that JSON consumers always see `{ "name": "..." }`
/// while exit codes continue to honour `--no-error` / `--on-error`.
///
/// ## Notes
///
/// When `rendered` is empty this returns `Some(1)` for `exit_code` so
/// future callers that route empty-name results through `BuildOutcome`
/// can surface the failure exit. The current callers in `commands.rs`
/// continue to handle exit codes directly because they need to honour
/// the `--no-error` / `--on-error` flags before exiting.
pub(crate) fn name_outcome(rendered: String) -> BuildOutcome {
    let exit_code = if rendered.is_empty() { Some(1) } else { None };
    BuildOutcome {
        value: json!({ "name": rendered }),
        exit_code,
    }
}

/// Build the JSON outcome for `has-merge-conflict --json`.
///
/// Returns `{ "has_merge_conflict": bool }` with exit code `0` when a
/// conflict is present and `1` otherwise — matching the text-mode
/// behaviour in `commands.rs` (where the conflict is detected outside the
/// normal detection pass).
pub(crate) fn has_merge_conflict_outcome(has_conflict: bool) -> BuildOutcome {
    BuildOutcome::with_exit(
        json!({ "has_merge_conflict": has_conflict }),
        if has_conflict { 0 } else { 1 },
    )
}

/// Build the JSON outcome for `repo worktree --json`.
///
/// Returns `{ "worktree": "name" }` on success and `{ "worktree": null }`
/// on failure. Exit code is `0` when a worktree is found and `1` otherwise
/// (or `0` when `no_error` is `true`).
pub(crate) fn worktree_outcome(name: Option<&str>, no_error: bool) -> BuildOutcome {
    let exit_code = if name.is_some() {
        None
    } else if no_error {
        Some(0)
    } else {
        Some(1)
    };
    BuildOutcome {
        value: json!({ "worktree": name }),
        exit_code,
    }
}

/// Build the JSON outcome for `repo is-monorepo --json`.
///
/// Inside a monorepo this returns the object
/// `{ "is_monorepo": true, "authority": "<kebab-id>", "orchestrators": [...] }`,
/// omitting `orchestrators` when empty. Outside a monorepo it returns
/// `{ "is_monorepo": false }` with an exit code of `1` (or `0` when
/// `no_error` is `true`).
pub(crate) fn is_monorepo_outcome(info: Option<&RepoInfo>, no_error: bool) -> BuildOutcome {
    match info {
        Some(repo) if repo.is_monorepo => {
            let layer = repo
                .primary_layer()
                .expect("is_monorepo implies at least one membership layer");
            let mut value = json!({
                "is_monorepo": true,
            });
            value["authority"] = json!(layer.authority.spec().id);
            if !layer.orchestrators.is_empty() {
                let orchestrators: Vec<&str> =
                    layer.orchestrators.iter().map(|s| s.spec().id).collect();
                value["orchestrators"] = json!(orchestrators);
            }
            BuildOutcome::pure(value)
        }
        _ => {
            let exit_code = if no_error { 0 } else { 1 };
            BuildOutcome::with_exit(json!({ "is_monorepo": false }), exit_code)
        }
    }
}

/// Build the JSON outcome for `repo package-count --json`.
///
/// Returns `{ "package-count": N }` with exit code `0`.
pub(crate) fn package_count_outcome(count: usize) -> BuildOutcome {
    BuildOutcome::pure(json!({ "package-count": count }))
}

/// Build the JSON value for `repo worktrees --json`.
///
/// Returns `{ "worktrees": [ { name, branch, path, current, detached }, ... ] }`.
pub(crate) fn worktrees_value(entries: &[sniff::filesystem::git::WorktreeEntry]) -> Value {
    json!({
        "worktrees": entries.iter().map(|e| json!({
            "name": e.name,
            "branch": e.branch,
            "path": e.path,
            "current": e.is_current,
            "detached": e.is_detached,
        })).collect::<Vec<_>>(),
    })
}

/// Build the JSON value for a file-list subcommand.
///
/// Returns `{ "scope": "...", "kind": "...", "paths": [] }` without
/// exiting, so the bare `repo --json` aggregate can include every file-list
/// child even when there are no matching changed files.
pub(crate) fn file_list_value(
    scope: ChangeScope,
    kind: ChangedPathKind,
    paths: &[std::path::PathBuf],
) -> Value {
    json!({
        "scope": scope,
        "kind": kind,
        "paths": paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
    })
}

/// Build the `{ scope, kind, names }` JSON shape used by the package and
/// package-area families.
///
/// `scope` is `"dirty"`, `"staged"`, or `"unstaged"`. `kind` is `"packages"`
/// or `"package_areas"`. JSON consumers always see an array (never a prose
/// "only intended for monorepo" error string) — non-monorepo repos return
/// an empty `names` array.
fn package_family_value(scope: &str, kind: &str, names: Vec<String>) -> Value {
    json!({
        "scope": scope,
        "kind": kind,
        "names": names,
    })
}

/// Build the JSON value for `sniff repo package-dependencies --json`.
///
/// Returns `{ "packages": [ ... ] }` where each entry is a hand-built
/// object with a narrow allowlist of fields:
/// `name`, `depends_on`, `used_by`, `dependencies`, `dev_dependencies`,
/// and (only when non-empty) `peer_dependencies` / `optional_dependencies`.
///
/// ## Notes
///
/// Hand-building (instead of `serde_json::to_value(&pkg)`) is deliberate:
/// it keeps the public `package-dependencies --json` contract narrow so future fields on
/// `Package` (e.g. languages, documentation, configuration) don't silently
/// leak into the output.
pub(crate) fn build_deps_value(
    result: &SniffResult,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> Value {
    let Some(fs) = result.filesystem.as_ref() else {
        return json!({ "packages": [] });
    };
    let Some(repo) = fs.repo.as_ref() else {
        return json!({ "packages": [] });
    };
    let entries = build_deps_entries(repo, repo_filter, package, package_area);
    json!({ "packages": entries })
}

/// Build the per-package JSON entries for `package-dependencies --json`.
///
/// Split out from [`build_deps_value`] so unit tests can construct a
/// `RepoInfo` fixture directly without wrapping it in a `SniffResult`.
fn build_deps_entries(
    repo: &RepoInfo,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> Vec<Value> {
    let packages = repo.packages.as_deref().unwrap_or(&[]);
    let filtered = filesystem::select_repo_packages(packages, repo_filter, package, package_area);
    filtered.into_iter().map(build_deps_package_entry).collect()
}

/// Build a single `package-dependencies --json` package entry from a `Package`.
///
/// The field allowlist is intentional — see [`build_deps_value`].
fn build_deps_package_entry(pkg: &Package) -> Value {
    let mut map = Map::new();
    map.insert("name".into(), Value::String(pkg.name.clone()));
    map.insert(
        "depends_on".into(),
        serde_json::to_value(&pkg.depends_on).expect("Vec<String> serializes"),
    );
    map.insert(
        "used_by".into(),
        serde_json::to_value(&pkg.used_by).expect("Vec<String> serializes"),
    );
    map.insert(
        "dependencies".into(),
        serde_json::to_value(pkg.dependencies.as_deref().unwrap_or(&[]))
            .expect("DependencyEntry serializes"),
    );
    map.insert(
        "dev_dependencies".into(),
        serde_json::to_value(pkg.dev_dependencies.as_deref().unwrap_or(&[]))
            .expect("DependencyEntry serializes"),
    );

    // Only include optional families when they have entries — matches the
    // sparse spec shape and avoids noisy empty arrays for ecosystems that
    // don't use peer/optional deps (e.g. Cargo).
    if let Some(peer) = pkg.peer_dependencies.as_deref()
        && !peer.is_empty()
    {
        map.insert(
            "peer_dependencies".into(),
            serde_json::to_value(peer).expect("DependencyEntry serializes"),
        );
    }
    if let Some(optional) = pkg.optional_dependencies.as_deref()
        && !optional.is_empty()
    {
        map.insert(
            "optional_dependencies".into(),
            serde_json::to_value(optional).expect("DependencyEntry serializes"),
        );
    }

    Value::Object(map)
}

/// Build the JSON value for `sniff repo git-status --json`.
///
/// Returns the `GitInfo` object directly. When git information is unavailable
/// (e.g. the working directory is not inside a git repository), an empty
/// object is returned so JSON consumers always see an object shape.
fn git_status_value(result: &SniffResult) -> Value {
    if let Some(ref fs) = result.filesystem
        && let Some(ref git) = fs.git
    {
        serde_json::to_value(git).unwrap_or(Value::Null)
    } else {
        json!({})
    }
}

/// Build the JSON value for `sniff repo structure --json`, applying
/// the `--filter` argument to scope the `packages` array.
///
/// When no filter is provided the output mirrors [`fallback_repo_value`]
/// exactly. With a filter, `repo.packages` is replaced with the matching
/// subset; all other `RepoInfo` fields are preserved so downstream JSON
/// consumers continue to see monorepo flags, `root`, `monorepo_layers`,
/// `monorepo_standards`, and aggregated dependency rollups.
///
/// `--latest-versions` enrichment is applied in `commands.rs` before this
/// builder runs, so per-package `latest_version` / `is_updatable` /
/// `has_major_update` fields automatically carry through.
fn structure_value(
    result: &SniffResult,
    filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> Value {
    let Some(fs) = result.filesystem.as_ref() else {
        return json!({});
    };
    let Some(repo) = fs.repo.as_ref() else {
        return json!({});
    };

    if filter.is_empty() && package.is_none() && package_area.is_none() {
        return serde_json::to_value(repo).unwrap_or(Value::Null);
    }

    let packages = repo.packages.as_deref().unwrap_or(&[]);
    let filtered: Vec<Package> =
        filesystem::select_repo_packages(packages, filter, package, package_area)
            .into_iter()
            .cloned()
            .collect();

    let mut repo_clone = repo.clone();
    repo_clone.packages = Some(filtered);
    serde_json::to_value(&repo_clone).unwrap_or(Value::Null)
}

#[derive(Debug, Serialize)]
struct SniffRepo {
    name: String,
    version: Option<String>,
    language: Option<String>,
    is_monorepo: bool,
    package_count: usize,
    root: String,
    structure: AggregateStructure,
    packages: Vec<String>,
    package_areas: Vec<String>,
    package_manager: Value,
    test_runner: Value,
    package_dependencies: Value,
    dependencies: Value,
    git_status: AggregateGitStatus,
    branches: Vec<BranchInfo>,
    worktrees: Vec<AggregateWorktree>,
    context: AggregateContext,
    dirty: ScopeBucket,
    staged: ScopeBucket,
    unstaged: ScopeBucket,
    untracked: ScopeBucket,
    has_merge_conflict: bool,
    recent_commits: Value,
    source_code_changes: Value,
    documentation_changes: Value,
}

#[derive(Debug, Serialize)]
struct AggregateStructure {
    is_monorepo: bool,
    root: PathBuf,
    monorepo_standards: Value,
    monorepo_layers: Value,
}

#[derive(Debug, Serialize)]
struct AggregateContext {
    package: String,
    package_area: String,
    area: String,
    package_root: String,
    package_area_root: String,
    worktree: Option<String>,
    is_current_package_area_dirty: bool,
    package_area_has_source_code_changes: bool,
}

#[derive(Debug, Serialize)]
struct AggregateGitStatus {
    current_branch: Option<String>,
    config: GitConfig,
    file_changes: Vec<AggregateFileChange>,
    is_dirty: bool,
    staged_count: usize,
    unstaged_count: usize,
    untracked_count: usize,
}

#[derive(Debug, Serialize)]
struct AggregateFileChange {
    path: PathBuf,
    status: &'static str,
    lines_added: usize,
    lines_removed: usize,
}

#[derive(Debug, Serialize)]
struct AggregateWorktree {
    name: String,
    branch: Option<String>,
    path: PathBuf,
    current: bool,
    detached: bool,
}

#[derive(Debug, Serialize)]
struct ScopeBucket {
    files: Vec<String>,
    source_code: Vec<String>,
    documentation: Vec<String>,
    packages: Vec<String>,
    package_areas: Vec<String>,
}

/// Assemble the consolidated aggregate for bare `sniff repo --json`.
///
/// Returns the `SniffRepo` projection with snake_case keys, compact git status,
/// top-level branches/worktrees, grouped change scopes, and deduplicated
/// package/package-area catalogs. Focused subcommands keep their richer JSON
/// shapes.
///
/// ## Notes
///
/// A **pure projection** over facts already observed by the detection pass and
/// by [`observe_repo_aggregate`]: it performs no filesystem read, repository
/// open, subprocess spawn, or network request (umbrella spec R2.7). The four
/// scope buckets are derived from the one `GitInfo.file_changes` collection via
/// [`scope_paths`], which is what removed this builder's eight post-detection
/// status walks, and the cwd-relative `context` block is resolved during
/// observation over a single package ownership index. Keep it that way — an
/// observation added here is paid on every bare `sniff repo --json`.
///
/// [`observe_repo_aggregate`]: sniff::filesystem::repo::observe_repo_aggregate
pub(crate) fn build_aggregate_value(result: &SniffResult, aggregate: &RepoAggregate) -> Value {
    let identity = &aggregate.identity;
    let repo = aggregate.repo.as_ref();
    let packages = repo
        .map(|repo| {
            filesystem::collect_repo_package_names(repo, &[], None, None)
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let package_areas = repo
        .map(|repo| {
            filesystem::collect_repo_package_area_names(repo, &[], None, None)
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let structure = aggregate_structure(repo);
    let worktrees = aggregate
        .worktrees
        .iter()
        .map(|entry| AggregateWorktree {
            name: entry.name.clone(),
            branch: entry.branch.clone(),
            path: entry.path.clone(),
            current: entry.is_current,
            detached: entry.is_detached,
        })
        .collect();
    let context = &aggregate.context;
    let commit_set = &aggregate.commits;
    let value = SniffRepo {
        name: identity.name.clone(),
        version: aggregate.version.clone(),
        language: filesystem::primary_language_name(result),
        is_monorepo: identity.is_monorepo,
        // Count the canonical catalog, not `identity.package_count` (which is
        // `None` for non-monorepos). A recognized standalone project counts as
        // one package, keeping `package_count` consistent with `packages`.
        package_count: packages.len(),
        root: filesystem::render_repo_root(result),
        structure,
        packages,
        package_areas,
        package_manager: aggregate_package_manager(repo),
        test_runner: aggregate_test_runner(repo),
        package_dependencies: match repo {
            Some(repo) => json!({ "packages": build_deps_entries(repo, &[], None, None) }),
            None => json!({ "packages": [] }),
        },
        dependencies: aggregate_external_dependencies(repo),
        git_status: aggregate_git_status(result),
        branches: aggregate.branches.clone(),
        worktrees,
        context: AggregateContext {
            package: context.package.clone(),
            package_area: context.package_area.clone(),
            area: context.area.clone(),
            package_root: context.package_root.clone(),
            package_area_root: context.package_area_root.clone(),
            worktree: aggregate.current_worktree.clone(),
            is_current_package_area_dirty: context.is_current_package_area_dirty,
            package_area_has_source_code_changes: context.package_area_has_source_code_changes,
        },
        dirty: scope_bucket(result, ChangeScope::Dirty),
        staged: scope_bucket(result, ChangeScope::Staged),
        unstaged: scope_bucket(result, ChangeScope::Unstaged),
        untracked: scope_bucket(result, ChangeScope::Untracked),
        has_merge_conflict: aggregate.has_merge_conflict,
        recent_commits: aggregate_commit_family_value(commit_set, RecentCommitsMode::RecentCommits),
        source_code_changes: aggregate_commit_family_value(
            commit_set,
            RecentCommitsMode::SourceCodeChanges,
        ),
        documentation_changes: aggregate_commit_family_value(
            commit_set,
            RecentCommitsMode::DocumentationChanges,
        ),
    };

    serde_json::to_value(value).unwrap_or(Value::Null)
}

fn aggregate_structure(repo: Option<&RepoInfo>) -> AggregateStructure {
    match repo {
        Some(repo) => AggregateStructure {
            is_monorepo: repo.is_monorepo,
            root: repo.root.clone(),
            monorepo_standards: serde_json::to_value(&repo.monorepo_standards)
                .unwrap_or_else(|_| json!([])),
            monorepo_layers: serde_json::to_value(&repo.monorepo_layers)
                .unwrap_or_else(|_| json!([])),
        },
        None => AggregateStructure {
            is_monorepo: false,
            root: PathBuf::new(),
            monorepo_standards: json!([]),
            monorepo_layers: json!([]),
        },
    }
}

/// Collapse package-manager usage across every package into the aggregate's
/// repo-wide `package_manager` fact (`string | string[] | null`).
///
/// Mirrors `sniff repo package-manager` at repo scope: a uniform set collapses
/// to a single string, divergent sets to a list, and no declared manager to
/// `null`.
fn aggregate_package_manager(repo: Option<&RepoInfo>) -> Value {
    use sniff::filesystem::repo::{AggregateScope, aggregate_package_values};

    let Some(packages) = repo.and_then(|repo| repo.packages.as_deref()) else {
        return Value::Null;
    };
    let result = aggregate_package_values(
        packages,
        &AggregateScope::Repo,
        |pkg| pkg.package_managers.clone(),
        |manager: &String| manager.clone(),
    );
    aggregate_result_to_value(&result, |manager| manager.clone())
}

/// Collapse declared test-runner usage across every package into the
/// aggregate's repo-wide `test_runner` fact (`string | string[] | null`).
///
/// Mirrors `sniff repo test-runner` at repo scope, emitting each runner's
/// display name.
fn aggregate_test_runner(repo: Option<&RepoInfo>) -> Value {
    use sniff::filesystem::repo::{AggregateScope, TestRunnerUsage, aggregate_package_values};

    let Some(packages) = repo.and_then(|repo| repo.packages.as_deref()) else {
        return Value::Null;
    };
    let result = aggregate_package_values(
        packages,
        &AggregateScope::Repo,
        |pkg| pkg.test_runners.clone(),
        |usage: &TestRunnerUsage| usage.runner,
    );
    aggregate_result_to_value(&result, |usage| usage.display_name().to_string())
}

/// Render an [`AggregateResult`] as the spec's `string | string[] | null`
/// shape: `Singular` → string, `Multiple` → array, `Empty` → `null`.
fn aggregate_result_to_value<T>(
    result: &sniff::filesystem::repo::AggregateResult<T>,
    render: impl Fn(&T) -> String,
) -> Value {
    use sniff::filesystem::repo::AggregateResult;

    match result {
        AggregateResult::Singular(value) => Value::String(render(value)),
        AggregateResult::Multiple(values) => {
            Value::Array(values.iter().map(|value| Value::String(render(value))).collect())
        }
        AggregateResult::Empty => Value::Null,
    }
}

/// Build the aggregate's repo-wide `dependencies` projection.
///
/// External dependencies are a group-A repo-wide fact: the aggregate must
/// report the same set regardless of where in the tree bare `sniff repo --json`
/// is invoked, so the scope is always [`AggregateScope::Repo`]. cwd-relative
/// scoping lives only on the focused `sniff repo dependencies` command.
fn aggregate_external_dependencies(repo: Option<&RepoInfo>) -> Value {
    use sniff::filesystem::repo::{AggregateScope, collect_external_dependencies};

    let Some(repo) = repo else {
        return json!({ "dependencies": [] });
    };
    json!({
        "dependencies": collect_external_dependencies(repo, &AggregateScope::Repo, ExternalDependencyFilter::all())
    })
}

fn aggregate_git_status(result: &SniffResult) -> AggregateGitStatus {
    let Some(git) = result.filesystem.as_ref().and_then(|fs| fs.git.as_ref()) else {
        return AggregateGitStatus {
            current_branch: None,
            config: GitConfig::default(),
            file_changes: Vec::new(),
            is_dirty: false,
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
        };
    };

    let status = git.status.as_ref();
    AggregateGitStatus {
        current_branch: git.current_branch.clone(),
        config: git.config.clone(),
        file_changes: git.file_changes.iter().map(aggregate_file_change).collect(),
        is_dirty: status.is_some_and(|status| status.is_dirty),
        staged_count: status.map_or(0, |status| status.staged_count),
        unstaged_count: status.map_or(0, |status| status.unstaged_count),
        untracked_count: status.map_or(0, |status| status.untracked_count),
    }
}

fn aggregate_file_change(change: &FileChange) -> AggregateFileChange {
    AggregateFileChange {
        path: change.path.clone(),
        status: change.action.label(),
        lines_added: change.lines_added,
        lines_removed: change.lines_removed,
    }
}

/// Project one scope bucket from the detected `file_changes`.
///
/// `collect_changed_paths` sorts and dedups its result; [`scope_paths`]
/// preserves `file_changes` order, so this sorts to match the previous output
/// byte-for-byte.
fn scope_bucket(result: &SniffResult, scope: ChangeScope) -> ScopeBucket {
    let file_changes = result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.git.as_ref())
        .map(|git| git.file_changes.as_slice())
        .unwrap_or(&[]);

    let mut files = scope_paths(file_changes, scope);
    files.sort();

    let source_code: Vec<PathBuf> = files
        .iter()
        .filter(|path| sniff::filesystem::blast_radius::is_source_code_path(path))
        .cloned()
        .collect();
    let documentation: Vec<PathBuf> = files
        .iter()
        .filter(|path| sniff::filesystem::blast_radius::is_documentation_path(path))
        .cloned()
        .collect();
    let attribution = changed_path_attribution(result, &files);

    ScopeBucket {
        files: paths_to_strings(&files),
        source_code: paths_to_strings(&source_code),
        documentation: paths_to_strings(&documentation),
        packages: attribution.packages,
        package_areas: attribution.package_areas,
    }
}

fn paths_to_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

/// Attribute changed paths to packages, for monorepos only.
///
/// A non-monorepo reports empty catalogs: its single package would otherwise
/// claim every changed path, which says nothing.
fn changed_path_attribution(result: &SniffResult, paths: &[PathBuf]) -> PathAttribution {
    let Some(repo) = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref()) else {
        return PathAttribution::default();
    };
    if !repo.is_monorepo {
        return PathAttribution::default();
    }
    let Some(packages) = repo.packages.as_deref() else {
        return PathAttribution::default();
    };
    attribute_paths(packages, paths)
}

fn aggregate_commit_family_value(
    commit_set: &sniff::filesystem::git::recent_commits::CommitDescSet,
    mode: RecentCommitsMode,
) -> Value {
    let mut value = commit_family_value(commit_set, mode);
    if let Some(obj) = value.as_object_mut() {
        obj.remove("repo_root");
        obj.remove("packages");
        obj.remove("filter");
        if let Some(period) = obj.remove("period_label") {
            obj.insert("period".into(), json!({ "label": period }));
        }
    }
    value
}

/// Build the legacy full-`RepoInfo` JSON value (today's behavior).
fn fallback_repo_value(result: &SniffResult) -> Value {
    if let Some(ref fs) = result.filesystem {
        serde_json::to_value(&fs.repo).unwrap_or(Value::Null)
    } else {
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniff::SniffResult;
    use sniff::filesystem::FilesystemInfo;
    use sniff::filesystem::git::{GitConfig, GitInfo, RepoStatus};
    use sniff::filesystem::repo::types::RepoInfo;
    use sniff::filesystem::repo::{MonorepoLayer, MonorepoStandard};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    fn fixture_git_info() -> GitInfo {
        GitInfo {
            repo_root: PathBuf::from("/tmp/repo"),
            org: Some("rusty-biscuit".to_string()),
            repo: Some("sniff".to_string()),
            current_branch: Some("main".to_string()),
            // Status-bearing GitInfo (non-identity) never carries `head_id`.
            head_id: None,
            branches: Vec::new(),
            in_worktree: false,
            base_repo_root: None,
            recent: Vec::new(),
            status: Some(RepoStatus::default()),
            remotes: Vec::new(),
            worktrees: HashMap::new(),
            config: GitConfig::default(),
            tracking: Vec::new(),
            file_changes: Vec::new(),
            aggregate: None,
        }
    }

    fn fixture_with_git_and_repo() -> SniffResult {
        let repo = RepoInfo {
            is_monorepo: true,
            root: PathBuf::from("/tmp/repo"),
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            monorepo_standards: Vec::new(),
            monorepo_layers: Vec::new(),
            packages: None,
        };
        let filesystem = FilesystemInfo {
            repo: Some(repo),
            git: Some(fixture_git_info()),
            ..Default::default()
        };
        SniffResult {
            os: None,
            hardware: None,
            network: None,
            filesystem: Some(filesystem),
            performance: None,
        }
    }

    fn git_status_action() -> RepoAction {
        RepoAction::GitStatus {
            history: 10,
            refresh_remotes: false,
            compact: false,
            package: None,
            package_area: None,
            branch: None,
            worktree: None,
        }
    }

    fn fixture_with_repo() -> SniffResult {
        let repo = RepoInfo {
            is_monorepo: true,
            root: PathBuf::from("/tmp/repo"),
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            monorepo_standards: Vec::new(),
            monorepo_layers: Vec::new(),
            packages: None,
        };
        let filesystem = FilesystemInfo {
            repo: Some(repo),
            ..Default::default()
        };
        SniffResult {
            os: None,
            hardware: None,
            network: None,
            filesystem: Some(filesystem),
            performance: None,
        }
    }

    #[test]
    fn build_with_no_action_returns_full_repo_info() {
        let result = fixture_with_repo();
        let value = build(&result, None, None);

        // Should be the full RepoInfo object — `is_monorepo` is a top-level field.
        assert!(value.is_object(), "expected a JSON object, got: {value}");
        assert_eq!(value["is_monorepo"], serde_json::Value::Bool(true));
    }

    #[test]
    fn build_with_structure_action_matches_no_action() {
        let result = fixture_with_repo();
        let action = RepoAction::Structure {
            filter: Vec::new(),
            latest_versions: false,
            package: None,
            package_area: None,
        };

        let bare = build(&result, None, None);
        let structure = build(&result, Some(&action), None);

        assert_eq!(
            bare, structure,
            "bare `repo` and `repo structure` JSON should match"
        );
    }

    #[test]
    fn build_without_filesystem_returns_empty_object() {
        let result = SniffResult {
            os: None,
            hardware: None,
            network: None,
            filesystem: None,
            performance: None,
        };
        let value = build(&result, None, None);
        assert_eq!(value, json!({}));
    }

    #[test]
    fn build_git_status_returns_git_info_shape() {
        let result = fixture_with_git_and_repo();
        let action = git_status_action();
        let value = build(&result, Some(&action), None);

        assert!(value.is_object(), "expected an object, got: {value}");

        // GitInfo fields are at the top level
        assert!(
            value.get("repo_root").is_some(),
            "GitInfo `repo_root` should be top-level: {value}"
        );
        assert!(
            value.get("status").is_some(),
            "GitInfo `status` should be top-level: {value}"
        );
        assert!(
            value.get("recent").is_some(),
            "GitInfo `recent` should be top-level: {value}"
        );
        assert!(
            value.get("branches").is_some(),
            "GitInfo `branches` should be top-level: {value}"
        );

        // Status-bearing git-status JSON must not gain the identity-only
        // `head_id` field — its shape is unchanged by the identity request work.
        assert!(
            value.get("head_id").is_none(),
            "git-status JSON should NOT contain identity-only `head_id`: {value}"
        );

        // RepoInfo-only fields must NOT appear at the top level
        assert!(
            value.get("is_monorepo").is_none(),
            "`is_monorepo` is a RepoInfo field; should not appear in git-status JSON: {value}"
        );
        assert!(
            value.get("packages").is_none(),
            "`packages` is a RepoInfo field; should not appear in git-status JSON: {value}"
        );
    }

    #[test]
    fn build_git_status_without_git_returns_empty_object() {
        // FilesystemInfo present but no git info — e.g. detection ran in a
        // non-git directory.
        let filesystem = FilesystemInfo::default();
        let result = SniffResult {
            os: None,
            hardware: None,
            network: None,
            filesystem: Some(filesystem),
            performance: None,
        };
        let action = git_status_action();
        let value = build(&result, Some(&action), None);
        assert_eq!(value, json!({}));
    }

    #[test]
    fn build_git_status_without_filesystem_returns_empty_object() {
        let result = SniffResult {
            os: None,
            hardware: None,
            network: None,
            filesystem: None,
            performance: None,
        };
        let action = git_status_action();
        let value = build(&result, Some(&action), None);
        assert_eq!(value, json!({}));
    }

    mod package_family {
        use super::*;
        use sniff::filesystem::FilesystemInfo;
        use sniff::filesystem::git::types::DirtyFile;
        use sniff::filesystem::git::{FileAction, FileChange, FileStatus};
        use sniff::filesystem::repo::Package;
        use sniff::filesystem::repo::types::RepoInfo;

        fn make_package(name: &str, area: &str) -> Package {
            Package {
                path: PathBuf::from(format!("/tmp/repo/{area}/{name}")),
                relative: format!("{area}/{name}"),
                package_area: area.to_string(),
                name: name.to_string(),
                ecosystem: sniff::filesystem::repo::PackageEcosystem::Unknown,
                standard: sniff::filesystem::repo::MonorepoStandard::Unknown,
                provenance: sniff::filesystem::repo::PackageProvenance::ManifestScan,
                nested_packages: vec![],
                primary_language: None,
                secondary_languages: vec![],
                languages: vec![],
                frameworks: vec![],
                file_associations: vec![],
                configuration: vec![],
                documentation: vec![],
                editor_config: None,
                command_runner: vec![],
                package_managers: vec![],
                test_runners: vec![],
                version: None,
                features: vec![],
                depends_on: vec![],
                used_by: vec![],
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                is_updatable: None,
                has_major_update: None,
                is_excluded: false,
            }
        }

        fn fixture(monorepo: bool, dirty_paths: &[&str], staged_paths: &[&str]) -> SniffResult {
            let packages = vec![
                make_package("alpha", "area-a"),
                make_package("beta", "area-b"),
            ];
            let repo = RepoInfo {
                is_monorepo: monorepo,
                root: PathBuf::from("/tmp/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                monorepo_standards: Vec::new(),
                monorepo_layers: Vec::new(),
                packages: Some(packages),
            };
            let mut git = fixture_git_info();
            git.status.as_mut().unwrap().dirty = dirty_paths
                .iter()
                .map(|p| DirtyFile {
                    filepath: PathBuf::from(p),
                    absolute_filepath: PathBuf::from(format!("/tmp/repo/{p}")),
                    diff: String::new(),
                    last_local_commit: String::new(),
                    origin_commit: None,
                })
                .collect();
            git.file_changes = staged_paths
                .iter()
                .map(|p| FileChange {
                    path: PathBuf::from(p),
                    status: FileStatus::Staged,
                    action: FileAction::Modified,
                    lines_added: 1,
                    lines_removed: 0,
                })
                .collect();
            let filesystem = FilesystemInfo {
                repo: Some(repo),
                git: Some(git),
                ..Default::default()
            };
            SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(filesystem),
                performance: None,
            }
        }

        #[test]
        fn dirty_packages_action_returns_scope_kind_names() {
            let result = fixture(true, &["area-a/alpha/src/main.rs"], &[]);
            let action = RepoAction::DirtyPackages {
                filter: vec![],
                package: None,
                package_area: None,
            };
            let value = build(&result, Some(&action), None);
            assert_eq!(value["scope"], "dirty");
            assert_eq!(value["kind"], "packages");
            let names = value["names"].as_array().expect("names must be array");
            assert_eq!(names, &vec![Value::String("alpha".into())]);
        }

        #[test]
        fn dirty_package_areas_action_returns_scope_kind_names() {
            let result = fixture(true, &["area-a/alpha/src/main.rs"], &[]);
            let action = RepoAction::DirtyPackageAreas {
                filter: vec![],
                package: None,
                package_area: None,
            };
            let value = build(&result, Some(&action), None);
            assert_eq!(value["scope"], "dirty");
            assert_eq!(value["kind"], "package_areas");
            let names = value["names"].as_array().expect("names must be array");
            assert_eq!(names, &vec![Value::String("area-a".into())]);
        }

        #[test]
        fn staged_packages_action_returns_scope_kind_names() {
            let result = fixture(true, &[], &["area-b/beta/src/lib.rs"]);
            let action = RepoAction::StagedPackages {
                filter: vec![],
                package: None,
                package_area: None,
            };
            let value = build(&result, Some(&action), None);
            assert_eq!(value["scope"], "staged");
            assert_eq!(value["kind"], "packages");
            let names = value["names"].as_array().expect("names must be array");
            assert_eq!(names, &vec![Value::String("beta".into())]);
        }

        #[test]
        fn staged_package_areas_action_returns_scope_kind_names() {
            let result = fixture(true, &[], &["area-b/beta/src/lib.rs"]);
            let action = RepoAction::StagedPackageAreas {
                filter: vec![],
                package: None,
                package_area: None,
            };
            let value = build(&result, Some(&action), None);
            assert_eq!(value["scope"], "staged");
            assert_eq!(value["kind"], "package_areas");
            let names = value["names"].as_array().expect("names must be array");
            assert_eq!(names, &vec![Value::String("area-b".into())]);
        }

        #[test]
        fn unstaged_packages_action_returns_scope_kind_names() {
            // Unstaged uses Modified/Both file_changes; rebuild by mutating fixture.
            let mut result = fixture(true, &[], &[]);
            if let Some(fs) = result.filesystem.as_mut()
                && let Some(git) = fs.git.as_mut()
            {
                git.file_changes = vec![FileChange {
                    path: PathBuf::from("area-a/alpha/src/main.rs"),
                    status: FileStatus::Modified,
                    action: FileAction::Modified,
                    lines_added: 1,
                    lines_removed: 0,
                }];
            }
            let action = RepoAction::UnstagedPackages {
                filter: vec![],
                package: None,
                package_area: None,
            };
            let value = build(&result, Some(&action), None);
            assert_eq!(value["scope"], "unstaged");
            assert_eq!(value["kind"], "packages");
            let names = value["names"].as_array().expect("names must be array");
            assert_eq!(names, &vec![Value::String("alpha".into())]);
        }

        #[test]
        fn unstaged_package_areas_action_returns_scope_kind_names() {
            let mut result = fixture(true, &[], &[]);
            if let Some(fs) = result.filesystem.as_mut()
                && let Some(git) = fs.git.as_mut()
            {
                git.file_changes = vec![FileChange {
                    path: PathBuf::from("area-a/alpha/src/main.rs"),
                    status: FileStatus::Modified,
                    action: FileAction::Modified,
                    lines_added: 1,
                    lines_removed: 0,
                }];
            }
            let action = RepoAction::UnstagedPackageAreas {
                filter: vec![],
                package: None,
                package_area: None,
            };
            let value = build(&result, Some(&action), None);
            assert_eq!(value["scope"], "unstaged");
            assert_eq!(value["kind"], "package_areas");
            let names = value["names"].as_array().expect("names must be array");
            assert_eq!(names, &vec![Value::String("area-a".into())]);
        }

        #[test]
        fn non_monorepo_returns_empty_names_not_prose_error() {
            let result = fixture(false, &["area-a/alpha/src/main.rs"], &[]);
            let action = RepoAction::DirtyPackages {
                filter: vec![],
                package: None,
                package_area: None,
            };
            let value = build(&result, Some(&action), None);
            assert_eq!(value["scope"], "dirty");
            assert_eq!(value["kind"], "packages");
            // Critical: JSON consumers must see an empty array, never a prose
            // "only intended to be used in a monorepo" string.
            assert_eq!(value["names"], json!([]));
        }
    }

    mod locators_and_booleans {
        //! Phase 4 — locator and boolean families.
        //!
        //! These arms must:
        //!   1. Always emit a stable JSON object so consumers don't have to
        //!      handle missing-key errors.
        //!   2. Surface an explicit exit code for boolean families and for
        //!      empty locator results — `commands.rs` honours
        //!      `BuildOutcome::exit_code` after stdout is flushed.

        use super::*;
        use sniff::filesystem::FilesystemInfo;
        use sniff::filesystem::git::types::DirtyFile;
        use sniff::filesystem::repo::Package;
        use sniff::filesystem::repo::types::RepoInfo;

        fn make_package(name: &str, area: &str) -> Package {
            Package {
                path: PathBuf::from(format!("/tmp/repo/{area}/{name}")),
                relative: format!("{area}/{name}"),
                package_area: area.to_string(),
                name: name.to_string(),
                ecosystem: sniff::filesystem::repo::PackageEcosystem::Unknown,
                standard: sniff::filesystem::repo::MonorepoStandard::Unknown,
                provenance: sniff::filesystem::repo::PackageProvenance::ManifestScan,
                nested_packages: vec![],
                primary_language: None,
                secondary_languages: vec![],
                languages: vec![],
                frameworks: vec![],
                file_associations: vec![],
                configuration: vec![],
                documentation: vec![],
                editor_config: None,
                command_runner: vec![],
                package_managers: vec![],
                test_runners: vec![],
                version: None,
                features: vec![],
                depends_on: vec![],
                used_by: vec![],
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                is_updatable: None,
                has_major_update: None,
                is_excluded: false,
            }
        }

        fn fixture_with_packages(dirty_paths: &[&str]) -> SniffResult {
            let repo = RepoInfo {
                is_monorepo: true,
                root: PathBuf::from("/tmp/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                monorepo_standards: Vec::new(),
                monorepo_layers: Vec::new(),
                packages: Some(vec![
                    make_package("alpha", "area-a"),
                    make_package("beta", "area-b"),
                ]),
            };
            let mut git = fixture_git_info();
            git.status.as_mut().unwrap().dirty = dirty_paths
                .iter()
                .map(|p| DirtyFile {
                    filepath: PathBuf::from(p),
                    absolute_filepath: PathBuf::from(format!("/tmp/repo/{p}")),
                    diff: String::new(),
                    last_local_commit: String::new(),
                    origin_commit: None,
                })
                .collect();
            let filesystem = FilesystemInfo {
                repo: Some(repo),
                git: Some(git),
                ..Default::default()
            };
            SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(filesystem),
                performance: None,
            }
        }

        #[test]
        fn package_root_present_emits_root_and_no_exit_code() {
            let result = fixture_with_packages(&[]);
            let outcome = build_with_outcome(
                &result,
                Some(&RepoAction::PackageRoot),
                Some(&PathBuf::from("/tmp/repo/area-a/alpha")),
            );
            assert_eq!(outcome.value["root"], "/tmp/repo/area-a/alpha");
            assert!(
                outcome.exit_code.is_none(),
                "non-empty locator must not set exit code: {:?}",
                outcome.exit_code
            );
        }

        #[test]
        fn package_root_absent_emits_empty_root_and_exit_1() {
            let result = fixture_with_packages(&[]);
            let outcome = build_with_outcome(
                &result,
                Some(&RepoAction::PackageRoot),
                Some(&PathBuf::from("/tmp/somewhere-else")),
            );
            assert_eq!(outcome.value, json!({ "root": "" }));
            assert_eq!(outcome.exit_code, Some(1));
        }

        #[test]
        fn package_area_root_present_emits_root_and_no_exit_code() {
            let result = fixture_with_packages(&[]);
            let outcome = build_with_outcome(
                &result,
                Some(&RepoAction::PackageAreaRoot),
                Some(&PathBuf::from("/tmp/repo/area-a")),
            );
            assert_eq!(outcome.value["root"], "/tmp/repo/area-a");
            assert!(outcome.exit_code.is_none());
        }

        #[test]
        fn package_area_root_absent_emits_empty_root_and_exit_1() {
            let result = fixture_with_packages(&[]);
            let outcome = build_with_outcome(
                &result,
                Some(&RepoAction::PackageAreaRoot),
                Some(&PathBuf::from("/tmp/elsewhere")),
            );
            assert_eq!(outcome.value, json!({ "root": "" }));
            assert_eq!(outcome.exit_code, Some(1));
        }

        #[test]
        fn is_dirty_true_emits_dirty_true_and_exit_0() {
            let result = fixture_with_packages(&["area-a/alpha/src/main.rs"]);
            let outcome = build_with_outcome(
                &result,
                Some(&RepoAction::IsCurrentPackageAreaDirty),
                Some(&PathBuf::from("/tmp/repo/area-a/alpha")),
            );
            assert_eq!(outcome.value, json!({ "dirty": true }));
            assert_eq!(outcome.exit_code, Some(0));
        }

        #[test]
        fn is_dirty_false_emits_dirty_false_and_exit_1() {
            let result = fixture_with_packages(&[]);
            let outcome = build_with_outcome(
                &result,
                Some(&RepoAction::IsCurrentPackageAreaDirty),
                Some(&PathBuf::from("/tmp/repo/area-a/alpha")),
            );
            assert_eq!(outcome.value, json!({ "dirty": false }));
            assert_eq!(outcome.exit_code, Some(1));
        }

        #[test]
        fn has_source_code_changes_true_emits_true_and_exit_0() {
            let result = fixture_with_packages(&["area-a/alpha/src/main.rs"]);
            let outcome = build_with_outcome(
                &result,
                Some(&RepoAction::PackageAreaHasSourceCodeChanges),
                Some(&PathBuf::from("/tmp/repo/area-a/alpha")),
            );
            assert_eq!(outcome.value, json!({ "has_source_code_changes": true }));
            assert_eq!(outcome.exit_code, Some(0));
        }

        #[test]
        fn has_source_code_changes_false_emits_false_and_exit_1() {
            let result = fixture_with_packages(&["area-a/alpha/README.md"]);
            let outcome = build_with_outcome(
                &result,
                Some(&RepoAction::PackageAreaHasSourceCodeChanges),
                Some(&PathBuf::from("/tmp/repo/area-a/alpha")),
            );
            assert_eq!(outcome.value, json!({ "has_source_code_changes": false }));
            assert_eq!(outcome.exit_code, Some(1));
        }

        #[test]
        fn has_merge_conflict_outcome_true() {
            let outcome = has_merge_conflict_outcome(true);
            assert_eq!(outcome.value, json!({ "has_merge_conflict": true }));
            assert_eq!(outcome.exit_code, Some(0));
        }

        #[test]
        fn has_merge_conflict_outcome_false() {
            let outcome = has_merge_conflict_outcome(false);
            assert_eq!(outcome.value, json!({ "has_merge_conflict": false }));
            assert_eq!(outcome.exit_code, Some(1));
        }

        #[test]
        fn name_outcome_wraps_string() {
            let outcome = name_outcome("alpha".to_string());
            assert_eq!(outcome.value, json!({ "name": "alpha" }));
            assert!(outcome.exit_code.is_none());
        }

        #[test]
        fn name_outcome_empty_sets_exit_code_one() {
            let outcome = name_outcome(String::new());
            assert_eq!(outcome.value, json!({ "name": "" }));
            assert_eq!(
                outcome.exit_code,
                Some(1),
                "empty name must surface failure exit code"
            );
        }

        #[test]
        fn is_monorepo_outcome_monorepo_shape() {
            let repo = RepoInfo {
                is_monorepo: true,
                root: PathBuf::from("/repo"),
                monorepo_layers: vec![MonorepoLayer {
                    authority: MonorepoStandard::CargoWorkspace,
                    orchestrators: vec![MonorepoStandard::Nx],
                    provenance: sniff::filesystem::repo::PackageProvenance::Explicit,
                    root: PathBuf::from("/repo"),
                    lockfile_match: None,
                    root_is_package: true,
                    packages: vec!["pkg-a".to_string()],
                }],
                ..RepoInfo::default()
            };
            let outcome = is_monorepo_outcome(Some(&repo), false);
            assert_eq!(
                outcome.value,
                json!({
                    "is_monorepo": true,
                    "authority": "cargo-workspace",
                    "orchestrators": ["nx"],
                })
            );
            assert!(outcome.exit_code.is_none());
        }

        #[test]
        fn is_monorepo_outcome_false_sets_exit_code() {
            let outcome = is_monorepo_outcome(None, false);
            assert_eq!(outcome.value, json!({ "is_monorepo": false }));
            assert_eq!(outcome.exit_code, Some(1));
        }

        #[test]
        fn is_monorepo_outcome_false_with_no_error_sets_exit_code_zero() {
            let outcome = is_monorepo_outcome(None, true);
            assert_eq!(outcome.value, json!({ "is_monorepo": false }));
            assert_eq!(outcome.exit_code, Some(0));
        }

        #[test]
        fn is_monorepo_outcome_monorepo_omits_empty_orchestrators() {
            let repo = RepoInfo {
                is_monorepo: true,
                root: PathBuf::from("/repo"),
                monorepo_layers: vec![MonorepoLayer {
                    authority: MonorepoStandard::CargoWorkspace,
                    orchestrators: vec![],
                    provenance: sniff::filesystem::repo::PackageProvenance::Explicit,
                    root: PathBuf::from("/repo"),
                    lockfile_match: None,
                    root_is_package: true,
                    packages: vec!["pkg-a".to_string()],
                }],
                ..RepoInfo::default()
            };
            let outcome = is_monorepo_outcome(Some(&repo), false);
            assert_eq!(
                outcome.value,
                json!({
                    "is_monorepo": true,
                    "authority": "cargo-workspace",
                })
            );
            assert!(
                outcome.value.get("orchestrators").is_none(),
                "empty orchestrators must be omitted: {}",
                outcome.value
            );
            assert!(outcome.exit_code.is_none());
        }

        #[test]
        fn is_monorepo_outcome_monorepo_no_error_still_exits_zero() {
            let repo = RepoInfo {
                is_monorepo: true,
                root: PathBuf::from("/repo"),
                monorepo_layers: vec![MonorepoLayer {
                    authority: MonorepoStandard::CargoWorkspace,
                    orchestrators: vec![MonorepoStandard::Nx],
                    provenance: sniff::filesystem::repo::PackageProvenance::Explicit,
                    root: PathBuf::from("/repo"),
                    lockfile_match: None,
                    root_is_package: true,
                    packages: vec!["pkg-a".to_string()],
                }],
                ..RepoInfo::default()
            };
            let outcome = is_monorepo_outcome(Some(&repo), true);
            assert_eq!(
                outcome.value,
                json!({
                    "is_monorepo": true,
                    "authority": "cargo-workspace",
                    "orchestrators": ["nx"],
                })
            );
            assert!(outcome.exit_code.is_none());
        }

        #[test]
        fn package_count_outcome_wraps_number() {
            let outcome = package_count_outcome(42);
            assert_eq!(outcome.value, json!({ "package-count": 42 }));
            assert!(outcome.exit_code.is_none());
        }

        #[test]
        fn worktrees_value_shapes_entries() {
            use sniff::filesystem::git::WorktreeEntry;
            use std::path::PathBuf;

            let entries = vec![
                WorktreeEntry {
                    name: "main".to_string(),
                    branch: Some("main".to_string()),
                    path: PathBuf::from("/tmp/repo"),
                    is_current: true,
                    is_detached: false,
                },
                WorktreeEntry {
                    name: "feature".to_string(),
                    branch: Some("feature-branch".to_string()),
                    path: PathBuf::from("/tmp/repo/feature"),
                    is_current: false,
                    is_detached: false,
                },
            ];
            let value = worktrees_value(&entries);
            let arr = value["worktrees"].as_array().expect("worktrees array");
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0]["name"], "main");
            assert_eq!(arr[0]["branch"], "main");
            assert_eq!(arr[0]["current"], true);
            assert_eq!(arr[0]["detached"], false);
            assert_eq!(arr[1]["name"], "feature");
            assert_eq!(arr[1]["branch"], "feature-branch");
            assert_eq!(arr[1]["current"], false);
            assert_eq!(arr[1]["detached"], false);
        }

        #[test]
        fn worktrees_value_detached_head_omits_branch() {
            use sniff::filesystem::git::WorktreeEntry;
            use std::path::PathBuf;

            let entries = vec![WorktreeEntry {
                name: "detached".to_string(),
                branch: None,
                path: PathBuf::from("/tmp/repo/detached"),
                is_current: false,
                is_detached: true,
            }];
            let value = worktrees_value(&entries);
            let arr = value["worktrees"].as_array().expect("worktrees array");
            assert_eq!(arr[0]["branch"], Value::Null);
            assert_eq!(arr[0]["detached"], true);
        }
    }

    mod deps {
        //! Phase 5 — `package-dependencies --json` builder.
        //!
        //! These tests pin the hand-built per-package shape so future
        //! additions to the `Package` struct (languages, configuration,
        //! documentation, etc.) can't silently leak into the contract.

        use super::*;
        use sniff::filesystem::repo::types::RepoInfo;
        use sniff::filesystem::repo::{DependencyEntry, DependencyKind, Package};
        use std::path::PathBuf;

        fn dep(name: &str, version: &str) -> DependencyEntry {
            DependencyEntry {
                name: name.to_string(),
                kind: DependencyKind::Normal,
                targeted_version: version.to_string(),
                actual_version: None,
                package_manager: None,
                latest_version: None,
                target: None,
                optional: false,
                features: vec![],
                is_updatable: false,
                has_major_update: false,
            }
        }

        fn make_pkg_full(name: &str, area: &str) -> Package {
            Package {
                path: PathBuf::from(format!("/tmp/repo/{area}/{name}")),
                relative: format!("{area}/{name}"),
                package_area: area.to_string(),
                name: name.to_string(),
                ecosystem: sniff::filesystem::repo::PackageEcosystem::Unknown,
                standard: sniff::filesystem::repo::MonorepoStandard::Unknown,
                provenance: sniff::filesystem::repo::PackageProvenance::ManifestScan,
                nested_packages: vec![],
                primary_language: None,
                secondary_languages: vec![],
                languages: vec![],
                frameworks: vec![],
                file_associations: vec![],
                configuration: vec![PathBuf::from("Cargo.toml")],
                documentation: vec![PathBuf::from("README.md")],
                editor_config: None,
                command_runner: vec![],
                package_managers: vec!["cargo".to_string()],
                test_runners: vec![],
                version: Some("0.1.0".to_string()),
                features: vec![],
                depends_on: vec![],
                used_by: vec![],
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                is_updatable: None,
                has_major_update: None,
                is_excluded: false,
            }
        }

        fn fixture_repo_two_packages() -> RepoInfo {
            let mut alpha = make_pkg_full("alpha", "area-a");
            alpha.depends_on = vec![];
            alpha.used_by = vec!["beta".to_string()];
            alpha.dependencies = Some(vec![dep("serde", "1.0")]);
            alpha.dev_dependencies = Some(vec![dep("tempfile", "3.0")]);
            // Empty peer/optional — must be omitted from output.
            alpha.peer_dependencies = Some(vec![]);
            alpha.optional_dependencies = Some(vec![]);

            let mut beta = make_pkg_full("beta", "area-b");
            beta.depends_on = vec!["alpha".to_string()];
            beta.used_by = vec![];
            beta.dependencies = Some(vec![dep("clap", "4.4"), dep("alpha", "0.1")]);
            beta.dev_dependencies = Some(vec![]);
            // Non-empty peer/optional — must be included.
            beta.peer_dependencies = Some(vec![dep("vue", "3")]);
            beta.optional_dependencies = Some(vec![dep("rayon", "1")]);

            RepoInfo {
                is_monorepo: true,
                root: PathBuf::from("/tmp/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                monorepo_standards: Vec::new(),
                monorepo_layers: Vec::new(),
                packages: Some(vec![alpha, beta]),
            }
        }

        #[test]
        fn deps_value_wraps_packages_array() {
            let repo = fixture_repo_two_packages();
            let entries = build_deps_entries(&repo, &[], None, None);
            assert_eq!(entries.len(), 2, "expected two package entries");
        }

        #[test]
        fn deps_entries_honor_package_filter() {
            let repo = fixture_repo_two_packages();
            let entries = build_deps_entries(&repo, &[], Some("alpha"), None);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0]["name"], "alpha");
        }

        #[test]
        fn deps_entries_honor_package_area_prefix() {
            let repo = fixture_repo_two_packages();
            let entries = build_deps_entries(&repo, &[], None, Some("area-b"));
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0]["name"], "beta");
        }

        #[test]
        fn deps_entry_has_required_fields() {
            let repo = fixture_repo_two_packages();
            let entries = build_deps_entries(&repo, &[], None, None);
            for entry in &entries {
                let obj = entry.as_object().expect("entry must be object");
                for required in [
                    "name",
                    "depends_on",
                    "used_by",
                    "dependencies",
                    "dev_dependencies",
                ] {
                    assert!(
                        obj.contains_key(required),
                        "deps entry must have `{required}`: {entry}"
                    );
                }
            }
        }

        #[test]
        fn deps_entry_omits_empty_peer_and_optional() {
            // alpha has empty peer/optional — neither key should appear.
            let repo = fixture_repo_two_packages();
            let entries = build_deps_entries(&repo, &[], None, None);
            let alpha = entries
                .iter()
                .find(|v| v["name"] == "alpha")
                .expect("alpha entry");
            let obj = alpha.as_object().expect("object");
            assert!(
                !obj.contains_key("peer_dependencies"),
                "empty peer_dependencies must be omitted: {alpha}"
            );
            assert!(
                !obj.contains_key("optional_dependencies"),
                "empty optional_dependencies must be omitted: {alpha}"
            );
        }

        #[test]
        fn deps_entry_includes_non_empty_peer_and_optional() {
            // beta has non-empty peer/optional — both keys must appear.
            let repo = fixture_repo_two_packages();
            let entries = build_deps_entries(&repo, &[], None, None);
            let beta = entries
                .iter()
                .find(|v| v["name"] == "beta")
                .expect("beta entry");
            let obj = beta.as_object().expect("object");
            assert!(
                obj.contains_key("peer_dependencies"),
                "non-empty peer_dependencies must be present: {beta}"
            );
            assert!(
                obj.contains_key("optional_dependencies"),
                "non-empty optional_dependencies must be present: {beta}"
            );
        }

        #[test]
        fn deps_entry_does_not_leak_unrelated_package_fields() {
            // The hand-built allowlist must NOT include path/languages/
            // documentation/configuration/etc. — these are present on the
            // `Package` struct but irrelevant to the `package-dependencies` contract.
            let repo = fixture_repo_two_packages();
            let entries = build_deps_entries(&repo, &[], None, None);
            for entry in &entries {
                let obj = entry.as_object().expect("entry must be object");
                for forbidden in [
                    "path",
                    "relative",
                    "package_area",
                    "ecosystem",
                    "languages",
                    "documentation",
                    "configuration",
                    "package_managers",
                    "version",
                    "is_excluded",
                ] {
                    assert!(
                        !obj.contains_key(forbidden),
                        "deps entry leaked `{forbidden}`: {entry}"
                    );
                }
            }
        }

        #[test]
        fn deps_value_wraps_in_packages_object() {
            let repo = fixture_repo_two_packages();
            let result = SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(FilesystemInfo {
                    repo: Some(repo),
                    ..Default::default()
                }),
                performance: None,
            };
            let value = build_deps_value(&result, &[], None, None);
            assert!(value.is_object(), "top-level must be object");
            let packages = value["packages"]
                .as_array()
                .expect("`packages` must be array");
            assert_eq!(packages.len(), 2);
        }

        #[test]
        fn deps_value_without_filesystem_returns_empty_packages() {
            let result = SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: None,
                performance: None,
            };
            let value = build_deps_value(&result, &[], None, None);
            assert_eq!(value, json!({ "packages": [] }));
        }

        #[test]
        fn deps_value_without_repo_returns_empty_packages() {
            let result = SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(FilesystemInfo::default()),
                performance: None,
            };
            let value = build_deps_value(&result, &[], None, None);
            assert_eq!(value, json!({ "packages": [] }));
        }

        #[test]
        fn deps_value_honors_filter() {
            let repo = fixture_repo_two_packages();
            let entries = build_deps_entries(&repo, &["alpha".to_string()], None, None);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0]["name"], "alpha");
        }

        #[test]
        fn deps_action_dispatches_to_deps_builder() {
            let repo = fixture_repo_two_packages();
            let result = SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(FilesystemInfo {
                    repo: Some(repo),
                    ..Default::default()
                }),
                performance: None,
            };
            let action = RepoAction::PackageDependencies {
                ui: false,
                svg: false,
                filter: vec![],
                package: None,
                package_area: None,
                width: None,
                orientation: None,
            };
            let value = build(&result, Some(&action), None);
            assert!(value.is_object(), "deps value must be object");
            assert!(value["packages"].is_array(), "must have packages array");
            assert!(
                value.get("is_monorepo").is_none(),
                "deps must NOT return full RepoInfo: {value}"
            );
        }

        #[test]
        fn deps_action_ignores_ui_flag() {
            let repo = fixture_repo_two_packages();
            let result = SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(FilesystemInfo {
                    repo: Some(repo),
                    ..Default::default()
                }),
                performance: None,
            };
            let text_action = RepoAction::PackageDependencies {
                ui: false,
                svg: false,
                filter: vec![],
                package: None,
                package_area: None,
                width: None,
                orientation: None,
            };
            let ui_action = RepoAction::PackageDependencies {
                ui: true,
                svg: false,
                filter: vec![],
                package: None,
                package_area: None,
                width: None,
                orientation: None,
            };
            let text_value = build(&result, Some(&text_action), None);
            let ui_value = build(&result, Some(&ui_action), None);
            assert_eq!(text_value, ui_value, "ui flag must not change JSON output");
        }
    }

    mod structure {
        //! Phase 3 — `repo structure --json --filter` builder.
        //!
        //! These tests pin the JSON shape so consumers can rely on
        //! `--filter` scoping the `packages` array (matching text mode)
        //! while every other `RepoInfo` field is preserved.

        use super::*;
        use sniff::filesystem::FilesystemInfo;
        use sniff::filesystem::repo::Package;
        use sniff::filesystem::repo::types::RepoInfo;

        fn make_package(name: &str, area: &str) -> Package {
            Package {
                path: PathBuf::from(format!("/tmp/repo/{area}/{name}")),
                relative: format!("{area}/{name}"),
                package_area: area.to_string(),
                name: name.to_string(),
                ecosystem: sniff::filesystem::repo::PackageEcosystem::Unknown,
                standard: sniff::filesystem::repo::MonorepoStandard::Unknown,
                provenance: sniff::filesystem::repo::PackageProvenance::ManifestScan,
                nested_packages: vec![],
                primary_language: None,
                secondary_languages: vec![],
                languages: vec![],
                frameworks: vec![],
                file_associations: vec![],
                configuration: vec![],
                documentation: vec![],
                editor_config: None,
                command_runner: vec![],
                package_managers: vec![],
                test_runners: vec![],
                version: None,
                features: vec![],
                depends_on: vec![],
                used_by: vec![],
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                is_updatable: None,
                has_major_update: None,
                is_excluded: false,
            }
        }

        fn fixture_with_two_packages() -> SniffResult {
            let repo = RepoInfo {
                is_monorepo: true,
                root: PathBuf::from("/tmp/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                monorepo_standards: Vec::new(),
                monorepo_layers: Vec::new(),
                packages: Some(vec![
                    make_package("alpha", "area-a"),
                    make_package("beta", "area-b"),
                ]),
            };
            SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(FilesystemInfo {
                    repo: Some(repo),
                    ..Default::default()
                }),
                performance: None,
            }
        }

        #[test]
        fn structure_no_filter_matches_fallback() {
            let result = fixture_with_two_packages();
            let unfiltered = structure_value(&result, &[], None, None);
            let fallback = fallback_repo_value(&result);
            assert_eq!(
                unfiltered, fallback,
                "no filter should match the unfiltered fallback shape"
            );
        }

        #[test]
        fn structure_with_filter_scopes_packages() {
            let result = fixture_with_two_packages();
            let value = structure_value(&result, &["alpha".to_string()], None, None);
            let pkgs = value["packages"]
                .as_array()
                .expect("packages must be array");
            assert_eq!(pkgs.len(), 1, "filter should narrow to 1 package: {value}");
            assert_eq!(pkgs[0]["name"], "alpha");
        }

        #[test]
        fn structure_with_filter_preserves_other_repo_fields() {
            // Filtering must NOT drop top-level RepoInfo metadata —
            // is_monorepo / root must still be present. (Empty Vec fields are
            // `skip_serializing_if = "is_empty"`, so we don't assert on them
            // in this fixture.)
            let result = fixture_with_two_packages();
            let value = structure_value(&result, &["alpha".to_string()], None, None);
            assert!(value.is_object(), "must be object: {value}");
            assert_eq!(value["is_monorepo"], Value::Bool(true));
            assert!(
                value.get("root").is_some(),
                "root must be preserved: {value}"
            );
        }

        #[test]
        fn structure_action_with_filter_returns_filtered_value() {
            let result = fixture_with_two_packages();
            let action = RepoAction::Structure {
                filter: vec!["beta".to_string()],
                latest_versions: false,
                package: None,
                package_area: None,
            };
            let value = build(&result, Some(&action), None);
            let pkgs = value["packages"]
                .as_array()
                .expect("packages must be array");
            assert_eq!(pkgs.len(), 1, "structure action must honor filter: {value}");
            assert_eq!(pkgs[0]["name"], "beta");
        }

        #[test]
        fn structure_without_filesystem_returns_empty_object() {
            let result = SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: None,
                performance: None,
            };
            let value = structure_value(&result, &["alpha".to_string()], None, None);
            assert_eq!(value, json!({}));
        }

        #[test]
        fn structure_without_repo_returns_empty_object() {
            let result = SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(FilesystemInfo::default()),
                performance: None,
            };
            let value = structure_value(&result, &["alpha".to_string()], None, None);
            assert_eq!(value, json!({}));
        }
    }

    mod aggregate {
        //! Phase 2 — scope-complete aggregate for bare `sniff repo --json`.

        use super::*;
        use sniff::programs::enums::TestRunner;
        use std::collections::HashSet;
        use std::path::PathBuf;

        fn temp_git_repo() -> (tempfile::TempDir, PathBuf) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().to_path_buf();
            let repo = git2::Repository::init(&path).unwrap();
            let mut config = repo.config().unwrap();
            config.set_str("user.email", "test@test.com").unwrap();
            config.set_str("user.name", "Test").unwrap();

            let sig = repo.signature().unwrap();
            let tree_id = repo.index().unwrap().write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();

            (dir, path)
        }

        /// Observe the aggregate exactly as `commands.rs` does, so these tests
        /// exercise the real library entry point rather than a hand-built
        /// `RepoAggregate` that could drift from what it produces.
        fn aggregate_fixture(path: &Path, result: &SniffResult) -> RepoAggregate {
            sniff::filesystem::repo::observe_repo_aggregate(path, result.filesystem.as_ref())
                .expect("aggregate observation succeeds for the fixture repo")
        }

        /// A `RepoAggregate` carrying only `repo`, for projection tests that
        /// assert repo-derived children and need no git observation at all.
        pub(super) fn synthetic_aggregate(repo: RepoInfo) -> RepoAggregate {
            RepoAggregate {
                identity: sniff::filesystem::repo::RepoIdentity {
                    name: "fixture-repo".to_string(),
                    version: Some("1.0.0".to_string()),
                    language: None,
                    is_monorepo: repo.is_monorepo,
                    package_count: repo.packages.as_ref().map(Vec::len),
                },
                repo: Some(repo),
                version: None,
                branches: Vec::new(),
                worktrees: Vec::new(),
                current_worktree: None,
                has_merge_conflict: false,
                commits: sniff::filesystem::git::recent_commits::CommitDescSet {
                    commits: Vec::new(),
                    period_label: "last 3d".to_string(),
                    repo_root: PathBuf::from("/tmp/repo"),
                    packages: None,
                },
                context: sniff::filesystem::repo::AggregateCwdContext::default(),
            }
        }

        fn result_fixture(repo_root: &Path) -> SniffResult {
            let repo = RepoInfo {
                is_monorepo: false,
                root: repo_root.to_path_buf(),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                monorepo_standards: Vec::new(),
                monorepo_layers: Vec::new(),
                packages: None,
            };
            let request = sniff::request::FilesystemRequest::new()
                .git(sniff::request::GitRequest::full().metadata(
                    sniff::request::GitMetadataRequest::none()
                        .remotes(true)
                        .config(true)
                        .aggregate(true),
                ))
                .without_repo()
                .without_file_inventory()
                .without_formatting()
                .without_docs();
            let mut filesystem = sniff::filesystem::detect_filesystem_with_request(
                repo_root,
                &request,
            )
            .expect("aggregate fixture detection succeeds");
            filesystem.repo = Some(repo);
            SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(filesystem),
                performance: None,
            }
        }

        fn package_with(name: &str, managers: &[&str], runners: &[TestRunner]) -> Package {
            use sniff::filesystem::repo::{TestRunnerSource, TestRunnerUsage};
            Package {
                name: name.to_string(),
                relative: name.to_string(),
                package_area: name.to_string(),
                package_managers: managers.iter().map(|m| (*m).to_string()).collect(),
                test_runners: runners
                    .iter()
                    .map(|runner| TestRunnerUsage {
                        runner: *runner,
                        source: TestRunnerSource::EcosystemDefault,
                    })
                    .collect(),
                ..Package::default()
            }
        }

        fn repo_with_packages(packages: Vec<Package>) -> RepoInfo {
            RepoInfo {
                is_monorepo: true,
                root: PathBuf::from("/tmp/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                monorepo_standards: Vec::new(),
                monorepo_layers: Vec::new(),
                packages: Some(packages),
            }
        }

        #[test]
        fn package_manager_and_test_runner_collapse_to_singular_string() {
            let repo = repo_with_packages(vec![
                package_with("alpha", &["cargo"], &[TestRunner::CargoTest]),
                package_with("beta", &["cargo"], &[TestRunner::CargoTest]),
            ]);
            // Uniform across packages → a single string, never an array.
            assert_eq!(aggregate_package_manager(Some(&repo)), json!("cargo"));
            assert!(aggregate_test_runner(Some(&repo)).is_string());
        }

        #[test]
        fn package_manager_collapses_divergent_set_to_list() {
            let repo = repo_with_packages(vec![
                package_with("alpha", &["cargo"], &[]),
                package_with("beta", &["pnpm"], &[]),
            ]);
            let value = aggregate_package_manager(Some(&repo));
            let list = value.as_array().expect("divergent managers → array");
            assert_eq!(list.len(), 2, "expected both managers: {value}");
        }

        #[test]
        fn package_manager_and_test_runner_null_without_packages() {
            assert_eq!(aggregate_package_manager(None), Value::Null);
            assert_eq!(aggregate_test_runner(None), Value::Null);
            let empty = repo_with_packages(Vec::new());
            assert_eq!(aggregate_package_manager(Some(&empty)), Value::Null);
            assert_eq!(aggregate_test_runner(Some(&empty)), Value::Null);
        }

        #[test]
        fn file_list_value_returns_stable_shape() {
            let value = file_list_value(
                sniff::filesystem::blast_radius::ChangeScope::Staged,
                sniff::filesystem::blast_radius::ChangedPathKind::SourceCode,
                &[PathBuf::from("src/main.rs")],
            );
            assert_eq!(value["scope"], "staged");
            assert_eq!(value["kind"], "source_code");
            assert_eq!(value["paths"], json!(["src/main.rs"]));
        }

        /// The aggregate's one shared history observation, which all three
        /// commit-family projections read, is loaded by the library entry point.
        #[test]
        fn aggregate_carries_the_default_commit_family_set() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let aggregate = aggregate_fixture(&path, &result);
            assert_eq!(aggregate.commits.period_label, "last 3d");
        }

        #[test]
        fn aggregate_includes_consolidated_keys() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let aggregate = aggregate_fixture(&path, &result);
            let value = build_aggregate_value(&result, &aggregate);

            let obj = value.as_object().expect("aggregate must be object");
            let keys: HashSet<_> = obj.keys().map(String::as_str).collect();

            let expected: HashSet<&str> = [
                "name",
                "version",
                "language",
                "is_monorepo",
                "package_count",
                "root",
                "structure",
                "packages",
                "package_areas",
                "package_manager",
                "test_runner",
                "package_dependencies",
                "dependencies",
                "git_status",
                "branches",
                "worktrees",
                "context",
                "dirty",
                "staged",
                "unstaged",
                "untracked",
                "has_merge_conflict",
                "recent_commits",
                "source_code_changes",
                "documentation_changes",
            ]
            .iter()
            .copied()
            .collect();

            let missing: Vec<_> = expected.difference(&keys).collect();
            assert!(
                missing.is_empty(),
                "missing aggregate keys: {missing:?}\n{value}"
            );
        }

        #[test]
        fn aggregate_excludes_network_parameterized_and_old_kebab_keys() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let aggregate = aggregate_fixture(&path, &result);
            let value = build_aggregate_value(&result, &aggregate);

            let obj = value.as_object().expect("aggregate must be object");
            for forbidden in [
                "remote",
                "pr",
                "hash",
                "is-monorepo",
                "package-count",
                "git-status",
                "recent-commits",
                "source-code-changes",
                "documentation-changes",
                "package-dependencies",
                "package-areas",
                "dirty-files",
                "staged-files",
                "unstaged-files",
                "untracked-files",
                "dirty-packages",
                "dirty-package-areas",
            ] {
                assert!(
                    !obj.contains_key(forbidden),
                    "aggregate must not contain `{forbidden}`: {value}"
                );
            }
        }

        #[test]
        fn aggregate_context_groups_cwd_relative_facts() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let aggregate = aggregate_fixture(&path, &result);
            let value = build_aggregate_value(&result, &aggregate);

            // The observed identity is projected verbatim. The fixture repo has
            // no manifest and no remote, so its name resolves to the temp
            // directory's basename rather than a fixed string.
            assert_eq!(value["name"], json!(aggregate.identity.name));
            // No packages in the fixture → the `AggregateScope::Repo`
            // collapse finds zero distinct versions and emits `null`. The
            // `RepoIdentity.version` is not consulted here.
            assert_eq!(value["version"], Value::Null);
            assert_eq!(value["language"], Value::Null);
            assert_eq!(value["is_monorepo"], false);
            assert_eq!(value["package_count"], 0);
            assert!(
                value["root"].is_string() && !value["root"].as_str().unwrap().is_empty(),
                "root must be a non-empty string: {value}"
            );
            let context = value["context"].as_object().expect("context object");
            assert_eq!(context["package"], "");
            assert_eq!(context["package_area"], "");
            assert_eq!(context["package_root"], "");
            assert_eq!(context["package_area_root"], "");
            assert_eq!(context["is_current_package_area_dirty"], false);
            assert_eq!(context["package_area_has_source_code_changes"], false);
            assert!(
                context["worktree"].is_string() || context["worktree"].is_null(),
                "worktree must be unwrapped: {value}"
            );
        }

        #[test]
        fn aggregate_scope_buckets_have_stable_empty_shape() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let aggregate = aggregate_fixture(&path, &result);
            let value = build_aggregate_value(&result, &aggregate);

            for key in ["dirty", "staged", "unstaged", "untracked"] {
                let leaf = &value[key];
                assert!(leaf.is_object(), "{key} must be an object: {value}");
                for field in ["files", "source_code", "documentation", "packages", "package_areas"]
                {
                    assert_eq!(
                        leaf[field],
                        json!([]),
                        "{key}.{field} must be an empty array: {leaf}"
                    );
                }
            }
        }

        #[test]
        fn aggregate_commit_family_leaves_are_objects() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let aggregate = aggregate_fixture(&path, &result);
            let value = build_aggregate_value(&result, &aggregate);

            for key in [
                "recent_commits",
                "source_code_changes",
                "documentation_changes",
            ] {
                assert!(
                    value[key].is_object(),
                    "{key} must be an object in aggregate: {value}"
                );
                assert!(value[key]["period"].is_object(), "{key} period: {value}");
                assert!(value[key].get("repo_root").is_none(), "{key} repo_root: {value}");
                assert!(value[key].get("packages").is_none(), "{key} packages: {value}");
                assert!(value[key].get("filter").is_none(), "{key} filter: {value}");
            }
        }

        #[test]
        fn aggregate_worktrees_and_branches_are_top_level_arrays() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let aggregate = aggregate_fixture(&path, &result);
            let value = build_aggregate_value(&result, &aggregate);

            let worktrees = value["worktrees"].as_array().expect("worktrees array");
            assert!(
                !worktrees.is_empty(),
                "main worktree must be present: {value}"
            );
            assert!(value["branches"].is_array(), "branches array: {value}");
            assert!(
                value["git_status"].get("worktrees").is_none(),
                "worktrees must not be duplicated under git_status: {value}"
            );
            assert!(
                value["git_status"].get("branches").is_none(),
                "branches must not be duplicated under git_status: {value}"
            );
        }

        #[test]
        fn aggregate_does_not_duplicate_full_package_catalogs() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let aggregate = aggregate_fixture(&path, &result);
            let value = build_aggregate_value(&result, &aggregate);

            assert!(value["packages"].is_array(), "top-level package names: {value}");
            assert!(
                value["structure"].get("packages").is_none(),
                "structure must not embed package catalog: {value}"
            );
            assert!(
                value["package_dependencies"]["packages"].is_array(),
                "package_dependencies keeps the narrow dependency projection: {value}"
            );
            assert!(
                value["recent_commits"].get("packages").is_none(),
                "recent_commits must not embed package catalog: {value}"
            );
        }

        /// The aggregate must fail before anything reaches stdout rather than
        /// emit partial JSON. The failure now lives on the library observation:
        /// `build_aggregate_value` is infallible precisely because every fact it
        /// could fail to obtain is obtained before it runs.
        #[test]
        fn aggregate_observation_errors_instead_of_yielding_partial_json() {
            let bad_path = PathBuf::from("/tmp/not-a-git-repo-for-aggregate-test-42");
            let result = fixture_with_git_and_repo();
            let outcome =
                sniff::filesystem::repo::observe_repo_aggregate(&bad_path, result.filesystem.as_ref());
            assert!(
                outcome.is_err(),
                "aggregate must fail rather than emit partial JSON"
            );
        }

        #[test]
        fn aggregate_projects_the_observed_repo_version() {
            let result = fixture_with_git_and_repo();
            let repo = repo_with_packages(vec![Package {
                name: "a".to_string(),
                version: Some("0.1.0".to_string()),
                ..Package::default()
            }]);
            let mut aggregate = synthetic_aggregate(repo);
            aggregate.version = Some("0.1.0".to_string());

            let value = build_aggregate_value(&result, &aggregate);

            assert_eq!(value["version"], "0.1.0");
        }
    }

    mod monorepo_topology {
        //! Phase 8 — monorepo topology JSON surface.
        //!
        //! `RepoInfo::monorepo_standards` and `RepoInfo::monorepo_layers` use
        //! `#[serde(default, skip_serializing_if = "Vec::is_empty")]`, so they
        //! appear only when detection populated them. `structure_value` and
        //! `build_aggregate_value` get them for free by serializing `RepoInfo`.

        use super::*;
        use sniff::filesystem::repo::standard::{
            BinarySource, DetectedStandard, DetectionConfidence, MonorepoLayer, MonorepoStandard,
            PackageProvenance, ResolvedBinary,
        };
        use sniff::filesystem::repo::types::RepoInfo;
        use std::path::PathBuf;

        fn repo_with_layers() -> RepoInfo {
            RepoInfo {
                is_monorepo: true,
                root: PathBuf::from("/tmp/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                monorepo_standards: vec![DetectedStandard {
                    standard: MonorepoStandard::CargoWorkspace,
                    root: PathBuf::from("/tmp/repo"),
                    matched_markers: vec![PathBuf::from("Cargo.toml")],
                    binary: Some(ResolvedBinary {
                        name: "cargo".to_string(),
                        path: Some(PathBuf::from("/usr/bin/cargo")),
                        version: Some("1.80.0".to_string()),
                        satisfies_min_version: None,
                        source: BinarySource::Path,
                    }),
                    confidence: DetectionConfidence::MarkerConfirmed,
                }],
                monorepo_layers: vec![MonorepoLayer {
                    root: PathBuf::from("/tmp/repo"),
                    authority: MonorepoStandard::CargoWorkspace,
                    orchestrators: vec![MonorepoStandard::Nx],
                    provenance: PackageProvenance::Globbed,
                    lockfile_match: None,
                    root_is_package: false,
                    packages: vec!["pkg-a".to_string(), "pkg-b".to_string()],
                }],
                packages: None,
            }
        }

        fn result_with_repo(repo: RepoInfo) -> SniffResult {
            let filesystem = FilesystemInfo {
                repo: Some(repo),
                ..Default::default()
            };
            SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(filesystem),
                performance: None,
            }
        }

        #[test]
        fn structure_value_includes_monorepo_topology_when_present() {
            let result = result_with_repo(repo_with_layers());
            let action = RepoAction::Structure {
                filter: Vec::new(),
                latest_versions: false,
                package: None,
                package_area: None,
            };

            let value = build(&result, Some(&action), None);
            assert!(
                value.get("monorepo_standards").is_some(),
                "structure JSON must include `monorepo_standards`: {value}"
            );
            assert!(
                value.get("monorepo_layers").is_some(),
                "structure JSON must include `monorepo_layers`: {value}"
            );

            let standards = value["monorepo_standards"].as_array().unwrap();
            assert_eq!(standards.len(), 1);
            assert_eq!(standards[0]["standard"], "cargo-workspace");

            let layers = value["monorepo_layers"].as_array().unwrap();
            assert_eq!(layers.len(), 1);
            assert_eq!(layers[0]["authority"], "cargo-workspace");
            assert_eq!(layers[0]["orchestrators"], json!(["nx"]));
            assert_eq!(layers[0]["provenance"], "globbed");
            assert!(
                layers[0]["packages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|p| p.is_string()),
                "layer packages must be path strings: {value}"
            );
        }

        #[test]
        fn structure_value_omits_empty_monorepo_topology() {
            let repo = RepoInfo {
                is_monorepo: false,
                root: PathBuf::from("/tmp/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                monorepo_standards: Vec::new(),
                monorepo_layers: Vec::new(),
                packages: None,
            };
            let result = result_with_repo(repo);
            let action = RepoAction::Structure {
                filter: Vec::new(),
                latest_versions: false,
                package: None,
                package_area: None,
            };

            let value = build(&result, Some(&action), None);
            assert!(
                value.get("monorepo_standards").is_none(),
                "empty `monorepo_standards` must be omitted: {value}"
            );
            assert!(
                value.get("monorepo_layers").is_none(),
                "empty `monorepo_layers` must be omitted: {value}"
            );
            // Legacy keys are still absent on a non-monorepo repo.
            assert!(value.get("monorepo_tool").is_none());
            assert!(value.get("workspace_tools").is_none());
        }

        #[test]
        fn aggregate_structure_child_includes_monorepo_topology() {
            let repo = repo_with_layers();
            let result = result_with_repo(repo.clone());
            let aggregate = super::aggregate::synthetic_aggregate(repo);

            let value = build_aggregate_value(&result, &aggregate);

            assert!(
                value["structure"]["monorepo_standards"].is_array(),
                "aggregate.structure must carry standards: {value}"
            );
            assert!(
                value["structure"]["monorepo_layers"].is_array(),
                "aggregate.structure must carry layers: {value}"
            );
            assert_eq!(
                value["structure"]["monorepo_layers"][0]["authority"],
                "cargo-workspace"
            );
        }

        #[test]
        fn structure_value_package_carries_standard_and_provenance() {
            let mut repo = repo_with_layers();
            repo.packages = Some(vec![Package {
                path: PathBuf::from("/tmp/repo/pkg-a"),
                relative: "pkg-a".to_string(),
                package_area: "root".to_string(),
                name: "pkg-a".to_string(),
                ecosystem: sniff::filesystem::repo::PackageEcosystem::default(),
                standard: MonorepoStandard::CargoWorkspace,
                provenance: PackageProvenance::Globbed,
                ..Package::default()
            }]);
            let result = result_with_repo(repo);
            let action = RepoAction::Structure {
                filter: Vec::new(),
                latest_versions: false,
                package: None,
                package_area: None,
            };

            let value = build(&result, Some(&action), None);
            let packages = value["packages"].as_array().expect("packages array");
            assert_eq!(packages.len(), 1);
            assert_eq!(packages[0]["standard"], "cargo-workspace");
            assert_eq!(packages[0]["provenance"], "globbed");
            assert!(
                packages[0].get("discovery_sources").is_none(),
                "legacy discovery_sources must be absent: {value}"
            );

            let layers = value["monorepo_layers"].as_array().unwrap();
            let layer_packages = layers[0]["packages"].as_array().unwrap();
            assert!(layer_packages.iter().all(|p| p.is_string()));
        }
    }

    mod aggregate_projection {
        //! `build_aggregate_value` is a pure projection (umbrella spec R2.7).
        //! These tests pin that property so a future edit cannot quietly
        //! reintroduce an observation that every bare `sniff repo --json` pays.

        use super::*;
        use sniff::filesystem::git::recent_commits::CommitDescSet;
        use sniff::filesystem::repo::{AggregateCwdContext, RepoIdentity};
        use sniff::performance::{PerformanceCollector, with_current_collector};

        fn fixture_aggregate() -> (tempfile::TempDir, RepoAggregate) {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path();
            let packages = ["a", "b"]
                .into_iter()
                .map(|name| {
                    let path = root.join(name);
                    std::fs::create_dir_all(&path).unwrap();
                    std::fs::write(
                        path.join("Cargo.toml"),
                        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
                    )
                    .unwrap();
                    Package {
                        path,
                        relative: name.to_string(),
                        package_area: "root".to_string(),
                        name: name.to_string(),
                        version: Some("0.1.0".to_string()),
                        ..Package::default()
                    }
                })
                .collect();
            let repo = RepoInfo {
                is_monorepo: true,
                root: root.to_path_buf(),
                packages: Some(packages),
                ..RepoInfo::default()
            };
            let aggregate = RepoAggregate {
                identity: RepoIdentity {
                    name: "fixture".to_string(),
                    version: None,
                    language: None,
                    is_monorepo: true,
                    package_count: Some(2),
                },
                repo: Some(repo),
                version: Some("0.1.0".to_string()),
                branches: Vec::new(),
                worktrees: Vec::new(),
                current_worktree: None,
                has_merge_conflict: false,
                commits: CommitDescSet {
                    commits: Vec::new(),
                    period_label: "last 3d".to_string(),
                    repo_root: PathBuf::from("/tmp/repo"),
                    packages: None,
                },
                context: AggregateCwdContext::default(),
            };
            (temp, aggregate)
        }

        /// The headline R2.7 assertion: the builder opens no repository, walks
        /// no status, reads no file, normalizes no path, spawns nothing, and
        /// makes no request. The cwd-relative `context` facts are resolved
        /// during `observe_repo_aggregate` over a single package ownership
        /// index, so the projection records no counters at all.
        #[test]
        fn build_aggregate_value_performs_no_observation() {
            let result = fixture_with_git_and_repo();
            let (_temp, aggregate) = fixture_aggregate();

            let collector = PerformanceCollector::new_shared();
            let value = with_current_collector(Some(collector.clone()), || {
                build_aggregate_value(&result, &aggregate)
            });
            let report = collector.snapshot(std::time::Duration::ZERO);

            assert!(value.is_object(), "aggregate must still be built: {value}");
            assert_eq!(value["version"], "0.1.0");
            assert_eq!(value["packages"].as_array().map(Vec::len), Some(2));
            assert!(
                report.counters.is_empty(),
                "build_aggregate_value must perform no repository open, status walk, \
                 file read, path normalization, subprocess spawn, or network request; \
                 it recorded: {:?}",
                report.counters
            );
        }

        /// The aggregate must project the facts it is handed rather than
        /// re-observing them, so what goes in is what comes out.
        #[test]
        fn aggregate_projects_supplied_facts_verbatim() {
            use sniff::filesystem::git::WorktreeEntry;

            let result = fixture_with_git_and_repo();
            let (_temp, mut aggregate) = fixture_aggregate();
            aggregate.has_merge_conflict = true;
            aggregate.current_worktree = Some("wt-1".to_string());
            aggregate.worktrees = vec![WorktreeEntry {
                name: "wt-1".to_string(),
                branch: Some("feature".to_string()),
                path: PathBuf::from("/tmp/repo/wt-1"),
                is_current: true,
                is_detached: false,
            }];
            aggregate.context = AggregateCwdContext {
                package: "sniff-lib".to_string(),
                package_area: "sniff".to_string(),
                area: "sniff-lib".to_string(),
                package_root: "/tmp/repo/sniff/lib".to_string(),
                package_area_root: "/tmp/repo/sniff".to_string(),
                is_current_package_area_dirty: true,
                package_area_has_source_code_changes: true,
            };

            let value = build_aggregate_value(&result, &aggregate);

            assert_eq!(value["name"], "fixture");
            assert_eq!(value["has_merge_conflict"], true);
            assert_eq!(value["context"]["package"], "sniff-lib");
            assert_eq!(value["context"]["package_area"], "sniff");
            assert_eq!(value["context"]["area"], "sniff-lib");
            assert_eq!(value["context"]["package_root"], "/tmp/repo/sniff/lib");
            assert_eq!(value["context"]["package_area_root"], "/tmp/repo/sniff");
            assert_eq!(value["context"]["worktree"], "wt-1");
            assert_eq!(value["context"]["is_current_package_area_dirty"], true);
            assert_eq!(
                value["context"]["package_area_has_source_code_changes"],
                true
            );
            assert_eq!(value["worktrees"][0]["name"], "wt-1");
            assert_eq!(value["worktrees"][0]["current"], true);
            assert_eq!(value["recent_commits"]["period"]["label"], "last 3d");
        }
    }
}
