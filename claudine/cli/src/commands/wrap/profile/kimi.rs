use claudine::provider::Provider;
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::Result;
use std::path::Path;

use super::{PromptDelivery, WrapperProfile};
use std::io::Write;

pub(crate) struct KimiWrapper;

impl WrapperProfile for KimiWrapper {
    fn provider(&self) -> Provider {
        Provider::KimiCode
    }

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        _interactive: bool,
        _cwd: &Path,
        _scoped_tmp: &Path,
    ) -> Result<crate::commands::wrap::system_prompt::SystemPromptApplication> {
        use crate::commands::wrap::system_prompt::{SystemPromptApplication, SystemPromptArtifact};

        let mut app = SystemPromptApplication::empty();
        match prompt.mode {
            SystemPromptMode::Append => {
                app.warnings.push(
                    "Kimi does not support append-mode system prompts; this flag was skipped"
                        .to_string(),
                );
            }
            SystemPromptMode::Replace => {
                let mut prompt_tmp = tempfile::NamedTempFile::new()?;
                prompt_tmp.write_all(prompt.composed_markdown.as_bytes())?;

                let agent_yaml = format!(
                    "extend: default\nsystem_prompt_path: {}\n",
                    prompt_tmp.path().display()
                );
                let mut agent_tmp = tempfile::NamedTempFile::new()?;
                agent_tmp.write_all(agent_yaml.as_bytes())?;

                app.args.push("--agent-file".to_string());
                app.args.push(agent_tmp.path().display().to_string());
                app.artifacts
                    .push(SystemPromptArtifact::TempFile(prompt_tmp));
                app.artifacts
                    .push(SystemPromptArtifact::TempFile(agent_tmp));
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
        // Non-interactive Kimi runs are dispatched through the JSON-RPC
        // wire-mode session orchestrator, which sends the prompt as a
        // typed `prompt` request rather than seeding it on stdin or argv.
        // Interactive runs fall back to the legacy `--prompt` argv form
        // because `--wire` is not active in that mode.
        if non_interactive {
            Ok(PromptDelivery::WireRpc(prompt.to_string()))
        } else {
            Ok(PromptDelivery::AppendArgs(vec![
                "--prompt".to_string(),
                prompt.to_string(),
            ]))
        }
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        Ok(vec![
            "kimi".to_string(),
            "--resume".to_string(),
            session_id.to_string(),
            "--wire".to_string(),
        ])
    }
}
