use claudine::provider::Provider;
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::{Result, bail};
use std::path::Path;

use super::{PromptDelivery, WrapperProfile};
use std::io::Write;

pub(crate) struct OpencodeWrapper;

impl WrapperProfile for OpencodeWrapper {
    fn provider(&self) -> Provider {
        Provider::OpenCode
    }

    fn apply_yolo(
        &self,
        args: &mut Vec<String>,
        env_overrides: &mut Vec<(String, String)>,
    ) -> Result<Option<String>> {
        // Delegate to the mode-aware variant with `interactive = false` so
        // the non-interactive forwarding path is used when callers have not
        // yet migrated to [`apply_yolo_for_mode`].
        self.apply_yolo_for_mode(args, env_overrides, false)
    }

    fn apply_yolo_for_mode(
        &self,
        args: &mut Vec<String>,
        _env_overrides: &mut Vec<(String, String)>,
        interactive: bool,
    ) -> Result<Option<String>> {
        // OpenCode only honors this flag under `run`; the typed catalog can
        // mark it non-interactive-only, but the wrapper still needs to emit a
        // refined warning and avoid mutating interactive TUI argv.
        if interactive {
            return Ok(Some(
                "--yolo mode is not supported in OpenCode <i>interactive</i> sessions and was ignored"
                    .to_string(),
            ));
        }
        if !args.iter().any(|a| a == "--dangerously-skip-permissions") {
            args.push("--dangerously-skip-permissions".to_string());
        }
        Ok(None)
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
                let mut tmp = tempfile::NamedTempFile::new()?;
                tmp.write_all(prompt.composed_markdown.as_bytes())?;

                let config = serde_json::json!({
                    "instructions": [tmp.path().display().to_string()]
                });
                app.env.push((
                    std::ffi::OsString::from("OPENCODE_CONFIG_CONTENT"),
                    std::ffi::OsString::from(config.to_string()),
                ));
                app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
            }
            SystemPromptMode::Replace => {
                let mut tmp = tempfile::NamedTempFile::new()?;
                tmp.write_all(prompt.composed_markdown.as_bytes())?;
                app.args.push("--system".to_string());
                app.args.push(tmp.path().display().to_string());
                app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
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
        // OpenCode reads MODEL from the environment; the env override is
        // sufficient. No --model argv push is needed.
        env_overrides.push(("MODEL".to_string(), model.to_string()));
        None
    }

    fn prompt_delivery(
        &self,
        _args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<PromptDelivery> {
        if non_interactive {
            // OpenCode's `run` entrypoint accepts the task as a positional
            // message. Recent CLI builds reject `--prompt` and have been
            // unreliable when Claudine seeds stdin without a positional task,
            // so keep non-interactive prompt delivery aligned with the native
            // contract and fail early if the prompt is too large for argv.
            const ARG_MAX_HEADROOM: usize = 768 * 1024; // conservative
            if prompt.len() > ARG_MAX_HEADROOM {
                bail!(
                    "OpenCode requires non-interactive prompts as positional arguments, \
                     but the composed prompt is too large ({} KB) for reliable argv delivery.\n\
                    Reduce the prompt size or switch providers for this run.",
                    prompt.len() / 1024
                );
            }
            // Separate the positional prompt with `--` so OpenCode's yargs
            // parser stops looking for flags. Composed prompts commonly
            // start with a bullet (`- ...`) or other `-`-prefixed token,
            // which yargs would otherwise treat as an unrecognized option
            // and respond to by printing `opencode run` help and exiting.
            Ok(PromptDelivery::AppendArgs(vec![
                "--".to_string(),
                prompt.to_string(),
            ]))
        } else {
            // Interactive TUI: use --prompt flag which auto-submits the
            // message (OpenCode PR #4510).  This keeps stdin inherited so
            // the TUI's raw-mode input and mouse tracking work natively.
            //
            // The OS enforces ARG_MAX (~1 MB on macOS) for the combined
            // size of argv + envp passed to execve.  Guard against the
            // rare case of an extremely large composed prompt.
            const ARG_MAX_HEADROOM: usize = 768 * 1024; // conservative
            if prompt.len() > ARG_MAX_HEADROOM {
                bail!(
                    "composed prompt is too large for interactive mode ({} KB); \
                     the OS limits command-line arguments to ~1 MB.\n\
                     Try running without -i to use non-interactive mode, \
                     which delivers the prompt via stdin instead.",
                    prompt.len() / 1024
                );
            }
            Ok(PromptDelivery::AppendArgs(vec![
                "--prompt".to_string(),
                prompt.to_string(),
            ]))
        }
    }

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        // OpenCode uses --format json (cataloged) plus --print-logs and
        // --log-level ERROR for reliable structured streaming. The extra
        // flags are transport-level concerns not modeled in the output-format
        // catalog, so this override is kept.
        args.push("--format".to_string());
        args.push("json".to_string());
        args.push("--print-logs".to_string());
        args.push("--log-level".to_string());
        args.push("ERROR".to_string());
    }

    fn stderr_noise_prefixes(&self) -> &'static [&'static str] {
        opencode_default_tui_noise_prefixes()
    }
}

/// The default-mode TUI formatter lines that OpenCode keeps emitting to
/// stderr even when `--format json` is set. Suppressed when wrapping
/// OpenCode so the NDJSON stream on stdout is the only visible output
/// surface.
pub(crate) fn opencode_default_tui_noise_prefixes() -> &'static [&'static str] {
    &[
        "\u{2731} ",                         // ✱  — bullet used for Glob/Grep/Read status lines
        "$ ",                                // bare shell command echo lines
        "> build ",                          // session banner
        "\u{2588}\u{2588}\u{2588}\u{2588} ", // ████  — subheader marker
        "\u{2699} ", // ⚙  — MCP tool-invocation prefix (see investigations.md §0b)
    ]
}
