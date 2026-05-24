use claudine::provider::Provider;
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::{Result, bail};
use std::path::Path;

use super::{
    PromptDelivery, WrapperProfile, has_any_flag, has_flag, option_value,
    prompt_delivery_append_flags, push_stream_json_flags,
};
use std::io::Write;

pub(crate) struct GeminiWrapper;

impl WrapperProfile for GeminiWrapper {
    fn provider(&self) -> Provider {
        Provider::Gemini
    }

    fn apply_yolo(
        &self,
        args: &mut Vec<String>,
        _env_overrides: &mut Vec<(String, String)>,
    ) -> Result<super::YoloOutcome> {
        // Gemini accepts both `--approval-mode yolo` and
        // `--approval-mode=yolo`, while the typed catalog only records the
        // native setting. Keep this override to preserve conflict checks for
        // existing separated-value argv.
        let flag = "--approval-mode";
        let value = "yolo";
        let aliases: &[&str] = &["--yolo", "-y"];

        if has_any_flag(args, flag, aliases) {
            if let Some(existing) = option_value(args, flag)
                && !existing.eq_ignore_ascii_case(value)
            {
                bail!("--yolo conflicts with existing '{flag} {existing}' for gemini");
            }
            return Ok(super::YoloOutcome::applied());
        }

        args.push(flag.to_string());
        args.push(value.to_string());
        Ok(super::YoloOutcome::applied())
    }

    fn reject_direct_yolo(&self, args: &[String]) -> Result<()> {
        let flag = "--approval-mode";
        let aliases: &[&str] = &["--yolo", "-y"];
        if has_any_flag(args, flag, aliases)
            && option_value(args, flag).is_some_and(|v| v.eq_ignore_ascii_case("yolo"))
        {
            bail!(
                "do not pass <blue>{flag} yolo</blue> directly to claudine gemini; \
                 use Claudine's <blue>--yolo</blue> or <blue>-y</blue> switches instead. \
                 Claudine uses this CLI convention for all agents it provides a wrapper to."
            );
        }
        Ok(())
    }

    fn allowed_env_keys(&self) -> &'static [&'static str] {
        &["GEMINI_API_KEY", "GOOGLE_API_KEY"]
    }

    fn prepare_captured_output(&self, args: &mut Vec<String>) {
        // Use stream-json so we can reliably separate the assistant
        // response from hook logs, skill conflict notices, and other
        // noise that Gemini dumps to stdout.
        if !has_flag(args, "-o") && !has_flag(args, "--output-format") {
            args.push("--output-format".to_string());
            args.push("stream-json".to_string());
        }
    }

    fn parse_captured_output(&self, raw: &str) -> String {
        // Extract assistant content from stream-json lines.
        // Each line is a JSON object; we want {"type":"message","role":"assistant","content":"..."}
        let mut result = String::new();
        for line in raw.lines() {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if value.get("role").and_then(|v| v.as_str()) == Some("assistant")
                && let Some(content) = value.get("content").and_then(|v| v.as_str())
            {
                result.push_str(content);
            }
        }
        result
    }

    fn stdout_noise_prefixes(&self) -> &'static [&'static str] {
        &[
            "Created execution plan for ",
            "Expanding hook command: ",
            "Hook execution for ",
            "Skill conflict detected: ",
            "[LocalAgentExecutor]",
        ]
    }

    fn stderr_noise_prefixes(&self) -> &'static [&'static str] {
        &["Skill conflict detected: ", "[LocalAgentExecutor]"]
    }

    fn suppress_structured_stderr_on_success(&self) -> bool {
        true
    }

    fn apply_non_interactive_flags(&self, args: &mut [String]) -> Result<()> {
        if has_flag(args, "-i") || has_flag(args, "--prompt-interactive") {
            bail!("--non-interactive conflicts with interactive prompt mode for gemini");
        }
        Ok(())
    }

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        interactive: bool,
        _cwd: &Path,
        scoped_tmp: &Path,
    ) -> Result<crate::commands::wrap::system_prompt::SystemPromptApplication> {
        let real_provider_dir = dirs::home_dir().map(|h| h.join(".gemini"));
        crate::commands::wrap::system_prompt::apply_system_prompt_via_spec(
            self.system_prompt_spec(),
            prompt.mode,
            interactive,
            &prompt.composed_markdown,
            real_provider_dir.as_deref(),
            scoped_tmp,
        )
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

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        // Gemini uses the catalog-default --output-format stream-json via
        // push_stream_json_flags, with no extra flags. This override is kept
        // because structured-stream setup is not modeled in the output-format
        // catalog (it is a transport concern, not an output format concern).
        push_stream_json_flags(args, &[]);
    }
}
