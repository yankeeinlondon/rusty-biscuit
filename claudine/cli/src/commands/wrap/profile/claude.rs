use claudine::invocation_context::EnvBaseline;
use claudine::provider::Provider;
use claudine::provider_overlay::OverlayPlan;
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::Result;
use std::path::Path;

use super::{PromptDelivery, WrapperProfile, prompt_delivery_stdin_or_append};

pub(crate) struct ClaudeWrapper;

/// Claude Code's selector for its credential store, independent of
/// `CLAUDE_CONFIG_DIR`.
const SECURE_STORAGE_SELECTOR: &str = "CLAUDE_SECURESTORAGE_CONFIG_DIR";

impl WrapperProfile for ClaudeWrapper {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn overlay_strategy(&self, plan: &mut OverlayPlan, env: &EnvBaseline) -> Result<()> {
        if plan.provider_visible_root().is_none() {
            return Ok(());
        }
        // Setting `CLAUDE_CONFIG_DIR` also renames Claude's secure-storage entry
        // (the macOS keychain service gains a hash of the directory) and moves
        // its credentials file, so an overlay alone would look signed out.
        // `CLAUDE_SECURESTORAGE_CONFIG_DIR` keeps both at the pre-overlay
        // location; an empty value selects the default, unsuffixed entry.
        // Observed in Claude Code 2.1.273; see the Phase 6 implementation log.
        let secure_storage = env
            .get(SECURE_STORAGE_SELECTOR)
            .or_else(|| env.get("CLAUDE_CONFIG_DIR").filter(|value| !value.is_empty()))
            .map(ToOwned::to_owned)
            .unwrap_or_default();
        plan.pin_external_state(SECURE_STORAGE_SELECTOR, secure_storage);
        Ok(())
    }

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        interactive: bool,
        _cwd: &Path,
        _scoped_tmp: &Path,
    ) -> Result<crate::commands::wrap::system_prompt::SystemPromptApplication> {
        use crate::commands::wrap::system_prompt::{SystemPromptApplication, SystemPromptArtifact};
        use std::io::Write as _;

        let mut app = SystemPromptApplication::empty();
        match prompt.mode {
            SystemPromptMode::Append => {
                if interactive {
                    app.args.push("--append-system-prompt".to_string());
                    app.args.push(prompt.composed_markdown.clone());
                } else {
                    let mut tmp = tempfile::NamedTempFile::new()?;
                    tmp.write_all(prompt.composed_markdown.as_bytes())?;
                    app.args.push("--append-system-prompt-file".to_string());
                    app.args.push(tmp.path().display().to_string());
                    app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
                }
            }
            SystemPromptMode::Replace => {
                if interactive {
                    app.args.push("--system-prompt".to_string());
                    app.args.push(prompt.composed_markdown.clone());
                } else {
                    let mut tmp = tempfile::NamedTempFile::new()?;
                    tmp.write_all(prompt.composed_markdown.as_bytes())?;
                    app.args.push("--system-prompt-file".to_string());
                    app.args.push(tmp.path().display().to_string());
                    app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
                }
            }
        }
        Ok(app)
    }

    fn prompt_delivery(
        &self,
        _args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<PromptDelivery> {
        Ok(prompt_delivery_stdin_or_append(
            prompt,
            non_interactive,
            &[],
        ))
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        Ok(vec![
            "claude".to_string(),
            "-r".to_string(),
            session_id.to_string(),
            "--print".to_string(),
        ])
    }
}
