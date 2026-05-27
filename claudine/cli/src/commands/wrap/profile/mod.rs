use std::path::{Path, PathBuf};

use claudine::provider::Provider;
use claudine::provider::SystemPromptSpec;
use claudine::provider::{
    EntrypointMode, OutputFormatSelector, PROVIDER_COUNT, PromptArgConventions, YoloSupport,
    provider_info,
};
use claudine::stream::StreamProtocol;
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::{Result, bail, eyre};

mod claude;
mod codex;
mod gemini;
mod goose;
mod kimi;
mod opencode;
mod qwen;

pub(crate) use self::claude::ClaudeWrapper;
pub(crate) use self::codex::CodexWrapper;
pub(crate) use self::gemini::GeminiWrapper;
pub(crate) use self::goose::GooseWrapper;
pub(crate) use self::kimi::KimiWrapper;
pub(crate) use self::opencode::OpencodeWrapper;
#[allow(unused_imports)]
pub(crate) use self::opencode::opencode_default_tui_noise_prefixes;
pub(crate) use self::qwen::QwenWrapper;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpenCodeModelSource {
    CliSwitch(String),
    OpenCodeModelEnv(String),
    ConfigDefault(String),
}

impl OpenCodeModelSource {
    pub(crate) fn model(&self) -> &str {
        match self {
            Self::CliSwitch(m) | Self::OpenCodeModelEnv(m) | Self::ConfigDefault(m) => m,
        }
    }

    pub(crate) fn status_markup(&self) -> String {
        let model = self.model();
        match self {
            Self::CliSwitch(_) => format!(
                "<dim><i>using the </i><yellow>{model}</yellow><i> based on the CLI switch override used by caller</i></dim>"
            ),
            Self::OpenCodeModelEnv(_) => format!(
                "<dim><i>using the </i><yellow>{model}</yellow><i> based on the OPENCODE_MODEL environment variable</i></dim>"
            ),
            Self::ConfigDefault(_) => format!(
                "<dim><i>using the </i><b>{model}</b><i> because this is the default configured in <blue>~/.config/opencode/config.json</blue></i></dim>"
            ),
        }
    }

    pub(crate) fn location_string(&self) -> &'static str {
        match self {
            Self::CliSwitch(_) => "the --model CLI switch",
            Self::OpenCodeModelEnv(_) => "the OPENCODE_MODEL environment variable",
            Self::ConfigDefault(_) => "the config file ~/.config/opencode/config.json",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoModelProvided;

impl std::fmt::Display for NoModelProvided {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no model provided")
    }
}

impl std::error::Error for NoModelProvided {}

#[derive(Debug, Default, Clone)]
pub(crate) struct OpenCodeEnvSnapshot {
    pub opencode_model_env: Option<String>,
    pub opencode_config_model: Option<String>,
}

impl OpenCodeEnvSnapshot {
    pub(crate) fn from_system() -> Self {
        Self {
            opencode_model_env: non_empty_env_var("OPENCODE_MODEL"),
            opencode_config_model: read_opencode_config_model(),
        }
    }
}

pub(crate) fn resolve_opencode_model(
    cli_model: Option<&str>,
    snapshot: &OpenCodeEnvSnapshot,
) -> std::result::Result<OpenCodeModelSource, NoModelProvided> {
    if let Some(m) = cli_model {
        return Ok(OpenCodeModelSource::CliSwitch(m.to_string()));
    }

    if let Some(m) = &snapshot.opencode_model_env {
        return Ok(OpenCodeModelSource::OpenCodeModelEnv(m.clone()));
    }

    if let Some(m) = &snapshot.opencode_config_model {
        return Ok(OpenCodeModelSource::ConfigDefault(m.clone()));
    }

    Err(NoModelProvided)
}

fn opencode_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".config/opencode/config.json"))
}

fn read_opencode_config_model() -> Option<String> {
    let path = opencode_config_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    let model = value.get("model")?.as_str()?;
    if model.is_empty() {
        return None;
    }
    Some(model.to_string())
}

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
    InsertArgs {
        index: usize,
        args: Vec<String>,
    },
    /// Wire-mode JSON-RPC prompt delivery: the prompt is sent as the
    /// `params.user_input` of a JSON-RPC `prompt` request after
    /// `initialize` completes. Used by Kimi non-interactive runs which
    /// route through [`super::wire_io::run_kimi_wire_session`] instead of
    /// the standard structured-stream child-stdin path.
    WireRpc(String),
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
            // Wire-mode delivery is handled by the wire_io session
            // orchestrator, not the structured-stream child-stdin path.
            // Returning `None` here keeps the standard pipeline from
            // seeding stdin; the dispatcher inspects the original
            // `PromptDelivery` value before `apply_to` is consumed when
            // it needs the prompt body for the JSON-RPC request.
            Self::WireRpc(_) => None,
        }
    }

    /// Returns the wire-mode prompt body when this delivery routes through
    /// the JSON-RPC `prompt` request path, otherwise `None`.
    pub(crate) fn as_wire_rpc(&self) -> Option<&str> {
        match self {
            Self::WireRpc(prompt) => Some(prompt.as_str()),
            _ => None,
        }
    }
}

/// Outcome of applying YOLO/auto-approve mode to a provider launch.
///
/// `applied` is the single source of truth for "did the provider's
/// native bypass mechanism actually take effect for this launch":
/// the wrapper reports `effective_yolo = applied` on the session's
/// synthesized session_end event, the header badge reads it, and the
/// trends-table "Yolo" column counts sessions where it was true.
///
/// `applied: true` means one of:
/// - the catalog's `native_flag` (or alias) was pushed onto argv
/// - the catalog's `env_var=value` pair was added to env_overrides
/// - the catalog's `non_interactive_flag` was pushed in non-interactive mode
///
/// `applied: false` (with an optional warning) means YOLO was requested
/// but suppressed — e.g. OpenCode in interactive TUI mode, or a
/// provider whose catalog records [`YoloSupport::None`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct YoloOutcome {
    /// Whether the provider's native bypass mechanism actually took
    /// effect on this launch.
    pub applied: bool,
    /// Optional user-facing warning to surface (e.g. mode mismatch).
    pub warning: Option<String>,
}

impl YoloOutcome {
    /// Yolo took effect on the launch.
    pub(crate) fn applied() -> Self {
        Self {
            applied: true,
            warning: None,
        }
    }

    /// Yolo did not take effect; `warning` explains why.
    pub(crate) fn not_applied(warning: impl Into<String>) -> Self {
        Self {
            applied: false,
            warning: Some(warning.into()),
        }
    }
}

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

impl PromptSource {
    /// Returns the inline prompt text if this source is `Inline`.
    pub(crate) fn as_inline(&self) -> Option<&str> {
        match self {
            Self::Inline(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Returns true when the source carries no prompt at all.
    #[allow(dead_code)] // used in Task 14 (composition path) and Task 17 cleanup
    pub(crate) fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns true when a prompt reaches the child by any means
    /// (inline delivery OR inherited stdin).
    pub(crate) fn has_prompt_or_stdin(&self) -> bool {
        !matches!(self, Self::None)
    }
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

impl From<claudine::composition::OutputFormat> for OutputFormat {
    fn from(value: claudine::composition::OutputFormat) -> Self {
        match value {
            claudine::composition::OutputFormat::Json => Self::Json,
            claudine::composition::OutputFormat::Text => Self::Text,
            claudine::composition::OutputFormat::Stream => Self::Stream,
        }
    }
}

/// Each wrapped provider implements this trait to define its CLI mapping.
///
/// The trait moves provider-specific logic (YOLO flags, non-interactive
/// modes, model injection, etc.) into self-contained implementations
/// rather than generic match arms over enum variants.
pub(crate) trait WrapperProfile: Send + Sync {
    /// The `Provider` enum variant for this profile.
    fn provider(&self) -> Provider;

    /// The binary name to exec (e.g. `"claude"`, `"codex"`).
    ///
    /// Defaults to `provider_info(self.provider()).binary`. Per-provider
    /// overrides are unnecessary because the binary name lives in the
    /// central provider catalog.
    fn binary(&self) -> &'static str {
        provider_info(self.provider()).binary
    }

    /// The value injected as the `AGENT` environment variable.
    ///
    /// Defaults to `provider_info(self.provider()).binary`. Every wrapped
    /// provider currently sets `AGENT` to its binary name; the default
    /// matches that convention.
    fn agent_env(&self) -> &'static str {
        provider_info(self.provider()).binary
    }

    // -- YOLO mode ----------------------------------------------------------

    /// Apply YOLO/auto-approve mode to `args` and `env_overrides`.
    ///
    /// Returns a [`YoloOutcome`] capturing both whether the provider's
    /// native bypass mechanism was actually activated on this launch
    /// (`applied`) and any user-facing warning text. `applied` is the
    /// single source of truth the reporter and badge layers should use
    /// — never re-interpret intent (`request.yolo`) as "yolo took
    /// effect", because some launch modes silently suppress it (e.g.
    /// OpenCode TUI rejects `--dangerously-skip-permissions`).
    ///
    /// Default implementation derives behavior from the central provider
    /// catalog's [`YoloSupport`] descriptor. The mode-unaware variant
    /// treats `NonInteractiveOnly` as "always emit warning, do not
    /// apply" — callers in compose/wrap should prefer
    /// [`apply_yolo_for_mode`] which is mode-aware.
    ///
    /// [`apply_yolo_for_mode`]: WrapperProfile::apply_yolo_for_mode
    fn apply_yolo(
        &self,
        args: &mut Vec<String>,
        env_overrides: &mut Vec<(String, String)>,
    ) -> Result<YoloOutcome> {
        match provider_info(self.provider()).yolo {
            YoloSupport::DirectFlag { native_flag } => {
                if !has_flag(args, native_flag) {
                    args.push(native_flag.to_string());
                }
                Ok(YoloOutcome::applied())
            }
            YoloSupport::DirectFlagWithAlias {
                native_flag,
                aliases,
            } => {
                if !has_any_flag(args, native_flag, aliases) {
                    args.push(native_flag.to_string());
                }
                Ok(YoloOutcome::applied())
            }
            YoloSupport::EnvVar { env_var, value } => {
                if !env_overrides
                    .iter()
                    .any(|(k, v)| k == env_var && v == value)
                {
                    env_overrides.push((env_var.to_string(), value.to_string()));
                }
                Ok(YoloOutcome::applied())
            }
            YoloSupport::NonInteractiveOnly { .. } => {
                // Mode-unaware path: we don't know if we're interactive,
                // so the conservative default is to NOT apply and emit
                // the warning. The mode-aware variant below will do the
                // right thing when callers thread `interactive` through.
                Ok(YoloOutcome::not_applied(format!(
                    "{} YOLO mode is non-interactive only",
                    self.provider()
                )))
            }
            YoloSupport::None => Ok(YoloOutcome::not_applied(format!(
                "{} does not support YOLO mode",
                self.provider()
            ))),
        }
    }

    /// Apply YOLO handling with awareness of interactive vs non-interactive
    /// mode.
    ///
    /// For providers whose catalog records [`YoloSupport::NonInteractiveOnly`]
    /// (OpenCode today):
    /// - **Interactive**: do not mutate argv; return `YoloOutcome::not_applied`
    ///   with a warning. The provider's TUI rejects the flag and silently
    ///   continues in default permission mode otherwise.
    /// - **Non-interactive**: push the catalog's `non_interactive_flag` and
    ///   return `YoloOutcome::applied()`.
    ///
    /// All other variants delegate to [`apply_yolo`] (their semantics are
    /// the same in both modes).
    ///
    /// [`apply_yolo`]: WrapperProfile::apply_yolo
    fn apply_yolo_for_mode(
        &self,
        args: &mut Vec<String>,
        env_overrides: &mut Vec<(String, String)>,
        interactive: bool,
    ) -> Result<YoloOutcome> {
        match provider_info(self.provider()).yolo {
            YoloSupport::NonInteractiveOnly {
                non_interactive_flag,
            } => {
                if interactive {
                    return Ok(YoloOutcome::not_applied(format!(
                        "{} YOLO mode is non-interactive only; ignored",
                        self.provider()
                    )));
                }
                if !has_flag(args, non_interactive_flag) {
                    args.push(non_interactive_flag.to_string());
                }
                Ok(YoloOutcome::applied())
            }
            _ => self.apply_yolo(args, env_overrides),
        }
    }

    /// Whether this provider supports YOLO mode at all.
    ///
    /// Defaults to `provider_info(self.provider()).yolo != YoloSupport::None`.
    fn has_supported_yolo(&self) -> bool {
        !matches!(provider_info(self.provider()).yolo, YoloSupport::None)
    }

    /// Reject native YOLO flags passed directly in passthrough args.
    ///
    /// Error messages may contain `<blue>` Prose tags for styled rendering.
    ///
    /// Default implementation rejects [`YoloSupport::DirectFlag`] and
    /// [`YoloSupport::DirectFlagWithAlias`] native flags.
    fn reject_direct_yolo(&self, args: &[String]) -> Result<()> {
        match provider_info(self.provider()).yolo {
            YoloSupport::DirectFlag { native_flag } if has_flag(args, native_flag) => {
                bail!(
                    "do not pass <blue>{native_flag}</blue> directly to claudine {}; \
                     use Claudine's <blue>--yolo</blue> or <blue>-y</blue> switches instead. \
                     Claudine uses this CLI convention for all agents it provides a wrapper to.",
                    self.provider()
                );
            }
            YoloSupport::DirectFlagWithAlias {
                native_flag,
                aliases,
            } if has_any_flag(args, native_flag, aliases) => {
                bail!(
                    "do not pass <blue>{native_flag}</blue> directly to claudine {}; \
                     use Claudine's <blue>--yolo</blue> or <blue>-y</blue> switches instead. \
                     Claudine uses this CLI convention for all agents it provides a wrapper to.",
                    self.provider()
                );
            }
            _ => {}
        }
        Ok(())
    }

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
    /// Default implementation reads from the central catalog's
    /// [`EntrypointSpec`] list.
    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool) {
        let info = provider_info(self.provider());
        let target_mode = if non_interactive {
            EntrypointMode::NonInteractive
        } else {
            EntrypointMode::Interactive
        };
        if let Some(ep) = info
            .entrypoints
            .iter()
            .find(|ep| matches!(ep.mode, EntrypointMode::Both) || ep.mode == target_mode)
        {
            if let Some(sub) = ep.subcommand
                && args.first().is_none_or(|first| first != sub)
            {
                args.insert(0, sub.to_string());
            }
            for flag in ep.required_flags {
                if !has_flag(args, flag) {
                    args.push(flag.to_string());
                }
            }
        }
    }

    /// Reject mode-conflict flags (e.g. `-i` / `--prompt-interactive`)
    /// when the pipeline is running in non-interactive mode. Runs only
    /// in non-interactive pipelines. Providers that do NOT have such
    /// conflicting flags use the default no-op.
    ///
    /// Default: no-op.
    fn apply_non_interactive_flags(&self, _args: &mut [String]) -> Result<()> {
        Ok(())
    }

    fn validate_non_interactive_requirements(&self, _args: &[String]) -> Result<()> {
        Ok(())
    }

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
    /// Default implementation reads from the central catalog's
    /// [`OutputFormatSupport`] list.
    fn apply_output_format(&self, args: &mut Vec<String>, format: OutputFormat) -> Option<String> {
        let info = provider_info(self.provider());
        let provider_format = match format {
            OutputFormat::Json => claudine::provider::OutputFormat::Json,
            OutputFormat::Text => claudine::provider::OutputFormat::Text,
            OutputFormat::Stream => claudine::provider::OutputFormat::Stream,
        };
        let Some(support) = info
            .output_formats
            .iter()
            .find(|s| s.format == provider_format)
        else {
            return Some(format!(
                "{} does not support --output {format}; this flag was skipped",
                self.provider()
            ));
        };

        match support.selector {
            OutputFormatSelector::Default | OutputFormatSelector::Positional { .. } => None,
            OutputFormatSelector::Flag { flag } | OutputFormatSelector::TransportFlag { flag } => {
                if !has_flag(args, flag) {
                    args.push(flag.to_string());
                }
                None
            }
            OutputFormatSelector::FlagValue { flag } => {
                if !has_flag(args, flag)
                    && !args.iter().any(|arg| arg.starts_with(&format!("{flag}=")))
                {
                    args.push(flag.to_string());
                    args.push(support.native_name.to_string());
                }
                None
            }
        }
    }

    // -- Universal --system-prompt flag --------------------------------------

    /// Return the [`SystemPromptSpec`] for this provider.
    ///
    /// Default: reads from the central provider catalog.
    fn system_prompt_spec(&self) -> &'static SystemPromptSpec {
        provider_info(self.provider()).system_prompt
    }

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
        _scoped_tmp: &Path,
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
    ///
    /// Defaults to `provider_info(self.provider()).stream_protocol.is_some()`
    /// so providers with a structured stream protocol declared in the
    /// central catalog inherit the correct value automatically.
    fn supports_structured_stream(&self) -> bool {
        provider_info(self.provider()).stream_protocol.is_some()
    }

    /// The stream protocol this provider uses.
    ///
    /// Defaults to `provider_info(self.provider()).stream_protocol`.
    fn stream_protocol(&self) -> Option<StreamProtocol> {
        provider_info(self.provider()).stream_protocol
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
    ///
    /// Defaults to `provider_info(self.provider()).agent_capabilities().runtime.non_interactive.resume_supported`.
    fn supports_resume(&self) -> bool {
        provider_info(self.provider())
            .agent_capabilities()
            .runtime
            .non_interactive
            .resume_supported
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
    /// remove a prompt from raw passthrough args. The default reads the
    /// provider's `PromptArgConventions` from the central provider
    /// catalog so per-provider overrides are not required for ordinary
    /// cases.
    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        provider_info(self.provider()).prompt_arg_conventions
    }
}

static CLAUDE: ClaudeWrapper = ClaudeWrapper;
static CODEX: CodexWrapper = CodexWrapper;
static GEMINI: GeminiWrapper = GeminiWrapper;
static KIMI: KimiWrapper = KimiWrapper;
static QWEN: QwenWrapper = QwenWrapper;
static OPENCODE: OpencodeWrapper = OpencodeWrapper;
static GOOSE: GooseWrapper = GooseWrapper;

/// Wrapper registry indexed by `Provider as usize`.
///
/// The array length is tied to [`PROVIDER_COUNT`], so adding a new
/// [`Provider`] variant forces a compile error here until the slot is
/// addressed. There is intentionally no wildcard fallback.
///
/// ## Documented "no wrapper" exceptions
///
/// - [`Provider::RooCode`] — Roo Code runs exclusively as a VS Code
///   extension, so there is no standalone CLI binary to wrap. Any future
///   provider that is intentionally unwrapped must be added to this table
///   *and* to the `NO_WRAPPER` allow-list referenced by
///   `wrapper_registry_covers_every_provider_and_documents_exceptions`.
///
/// Slot order MUST match `Provider as usize` — see
/// `claudine::provider::identity::PROVIDERS_DISPLAY_ORDER`.
static WRAPPER_REGISTRY: [Option<&'static dyn WrapperProfile>; PROVIDER_COUNT] = [
    /* 0: Claude   */ Some(&CLAUDE),
    /* 1: Codex    */ Some(&CODEX),
    /* 2: Gemini   */ Some(&GEMINI),
    /* 3: Goose    */ Some(&GOOSE),
    /* 4: KimiCode */ Some(&KIMI),
    /* 5: OpenCode */ Some(&OPENCODE),
    /* 6: QwenCode */ Some(&QWEN),
    /* 7: RooCode  */ None,
];

/// Look up the wrapper profile for a given provider.
///
/// Returns `None` only for the documented "no wrapper" exceptions listed on
/// [`WRAPPER_REGISTRY`].
pub(crate) fn profile_for_provider(provider: Provider) -> Option<&'static dyn WrapperProfile> {
    WRAPPER_REGISTRY[provider as usize]
}

fn non_empty_env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
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

fn push_stream_json_flags(args: &mut Vec<String>, extra: &[&str]) {
    for flag in extra {
        args.push((*flag).to_string());
    }
    args.push("--output-format".to_string());
    args.push("stream-json".to_string());
}

fn prompt_delivery_stdin_or_append(
    prompt: &str,
    non_interactive: bool,
    interactive_flags: &[&str],
) -> PromptDelivery {
    if non_interactive {
        PromptDelivery::Stdin(prompt.to_string())
    } else {
        let mut args = interactive_flags
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        args.push(prompt.to_string());
        PromptDelivery::AppendArgs(args)
    }
}

fn prompt_delivery_append_flags(
    prompt: &str,
    non_interactive: bool,
    non_interactive_flag: &str,
    interactive_flag: &str,
) -> PromptDelivery {
    let flag = if non_interactive {
        non_interactive_flag
    } else {
        interactive_flag
    };
    // When the prompt starts with '-' some CLI parsers (notably yargs,
    // used by Gemini) interpret the value as a flag or end-of-options
    // marker. Using '--flag=value' syntax is unambiguous.
    if prompt.starts_with('-') {
        PromptDelivery::AppendArgs(vec![format!("{flag}={prompt}")])
    } else {
        PromptDelivery::AppendArgs(vec![flag.to_string(), prompt.to_string()])
    }
}

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
        if idx > 0 && args[idx - 1] == "--" {
            args.remove(idx - 1);
        }
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
fn find_positional_prompt_index(args: &[String], conv: &PromptArgConventions) -> Option<usize> {
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

pub(crate) fn validate_argv_flags_before_separator(binary: &str, args: &[String]) {
    if let Some(pos) = args.iter().position(|a| a == "--") {
        for arg in args.iter().skip(pos + 2) {
            if arg.starts_with('-') {
                tracing::warn!(
                    "Flag {:?} appears after -- separator in {} argv: {:?}",
                    arg,
                    binary,
                    args
                );
            }
        }
    }
}

pub(crate) fn apply_opencode_model_resolution(
    child_args: &mut Vec<String>,
    env_setter: &mut dyn FnMut(String, String),
    has_model_env: bool,
    cli_model: Option<&str>,
    non_interactive: bool,
    snapshot: &OpenCodeEnvSnapshot,
) -> Result<Option<OpenCodeModelSource>> {
    if !non_interactive {
        return Ok(None);
    }

    let opencode_model_source = match resolve_opencode_model(cli_model, snapshot) {
        Ok(source) => {
            let model = source.model().to_string();
            match &source {
                OpenCodeModelSource::CliSwitch(_) | OpenCodeModelSource::OpenCodeModelEnv(_) => {
                    if !has_flag(child_args, "--model") && !has_flag(child_args, "-m") {
                        child_args.push("--model".to_string());
                        child_args.push(model.clone());
                    }
                    env_setter("MODEL".to_string(), model);
                }
                OpenCodeModelSource::ConfigDefault(_) => {
                    env_setter("MODEL".to_string(), model);
                }
            }
            Some(source)
        }
        Err(NoModelProvided) => None,
    };

    let has_model_arg = has_flag(child_args, "--model") || has_flag(child_args, "-m");
    if !has_model_arg && !has_model_env && opencode_model_source.is_none() {
        return Err(eyre!(
            "No model specified! OpenCode by default does not specify a model but you can\n\
             change this behavior by adding a model property to ~/.config/opencode/config.json.\n\
             You can override/set the default model with any of the following methods:\n\n\
             \x20\x20• set OPENCODE_MODEL to a valid model name\n\
             \x20\x20• use the CLI switch --model <model>\n\n\
             Running `opencode models` will give you a list of all valid models."
        ));
    }

    Ok(opencode_model_source)
}

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

    /// When a prompt starts with '-' some CLI parsers (notably yargs,
    /// used by Gemini) interpret the value as a flag or end-of-options
    /// marker, causing '--prompt ---...' to fail with "Not enough
    /// arguments following: prompt". The fix uses '--prompt=---...'
    /// syntax when the prompt starts with '-'.
    #[test]
    fn gemini_prompt_delivery_uses_equals_syntax_when_prompt_starts_with_dash() {
        let p = profile(Provider::Gemini);
        let args = Vec::new();
        let delivery = p.prompt_delivery(&args, "---\nfoo", true).unwrap();
        let mut applied = Vec::new();
        delivery.apply_to(&mut applied);
        assert_eq!(applied, vec!["--prompt=---\nfoo"]);
    }

    #[test]
    fn gemini_prompt_delivery_uses_space_syntax_for_normal_prompt() {
        let p = profile(Provider::Gemini);
        let args = Vec::new();
        let delivery = p.prompt_delivery(&args, "hello world", true).unwrap();
        let mut applied = Vec::new();
        delivery.apply_to(&mut applied);
        assert_eq!(applied, vec!["--prompt", "hello world"]);
    }

    #[test]
    fn qwen_prompt_delivery_uses_equals_syntax_when_prompt_starts_with_dash() {
        let p = profile(Provider::QwenCode);
        let args = Vec::new();
        let delivery = p.prompt_delivery(&args, "---\nfoo", true).unwrap();
        let mut applied = Vec::new();
        delivery.apply_to(&mut applied);
        assert_eq!(applied, vec!["--prompt=---\nfoo"]);
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
    fn opencode_noise_prefixes_cover_captured_symptoms() {
        let noise = opencode_default_tui_noise_prefixes();

        // Representative lines taken verbatim from
        // claudine/claudine-output/opencode.err (2026-04-14 capture).
        let symptoms = [
            r#"✱ Glob "**/claudine/**/improved-sequences/**" 2 matches"#,
            r#"$ cd /tmp && git log --all --oneline"#,
            r#"> build · MiniMax-M2.7-highspeed"#,
            r#"████ Subprocess hygiene"#,
            "\u{2699} firecrawl_firecrawl_search {\"query\":\"NFL draft 2026 date\",\"limit\":5}",
        ];

        for line in symptoms {
            assert!(
                noise.iter().any(|p| line.starts_with(p)),
                "noise prefixes must match representative line: {line}"
            );
        }
    }

    #[test]
    fn opencode_profile_advertises_default_tui_noise_prefixes() {
        let profile = profile(Provider::OpenCode);
        let prefixes: &[&str] = profile.stderr_noise_prefixes();
        assert!(
            prefixes.contains(&"\u{2731} "),
            "OpenCode profile must expose the default TUI noise prefixes; got {prefixes:?}"
        );
    }

    #[test]
    fn opencode_yolo_interactive_warns_without_mutating_args() {
        // The mode-aware variant in interactive mode must report
        // `applied = false`, emit the refined warning, and NOT mutate argv.
        let p = profile(Provider::OpenCode);
        let mut args = vec!["run".to_string(), "status".to_string()];
        let mut env_overrides = Vec::new();

        let outcome = p
            .apply_yolo_for_mode(&mut args, &mut env_overrides, /* interactive = */ true)
            .unwrap();
        assert!(
            !outcome.applied,
            "interactive mode must report applied=false; got {outcome:?}",
        );
        assert_eq!(
            outcome.warning.as_deref(),
            Some(
                "--yolo mode is not supported in OpenCode <i>interactive</i> sessions and was ignored"
            ),
        );
        assert_eq!(args, vec!["run", "status"]);
    }

    #[test]
    fn opencode_yolo_non_interactive_forwards_dangerously_skip_permissions() {
        let mut args: Vec<String> = vec!["run".to_string()];
        let mut env = Vec::new();
        let wrapper = OpencodeWrapper;
        let outcome = wrapper
            .apply_yolo_for_mode(&mut args, &mut env, /* interactive = */ false)
            .unwrap();
        assert!(
            outcome.applied,
            "non-interactive mode must report applied=true; got {outcome:?}",
        );
        assert!(
            args.iter().any(|a| a == "--dangerously-skip-permissions"),
            "flag must be forwarded in non-interactive mode; args={args:?}"
        );
        assert!(
            outcome.warning.is_none(),
            "no warning expected in non-interactive: got {outcome:?}"
        );
    }

    #[test]
    fn opencode_yolo_interactive_emits_refined_warning_only() {
        let mut args: Vec<String> = vec![];
        let mut env = Vec::new();
        let wrapper = OpencodeWrapper;
        let outcome = wrapper
            .apply_yolo_for_mode(&mut args, &mut env, /* interactive = */ true)
            .unwrap();
        assert!(
            !outcome.applied,
            "interactive mode must report applied=false; got {outcome:?}",
        );
        assert_eq!(
            outcome.warning.as_deref(),
            Some(
                "--yolo mode is not supported in OpenCode <i>interactive</i> sessions and was ignored"
            ),
        );
        assert!(
            args.is_empty(),
            "no args should be added in interactive mode"
        );
    }

    #[test]
    fn opencode_yolo_non_interactive_idempotent() {
        let mut args: Vec<String> = vec!["--dangerously-skip-permissions".to_string()];
        let mut env = Vec::new();
        let wrapper = OpencodeWrapper;
        wrapper
            .apply_yolo_for_mode(&mut args, &mut env, false)
            .unwrap();
        let count = args
            .iter()
            .filter(|a| *a == "--dangerously-skip-permissions")
            .count();
        assert_eq!(count, 1, "flag must not be duplicated");
    }

    // -- resolve_opencode_model tests ----------------------------------------

    #[test]
    fn opencode_resolve_cli_switch_when_model_provided() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: None,
            opencode_config_model: None,
        };
        let source = resolve_opencode_model(Some("cli-model"), &snapshot).unwrap();
        assert_eq!(
            source,
            OpenCodeModelSource::CliSwitch("cli-model".to_string())
        );
    }

    #[test]
    fn opencode_resolve_env_var_when_no_cli_switch() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: Some("env-model".to_string()),
            opencode_config_model: None,
        };
        let source = resolve_opencode_model(None, &snapshot).unwrap();
        assert_eq!(
            source,
            OpenCodeModelSource::OpenCodeModelEnv("env-model".to_string())
        );
    }

    #[test]
    fn opencode_resolve_config_default_when_json_has_model() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: None,
            opencode_config_model: Some("config-model".to_string()),
        };
        let source = resolve_opencode_model(None, &snapshot).unwrap();
        assert_eq!(
            source,
            OpenCodeModelSource::ConfigDefault("config-model".to_string())
        );
    }

    #[test]
    fn opencode_resolve_err_no_model_provided_when_none_available() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: None,
            opencode_config_model: None,
        };
        let result = resolve_opencode_model(None, &snapshot);
        assert_eq!(result, Err(NoModelProvided));
    }

    #[test]
    fn opencode_resolve_precedence_cli_over_env() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: Some("env-model".to_string()),
            opencode_config_model: Some("config-model".to_string()),
        };
        let source = resolve_opencode_model(Some("cli-model"), &snapshot).unwrap();
        assert_eq!(
            source,
            OpenCodeModelSource::CliSwitch("cli-model".to_string())
        );
    }

    #[test]
    fn opencode_resolve_precedence_env_over_config() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: Some("env-model".to_string()),
            opencode_config_model: Some("config-model".to_string()),
        };
        let source = resolve_opencode_model(None, &snapshot).unwrap();
        assert_eq!(
            source,
            OpenCodeModelSource::OpenCodeModelEnv("env-model".to_string())
        );
    }

    #[test]
    fn opencode_resolve_model_env_var_ignored_entirely() {
        // This test was to ensure `MODEL` env is ignored and only `OPENCODE_MODEL` is checked
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: None,
            opencode_config_model: None,
        };
        let result = resolve_opencode_model(None, &snapshot);
        assert_eq!(result, Err(NoModelProvided));
    }

    #[test]
    fn opencode_resolve_malformed_config_json_yields_no_model() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: None,
            opencode_config_model: None,
        };
        let result = resolve_opencode_model(None, &snapshot);
        assert_eq!(result, Err(NoModelProvided));
    }

    #[test]
    fn opencode_resolve_missing_config_file_yields_no_model() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: None,
            opencode_config_model: None,
        };
        let result = resolve_opencode_model(None, &snapshot);
        assert_eq!(result, Err(NoModelProvided));
    }

    #[test]
    fn opencode_resolve_empty_string_model_yields_no_model() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: None,
            opencode_config_model: None,
        };
        let result = resolve_opencode_model(None, &snapshot);
        assert_eq!(result, Err(NoModelProvided));
    }

    #[test]
    fn opencode_model_source_location_strings() {
        assert_eq!(
            OpenCodeModelSource::CliSwitch(String::new()).location_string(),
            "the --model CLI switch"
        );
        assert_eq!(
            OpenCodeModelSource::OpenCodeModelEnv(String::new()).location_string(),
            "the OPENCODE_MODEL environment variable"
        );
        assert_eq!(
            OpenCodeModelSource::ConfigDefault(String::new()).location_string(),
            "the config file ~/.config/opencode/config.json"
        );
    }

    #[test]
    fn opencode_no_model_provided_display() {
        assert_eq!(NoModelProvided.to_string(), "no model provided");
    }

    #[test]
    fn opencode_apply_to_args_cli_switch_pushes_model_flag_and_env() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: None,
            opencode_config_model: None,
        };
        let mut args = vec!["run".to_string()];
        let mut env = Vec::new();
        apply_opencode_model_resolution(
            &mut args,
            &mut |k, v| env.push((k, v)),
            false,
            Some("gpt-4o"),
            true,
            &snapshot,
        )
        .unwrap();
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"gpt-4o".to_string()));
        assert!(env.contains(&("MODEL".to_string(), "gpt-4o".to_string())));
    }

    #[test]
    fn opencode_apply_to_args_env_var_pushes_model_flag_and_env() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: Some("env-model".to_string()),
            opencode_config_model: None,
        };
        let mut args = vec!["run".to_string()];
        let mut env = Vec::new();
        apply_opencode_model_resolution(
            &mut args,
            &mut |k, v| env.push((k, v)),
            false,
            None,
            true,
            &snapshot,
        )
        .unwrap();
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"env-model".to_string()));
        assert!(env.contains(&("MODEL".to_string(), "env-model".to_string())));
    }

    #[test]
    fn opencode_apply_to_args_config_default_pushes_env_only() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: None,
            opencode_config_model: Some("config-model".to_string()),
        };
        let mut args = vec!["run".to_string()];
        let mut env = Vec::new();
        apply_opencode_model_resolution(
            &mut args,
            &mut |k, v| env.push((k, v)),
            false,
            None,
            true,
            &snapshot,
        )
        .unwrap();
        assert!(!args.contains(&"--model".to_string()));
        assert!(env.contains(&("MODEL".to_string(), "config-model".to_string())));
    }

    #[test]
    fn opencode_apply_to_args_does_not_duplicate_existing_model_flag() {
        let snapshot = OpenCodeEnvSnapshot {
            opencode_model_env: None,
            opencode_config_model: None,
        };
        let mut args = vec![
            "run".to_string(),
            "--model".to_string(),
            "existing".to_string(),
        ];
        let mut env = Vec::new();
        apply_opencode_model_resolution(
            &mut args,
            &mut |k, v| env.push((k, v)),
            false,
            Some("existing"),
            true,
            &snapshot,
        )
        .unwrap();
        let count = args.iter().filter(|a| *a == "--model").count();
        assert_eq!(count, 1, "should not duplicate --model flag");
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
        assert_eq!(args, vec!["run", "--", "summarize staged files"]);
    }

    #[test]
    fn opencode_non_interactive_prompt_starting_with_dash_is_separated_with_end_of_options() {
        // Regression: OpenCode's yargs parser prints help and exits when a
        // positional prompt begins with `-`. Claudine must emit `--` before
        // the prompt so composed bullet-list prompts are delivered intact.
        let p = profile(Provider::OpenCode);
        let mut args = vec![
            "run".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        p.prompt_delivery(&args, "- implement the plan\n- use the skill", true)
            .unwrap()
            .apply_to(&mut args);
        let sep_index = args
            .iter()
            .position(|a| a == "--")
            .expect("`--` separator must be present");
        let prompt_index = args
            .iter()
            .position(|a| a == "- implement the plan\n- use the skill")
            .expect("prompt must be present as a positional");
        assert!(
            sep_index < prompt_index,
            "`--` must precede the prompt: {args:?}"
        );
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

    /// Documented "no wrapper" providers. Must stay in lock-step with the
    /// `None` slots in [`WRAPPER_REGISTRY`].
    const NO_WRAPPER: &[Provider] = &[Provider::RooCode];

    #[test]
    fn roo_code_has_no_wrapper_profile() {
        assert!(profile_for_provider(Provider::RooCode).is_none());
    }

    /// Phase 3 invariant: the array-backed [`WRAPPER_REGISTRY`] either
    /// returns a wrapper whose `provider()` matches the lookup key, or
    /// returns `None` for a provider explicitly listed in [`NO_WRAPPER`].
    /// A future [`Provider`] variant fails compilation at the array
    /// declaration; this test then guards the `None`/`Some` decision.
    #[test]
    fn wrapper_registry_covers_every_provider_and_documents_exceptions() {
        use claudine::provider::PROVIDERS_DISPLAY_ORDER;

        for provider in PROVIDERS_DISPLAY_ORDER {
            let result = profile_for_provider(provider);
            if NO_WRAPPER.contains(&provider) {
                assert!(
                    result.is_none(),
                    "{provider:?}: documented as no-wrapper but registry returned Some"
                );
            } else {
                let profile = result.unwrap_or_else(|| {
                    panic!(
                        "{provider:?}: registry must provide a wrapper unless explicitly \
                         listed in NO_WRAPPER"
                    )
                });
                assert_eq!(
                    profile.provider(),
                    provider,
                    "{provider:?}: registry slot returned a profile for the wrong provider"
                );
            }
        }
    }

    /// The registry array length is wired to [`PROVIDER_COUNT`], so adding
    /// a new [`Provider`] variant forces a compile error at the declaration.
    /// This runtime assertion documents the invariant for readers and
    /// catches accidental drift if the static is ever rebuilt by hand.
    #[test]
    fn wrapper_registry_length_matches_provider_count() {
        assert_eq!(WRAPPER_REGISTRY.len(), PROVIDER_COUNT);
    }

    fn provider_output_format(format: OutputFormat) -> claudine::provider::OutputFormat {
        match format {
            OutputFormat::Json => claudine::provider::OutputFormat::Json,
            OutputFormat::Text => claudine::provider::OutputFormat::Text,
            OutputFormat::Stream => claudine::provider::OutputFormat::Stream,
        }
    }

    fn catalog_output_args(provider: Provider, format: OutputFormat) -> Option<Vec<String>> {
        let support = provider_info(provider)
            .output_formats
            .iter()
            .find(|support| support.format == provider_output_format(format))?;

        let args = match support.selector {
            OutputFormatSelector::Default | OutputFormatSelector::Positional { .. } => Vec::new(),
            OutputFormatSelector::Flag { flag } | OutputFormatSelector::TransportFlag { flag } => {
                vec![flag.to_string()]
            }
            OutputFormatSelector::FlagValue { flag } => {
                vec![flag.to_string(), support.native_name.to_string()]
            }
        };
        Some(args)
    }

    #[test]
    fn apply_output_format_matches_provider_catalog_for_every_provider() {
        for provider in claudine::provider::PROVIDERS_DISPLAY_ORDER {
            let Some(profile) = profile_for_provider(provider) else {
                continue;
            };
            for format in [OutputFormat::Json, OutputFormat::Text, OutputFormat::Stream] {
                let mut args = Vec::new();
                let warning = profile.apply_output_format(&mut args, format);
                match catalog_output_args(provider, format) {
                    Some(expected) => {
                        assert!(
                            warning.is_none(),
                            "{provider:?} should support cataloged output format {format}"
                        );
                        assert_eq!(args, expected, "{provider:?} --output {format}");
                    }
                    None => {
                        assert!(
                            warning.is_some(),
                            "{provider:?} should warn for uncataloged output format {format}"
                        );
                        assert!(args.is_empty(), "{provider:?} --output {format}");
                    }
                }
            }
        }
    }

    fn catalog_entrypoint_args(provider: Provider, non_interactive: bool) -> Vec<String> {
        let info = provider_info(provider);
        let target_mode = if non_interactive {
            EntrypointMode::NonInteractive
        } else {
            EntrypointMode::Interactive
        };
        let mut args = Vec::new();
        if let Some(ep) = info
            .entrypoints
            .iter()
            .find(|ep| matches!(ep.mode, EntrypointMode::Both) || ep.mode == target_mode)
        {
            if let Some(sub) = ep.subcommand {
                args.push(sub.to_string());
            }
            args.extend(ep.required_flags.iter().map(|flag| (*flag).to_string()));
        }
        args
    }

    #[test]
    fn apply_entrypoint_matches_provider_catalog_for_every_provider() {
        for provider in claudine::provider::PROVIDERS_DISPLAY_ORDER {
            let Some(profile) = profile_for_provider(provider) else {
                continue;
            };
            let mut args = Vec::new();
            profile.apply_entrypoint(&mut args, true);
            assert_eq!(
                args,
                catalog_entrypoint_args(provider, true),
                "{provider:?} non-interactive entrypoint"
            );
        }
    }

    fn catalog_yolo_applied(
        provider: Provider,
        args: &[String],
        env: &[(String, String)],
        outcome: &YoloOutcome,
    ) -> bool {
        match provider_info(provider).yolo {
            YoloSupport::None => {
                !outcome.applied && outcome.warning.is_some() && args.is_empty() && env.is_empty()
            }
            YoloSupport::DirectFlag { native_flag } => {
                outcome.applied && yolo_flag_present(args, native_flag)
            }
            YoloSupport::DirectFlagWithAlias { native_flag, .. } => {
                outcome.applied && yolo_flag_present(args, native_flag)
            }
            YoloSupport::NonInteractiveOnly {
                non_interactive_flag,
            } => outcome.applied && args.iter().any(|arg| arg == non_interactive_flag),
            YoloSupport::EnvVar { env_var, value } => {
                outcome.applied && env.iter().any(|(k, v)| k == env_var && v == value)
            }
        }
    }

    fn yolo_flag_present(args: &[String], native_flag: &str) -> bool {
        if args.iter().any(|arg| arg == native_flag) {
            return true;
        }
        let Some((flag, value)) = native_flag.split_once('=') else {
            return false;
        };
        args.windows(2)
            .any(|window| window[0] == flag && window[1] == value)
    }

    /// Regression: the `--yolo` CLI flag and the `CLAUDINE_YOLO` env var
    /// must reach the SAME field (`request.yolo`) so they cannot diverge.
    /// Before this binding, the env var was only read by the dispatch
    /// reporter for metadata stamping while the wrapper's flag-push gate
    /// looked at the CLI arg alone — producing reports of "yolo: true"
    /// for sessions where the provider flag never actually landed.
    ///
    /// Test mutates process env, so it must be serialized against any
    /// other test that reads `CLAUDINE_YOLO`. We snapshot/restore here
    /// rather than relying on a test mutex — the env var is uncommon
    /// enough that parallel parsing collisions are unlikely.
    #[test]
    fn claudine_yolo_env_binds_to_compose_shared_yolo_flag() {
        use clap::Parser;

        use crate::commands::compose::SharedComposeArgs;

        // Probe shim: `SharedComposeArgs` is `Args`, not `Parser`. Wrap
        // it in a derive(Parser) container so `try_parse_from` is callable.
        #[derive(Debug, clap::Parser)]
        struct Probe {
            #[command(flatten)]
            shared: SharedComposeArgs,
        }

        let prior = std::env::var("CLAUDINE_YOLO").ok();
        // SAFETY: single-threaded scoped mutation. We restore the prior
        // value below so sibling tests are unaffected.
        unsafe {
            std::env::remove_var("CLAUDINE_YOLO");
        }
        let baseline = Probe::try_parse_from(["probe"])
            .expect("probe must parse without flag or env")
            .shared
            .yolo;

        let with_flag = Probe::try_parse_from(["probe", "-y"])
            .expect("probe with -y must parse")
            .shared
            .yolo;

        unsafe {
            std::env::set_var("CLAUDINE_YOLO", "true");
        }
        let from_env = Probe::try_parse_from(["probe"])
            .expect("probe with CLAUDINE_YOLO=true must parse")
            .shared
            .yolo;

        // Restore prior env so we don't leak state.
        unsafe {
            match prior {
                Some(value) => std::env::set_var("CLAUDINE_YOLO", value),
                None => std::env::remove_var("CLAUDINE_YOLO"),
            }
        }

        assert!(
            !baseline,
            "no flag and no env must leave yolo=false (got {baseline})",
        );
        assert!(with_flag, "-y on argv must enable yolo (got {with_flag})",);
        assert!(
            from_env,
            "CLAUDINE_YOLO=true must enable yolo on SharedComposeArgs (got {from_env})",
        );
    }

    #[test]
    fn apply_yolo_matches_provider_catalog_for_every_provider() {
        for provider in claudine::provider::PROVIDERS_DISPLAY_ORDER {
            let Some(profile) = profile_for_provider(provider) else {
                continue;
            };
            let mut args = Vec::new();
            let mut env = Vec::new();
            let outcome = profile
                .apply_yolo_for_mode(&mut args, &mut env, false)
                .expect("YOLO application should not fail from empty args");

            assert!(
                catalog_yolo_applied(provider, &args, &env, &outcome),
                "{provider:?} yolo mismatch: args={args:?}, env={env:?}, outcome={outcome:?}"
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
    fn kimi_non_interactive_uses_wire_protocol_and_wire_rpc_delivery() {
        let p = profile(Provider::KimiCode);
        assert_eq!(p.stream_protocol(), Some(StreamProtocol::WireJsonRpc));

        let mut args: Vec<String> = Vec::new();
        p.apply_entrypoint(&mut args, true);
        assert!(args.contains(&"--wire".to_string()));
        assert!(!args.contains(&"--print".to_string()));

        let mut structured_args: Vec<String> = Vec::new();
        p.apply_structured_stream(&mut structured_args);
        assert_eq!(structured_args, vec!["--wire".to_string()]);

        let delivery = p
            .prompt_delivery(&args, "hello kimi", true)
            .expect("kimi prompt_delivery should succeed");
        match delivery {
            PromptDelivery::WireRpc(prompt) => assert_eq!(prompt, "hello kimi"),
            other => panic!("expected WireRpc delivery, got {other:?}"),
        }
    }

    #[test]
    fn kimi_interactive_continues_using_prompt_argv_flag() {
        let p = profile(Provider::KimiCode);
        let mut args: Vec<String> = Vec::new();
        p.apply_entrypoint(&mut args, false);
        assert!(args.is_empty(), "interactive must not append --wire");

        let delivery = p
            .prompt_delivery(&args, "hello", false)
            .expect("kimi prompt_delivery should succeed in interactive mode");
        match delivery {
            PromptDelivery::AppendArgs(extra) => {
                assert_eq!(extra, vec!["--prompt".to_string(), "hello".to_string()]);
            }
            other => panic!("expected AppendArgs delivery, got {other:?}"),
        }
    }

    #[test]
    fn kimi_resume_uses_wire_flag() {
        let p = profile(Provider::KimiCode);
        let resume = p.build_resume_args("session-123").unwrap();
        assert_eq!(
            resume,
            vec![
                "kimi".to_string(),
                "--resume".to_string(),
                "session-123".to_string(),
                "--wire".to_string(),
            ]
        );
        assert!(!resume.contains(&"--print".to_string()));
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
        let (args, source) = extract(Provider::Claude, &["--model", "opus", "fix the bug"], false);
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
        let (args, source) = extract(Provider::Gemini, &["--prompt", "hi"], false);
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
        let (args, source) = extract(Provider::Gemini, &["--prompt=hi"], false);
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
        let (args, source) = extract(Provider::Goose, &["run", "-t", "hello"], false);
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
        let (args, source) = extract(Provider::OpenCode, &["run", "build it"], false);
        assert_eq!(args, vec!["run"]);
        assert_eq!(source, PromptSource::Inline("build it".to_string()));
    }

    #[test]
    fn extract_qwen_long_prompt_flag() {
        let (args, source) = extract(Provider::QwenCode, &["--prompt", "hi"], false);
        assert!(args.is_empty());
        assert_eq!(source, PromptSource::Inline("hi".to_string()));
    }

    #[test]
    fn extract_flags_only_returns_none_when_no_piped_stdin() {
        let (args, source) = extract(Provider::Codex, &["exec", "--json"], false);
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
        let err = extract_prompt_source_from_passthrough(profile(Provider::Gemini), &args, false)
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
        let (args, source) = extract(Provider::Claude, &["KEY=VALUE", "the actual prompt"], false);
        assert_eq!(args, vec!["the actual prompt".to_string()]);
        assert_eq!(source, PromptSource::Inline("KEY=VALUE".to_string()));
    }

    // -- require_prompt_present tests -----------------------------------------

    #[test]
    fn require_prompt_present_passes_in_interactive_mode_with_no_source() {
        require_prompt_present("claude", false, &PromptSource::None).unwrap();
    }

    #[test]
    fn require_prompt_present_passes_with_inline_prompt() {
        require_prompt_present("claude", true, &PromptSource::Inline("x".to_string())).unwrap();
    }

    #[test]
    fn require_prompt_present_passes_with_inherit_stdin() {
        require_prompt_present("claude", true, &PromptSource::InheritStdin).unwrap();
    }

    #[test]
    fn require_prompt_present_fails_non_interactive_with_no_source() {
        let err = require_prompt_present("codex", true, &PromptSource::None).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("codex"));
        assert!(message.contains("requires a prompt"));
    }
    // -- Pipeline Order Regression Tests (Issue 2026-04-15) -----------------

    fn run_direct_wrap_pipeline_simulation(
        provider: Provider,
        cli_args: &[&str],
        prompt: &str,
    ) -> Vec<String> {
        let profile = profile(provider);
        let mut child_args: Vec<String> = cli_args.iter().map(|s| s.to_string()).collect();
        let mut env_overrides: Vec<(String, String)> = Vec::new();

        // 1. apply_yolo
        let _ = profile.apply_yolo_for_mode(&mut child_args, &mut env_overrides, false);
        // 2. apply_entrypoint
        profile.apply_entrypoint(&mut child_args, true);
        // 3. apply_non_interactive
        let _ = profile.apply_non_interactive_flags(&mut child_args);

        // 4. Model resolution (specifically OpenCode simulation)
        if provider == Provider::OpenCode {
            let snapshot = OpenCodeEnvSnapshot {
                opencode_model_env: None,
                opencode_config_model: None,
            };
            let _ = apply_opencode_model_resolution(
                &mut child_args,
                &mut |k, v| env_overrides.push((k, v)),
                false,
                Some("test-model"),
                true,
                &snapshot,
            );
        }

        // 5. Output format (simulation of --format stream-json)
        let _ = profile.apply_output_format(&mut child_args, OutputFormat::Stream);

        // 6. apply_structured_stream
        profile.apply_structured_stream(&mut child_args);

        // 7. prompt_delivery (NEW CORRECT ORDER)
        let _ = profile
            .prompt_delivery(&child_args, prompt, true)
            .unwrap()
            .apply_to(&mut child_args);

        child_args
    }

    #[test]
    fn test_opencode_non_interactive_args_order() {
        let args = run_direct_wrap_pipeline_simulation(Provider::OpenCode, &[], "do the thing");

        // We want to verify that flags appear before any positional arguments,
        // specifically before the `--` separator if there is one.
        if let Some(pos) = args.iter().position(|a| a == "--") {
            for arg in args.iter().skip(pos + 2) {
                assert!(
                    !arg.starts_with('-'),
                    "Flag {:?} appears after -- separator in argv: {:?}",
                    arg,
                    args
                );
            }
        } else {
            // No separator, check the end
        }
    }

    #[test]
    fn test_goose_non_interactive_no_duplicate_run() {
        let args = run_direct_wrap_pipeline_simulation(Provider::Goose, &[], "run this");
        let run_count = args.iter().filter(|a| *a == "run").count();
        assert_eq!(
            run_count, 1,
            "Goose pipeline should contain exactly one 'run' entrypoint, found: {:?}",
            args
        );
    }

    #[test]
    fn test_all_providers_flags_before_double_dash() {
        for provider in [
            Provider::Claude,
            Provider::Codex,
            Provider::Gemini,
            Provider::KimiCode,
            Provider::QwenCode,
            Provider::OpenCode,
            Provider::Goose,
        ] {
            let args = run_direct_wrap_pipeline_simulation(
                provider,
                &[],
                "some generic prompt --with-flag",
            );
            if let Some(pos) = args.iter().position(|a| a == "--") {
                for arg in &args[pos + 1..] {
                    if arg != "some generic prompt --with-flag" {
                        assert!(
                            !arg.starts_with('-'),
                            "[{:?}] Flag {:?} appears after -- separator in argv: {:?}",
                            provider,
                            arg,
                            args
                        );
                    }
                }
            }
        }
    }

    // -- has_explicit_native_output_request tests (Phase 3, Task 4) ---------

    fn detect_native_output(provider: Provider, args: &[&str]) -> bool {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        super::super::has_explicit_native_output_request(provider, &args)
    }

    #[test]
    fn codex_json_flag_disables_structured_parsing() {
        assert!(detect_native_output(Provider::Codex, &["--json"]));
    }

    #[test]
    fn claude_output_format_json_disables_structured_parsing() {
        assert!(detect_native_output(
            Provider::Claude,
            &["--output-format", "json"]
        ));
    }

    #[test]
    fn claude_output_format_equals_stream_json_disables_structured_parsing() {
        assert!(detect_native_output(
            Provider::Claude,
            &["--output-format=stream-json"]
        ));
    }

    #[test]
    fn gemini_output_format_json_disables_structured_parsing() {
        assert!(detect_native_output(
            Provider::Gemini,
            &["--output-format", "json"]
        ));
    }

    #[test]
    fn gemini_output_format_equals_stream_json_disables_structured_parsing() {
        assert!(detect_native_output(
            Provider::Gemini,
            &["--output-format=stream-json"]
        ));
    }

    #[test]
    fn qwen_output_format_json_disables_structured_parsing() {
        assert!(detect_native_output(
            Provider::QwenCode,
            &["--output-format", "json"]
        ));
    }

    #[test]
    fn qwen_output_format_equals_stream_json_disables_structured_parsing() {
        assert!(detect_native_output(
            Provider::QwenCode,
            &["--output-format=stream-json"]
        ));
    }

    #[test]
    fn opencode_format_json_disables_structured_parsing() {
        assert!(detect_native_output(
            Provider::OpenCode,
            &["--format", "json"]
        ));
    }

    #[test]
    fn opencode_format_equals_json_disables_structured_parsing() {
        assert!(detect_native_output(Provider::OpenCode, &["--format=json"]));
    }

    #[test]
    fn kimi_wire_transport_flag_does_not_count_as_native_output_request() {
        assert!(!detect_native_output(Provider::KimiCode, &["--wire"]));
    }

    #[test]
    fn unknown_provider_flags_do_not_match() {
        assert!(!detect_native_output(Provider::Claude, &["--unknown-flag"]));
        assert!(!detect_native_output(Provider::Codex, &["--unknown-flag"]));
        assert!(!detect_native_output(
            Provider::OpenCode,
            &["--unknown-flag"]
        ));
    }

    #[test]
    fn goose_no_output_flags_always_false() {
        assert!(!detect_native_output(Provider::Goose, &[]));
        assert!(!detect_native_output(
            Provider::Goose,
            &["--some-random-flag"]
        ));
    }

    #[test]
    fn empty_args_always_false() {
        for provider in claudine::provider::PROVIDERS_DISPLAY_ORDER {
            assert!(
                !detect_native_output(provider, &[]),
                "{provider:?}: empty args should not trigger native output detection"
            );
        }
    }

    #[test]
    fn flags_after_double_dash_separator_are_ignored() {
        assert!(!detect_native_output(Provider::Codex, &["--", "--json"]));
        assert!(!detect_native_output(
            Provider::Claude,
            &["--", "--output-format", "json"]
        ));
    }

    // -- Parity: explicit_native_output_detection_matches_provider_catalog (Phase 3, Task 7)

    #[test]
    fn explicit_native_output_detection_matches_provider_catalog() {
        for provider in claudine::provider::PROVIDERS_DISPLAY_ORDER {
            let info = provider_info(provider);
            for support in info.output_formats {
                match support.selector {
                    OutputFormatSelector::Flag { flag } => {
                        let args: Vec<String> = vec![flag.to_string()];
                        assert!(
                            super::super::has_explicit_native_output_request(provider, &args),
                            "{provider:?}: catalog Flag {flag} must be detected as native output"
                        );
                    }
                    OutputFormatSelector::FlagValue { flag } => {
                        let separated: Vec<String> =
                            vec![flag.to_string(), support.native_name.to_string()];
                        assert!(
                            super::super::has_explicit_native_output_request(provider, &separated),
                            "{provider:?}: catalog FlagValue {flag} {} must be detected as native output",
                            support.native_name
                        );
                        let inline: Vec<String> = vec![format!("{flag}={}", support.native_name)];
                        assert!(
                            super::super::has_explicit_native_output_request(provider, &inline),
                            "{provider:?}: catalog FlagValue {flag}={} (inline) must be detected as native output",
                            support.native_name
                        );
                    }
                    OutputFormatSelector::TransportFlag { flag } => {
                        let args: Vec<String> = vec![flag.to_string()];
                        assert!(
                            !super::super::has_explicit_native_output_request(provider, &args),
                            "{provider:?}: catalog TransportFlag {flag} must NOT be detected as native output"
                        );
                    }
                    OutputFormatSelector::Default | OutputFormatSelector::Positional { .. } => {}
                }
            }
        }
    }

    /// Source guard: the default `apply_output_format` and `apply_entrypoint`
    /// implementations must read from the catalog. If a future change adds
    /// a raw `match provider` block inside either default method body,
    /// this test catches it. The `has_explicit_native_output_request`
    /// helper has its own source guard in `wrap::flags`.
    #[test]
    fn output_detection_and_application_uses_catalog_not_raw_dispatch() {
        let source = include_str!("mod.rs");

        for fn_signature in ["fn apply_output_format(", "fn apply_entrypoint("] {
            let start = source
                .find(fn_signature)
                .unwrap_or_else(|| panic!("{fn_signature} default impl should exist in mod.rs"));
            let end = source[start..]
                .find("\n    fn ")
                .or_else(|| source[start..].find("\n    /// "))
                .map(|offset| start + offset)
                .unwrap_or(source.len());
            let body = &source[start..end];

            assert!(
                body.contains("provider_info(self.provider())"),
                "{fn_signature} default must read from provider catalog: {body}"
            );

            for variant in [
                "Provider::Claude",
                "Provider::Codex",
                "Provider::Gemini",
                "Provider::KimiCode",
                "Provider::OpenCode",
                "Provider::QwenCode",
                "Provider::Goose",
            ] {
                assert!(
                    !body.contains(variant),
                    "{fn_signature} default must not contain {variant} — use catalog data"
                );
            }
        }
    }
}
