use claudine::provider::Provider;
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::{Result, bail};
use std::path::Path;

use super::{ConfiguredModel, PromptDelivery, WrapperProfile};
use std::io::Write;
use std::path::PathBuf;

pub(crate) struct OpencodeWrapper;

/// OpenCode's global config file names in its own precedence order: the
/// current `opencode.jsonc` / `opencode.json`, then the legacy `config.json`
/// older installs still carry. A later file is consulted only when an earlier
/// one names no model.
const GLOBAL_CONFIG_FILES: [&str; 3] = ["opencode.jsonc", "opencode.json", "config.json"];

/// `$XDG_CONFIG_HOME/opencode`, else `~/.config/opencode` — the directory
/// OpenCode reads its global config from.
fn opencode_config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
        .map(|config| config.join("opencode"))
}

/// The `model` entry of the first global config file under `dir` that names
/// one. Files are JSONC (comments, trailing commas), so they are parsed as
/// JSON5; an unreadable or malformed file is skipped, not an error.
pub(crate) fn configured_default_model_in(dir: &std::path::Path) -> Option<ConfiguredModel> {
    GLOBAL_CONFIG_FILES.iter().find_map(|name| {
        let path = dir.join(name);
        let text = std::fs::read_to_string(&path).ok()?;
        let json = biscuit_file::Json5::from_str(&text).ok()?;
        let model = json.value().get("model")?.as_str()?.trim();
        (!model.is_empty()).then(|| ConfiguredModel {
            model: model.to_string(),
            path,
        })
    })
}

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

    fn configured_default_model(&self) -> Option<ConfiguredModel> {
        configured_default_model_in(&opencode_config_dir()?)
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

    // The OpenCode TUI honors `--model` / `OPENCODE_MODEL`, never a bare
    // `MODEL` env var — so the catalog-driven `apply_model` must push the
    // `--model` argv flag or the user's `-i --model X` selection is silently
    // dropped (regression in 9e38c794c).
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
        assert_eq!(
            args.get(model_idx + 1).map(String::as_str),
            Some("kimi-for-coding/k2p6")
        );
    }

    fn write(dir: &std::path::Path, name: &str, text: &str) {
        std::fs::write(dir.join(name), text).unwrap();
    }

    #[test]
    fn configured_default_reads_jsonc_with_comments_and_trailing_comma() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "opencode.jsonc",
            "{\n  // global default\n  \"model\": \"zai-coding-plan/glm-5.2\",\n}\n",
        );
        let configured = configured_default_model_in(dir.path()).expect("model");
        assert_eq!(configured.model, "zai-coding-plan/glm-5.2");
        assert_eq!(configured.path, dir.path().join("opencode.jsonc"));
    }

    #[test]
    fn configured_default_prefers_current_file_names_over_legacy_config_json() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "config.json", "{\"model\": \"legacy/model\"}");
        write(dir.path(), "opencode.json", "{\"model\": \"json/model\"}");
        assert_eq!(
            configured_default_model_in(dir.path()).unwrap().model,
            "json/model"
        );
        write(dir.path(), "opencode.jsonc", "{\"model\": \"jsonc/model\"}");
        assert_eq!(
            configured_default_model_in(dir.path()).unwrap().model,
            "jsonc/model"
        );
    }

    #[test]
    fn configured_default_falls_through_files_that_name_no_model() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "opencode.jsonc", "{\"model\": \"\"}");
        write(dir.path(), "opencode.json", "{\"theme\": \"dark\"}");
        write(dir.path(), "config.json", "{\"model\": \"legacy/model\"}");
        let configured = configured_default_model_in(dir.path()).unwrap();
        assert_eq!(configured.model, "legacy/model");
        assert_eq!(configured.path, dir.path().join("config.json"));
    }

    #[test]
    fn configured_default_is_none_without_a_config_directory_or_model() {
        let dir = tempfile::tempdir().unwrap();
        assert!(configured_default_model_in(&dir.path().join("missing")).is_none());
        write(dir.path(), "opencode.json", "not json at all");
        assert!(configured_default_model_in(dir.path()).is_none());
    }
}
