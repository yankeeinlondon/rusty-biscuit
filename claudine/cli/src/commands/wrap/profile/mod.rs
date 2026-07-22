use std::path::Path;

use claudine::provider::Provider;
use claudine::provider::SystemPromptSpec;
use claudine::provider::{
    PROVIDER_COUNT, PromptArgConventions, ResumeSupport, YoloSupport, provider_info,
};
use claudine::stream::StreamProtocol;
use claudine::system_prompt::PreparedSystemPrompt;
use color_eyre::eyre::{Result, bail};

// The native-output and positional test suites (`tests/*.rs`) reconstruct the
// catalog matching logic and reach these enums through `use super::super::*`.
// The production bodies that referenced them moved into `apply`, so the imports
// are test-only.
#[cfg(test)]
use claudine::provider::{EntrypointMode, OutputFormatSelector};

mod antigravity;
mod apply;
mod claude;
mod codex;
mod gemini;
mod goose;
mod kilo;
mod kimi;
mod opencode;
mod pi;
mod qwen;
mod resolve;

pub(crate) use self::resolve::{
    NoModelProvided, OpenCodeEnvSnapshot, OpenCodeModelSource, apply_opencode_model_resolution,
    extract_prompt_source_from_passthrough, require_prompt_present, resolve_opencode_model,
};
pub(crate) use self::antigravity::AntigravityWrapper;
pub(crate) use self::claude::ClaudeWrapper;
pub(crate) use self::codex::CodexWrapper;
pub(crate) use self::gemini::GeminiWrapper;
pub(crate) use self::goose::GooseWrapper;
pub(crate) use self::kilo::KiloWrapper;
pub(crate) use self::kimi::KimiWrapper;
pub(crate) use self::opencode::OpencodeWrapper;
pub(crate) use self::pi::PiWrapper;
pub(crate) use self::qwen::QwenWrapper;

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
        apply::apply_yolo(self.provider(), args, env_overrides)
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
        // The `_` arm dispatches through `self.apply_yolo` so provider
        // overrides (Gemini, Qwen) still win; only the NonInteractiveOnly
        // branch is catalog-uniform, so the body stays here rather than
        // collapsing into a free function that can't see the override.
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
        apply::reject_direct_yolo(self.provider(), args)
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
        apply::apply_entrypoint(self.provider(), args, non_interactive)
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
    /// Default implementation reads the provider's catalog `model_cli_flag`
    /// (the authoritative delivery flag, e.g. `--model`): when present it is
    /// pushed with the model value (skipped if the user already passed it, or
    /// the conventional short alias `-m`); when absent a warning is returned
    /// (a provider with non-flag delivery must override this method). The
    /// generic `MODEL` env var is always exported — Claudine's wrapper
    /// contract, consumed by composition templates, hook dispatch, and
    /// reporting independent of how the provider itself reads the model.
    fn apply_model(
        &self,
        args: &mut Vec<String>,
        env_overrides: &mut Vec<(String, String)>,
        model: &str,
    ) -> Option<String> {
        apply::apply_model(self.provider(), args, env_overrides, model)
    }

    // -- Universal --output flag ---------------------------------------------

    /// Map the universal `--output <format>` to provider-specific flags.
    ///
    /// Default implementation reads from the central catalog's
    /// [`OutputFormatSupport`] list.
    fn apply_output_format(&self, args: &mut Vec<String>, format: OutputFormat) -> Option<String> {
        apply::apply_output_format(self.provider(), args, format)
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
        interactive: bool,
        cwd: &Path,
        scoped_tmp: &Path,
    ) -> Result<super::system_prompt::SystemPromptApplication> {
        apply::apply_system_prompt(self.provider(), prompt, interactive, cwd, scoped_tmp)
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
    /// Default: the central catalog's curated suppression list.
    fn stdout_noise_prefixes(&self) -> &'static [&'static str] {
        provider_info(self.provider()).display_policy.stdout_noise_prefixes
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
    /// Default: the central catalog's curated suppression list.
    fn stderr_noise_prefixes(&self) -> &'static [&'static str] {
        provider_info(self.provider()).display_policy.stderr_noise_prefixes
    }

    /// When true, structured non-interactive runs buffer filtered stderr and
    /// only surface it if the provider exits with an error.
    ///
    /// Default: reads the central catalog.
    fn suppress_structured_stderr_on_success(&self) -> bool {
        provider_info(self.provider()).suppress_structured_stderr_on_success
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
    /// Default: the central catalog's hand-ruled security allowlist.
    fn allowed_env_keys(&self) -> &'static [&'static str] {
        provider_info(self.provider()).allowed_env_keys
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
    /// Defaults to the central catalog's [`ResumeSupport`] level:
    /// `FirstClass` and `Partial` map to `true`; the mode-scoped
    /// variants (`InteractiveOnly` / `NonInteractiveOnly`) map to
    /// `false` pending a mode-aware gate. Ratified end-state (Ken,
    /// 2026-07-04): whenever the provider natively supports resume,
    /// Claudine must too — a `false` here is only ever a
    /// not-yet-implemented gap, never a durable posture. Every compiled
    /// provider is currently `FirstClass` and implements
    /// [`build_resume_args`](Self::build_resume_args).
    fn supports_resume(&self) -> bool {
        matches!(
            provider_info(self.provider()).resume,
            ResumeSupport::FirstClass | ResumeSupport::Partial
        )
    }

    // -- Structured stream support -------------------------------------------

    /// Apply internal structured stream flags to child args.
    ///
    /// Called only when `supports_structured_stream()` returns true and
    /// the user did not explicitly request an output format.
    ///
    /// Default implementation derives the argv from the central
    /// catalog's `Stream`-format [`OutputFormatSupport`] record:
    /// companion flags first, then the selector.
    ///
    /// [`OutputFormatSupport`]: claudine::provider::OutputFormatSupport
    fn apply_structured_stream(&self, args: &mut Vec<String>) {
        apply::apply_structured_stream(self.provider(), args)
    }

    /// Whether Claudine can recover a final assistant body after an
    /// interactive session ends for inline composition closure.
    ///
    /// Default: reads the central catalog.
    fn supports_interactive_inline_closure(&self) -> bool {
        provider_info(self.provider()).supports_interactive_inline_closure
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
static KILO: KiloWrapper = KiloWrapper;
static PI: PiWrapper = PiWrapper;
static ANTIGRAVITY: AntigravityWrapper = AntigravityWrapper;

/// Wrapper registry indexed by `Provider as usize`.
///
/// The array length is tied to [`PROVIDER_COUNT`], so adding a new
/// [`Provider`] variant forces a compile error here until the slot is
/// addressed. There is intentionally no wildcard fallback.
///
/// ## Documented "no wrapper" exceptions
///
/// Currently none. Any future provider that is intentionally unwrapped
/// must use a `None` slot in this table *and* be added to the
/// `NO_WRAPPER` allow-list referenced by
/// `wrapper_registry_covers_every_provider_and_documents_exceptions`.
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
    /* 7: Kilo     */ Some(&KILO),
    /* 8: Pi       */ Some(&PI),
    /* 9: Antigravity */ Some(&ANTIGRAVITY),
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
        if prompt.starts_with('-') {
            args.push("--".to_string());
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    mod apply_output_format;
    mod apply_yolo;
    mod native_output;
    mod positional;
}
