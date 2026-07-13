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
        let user_home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        user_home.join(&self.agent_offset)
    }

    pub fn ensure_shadow_home(&self) -> Result<PathBuf> {
        let original_home = self.original_home();
        if !original_home.exists() {
            bail!(
                "original HOME directory '{}' does not exist",
                original_home.display()
            );
        }

        if !self.shadow_home.exists() {
            fs::create_dir_all(&self.shadow_home).with_context(|| {
                format!(
                    "failed to create shadow HOME directory at '{}'",
                    self.shadow_home.display()
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
        // so the provider rebuilds a self-consistent, shadow-owned database.
        // Regular shadow-owned databases are preserved across launches.
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
/// the whole volatile family is removed so Codex can rebuild it consistently.
/// Once the files are regular shadow-owned state, retaining them avoids rebuilding
/// tens of megabytes of SQLite data on every interactive launch.
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
mod tests {
    use std::process::Command;

    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    fn init_git_repo(path: &Path) -> bool {
        Command::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(path)
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[test]
    fn purge_volatile_state_preserves_shadow_owned_databases() {
        let tmp = TempDir::new().unwrap();
        let shadow = tmp.path();

        fs::write(shadow.join("goals_1.sqlite"), b"db").unwrap();
        fs::write(shadow.join("goals_1.sqlite-wal"), b"wal").unwrap();
        fs::write(shadow.join("goals_1.sqlite-shm"), b"stale-shm").unwrap();
        fs::write(shadow.join("config.toml"), b"cfg").unwrap();

        purge_volatile_state(shadow).unwrap();

        assert!(shadow.join("goals_1.sqlite").exists());
        assert!(shadow.join("goals_1.sqlite-wal").exists());
        assert!(shadow.join("goals_1.sqlite-shm").exists());
        assert!(shadow.join("config.toml").exists());
    }

    #[cfg(unix)]
    #[test]
    fn purge_volatile_state_removes_legacy_link_and_sidecars() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source.sqlite");
        let shadow = tmp.path().join("shadow");
        fs::write(&source, b"db").unwrap();
        fs::create_dir(&shadow).unwrap();
        std::os::unix::fs::symlink(&source, shadow.join("goals_1.sqlite")).unwrap();
        fs::write(shadow.join("goals_1.sqlite-wal"), b"wal").unwrap();
        fs::write(shadow.join("goals_1.sqlite-shm"), b"stale-shm").unwrap();
        fs::write(shadow.join("config.toml"), b"cfg").unwrap();

        purge_volatile_state(&shadow).unwrap();

        assert!(!shadow.join("goals_1.sqlite").exists());
        assert!(!shadow.join("goals_1.sqlite-wal").exists());
        assert!(!shadow.join("goals_1.sqlite-shm").exists());
        assert!(shadow.join("config.toml").exists());
        assert!(source.exists());
    }

    #[test]
    fn purge_volatile_state_tolerates_missing_shadow_dir() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist");
        purge_volatile_state(&missing).unwrap();
    }

    #[test]
    fn volatile_state_files_match_live_dbs_only() {
        // Live DBs + sidecars must be detected (never shared via symlink).
        assert!(is_volatile_state_file(OsStr::new("state_5.sqlite")));
        assert!(is_volatile_state_file(OsStr::new("logs_2.sqlite-wal")));
        assert!(is_volatile_state_file(OsStr::new("memories_1.sqlite-shm")));
        assert!(is_volatile_state_file(OsStr::new("goals_1.sqlite-journal")));

        // Shared config and codex's own repair backups must NOT match.
        assert!(!is_volatile_state_file(OsStr::new("config.toml")));
        assert!(!is_volatile_state_file(OsStr::new("auth.json")));
        assert!(!is_volatile_state_file(OsStr::new(
            "state_5.sqlite.codex-repair-1780436523.0.bak"
        )));
    }

    #[test]
    fn codex_repo_prompts_source_prefers_codex_dir_then_claude_commands() {
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path();
        let claude_commands = repo_root.join(".claude/commands");
        let codex_prompts = repo_root.join(".codex/prompts");

        fs::create_dir_all(&claude_commands).unwrap();
        assert_eq!(
            codex_repo_prompts_source(repo_root).as_deref(),
            Some(claude_commands.as_path())
        );

        fs::create_dir_all(&codex_prompts).unwrap();
        assert_eq!(
            codex_repo_prompts_source(repo_root).as_deref(),
            Some(codex_prompts.as_path())
        );
    }

    #[cfg(unix)]
    #[test]
    fn materialize_repo_scoped_resources_merges_user_and_repo_prompts() {
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path().join("repo");
        let shadow_home = tmp.path().join("shadow/.codex");
        let original_home = tmp.path().join("home/.codex");
        let user_prompts = original_home.join("prompts");
        let claude_commands = repo_root.join(".claude/commands");
        let prompts_dir = shadow_home.join("prompts");
        let repo_review = claude_commands.join("review.md");
        let user_review = user_prompts.join("review.md");
        let user_commit = user_prompts.join("commit.md");

        fs::create_dir_all(&claude_commands).unwrap();
        fs::create_dir_all(&user_prompts).unwrap();
        fs::create_dir_all(&shadow_home).unwrap();
        fs::write(&user_review, "user review").unwrap();
        fs::write(&user_commit, "user commit").unwrap();
        fs::write(&repo_review, "repo review").unwrap();
        fs::create_dir_all(claude_commands.join("nested")).unwrap();
        fs::write(claude_commands.join("nested/plan.md"), "repo nested").unwrap();

        materialize_repo_scoped_resources(
            Provider::Codex,
            &shadow_home,
            &original_home,
            &repo_root,
            false,
        )
        .unwrap();

        assert_eq!(
            fs::read_link(prompts_dir.join("review.md")).unwrap(),
            repo_review
        );
        assert_eq!(
            fs::read_link(prompts_dir.join("commit.md")).unwrap(),
            user_commit
        );
        assert_eq!(
            fs::read_link(prompts_dir.join("nested/plan.md")).unwrap(),
            claude_commands.join("nested/plan.md")
        );
    }

    #[cfg(unix)]
    #[test]
    fn materialize_repo_scoped_resources_repo_only_uses_repo_prompts() {
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path().join("repo");
        let shadow_home = tmp.path().join("shadow/.codex");
        let original_home = tmp.path().join("home/.codex");
        let user_prompts = original_home.join("prompts");
        let claude_commands = repo_root.join(".claude/commands");
        let prompts_dir = shadow_home.join("prompts");

        fs::create_dir_all(&user_prompts).unwrap();
        fs::create_dir_all(&claude_commands).unwrap();
        fs::create_dir_all(&shadow_home).unwrap();
        fs::write(user_prompts.join("commit.md"), "user commit").unwrap();
        fs::write(claude_commands.join("review.md"), "repo review").unwrap();

        materialize_repo_scoped_resources(
            Provider::Codex,
            &shadow_home,
            &original_home,
            &repo_root,
            true,
        )
        .unwrap();

        assert!(fs::symlink_metadata(prompts_dir.join("commit.md")).is_err());
        assert_eq!(
            fs::read_link(prompts_dir.join("review.md")).unwrap(),
            claude_commands.join("review.md")
        );
    }

    #[cfg(unix)]
    #[test]
    fn materialize_repo_scoped_resources_replaces_existing_repo_overlay_when_switching_repos() {
        let tmp = TempDir::new().unwrap();
        let first_repo = tmp.path().join("repo-one");
        let second_repo = tmp.path().join("repo-two");
        let original_home = tmp.path().join("home/.codex");
        let shadow_home = tmp.path().join("shadow/.codex");
        let prompts_dir = shadow_home.join("prompts");
        let first_review = first_repo.join(".claude/commands/review.md");
        let second_review = second_repo.join(".claude/commands/review.md");

        fs::create_dir_all(first_review.parent().unwrap()).unwrap();
        fs::create_dir_all(second_review.parent().unwrap()).unwrap();
        fs::create_dir_all(&shadow_home).unwrap();
        fs::create_dir_all(original_home.join("prompts")).unwrap();
        fs::write(&first_review, "repo one").unwrap();
        fs::write(&second_review, "repo two").unwrap();

        materialize_repo_scoped_resources(
            Provider::Codex,
            &shadow_home,
            &original_home,
            &first_repo,
            false,
        )
        .unwrap();
        assert_eq!(
            fs::read_link(prompts_dir.join("review.md")).unwrap(),
            first_review
        );

        materialize_repo_scoped_resources(
            Provider::Codex,
            &shadow_home,
            &original_home,
            &second_repo,
            false,
        )
        .unwrap();
        assert_eq!(
            fs::read_link(prompts_dir.join("review.md")).unwrap(),
            second_review
        );
    }

    #[cfg(unix)]
    #[test]
    fn materialize_root_level_state_preserves_claude_global_state() {
        let tmp = TempDir::new().unwrap();
        let user_home = tmp.path().join("home");
        let shadow_home_root = tmp.path().join("shadow");
        let source = user_home.join(".claude.json");
        let dest = shadow_home_root.join(".claude.json");

        fs::create_dir_all(&user_home).unwrap();
        fs::create_dir_all(&shadow_home_root).unwrap();
        fs::write(&source, "{\"hasCompletedOnboarding\":true}").unwrap();

        materialize_root_level_state(Provider::Claude, &user_home, &shadow_home_root).unwrap();

        assert_eq!(fs::read_link(&dest).unwrap(), source);
    }

    #[cfg(unix)]
    #[test]
    fn materialize_root_level_state_replaces_stale_shadow_file() {
        let tmp = TempDir::new().unwrap();
        let user_home = tmp.path().join("home");
        let shadow_home_root = tmp.path().join("shadow");
        let source = user_home.join(".claude.json");
        let dest = shadow_home_root.join(".claude.json");

        fs::create_dir_all(&user_home).unwrap();
        fs::create_dir_all(&shadow_home_root).unwrap();
        fs::write(&source, "{\"userID\":\"real\"}").unwrap();
        fs::write(&dest, "{\"userID\":\"stale\"}").unwrap();

        materialize_root_level_state(Provider::Claude, &user_home, &shadow_home_root).unwrap();

        assert_eq!(fs::read_link(&dest).unwrap(), source);
    }

    #[test]
    fn needs_shadow_home_supplied_effective_root_used_for_codex_detection() {
        let tmp = TempDir::new().unwrap();
        let with_prompts = tmp.path().join("with-prompts");
        let without_prompts = tmp.path().join("without-prompts");
        fs::create_dir_all(with_prompts.join(".codex/prompts")).unwrap();
        fs::create_dir_all(&without_prompts).unwrap();

        // When the supplied effective root contains prompts, Codex needs a
        // shadow home even if cwd lives somewhere without prompts.
        assert!(
            needs_shadow_home(Provider::Codex, &without_prompts, false, Some(&with_prompts)),
            "expected true when effective_root has codex prompts"
        );

        // When the supplied effective root lacks prompts, Codex does not need
        // a shadow home (repo_only is false).
        assert!(
            !needs_shadow_home(Provider::Codex, &with_prompts, false, Some(&without_prompts)),
            "expected false when effective_root has no codex prompts"
        );

        // Non-Codex providers are never affected by repo-local prompt detection.
        assert!(
            !needs_shadow_home(Provider::Claude, &with_prompts, false, Some(&with_prompts)),
            "expected false for non-Codex regardless of effective_root"
        );
    }

    #[test]
    fn needs_shadow_home_repo_only_short_circuits_regardless_of_effective_root() {
        let tmp = TempDir::new().unwrap();
        let empty = tmp.path().join("empty");
        fs::create_dir_all(&empty).unwrap();

        // repo_only=true forces shadow home for every provider.
        assert!(needs_shadow_home(Provider::Codex, &empty, true, Some(&empty)));
        assert!(needs_shadow_home(Provider::Claude, &empty, true, Some(&empty)));
        assert!(needs_shadow_home(Provider::OpenCode, &empty, true, None));
    }

    /// Proves that `build_repo_home_env` materializes Codex repo prompts from
    /// the supplied `effective_root` even when `cwd` points to a different
    /// directory (or repo). This is the core Phase 3/4 contract: the caller
    /// threads a pre-resolved launch-child root through the shadow-HOME API
    /// so the redundant `resolve_repo_root(cwd)` sniff walk is skipped.
    #[cfg(unix)]
    #[test]
    #[serial]
    fn build_repo_home_env_uses_supplied_effective_root_not_cwd() {
        let tmp = TempDir::new().unwrap();
        let fake_home = tmp.path().join("home");
        let launch_repo = tmp.path().join("launch-repo");
        let source_repo = tmp.path().join("source-repo");

        fs::create_dir_all(fake_home.join(".codex")).unwrap();
        fs::create_dir_all(launch_repo.join(".claude/commands")).unwrap();
        fs::create_dir_all(source_repo.join(".claude/commands")).unwrap();
        fs::write(launch_repo.join(".claude/commands/launch.md"), "launch").unwrap();
        fs::write(source_repo.join(".claude/commands/source.md"), "source").unwrap();

        let old_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", &fake_home) };

        let (_env, shadow_path, _timings) = build_repo_home_env(
            Provider::Codex,
            &source_repo, // cwd points to source repo (simulates metadata root)
            false,
            false,
            Some(&launch_repo), // effective_root is launch repo (simulates child_cwd)
        )
        .unwrap();

        // Restore HOME so later tests see a clean environment.
        match old_home {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }

        let shadow_path = shadow_path.expect("shadow home path must be returned");
        let prompts_dir = shadow_path.join("prompts");

        // Prompt materialization must follow effective_root (launch repo),
        // not cwd (source repo).
        assert!(
            fs::symlink_metadata(prompts_dir.join("launch.md")).is_ok(),
            "expected launch.md from effective_root in shadow home"
        );
        assert!(
            fs::symlink_metadata(prompts_dir.join("source.md")).is_err(),
            "expected source.md from cwd NOT in shadow home"
        );
    }

    /// Proves that `build_repo_home_env(..., None)` falls back to the legacy
    /// `resolve_repo_root(cwd)` behavior, which walks up to the *git root* — not
    /// the literal `cwd`. The repo-local prompt lives at the repository root,
    /// while `cwd` is a nested subdirectory with no `.claude/commands` of its
    /// own. The prompt only materializes if `resolve_repo_root` ascends to the
    /// root, so this fails if the fallback degrades to `cwd.to_path_buf()`.
    #[cfg(unix)]
    #[test]
    #[serial]
    fn build_repo_home_env_fallback_resolves_repo_root_from_nested_cwd() {
        let tmp = TempDir::new().unwrap();
        let fake_home = tmp.path().join("home");
        let repo = tmp.path().join("repo");
        let nested_cwd = repo.join("crate/src/deep");

        fs::create_dir_all(fake_home.join(".codex")).unwrap();
        fs::create_dir_all(repo.join(".claude/commands")).unwrap();
        fs::create_dir_all(&nested_cwd).unwrap();
        fs::write(repo.join(".claude/commands/review.md"), "review").unwrap();

        if !init_git_repo(&repo) {
            // Skip when git is unavailable: without a detectable repo root the
            // fallback cannot distinguish itself from direct cwd reuse.
            return;
        }

        let old_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", &fake_home) };

        // cwd is a nested subdir with no .claude/commands; only repo-root
        // resolution can locate the root-level prompt.
        let (_env, shadow_path, _timings) =
            build_repo_home_env(Provider::Codex, &nested_cwd, false, false, None).unwrap();

        match old_home {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }

        let shadow_path = shadow_path.expect("shadow home path must be returned");
        let prompts_dir = shadow_path.join("prompts");

        assert!(
            fs::symlink_metadata(prompts_dir.join("review.md")).is_ok(),
            "expected root-level review.md to materialize via resolve_repo_root from nested cwd"
        );
    }
}
