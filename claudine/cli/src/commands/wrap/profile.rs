use std::io::Write;
use std::path::Path;

use claudine::events::Provider;
use claudine::stream::StreamProtocol;
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::{Result, bail, eyre};

// ---------------------------------------------------------------------------
// Output format enum (universal --output flag)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    Json,
    Text,
    Stream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PromptDelivery {
    Stdin(String),
    AppendArgs(Vec<String>),
    InsertArgs { index: usize, args: Vec<String> },
}

impl PromptDelivery {
    pub(crate) fn apply_to(self, args: &mut Vec<String>) -> Option<String> {
        match self {
            Self::Stdin(prompt) => Some(prompt),
            Self::AppendArgs(extra) => {
                args.extend(extra);
                None
            }
            Self::InsertArgs { index, args: extra } => {
                args.splice(index..index, extra);
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PromptSource — typed prompt input to the wrap pipeline
// ---------------------------------------------------------------------------

/// A prompt supplied to the wrap pipeline, already extracted from any
/// CLI passthrough or composition source.
///
/// The wrap pipeline holds the prompt as this typed value between
/// extraction (at the entrypoint) and delivery (via `prompt_delivery`).
/// Provider flag-injection methods (`apply_entrypoint`,
/// `apply_non_interactive_flags`) never see or mutate the prompt text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PromptSource {
    /// No prompt provided. Valid only for interactive sessions or
    /// when stdin will be inherited from the parent (the child reads
    /// the TTY directly).
    None,
    /// A text prompt to be placed by `prompt_delivery`.
    Inline(String),
    /// The caller is forwarding piped stdin from its own stdin.
    /// The pipeline should not seed stdin; the child inherits it.
    InheritStdin,
}

#[allow(dead_code)] // first production caller lands in Task 13 (direct-wrap) / Task 14 (composition); drop this attr then
impl PromptSource {
    /// Returns the inline prompt text if this source is `Inline`.
    pub(crate) fn as_inline(&self) -> Option<&str> {
        match self {
            Self::Inline(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Returns true when the source carries no prompt at all.
    pub(crate) fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns true when a prompt reaches the child by any means
    /// (inline delivery OR inherited stdin).
    pub(crate) fn has_prompt_or_stdin(&self) -> bool {
        !matches!(self, Self::None)
    }
}

// ---------------------------------------------------------------------------
// PromptArgConventions — per-provider prompt-argv parsing knowledge
// ---------------------------------------------------------------------------

/// Describes how a provider's native CLI represents a prompt on argv.
///
/// Used by `extract_prompt_source_from_passthrough` to find a prompt in
/// raw passthrough arguments without embedding per-provider logic in a
/// central match.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PromptArgConventions {
    /// Value-taking flags that carry the prompt string when present,
    /// e.g. `&["-p", "--prompt"]` for Gemini, `&["-t", "--text"]` for
    /// Goose. Empty for providers that accept only a positional prompt.
    pub prompt_flags: &'static [&'static str],
    /// An optional entrypoint subcommand that must be skipped when
    /// scanning for a positional prompt, e.g. `Some("exec")` for Codex
    /// or `Some("run")` for OpenCode / Goose. `None` for providers that
    /// have no subcommand entrypoint.
    pub entrypoint: Option<&'static str>,
    /// Additional value-taking flags whose values must not be mistaken
    /// for a positional prompt, e.g. `&["-m", "--model", "--output-format"]`.
    pub value_taking_flags: &'static [&'static str],
}

impl PromptArgConventions {
    /// Conventions for a provider that accepts only a positional prompt
    /// after an entrypoint subcommand (e.g. Codex `exec`, OpenCode `run`).
    pub(crate) const fn positional_after(entrypoint: &'static str) -> Self {
        Self {
            prompt_flags: &[],
            entrypoint: Some(entrypoint),
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
    }
}

/// Value-taking flags recognized by the prompt extractor across every
/// wrapped provider. This is intentionally the UNION of every provider's
/// value-taking flags, not a per-provider list — the extractor's job is
/// to avoid mistaking a flag's value for a positional prompt, and
/// over-skipping an unknown flag's value is harmless. Per-provider
/// `prompt_arg_conventions` implementations can swap in a narrower list
/// if a future provider needs it.
const COMMON_VALUE_TAKING_FLAGS: &[&str] = &[
    "-m",
    "--model",
    "-o",
    "--output",
    "--output-format",
    "--output-last-message",
    "--approval-mode",
    "--config",
    "-c",
    "--profile",
    "--system-prompt",
    "--sandbox-image",
    "--auth-type",
    "--format",
];

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

impl From<claudine::composition::OutputFormat> for OutputFormat {
    fn from(value: claudine::composition::OutputFormat) -> Self {
        match value {
            claudine::composition::OutputFormat::Json => Self::Json,
            claudine::composition::OutputFormat::Text => Self::Text,
            claudine::composition::OutputFormat::Stream => Self::Stream,
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

    // -- Non-interactive mode -----------------------------------------------

    /// Inject the provider's entrypoint subcommand (if any) and any
    /// mode-agnostic launch flags. Called in BOTH interactive and
    /// non-interactive pipelines because some entrypoints (e.g. Codex
    /// `exec`, OpenCode `run`) are needed in both.
    ///
    /// `non_interactive` is true when the wrap is running in
    /// non-interactive mode, so providers whose entrypoint is
    /// conditional on the mode (Claude: `--print`; Kimi: `--print`) can
    /// decide here.
    ///
    /// Default: no-op.
    fn apply_entrypoint(&self, _args: &mut Vec<String>, _non_interactive: bool) {}

    /// Reject mode-conflict flags (e.g. `-i` / `--prompt-interactive`)
    /// when the pipeline is running in non-interactive mode. Runs only
    /// in non-interactive pipelines. Providers that do NOT have such
    /// conflicting flags use the default no-op.
    ///
    /// Default: no-op.
    fn apply_non_interactive_flags(&self, _args: &mut [String]) -> Result<()> {
        Ok(())
    }

    /// **Deprecated** — shim retained during Phase 2-3 of the DRY
    /// providers refactor so callers remain unchanged. Will be removed
    /// in Phase 5 (Task 16). New providers should implement
    /// `apply_entrypoint` and `apply_non_interactive_flags` instead.
    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        self.apply_entrypoint(args, true);
        self.apply_non_interactive_flags(args)
    }

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

    /// Map a resolved system prompt to provider-specific flags, env, and
    /// temp artifacts.
    ///
    /// Default: returns a warning that the provider doesn't support the
    /// given mode.
    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        _interactive: bool,
        _cwd: &Path,
    ) -> Result<super::system_prompt::SystemPromptApplication> {
        Ok(super::system_prompt::SystemPromptApplication {
            args: vec![],
            env: vec![],
            artifacts: vec![],
            warnings: vec![format!(
                "{} does not support {} system prompt; this flag was skipped",
                self.provider(),
                match prompt.mode {
                    SystemPromptMode::Append => "append",
                    SystemPromptMode::Replace => "replace",
                },
            )],
        })
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

    // -- Non-interactive stdout noise filtering --------------------------------

    /// Line prefixes that should be stripped from stdout in non-interactive mode.
    ///
    /// Some CLIs print hook execution debug info, skill conflict warnings, or
    /// other noise to stdout that contaminates non-interactive output. Lines
    /// starting with any of these prefixes are suppressed.
    ///
    /// Default: empty (no filtering).
    fn stdout_noise_prefixes(&self) -> &'static [&'static str] {
        &[]
    }

    // -- Captured output (compose mode) ----------------------------------------

    /// Inject provider-specific flags needed for reliable captured output.
    ///
    /// Providers that dump noise to stdout (hook logs, skill conflicts, etc.)
    /// can override this to request a structured output format that
    /// `parse_captured_output` knows how to parse.
    ///
    /// Only called for compose / captured-output paths, NOT for regular
    /// non-interactive mode where stdout is forwarded to the terminal.
    ///
    /// Default: no-op.
    fn prepare_captured_output(&self, _args: &mut Vec<String>) {}

    /// Extract the assistant response from captured stdout.
    ///
    /// Called after `run_child_capture` when `prepare_captured_output` was
    /// used to inject structured output flags.
    ///
    /// Default: returns raw stdout unchanged.
    fn parse_captured_output(&self, raw: &str) -> String {
        raw.to_string()
    }

    // -- Stderr noise filtering -----------------------------------------------

    /// Line prefixes that should be stripped from stderr in all modes.
    ///
    /// Some CLIs print warnings (e.g. skill conflict notices) to stderr that
    /// are noisy but harmless. Lines starting with any of these prefixes are
    /// suppressed.
    ///
    /// Default: empty (no filtering).
    fn stderr_noise_prefixes(&self) -> &'static [&'static str] {
        &[]
    }

    /// When true, structured non-interactive runs buffer filtered stderr and
    /// only surface it if the provider exits with an error.
    fn suppress_structured_stderr_on_success(&self) -> bool {
        false
    }

    // -- Prompt-file delivery -------------------------------------------------

    /// Build a prompt delivery plan for the provider.
    ///
    /// Providers that need to inspect existing args to place the prompt
    /// correctly (for example after an entrypoint) can do that here, but the
    /// caller remains the only place that mutates child args/stdin state.
    fn prompt_delivery(
        &self,
        args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<PromptDelivery>;

    // -- Final argument validation -------------------------------------------

    /// Validate the final child args after all prompt sources have been
    /// processed (passthrough, --prompt-file, --compose, --frontmatter-prompt).
    ///
    /// Providers that require a positional prompt (Codex, OpenCode, Goose)
    /// should check for it here rather than in `apply_non_interactive()`,
    /// because prompt delivery may happen after non-interactive setup.
    ///
    /// Default: no-op.
    fn validate_final_args(
        &self,
        _args: &[String],
        _non_interactive: bool,
        _has_stdin: bool,
    ) -> Result<()> {
        Ok(())
    }

    // -- Provider-required env vars ------------------------------------------

    /// Env var names that this provider requires and should bypass the
    /// sensitive-key sanitizer automatically.
    ///
    /// Default: empty (no automatic includes).
    fn allowed_env_keys(&self) -> &'static [&'static str] {
        &[]
    }

    // -- Structured stream support -------------------------------------------

    /// Whether this provider supports internal structured streaming.
    fn supports_structured_stream(&self) -> bool {
        false
    }

    /// The stream protocol this provider uses.
    fn stream_protocol(&self) -> Option<StreamProtocol> {
        None
    }

    // -- Resume support -------------------------------------------------------

    /// Build CLI args for resuming a previous session.
    ///
    /// Returns `Ok(args)` with the full argv for a resume invocation, or
    /// `Err` if the provider does not support session resume.
    fn build_resume_args(&self, _session_id: &str) -> Result<Vec<String>> {
        bail!(
            "provider {} does not support session resume",
            self.provider()
        )
    }

    /// Whether this provider supports resuming sessions.
    fn supports_resume(&self) -> bool {
        false
    }

    // -- Structured stream support -------------------------------------------

    /// Apply internal structured stream flags to child args.
    ///
    /// Called only when `supports_structured_stream()` returns true and
    /// the user did not explicitly request an output format.
    fn apply_structured_stream(&self, _args: &mut Vec<String>) {}

    /// Whether Claudine can recover a final assistant body after an
    /// interactive session ends for inline composition closure.
    fn supports_interactive_inline_closure(&self) -> bool {
        false
    }

    // -- Prompt argv conventions --------------------------------------------

    /// Describe how this provider represents a prompt on argv.
    ///
    /// Used by `extract_prompt_source_from_passthrough` to locate and
    /// remove a prompt from raw passthrough args. Every provider that
    /// supports non-interactive mode must implement this; the default
    /// returns "positional-only, no entrypoint" which works for Claude
    /// (prompt as bare positional, no subcommand).
    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions {
            prompt_flags: &[],
            entrypoint: None,
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
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

    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool) {
        if non_interactive && !has_flag(args, "--print") {
            args.push("--print".to_string());
        }
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

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        interactive: bool,
        _cwd: &Path,
    ) -> Result<super::system_prompt::SystemPromptApplication> {
        use super::system_prompt::{SystemPromptApplication, SystemPromptArtifact};
        use std::io::Write as _;

        let mut app = SystemPromptApplication::empty();
        match prompt.mode {
            SystemPromptMode::Append => {
                if interactive {
                    app.args.push("--append-system-prompt".to_string());
                    app.args.push(prompt.composed_markdown.clone());
                } else {
                    let mut tmp = tempfile::NamedTempFile::new()?;
                    tmp.write_all(prompt.composed_markdown.as_bytes())?;
                    app.args.push("--append-system-prompt-file".to_string());
                    app.args.push(tmp.path().display().to_string());
                    app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
                }
            }
            SystemPromptMode::Replace => {
                if interactive {
                    app.args.push("--system-prompt".to_string());
                    app.args.push(prompt.composed_markdown.clone());
                } else {
                    let mut tmp = tempfile::NamedTempFile::new()?;
                    tmp.write_all(prompt.composed_markdown.as_bytes())?;
                    app.args.push("--system-prompt-file".to_string());
                    app.args.push(tmp.path().display().to_string());
                    app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
                }
            }
        }
        Ok(app)
    }

    fn prompt_delivery(
        &self,
        _args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<PromptDelivery> {
        if non_interactive {
            Ok(PromptDelivery::Stdin(prompt.to_string()))
        } else {
            Ok(PromptDelivery::AppendArgs(vec![prompt.to_string()]))
        }
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        Ok(vec![
            "claude".to_string(),
            "-r".to_string(),
            session_id.to_string(),
            "--print".to_string(),
        ])
    }

    fn supports_resume(&self) -> bool {
        true
    }

    fn supports_structured_stream(&self) -> bool {
        true
    }

    fn stream_protocol(&self) -> Option<StreamProtocol> {
        Some(StreamProtocol::StreamJson)
    }

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        args.push("--print".to_string());
        args.push("--verbose".to_string());
        args.push("--output-format".to_string());
        args.push("stream-json".to_string());
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

    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool) {
        // Codex `exec` is the non-interactive entrypoint; interactive
        // sessions use the default TUI (no `exec`). Only inject when
        // the caller is running non-interactively.
        if !non_interactive {
            return;
        }
        let entrypoint = "exec";
        let aliases: &[&str] = &["e"];
        if !args
            .first()
            .is_some_and(|first| first == entrypoint || aliases.contains(&first.as_str()))
        {
            args.insert(0, entrypoint.to_string());
        }
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

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        _interactive: bool,
        _cwd: &Path,
    ) -> Result<super::system_prompt::SystemPromptApplication> {
        use super::system_prompt::{SystemPromptApplication, SystemPromptArtifact};

        let mut app = SystemPromptApplication::empty();
        match prompt.mode {
            SystemPromptMode::Append => {
                let (tmp_home, _overlay_path) =
                    super::system_prompt::create_ephemeral_overlay_home(
                        ".codex",
                        "AGENTS.override.md",
                        &prompt.composed_markdown,
                    )?;
                app.env.push((
                    std::ffi::OsString::from("HOME"),
                    tmp_home.path().as_os_str().to_owned(),
                ));
                app.artifacts.push(SystemPromptArtifact::TempDir(tmp_home));
            }
            SystemPromptMode::Replace => {
                let mut tmp = tempfile::NamedTempFile::new()?;
                tmp.write_all(prompt.composed_markdown.as_bytes())?;
                app.args.push("-c".to_string());
                app.args
                    .push(format!("model_instructions_file={}", tmp.path().display()));
                app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
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
        args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<PromptDelivery> {
        if non_interactive {
            // In non-interactive mode, deliver via stdin to avoid ENAMETOOLONG
            // errors when prompt-file content exceeds OS argument length limits.
            // Codex exec reads from stdin when no positional prompt is provided.
            Ok(PromptDelivery::Stdin(prompt.to_string()))
        } else {
            // Interactive: insert as positional after "exec"
            let insert_at = if args.first().is_some_and(|f| f == "exec" || f == "e") {
                1
            } else {
                0
            };
            Ok(PromptDelivery::InsertArgs {
                index: insert_at,
                args: vec![prompt.to_string()],
            })
        }
    }

    fn validate_final_args(
        &self,
        args: &[String],
        non_interactive: bool,
        has_stdin: bool,
    ) -> Result<()> {
        require_prompt_or_stdin(
            args,
            non_interactive,
            has_stdin,
            &PromptRequirement {
                flag_names: &[],
                skip_entrypoint: true,
                error_message: "--non-interactive for codex requires a prompt after the entrypoint",
            },
        )
    }

    fn allowed_env_keys(&self) -> &'static [&'static str] {
        &["OPENAI_API_KEY", "CODEX_API_KEY"]
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        Ok(vec![
            "codex".to_string(),
            "exec".to_string(),
            "resume".to_string(),
            session_id.to_string(),
        ])
    }

    fn supports_resume(&self) -> bool {
        true
    }

    fn supports_structured_stream(&self) -> bool {
        true
    }

    fn stream_protocol(&self) -> Option<StreamProtocol> {
        Some(StreamProtocol::Jsonl)
    }

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        // Codex uses `exec --json` for structured output.
        // The `exec` subcommand is expected to already be present.
        if !has_flag(args, "--json") {
            args.push("--json".to_string());
        }
    }

    fn stderr_noise_prefixes(&self) -> &'static [&'static str] {
        &["Reading prompt from stdin..."]
    }

    fn supports_interactive_inline_closure(&self) -> bool {
        true
    }

    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions::positional_after("exec")
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

    fn validate_final_args(
        &self,
        args: &[String],
        non_interactive: bool,
        has_stdin: bool,
    ) -> Result<()> {
        require_prompt_or_stdin(
            args,
            non_interactive,
            has_stdin,
            &PromptRequirement {
                flag_names: &["-p", "--prompt"],
                skip_entrypoint: false,
                error_message:
                    "--non-interactive for gemini requires a prompt (positional or --prompt/-p)",
            },
        )
    }

    fn apply_output_format(&self, args: &mut Vec<String>, format: OutputFormat) -> Option<String> {
        match format {
            OutputFormat::Json => {
                if !has_flag(args, "-o") && !has_flag(args, "--output-format") {
                    args.push("--output-format".to_string());
                    args.push("json".to_string());
                }
                None
            }
            OutputFormat::Text => {
                if !has_flag(args, "-o") && !has_flag(args, "--output-format") {
                    args.push("--output-format".to_string());
                    args.push("text".to_string());
                }
                None
            }
            OutputFormat::Stream => {
                if !has_flag(args, "-o") && !has_flag(args, "--output-format") {
                    args.push("--output-format".to_string());
                    args.push("stream-json".to_string());
                }
                None
            }
        }
    }

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        _interactive: bool,
        _cwd: &Path,
    ) -> Result<super::system_prompt::SystemPromptApplication> {
        use super::system_prompt::{SystemPromptApplication, SystemPromptArtifact};

        let mut app = SystemPromptApplication::empty();
        match prompt.mode {
            SystemPromptMode::Append => {
                let (tmp_home, _overlay_path) =
                    super::system_prompt::create_ephemeral_overlay_home(
                        ".gemini",
                        "GEMINI.md",
                        &prompt.composed_markdown,
                    )?;
                app.env.push((
                    std::ffi::OsString::from("HOME"),
                    tmp_home.path().as_os_str().to_owned(),
                ));
                app.artifacts.push(SystemPromptArtifact::TempDir(tmp_home));
            }
            SystemPromptMode::Replace => {
                let mut tmp = tempfile::NamedTempFile::new()?;
                tmp.write_all(prompt.composed_markdown.as_bytes())?;
                app.env.push((
                    std::ffi::OsString::from("GEMINI_SYSTEM_MD"),
                    std::ffi::OsString::from(tmp.path().display().to_string()),
                ));
                app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
            }
        }
        Ok(app)
    }

    fn prompt_delivery(
        &self,
        _args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<PromptDelivery> {
        Ok(PromptDelivery::AppendArgs(vec![
            if non_interactive {
                "--prompt".to_string()
            } else {
                "--prompt-interactive".to_string()
            },
            prompt.to_string(),
        ]))
    }

    fn supports_structured_stream(&self) -> bool {
        true
    }

    fn stream_protocol(&self) -> Option<StreamProtocol> {
        Some(StreamProtocol::StreamJson)
    }

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        args.push("--output-format".to_string());
        args.push("stream-json".to_string());
    }

    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions {
            prompt_flags: &["-p", "--prompt"],
            entrypoint: None,
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
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

    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool) {
        if non_interactive && !has_flag(args, "--print") {
            args.push("--print".to_string());
        }
    }

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        _interactive: bool,
        _cwd: &Path,
    ) -> Result<super::system_prompt::SystemPromptApplication> {
        use super::system_prompt::{SystemPromptApplication, SystemPromptArtifact};

        let mut app = SystemPromptApplication::empty();
        match prompt.mode {
            SystemPromptMode::Append => {
                app.warnings.push(
                    "Kimi does not support append-mode system prompts; this flag was skipped"
                        .to_string(),
                );
            }
            SystemPromptMode::Replace => {
                let mut prompt_tmp = tempfile::NamedTempFile::new()?;
                prompt_tmp.write_all(prompt.composed_markdown.as_bytes())?;

                let agent_yaml = format!(
                    "extend: default\nsystem_prompt_path: {}\n",
                    prompt_tmp.path().display()
                );
                let mut agent_tmp = tempfile::NamedTempFile::new()?;
                agent_tmp.write_all(agent_yaml.as_bytes())?;

                app.args.push("--agent-file".to_string());
                app.args.push(agent_tmp.path().display().to_string());
                app.artifacts
                    .push(SystemPromptArtifact::TempFile(prompt_tmp));
                app.artifacts
                    .push(SystemPromptArtifact::TempFile(agent_tmp));
            }
        }
        Ok(app)
    }

    fn prompt_delivery(
        &self,
        _args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<PromptDelivery> {
        if non_interactive {
            Ok(PromptDelivery::Stdin(prompt.to_string()))
        } else {
            Ok(PromptDelivery::AppendArgs(vec![
                "--prompt".to_string(),
                prompt.to_string(),
            ]))
        }
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        Ok(vec![
            "kimi".to_string(),
            "--resume".to_string(),
            session_id.to_string(),
            "--print".to_string(),
        ])
    }

    fn supports_resume(&self) -> bool {
        true
    }

    fn supports_structured_stream(&self) -> bool {
        true
    }

    fn stream_protocol(&self) -> Option<StreamProtocol> {
        Some(StreamProtocol::StreamJson)
    }

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        args.push("--print".to_string());
        args.push("--output-format".to_string());
        args.push("stream-json".to_string());
    }

    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions {
            prompt_flags: &["--prompt"],
            entrypoint: None,
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
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

    fn apply_non_interactive_flags(&self, args: &mut [String]) -> Result<()> {
        if has_flag(args, "-i") || has_flag(args, "--prompt-interactive") {
            bail!("--non-interactive conflicts with interactive prompt mode for qwen");
        }
        Ok(())
    }

    fn validate_final_args(
        &self,
        args: &[String],
        non_interactive: bool,
        has_stdin: bool,
    ) -> Result<()> {
        require_prompt_or_stdin(
            args,
            non_interactive,
            has_stdin,
            &PromptRequirement {
                flag_names: &["-p", "--prompt"],
                skip_entrypoint: false,
                error_message:
                    "--non-interactive for qwen requires a prompt (positional or --prompt/-p)",
            },
        )
    }

    fn allowed_env_keys(&self) -> &'static [&'static str] {
        &["DASHSCOPE_API_KEY", "QWEN_API_KEY"]
    }

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        _interactive: bool,
        _cwd: &Path,
    ) -> Result<super::system_prompt::SystemPromptApplication> {
        use super::system_prompt::{SystemPromptApplication, SystemPromptArtifact};

        let mut app = SystemPromptApplication::empty();
        match prompt.mode {
            SystemPromptMode::Append => {
                let (tmp_home, _overlay_path) =
                    super::system_prompt::create_ephemeral_overlay_home(
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
        Ok(PromptDelivery::AppendArgs(vec![
            if non_interactive {
                "--prompt".to_string()
            } else {
                "--prompt-interactive".to_string()
            },
            prompt.to_string(),
        ]))
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        Ok(vec![
            "qwen".to_string(),
            "--resume".to_string(),
            session_id.to_string(),
        ])
    }

    fn supports_resume(&self) -> bool {
        true
    }

    fn supports_structured_stream(&self) -> bool {
        true
    }

    fn stream_protocol(&self) -> Option<StreamProtocol> {
        Some(StreamProtocol::StreamJson)
    }

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        args.push("--output-format".to_string());
        args.push("stream-json".to_string());
    }

    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions {
            prompt_flags: &["-p", "--prompt"],
            entrypoint: None,
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
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

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        _interactive: bool,
        _cwd: &Path,
    ) -> Result<super::system_prompt::SystemPromptApplication> {
        use super::system_prompt::{SystemPromptApplication, SystemPromptArtifact};

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

    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool) {
        if !non_interactive {
            return;
        }
        let entrypoint = "run";
        if args.first().is_none_or(|first| first != entrypoint) {
            args.insert(0, entrypoint.to_string());
        }
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
                if !has_flag(args, "--format") {
                    args.push("--format".to_string());
                    args.push("json".to_string());
                }
                None
            }
            _ => Some(format!(
                "OpenCode only supports --format json; {format} was skipped"
            )),
        }
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
            Ok(PromptDelivery::AppendArgs(vec![prompt.to_string()]))
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

    fn validate_final_args(
        &self,
        args: &[String],
        non_interactive: bool,
        has_stdin: bool,
    ) -> Result<()> {
        require_prompt_or_stdin(
            args,
            non_interactive,
            has_stdin,
            &PromptRequirement {
                flag_names: &[],
                skip_entrypoint: true,
                error_message:
                    "--non-interactive for opencode requires a prompt after the entrypoint",
            },
        )
    }

    fn supports_structured_stream(&self) -> bool {
        true
    }

    fn stream_protocol(&self) -> Option<StreamProtocol> {
        Some(StreamProtocol::Ndjson)
    }

    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        args.push("--format".to_string());
        args.push("json".to_string());
    }

    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions::positional_after("run")
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

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        _interactive: bool,
        _cwd: &Path,
    ) -> Result<super::system_prompt::SystemPromptApplication> {
        use super::system_prompt::SystemPromptApplication;

        let mut app = SystemPromptApplication::empty();
        match prompt.mode {
            SystemPromptMode::Append => {
                app.args.push("--system".to_string());
                app.args.push(prompt.composed_markdown.clone());
            }
            SystemPromptMode::Replace => {
                app.warnings.push(
                    "Goose does not support replace-mode system prompts; this flag was skipped"
                        .to_string(),
                );
            }
        }
        Ok(app)
    }

    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool) {
        if !non_interactive {
            return;
        }
        let entrypoint = "run";
        if args.first().is_none_or(|first| first != entrypoint) {
            args.insert(0, entrypoint.to_string());
        }
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

    fn prompt_delivery(
        &self,
        args: &[String],
        prompt: &str,
        _non_interactive: bool,
    ) -> Result<PromptDelivery> {
        // Goose: insert -t <prompt> after "run" subcommand
        if let Some(pos) = args.iter().position(|a| a == "run") {
            Ok(PromptDelivery::InsertArgs {
                index: pos + 1,
                args: vec!["-t".to_string(), prompt.to_string()],
            })
        } else {
            Ok(PromptDelivery::AppendArgs(vec![
                "run".to_string(),
                "-t".to_string(),
                prompt.to_string(),
            ]))
        }
    }

    fn validate_final_args(
        &self,
        args: &[String],
        non_interactive: bool,
        has_stdin: bool,
    ) -> Result<()> {
        require_prompt_or_stdin(
            args,
            non_interactive,
            has_stdin,
            &PromptRequirement {
                flag_names: &["-t"],
                skip_entrypoint: true,
                error_message: "--non-interactive for goose requires a prompt after the entrypoint",
            },
        )
    }

    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions {
            prompt_flags: &["-t", "--text"],
            entrypoint: Some("run"),
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
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
            || arg == "--output-last-message"
            || arg == "--auth-type"
            || arg == "--sandbox-image"
            || arg == "--approval-mode"
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

/// Configuration describing how a provider considers a prompt to be
/// "delivered" when validating the final child argv.
///
/// Providers call [`require_prompt_or_stdin`] from their
/// [`WrapperProfile::validate_final_args`] implementations with one of
/// these, centralizing the "did a prompt reach the provider?" question
/// that previously drifted across per-provider impls (and which caused
/// composition pipelines to bail for Gemini and Qwen because their
/// validation accidentally lived in `apply_non_interactive`).
pub(super) struct PromptRequirement<'a> {
    /// Flag names whose presence counts as a delivered prompt
    /// (e.g. `["-p", "--prompt"]` for Gemini, `["-t"]` for Goose).
    ///
    /// Pass an empty slice for providers that accept only a positional
    /// prompt (Codex, OpenCode).
    pub flag_names: &'a [&'a str],
    /// When true, skip `args[0]` (the entrypoint/subcommand) while
    /// scanning for a positional prompt. Providers whose
    /// `apply_non_interactive` injects an entrypoint (Codex `exec`,
    /// OpenCode/Goose `run`) set this so the entrypoint itself is
    /// never mistaken for a prompt.
    pub skip_entrypoint: bool,
    /// Error message used when no prompt can be found. Kept
    /// per-provider so existing test substrings and wording remain
    /// stable.
    pub error_message: &'a str,
}

/// Shared "does a prompt reach the provider?" check for non-interactive
/// launches.
///
/// Returns `Ok(())` when any of the following holds:
/// - `non_interactive` is false (interactive launches do not require a
///   preloaded prompt — the user types it)
/// - `has_stdin` is true (the prompt is being piped on stdin)
/// - Any flag listed in `req.flag_names` is present in `args`
/// - A non-flag positional arg exists in `args` (after optionally
///   skipping the entrypoint per `req.skip_entrypoint`)
///
/// Otherwise bails with `req.error_message`.
pub(super) fn require_prompt_or_stdin(
    args: &[String],
    non_interactive: bool,
    has_stdin: bool,
    req: &PromptRequirement<'_>,
) -> Result<()> {
    if !non_interactive || has_stdin {
        return Ok(());
    }
    if req.flag_names.iter().any(|flag| has_flag(args, flag)) {
        return Ok(());
    }
    let scan: &[String] = if req.skip_entrypoint && !args.is_empty() {
        &args[1..]
    } else {
        args
    };
    if has_non_flag_positional(scan) {
        return Ok(());
    }
    bail!("{}", req.error_message);
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
// Prompt extraction — consolidates the old extract_user_prompt /
// find_prompt_location / strip_prompt_from_args per-provider logic into
// one provider-blind algorithm that dispatches on PromptArgConventions.
// ---------------------------------------------------------------------------

/// Extract a prompt from raw passthrough args, returning the cleaned
/// args and the typed `PromptSource`.
///
/// This is the *single* place in the codebase that knows how to locate
/// a prompt inside provider passthrough arguments. It replaces the
/// previous per-provider extractors (`extract_user_prompt`,
/// `find_prompt_location`, `strip_prompt_from_args`) and the inline
/// positional-to-flag shuffling that used to live in
/// `apply_non_interactive` for Gemini and Qwen.
///
/// Precedence (highest wins):
/// 1. A prompt-carrying flag from `prompt_arg_conventions().prompt_flags`
///    (e.g. `--prompt VALUE`, `-p=VALUE`)
/// 2. A bare positional arg (after skipping the entrypoint subcommand
///    and any value-taking flags)
/// 3. `has_piped_stdin == true` → `PromptSource::InheritStdin`
/// 4. Otherwise → `PromptSource::None`
///
/// Whenever a flag or positional is returned as the prompt, it is
/// removed from the returned `Vec<String>` so downstream trait methods
/// see clean args with zero prompt characters.
///
/// ## Errors
///
/// Returns an error if a prompt-carrying flag appears in `passthrough`
/// without a following value (e.g. a bare trailing `--prompt`). Silent
/// fall-through in that case would drop the user's intent — piped
/// stdin, if present, would take its place. Surface the problem at
/// extraction time instead.
#[allow(dead_code)] // first production caller lands in Task 13 (direct-wrap) / Task 14 (composition); drop this attr then
pub(crate) fn extract_prompt_source_from_passthrough(
    profile: &dyn WrapperProfile,
    passthrough: &[String],
    has_piped_stdin: bool,
) -> Result<(Vec<String>, PromptSource)> {
    let conv = profile.prompt_arg_conventions();
    let mut args: Vec<String> = passthrough.to_vec();

    // 1. Look for a prompt-carrying flag.
    if let Some((prompt, indices)) = find_prompt_flag(&args, conv.prompt_flags)? {
        // Remove the matched indices in reverse order so earlier
        // indices stay valid while splicing.
        for idx in indices.iter().rev() {
            args.remove(*idx);
        }
        return Ok((args, PromptSource::Inline(prompt)));
    }

    // 2. Look for a positional prompt, skipping the entrypoint (if any)
    //    and any value-taking flags.
    if let Some(idx) = find_positional_prompt_index(&args, &conv) {
        let prompt = args.remove(idx);
        return Ok((args, PromptSource::Inline(prompt)));
    }

    // 3. Piped stdin.
    if has_piped_stdin {
        return Ok((args, PromptSource::InheritStdin));
    }

    // 4. No prompt.
    Ok((args, PromptSource::None))
}

/// Find a prompt delivered via one of `prompt_flags`. Returns the prompt
/// text and the argv indices to remove.
///
/// Supports four shapes:
/// - `--prompt VALUE`      → two indices
/// - `--prompt=VALUE`      → one index
/// - `-p VALUE`            → two indices
/// - `-p=VALUE`            → one index
fn find_prompt_flag(
    args: &[String],
    prompt_flags: &[&str],
) -> Result<Option<(String, Vec<usize>)>> {
    for (idx, arg) in args.iter().enumerate() {
        for flag in prompt_flags {
            if arg == flag {
                let value = args.get(idx + 1).cloned().ok_or_else(|| {
                    eyre!("prompt flag `{flag}` requires a value but none was provided")
                })?;
                return Ok(Some((value, vec![idx, idx + 1])));
            }
            let inline_prefix = format!("{flag}=");
            if let Some(value) = arg.strip_prefix(&inline_prefix) {
                return Ok(Some((value.to_string(), vec![idx])));
            }
        }
    }
    Ok(None)
}

/// Find the index of the first positional prompt candidate in `args`,
/// honoring the entrypoint skip and the set of value-taking flags.
fn find_positional_prompt_index(
    args: &[String],
    conv: &PromptArgConventions,
) -> Option<usize> {
    let mut skip_next = false;
    for (idx, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        // Skip the entrypoint subcommand if it matches at index 0.
        if idx == 0
            && let Some(entry) = conv.entrypoint
            && arg == entry
        {
            continue;
        }

        if arg == "--" {
            return (idx + 1 < args.len()).then_some(idx + 1);
        }

        // Skip value-taking flags so their values are not mistaken for
        // positional prompts. Handle both `--flag value` and
        // `--flag=value` shapes.
        if let Some(eq_idx) = arg.find('=')
            && conv
                .value_taking_flags
                .iter()
                .any(|flag| arg[..eq_idx] == **flag)
        {
            continue;
        }
        if conv.value_taking_flags.iter().any(|flag| arg == *flag) {
            skip_next = true;
            continue;
        }

        if !arg.starts_with('-') {
            return Some(idx);
        }
    }
    None
}

/// Generic "is the prompt requirement satisfied?" check for the wrap
/// pipeline. Called from every call site after all `apply_*` methods
/// have run and `prompt_delivery` has placed any inline prompt.
///
/// Returns `Ok(())` when any of the following holds:
/// - `non_interactive == false` (interactive sessions never require a
///   preloaded prompt — the user will type one)
/// - `source.has_prompt_or_stdin()` is true (inline prompt or piped
///   stdin reaches the child)
///
/// Otherwise bails with a provider-agnostic error message that
/// interpolates `provider_name` so the user knows which wrap failed.
#[allow(dead_code)] // first production caller lands in Task 13 (direct-wrap) / Task 14 (composition); drop this attr then
pub(crate) fn require_prompt_present(
    provider_name: &str,
    non_interactive: bool,
    source: &PromptSource,
) -> Result<()> {
    if !non_interactive {
        return Ok(());
    }
    if source.has_prompt_or_stdin() {
        return Ok(());
    }
    bail!(
        "--non-interactive for {provider_name} requires a prompt \
         (positional, via a prompt flag, or piped on stdin)"
    );
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

    // -- PromptSource tests ------------------------------------------------

    #[test]
    fn prompt_source_as_inline_returns_text_for_inline_variant() {
        let source = PromptSource::Inline("hello".to_string());
        assert_eq!(source.as_inline(), Some("hello"));
    }

    #[test]
    fn prompt_source_as_inline_returns_none_for_non_inline_variants() {
        assert_eq!(PromptSource::None.as_inline(), None);
        assert_eq!(PromptSource::InheritStdin.as_inline(), None);
    }

    #[test]
    fn prompt_source_is_none_only_true_for_none_variant() {
        assert!(PromptSource::None.is_none());
        assert!(!PromptSource::Inline("hi".to_string()).is_none());
        assert!(!PromptSource::InheritStdin.is_none());
    }

    #[test]
    fn prompt_source_has_prompt_or_stdin_accepts_inline_and_stdin() {
        assert!(!PromptSource::None.has_prompt_or_stdin());
        assert!(PromptSource::Inline("x".to_string()).has_prompt_or_stdin());
        assert!(PromptSource::InheritStdin.has_prompt_or_stdin());
    }

    // -- require_prompt_or_stdin shared helper ------------------------------

    fn req_positional_only() -> PromptRequirement<'static> {
        PromptRequirement {
            flag_names: &[],
            skip_entrypoint: true,
            error_message: "requires a prompt",
        }
    }

    fn req_prompt_flag() -> PromptRequirement<'static> {
        PromptRequirement {
            flag_names: &["-p", "--prompt"],
            skip_entrypoint: false,
            error_message: "requires a prompt",
        }
    }

    #[test]
    fn require_prompt_or_stdin_passes_in_interactive_mode() {
        let args: Vec<String> = Vec::new();
        require_prompt_or_stdin(&args, false, false, &req_positional_only()).unwrap();
    }

    #[test]
    fn require_prompt_or_stdin_passes_when_stdin_supplied() {
        let args = vec!["exec".to_string()];
        require_prompt_or_stdin(&args, true, true, &req_positional_only()).unwrap();
    }

    #[test]
    fn require_prompt_or_stdin_accepts_positional_after_entrypoint() {
        let args = vec!["exec".to_string(), "do the thing".to_string()];
        require_prompt_or_stdin(&args, true, false, &req_positional_only()).unwrap();
    }

    #[test]
    fn require_prompt_or_stdin_rejects_entrypoint_as_positional() {
        // Only the entrypoint itself is present — skip_entrypoint must prevent
        // it from being mistaken for a prompt.
        let args = vec!["exec".to_string()];
        let err =
            require_prompt_or_stdin(&args, true, false, &req_positional_only()).unwrap_err();
        assert!(err.to_string().contains("requires a prompt"));
    }

    #[test]
    fn require_prompt_or_stdin_rejects_flags_only() {
        let args = vec!["exec".to_string(), "--json".to_string()];
        let err =
            require_prompt_or_stdin(&args, true, false, &req_positional_only()).unwrap_err();
        assert!(err.to_string().contains("requires a prompt"));
    }

    #[test]
    fn require_prompt_or_stdin_accepts_flag_delivered_prompt() {
        let args = vec!["--prompt".to_string(), "hello".to_string()];
        require_prompt_or_stdin(&args, true, false, &req_prompt_flag()).unwrap();
    }

    #[test]
    fn require_prompt_or_stdin_accepts_short_flag_delivered_prompt() {
        let args = vec!["-p".to_string(), "hello".to_string()];
        require_prompt_or_stdin(&args, true, false, &req_prompt_flag()).unwrap();
    }

    #[test]
    fn require_prompt_or_stdin_empty_args_rejected() {
        let args: Vec<String> = Vec::new();
        let err = require_prompt_or_stdin(&args, true, false, &req_prompt_flag()).unwrap_err();
        assert!(err.to_string().contains("requires a prompt"));
    }

    #[test]
    fn require_prompt_or_stdin_skip_entrypoint_false_accepts_first_positional() {
        // Providers like Gemini/Qwen don't inject an entrypoint, so the first
        // positional IS the prompt (after apply_non_interactive has converted
        // it to --prompt, but also in the rare direct-positional path).
        let args = vec!["hello".to_string()];
        require_prompt_or_stdin(&args, true, false, &req_prompt_flag()).unwrap();
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

        p.apply_non_interactive(&mut args).unwrap();
        let err = p.validate_final_args(&args, true, false).unwrap_err();
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
    fn qwen_apply_non_interactive_flags_rejects_prompt_interactive() {
        let p = profile(Provider::QwenCode);
        let mut args = vec!["-i".to_string(), "task".to_string()];
        let err = p.apply_non_interactive_flags(&mut args).unwrap_err();
        assert!(err.to_string().contains("conflicts"));
    }

    #[test]
    fn qwen_apply_non_interactive_flags_allows_empty_args_for_composition() {
        let p = profile(Provider::QwenCode);
        let mut args: Vec<String> = Vec::new();
        p.apply_non_interactive_flags(&mut args).unwrap();
        assert!(args.is_empty());
    }

    /// Regression test for the composition pipeline path: non-interactive
    /// flag application must NOT bail when args are empty, because the
    /// prompt arrives later via `prompt_delivery`.
    #[test]
    fn gemini_apply_non_interactive_flags_allows_empty_args_for_composition() {
        let p = profile(Provider::Gemini);
        let mut args: Vec<String> = Vec::new();
        p.apply_non_interactive_flags(&mut args).unwrap();
        assert!(args.is_empty());
    }

    #[test]
    fn gemini_apply_non_interactive_flags_rejects_interactive_mode_flags() {
        let p = profile(Provider::Gemini);
        let mut args = vec!["-i".to_string()];
        let err = p.apply_non_interactive_flags(&mut args).unwrap_err();
        assert!(err.to_string().contains("conflicts"));
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

        p.apply_non_interactive(&mut args).unwrap();
        let err = p.validate_final_args(&args, true, false).unwrap_err();
        assert!(err.to_string().contains("requires a prompt"));
    }

    #[test]
    fn opencode_non_interactive_prompt_body_uses_positional_arg() {
        let p = profile(Provider::OpenCode);
        let mut args = vec!["run".to_string()];
        let stdin_seed = p
            .prompt_delivery(&args, "summarize staged files", true)
            .unwrap()
            .apply_to(&mut args);
        assert_eq!(stdin_seed, None);
        assert_eq!(args, vec!["run", "summarize staged files"]);
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

        p.apply_non_interactive(&mut args).unwrap();
        let err = p.validate_final_args(&args, true, false).unwrap_err();
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
    fn gemini_output_format_uses_output_format_flag_and_supports_stream_json() {
        let p = profile(Provider::Gemini);

        let mut json_args = Vec::new();
        assert!(
            p.apply_output_format(&mut json_args, OutputFormat::Json)
                .is_none()
        );
        assert_eq!(json_args, vec!["--output-format", "json"]);

        let mut text_args = Vec::new();
        assert!(
            p.apply_output_format(&mut text_args, OutputFormat::Text)
                .is_none()
        );
        assert_eq!(text_args, vec!["--output-format", "text"]);

        let mut stream_args = Vec::new();
        assert!(
            p.apply_output_format(&mut stream_args, OutputFormat::Stream)
                .is_none()
        );
        assert_eq!(stream_args, vec!["--output-format", "stream-json"]);
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

    // -- PromptArgConventions tests -----------------------------------------

    #[test]
    fn prompt_arg_conventions_claude_uses_defaults() {
        let conv = profile(Provider::Claude).prompt_arg_conventions();
        assert!(conv.prompt_flags.is_empty());
        assert_eq!(conv.entrypoint, None);
    }

    #[test]
    fn prompt_arg_conventions_codex_uses_exec_entrypoint() {
        let conv = profile(Provider::Codex).prompt_arg_conventions();
        assert_eq!(conv.entrypoint, Some("exec"));
        assert!(conv.prompt_flags.is_empty());
    }

    #[test]
    fn prompt_arg_conventions_gemini_uses_prompt_flags() {
        let conv = profile(Provider::Gemini).prompt_arg_conventions();
        assert_eq!(conv.prompt_flags, &["-p", "--prompt"]);
        assert_eq!(conv.entrypoint, None);
    }

    #[test]
    fn prompt_arg_conventions_goose_uses_run_entrypoint_and_text_flags() {
        let conv = profile(Provider::Goose).prompt_arg_conventions();
        assert_eq!(conv.entrypoint, Some("run"));
        assert_eq!(conv.prompt_flags, &["-t", "--text"]);
    }

    #[test]
    fn prompt_arg_conventions_kimi_uses_long_prompt_flag_only() {
        let conv = profile(Provider::KimiCode).prompt_arg_conventions();
        assert_eq!(conv.prompt_flags, &["--prompt"]);
        assert_eq!(conv.entrypoint, None);
    }

    #[test]
    fn prompt_arg_conventions_opencode_uses_run_entrypoint() {
        let conv = profile(Provider::OpenCode).prompt_arg_conventions();
        assert_eq!(conv.entrypoint, Some("run"));
        assert!(conv.prompt_flags.is_empty());
    }

    #[test]
    fn prompt_arg_conventions_qwen_uses_prompt_flags() {
        let conv = profile(Provider::QwenCode).prompt_arg_conventions();
        assert_eq!(conv.prompt_flags, &["-p", "--prompt"]);
        assert_eq!(conv.entrypoint, None);
    }

    // -- extract_prompt_source_from_passthrough ----------------------------

    fn extract(
        provider: Provider,
        passthrough: &[&str],
        has_piped_stdin: bool,
    ) -> (Vec<String>, PromptSource) {
        let args: Vec<String> = passthrough.iter().map(|s| s.to_string()).collect();
        extract_prompt_source_from_passthrough(profile(provider), &args, has_piped_stdin)
            .expect("extract_prompt_source_from_passthrough should succeed")
    }

    #[test]
    fn extract_claude_no_args_yields_none() {
        let (args, source) = extract(Provider::Claude, &[], false);
        assert!(args.is_empty());
        assert_eq!(source, PromptSource::None);
    }

    #[test]
    fn extract_claude_bare_positional_yields_inline() {
        let (args, source) = extract(Provider::Claude, &["hello"], false);
        assert!(args.is_empty());
        assert_eq!(source, PromptSource::Inline("hello".to_string()));
    }

    #[test]
    fn extract_claude_piped_stdin_yields_inherit_stdin() {
        let (args, source) = extract(Provider::Claude, &[], true);
        assert!(args.is_empty());
        assert_eq!(source, PromptSource::InheritStdin);
    }

    #[test]
    fn extract_claude_flag_before_positional_is_preserved() {
        let (args, source) = extract(
            Provider::Claude,
            &["--model", "opus", "fix the bug"],
            false,
        );
        assert_eq!(args, vec!["--model", "opus"]);
        assert_eq!(source, PromptSource::Inline("fix the bug".to_string()));
    }

    #[test]
    fn extract_codex_skips_exec_entrypoint() {
        let (args, source) = extract(Provider::Codex, &["exec", "do it"], false);
        assert_eq!(args, vec!["exec"]);
        assert_eq!(source, PromptSource::Inline("do it".to_string()));
    }

    #[test]
    fn extract_codex_without_exec_still_finds_positional() {
        let (args, source) = extract(Provider::Codex, &["--json", "task"], false);
        assert_eq!(args, vec!["--json"]);
        assert_eq!(source, PromptSource::Inline("task".to_string()));
    }

    #[test]
    fn extract_gemini_long_prompt_flag() {
        let (args, source) =
            extract(Provider::Gemini, &["--prompt", "hi"], false);
        assert!(args.is_empty());
        assert_eq!(source, PromptSource::Inline("hi".to_string()));
    }

    #[test]
    fn extract_gemini_short_prompt_flag() {
        let (args, source) = extract(Provider::Gemini, &["-p", "hi"], false);
        assert!(args.is_empty());
        assert_eq!(source, PromptSource::Inline("hi".to_string()));
    }

    #[test]
    fn extract_gemini_inline_prompt_flag() {
        let (args, source) =
            extract(Provider::Gemini, &["--prompt=hi"], false);
        assert!(args.is_empty());
        assert_eq!(source, PromptSource::Inline("hi".to_string()));
    }

    #[test]
    fn extract_gemini_positional_prompt_after_model_flag() {
        let (args, source) = extract(
            Provider::Gemini,
            &["--model", "flash", "explain this"],
            false,
        );
        assert_eq!(args, vec!["--model", "flash"]);
        assert_eq!(source, PromptSource::Inline("explain this".to_string()));
    }

    #[test]
    fn extract_gemini_positional_skips_approval_mode_value() {
        let (args, source) = extract(
            Provider::Gemini,
            &["--approval-mode", "yolo", "explain this"],
            false,
        );
        assert_eq!(args, vec!["--approval-mode", "yolo"]);
        assert_eq!(source, PromptSource::Inline("explain this".to_string()));
    }

    #[test]
    fn extract_goose_text_flag() {
        let (args, source) =
            extract(Provider::Goose, &["run", "-t", "hello"], false);
        assert_eq!(args, vec!["run"]);
        assert_eq!(source, PromptSource::Inline("hello".to_string()));
    }

    #[test]
    fn extract_kimi_prompt_flag() {
        let (args, source) = extract(Provider::KimiCode, &["--prompt", "hi"], false);
        assert!(args.is_empty());
        assert_eq!(source, PromptSource::Inline("hi".to_string()));
    }

    #[test]
    fn extract_opencode_skips_run_entrypoint() {
        let (args, source) =
            extract(Provider::OpenCode, &["run", "build it"], false);
        assert_eq!(args, vec!["run"]);
        assert_eq!(source, PromptSource::Inline("build it".to_string()));
    }

    #[test]
    fn extract_qwen_long_prompt_flag() {
        let (args, source) =
            extract(Provider::QwenCode, &["--prompt", "hi"], false);
        assert!(args.is_empty());
        assert_eq!(source, PromptSource::Inline("hi".to_string()));
    }

    #[test]
    fn extract_flags_only_returns_none_when_no_piped_stdin() {
        let (args, source) =
            extract(Provider::Codex, &["exec", "--json"], false);
        assert_eq!(args, vec!["exec", "--json"]);
        assert_eq!(source, PromptSource::None);
    }

    #[test]
    fn extract_flags_only_with_piped_stdin_returns_inherit_stdin() {
        let (args, source) = extract(Provider::Codex, &["exec", "--json"], true);
        assert_eq!(args, vec!["exec", "--json"]);
        assert_eq!(source, PromptSource::InheritStdin);
    }

    #[test]
    fn extract_dangling_prompt_flag_returns_error() {
        // Regression test: a prompt flag with no following value must
        // surface as an error rather than silently falling through to the
        // positional / stdin / None branches. Silent fall-through is the
        // original DRY-providers bug this refactor exists to prevent.
        let args: Vec<String> = vec!["--prompt".to_string()];
        let err =
            extract_prompt_source_from_passthrough(profile(Provider::Gemini), &args, false)
                .expect_err("dangling --prompt must return an error");
        let message = err.to_string();
        assert!(
            message.contains("--prompt"),
            "error message should mention the flag: {message}"
        );
        assert!(
            message.contains("requires a value"),
            "error message should mention missing value: {message}"
        );
    }

    #[test]
    fn extract_positional_with_equals_is_not_mistaken_for_flag() {
        // Regression test: a positional argument containing `=` (e.g.
        // an env-var-style token like `KEY=VALUE`) must not be mistaken
        // for a value-taking flag by `find_positional_prompt_index`.
        // `KEY` is not in the known value-taking flag list, so `KEY=VALUE`
        // must be treated as the first positional prompt (not skipped), and
        // the second positional remains in args.
        let (args, source) =
            extract(Provider::Claude, &["KEY=VALUE", "the actual prompt"], false);
        assert_eq!(args, vec!["the actual prompt".to_string()]);
        assert_eq!(
            source,
            PromptSource::Inline("KEY=VALUE".to_string())
        );
    }

    // -- require_prompt_present tests -----------------------------------------

    #[test]
    fn require_prompt_present_passes_in_interactive_mode_with_no_source() {
        require_prompt_present("claude", false, &PromptSource::None).unwrap();
    }

    #[test]
    fn require_prompt_present_passes_with_inline_prompt() {
        require_prompt_present(
            "claude",
            true,
            &PromptSource::Inline("x".to_string()),
        )
        .unwrap();
    }

    #[test]
    fn require_prompt_present_passes_with_inherit_stdin() {
        require_prompt_present("claude", true, &PromptSource::InheritStdin).unwrap();
    }

    #[test]
    fn require_prompt_present_fails_non_interactive_with_no_source() {
        let err =
            require_prompt_present("codex", true, &PromptSource::None).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("codex"));
        assert!(message.contains("requires a prompt"));
    }
}
