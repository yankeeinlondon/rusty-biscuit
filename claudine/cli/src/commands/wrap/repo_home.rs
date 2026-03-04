use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::{bail, Context, Result};

use claudine::events::Provider;

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

    pub fn sync_shadow_home(&self) -> Result<()> {
        let original_home = self.original_home();
        if !original_home.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(original_home)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let source = entry.path();
            let dest = self.shadow_home.join(&file_name);

            if self.should_exclude(&file_name) {
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

    fn should_exclude(&self, file_name: &std::ffi::OsStr) -> bool {
        let excluded = self.excluded_dirs();
        let file_name_str = file_name.to_string_lossy();
        excluded
            .iter()
            .any(|&excluded| file_name_str == excluded || file_name_str == format!(".{}", excluded))
    }

    fn excluded_dirs(&self) -> Vec<&'static str> {
        match self.agent_offset.as_str() {
            ".claude" => vec!["skills", "commands", "agents", "hooks"],
            ".codex" => vec!["skills", "agents"],
            ".gemini" => vec!["skills", "agents"],
            ".goose" => vec!["skills", "agents"],
            ".kimi" => vec!["skills", "agents"],
            ".opencode" => vec!["skills"],
            ".qwen" => vec!["skills", "commands"],
            ".roo" => vec!["skills", "commands", "hooks"],
            _ => vec!["skills", "commands", "agents", "hooks"],
        }
    }

    pub fn is_fresh(&self) -> bool {
        if !self.shadow_home.exists() {
            return false;
        }

        let Ok(shadow_meta) = fs::metadata(&self.shadow_home) else {
            return false;
        };

        let Ok(shadow_modified) = shadow_meta.modified() else {
            return false;
        };

        let original_home = self.original_home();
        if !original_home.exists() {
            return true;
        }

        if let Ok(original_meta) = fs::metadata(&original_home)
            && let Ok(original_modified) = original_meta.modified()
        {
            return shadow_modified >= original_modified;
        }

        true
    }
}

pub fn build_repo_home_env(
    provider: Provider,
) -> Result<(HashMap<OsString, OsString>, Option<PathBuf>)> {
    let manager = RepoHomeManager::new(provider);
    let shadow_home = manager.ensure_shadow_home()?;

    if !manager.is_fresh() {
        manager.sync_shadow_home()?;
    }

    let mut env = HashMap::new();
    // Set HOME to the shadow home root (~/.claudine), not the agent subdirectory
    // This makes the agent look in ~/.claude/ for its config, which is a symlink
    let shadow_home_root = shadow_home
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("~")).join(".claudine"));
    env.insert(OsString::from("HOME"), OsString::from(shadow_home_root));

    Ok((env, Some(shadow_home)))
}
