use claudine::provider::Provider;
use color_eyre::eyre::{Result, bail};

use super::{PromptDelivery, WrapperProfile};

/// Wrapper profile for Kilo Code.
///
/// Kilo Code's CLI is an OpenCode fork, so the wrapper mirrors the OpenCode
/// profile: the non-interactive entrypoint is `kilo run`, structured streaming
/// is `--format json` NDJSON (parsed by the reused OpenCode parser), and the
/// prompt is delivered as a positional message. The one Kilo-specific flag is
/// `--auto`: `kilo run` denies questions and auto-rejects permissions by
/// default, so `--auto` (approve root + tracked task) is required for any
/// autonomous non-interactive progress. `--dangerously-skip-permissions` is a
/// further `--yolo`-only escalation layered on top.
pub(crate) struct KiloWrapper;

impl WrapperProfile for KiloWrapper {
    fn provider(&self) -> Provider {
        Provider::Kilo
    }

    fn apply_yolo(
        &self,
        args: &mut Vec<String>,
        env_overrides: &mut Vec<(String, String)>,
    ) -> Result<super::YoloOutcome> {
        self.apply_yolo_for_mode(args, env_overrides, false)
    }

    fn apply_yolo_for_mode(
        &self,
        args: &mut Vec<String>,
        _env_overrides: &mut Vec<(String, String)>,
        interactive: bool,
    ) -> Result<super::YoloOutcome> {
        // Like OpenCode, the skip-permissions flag only applies under the
        // non-interactive `run` entrypoint; in interactive mode it is ignored.
        if interactive {
            return Ok(super::YoloOutcome::not_applied(
                "--yolo mode is not supported in Kilo <i>interactive</i> sessions and was ignored",
            ));
        }
        if !args.iter().any(|a| a == "--dangerously-skip-permissions") {
            args.push("--dangerously-skip-permissions".to_string());
        }
        Ok(super::YoloOutcome::applied())
    }

    fn prompt_delivery(
        &self,
        _args: &[String],
        prompt: &str,
        _non_interactive: bool,
    ) -> Result<PromptDelivery> {
        // `kilo run` accepts the task as a positional message. Separate it with
        // `--` so the parser stops looking for flags — composed prompts often
        // open with a Markdown bullet (`- ...`) that would otherwise be read as
        // an unknown option.
        const ARG_MAX_HEADROOM: usize = 768 * 1024; // conservative vs OS ARG_MAX
        if prompt.len() > ARG_MAX_HEADROOM {
            bail!(
                "Kilo requires the prompt as a positional argument, but the composed \
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

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        // `--auto` approves root + tracked-task permissions so a non-interactive
        // run makes progress instead of auto-rejecting; `--format json` selects
        // the OpenCode-shaped NDJSON stream the reused parser consumes.
        args.push("--auto".to_string());
        args.push("--format".to_string());
        args.push("json".to_string());
    }

    fn supports_resume(&self) -> bool {
        true
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        // Explicit `kilo run --session <id>` is the scriptable resume selector
        // (session-resumption research: `--format json` emits `sessionID`);
        // `--continue`'s implicit "latest" can pick the wrong session. Resume
        // args replace the base argv wholesale in the harness relaunch.
        Ok(vec![
            "kilo".to_string(),
            "run".to_string(),
            "--session".to_string(),
            session_id.to_string(),
        ])
    }
}
