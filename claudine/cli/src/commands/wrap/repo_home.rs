use std::collections::HashMap;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use color_eyre::eyre::{Context, Result, bail};

use claudine::linking::resolve_repo_root;
use claudine::provider::{Provider, provider_info};

pub struct RepoHomeManager {
    agent_offset: String,
    shadow_home: PathBuf,
}

impl RepoHomeManager {
    pub fn new(provider: Provider) -> Self {
        let agent_offset = provider.agent_offset().to_string();
        let user_home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));

        // Shadow home root: ~/.claudine/
        // Agent config in shadow: ~/.claudine/.claude (for Claude), etc.
        let shadow_home = user_home.join(".claudine").join(&agent_offset);

        Self {
            agent_offset,
            shadow_home,
        }
    }

    /// Returns the original agent config directory (e.g., ~/.claude for Claude)
    pub fn original_home(&self) -> PathBuf {
        if self.agent_offset == ".codex"
            && let Some(codex_home) = std::env::var_os("CODEX_HOME")
        {
            return PathBuf::from(codex_home);
        }
        let user_home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        user_home.join(&self.agent_offset)
    }

    pub fn ensure_shadow_home(&self) -> Result<PathBuf> {
        let original_home = self.original_home();
        if !original_home.is_absolute() {
            bail!(
                "original provider home '{}' must be an absolute path",
                biscuit_file::to_portable_string(&original_home)
            );
        }
        if !original_home.exists() {
            bail!(
                "original HOME directory '{}' does not exist",
                biscuit_file::to_portable_string(&original_home)
            );
        }

        if !self.shadow_home.exists() {
            fs::create_dir_all(&self.shadow_home).with_context(|| {
                format!(
                    "failed to create shadow HOME directory at '{}'",
                    biscuit_file::to_portable_string(&self.shadow_home)
                )
            })?;
        }

        Ok(self.shadow_home.clone())
    }

    pub fn sync_shadow_home(&self, repo_only: bool) -> Result<()> {
        let original_home = self.original_home();
        if !original_home.exists() {
            return Ok(());
        }

        // Live SQLite databases must never be symlinked into the shadow home.
        // Older shadow homes may still contain links created before that rule;
        // remove the entire volatile family once when such a link is detected
        // so the provider cannot open one main database through split sidecar
        // paths. Regular shadow-owned databases may contain recoverable state
        // and are preserved, even though Codex now uses its pre-shadow SQLite
        // home.
        purge_volatile_state(&self.shadow_home)?;

        for entry in fs::read_dir(original_home)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let source = entry.path();
            let dest = self.shadow_home.join(&file_name);

            if self.should_exclude(&file_name, repo_only) {
                continue;
            }

            // Live databases remain private to the shadow home.
            if is_volatile_state_file(&file_name) {
                continue;
            }

            if dest.exists() || dest.is_symlink() {
                if let Ok(dest_meta) = fs::symlink_metadata(&dest)
                    && dest_meta.file_type().is_symlink()
                {
                    let _ = fs::remove_file(&dest);
                } else {
                    continue;
                }
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::symlink;
                symlink(&source, &dest)?;
            }

            #[cfg(not(unix))]
            {
                if source.is_dir() {
                    if !dest.exists() {
                        fs::create_dir(&dest)?;
                    }
                    for sub_entry in fs::read_dir(&source)? {
                        let sub_entry = sub_entry?;
                        let sub_name = sub_entry.file_name();
                        let sub_source = source.join(&sub_name);
                        let sub_dest = dest.join(&sub_name);
                        if !sub_dest.exists() {
                            fs::hard_link(&sub_source, &sub_dest)?;
                        }
                    }
                } else {
                    fs::hard_link(&source, &dest)?;
                }
            }
        }

        Ok(())
    }

    fn should_exclude(&self, file_name: &std::ffi::OsStr, repo_only: bool) -> bool {
        let excluded = self.excluded_dirs(repo_only);
        let file_name_str = file_name.to_string_lossy();
        excluded
            .iter()
            .any(|&excluded| file_name_str == excluded || file_name_str == format!(".{}", excluded))
    }

    fn excluded_dirs(&self, repo_only: bool) -> Vec<&'static str> {
        if !repo_only {
            return match self.agent_offset.as_str() {
                ".codex" => vec!["prompts"],
                _ => vec![],
            };
        }

        match self.agent_offset.as_str() {
            ".claude" => vec!["skills", "commands", "agents", "hooks"],
            ".codex" => vec!["skills", "agents", "prompts"],
            ".gemini" => vec!["skills", "agents"],
            ".goose" => vec!["skills", "agents"],
            ".kimi" => vec!["skills", "agents"],
            ".opencode" => vec!["skills"],
            ".qwen" => vec!["skills", "commands"],
            _ => vec!["skills", "commands", "agents", "hooks"],
        }
    }
}

/// Remove legacy linked SQLite state from the shadow home.
///
/// A legacy database symlink can share the main file with the real home while
/// leaving WAL/SHM sidecars local to the shadow. If any volatile symlink remains,
/// the whole volatile family is removed to eliminate that split-path hazard.
/// Regular shadow-owned databases are legacy state and remain untouched.
///
/// A missing shadow directory is not an error: it simply means there is nothing
/// to sweep.
fn purge_volatile_state(shadow_home: &Path) -> Result<()> {
    let Ok(entries) = fs::read_dir(shadow_home) else {
        return Ok(());
    };

    let entries = entries.collect::<std::io::Result<Vec<_>>>()?;
    let has_legacy_link = entries.iter().any(|entry| {
        is_volatile_state_file(&entry.file_name())
            && fs::symlink_metadata(entry.path())
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
    });
    if !has_legacy_link {
        return Ok(());
    }

    for entry in entries
        .into_iter()
        .filter(|entry| is_volatile_state_file(&entry.file_name()))
    {
        remove_existing_path(&entry.path())?;
    }
    Ok(())
}

/// Live SQLite state databases and their WAL/SHM/journal sidecars. These are
/// per-environment runtime state, not shareable config — see the call site in
/// `sync_shadow_home` for why sharing them via symlink corrupts the DB.
fn is_volatile_state_file(file_name: &OsStr) -> bool {
    let name = file_name.to_string_lossy();
    name.ends_with(".sqlite")
        || name.ends_with(".sqlite-wal")
        || name.ends_with(".sqlite-shm")
        || name.ends_with(".sqlite-journal")
}

/// Resolve the SQLite directory Codex would use before Claudine changes `HOME`.
///
/// Codex's configured `sqlite_home` retains higher precedence when it reads its
/// configuration. This value supplies the native environment fallback: an
/// explicit `CODEX_SQLITE_HOME`, otherwise the effective pre-shadow Codex home.
pub(crate) fn codex_sqlite_home() -> Result<PathBuf> {
    let path = std::env::var_os("CODEX_SQLITE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    dirs::home_dir()
                        .unwrap_or_else(|| PathBuf::from("~"))
                        .join(".codex")
                })
        });
    if !path.is_absolute() {
        bail!(
            "Codex SQLite home '{}' must be an absolute path",
            biscuit_file::to_portable_string(&path)
        );
    }
    Ok(path)
}

pub fn needs_shadow_home(
    provider: Provider,
    cwd: &Path,
    repo_only: bool,
    effective_root: Option<&Path>,
) -> bool {
    repo_only
        || matches!(provider, Provider::Codex)
            && {
                let repo_root = effective_root
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| resolve_repo_root(cwd));
                codex_repo_prompts_source(&repo_root).is_some()
            }
}

/// Measured breakdown of [`build_repo_home_env`], for `--perf`.
///
/// `total` is the whole shadow-HOME materialization; `repo_root_detect` is the
/// time spent obtaining the repo root used for Codex prompt materialization.
/// When the caller supplies an `effective_root` this is microsecond-scale
/// local work (a clone); when no root is supplied it falls back to the
/// `resolve_repo_root` sniff git walk that previously dominated the stage.
/// Only produced when the caller passes `perf = true`.
#[derive(Debug, Clone, Copy)]
pub struct RepoHomeTimings {
    pub total: std::time::Duration,
    pub repo_root_detect: std::time::Duration,
}

#[allow(clippy::type_complexity)]
pub fn build_repo_home_env(
    provider: Provider,
    cwd: &Path,
    repo_only: bool,
    perf: bool,
    effective_root: Option<&Path>,
) -> Result<(
    HashMap<OsString, OsString>,
    Option<PathBuf>,
    Option<RepoHomeTimings>,
)> {
    let total_start = perf.then(std::time::Instant::now);
    let manager = RepoHomeManager::new(provider);
    let user_home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    let shadow_home = manager.ensure_shadow_home()?;
    manager.sync_shadow_home(repo_only)?;

    let repo_root_start = perf.then(std::time::Instant::now);
    let repo_root = effective_root
        .map(Path::to_path_buf)
        .unwrap_or_else(|| resolve_repo_root(cwd));
    let repo_root_detect = repo_root_start.map(|t| t.elapsed());
    materialize_repo_scoped_resources(
        provider,
        &shadow_home,
        &manager.original_home(),
        &repo_root,
        repo_only,
    )?;

    let mut env = HashMap::new();
    // Set HOME to the shadow home root (~/.claudine), not the agent subdirectory
    // This makes the agent look in ~/.claude/ for its config, which is a symlink
    let shadow_home_root = shadow_home
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| user_home.join(".claudine"));
    materialize_root_level_state(provider, &user_home, &shadow_home_root)?;
    env.insert(OsString::from("HOME"), OsString::from(shadow_home_root));
    if provider == Provider::Codex {
        env.insert(
            OsString::from("CODEX_SQLITE_HOME"),
            OsString::from(codex_sqlite_home()?),
        );
    }

    let timings = total_start.map(|t| RepoHomeTimings {
        total: t.elapsed(),
        repo_root_detect: repo_root_detect.unwrap_or_default(),
    });

    Ok((env, Some(shadow_home), timings))
}

fn materialize_repo_scoped_resources(
    provider: Provider,
    shadow_home: &Path,
    original_home: &Path,
    repo_root: &Path,
    repo_only: bool,
) -> Result<()> {
    if provider == Provider::Codex {
        materialize_codex_prompts(
            &original_home.join("prompts"),
            &shadow_home.join("prompts"),
            repo_root,
            repo_only,
        )?;
    }

    Ok(())
}

fn materialize_root_level_state(
    provider: Provider,
    user_home: &Path,
    shadow_home_root: &Path,
) -> Result<()> {
    for relative_path in root_level_state_files(provider) {
        let source = user_home.join(relative_path);
        if !source.exists() {
            continue;
        }

        let dest = shadow_home_root.join(relative_path);
        remove_existing_path(&dest)?;
        link_or_copy_file(&source, &dest)?;
    }

    Ok(())
}

fn root_level_state_files(provider: Provider) -> &'static [&'static str] {
    provider_info(provider).repo_home_root_files
}

fn codex_repo_prompts_source(repo_root: &Path) -> Option<PathBuf> {
    [
        repo_root.join(".codex/prompts"),
        repo_root.join(".claude/commands"),
    ]
    .into_iter()
    .find(|path| path.is_dir())
}

fn materialize_codex_prompts(
    user_prompts: &Path,
    dest: &Path,
    repo_root: &Path,
    repo_only: bool,
) -> Result<()> {
    remove_existing_path(dest)?;
    fs::create_dir_all(dest)?;

    if !repo_only {
        merge_prompt_tree(user_prompts, dest, false)?;
    }

    if let Some(repo_prompts) = codex_repo_prompts_source(repo_root) {
        merge_prompt_tree(&repo_prompts, dest, true)?;
    }

    Ok(())
}

fn merge_prompt_tree(source_root: &Path, dest_root: &Path, overwrite: bool) -> Result<()> {
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
            merge_prompt_tree(&source_path, &dest_path, overwrite)?;
            continue;
        }

        if overwrite {
            remove_existing_path(&dest_path)?;
        } else if dest_path.exists() || dest_path.is_symlink() {
            continue;
        }

        link_or_copy_file(&source_path, &dest_path)?;
    }

    Ok(())
}

fn link_or_copy_file(source: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, dest)?;
    }

    #[cfg(windows)]
    {
        if let Ok(()) = std::os::windows::fs::symlink_file(source, dest) {
            return Ok(());
        }
        fs::copy(source, dest)?;
    }

    #[cfg(not(any(unix, windows)))]
    {
        fs::copy(source, dest)?;
    }

    Ok(())
}

fn remove_existing_path(path: &Path) -> Result<()> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };

    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)?;
    } else if metadata.is_dir() {
        fs::remove_dir_all(path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests;
