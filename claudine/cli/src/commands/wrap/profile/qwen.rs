use claudine::provider::Provider;
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::{Result, bail};
use std::path::Path;

use super::{
    PromptDelivery, WrapperProfile, has_flag, option_value, prompt_delivery_append_flags,
    push_stream_json_flags,
};

pub(crate) struct QwenWrapper;

impl WrapperProfile for QwenWrapper {
    fn provider(&self) -> Provider {
        Provider::QwenCode
    }

    fn apply_yolo(
        &self,
        args: &mut Vec<String>,
        _env_overrides: &mut Vec<(String, String)>,
    ) -> Result<Option<String>> {
        // Qwen's catalog records the direct `--yolo` flag, but the runtime
        // also accepts `--approval-mode yolo`. Keep this override to reject
        // conflicting approval modes before appending the catalog flag.
        if let Some(value) = option_value(args, "--approval-mode")
            && !value.eq_ignore_ascii_case("yolo")
        {
            bail!("--yolo conflicts with existing '--approval-mode {value}' for qwen");
        }
        if !has_flag(args, "--yolo") {
            args.push("--yolo".to_string());
        }
        Ok(None)
    }

    fn reject_direct_yolo(&self, args: &[String]) -> Result<()> {
        // Qwen's native YOLO flag is `--yolo`, which is extracted by Claudine's
        // flag extraction before reaching here. However, if the user passes
        // `--approval-mode yolo` directly, that should be rejected.
        if has_flag(args, "--approval-mode")
            && option_value(args, "--approval-mode").is_some_and(|v| v.eq_ignore_ascii_case("yolo"))
        {
            bail!(
                "do not pass <blue>--approval-mode yolo</blue> directly to claudine qwen; \
                 use Claudine's <blue>--yolo</blue> or <blue>-y</blue> switches instead. \
                 Claudine uses this CLI convention for all agents it provides a wrapper to."
            );
        }
        Ok(())
    }

    fn apply_non_interactive_flags(&self, args: &mut [String]) -> Result<()> {
        if has_flag(args, "-i") || has_flag(args, "--prompt-interactive") {
            bail!("--non-interactive conflicts with interactive prompt mode for qwen");
        }
        Ok(())
    }

    fn allowed_env_keys(&self) -> &'static [&'static str] {
        &["DASHSCOPE_API_KEY", "QWEN_API_KEY"]
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
                        ".qwen",
                        "QWEN.md",
                        &prompt.composed_markdown,
                    )?;
                app.env.push((
                    std::ffi::OsString::from("HOME"),
                    tmp_home.path().as_os_str().to_owned(),
                ));
                app.artifacts.push(SystemPromptArtifact::TempDir(tmp_home));
            }
            SystemPromptMode::Replace => {
                app.warnings.push(
                    "Qwen does not support replace-mode system prompts; this flag was skipped"
                        .to_string(),
                );
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
        _args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<PromptDelivery> {
        Ok(prompt_delivery_append_flags(
            prompt,
            non_interactive,
            "--prompt",
            "--prompt-interactive",
        ))
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        Ok(vec![
            "qwen".to_string(),
            "--resume".to_string(),
            session_id.to_string(),
        ])
    }

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        // Qwen uses the catalog-default --output-format stream-json via
        // push_stream_json_flags, with no extra flags. Structured-stream
        // setup is not modeled in the output-format catalog.
        push_stream_json_flags(args, &[]);
    }
}
