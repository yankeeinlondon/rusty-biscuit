use claudine::provider::Provider;
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::Result;
use std::path::Path;

use super::{PromptDelivery, WrapperProfile, push_stream_json_flags, prompt_delivery_stdin_or_append};

pub(crate) struct ClaudeWrapper;

impl WrapperProfile for ClaudeWrapper {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        interactive: bool,
        _cwd: &Path,
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

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        // Claude needs --print --verbose alongside stream-json for reliable
        // structured output. The typed catalog does not model "extra flags
        // required for structured streaming", so this override is kept.
        push_stream_json_flags(args, &["--print", "--verbose"]);
    }
}
