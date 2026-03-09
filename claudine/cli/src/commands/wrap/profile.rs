use claudine::events::Provider;
use color_eyre::eyre::{Result, bail};

// ---------------------------------------------------------------------------
// Output format enum (universal --output flag)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    Json,
    Text,
    Stream,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Stream => write!(f, "stream"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "text" => Ok(OutputFormat::Text),
            "stream" | "stream-json" => Ok(OutputFormat::Stream),
            _ => Err(format!(
                "unknown output format '{s}'; expected json, text, or stream"
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// WrapperProfile trait
// ---------------------------------------------------------------------------

/// Each wrapped provider implements this trait to define its CLI mapping.
///
/// The trait moves provider-specific logic (YOLO flags, non-interactive
/// modes, model injection, etc.) into self-contained implementations
/// rather than generic match arms over enum variants.
pub(crate) trait WrapperProfile: Send + Sync {
    /// The `Provider` enum variant for this profile.
    fn provider(&self) -> Provider;

    /// The binary name to exec (e.g. `"claude"`, `"codex"`).
    fn binary(&self) -> &'static str;

    /// The value injected as the `AGENT` environment variable.
    fn agent_env(&self) -> &'static str;

    // -- YOLO mode ----------------------------------------------------------

    /// Apply YOLO/auto-approve mode to `args` and `env_overrides`.
    ///
    /// Returns `Ok(Some(warning))` when YOLO is unsupported for this provider.
    fn apply_yolo(
        &self,
        args: &mut Vec<String>,
        env_overrides: &mut Vec<(String, String)>,
    ) -> Result<Option<String>>;

    /// Whether this provider supports YOLO mode at all.
    fn has_supported_yolo(&self) -> bool;

    /// Reject native YOLO flags passed directly in passthrough args.
    ///
    /// Error messages may contain `<blue>` Prose tags for styled rendering.
    fn reject_direct_yolo(&self, args: &[String]) -> Result<()>;

    // -- Non-interactive mode ------------------------------------------------

    /// Apply non-interactive mode to `args`.
    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()>;

    /// Apply provider-specific defaults for non-interactive mode (e.g.
    /// OpenCode's default model injection). Default: no-op.
    fn apply_non_interactive_defaults(&self, _args: &mut Vec<String>) {}

    // -- Universal --model flag ----------------------------------------------

    /// Map the universal `--model <value>` to provider-specific flags/env.
    ///
    /// Returns `Some(warning)` if the provider doesn't support model selection.
    /// Default implementation pushes `--model <value>`.
    fn apply_model(
        &self,
        args: &mut Vec<String>,
        _env_overrides: &mut Vec<(String, String)>,
        model: &str,
    ) -> Option<String> {
        args.push("--model".to_string());
        args.push(model.to_string());
        None
    }

    // -- Universal --output flag ---------------------------------------------

    /// Map the universal `--output <format>` to provider-specific flags.
    ///
    /// Default: returns a warning that the provider doesn't support it.
    fn apply_output_format(&self, _args: &mut Vec<String>, format: OutputFormat) -> Option<String> {
        Some(format!(
            "{} does not support --output {format}; this flag was skipped",
            self.provider()
        ))
    }

    // -- Universal --system-prompt flag --------------------------------------

    /// Map the universal `--system-prompt <value>` to provider-specific flags.
    ///
    /// Default: returns a warning that the provider doesn't support it.
    fn apply_system_prompt(&self, _args: &mut Vec<String>, _prompt: &str) -> Option<String> {
        Some(format!(
            "{} does not support --system-prompt; this flag was skipped",
            self.provider()
        ))
    }

    // -- Universal --sandbox flag --------------------------------------------

    /// Map the universal `--sandbox` to provider-specific sandboxing.
    ///
    /// Default: returns a warning that the provider doesn't support it.
    fn apply_sandbox(&self, _args: &mut Vec<String>) -> Option<String> {
        Some(format!(
            "{} does not provide sandboxing; the --sandbox flag was skipped",
            self.provider()
        ))
    }

    // -- Provider-required env vars ------------------------------------------

    /// Env var names that this provider requires and should bypass the
    /// sensitive-key sanitizer automatically.
    ///
    /// Default: empty (no automatic includes).
    fn allowed_env_keys(&self) -> &'static [&'static str] {
        &[]
    }
}

// ---------------------------------------------------------------------------
// Provider structs + static instances
// ---------------------------------------------------------------------------

struct ClaudeWrapper;
struct CodexWrapper;
struct GeminiWrapper;
struct KimiWrapper;
struct QwenWrapper;
struct OpencodeWrapper;
struct GooseWrapper;

static CLAUDE: ClaudeWrapper = ClaudeWrapper;
static CODEX: CodexWrapper = CodexWrapper;
static GEMINI: GeminiWrapper = GeminiWrapper;
static KIMI: KimiWrapper = KimiWrapper;
static QWEN: QwenWrapper = QwenWrapper;
static OPENCODE: OpencodeWrapper = OpencodeWrapper;
static GOOSE: GooseWrapper = GooseWrapper;

/// Look up the wrapper profile for a given provider.
///
/// Returns `None` for `Provider::RooCode` because Roo Code is a VS Code
/// extension that cannot be wrapped as a standalone CLI process.
pub(crate) fn profile_for_provider(provider: Provider) -> Option<&'static dyn WrapperProfile> {
    match provider {
        Provider::Claude => Some(&CLAUDE),
        Provider::Codex => Some(&CODEX),
        Provider::Gemini => Some(&GEMINI),
        Provider::KimiCode => Some(&KIMI),
        Provider::QwenCode => Some(&QWEN),
        Provider::OpenCode => Some(&OPENCODE),
        Provider::Goose => Some(&GOOSE),
        // Roo Code runs exclusively as a VS Code extension; there is no
        // standalone CLI binary to wrap.
        Provider::RooCode | _ => None,
    }
}

// ---------------------------------------------------------------------------
// Claude
// ---------------------------------------------------------------------------

impl WrapperProfile for ClaudeWrapper {
    fn provider(&self) -> Provider {
        Provider::Claude
    }
    fn binary(&self) -> &'static str {
        "claude"
    }
    fn agent_env(&self) -> &'static str {
        "claude"
    }

    fn apply_yolo(
        &self,
        args: &mut Vec<String>,
        _env_overrides: &mut Vec<(String, String)>,
    ) -> Result<Option<String>> {
        let flag = "--dangerously-skip-permissions";
        if !has_flag(args, flag) {
            args.push(flag.to_string());
        }
        Ok(None)
    }

    fn has_supported_yolo(&self) -> bool {
        true
    }

    fn reject_direct_yolo(&self, args: &[String]) -> Result<()> {
        let flag = "--dangerously-skip-permissions";
        if has_flag(args, flag) {
            bail!(
                "do not pass <blue>{flag}</blue> directly to claudine claude; \
                 use Claudine's <blue>--yolo</blue> or <blue>-y</blue> switches instead. \
                 Claudine uses this CLI convention for all agents it provides a wrapper to."
            );
        }
        Ok(())
    }

    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        if !has_flag(args, "--print") {
            args.push("--print".to_string());
        }
        Ok(())
    }

    fn apply_output_format(&self, args: &mut Vec<String>, format: OutputFormat) -> Option<String> {
        let value = match format {
            OutputFormat::Json => "json",
            OutputFormat::Text => "text",
            OutputFormat::Stream => "stream-json",
        };
        if !has_flag(args, "--output-format") {
            args.push("--output-format".to_string());
            args.push(value.to_string());
        }
        None
    }

    fn apply_system_prompt(&self, args: &mut Vec<String>, prompt: &str) -> Option<String> {
        args.push("--system-prompt".to_string());
        args.push(prompt.to_string());
        None
    }
}

// ---------------------------------------------------------------------------
// Codex
// ---------------------------------------------------------------------------

impl WrapperProfile for CodexWrapper {
    fn provider(&self) -> Provider {
        Provider::Codex
    }
    fn binary(&self) -> &'static str {
        "codex"
    }
    fn agent_env(&self) -> &'static str {
        "codex"
    }

    fn apply_yolo(
        &self,
        args: &mut Vec<String>,
        _env_overrides: &mut Vec<(String, String)>,
    ) -> Result<Option<String>> {
        let flag = "--dangerously-bypass-approvals-and-sandbox";
        let aliases: &[&str] = &["--yolo"];
        if !has_any_flag(args, flag, aliases) {
            args.push(flag.to_string());
        }
        Ok(None)
    }

    fn has_supported_yolo(&self) -> bool {
        true
    }

    fn reject_direct_yolo(&self, args: &[String]) -> Result<()> {
        let flag = "--dangerously-bypass-approvals-and-sandbox";
        let aliases: &[&str] = &["--yolo"];
        if has_any_flag(args, flag, aliases) {
            bail!(
                "do not pass <blue>{flag}</blue> directly to claudine codex; \
                 use Claudine's <blue>--yolo</blue> or <blue>-y</blue> switches instead. \
                 Claudine uses this CLI convention for all agents it provides a wrapper to."
            );
        }
        Ok(())
    }

    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        let entrypoint = "exec";
        let aliases: &[&str] = &["e"];
        if !args
            .first()
            .is_some_and(|first| first == entrypoint || aliases.contains(&first.as_str()))
        {
            args.insert(0, entrypoint.to_string());
        }

        // Validate prompt is present (consistency with EnsurePromptMode providers)
        if !has_non_flag_positional(&args[1..]) {
            bail!("--non-interactive for codex requires a prompt after the entrypoint");
        }

        Ok(())
    }

    fn apply_output_format(&self, args: &mut Vec<String>, format: OutputFormat) -> Option<String> {
        match format {
            OutputFormat::Json => {
                if !has_flag(args, "--json") {
                    args.push("--json".to_string());
                }
                None
            }
            _ => Some(format!(
                "Codex only supports --output json; {format} was skipped"
            )),
        }
    }

    fn apply_sandbox(&self, args: &mut Vec<String>) -> Option<String> {
        if !has_flag(args, "--sandbox") {
            args.push("--sandbox".to_string());
        }
        None
    }

    fn allowed_env_keys(&self) -> &'static [&'static str] {
        &["OPENAI_API_KEY", "CODEX_API_KEY"]
    }
}

// ---------------------------------------------------------------------------
// Gemini
// ---------------------------------------------------------------------------

impl WrapperProfile for GeminiWrapper {
    fn provider(&self) -> Provider {
        Provider::Gemini
    }
    fn binary(&self) -> &'static str {
        "gemini"
    }
    fn agent_env(&self) -> &'static str {
        "gemini"
    }

    fn apply_yolo(
        &self,
        args: &mut Vec<String>,
        _env_overrides: &mut Vec<(String, String)>,
    ) -> Result<Option<String>> {
        let flag = "--approval-mode";
        let value = "yolo";
        let aliases: &[&str] = &["--yolo", "-y"];

        if has_any_flag(args, flag, aliases) {
            if let Some(existing) = option_value(args, flag)
                && !existing.eq_ignore_ascii_case(value)
            {
                bail!("--yolo conflicts with existing '{flag} {existing}' for gemini");
            }
            return Ok(None);
        }

        args.push(flag.to_string());
        args.push(value.to_string());
        Ok(None)
    }

    fn has_supported_yolo(&self) -> bool {
        true
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

    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        if has_flag(args, "-i") || has_flag(args, "--prompt-interactive") {
            bail!("--non-interactive conflicts with interactive prompt mode for gemini");
        }
        if has_flag(args, "-p") || has_flag(args, "--prompt") {
            return Ok(());
        }
        // Convert a bare positional prompt to --prompt so Gemini CLI
        // runs in explicit headless mode even when stdin is a TTY.
        if let Some(index) = find_first_positional(args) {
            let prompt = args.remove(index);
            args.push("--prompt".to_string());
            args.push(prompt);
            return Ok(());
        }
        bail!("--non-interactive for gemini requires a prompt (positional or --prompt/-p)");
    }

    fn apply_output_format(&self, args: &mut Vec<String>, format: OutputFormat) -> Option<String> {
        match format {
            OutputFormat::Json => {
                if !has_flag(args, "--output") {
                    args.push("--output".to_string());
                    args.push("json".to_string());
                }
                None
            }
            OutputFormat::Text => {
                if !has_flag(args, "--output") {
                    args.push("--output".to_string());
                    args.push("text".to_string());
                }
                None
            }
            OutputFormat::Stream => Some(
                "Gemini does not support --output stream; use json or text instead".to_string(),
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Kimi
// ---------------------------------------------------------------------------

impl WrapperProfile for KimiWrapper {
    fn provider(&self) -> Provider {
        Provider::KimiCode
    }
    fn binary(&self) -> &'static str {
        "kimi"
    }
    fn agent_env(&self) -> &'static str {
        "kimi"
    }

    fn apply_yolo(
        &self,
        args: &mut Vec<String>,
        _env_overrides: &mut Vec<(String, String)>,
    ) -> Result<Option<String>> {
        if !has_flag(args, "--yolo") {
            args.push("--yolo".to_string());
        }
        Ok(None)
    }

    fn has_supported_yolo(&self) -> bool {
        true
    }

    fn reject_direct_yolo(&self, args: &[String]) -> Result<()> {
        // Kimi's native YOLO flag is also `--yolo`, which is extracted by
        // Claudine's flag extraction before reaching here. No additional
        // rejection needed.
        let _ = args;
        Ok(())
    }

    fn allowed_env_keys(&self) -> &'static [&'static str] {
        &["KIMI_API_KEY"]
    }

    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        if !has_flag(args, "--print") {
            args.push("--print".to_string());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Qwen
// ---------------------------------------------------------------------------

impl WrapperProfile for QwenWrapper {
    fn provider(&self) -> Provider {
        Provider::QwenCode
    }
    fn binary(&self) -> &'static str {
        "qwen"
    }
    fn agent_env(&self) -> &'static str {
        "qwen"
    }

    fn apply_yolo(
        &self,
        args: &mut Vec<String>,
        _env_overrides: &mut Vec<(String, String)>,
    ) -> Result<Option<String>> {
        // Qwen supports both --yolo and --approval-mode; check for conflicts.
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

    fn has_supported_yolo(&self) -> bool {
        true
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

    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        if has_flag(args, "-i") || has_flag(args, "--prompt-interactive") {
            bail!("--non-interactive conflicts with interactive prompt mode for qwen");
        }
        if has_flag(args, "-p") || has_flag(args, "--prompt") {
            return Ok(());
        }
        // Convert a bare positional prompt to --prompt so Qwen CLI
        // runs in explicit headless mode even when stdin is a TTY.
        if let Some(index) = find_first_positional(args) {
            let prompt = args.remove(index);
            args.push("--prompt".to_string());
            args.push(prompt);
            return Ok(());
        }
        bail!("--non-interactive for qwen requires a prompt (positional or --prompt/-p)");
    }

    fn allowed_env_keys(&self) -> &'static [&'static str] {
        &["DASHSCOPE_API_KEY", "QWEN_API_KEY"]
    }

    fn apply_sandbox(&self, args: &mut Vec<String>) -> Option<String> {
        if !has_flag(args, "--sandbox") {
            args.push("--sandbox".to_string());
        }
        None
    }
}

// ---------------------------------------------------------------------------
// OpenCode
// ---------------------------------------------------------------------------

impl WrapperProfile for OpencodeWrapper {
    fn provider(&self) -> Provider {
        Provider::OpenCode
    }
    fn binary(&self) -> &'static str {
        "opencode"
    }
    fn agent_env(&self) -> &'static str {
        "opencode"
    }

    fn apply_yolo(
        &self,
        _args: &mut Vec<String>,
        _env_overrides: &mut Vec<(String, String)>,
    ) -> Result<Option<String>> {
        Ok(Some(
            "--yolo is not supported for 'opencode' and was ignored".to_string(),
        ))
    }

    fn has_supported_yolo(&self) -> bool {
        false
    }

    fn reject_direct_yolo(&self, _args: &[String]) -> Result<()> {
        Ok(())
    }

    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        let entrypoint = "run";
        if args.first().is_none_or(|first| first != entrypoint) {
            args.insert(0, entrypoint.to_string());
        }

        // Validate prompt is present (consistency with EnsurePromptMode providers)
        if !has_non_flag_positional(&args[1..]) {
            bail!("--non-interactive for opencode requires a prompt after the entrypoint");
        }

        Ok(())
    }

    fn apply_non_interactive_defaults(&self, args: &mut Vec<String>) {
        if has_flag(args, "--model") || has_flag(args, "-m") {
            return;
        }
        let model = non_empty_env_var("OPENCODE_MODEL")
            .or_else(|| non_empty_env_var("MODEL"))
            .unwrap_or_else(|| "minimax/MiniMax-M2.5-highspeed".to_string());
        args.push("--model".to_string());
        args.push(model);
    }

    fn apply_model(
        &self,
        args: &mut Vec<String>,
        env_overrides: &mut Vec<(String, String)>,
        model: &str,
    ) -> Option<String> {
        args.push("--model".to_string());
        args.push(model.to_string());
        // OpenCode also needs the MODEL env var set.
        env_overrides.push(("MODEL".to_string(), model.to_string()));
        None
    }

    fn apply_output_format(&self, args: &mut Vec<String>, format: OutputFormat) -> Option<String> {
        match format {
            OutputFormat::Json => {
                if !has_flag(args, "--output-format") {
                    args.push("--output-format".to_string());
                    args.push("json".to_string());
                }
                None
            }
            _ => Some(format!(
                "OpenCode only supports --output json; {format} was skipped"
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Goose
// ---------------------------------------------------------------------------

impl WrapperProfile for GooseWrapper {
    fn provider(&self) -> Provider {
        Provider::Goose
    }
    fn binary(&self) -> &'static str {
        "goose"
    }
    fn agent_env(&self) -> &'static str {
        "goose"
    }

    fn apply_yolo(
        &self,
        _args: &mut Vec<String>,
        env_overrides: &mut Vec<(String, String)>,
    ) -> Result<Option<String>> {
        let key = "GOOSE_MODE";
        let value = "auto";
        if !env_overrides.iter().any(|(k, v)| k == key && v == value) {
            env_overrides.push((key.to_string(), value.to_string()));
        }
        Ok(None)
    }

    fn has_supported_yolo(&self) -> bool {
        true
    }

    fn reject_direct_yolo(&self, _args: &[String]) -> Result<()> {
        // Goose YOLO is via env var, not a CLI flag. Nothing to reject.
        Ok(())
    }

    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        let entrypoint = "run";
        if args.first().is_none_or(|first| first != entrypoint) {
            args.insert(0, entrypoint.to_string());
        }

        // Validate prompt is present (consistency with EnsurePromptMode providers)
        if !has_non_flag_positional(&args[1..]) {
            bail!("--non-interactive for goose requires a prompt after the entrypoint");
        }

        Ok(())
    }

    fn apply_model(
        &self,
        _args: &mut Vec<String>,
        env_overrides: &mut Vec<(String, String)>,
        model: &str,
    ) -> Option<String> {
        env_overrides.push(("GOOSE_MODEL".to_string(), model.to_string()));
        None
    }
}

// ---------------------------------------------------------------------------
// Shared helper functions
// ---------------------------------------------------------------------------

fn non_empty_env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

pub(super) fn has_non_flag_positional(args: &[String]) -> bool {
    find_first_positional(args).is_some()
}

/// Return the index of the first positional (non-flag) argument.
fn find_first_positional(args: &[String]) -> Option<usize> {
    let mut skip_next = false;
    for (index, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        if arg == "--" {
            return Some(index);
        }

        // Skip known value-taking flags so their values aren't mistaken for
        // positional arguments.
        if arg == "-m"
            || arg == "--model"
            || arg == "--output-format"
            || arg == "-o"
            || arg == "--auth-type"
            || arg == "--sandbox-image"
        {
            skip_next = true;
            continue;
        }

        if !arg.starts_with('-') {
            return Some(index);
        }
    }

    None
}

fn has_any_flag(args: &[String], primary: &str, aliases: &[&str]) -> bool {
    if has_flag(args, primary) {
        return true;
    }
    aliases.iter().any(|alias| has_flag(args, alias))
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter()
        .any(|arg| arg == flag || arg.starts_with(&format!("{flag}=")))
}

fn option_value(args: &[String], option: &str) -> Option<String> {
    for (index, arg) in args.iter().enumerate() {
        if arg == option {
            return args.get(index + 1).cloned();
        }
        if let Some(value) = arg.strip_prefix(&format!("{option}=")) {
            return Some(value.to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn profile(provider: Provider) -> &'static dyn WrapperProfile {
        profile_for_provider(provider).unwrap()
    }

    #[test]
    fn codex_non_interactive_ensures_exec_once() {
        let p = profile(Provider::Codex);
        let mut args = vec!["exec".to_string(), "--json".to_string(), "task".to_string()];

        p.apply_non_interactive(&mut args).unwrap();
        p.apply_non_interactive(&mut args).unwrap();

        assert_eq!(args, vec!["exec", "--json", "task"]);
    }

    #[test]
    fn codex_non_interactive_prepends_exec() {
        let p = profile(Provider::Codex);
        let mut args = vec!["--json".to_string(), "summarize".to_string()];

        p.apply_non_interactive(&mut args).unwrap();

        assert_eq!(args, vec!["exec", "--json", "summarize"]);
    }

    #[test]
    fn codex_non_interactive_rejects_missing_prompt() {
        let p = profile(Provider::Codex);
        let mut args = vec!["--json".to_string()];

        let err = p.apply_non_interactive(&mut args).unwrap_err();
        assert!(err.to_string().contains("requires a prompt"));
    }

    #[test]
    fn gemini_yolo_mapping_is_idempotent() {
        let p = profile(Provider::Gemini);
        let mut args = vec!["--approval-mode".to_string(), "yolo".to_string()];
        let mut env_overrides = Vec::new();

        p.apply_yolo(&mut args, &mut env_overrides).unwrap();
        p.apply_yolo(&mut args, &mut env_overrides).unwrap();

        assert_eq!(args, vec!["--approval-mode", "yolo"]);
    }

    #[test]
    fn gemini_yolo_conflicts_with_non_yolo_approval_mode() {
        let p = profile(Provider::Gemini);
        let mut args = vec!["--approval-mode".to_string(), "default".to_string()];
        let mut env_overrides = Vec::new();

        let error = p.apply_yolo(&mut args, &mut env_overrides).unwrap_err();
        assert!(error.to_string().contains("conflicts"));
    }

    #[test]
    fn qwen_yolo_conflicts_with_non_yolo_approval_mode() {
        let p = profile(Provider::QwenCode);
        let mut args = vec!["--approval-mode".to_string(), "careful".to_string()];
        let mut env_overrides = Vec::new();

        let error = p.apply_yolo(&mut args, &mut env_overrides).unwrap_err();
        assert!(error.to_string().contains("conflicts"));
    }

    #[test]
    fn qwen_non_interactive_rejects_prompt_interactive() {
        let p = profile(Provider::QwenCode);
        let mut args = vec!["-i".to_string(), "task".to_string()];

        let error = p.apply_non_interactive(&mut args).unwrap_err();
        assert!(error.to_string().contains("conflicts"));
    }

    #[test]
    fn gemini_non_interactive_converts_positional_to_prompt_flag() {
        let p = profile(Provider::Gemini);
        let mut args = vec!["hi".to_string()];

        p.apply_non_interactive(&mut args).unwrap();

        assert_eq!(args, vec!["--prompt", "hi"]);
    }

    #[test]
    fn gemini_non_interactive_preserves_existing_prompt_flag() {
        let p = profile(Provider::Gemini);
        let mut args = vec!["--prompt".to_string(), "hi".to_string()];

        p.apply_non_interactive(&mut args).unwrap();

        assert_eq!(args, vec!["--prompt", "hi"]);
    }

    #[test]
    fn gemini_non_interactive_converts_positional_with_other_flags() {
        let p = profile(Provider::Gemini);
        let mut args = vec![
            "--model".to_string(),
            "flash".to_string(),
            "explain this".to_string(),
        ];

        p.apply_non_interactive(&mut args).unwrap();

        assert_eq!(args, vec!["--model", "flash", "--prompt", "explain this"]);
    }

    #[test]
    fn qwen_non_interactive_converts_positional_to_prompt_flag() {
        let p = profile(Provider::QwenCode);
        let mut args = vec!["hi".to_string()];

        p.apply_non_interactive(&mut args).unwrap();

        assert_eq!(args, vec!["--prompt", "hi"]);
    }

    #[test]
    fn qwen_reject_direct_yolo_catches_approval_mode_yolo() {
        let p = profile(Provider::QwenCode);
        let args = vec!["--approval-mode".to_string(), "yolo".to_string()];

        let error = p.reject_direct_yolo(&args).unwrap_err();
        assert!(error.to_string().contains("do not pass"));
        assert!(error.to_string().contains("--approval-mode yolo"));
    }

    #[test]
    fn opencode_yolo_warns_without_mutating_args() {
        let p = profile(Provider::OpenCode);
        let mut args = vec!["run".to_string(), "status".to_string()];
        let mut env_overrides = Vec::new();

        let warning = p.apply_yolo(&mut args, &mut env_overrides).unwrap();
        assert!(warning.unwrap().contains("ignored"));
        assert_eq!(args, vec!["run", "status"]);
    }

    #[test]
    fn opencode_non_interactive_defaults_add_model_when_missing() {
        let p = profile(Provider::OpenCode);
        let mut args = vec!["run".to_string(), "status".to_string()];

        p.apply_non_interactive_defaults(&mut args);

        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"minimax/MiniMax-M2.5-highspeed".to_string()));
    }

    #[test]
    fn opencode_non_interactive_defaults_respect_existing_model_flag() {
        let p = profile(Provider::OpenCode);
        let mut args = vec![
            "run".to_string(),
            "--model".to_string(),
            "user-choice".to_string(),
        ];

        p.apply_non_interactive_defaults(&mut args);

        let model_flags = args.iter().filter(|a| a.as_str() == "--model").count();
        assert_eq!(model_flags, 1);
        assert!(args.contains(&"user-choice".to_string()));
    }

    #[test]
    fn opencode_non_interactive_rejects_missing_prompt() {
        let p = profile(Provider::OpenCode);
        let mut args = vec!["--json".to_string()];

        let err = p.apply_non_interactive(&mut args).unwrap_err();
        assert!(err.to_string().contains("requires a prompt"));
    }

    #[test]
    fn goose_yolo_env_override_is_idempotent() {
        let p = profile(Provider::Goose);
        let mut args = Vec::new();
        let mut env_overrides = Vec::new();

        p.apply_yolo(&mut args, &mut env_overrides).unwrap();
        p.apply_yolo(&mut args, &mut env_overrides).unwrap();

        let unique: HashSet<_> = env_overrides.into_iter().collect();
        assert_eq!(unique.len(), 1);
        assert!(unique.contains(&("GOOSE_MODE".to_string(), "auto".to_string())));
    }

    #[test]
    fn goose_non_interactive_rejects_missing_prompt() {
        let p = profile(Provider::Goose);
        let mut args = Vec::new();

        let err = p.apply_non_interactive(&mut args).unwrap_err();
        assert!(err.to_string().contains("requires a prompt"));
    }

    #[test]
    fn goose_model_uses_env_var() {
        let p = profile(Provider::Goose);
        let mut args = Vec::new();
        let mut env_overrides = Vec::new();

        let warning = p.apply_model(&mut args, &mut env_overrides, "gpt-4o");
        assert!(warning.is_none());
        assert!(args.is_empty());
        assert!(env_overrides.contains(&("GOOSE_MODEL".to_string(), "gpt-4o".to_string())));
    }

    #[test]
    fn direct_provider_yolo_flag_is_rejected_with_guidance() {
        let p = profile(Provider::Claude);
        let args = vec!["--dangerously-skip-permissions".to_string()];

        let error = p.reject_direct_yolo(&args).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("do not pass"));
        assert!(message.contains("--dangerously-skip-permissions"));
        assert!(message.contains("--yolo"));
    }

    #[test]
    fn roo_code_has_no_wrapper_profile() {
        assert!(profile_for_provider(Provider::RooCode).is_none());
    }

    #[test]
    fn all_wrapped_providers_have_profiles() {
        let wrapped = [
            Provider::Claude,
            Provider::Codex,
            Provider::Gemini,
            Provider::KimiCode,
            Provider::QwenCode,
            Provider::OpenCode,
            Provider::Goose,
        ];
        for provider in wrapped {
            assert!(
                profile_for_provider(provider).is_some(),
                "Missing profile for {provider:?}"
            );
        }
    }

    #[test]
    fn claude_output_format_supports_all_formats() {
        let p = profile(Provider::Claude);
        for format in [OutputFormat::Json, OutputFormat::Text, OutputFormat::Stream] {
            let mut args = Vec::new();
            let warning = p.apply_output_format(&mut args, format);
            assert!(warning.is_none(), "Claude should support --output {format}");
            assert!(!args.is_empty());
        }
    }

    #[test]
    fn codex_sandbox_adds_flag() {
        let p = profile(Provider::Codex);
        let mut args = Vec::new();
        let warning = p.apply_sandbox(&mut args);
        assert!(warning.is_none());
        assert_eq!(args, vec!["--sandbox"]);
    }

    #[test]
    fn qwen_sandbox_adds_flag() {
        let p = profile(Provider::QwenCode);
        let mut args = Vec::new();
        let warning = p.apply_sandbox(&mut args);
        assert!(warning.is_none());
        assert_eq!(args, vec!["--sandbox"]);
    }

    #[test]
    fn unsupported_sandbox_returns_warning() {
        let p = profile(Provider::Claude);
        let mut args = Vec::new();
        let warning = p.apply_sandbox(&mut args);
        assert!(warning.is_some());
        assert!(args.is_empty());
    }
}
