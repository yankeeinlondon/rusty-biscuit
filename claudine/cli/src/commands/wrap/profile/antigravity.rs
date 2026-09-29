use claudine::provider::Provider;
use color_eyre::eyre::Result;

use super::{PromptDelivery, WrapperProfile, has_flag};

/// Wrapper profile for Antigravity (the headless `agy` CLI).
///
/// agy is bespoke and non-streaming: its structured non-interactive output is
/// `agy --print <prompt> --output-format json`, a SINGLE buffered JSON envelope
/// (parsed by `AntigravitySemanticStreamParser`). agy ignores stdin, so the
/// prompt is a flag VALUE: `--print <prompt>` appended LAST in non-interactive
/// mode (adjacent to its flag for Go's flag parser), `--prompt-interactive
/// <prompt>` in an interactive session.
/// The structured selector is `--output-format json` (overridden here because
/// the catalog records it as `OutputFormat::Json`, not the `Stream` record the
/// default `apply_structured_stream` keys on). Resume is `agy --conversation
/// <id>` (the full conversation UUID from the envelope). YOLO
/// (`--dangerously-skip-permissions`) and model selection (`--model`) are
/// catalog-driven; system-prompt override is genuinely unsupported (agy has no
/// flag), so the default "unsupported" stub is left in place.
pub(crate) struct AntigravityWrapper;

impl WrapperProfile for AntigravityWrapper {
    fn provider(&self) -> Provider {
        Provider::Antigravity
    }

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        // agy's structured format is `--output-format json` (a single buffered
        // envelope). The catalog records it as `OutputFormat::Json`, so the
        // default `apply_structured_stream` (which keys on the `Stream` record)
        // is a no-op — push the selector here instead.
        if !has_flag(args, "--output-format")
            && !args.iter().any(|a| a.starts_with("--output-format="))
        {
            args.push("--output-format".to_string());
            args.push("json".to_string());
        }
    }

    fn apply_sandbox(&self, args: &mut Vec<String>) -> Option<String> {
        // agy runs sandboxed via `--sandbox` (terminal restrictions enabled).
        if !has_flag(args, "--sandbox") {
            args.push("--sandbox".to_string());
        }
        None
    }

    fn prompt_delivery(
        &self,
        _args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<PromptDelivery> {
        if !non_interactive {
            // `--prompt-interactive` "runs an initial prompt interactively and
            // continues the session" (agy 1.1.0); `--print` would run headless
            // and exit. Go's flag parser takes the next argv item as the value
            // whatever its first character, but the attached form keeps a
            // `-`-prefixed prompt unambiguous.
            return Ok(if prompt.starts_with('-') {
                PromptDelivery::AppendArgs(vec![format!("--prompt-interactive={prompt}")])
            } else {
                PromptDelivery::AppendArgs(vec![
                    "--prompt-interactive".to_string(),
                    prompt.to_string(),
                ])
            });
        }
        // The prompt is the value of `--print` (aliases `-p`/`--prompt`); agy
        // ignores stdin. Appending `["--print", <prompt>]` LAST keeps the flag
        // and its value adjacent and last on argv, so the other flags
        // (`--output-format json`, `--model`, …) sit ahead of it and Go's flag
        // parser reads the prompt as `--print`'s value. Verified against agy
        // 1.1.0: `agy --print "…" --output-format json` runs headless.
        Ok(PromptDelivery::AppendArgs(vec![
            "--print".to_string(),
            prompt.to_string(),
        ]))
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        // `agy --conversation <id>` is the unattended-safe resume selector
        // (research resume/antigravity.md): it takes the exact conversation
        // UUID (emitted as `conversation_id` in the `--output-format json`
        // envelope), appends to the same conversation, and never prompts. The
        // `-c`/`--continue` alternative selects the latest conversation and is
        // unsafe for parallel wrappers. Resume args replace the base argv; the
        // relaunch layers `--output-format json` and the follow-up `--print`
        // prompt back on.
        Ok(vec![
            "agy".to_string(),
            "--conversation".to_string(),
            session_id.to_string(),
        ])
    }
}
