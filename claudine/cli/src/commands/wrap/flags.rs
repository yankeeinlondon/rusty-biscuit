use clap::Args;
use claudine::provider::{OutputFormatSelector, Provider, provider_info};
use color_eyre::eyre::{Result, eyre};

/// Shared wrapper args for provider subcommands.
///
/// Boolean flags like `--yolo`, `--interactive`, `--quiet`, `--silent`,
/// `--verbose`, and `--repo` are declared here for clap to parse AND also
/// extracted from the passthrough bucket by `extract_wrapper_flags_from_passthrough`.
/// The two sources are OR-merged so flags work whether placed on either side of
/// the first positional argument. This avoids bug #2.2 (dual-source truth) by
/// keeping clap as the primary parser while the passthrough extractor serves as
/// a fallback for flags that land after `trailing_var_arg` has started capturing.
///
/// The extractor honors the POSIX `--` convention: anything after the first
/// `--` separator is opaque to Claudine and is never read as a wrapper flag,
/// even when it collides with a Claudine flag name. The separator is
/// Claudine's own boundary and is not forwarded, so the arguments after it
/// reach the provider as provider options. See
/// `find_passthrough_dash_boundary_with_raw` for the detection strategy.
///
/// Unknown flags (belonging to the underlying agent) flow into `passthrough`
/// thanks to `ignore_errors(true)` on wrapper subcommands (see `parse_cli`).
#[derive(Debug, Clone, Args)]
pub struct WrapperArgs {
    /// Print help for this wrapper command.
    #[arg(short, long)]
    pub help: bool,

    /// Enable provider-specific YOLO/auto-approval mode.
    ///
    /// `CLAUDINE_YOLO=true` in the environment is equivalent to `--yolo`
    /// on the command line; both activate the same single intent signal.
    /// Note: yolo intent does not mean yolo *applied* — interactive
    /// launches that don't expose a non-interactive bypass (e.g.
    /// OpenCode TUI) will suppress the flag and the reporter's
    /// `effective_yolo` field will be `false` for those sessions.
    #[arg(short = 'y', long, env = "CLAUDINE_YOLO")]
    pub yolo: bool,

    /// Preserve this env var even when it matches sensitive-name filters.
    #[arg(long = "include", value_name = "ENV_NAME")]
    pub include: Vec<String>,

    /// Force interactive mode even when a prompt string is provided.
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,

    /// Draft the initial prompt in an external editor before launching the provider.
    ///
    /// Authoring only: the edited prompt then selects the session mode like a
    /// prompt given on the command line (non-interactive by default,
    /// interactive with `-i`). The editor needs a terminal on stdin and stdout
    /// whatever the session mode.
    #[arg(long)]
    pub edit: bool,

    /// Override the model used by the provider.
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Set the output format (json, text, stream).
    #[arg(short = 'o', long = "output", value_name = "FORMAT")]
    pub output: Option<String>,

    /// Append a system prompt from a file.
    #[arg(
        long = "append-system-prompt",
        visible_alias = "asp",
        value_name = "FILE",
        conflicts_with = "replace_system_prompt"
    )]
    pub append_system_prompt: Option<String>,

    /// Replace the provider's system prompt with contents from a file.
    #[arg(
        long = "replace-system-prompt",
        visible_alias = "rsp",
        value_name = "FILE",
        conflicts_with = "append_system_prompt"
    )]
    pub replace_system_prompt: Option<String>,

    /// Wall-clock timeout (e.g. `30s`, `5m`, `2h`). Sends SIGTERM then SIGKILL.
    /// Only valid in non-interactive mode.
    #[arg(short = 't', long = "timeout", value_name = "DURATION")]
    pub timeout: Option<String>,

    /// Step-silence timeout (e.g. `30s`, `5m`). Kills the child when no stream
    /// event is observed for this long. Only valid in non-interactive
    /// structured-stream mode.
    #[arg(long = "step-timeout", value_name = "DURATION")]
    pub step_timeout: Option<String>,

    /// OpenCode stalled-generation backstop budget (e.g. `10m`). Aborts when
    /// OpenCode repeatedly drops generations with no progress for this long.
    /// OpenCode-scoped, structured-stream only; `0s` disables. Default `10m`.
    #[arg(long = "stall-timeout", value_name = "DURATION")]
    pub stall_timeout: Option<String>,

    /// Show what would be executed without requiring or launching the child executable.
    #[arg(long)]
    pub dry_run: bool,

    /// Suppress env details and info messages, but still show the system prompt when set.
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Suppress all Claudine preflight output (header, env, info, warnings).
    #[arg(long, conflicts_with = "quiet")]
    pub silent: bool,

    /// Set the OPERATION env var for the wrapped session.
    #[arg(long = "operation", visible_alias = "op", value_name = "OP")]
    pub operation: Option<String>,

    /// Enable provider-specific sandboxing.
    #[arg(long)]
    pub sandbox: bool,

    /// Use only repo-scoped skills, commands, and agents via a provider overlay.
    #[arg(long)]
    pub repo: bool,

    /// Enable Claudine-managed MCP session composition.
    #[arg(long)]
    pub mcp: bool,

    /// Activate specific MCP servers by ID or alias (comma-separated).
    #[arg(long = "use", value_name = "ID", value_delimiter = ',')]
    pub mcp_use: Vec<String>,

    /// Treat unresolved or ambiguous MCP tags as hard errors.
    #[arg(long)]
    pub strict: bool,

    /// Emit a performance report to stderr after command completion.
    #[arg(long)]
    pub perf: bool,

    /// Arguments forwarded to the wrapped provider CLI.
    ///
    /// Because wrapper subcommands use `ignore_errors(true)` (see `parse_cli`
    /// in main.rs), unknown flags from the underlying agent CLI land here
    /// instead of causing a clap error.
    #[arg(
        value_name = "ARGS",
        num_args = 0..,
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    pub passthrough: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExtractedWrapperFlags {
    pub(crate) yolo: bool,
    pub(crate) interactive: bool,
    pub(crate) edit: bool,
    pub(crate) repo: bool,
    pub(crate) quiet: bool,
    pub(crate) silent: bool,
    pub(crate) verbose: bool,
    pub(crate) operation: Option<String>,
    pub(crate) perf: bool,
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn has_flag_value(args: &[String], flag: &str, value: &str) -> bool {
    args.iter().enumerate().any(|(index, arg)| {
        if let Some(inline) = arg.strip_prefix(&format!("{flag}=")) {
            return inline == value;
        }
        arg == flag && args.get(index + 1).is_some_and(|next| next == value)
    })
}

/// Reject retired composition flags that should no longer be forwarded to
/// wrapped providers. Users should migrate to `claudine compose` or
/// `claudine inline-compose`.
pub(crate) fn reject_retired_composition_flags(args: &[String]) -> Result<()> {
    const RETIRED: &[(&str, &str)] = &[
        ("--compose", "claudine compose --<provider> <file>"),
        (
            "--frontmatter-prompt",
            "claudine inline-compose --<provider> <file>",
        ),
        (
            "--prompt-file",
            "the provider CLI directly (claudine compose has different semantics)",
        ),
    ];

    for (flag, replacement) in RETIRED {
        if args
            .iter()
            .any(|a| a == flag || a.starts_with(&format!("{flag}=")))
        {
            return Err(eyre!(
                "{flag} has been retired; use `{replacement}` instead"
            ));
        }
    }

    Ok(())
}

pub(crate) fn has_explicit_native_output_request(provider: Provider, args: &[String]) -> bool {
    let args_before_separator = args
        .iter()
        .position(|arg| arg == "--")
        .map_or(args, |separator| &args[..separator]);

    provider_info(provider)
        .output_formats
        .iter()
        .any(|format| match format.selector {
            OutputFormatSelector::Default | OutputFormatSelector::TransportFlag { .. } => false,
            OutputFormatSelector::Flag { flag } => has_flag(args_before_separator, flag),
            OutputFormatSelector::FlagValue { flag } => {
                has_flag_value(args_before_separator, flag, format.native_name)
            }
            OutputFormatSelector::Positional { token } => {
                args_before_separator.iter().any(|arg| arg == token)
            }
        })
}

#[allow(dead_code)]
fn model_value_from_args(args: &[String]) -> Option<String> {
    for (index, arg) in args.iter().enumerate() {
        if arg == "--model" || arg == "-m" {
            return args.get(index + 1).cloned();
        }
        if let Some(value) = arg.strip_prefix("--model=") {
            return Some(value.to_string());
        }
        if let Some(value) = arg.strip_prefix("-m=") {
            return Some(value.to_string());
        }
    }
    None
}

pub(crate) fn print_wrapper_help(provider: Provider) {
    let slug = provider.as_slug();
    println!(
        "Wrap {provider} with Claudine preflight/env handling\n\
         \n\
         Usage: claudine {slug} [OPTIONS] [ARGS]...\n\
         \n\
         Arguments:\n\
         \x20 [ARGS]...  Arguments forwarded to the wrapped provider CLI\n\
         \n\
         Options:\n\
         \x20 -y, --yolo               Enable provider-specific YOLO/auto-approval mode\n\
         \x20     --include <ENV_NAME>  Preserve this env var even when it matches sensitive-name filters\n\
         \x20 -i, --interactive         Force interactive mode even when a prompt string is provided\n\
         \x20     --edit                Draft the initial prompt in an external editor; combine with -i for an interactive session\n\
         \x20 -m, --model <MODEL>       Override the model used by the provider\n\
         \x20 -o, --output <FORMAT>     Set the output format (json, text, stream)\n\
          \x20     --asp <FILE>             Append a system prompt from a file\n\
           \x20     --rsp <FILE>             Replace the provider's system prompt with contents from a file\n\
            \x20 -t, --timeout <DURATION>  Wall-clock timeout like 30s, 5m, 2h (non-interactive only)\n\
         \x20     --stall-timeout <DURATION>  OpenCode stalled-generation backstop like 10m; 0s disables\n\
         \x20     --dry-run             Show what would be executed without requiring or launching the child executable\n\
         \x20 -q, --quiet              Suppress env details and info; still show the system prompt when set\n\
         \x20     --silent              Suppress all Claudine preflight output\n\
         \x20     --operation <OP>      Set the OPERATION env var for the wrapped session\n\
         \x20     --sandbox             Enable provider-specific sandboxing\n\
         \x20     --repo                Use only repo-scoped skills, commands, and agents\n\
         \x20     --mcp                 Enable Claudine-managed MCP session composition\n\
         \x20     --use <ID>            Activate specific MCP servers by ID or alias\n\
         \x20     --strict              Treat unresolved or ambiguous MCP tags as hard errors\n\
         \x20     --perf                Emit a performance report to stderr after command completion\n\
         \x20 -h, --help               Print help"
    );
}

/// Where the user's `--` separator put Claudine's argument boundary in the
/// wrapper passthrough vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DashBoundary {
    /// clap kept the `--` token (it followed the first positional, so
    /// `trailing_var_arg` captured it verbatim) at this index.
    Literal(usize),
    /// clap consumed the `--` (it preceded every positional); the protected
    /// tail starts at this index and no separator token remains.
    Consumed(usize),
}

impl DashBoundary {
    fn protected_from(self) -> usize {
        match self {
            Self::Literal(index) | Self::Consumed(index) => index,
        }
    }
}

/// Locate the user's first `--` relative to the passthrough vector.
///
/// The raw process arguments decide which `--` is Claudine's: the tokens
/// after the first raw `--` are the passthrough's tail, so a later `--`
/// inside that tail stays the provider's own separator. When the raw
/// arguments do not end the passthrough (a caller that did not come from the
/// process command line), the first literal `--` in the passthrough is used.
fn find_passthrough_dash_boundary_with_raw(
    passthrough: &[String],
    raw_args: &[String],
) -> Option<DashBoundary> {
    if let Some(raw_pos) = raw_args.iter().position(|arg| arg == "--") {
        let raw_tail = &raw_args[raw_pos + 1..];
        if passthrough.ends_with(raw_tail) {
            let index = passthrough.len() - raw_tail.len();
            return Some(if index > 0 && passthrough[index - 1] == "--" {
                DashBoundary::Literal(index - 1)
            } else {
                DashBoundary::Consumed(index)
            });
        }
    }
    passthrough
        .iter()
        .position(|arg| arg == "--")
        .map(DashBoundary::Literal)
}

/// Recover wrapper flags that landed in the passthrough and consume the
/// user's `--` separator.
///
/// The first `--` is Claudine's boundary, not the provider's: arguments after
/// it are never read as wrapper flags, and the separator token itself is
/// removed so those arguments reach the provider as the options they were
/// written as, ahead of any separator the provider profile adds before a
/// prompt.
pub(crate) fn extract_wrapper_flags_from_passthrough(
    args: &mut Vec<String>,
) -> Result<ExtractedWrapperFlags> {
    let raw: Vec<String> = std::env::args().collect();
    extract_wrapper_flags_from_passthrough_with_raw(args, &raw)
}

/// [`extract_wrapper_flags_from_passthrough`] against explicit raw process
/// arguments (`argv[0]` included).
pub(crate) fn extract_wrapper_flags_from_passthrough_with_raw(
    args: &mut Vec<String>,
    raw_args: &[String],
) -> Result<ExtractedWrapperFlags> {
    let boundary = find_passthrough_dash_boundary_with_raw(args, raw_args);
    extract_wrapper_flags_from_passthrough_with_boundary(args, boundary)
}

fn extract_wrapper_flags_from_passthrough_with_boundary(
    args: &mut Vec<String>,
    dash_boundary: Option<DashBoundary>,
) -> Result<ExtractedWrapperFlags> {
    let boundary = dash_boundary.map_or(args.len(), DashBoundary::protected_from);
    let boundary = boundary.min(args.len());
    let mut extracted = ExtractedWrapperFlags::default();
    let mut skip_next = false;
    let mut remove_indices = Vec::new();

    for i in 0..boundary {
        if skip_next {
            skip_next = false;
            continue;
        }
        let arg = &args[i];
        match arg.as_str() {
            "-y" | "--yolo" => {
                extracted.yolo = true;
                remove_indices.push(i);
            }
            "-i" | "--interactive" => {
                extracted.interactive = true;
                remove_indices.push(i);
            }
            "--edit" => {
                extracted.edit = true;
                remove_indices.push(i);
            }
            "--repo" => {
                extracted.repo = true;
                remove_indices.push(i);
            }
            "-q" | "--quiet" => {
                extracted.quiet = true;
                remove_indices.push(i);
            }
            "--silent" => {
                extracted.silent = true;
                remove_indices.push(i);
            }
            "-v" | "--verbose" => {
                extracted.verbose = true;
                remove_indices.push(i);
            }
            "--perf" => {
                extracted.perf = true;
                remove_indices.push(i);
            }
            "--operation" | "--op" => {
                let next = args.get(i + 1);
                let value_within_boundary = i + 1 < boundary;
                let next_is_separator = next.map(|v| v == "--").unwrap_or(false);

                if next.is_none() || !value_within_boundary || next_is_separator {
                    return Err(eyre!(
                        "missing value for `{arg}`; pass a value like `{arg} <OP>` \
                         before any `--` separator"
                    ));
                }

                extracted.operation = Some(next.unwrap().clone());
                remove_indices.push(i);
                remove_indices.push(i + 1);
                skip_next = true;
            }
            _ => {
                if let Some(value) = arg.strip_prefix("--operation=") {
                    extracted.operation = Some(value.to_string());
                    remove_indices.push(i);
                } else if let Some(value) = arg.strip_prefix("--op=") {
                    extracted.operation = Some(value.to_string());
                    remove_indices.push(i);
                }
            }
        }
    }

    // The separator sits past every removed index, so dropping it first
    // leaves those indices valid.
    if let Some(DashBoundary::Literal(index)) = dash_boundary {
        args.remove(index);
    }
    // Remove in reverse order to preserve indices.
    for i in remove_indices.into_iter().rev() {
        args.remove(i);
    }

    Ok(extracted)
}

#[cfg(test)]
mod tests;
