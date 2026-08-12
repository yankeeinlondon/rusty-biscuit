use std::path::Path;

use claudine::provider::Provider;
use claudine::system_prompt::PreparedSystemPrompt;
use color_eyre::eyre::Result;

use super::{PromptDelivery, WrapperProfile};

/// Wrapper profile for Pi.
///
/// Pi is a bespoke (non-fork) provider. Almost everything is catalog-driven: the
/// non-interactive entrypoint is `-p` (no subcommand), the structured stream is
/// `--mode json` NDJSON (the headless determinism flags ride as its catalog
/// companion flags), the system prompt is delivered via `--append-system-prompt`
/// / `--system-prompt` file flags, and there is no YOLO mode (Pi is permissive
/// by default). Model selection (`--model`) is catalog-driven via the default
/// `apply_model`. Only prompt delivery, system-prompt delivery, and the resume
/// selector need Pi-specific handling.
pub(crate) struct PiWrapper;

impl WrapperProfile for PiWrapper {
    fn provider(&self) -> Provider {
        Provider::Pi
    }

    fn prompt_delivery(
        &self,
        _args: &[String],
        prompt: &str,
        _non_interactive: bool,
    ) -> Result<PromptDelivery> {
        // Pi reads the prompt from STDIN in `-p` mode (verified against pi
        // 0.80.3: `echo PROMPT | pi -p --mode json` streams the run). Stdin is
        // the only safe channel: Pi's bespoke arg parser rejects both a bare
        // `--` separator (`Error: Unknown option: --`) and any positional
        // message beginning with `-` (`Error: Unknown option: - ...`), and
        // composed prompts routinely open with a Markdown bullet.
        Ok(PromptDelivery::Stdin(prompt.to_string()))
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

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        // `pi --session-id <id>` is the unattended-safe resume selector, verified
        // against pi 0.80.3: it takes the EXACT project session id (JSON mode
        // emits the full UUID in its header), appends to the same session file
        // (true resume — no fork), and never prompts. `--session <id>` was
        // rejected: its "partial UUID" match can trigger a cross-project fork
        // prompt (a hang risk unattended); `--session-id` also degrades safely
        // ("creating it if missing") instead of erroring. Resume args replace
        // the base argv wholesale; the relaunch layers the structured-stream
        // flags and the follow-up prompt on stdin.
        Ok(vec![
            "pi".to_string(),
            "--session-id".to_string(),
            session_id.to_string(),
        ])
    }
}
