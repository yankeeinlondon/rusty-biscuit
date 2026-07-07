use claudine::provider::Provider;
use color_eyre::eyre::{Result, bail};

use super::{PromptDelivery, WrapperProfile};

/// Wrapper profile for Pi.
///
/// Pi is a bespoke (non-fork) provider. Almost everything is catalog-driven: the
/// non-interactive entrypoint has no subcommand (`pi [flags] "PROMPT"`), the
/// structured stream is `--mode json` NDJSON (the headless determinism flags
/// ride as its catalog companion flags), the system prompt is delivered via
/// `--append-system-prompt` / `--system-prompt` file flags, and there is no YOLO
/// mode (Pi is permissive by default). Only prompt placement, model-flag
/// de-duplication, and the resume selector need Pi-specific handling.
pub(crate) struct PiWrapper;

impl WrapperProfile for PiWrapper {
    fn provider(&self) -> Provider {
        Provider::Pi
    }

    fn apply_model(
        &self,
        args: &mut Vec<String>,
        env_overrides: &mut Vec<(String, String)>,
        model: &str,
    ) -> Option<String> {
        // Pi selects the model with `--model` (catalog model_cli_flag). Guard
        // against a passthrough `--model` so we never emit it twice. Keep the
        // generic MODEL env override for Claudine's templating/reporting.
        if !args.iter().any(|a| a == "--model") {
            args.push("--model".to_string());
            args.push(model.to_string());
        }
        env_overrides.push(("MODEL".to_string(), model.to_string()));
        None
    }

    fn prompt_delivery(
        &self,
        _args: &[String],
        prompt: &str,
        _non_interactive: bool,
    ) -> Result<PromptDelivery> {
        // Pi takes the task as a positional argument (`pi --mode json "PROMPT"`).
        // Separate it with `--` so option parsing stops first — composed prompts
        // routinely open with a Markdown bullet (`- ...`) that would otherwise be
        // read as an unknown flag.
        const ARG_MAX_HEADROOM: usize = 768 * 1024; // conservative vs OS ARG_MAX
        if prompt.len() > ARG_MAX_HEADROOM {
            bail!(
                "Pi requires the prompt as a positional argument, but the composed \
                 prompt is too large ({} KB) for reliable argv delivery.\n\
                 Reduce the prompt size or switch providers for this run.",
                prompt.len() / 1024
            );
        }
        Ok(PromptDelivery::AppendArgs(vec![
            "--".to_string(),
            prompt.to_string(),
        ]))
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        // `pi --session <id>` is the scriptable resume selector (session-
        // resumption research; JSON mode emits the full session UUID in its
        // header). The full id is passed to avoid the cross-project fork prompt a
        // partial UUID can trigger. Resume args replace the base argv wholesale;
        // the relaunch layers the structured-stream flags and follow-up prompt.
        Ok(vec![
            "pi".to_string(),
            "--session".to_string(),
            session_id.to_string(),
        ])
    }
}
