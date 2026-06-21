//! Non-interactive session planning, the spawn seam, and stream capture.
//!
//! [`build_plan`] turns Claudine's typed provider catalog into a fully
//! resolved [`SessionPlan`] (program, argv, isolated working directory,
//! allowlisted environment). [`SessionRunner`] is the seam between the plan and
//! a running process: [`TokioSessionRunner`] spawns it, while tests inject a
//! fake runner that returns canned stdout. The adapter feeds the captured
//! lines through Claudine's real semantic parser, so planning and parsing are
//! exercised without any agentic CLI installed.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use async_trait::async_trait;
use biscuit_contract::inference::{InferenceError, InferenceErrorKind};
use claudine::provider::{
    OutputFormat, OutputFormatSelector, ProviderInfo, SystemPromptDelivery, provider_info,
};
use claudine::provider_id::Provider;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

use crate::error::{inference_error, map_spawn_error};
use crate::profile::ResolvedReasoning;
use crate::support::{auth_env_vars, non_interactive_entrypoint};

/// A fully resolved non-interactive session, ready to spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionPlan {
    /// The provider this plan targets.
    pub provider: Provider,
    /// The program to execute (the provider binary).
    pub program: String,
    /// Arguments after `program`, in order. The prompt is always last.
    pub args: Vec<String>,
    /// Isolated working directory the process runs in.
    pub working_dir: PathBuf,
    /// Allowlisted environment passed to the process. The child's environment
    /// is otherwise cleared — nothing from the caller leaks through.
    pub env: Vec<(String, String)>,
    /// Resolved model passed via `--model`, or `None` to use the provider
    /// default.
    pub model: Option<String>,
    /// Resolved reasoning preference (recorded, not emitted in v1).
    pub(crate) reasoning: Option<ResolvedReasoning>,
}

/// How the calling environment is read when building the allowlist.
///
/// Defaults to the process environment; tests inject a fixed map to prove the
/// allowlist forwards only intended keys and drops everything else.
#[derive(Clone)]
pub(crate) enum EnvSource {
    Process,
    /// Test-only fixed environment for asserting the allowlist deterministically.
    #[cfg_attr(not(test), allow(dead_code))]
    Fixed(HashMap<String, String>),
}

impl EnvSource {
    pub(crate) fn get(&self, key: &str) -> Option<String> {
        match self {
            EnvSource::Process => std::env::var(key).ok(),
            EnvSource::Fixed(map) => map.get(key).cloned(),
        }
    }
}

/// OS variables an agentic CLI typically needs just to start.
///
/// `HOME` is deliberately excluded: the caller's home is never forwarded.
/// The adapter sets `HOME` to an isolated shadow tree instead (see
/// [`crate::home`]).
const OS_BASELINE_ENV: &[&str] = &[
    "PATH", "LANG", "LC_ALL", "LC_CTYPE", "TMPDIR", "USER", "LOGNAME", "SHELL", "TERM",
];

/// Build the allowlisted environment for a provider session.
///
/// Includes only the OS baseline, the provider's model-selection variables,
/// and the provider's authentication variables — and only those actually
/// present in `source`. The caller's full environment is never forwarded, and
/// `HOME` is set separately to the isolated shadow home, not copied from here.
pub(crate) fn build_env(provider: Provider, source: &EnvSource) -> Vec<(String, String)> {
    let info = provider_info(provider);
    let mut names: Vec<&str> = Vec::new();
    names.extend_from_slice(OS_BASELINE_ENV);
    names.extend_from_slice(info.model_env_vars);
    names.extend_from_slice(auth_env_vars(provider));

    let mut seen = std::collections::HashSet::new();
    let mut env = Vec::new();
    for name in names {
        if !seen.insert(name) {
            continue;
        }
        if let Some(value) = source.get(name) {
            env.push((name.to_string(), value));
        }
    }
    env
}

/// Build a non-interactive session plan from the typed provider catalog.
///
/// `prompt` is the (already structured-augmented, if applicable) untrusted
/// user text. `system_instruction` is the adapter-owned guard delivered
/// out-of-band where the provider exposes a system-prompt channel, and
/// prepended to the prompt otherwise.
pub(crate) fn build_plan(
    provider: Provider,
    model: Option<&str>,
    reasoning: Option<ResolvedReasoning>,
    prompt: &str,
    system_instruction: &str,
    working_dir: PathBuf,
    env: Vec<(String, String)>,
) -> Result<SessionPlan, InferenceError> {
    let info = provider_info(provider);
    let entry = non_interactive_entrypoint(info).ok_or_else(|| {
        inference_error(
            InferenceErrorKind::Unsupported,
            "provider has no non-interactive entrypoint",
        )
    })?;

    let mut args: Vec<String> = Vec::new();

    // Entrypoint subcommand (e.g. Codex `exec`) then its required flags.
    if let Some(subcommand) = entry.subcommand {
        args.extend(subcommand.split_whitespace().map(str::to_string));
    }
    args.extend(entry.required_flags.iter().map(|flag| flag.to_string()));

    // Structured stream output so the semantic parser can capture final text.
    args.extend(stream_output_args(provider, info));
    args.extend(companion_output_args(provider));

    // Tool-denial and reasoning controls before the model turn starts.
    args.extend(tool_denial_args(provider));
    args.extend(reasoning_args(provider, reasoning.as_ref()));

    // Adapter-owned guard instruction, out-of-band where possible.
    let prompt = match system_prompt_args(info, system_instruction) {
        Some(sp_args) => {
            args.extend(sp_args);
            prompt.to_string()
        }
        None => format!("{system_instruction}\n\n---\n\n{prompt}"),
    };

    // Explicit model override only; absent one the provider uses its default.
    if let Some(model) = model {
        args.push("--model".to_string());
        args.push(model.to_string());
    }

    // Prompt placement per the provider's argv conventions — always last.
    if let Some(flag) = info.prompt_arg_conventions.prompt_flags.first() {
        args.push((*flag).to_string());
    }
    args.push(prompt);

    Ok(SessionPlan {
        provider,
        program: info.binary.to_string(),
        args,
        working_dir,
        env,
        model: model.map(str::to_string),
        reasoning,
    })
}

/// The catalog output format whose flag makes the provider emit the JSONL event
/// stream Claudine's semantic parser consumes.
///
/// This is deliberately *not* uniformly [`OutputFormat::Stream`]: the catalog
/// names each provider's formats independently. Codex's event telemetry is the
/// `--json` ([`OutputFormat::Json`], native `jsonl`) stream, while its
/// [`OutputFormat::Stream`] entry is `--output-schema` — a final-output schema
/// validator, not a telemetry stream. Selecting `Stream` for Codex would plan
/// `codex exec --output-schema …` and starve the parser of events, returning
/// empty output for an enabled provider.
fn stream_output_format(provider: Provider) -> OutputFormat {
    match provider {
        Provider::Codex => OutputFormat::Json,
        _ => OutputFormat::Stream,
    }
}

/// Arguments selecting the provider's parser-compatible JSONL stream output.
fn stream_output_args(provider: Provider, info: &'static ProviderInfo) -> Vec<String> {
    let target = stream_output_format(provider);
    let Some(format) = info
        .output_formats
        .iter()
        .find(|format| format.format == target)
    else {
        return Vec::new();
    };
    match &format.selector {
        OutputFormatSelector::FlagValue { flag } => {
            vec![(*flag).to_string(), format.native_name.to_string()]
        }
        OutputFormatSelector::Flag { flag } => vec![(*flag).to_string()],
        OutputFormatSelector::Positional { token } => vec![(*token).to_string()],
        OutputFormatSelector::Default | OutputFormatSelector::TransportFlag { .. } => Vec::new(),
    }
}

/// Provider-specific companion flags required alongside the stream format.
///
/// Claude's `--print --output-format stream-json` requires `--verbose`; the
/// requirement is not encoded in the typed catalog, so it is supplied here for
/// the v1-enabled providers that need it.
fn companion_output_args(provider: Provider) -> Vec<String> {
    match provider {
        Provider::Claude => vec!["--verbose".to_string()],
        _ => Vec::new(),
    }
}

/// Provider-specific argv that constrains tool use before the model turn starts.
///
/// This is the pre-turn layer, distinct from the guard prompt (advisory) and
/// post-hoc stream rejection (voids the response if a tool was attempted; see
/// [`crate::error::check_security`]). For Claude the deny-all lives in the
/// shadow-home `settings.json` (see [`crate::home`]); here it only forces a
/// strict, empty MCP config so no server can load for the run.
///
/// Codex has no deny-all equivalent: execution-rules (`forbidden`) only gate
/// commands *escaping* the sandbox, cannot express a catch-all (a `pattern` is a
/// command prefix), and a malformed rules file panics the binary. So the tightest
/// pre-turn lever is `--sandbox read-only`, which blocks writes. Network denial
/// is treated as a defense-in-depth assumption rather than a guarantee; any
/// read-only command attempt runs against the isolated empty CWD and shadow HOME,
/// surfaces as a tool item in the JSONL stream, and is rejected post-hoc.
fn tool_denial_args(provider: Provider) -> Vec<String> {
    match provider {
        Provider::Claude => vec![
            "--strict-mcp-config".to_string(),
            "--mcp-config".to_string(),
            r#"{"mcpServers":{}}"#.to_string(),
        ],
        Provider::Codex => vec!["--sandbox".to_string(), "read-only".to_string()],
        _ => Vec::new(),
    }
}

/// Emit the resolved reasoning preference onto argv where the provider exposes
/// a verified non-interactive config-override.
///
/// Codex reads `model_reasoning_effort` from config; `-c key=value` overrides
/// it for the run. Claude's non-interactive reasoning control is not reliably
/// verifiable, so its preference is recorded on the plan but not emitted (see
/// [`crate::profile`]).
fn reasoning_args(provider: Provider, reasoning: Option<&ResolvedReasoning>) -> Vec<String> {
    let Some(reasoning) = reasoning else {
        return Vec::new();
    };
    match provider {
        Provider::Codex => vec![
            "-c".to_string(),
            format!("{}={}", reasoning.flag, toml_basic_string(&reasoning.value)),
        ],
        _ => Vec::new(),
    }
}

/// Out-of-band system-prompt argv for the provider's non-interactive append
/// channel, or `None` when the channel is not a simple inline flag/config key.
fn system_prompt_args(info: &'static ProviderInfo, instruction: &str) -> Option<Vec<String>> {
    match &info.system_prompt.append.non_interactive {
        SystemPromptDelivery::InlineFlag { flag } => {
            Some(vec![(*flag).to_string(), instruction.to_string()])
        }
        SystemPromptDelivery::ConfigKeyInline { flag, key } => Some(vec![
            (*flag).to_string(),
            format!("{key}={}", toml_basic_string(instruction)),
        ]),
        _ => None,
    }
}

/// Serialize a string as a TOML basic-string literal for a `-c key="value"`
/// config override.
///
/// Codex parses `-c` overrides as TOML, so a value carrying spaces, quotes,
/// backslashes, or newlines (the adapter-owned guard is multi-sentence prose)
/// must be quoted and escaped or the override is rejected before inference
/// runs. Escapes follow the TOML 1.0 basic-string grammar.
fn toml_basic_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{000C}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            // Remaining C0 controls and DEL have no short escape in TOML.
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Raw captured output of one provider session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawSession {
    /// Stdout, split into lines, in order.
    pub stdout_lines: Vec<String>,
    /// Process exit code (`-1` if terminated by signal).
    pub exit_code: i32,
    /// Captured stderr, if any.
    pub stderr: Option<String>,
}

/// The seam between a [`SessionPlan`] and a running process.
///
/// The production [`TokioSessionRunner`] spawns the process; tests inject a
/// fake that returns canned stdout, so the adapter is exercised without any
/// agentic CLI installed.
#[async_trait]
pub(crate) trait SessionRunner: Send + Sync {
    /// Run the plan and return its raw captured output.
    async fn run(&self, plan: &SessionPlan) -> Result<RawSession, InferenceError>;
}

/// Spawns the session as a child process via Tokio.
#[derive(Debug, Default, Clone)]
pub(crate) struct TokioSessionRunner;

#[async_trait]
impl SessionRunner for TokioSessionRunner {
    async fn run(&self, plan: &SessionPlan) -> Result<RawSession, InferenceError> {
        let mut command = tokio::process::Command::new(&plan.program);
        command
            .args(&plan.args)
            .current_dir(&plan.working_dir)
            .env_clear();
        for (key, value) in &plan.env {
            command.env(key, value);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = command.spawn().map_err(|err| map_spawn_error(err, &plan.program))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| inference_error(InferenceErrorKind::Provider, "failed to capture provider stdout"))?;
        let mut stderr = child.stderr.take();

        // Drain stdout and stderr concurrently while awaiting exit. Reading one
        // pipe to EOF before the other deadlocks when a provider fills the
        // unread pipe (agentic CLIs emit diagnostics and progress on stderr) and
        // blocks before its stdout reaches EOF.
        let stdout_fut = async {
            let mut reader = BufReader::new(stdout).lines();
            let mut lines = Vec::new();
            while let Some(line) = reader
                .next_line()
                .await
                .map_err(|_| inference_error(InferenceErrorKind::Provider, "failed to read provider stdout"))?
            {
                lines.push(line);
            }
            Ok::<Vec<String>, InferenceError>(lines)
        };
        let stderr_fut = async {
            let mut text = String::new();
            if let Some(stderr) = stderr.as_mut() {
                let _ = stderr.read_to_string(&mut text).await;
            }
            text
        };
        let wait_fut = child.wait();

        let (stdout_lines, stderr_text, status) =
            tokio::join!(stdout_fut, stderr_fut, wait_fut);
        let stdout_lines = stdout_lines?;
        let status = status
            .map_err(|_| inference_error(InferenceErrorKind::Provider, "failed to await provider process"))?;

        Ok(RawSession {
            stdout_lines,
            exit_code: status.code().unwrap_or(-1),
            stderr: Some(stderr_text).filter(|text| !text.is_empty()),
        })
    }
}

/// The default production runner as a shared trait object.
pub(crate) fn default_runner() -> Arc<dyn SessionRunner> {
    Arc::new(TokioSessionRunner)
}
