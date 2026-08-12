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
    ) -> Result<super::YoloOutcome> {
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
    ) -> Result<super::YoloOutcome> {
        // OpenCode only honors `--dangerously-skip-permissions` under
        // `opencode run` (the non-interactive entrypoint). In interactive
        // TUI mode the flag is silently rejected, so emit a refined
        // warning and report `applied = false` so the badge / reporter
        // surface reflects the disabled state.
        if interactive {
            return Ok(super::YoloOutcome::not_applied(
                "--yolo mode is not supported in OpenCode <i>interactive</i> sessions and was ignored",
            ));
        }
        if !args.iter().any(|a| a == "--dangerously-skip-permissions") {
            args.push("--dangerously-skip-permissions".to_string());
        }
        Ok(super::YoloOutcome::applied())
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
        args: &mut Vec<String>,
        env_overrides: &mut Vec<(String, String)>,
        model: &str,
    ) -> Option<String> {
        // This path runs only for *interactive* OpenCode — the non-interactive
        // pipeline resolves the model via `apply_opencode_model_resolution`,
        // which short-circuits when interactive. The OpenCode TUI honors the
        // `--model` flag (never a bare `MODEL` env var), so the argv push is
        // what actually selects the model; the env override is kept for
        // Claudine's own templating/reporting surfaces.
        let has_model_flag = args.iter().any(|a| a == "--model" || a == "-m");
        if !has_model_flag {
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
            // A prompt beginning with `-` (composed prompts commonly open
            // with a Markdown bullet) makes OpenCode's yargs parser treat the
            // value as an option: it prints its top-level help and exits 1.
            // The attached `--prompt=<value>` form binds the value to the flag
            // unambiguously, mirroring the non-interactive `--` guard above.
            Ok(if prompt.starts_with('-') {
                PromptDelivery::AppendArgs(vec![format!("--prompt={prompt}")])
            } else {
                PromptDelivery::AppendArgs(vec!["--prompt".to_string(), prompt.to_string()])
            })
        }
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        // Explicit `--session <ses_...>` is the automation-safe selector;
        // `--continue` is a human convenience whose implicit "latest" can
        // pick the wrong session (session-resumption research, 2026-07-03).
        // The `run` entrypoint is included because resume args replace the
        // base argv wholesale in the harness relaunch.
        Ok(vec![
            "opencode".to_string(),
            "run".to_string(),
            "--session".to_string(),
            session_id.to_string(),
        ])
    }

    // Kept as an override (ratified 2026-07-04): these flags bundle the
    // stderr log-promotion contract (--print-logs --log-level INFO) with
    // the stream selector — behavior, not catalog data.
    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        // OpenCode uses --format json (cataloged) plus --print-logs and
        // --log-level INFO for reliable structured streaming. INFO provides
        // enough signal (sessions, LLM calls, step loops, HTTP responses)
        // for the stderr bridge to detect progress during NDJSON silence
        // windows, while the aggressive `service=bus` filter in the bridge
        // keeps noise out of the semantic event stream.
        args.push("--format".to_string());
        args.push("json".to_string());
        args.push("--print-logs".to_string());
        args.push("--log-level".to_string());
        args.push("INFO".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Interactive OpenCode reaches model selection only through `apply_model`
    // (`apply_opencode_model_resolution` returns early when interactive). The
    // OpenCode TUI honors `--model` / `OPENCODE_MODEL`, never a bare `MODEL`
    // env var — so `apply_model` must push the `--model` argv flag or the
    // user's `-i --model X` selection is silently dropped (regression in
    // 9e38c794c).
    #[test]
    fn apply_model_pushes_model_flag_for_interactive_opencode() {
        let wrapper = OpencodeWrapper;
        let mut args: Vec<String> = Vec::new();
        let mut env_overrides: Vec<(String, String)> = Vec::new();

        let warn = wrapper.apply_model(&mut args, &mut env_overrides, "kimi-for-coding/k2p6");

        assert!(warn.is_none());
        let model_idx = args
            .iter()
            .position(|a| a == "--model")
            .expect("--model flag must be pushed for interactive OpenCode");
        assert_eq!(args.get(model_idx + 1).map(String::as_str), Some("kimi-for-coding/k2p6"));
    }
}
