//! The production [`PushServices`]: the real repository, processes, and clock.

use std::path::Path;

use anyhow::{Context, Result};

use super::PushServices;

/// Services that read the host `ci-push` runs on.
#[derive(Debug, Default)]
pub struct HostServices;

impl PushServices for HostServices {
    fn object_hash(&mut self, cwd: &Path) -> Result<gix::hash::Kind> {
        let repository = sniff::filesystem::git::GitRepo::discover(cwd)
            .with_context(|| format!("cannot open the Git repository at {}", cwd.display()))?
            .with_context(|| format!("{} is not inside a Git repository", cwd.display()))?;
        let root = repository.repo_root();
        let opened = gix::open(root)
            .with_context(|| format!("cannot open the Git repository at {}", root.display()))?;
        Ok(opened.object_hash())
    }
}
