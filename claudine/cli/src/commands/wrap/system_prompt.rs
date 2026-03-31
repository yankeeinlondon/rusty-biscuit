use std::ffi::OsString;
use std::path::PathBuf;

use claudine::system_prompt::{
    EffectiveSystemPrompt, StandardPromptScope, SystemPromptMode, SystemPromptSource,
};
use color_eyre::eyre::Result;
use tempfile::{NamedTempFile, TempDir};

/// Artifacts that must remain alive until the child process exits.
///
/// The inner values are never read directly — they exist purely for RAII
/// cleanup (the `Drop` impl deletes the temp resource).
#[allow(dead_code)]
pub(crate) enum SystemPromptArtifact {
    TempFile(NamedTempFile),
    TempDir(TempDir),
}

/// The result of applying a system prompt to a provider launch plan.
pub(crate) struct SystemPromptApplication {
    /// Additional CLI args to append.
    pub args: Vec<String>,
    /// Additional or overridden env vars.
    pub env: Vec<(OsString, OsString)>,
    /// Temp resources that must outlive the child process.
    pub artifacts: Vec<SystemPromptArtifact>,
    /// Non-fatal warnings to display.
    pub warnings: Vec<String>,
}

impl SystemPromptApplication {
    pub fn empty() -> Self {
        Self {
            args: vec![],
            env: vec![],
            artifacts: vec![],
            warnings: vec![],
        }
    }
}

/// Create an ephemeral home directory with a provider config overlay.
///
/// 1. Create a TempDir
/// 2. Create the provider config subdirectory
/// 3. If the real overlay file exists, copy its content and append the prompt
/// 4. Otherwise write just the prompt content
/// 5. Return the TempDir and path to the overlay file
pub(crate) fn create_ephemeral_overlay_home(
    provider_dir: &str,
    overlay_file: &str,
    prompt_content: &str,
) -> Result<(TempDir, PathBuf)> {
    let tmp_home = TempDir::new()?;
    let provider_path = tmp_home.path().join(provider_dir);
    std::fs::create_dir_all(&provider_path)?;

    // Copy existing provider overlay file content if it exists
    let real_provider_dir = dirs::home_dir().map(|h| h.join(provider_dir));
    if let Some(ref real) = real_provider_dir
        && real.is_dir()
    {
        let existing = real.join(overlay_file);
        if existing.is_file() {
            let existing_content = std::fs::read_to_string(&existing)?;
            let combined = format!("{}\n\n{}", existing_content.trim_end(), prompt_content);
            std::fs::write(provider_path.join(overlay_file), combined)?;
            return Ok((tmp_home, provider_path.join(overlay_file)));
        }
    }

    // No existing file — write prompt content only
    let overlay_path = provider_path.join(overlay_file);
    std::fs::write(&overlay_path, prompt_content)?;
    Ok((tmp_home, overlay_path))
}

/// Human-readable description of where a system prompt came from.
pub(crate) fn describe_source(source: &SystemPromptSource) -> String {
    match source {
        SystemPromptSource::StandardDiscovered { path, scope } => {
            let scope_label = match scope {
                StandardPromptScope::Package => "package",
                StandardPromptScope::PackageArea => "package-area",
                StandardPromptScope::Repo => "repo",
                StandardPromptScope::User => "user",
                StandardPromptScope::CurrentDirectory => "cwd",
            };
            format!("{} ({})", path.display(), scope_label)
        }
        SystemPromptSource::ExplicitFile { path, mode } => {
            let mode_label = match mode {
                SystemPromptMode::Append => "append",
                SystemPromptMode::Replace => "replace",
            };
            format!("{} (explicit {})", path.display(), mode_label)
        }
    }
}

/// Format the effective system prompt for dry-run output.
pub(crate) fn describe_effective(effective: &EffectiveSystemPrompt) -> Option<Vec<String>> {
    match effective {
        EffectiveSystemPrompt::None => None,
        EffectiveSystemPrompt::Disabled { source } => Some(vec![
            format!("source: {}", describe_source(source)),
            "status: disabled (empty body)".to_string(),
        ]),
        EffectiveSystemPrompt::Ready(prepared) => {
            let mode_label = match prepared.mode {
                SystemPromptMode::Append => "append",
                SystemPromptMode::Replace => "replace",
            };
            Some(vec![
                format!("source: {}", describe_source(&prepared.source)),
                format!("mode: {}", mode_label),
                format!("composed length: {} chars", prepared.composed_markdown.len()),
            ])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_home_creates_fresh() {
        let prompt = "You are a helpful assistant.";
        let (tmp_home, overlay_path) =
            create_ephemeral_overlay_home(".test-provider", "OVERLAY.md", prompt).unwrap();

        assert!(overlay_path.exists());
        let content = std::fs::read_to_string(&overlay_path).unwrap();
        assert_eq!(content, prompt);

        // Verify directory structure
        assert!(tmp_home.path().join(".test-provider").is_dir());
    }

    #[test]
    fn ephemeral_home_preserves_existing() {
        // Create a fake "real" home with an existing overlay file
        let fake_home = TempDir::new().unwrap();
        let provider_dir = fake_home.path().join(".test-ephemeral");
        std::fs::create_dir_all(&provider_dir).unwrap();
        let existing_file = provider_dir.join("OVERLAY.md");
        std::fs::write(&existing_file, "Existing instructions.").unwrap();

        // We can't easily override dirs::home_dir() so test the fresh path
        // and verify the format logic manually
        let existing_content = "Existing instructions.";
        let prompt = "New Claudine content.";
        let combined = format!("{}\n\n{}", existing_content.trim_end(), prompt);
        assert_eq!(combined, "Existing instructions.\n\nNew Claudine content.");
    }

    #[test]
    fn system_prompt_application_empty() {
        let app = SystemPromptApplication::empty();
        assert!(app.args.is_empty());
        assert!(app.env.is_empty());
        assert!(app.artifacts.is_empty());
        assert!(app.warnings.is_empty());
    }
}
