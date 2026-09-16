//! Provider-overlay materialization, driven by an [`OverlayPlan`].
//!
//! The plan decides *what* the overlay holds; this module puts it on disk. Each
//! launch builds into a fresh root it owns through an [`OverlayLease`], so the
//! view is exactly this plan's: nothing an earlier launch excluded, deleted, or
//! injected can survive into it, and a concurrent launch cannot change it. The
//! lease removes the root once the plan and every clone of it are dropped; a
//! root orphaned by a process that never dropped it is reclaimed by the next
//! launch's sweep. The source root is classified entry by entry rather than
//! mirrored wholesale:
//!
//! - names the plan excludes (the `--repo` isolation set, and every entry a
//!   materialization owns) are omitted;
//! - live state — SQLite databases and their sidecars, lock files, sockets —
//!   is never linked or copied; a provider that needs it reachable pins it
//!   outside the overlay through a provider-native state selector instead;
//! - everything else is mirrored: a symbolic link on Unix, a recursive copy on
//!   native Windows, where a directory cannot be hard-linked and a mutable
//!   file must not be. A provider's write through a linked entry reaches the
//!   user's source directly. A top-level mirrored *file* the provider replaced
//!   or rewrote is carried back by the lease's guarded [`WriteBack`] when the
//!   launch ends, unless the source changed meanwhile; a new entry, or a change
//!   deeper inside a copied directory, ends with the launch;
//! - materialized entries become real content, with repository-scoped
//!   resources merged over the user's.
//!
//! Every file this module copies is written through
//! [`claudine::config::atomic::atomic_write`].
//!
//! The spec is `fixes/2026-09-12-shadow-home/spec.md`.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use color_eyre::eyre::Result;

use claudine::config::atomic::atomic_write;
use claudine::error::ClaudineError;
use claudine::invocation_context::{EnvBaseline, HOME_VARIABLES, HomeBaseline};
use claudine::linking::resolve_repo_root;
use claudine::provider::{Provider, provider_info};
use claudine::provider_overlay::{
    OverlayCapability, OverlayEntryKind, OverlayLease, OverlayMaterialization, OverlayPlan,
    OverlayPlanner, OverlayReason, OverlayReasons, OverlayResourceClass, OverlayStage, WriteBack,
    sweep_abandoned_overlays,
};

use super::profile::profile_for_provider;

/// How mirrored entries reach the overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MirrorMode {
    /// A symbolic link per top-level entry.
    Link,
    /// A recursive copy.
    Copy,
}

impl MirrorMode {
    /// The strategy this platform supports without elevated privileges.
    pub(crate) const fn native() -> Self {
        if cfg!(unix) { Self::Link } else { Self::Copy }
    }
}

/// The reasons a launch raises, from the wrapper's launch facts.
///
/// `mcp_requested` is `--mcp`/`--use`. It raises `Mcp` only for a provider with
/// a verdict for it; a provider with no runtime injector keeps its `claudine mcp
/// export` guidance instead of refusing (audit D1). `repo_root` is where repo
/// prompts are discovered.
pub fn overlay_reasons(
    provider: Provider,
    repo_resources: bool,
    mcp_requested: bool,
    repo_root: &Path,
) -> OverlayReasons {
    let mut reasons = OverlayReasons::none();
    if repo_resources {
        reasons.insert(OverlayReason::RepoResources);
    }
    if mcp_requested
        && provider.overlay_capability(OverlayReason::Mcp) != OverlayCapability::Unsupported
    {
        reasons.insert(OverlayReason::Mcp);
    }
    if provider.overlay_capability(OverlayReason::RepoPrompt) != OverlayCapability::Unsupported
        && codex_repo_prompts_source(repo_root).is_some()
    {
        reasons.insert(OverlayReason::RepoPrompt);
    }
    reasons
}

/// Measured breakdown of [`build_overlay`], for `--perf`.
///
/// `repo_root_detect` is microsecond-scale when the caller supplies an
/// `effective_root`, and the `resolve_repo_root` sniff git walk otherwise.
#[derive(Debug, Clone, Copy)]
pub struct OverlayTimings {
    pub total: std::time::Duration,
    pub repo_root_detect: std::time::Duration,
}

/// Plan and materialize the overlay for one launch.
///
/// Returns the plan, whose [`OverlayPlan::env_patch`] the caller applies with
/// [`apply_overlay_env`], and timings when `perf` is set. A plan satisfied
/// inline (OpenCode MCP) is returned without touching the filesystem.
///
/// The returned plan owns the launch's overlay root: keep it (or a clone)
/// alive until the provider exits, because dropping the last one removes the
/// root.
///
/// ## Errors
///
/// [`ClaudineError::ProviderOverlayUnsupported`] when the provider has no
/// verified mechanism for a requested reason, and
/// [`ClaudineError::ProviderOverlayFailed`] when the overlay cannot be built.
/// Either way nothing has been spawned, and there is no fallback launch.
pub fn build_overlay(
    provider: Provider,
    reasons: OverlayReasons,
    cwd: &Path,
    perf: bool,
    effective_root: Option<&Path>,
    home: &HomeBaseline,
    env_baseline: &EnvBaseline,
) -> Result<(OverlayPlan, Option<OverlayTimings>)> {
    let total_start = perf.then(std::time::Instant::now);
    let mut plan = OverlayPlanner::new(home, env_baseline)
        .plan(provider, reasons)
        .map_err(ClaudineError::from)?;
    if !plan.requires_materialization() {
        return Ok((plan, None));
    }
    if let Some(profile) = profile_for_provider(provider) {
        profile.overlay_strategy(&mut plan, env_baseline)?;
    }

    let repo_root_start = perf.then(std::time::Instant::now);
    let repo_root = effective_root
        .map(Path::to_path_buf)
        .unwrap_or_else(|| resolve_repo_root(cwd));
    let repo_root_detect = repo_root_start.map(|t| t.elapsed());

    if let Some(mut lease) = materialize(&plan, &repo_root, MirrorMode::native())? {
        materialize_root_level_state(&plan, MirrorMode::native(), lease.write_back(provider))
            .map_err(overlay_failure(&plan, OverlayStage::Materialization))?;
        plan.hold_lease(lease);
    }

    let timings = total_start.map(|t| OverlayTimings {
        total: t.elapsed(),
        repo_root_detect: repo_root_detect.unwrap_or_default(),
    });
    Ok((plan, timings))
}

/// Write a plan's provider-owned patch into a child environment.
///
/// The only place overlay entries reach a child environment. A home variable
/// is never written (Invariant 1): a plan cannot name one, so a patch that does
/// is a profile bug, caught by `debug_assert!` and skipped in release rather
/// than moving the child's home.
pub(crate) fn apply_overlay_env(env: &mut HashMap<OsString, OsString>, plan: &OverlayPlan) {
    env.extend(overlay_env_entries(plan));
}

/// A plan's provider-owned patch as entries, with the same home-variable guard
/// as [`apply_overlay_env`], for a caller that records a patch rather than
/// writing a map.
pub(crate) fn overlay_env_entries(plan: &OverlayPlan) -> Vec<(OsString, OsString)> {
    plan.env_patch()
        .into_iter()
        .filter(|(name, _)| {
            let is_home = is_home_variable(name);
            debug_assert!(!is_home, "overlay patch names home variable {name:?}");
            !is_home
        })
        .collect()
}

/// Return every provider-owned overlay variable in `env` to the launch
/// baseline: the ambient value when the user supplied one, absent otherwise.
///
/// The names are every provider's selector plus whatever `written` patched —
/// the external state selectors a profile pinned (`CODEX_SQLITE_HOME`,
/// `CLAUDE_SECURESTORAGE_CONFIG_DIR`) are not provider metadata. A provider
/// transition starts from this, so no selector the previous provider owned
/// reaches the next attempt (Invariant 7).
pub(crate) fn restore_overlay_selectors(
    env: &mut HashMap<OsString, OsString>,
    written: Option<&OverlayPlan>,
    baseline: &EnvBaseline,
) {
    let mut names: BTreeSet<OsString> = claudine::provider::all_providers()
        .filter_map(|info| info.overlay_selector)
        .map(|spec| OsString::from(spec.env_var))
        .collect();
    names.extend(written.into_iter().flat_map(|plan| plan.env_patch()).map(|(name, _)| name));
    for name in names {
        match baseline.get(&name) {
            Some(value) => env.insert(name, value.to_os_string()),
            None => env.remove(&name),
        };
    }
}

/// Windows resolves variable names case-insensitively, Unix does not.
fn is_home_variable(name: &OsStr) -> bool {
    HOME_VARIABLES.iter().any(|home| {
        if cfg!(windows) {
            name.to_string_lossy().eq_ignore_ascii_case(home)
        } else {
            name == *home
        }
    })
}

/// The first home variable in `env` that carries a null device or a path into
/// Claudine's overlay storage, with its value.
///
/// Invariant 1 as a pure check over a finished child environment, so every
/// spawn route asserts it at the one place they share.
pub(crate) fn home_identity_violation(
    env: &HashMap<OsString, OsString>,
) -> Option<(&OsStr, &OsStr)> {
    env.iter()
        .filter(|(name, _)| is_home_variable(name))
        .find(|(_, value)| {
            let text = value.to_string_lossy();
            text == "/dev/null"
                || text.eq_ignore_ascii_case("NUL")
                || Path::new(value)
                    .components()
                    .any(|component| component.as_os_str() == OVERLAY_HOME_DIR)
        })
        .map(|(name, value)| (name.as_os_str(), value.as_os_str()))
}

/// The directory under the user home that holds every provider overlay, as
/// `OverlayPlanner` places it.
const OVERLAY_HOME_DIR: &str = ".claudine";

/// Build the overlay a plan describes into a new root owned by the returned
/// lease, first reclaiming roots whose launches have ended.
///
/// `repo_root` supplies the repository's copy of every repo-scoped
/// materialization. The source root is only ever read. A default source root
/// the provider has not created yet mirrors nothing; an explicit one must exist.
/// Returns `None` for a plan with no storage. A failure after the root exists
/// drops the lease, which removes the partial root.
pub(crate) fn materialize(
    plan: &OverlayPlan,
    repo_root: &Path,
    mode: MirrorMode,
) -> Result<Option<OverlayLease>, ClaudineError> {
    let (Some(source_root), Some(storage_root), Some(visible_root)) =
        (plan.source_root(), plan.storage_root(), plan.provider_visible_root())
    else {
        return Ok(None);
    };

    let source_failure = overlay_failure(plan, OverlayStage::SourceRoot);
    if !source_root.is_absolute() {
        return Err(source_failure(io::Error::new(
            io::ErrorKind::InvalidInput,
            "provider source root must be an absolute path",
        )));
    }
    let source_exists = match fs::metadata(source_root) {
        Ok(metadata) if metadata.is_dir() => true,
        Ok(_) => {
            return Err(source_failure(io::Error::new(
                io::ErrorKind::NotADirectory,
                "provider source root is not a directory",
            )));
        }
        // A provider that has never run has no default root yet, and reads
        // nothing from it. The overlay starts empty, as the pre-overlay launch
        // did. A root the user named must exist, or their configuration would
        // be dropped without a word.
        Err(error) if error.kind() == io::ErrorKind::NotFound && !plan.source_root_is_explicit() => false,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(source_failure(io::Error::new(
                io::ErrorKind::NotFound,
                "provider source root does not exist",
            )));
        }
        Err(error) => return Err(source_failure(error)),
    };

    // `<launches>/<provider>/<launch-id>`: every provider's roots are swept.
    if let Some(launches) = storage_root.parent().and_then(Path::parent) {
        sweep_abandoned_overlays(launches);
    }
    let storage_failure = overlay_failure(plan, OverlayStage::StorageRoot);
    let mut lease = OverlayLease::acquire(storage_root).map_err(&storage_failure)?;
    fs::create_dir_all(visible_root).map_err(&storage_failure)?;

    let materialization_failure = overlay_failure(plan, OverlayStage::Materialization);
    if source_exists {
        let write_back = lease.write_back(plan.provider());
        mirror_source_root(source_root, visible_root, &plan.excluded_resources(), mode, write_back)
            .map_err(&materialization_failure)?;
    }

    let repo_resources = plan.reasons().contains(OverlayReason::RepoResources);
    for entry in plan.materializations() {
        materialize_entry(entry, repo_root, repo_resources, mode).map_err(&materialization_failure)?;
    }
    Ok(Some(lease))
}

fn overlay_failure(plan: &OverlayPlan, stage: OverlayStage) -> impl Fn(io::Error) -> ClaudineError {
    let provider = plan.provider();
    let reason = plan
        .reasons()
        .iter()
        .next()
        .unwrap_or(OverlayReason::RepoResources);
    move |source| ClaudineError::ProviderOverlayFailed {
        provider,
        reason,
        stage,
        source,
    }
}

/// Mirror every eligible top-level source entry, recording each mirrored file
/// in `write_back`. Excluded names and live state are neither mirrored nor
/// recorded.
fn mirror_source_root(
    source_root: &Path,
    visible_root: &Path,
    excluded: &BTreeSet<String>,
    mode: MirrorMode,
    write_back: &mut WriteBack,
) -> io::Result<()> {
    for entry in fs::read_dir(source_root)? {
        let entry = entry?;
        let file_name = entry.file_name();
        if is_excluded(excluded, &file_name) || is_live_state(&file_name, &entry.file_type()?) {
            continue;
        }

        let source = entry.path();
        let dest = visible_root.join(&file_name);
        match mode {
            #[cfg(unix)]
            MirrorMode::Link => std::os::unix::fs::symlink(&source, &dest)?,
            #[cfg(not(unix))]
            MirrorMode::Link => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "link mirroring requires Unix symbolic links",
                ));
            }
            MirrorMode::Copy => copy_entry(&source, &dest, &mut HashSet::new())?,
        }
        write_back.record(&source, &dest)?;
    }
    Ok(())
}

/// Names match bare or dot-prefixed, as the `--repo` isolation table is
/// written without the leading dot some providers use on disk.
fn is_excluded(excluded: &BTreeSet<String>, file_name: &OsStr) -> bool {
    let name = file_name.to_string_lossy();
    let bare = name.strip_prefix('.').unwrap_or(&name);
    excluded.contains(name.as_ref()) || excluded.contains(bare)
}

/// Recursively copy `source` to `dest`, skipping live state at every depth.
///
/// `visited` holds canonical directories already entered, so a directory link
/// cycle ends instead of recursing forever.
fn copy_entry(source: &Path, dest: &Path, visited: &mut HashSet<PathBuf>) -> io::Result<()> {
    let metadata = fs::metadata(source)?;
    if metadata.is_dir() {
        if !visited.insert(fs::canonicalize(source)?) {
            return Ok(());
        }
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let file_name = entry.file_name();
            if is_live_state(&file_name, &entry.file_type()?) {
                continue;
            }
            copy_entry(&entry.path(), &dest.join(&file_name), visited)?;
        }
        return Ok(());
    }
    if !metadata.is_file() {
        return Ok(());
    }
    atomic_write(dest, &fs::read(source)?)
}

fn materialize_entry(
    entry: &OverlayMaterialization,
    repo_root: &Path,
    repo_resources: bool,
    mode: MirrorMode,
) -> io::Result<()> {
    if entry.class == OverlayResourceClass::State {
        // State is pinned outside the overlay, never placed inside it.
        return Ok(());
    }
    match entry.kind {
        OverlayEntryKind::Directory if entry.repo_scoped => materialize_prompt_overlay(
            &entry.source,
            &entry.destination,
            repo_root,
            !repo_resources,
            mode,
        ),
        OverlayEntryKind::Directory => {
            if !entry.source.is_dir() {
                return Ok(());
            }
            remove_existing_path(&entry.destination)?;
            copy_entry(&entry.source, &entry.destination, &mut HashSet::new())
        }
        OverlayEntryKind::File => {
            if fs::symlink_metadata(&entry.destination).is_ok_and(|meta| meta.file_type().is_symlink()) {
                remove_existing_path(&entry.destination)?;
            }
            if !entry.source.is_file() {
                return Ok(());
            }
            atomic_write(&entry.destination, &fs::read(&entry.source)?)
        }
    }
}

/// Live SQLite state databases and their WAL/SHM/journal sidecars. These are
/// per-environment runtime state, not shareable config: a symlinked main
/// database with overlay-local sidecars corrupts the database.
fn is_volatile_state_file(file_name: &OsStr) -> bool {
    let name = file_name.to_string_lossy();
    name.ends_with(".sqlite")
        || name.ends_with(".sqlite-wal")
        || name.ends_with(".sqlite-shm")
        || name.ends_with(".sqlite-journal")
}

/// Entries that belong to a running process rather than to configuration:
/// SQLite state, lock files, and anything that is not a file, directory, or
/// symbolic link (sockets, FIFOs).
fn is_live_state(file_name: &OsStr, file_type: &fs::FileType) -> bool {
    is_volatile_state_file(file_name)
        || file_name.to_string_lossy().ends_with(".lock")
        || !(file_type.is_file() || file_type.is_dir() || file_type.is_symlink())
}

/// Place the provider's home-root files beside its configuration.
///
/// Claude reads `.claude.json` from `$CLAUDE_CONFIG_DIR` when the selector is
/// set and from the home root otherwise, so a default-rooted Claude overlay
/// needs the user's file inside the provider-visible root. An explicit source
/// root already holds its own copy, which the mirror carries. Each placed file
/// is recorded in `write_back`: Claude rewrites it during a session.
fn materialize_root_level_state(
    plan: &OverlayPlan,
    mode: MirrorMode,
    write_back: &mut WriteBack,
) -> io::Result<()> {
    let (Some(source_root), Some(visible_root)) = (plan.source_root(), plan.provider_visible_root())
    else {
        return Ok(());
    };
    let Some(home_root) = source_root.parent().filter(|_| !plan.source_root_is_explicit()) else {
        return Ok(());
    };
    for relative_path in provider_info(plan.provider()).repo_home_root_files {
        let source = home_root.join(relative_path);
        if !source.exists() {
            continue;
        }

        let dest = visible_root.join(relative_path);
        remove_existing_path(&dest)?;
        link_or_copy_file(&source, &dest, mode)?;
        write_back.record(&source, &dest)?;
    }

    Ok(())
}

fn codex_repo_prompts_source(repo_root: &Path) -> Option<PathBuf> {
    [
        repo_root.join(".codex").join("prompts"),
        repo_root.join(".claude").join("commands"),
    ]
    .into_iter()
    .find(|path| path.is_dir())
}

/// Rebuild a prompt directory: the user's prompts first (unless `--repo` hides
/// them), then the repository's, which win on a name collision.
fn materialize_prompt_overlay(
    user_prompts: &Path,
    dest: &Path,
    repo_root: &Path,
    include_user_prompts: bool,
    mode: MirrorMode,
) -> io::Result<()> {
    remove_existing_path(dest)?;
    fs::create_dir_all(dest)?;

    if include_user_prompts {
        merge_prompt_tree(user_prompts, dest, false, mode)?;
    }

    if let Some(repo_prompts) = codex_repo_prompts_source(repo_root) {
        merge_prompt_tree(&repo_prompts, dest, true, mode)?;
    }

    Ok(())
}

fn merge_prompt_tree(
    source_root: &Path,
    dest_root: &Path,
    overwrite: bool,
    mode: MirrorMode,
) -> io::Result<()> {
    if !source_root.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(source_root)? {
        let entry = entry?;
        let source_path = entry.path();
        let Some(file_name) = source_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if file_name.starts_with('.') {
            continue;
        }

        let dest_path = dest_root.join(file_name);

        if source_path.is_dir() {
            fs::create_dir_all(&dest_path)?;
            merge_prompt_tree(&source_path, &dest_path, overwrite, mode)?;
            continue;
        }

        if overwrite {
            remove_existing_path(&dest_path)?;
        } else if dest_path.exists() || dest_path.is_symlink() {
            continue;
        }

        link_or_copy_file(&source_path, &dest_path, mode)?;
    }

    Ok(())
}

fn link_or_copy_file(source: &Path, dest: &Path, mode: MirrorMode) -> io::Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    match mode {
        #[cfg(unix)]
        MirrorMode::Link => std::os::unix::fs::symlink(source, dest),
        _ => atomic_write(dest, &fs::read(source)?),
    }
}

fn remove_existing_path(path: &Path) -> io::Result<()> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };

    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else if metadata.file_type().is_symlink() {
        // A Windows directory symlink or junction is removed as a directory.
        fs::remove_file(path).or_else(|_| fs::remove_dir(path))
    } else {
        fs::remove_file(path)
    }
}

#[cfg(test)]
mod tests;
