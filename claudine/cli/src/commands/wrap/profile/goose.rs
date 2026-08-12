use claudine::provider::Provider;
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::Result;
use std::path::Path;

use super::{PromptDelivery, WrapperProfile};

pub(crate) struct GooseWrapper;

impl WrapperProfile for GooseWrapper {
    fn provider(&self) -> Provider {
        Provider::Goose
    }

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        _interactive: bool,
        _cwd: &Path,
        _scoped_tmp: &Path,
    ) -> Result<crate::commands::wrap::system_prompt::SystemPromptApplication> {
        use crate::commands::wrap::system_prompt::SystemPromptApplication;

        let mut app = SystemPromptApplication::empty();
        match prompt.mode {
            SystemPromptMode::Append => {
                app.args.push("--system".to_string());
                app.args.push(prompt.composed_markdown.clone());
            }
            SystemPromptMode::Replace => {
                app.warnings.push(
                    "Goose does not support replace-mode system prompts; this flag was skipped"
                        .to_string(),
                );
            }
        }
        Ok(app)
    }

    fn apply_model(
        &self,
        _args: &mut Vec<String>,
        env_overrides: &mut Vec<(String, String)>,
        model: &str,
    ) -> Option<String> {
        // Goose selects models via the GOOSE_MODEL env var, not a CLI flag.
        // The typed catalog has no model-delivery-mechanism field, so this
        // override routes the provider's own selection through env. The
        // generic `MODEL` env var is Claudine's wrapper contract (templates,
        // hooks, reporting) and is exported alongside it.
        env_overrides.push(("GOOSE_MODEL".to_string(), model.to_string()));
        env_overrides.push(("MODEL".to_string(), model.to_string()));
        None
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        // Explicit `--session-id` is mandatory: plain `goose run --resume`
        // selects globally (not repo-scoped) and name-based selection can
        // drop the session's saved provider/model metadata
        // (session-resumption research, 2026-07-03). The `run` entrypoint is
        // included so `prompt_delivery` inserts `-t <prompt>` after it.
        Ok(vec![
            "goose".to_string(),
            "run".to_string(),
            "--resume".to_string(),
            "--session-id".to_string(),
            session_id.to_string(),
        ])
    }

    fn prompt_delivery(
        &self,
        args: &[String],
        prompt: &str,
        _non_interactive: bool,
    ) -> Result<PromptDelivery> {
        // Goose: insert -t <prompt> after "run" subcommand
        if let Some(pos) = args.iter().position(|a| a == "run") {
            Ok(PromptDelivery::InsertArgs {
                index: pos + 1,
                args: vec!["-t".to_string(), prompt.to_string()],
            })
        } else {
            Ok(PromptDelivery::AppendArgs(vec![
                "run".to_string(),
                "-t".to_string(),
                prompt.to_string(),
            ]))
        }
    }
}
