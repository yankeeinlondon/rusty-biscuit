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

use std::path::Path;

use serde_json::{Map, Value, json};
use sniff::SniffResult;
use sniff::filesystem::blast_radius::{
    ChangeScope, ChangedPathKind, ChangedPathQuery, collect_changed_paths,
};
use sniff::filesystem::git::list_worktrees;
use sniff::filesystem::repo::Package;
use sniff::filesystem::repo::RepoIdentity;
use sniff::filesystem::repo::types::RepoInfo;

use crate::args::RepoAction;
use crate::output::filesystem;
use crate::output::recent_commits::{
    RecentCommitsMode, commit_family_value, default_commit_family_set,
};

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
        // Phase 5: `deps --json` emits a hand-built per-package object so
        // future fields on `Package` don't leak into the public contract.
        // The `ui` flag is text-only and is intentionally ignored in JSON.
        Some(RepoAction::Deps {
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
/// Returns `{ "is-monorepo": bool }` with exit code `0`.
pub(crate) fn is_monorepo_outcome(value: bool) -> BuildOutcome {
    BuildOutcome::pure(json!({ "is-monorepo": value }))
}

/// Build the JSON outcome for `repo package-count --json`.
///
/// Returns `{ "package-count": N }` with exit code `0`.
pub(crate) fn package_count_outcome(count: usize) -> BuildOutcome {
    BuildOutcome::pure(json!({ "package-count": count }))
}

/// Build the JSON outcome for `repo version --json`.
///
/// Returns `{ "version": "..." | null }`. Exit code is `0` when a version
/// is found and `1` otherwise (or `0` when `no_error` is `true`).
pub(crate) fn version_outcome(version: Option<&str>, no_error: bool) -> BuildOutcome {
    let exit_code = if version.is_some() {
        None
    } else if no_error {
        Some(0)
    } else {
        Some(1)
    };
    BuildOutcome {
        value: json!({ "version": version }),
        exit_code,
    }
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

/// Build the JSON value for `sniff repo deps --json`.
///
/// Returns `{ "packages": [ ... ] }` where each entry is a hand-built
/// object with a narrow allowlist of fields:
/// `name`, `depends_on`, `used_by`, `dependencies`, `dev_dependencies`,
/// and (only when non-empty) `peer_dependencies` / `optional_dependencies`.
///
/// ## Notes
///
/// Hand-building (instead of `serde_json::to_value(&pkg)`) is deliberate:
/// it keeps the public `deps --json` contract narrow so future fields on
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

/// Build the per-package JSON entries for `deps --json`.
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

/// Build a single `deps --json` package entry from a `Package`.
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
/// consumers continue to see workspace tools, monorepo flags, root path,
/// and aggregated dependency rollups.
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

/// Assemble the scope-complete aggregate for bare `sniff repo --json`.
///
/// Returns a flat object keyed by the participating children's subcommand
/// names. Single-key leaves contribute their unwrapped value; multi-field
/// children contribute their whole scope object. Network-primary children
/// (`remote`, `pr`) and parameterized children (`hash`) are excluded.
///
/// ## Errors
///
/// Propagates any local detection failure so the parent command fails
/// rather than emitting a partial aggregate. In `--json` mode the caller
/// must ensure diagnostics go to stderr and stdout contains either the
/// valid aggregate or nothing.
pub(crate) fn build_aggregate_value(
    result: &SniffResult,
    base_dir: Option<&Path>,
    identity: &RepoIdentity,
) -> Result<Value, Box<dyn std::error::Error>> {
    let dir = base_dir.unwrap_or_else(|| Path::new("."));
    let mut map = Map::new();

    // Identity leaves — single-key, unwrapped.
    map.insert("name".into(), Value::String(identity.name.clone()));
    map.insert(
        "version".into(),
        identity
            .version
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    map.insert(
        "language".into(),
        filesystem::primary_language_name(result)
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    map.insert("is-monorepo".into(), Value::Bool(identity.is_monorepo));
    map.insert(
        "package-count".into(),
        Value::Number(identity.package_count.unwrap_or(0).into()),
    );

    // Structure / dependency leaves — whole scope objects.
    map.insert("structure".into(), structure_value(result, &[], None, None));
    map.insert("deps".into(), build_deps_value(result, &[], None, None));

    // Package and package-area name arrays.
    let (packages, package_areas) = result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.repo.as_ref())
        .map(|repo| {
            let names = filesystem::collect_repo_package_names(repo, &[], None, None);
            let areas = filesystem::collect_repo_package_area_names(repo, &[], None, None);
            (
                names
                    .into_iter()
                    .map(|s| Value::String(s.to_string()))
                    .collect::<Vec<_>>(),
                areas
                    .into_iter()
                    .map(|s| Value::String(s.to_string()))
                    .collect::<Vec<_>>(),
            )
        })
        .unwrap_or_default();
    map.insert("packages".into(), Value::Array(packages));
    map.insert("package-areas".into(), Value::Array(package_areas));

    // Worktrees — enumerate separately; the aggregate shape matches the
    // focused `repo worktrees --json` object.
    let worktrees = match list_worktrees(dir)? {
        Some(entries) => worktrees_value(&entries),
        None => json!({ "worktrees": [] }),
    };
    map.insert("worktrees".into(), worktrees);

    // Git status — local form only.
    map.insert("git-status".into(), git_status_value(result));

    // File-list leaves — stable empty shape when nothing changed.
    let file_list_queries = [
        (
            "staged-files",
            ChangeScope::Staged,
            ChangedPathKind::AllFiles,
        ),
        (
            "unstaged-files",
            ChangeScope::Unstaged,
            ChangedPathKind::AllFiles,
        ),
        (
            "untracked-files",
            ChangeScope::Untracked,
            ChangedPathKind::AllFiles,
        ),
        (
            "dirty-source-code",
            ChangeScope::Dirty,
            ChangedPathKind::SourceCode,
        ),
        (
            "staged-source-code",
            ChangeScope::Staged,
            ChangedPathKind::SourceCode,
        ),
        (
            "unstaged-source-code",
            ChangeScope::Unstaged,
            ChangedPathKind::SourceCode,
        ),
        ("dirty-files", ChangeScope::Dirty, ChangedPathKind::AllFiles),
    ];
    for (key, scope, kind) in file_list_queries {
        let query = ChangedPathQuery {
            scope,
            kind,
            package: None,
            package_area: None,
            filters: Vec::new(),
        };
        let changed = collect_changed_paths(dir, &query)?;
        map.insert(key.into(), file_list_value(scope, kind, &changed.paths));
    }

    // Package-context locator leaves — unwrapped string values.
    map.insert(
        "package".into(),
        Value::String(filesystem::render_repo_package(result, base_dir, 0)),
    );
    map.insert(
        "package-area".into(),
        Value::String(filesystem::render_repo_package_area(result, base_dir)),
    );
    map.insert(
        "area".into(),
        Value::String(filesystem::render_repo_area(result, base_dir)),
    );
    map.insert(
        "package-root".into(),
        Value::String(filesystem::render_repo_package_root(result, base_dir)),
    );
    map.insert(
        "package-area-root".into(),
        Value::String(filesystem::render_repo_package_area_root(result, base_dir)),
    );
    map.insert(
        "root".into(),
        Value::String(filesystem::render_repo_root(result)),
    );

    // Package/area change-family leaves.
    map.insert(
        "dirty-packages".into(),
        package_family_value(
            "dirty",
            "packages",
            filesystem::select_dirty_package_names(result, &[], None, None),
        ),
    );
    map.insert(
        "dirty-package-areas".into(),
        package_family_value(
            "dirty",
            "package_areas",
            filesystem::select_dirty_package_area_names(result, &[], None, None),
        ),
    );
    map.insert(
        "staged-packages".into(),
        package_family_value(
            "staged",
            "packages",
            filesystem::select_staged_package_names(result, &[], None, None),
        ),
    );
    map.insert(
        "staged-package-areas".into(),
        package_family_value(
            "staged",
            "package_areas",
            filesystem::select_staged_package_area_names(result, &[], None, None),
        ),
    );
    map.insert(
        "unstaged-packages".into(),
        package_family_value(
            "unstaged",
            "packages",
            filesystem::select_unstaged_package_names(result, &[], None, None),
        ),
    );
    map.insert(
        "unstaged-package-areas".into(),
        package_family_value(
            "unstaged",
            "package_areas",
            filesystem::select_unstaged_package_area_names(result, &[], None, None),
        ),
    );

    // Boolean leaves — unwrapped bool values.
    let dirty = filesystem::current_package_area_is_dirty(result, base_dir).unwrap_or(false);
    map.insert("is-current-package-area-dirty".into(), Value::Bool(dirty));
    let has_source_changes = filesystem::package_area_source_code_change_count(result, base_dir)
        .map(|(has, _, _)| has)
        .unwrap_or(false);
    map.insert(
        "package-area-has-source-code-changes".into(),
        Value::Bool(has_source_changes),
    );
    let has_conflict = if let Some(repo_root) = sniff::filesystem::repo_root(dir)? {
        let conflicted = sniff::filesystem::merge_conflicts_at(&repo_root)?;
        !conflicted.is_empty()
    } else {
        false
    };
    map.insert("has-merge-conflict".into(), Value::Bool(has_conflict));

    // Worktree leaf — unwrapped value.
    let worktree_name = sniff::filesystem::git::get_current_worktree_name(dir)
        .ok()
        .flatten();
    map.insert("worktree".into(), json!(worktree_name));

    // Commit-family leaves — default period (last 3 days), local only.
    let commit_set = default_commit_family_set(dir)?;
    map.insert(
        "recent-commits".into(),
        commit_family_value(&commit_set, RecentCommitsMode::RecentCommits),
    );
    map.insert(
        "source-code-changes".into(),
        commit_family_value(&commit_set, RecentCommitsMode::SourceCodeChanges),
    );
    map.insert(
        "documentation-changes".into(),
        commit_family_value(&commit_set, RecentCommitsMode::DocumentationChanges),
    );

    Ok(Value::Object(map))
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
    use std::collections::HashMap;
    use std::path::PathBuf;

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
        }
    }

    fn fixture_with_git_and_repo() -> SniffResult {
        let repo = RepoInfo {
            is_monorepo: true,
            monorepo_tool: None,
            workspace_tools: Vec::new(),
            root: PathBuf::from("/tmp/repo"),
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
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
            monorepo_tool: None,
            workspace_tools: Vec::new(),
            root: PathBuf::from("/tmp/repo"),
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
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
                discovery_sources: vec![],
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
                monorepo_tool: None,
                workspace_tools: Vec::new(),
                root: PathBuf::from("/tmp/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
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
                discovery_sources: vec![],
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
                monorepo_tool: None,
                workspace_tools: Vec::new(),
                root: PathBuf::from("/tmp/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
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
        fn is_monorepo_outcome_wraps_bool() {
            let outcome = is_monorepo_outcome(true);
            assert_eq!(outcome.value, json!({ "is-monorepo": true }));
            assert!(outcome.exit_code.is_none());
        }

        #[test]
        fn package_count_outcome_wraps_number() {
            let outcome = package_count_outcome(42);
            assert_eq!(outcome.value, json!({ "package-count": 42 }));
            assert!(outcome.exit_code.is_none());
        }

        #[test]
        fn version_outcome_wraps_some_string() {
            let outcome = version_outcome(Some("1.2.3"), false);
            assert_eq!(outcome.value, json!({ "version": "1.2.3" }));
            assert!(outcome.exit_code.is_none());
        }

        #[test]
        fn version_outcome_none_sets_exit_code_one() {
            let outcome = version_outcome(None, false);
            assert_eq!(outcome.value, json!({ "version": null }));
            assert_eq!(outcome.exit_code, Some(1));
        }

        #[test]
        fn version_outcome_none_with_no_error_sets_exit_code_zero() {
            let outcome = version_outcome(None, true);
            assert_eq!(outcome.value, json!({ "version": null }));
            assert_eq!(outcome.exit_code, Some(0));
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
        //! Phase 5 — `deps --json` builder.
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
                discovery_sources: vec![],
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
                monorepo_tool: None,
                workspace_tools: Vec::new(),
                root: PathBuf::from("/tmp/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
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
            // `Package` struct but irrelevant to the `deps` contract.
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
            let action = RepoAction::Deps {
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
            let text_action = RepoAction::Deps {
                ui: false,
                svg: false,
                filter: vec![],
                package: None,
                package_area: None,
                width: None,
                orientation: None,
            };
            let ui_action = RepoAction::Deps {
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
                discovery_sources: vec![],
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
                monorepo_tool: None,
                workspace_tools: Vec::new(),
                root: PathBuf::from("/tmp/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
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
            // is_monorepo / root must still be present. (`workspace_tools`
            // and other Vec fields are `skip_serializing_if = "is_empty"`,
            // so we don't assert on them in this fixture.)
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
        use sniff::filesystem::repo::RepoIdentity;
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

        fn identity_fixture() -> RepoIdentity {
            RepoIdentity {
                name: "fixture-repo".into(),
                version: Some("1.0.0".into()),
                language: None,
                is_monorepo: false,
                package_count: Some(0),
            }
        }

        fn result_fixture(repo_root: &Path) -> SniffResult {
            let repo = RepoInfo {
                is_monorepo: false,
                monorepo_tool: None,
                workspace_tools: Vec::new(),
                root: repo_root.to_path_buf(),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                packages: None,
            };
            let mut git = fixture_git_info();
            git.repo_root = repo_root.to_path_buf();
            git.status = Some(sniff::filesystem::git::RepoStatus::default());
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

        #[test]
        fn default_commit_family_set_loads_without_error() {
            let (_temp, path) = temp_git_repo();
            let set = default_commit_family_set(&path).expect("load default commit set");
            assert!(
                set.commits.is_empty() || !set.commits.is_empty(),
                "commit set must be well-formed"
            );
        }

        #[test]
        fn aggregate_includes_all_participating_keys() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let identity = identity_fixture();
            let value = build_aggregate_value(&result, Some(&path), &identity)
                .expect("aggregate should build");

            let obj = value.as_object().expect("aggregate must be object");
            let keys: HashSet<_> = obj.keys().map(String::as_str).collect();

            let expected: HashSet<&str> = [
                // Identity leaves
                "name",
                "version",
                "language",
                "is-monorepo",
                "package-count",
                // Structure / dependency leaves
                "structure",
                "packages",
                "package-areas",
                "deps",
                // Git and file leaves
                "git-status",
                "staged-files",
                "unstaged-files",
                "untracked-files",
                "dirty-source-code",
                "staged-source-code",
                "unstaged-source-code",
                "dirty-files",
                // Package-context leaves
                "package",
                "package-area",
                "area",
                "package-root",
                "package-area-root",
                "root",
                // Change-family leaves
                "dirty-packages",
                "dirty-package-areas",
                "staged-packages",
                "staged-package-areas",
                "unstaged-packages",
                "unstaged-package-areas",
                // Boolean leaves
                "is-current-package-area-dirty",
                "package-area-has-source-code-changes",
                "has-merge-conflict",
                // History / worktree leaves
                "recent-commits",
                "source-code-changes",
                "documentation-changes",
                "worktree",
                "worktrees",
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
        fn aggregate_excludes_network_and_parameterized_keys() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let identity = identity_fixture();
            let value = build_aggregate_value(&result, Some(&path), &identity)
                .expect("aggregate should build");

            let obj = value.as_object().expect("aggregate must be object");
            for forbidden in ["remote", "pr", "hash"] {
                assert!(
                    !obj.contains_key(forbidden),
                    "aggregate must not contain `{forbidden}`: {value}"
                );
            }
        }

        #[test]
        fn aggregate_single_key_leaves_are_unwrapped() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let identity = identity_fixture();
            let value = build_aggregate_value(&result, Some(&path), &identity)
                .expect("aggregate should build");

            assert_eq!(value["name"], "fixture-repo");
            assert_eq!(value["version"], "1.0.0");
            assert_eq!(value["language"], Value::Null);
            assert_eq!(value["is-monorepo"], false);
            assert_eq!(value["package-count"], 0);
            assert_eq!(value["package"], "");
            assert_eq!(value["package-area"], "");
            assert_eq!(value["package-root"], "");
            assert_eq!(value["package-area-root"], "");
            assert!(
                value["root"].is_string() && !value["root"].as_str().unwrap().is_empty(),
                "root must be a non-empty string: {value}"
            );
            assert!(
                value["worktree"].is_string() || value["worktree"].is_null(),
                "worktree must be unwrapped: {value}"
            );
        }

        #[test]
        fn aggregate_file_list_leaves_have_stable_empty_shape() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let identity = identity_fixture();
            let value = build_aggregate_value(&result, Some(&path), &identity)
                .expect("aggregate should build");

            for key in [
                "staged-files",
                "unstaged-files",
                "untracked-files",
                "dirty-source-code",
                "staged-source-code",
                "unstaged-source-code",
                "dirty-files",
            ] {
                let leaf = &value[key];
                assert!(leaf.is_object(), "{key} must be an object: {value}");
                assert!(leaf.get("scope").is_some(), "{key} missing scope: {leaf}");
                assert!(leaf.get("kind").is_some(), "{key} missing kind: {leaf}");
                assert_eq!(
                    leaf["paths"],
                    json!([]),
                    "{key} paths must be empty: {leaf}"
                );
            }
        }

        #[test]
        fn aggregate_change_family_leaves_have_scope_kind_names() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let identity = identity_fixture();
            let value = build_aggregate_value(&result, Some(&path), &identity)
                .expect("aggregate should build");

            for key in [
                "dirty-packages",
                "dirty-package-areas",
                "staged-packages",
                "staged-package-areas",
                "unstaged-packages",
                "unstaged-package-areas",
            ] {
                let leaf = &value[key];
                assert!(leaf.is_object(), "{key} must be an object: {value}");
                assert!(leaf.get("scope").is_some(), "{key} missing scope: {leaf}");
                assert!(leaf.get("kind").is_some(), "{key} missing kind: {leaf}");
                assert_eq!(
                    leaf["names"],
                    json!([]),
                    "{key} names must be empty: {leaf}"
                );
            }
        }

        #[test]
        fn aggregate_commit_family_leaves_are_objects() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let identity = identity_fixture();
            let value = build_aggregate_value(&result, Some(&path), &identity)
                .expect("aggregate should build");

            for key in [
                "recent-commits",
                "source-code-changes",
                "documentation-changes",
            ] {
                assert!(
                    value[key].is_object(),
                    "{key} must be an object in aggregate: {value}"
                );
            }
        }

        #[test]
        fn aggregate_worktrees_contains_main_entry() {
            let (_temp, path) = temp_git_repo();
            let result = result_fixture(&path);
            let identity = identity_fixture();
            let value = build_aggregate_value(&result, Some(&path), &identity)
                .expect("aggregate should build");

            let worktrees = value["worktrees"]["worktrees"]
                .as_array()
                .expect("worktrees array");
            assert!(
                !worktrees.is_empty(),
                "main worktree must be present: {value}"
            );
        }

        #[test]
        fn aggregate_propagates_detection_errors_instead_of_partial_json() {
            // Point base_dir at a path that is not a git repository so local
            // detection required by the aggregate fails.
            let bad_path = PathBuf::from("/tmp/not-a-git-repo-for-aggregate-test-42");
            let result = result_fixture(&bad_path);
            let identity = identity_fixture();
            let outcome = build_aggregate_value(&result, Some(&bad_path), &identity);
            assert!(
                outcome.is_err(),
                "aggregate must fail rather than emit partial JSON"
            );
        }
    }
}
