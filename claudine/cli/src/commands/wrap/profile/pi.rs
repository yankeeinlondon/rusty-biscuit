use std::path::Path;
use std::sync::Arc;

use claudine::provider::Provider;
use claudine::system_prompt::PreparedSystemPrompt;
use color_eyre::eyre::{Result, bail};

use super::{PromptDelivery, WrapperProfile, has_flag, option_value};
use crate::commands::wrap::exec::control::{FallbackLaunch, StdioControl};
use crate::commands::wrap::exec::pi_rpc::PiRpcSession;

/// Research execution-interface ids (`docs/research/non-interactive-sessions/pi.md`)
/// the wrapper implements, and the `--mode` value each launches.
const RPC_INTERFACE: &str = "rpc";
const JSON_INTERFACE: &str = "json";
const MODE_FLAG: &str = "--mode";

/// What a Pi run gives up when it falls back to the JSON stream.
const JSON_FALLBACK_LOSSES: &str = "Running Pi's one-way JSON stream instead: this run cannot be steered or \
     queried, and Pi tells extensions no UI is available, so their dialogs take their own defaults.";

/// Wrapper profile for Pi.
///
/// Pi is a bespoke (non-fork) provider. Almost everything is catalog-driven:
/// the system prompt is delivered via `--append-system-prompt` /
/// `--system-prompt`, there is no YOLO mode (Pi is permissive by default), and
/// model selection (`--model`) is the default `apply_model`.
///
/// A structured (non-interactive) run uses the interface the Pi research
/// selects: the managed RPC control interface (`--mode rpc`), whose
/// [`PiRpcSession`] owns stdin and submits the prompt as an RPC `prompt`, with
/// the JSON stream (`-p --mode json`, prompt on stdin) as its pre-submission
/// fallback. Extensions, skills, prompt templates, and context files stay
/// enabled in both. Interactive sessions need Pi 0.84.3 or later, the first
/// release that accepts `--` before a positional message.
pub(crate) struct PiWrapper;

/// Whether the research selects the managed RPC interface for structured runs.
fn rpc_selected() -> bool {
    claudine::steering::execution_selection(Provider::Pi).is_some_and(|selection| selection.preferred == RPC_INTERFACE)
}

/// Points the last `--mode` at `mode`.
fn set_mode(args: &mut [String], mode: &str) {
    if let Some(index) = args.iter().rposition(|arg| arg == MODE_FLAG)
        && let Some(value) = args.get_mut(index + 1)
    {
        *value = mode.to_string();
    }
}

/// The JSON-stream launch equivalent to the RPC launch `args`, when the
/// research names JSON as the fallback.
fn json_fallback(args: &[String]) -> Option<FallbackLaunch> {
    let selection = claudine::steering::execution_selection(Provider::Pi)?;
    if selection.fallback != Some(JSON_INTERFACE) {
        return None;
    }
    let mut args = args.to_vec();
    set_mode(&mut args, JSON_INTERFACE);
    if !has_flag(&args, "-p") && !has_flag(&args, "--print") {
        let at = args.iter().position(|arg| arg == MODE_FLAG).unwrap_or(args.len());
        args.insert(at, "-p".to_string());
    }
    Some(FallbackLaunch { args, warning: JSON_FALLBACK_LOSSES.to_string() })
}

impl WrapperProfile for PiWrapper {
    fn provider(&self) -> Provider {
        Provider::Pi
    }

    fn prompt_delivery(
        &self,
        _args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<PromptDelivery> {
        if non_interactive {
            // `-p` mode reads the prompt from stdin (`echo PROMPT | pi -p
            // --mode json`), which keeps large prompts off argv. Under RPC
            // the control session submits this same seed as a `prompt`.
            return Ok(PromptDelivery::Stdin(prompt.to_string()));
        }
        // Piped stdin always switches Pi to print mode, so an interactive first
        // turn must be a positional message: `pi -- <prompt>`. `--` ends
        // options from Pi 0.84.3 (earlier releases reject it), so a prompt
        // opening with a Markdown bullet is not read as an option.
        const ARG_MAX_HEADROOM: usize = 768 * 1024; // conservative vs OS ARG_MAX
        if prompt.len() > ARG_MAX_HEADROOM {
            bail!(
                "Pi needs an interactive first message on the command line, but the \
                 composed prompt is too large ({} KB) for reliable argv delivery.\n\
                 Reduce the prompt size or run without -i, which delivers it on stdin.",
                prompt.len() / 1024
            );
        }
        // Even after `--`, Pi reads a token starting with `@` as a file
        // reference; a leading space keeps the prompt a message.
        let message = if prompt.starts_with('@') {
            format!(" {prompt}")
        } else {
            prompt.to_string()
        };
        Ok(PromptDelivery::AppendArgs(vec!["--".to_string(), message]))
    }

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        interactive: bool,
        _cwd: &Path,
        scoped_tmp: &Path,
    ) -> Result<crate::commands::wrap::system_prompt::SystemPromptApplication> {
        // Pi supports `--append-system-prompt` / `--system-prompt` (inline_flag
        // per the catalog spec). The shared spec-driven helper emits the flag
        // with the composed prompt inline; the default trait impl only reports
        // "unsupported", so this delegation is required for delivery.
        crate::commands::wrap::system_prompt::apply_system_prompt_via_spec(
            self.system_prompt_spec(),
            prompt.mode,
            interactive,
            &prompt.composed_markdown,
            None,
            scoped_tmp,
        )
    }

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        super::apply::apply_structured_stream(Provider::Pi, args);
        if rpc_selected() {
            // RPC reads commands, not a prompt, from stdin; `-p` would make
            // Pi read stdin as the prompt instead.
            args.retain(|arg| arg != "-p" && arg != "--print");
            set_mode(args, RPC_INTERFACE);
        }
    }

    fn stdio_control(&self, args: &[String], _cwd: &Path) -> Option<Arc<dyn StdioControl>> {
        (option_value(args, MODE_FLAG).as_deref() == Some(RPC_INTERFACE))
            .then(|| Arc::new(PiRpcSession::new(json_fallback(args))) as Arc<dyn StdioControl>)
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        // `pi --session-id <id>` is the unattended-safe resume selector, verified
        // against pi 0.80.3: it takes the EXACT project session id (JSON mode
        // emits the full UUID in its header), appends to the same session file
        // (true resume — no fork), and never prompts. `--session <id>` was
        // rejected: its "partial UUID" match can trigger a cross-project fork
        // prompt (a hang risk unattended); `--session-id` also degrades safely
        // ("creating it if missing") instead of erroring. Resume args replace
        // the base argv wholesale; `assemble_resume_args` carries
        // the structured launch's `--mode` and trust flag over, and the
        // follow-up prompt goes wherever that mode takes it.
        Ok(vec![
            "pi".to_string(),
            "--session-id".to_string(),
            session_id.to_string(),
        ])
    }
}
