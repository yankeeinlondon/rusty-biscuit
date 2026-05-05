use claudine::provider::Provider;
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::Result;
use std::path::Path;

use std::io::Write;
use super::{PromptDelivery, WrapperProfile, has_flag};

pub(crate) struct CodexWrapper;

impl WrapperProfile for CodexWrapper {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool) {
        // The typed entrypoint catalog records `exec` but not Codex's native
        // `e` alias. Keep this override so callers that already supplied
        // `e` are not rewritten to `exec e ...`.
        if !non_interactive {
            return;
        }
        let entrypoint = "exec";
        let aliases: &[&str] = &["e"];
        if !args
            .first()
            .is_some_and(|first| first == entrypoint || aliases.contains(&first.as_str()))
        {
            args.insert(0, entrypoint.to_string());
        }
    }

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        _interactive: bool,
        _cwd: &Path,
    ) -> Result<crate::commands::wrap::system_prompt::SystemPromptApplication> {
        use crate::commands::wrap::system_prompt::{SystemPromptApplication, SystemPromptArtifact};

        let mut app = SystemPromptApplication::empty();
        match prompt.mode {
            SystemPromptMode::Append => {
                let (tmp_home, _overlay_path) =
                    crate::commands::wrap::system_prompt::create_ephemeral_overlay_home(
                        ".codex",
                        "AGENTS.override.md",
                        &prompt.composed_markdown,
                    )?;
                app.env.push((
                    std::ffi::OsString::from("HOME"),
                    tmp_home.path().as_os_str().to_owned(),
                ));
                app.artifacts.push(SystemPromptArtifact::TempDir(tmp_home));
            }
            SystemPromptMode::Replace => {
                let mut tmp = tempfile::NamedTempFile::new()?;
                tmp.write_all(prompt.composed_markdown.as_bytes())?;
                app.args.push("-c".to_string());
                app.args
                    .push(format!("model_instructions_file={}", tmp.path().display()));
                app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
            }
        }
        Ok(app)
    }

    fn apply_sandbox(&self, args: &mut Vec<String>) -> Option<String> {
        if !has_flag(args, "--sandbox") {
            args.push("--sandbox".to_string());
        }
        None
    }

    fn prompt_delivery(
        &self,
        args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<PromptDelivery> {
        if non_interactive {
            // In non-interactive mode, deliver via stdin to avoid ENAMETOOLONG
            // errors when prompt-file content exceeds OS argument length limits.
            // Codex exec reads from stdin when no positional prompt is provided.
            Ok(PromptDelivery::Stdin(prompt.to_string()))
        } else {
            // Interactive: insert as positional after "exec"
            let insert_at = if args.first().is_some_and(|f| f == "exec" || f == "e") {
                1
            } else {
                0
            };
            Ok(PromptDelivery::InsertArgs {
                index: insert_at,
                args: vec![prompt.to_string()],
            })
        }
    }

    fn allowed_env_keys(&self) -> &'static [&'static str] {
        &["OPENAI_API_KEY", "CODEX_API_KEY"]
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        Ok(vec![
            "codex".to_string(),
            "exec".to_string(),
            "resume".to_string(),
            session_id.to_string(),
        ])
    }

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        // Codex uses `exec --json` for structured output.
        // The `exec` subcommand is expected to already be present.
        if !has_flag(args, "--json") {
            args.push("--json".to_string());
        }
    }

    fn stderr_noise_prefixes(&self) -> &'static [&'static str] {
        &["Reading prompt from stdin..."]
    }

    fn supports_interactive_inline_closure(&self) -> bool {
        true
    }
}
