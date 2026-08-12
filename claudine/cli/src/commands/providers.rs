use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::{Result, WrapErr, bail};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::layout::{Alignment, Length, Edges};
use clap::{Args, Subcommand, ValueEnum};
use claudine::events::AgenticEvent;
use claudine::linking::{LinkableResource, capabilities_for};
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};
use claudine::provider::{all_providers, provider_info};

use super::context_render::{
    TABLE_WIDTH_ERROR_SENTINEL, configure_shared_table, render_table_within_contract,
    report_column,
};
use crate::cli_utils::bool_indicator;
use crate::log;

/// Output format for `claudine providers --describe`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProvidersFormat {
    /// Default human-readable text output (table).
    Text,
    /// Structured JSON output sourced from `provider_info(p)`.
    Json,
}

/// Arguments accepted by the `claudine providers` command.
#[derive(Debug, Args)]
pub struct ProvidersArgs {
    /// Render structured `ProviderInfo` data from the central catalog.
    ///
    /// Without this flag, `claudine providers` shows the legacy capability
    /// matrix (skill/slash/agent/hooks). With this flag, output reflects the
    /// fields exposed by `claudine::provider::provider_info`.
    #[arg(long)]
    pub describe: bool,

    /// Output format. Only meaningful with `--describe`.
    #[arg(long, value_enum, default_value_t = ProvidersFormat::Text)]
    pub format: ProvidersFormat,

    #[command(subcommand)]
    pub command: Option<ProvidersCommand>,
}

/// Subcommands under `claudine providers`.
#[derive(Debug, Subcommand)]
pub enum ProvidersCommand {
    /// Run the provider-catalog generator (shells out to `claudine-gen`;
    /// this CLI never links the generator).
    Generate(GenerateArgs),
    /// Run deterministic checks for provider research topics.
    AgentErrors(AgentErrorsArgs),
}

/// Arguments accepted by `claudine providers generate`.
#[derive(Debug, Args)]
pub struct GenerateArgs {
    /// Regenerate a single provider's data.rs (default: all). Forwarded to
    /// `claudine-gen generate`, which confirms each drifted file on a TTY.
    pub slug: Option<String>,

    /// Show the generator's mapping registry (field -> source -> coercion)
    /// as a rendered table.
    #[arg(long, conflicts_with_all = ["slug", "dry_run", "yes"])]
    pub mapping: bool,

    /// Emit the raw mapping JSON instead of the rendered table.
    #[arg(long, requires = "mapping")]
    pub json: bool,

    /// Report-only: show what would change without writing (forwarded).
    #[arg(long)]
    pub dry_run: bool,

    /// Write every drifted file without prompting (forwarded).
    #[arg(long, conflicts_with = "dry_run")]
    pub yes: bool,

    /// Scaffold a newly wired provider (facts skeleton + mod.rs/behavior.rs
    /// stubs) before generating its data.rs (forwarded). Requires a slug.
    #[arg(long, requires = "slug", conflicts_with = "mapping")]
    pub scaffold: bool,
}

/// Arguments accepted by `claudine providers agent-errors`.
#[derive(Debug, Args)]
pub struct AgentErrorsArgs {
    #[command(subcommand)]
    pub command: AgentErrorsCommand,
}

/// Deterministic checks for the `agent-errors` research topic.
#[derive(Debug, Subcommand)]
pub enum AgentErrorsCommand {
    /// Validate one provider document and write an explicit outcome report.
    Check(AgentErrorsCheckArgs),
}

/// Arguments accepted by `claudine providers agent-errors check`.
#[derive(Debug, Args)]
pub struct AgentErrorsCheckArgs {
    /// Provider slug (the research document stem).
    pub slug: String,

    /// Outcome-report path. Defaults to the topic's `.findings` directory.
    #[arg(long)]
    pub findings: Option<PathBuf>,
}

fn supports_custom_resource(provider: Provider, resource: LinkableResource) -> bool {
    capabilities_for(provider)
        .support_for(resource)
        .level
        .allows_custom()
}

fn supported_hook_count(provider: Provider) -> usize {
    AgenticEvent::ALL
        .into_iter()
        .filter(|event| provider.supports_event_via_hook(event))
        .count()
}

/// Show provider capabilities for skills, slash commands, agents, and hooks.
pub fn run(args: ProvidersArgs) -> Result<()> {
    if let Some(command) = args.command {
        return match command {
            ProvidersCommand::Generate(generate) => run_generate(generate),
            ProvidersCommand::AgentErrors(agent_errors) => run_agent_errors(agent_errors),
        };
    }
    if args.describe {
        return run_describe(args.format);
    }
    let term = crate::log::terminal();

    let columns = vec![
        TableColumn::new("Provider"),
        TableColumn::new("Skill").with_alignment(Alignment::Center),
        TableColumn::new("Slash").with_alignment(Alignment::Center),
        TableColumn::new("Agent").with_alignment(Alignment::Center),
        TableColumn::new("Hooks").with_alignment(Alignment::Center),
    ];

    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().margin = Edges::x(Length::ch(1));

    for provider in PROVIDERS_DISPLAY_ORDER {
        let provider_cell: TableCellContent = if crate::log::is_plain()
            || std::env::var_os("NO_COLOR").is_some()
        {
            provider.to_string().into()
        } else {
            let provider_link = format!(r#"<a href="{}">{}</a>"#, provider.docs_url(), provider);
            Prose::new(provider_link)
                .render(&crate::log::optimistic_terminal(None))
                .into()
        };

        table.add_row(vec![
            provider_cell,
            bool_indicator(supports_custom_resource(provider, LinkableResource::Skill)),
            bool_indicator(supports_custom_resource(
                provider,
                LinkableResource::Command,
            )),
            bool_indicator(supports_custom_resource(provider, LinkableResource::Agent)),
            supported_hook_count(provider).to_string().into(),
        ]);
    }

    let rendered = table.render(&term);
    log::data(&format!("\n{rendered}"));

    Ok(())
}

/// Shells out to the `claudine-gen` binary. `--mapping` renders the
/// registry through the shared table components (`--json` passes the raw
/// payload through); everything else forwards to `claudine-gen generate`
/// with inherited stdio so its per-file TTY prompts work.
fn run_generate(args: GenerateArgs) -> Result<()> {
    if args.mapping {
        if args.json {
            return run_gen_passthrough(&["mapping"]);
        }
        return render_mapping();
    }
    let mut forwarded: Vec<&str> = vec!["generate"];
    if let Some(slug) = args.slug.as_deref() {
        forwarded.push(slug);
    }
    if args.dry_run {
        forwarded.push("--dry-run");
    }
    if args.yes {
        forwarded.push("--yes");
    }
    if args.scaffold {
        forwarded.push("--scaffold");
    }
    run_gen_passthrough(&forwarded)
}

/// Runs the `agent-errors` deterministic gate through the trusted Claudine
/// command surface. The generator remains a separate binary; this mirrors
/// `providers generate` and retains its installed-binary/dev-checkout fallback.
fn run_agent_errors(args: AgentErrorsArgs) -> Result<()> {
    match args.command {
        AgentErrorsCommand::Check(check) => {
            let mut command = gen_command()?;
            command.args(["agent-errors", "check"]).arg(&check.slug);
            if let Some(findings) = check.findings {
                command.arg("--findings").arg(findings);
            }
            let status = command
                .status()
                .wrap_err("failed to launch claudine-gen agent-errors check")?;
            if !status.success() {
                bail!(
                    "claudine-gen agent-errors check {} exited with {status}",
                    check.slug
                );
            }
            Ok(())
        }
    }
}

/// Runs `claudine-gen` with inherited stdio (interactive passthrough).
fn run_gen_passthrough(gen_args: &[&str]) -> Result<()> {
    let status = gen_command()?
        .args(gen_args)
        .status()
        .wrap_err("failed to launch claudine-gen")?;
    if !status.success() {
        bail!("claudine-gen {} exited with {status}", gen_args.join(" "));
    }
    Ok(())
}

/// Captures `claudine-gen mapping` and renders it as a table (stderr stays
/// inherited so generator diagnostics remain visible).
fn render_mapping() -> Result<()> {
    let output = gen_command()?
        .arg("mapping")
        .stderr(std::process::Stdio::inherit())
        .output()
        .wrap_err("failed to launch claudine-gen")?;
    if !output.status.success() {
        bail!("claudine-gen mapping exited with {}", output.status);
    }
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).wrap_err("claudine-gen mapping emitted invalid JSON")?;
    let rows = mapping_rows(&payload)?;

    let term = crate::log::terminal();
    let build = |with_description: bool| {
        let mut columns = vec![
            report_column("Field"),
            report_column("Source"),
            report_column("Expected shape"),
            report_column("Coercion"),
        ];
        if with_description {
            columns.push(report_column("Description"));
        }
        let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
        configure_shared_table(&mut table);
        for row in &rows {
            let mut cells: Vec<TableCellContent> = vec![
                row.field.clone().into(),
                row.source.clone().into(),
                row.expected.clone().into(),
                row.coercion.clone().into(),
            ];
            if with_description {
                cells.push(row.description.clone().into());
            }
            table.add_row(cells);
        }
        table
    };
    // The registry description is a detail column shown only where the
    // width contract can afford it (same stance as the context report's
    // Example column).
    let with_description = render_table_within_contract(&build(true), &term);
    let rendered = if with_description.contains(TABLE_WIDTH_ERROR_SENTINEL) {
        render_table_within_contract(&build(false), &term)
    } else {
        with_description
    };
    log::data(&format!("\n{rendered}"));
    Ok(())
}

/// One rendered `--mapping` table row (data plumbing, unit-tested).
#[derive(Debug, PartialEq, Eq)]
struct MappingRow {
    field: String,
    source: String,
    expected: String,
    coercion: String,
    description: String,
}

/// JSON payload (`claudine-gen mapping`) → table rows.
///
/// Source renders as `kind:key` (roster/facts) or `kind:topic.path`
/// (research); expected shapes join with ` | ` and carry an
/// ` (optional)` suffix for optional fields.
fn mapping_rows(payload: &serde_json::Value) -> Result<Vec<MappingRow>> {
    let fields = payload
        .get("fields")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| color_eyre::eyre::eyre!("mapping payload has no `fields` array"))?;
    let text = |value: &serde_json::Value, key: &str| -> Result<String> {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| color_eyre::eyre::eyre!("mapping entry missing string `{key}`"))
    };
    fields
        .iter()
        .map(|entry| {
            let source = entry
                .get("source")
                .ok_or_else(|| color_eyre::eyre::eyre!("mapping entry missing `source`"))?;
            let kind = text(source, "kind")?;
            let source = match kind.as_str() {
                "research" => format!("{kind}:{}.{}", text(source, "topic")?, text(source, "path")?),
                _ => format!("{kind}:{}", text(source, "key")?),
            };
            let mut expected = entry
                .get("expected")
                .and_then(serde_json::Value::as_array)
                .map(|shapes| {
                    shapes
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" | ")
                })
                .unwrap_or_default();
            if entry.get("optional").and_then(serde_json::Value::as_bool) == Some(true) {
                expected.push_str(" (optional)");
            }
            Ok(MappingRow {
                field: text(entry, "field")?,
                source,
                expected,
                coercion: text(entry, "coercion")?,
                description: text(entry, "description")?,
            })
        })
        .collect()
}

/// Resolves the `claudine-gen` command (installed binary or the
/// dev-checkout `cargo run` fallback).
fn gen_command() -> Result<Command> {
    match resolve_gen_binary() {
        Some(binary) => Ok(Command::new(binary)),
        None => match repo_root_for_cargo_fallback() {
            Some(root) => {
                let mut cargo = Command::new("cargo");
                cargo
                    .args(["run", "-p", "claudine-gen", "--quiet", "--"])
                    .current_dir(root);
                Ok(cargo)
            }
            None => bail!(
                "could not find the `claudine-gen` binary (looked next to this \
                 executable and on PATH) and no repo root is detectable for a \
                 `cargo run -p claudine-gen` fallback.\n\
                 Install it with `cargo install --path claudine/gen` or run from \
                 inside the rusty-biscuit repo."
            ),
        },
    }
}

/// Resolution order: sibling of the current executable, then `$PATH`.
fn resolve_gen_binary() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "claudine-gen.exe"
    } else {
        "claudine-gen"
    };
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name(name);
        if sibling.is_file() {
            return Some(sibling);
        }
    }
    which::which(name).ok()
}

/// A git root that looks like a cargo workspace, for the dev-checkout
/// `cargo run -p claudine-gen` fallback.
fn repo_root_for_cargo_fallback() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let root = biscuit_file::find_git_root(&cwd).ok().flatten()?;
    root.join("Cargo.toml").is_file().then_some(root)
}

/// Render structured provider catalog data sourced from
/// [`claudine::provider::provider_info`].
fn run_describe(format: ProvidersFormat) -> Result<()> {
    match format {
        ProvidersFormat::Json => {
            let payload: Vec<&'static claudine::provider::ProviderInfo> = all_providers().collect();
            let json = serde_json::to_string_pretty(&payload)
                .wrap_err("failed to serialize ProviderInfo catalog")?;
            log::data(&json);
        }
        ProvidersFormat::Text => {
            let term = crate::log::terminal();
            let columns = vec![
                TableColumn::new("Provider"),
                TableColumn::new("Slug"),
                TableColumn::new("Binary"),
                TableColumn::new("Sniff Binding"),
                TableColumn::new("Skills").with_alignment(Alignment::Center),
            ];
            let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
            table.layout_mut().margin = Edges::x(Length::ch(1));

            for provider in PROVIDERS_DISPLAY_ORDER {
                let info = provider_info(provider);
                table.add_row(vec![
                    info.display_name.into(),
                    info.slug.into(),
                    info.binary.into(),
                    format!("{:?}", info.sniff_binding).into(),
                    bool_indicator(info.supports_skills),
                ]);
            }
            let rendered = table.render(&term);
            log::data(&format!("\n{rendered}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_custom_command_support() {
        assert!(supports_custom_resource(
            Provider::Claude,
            LinkableResource::Command
        ));
        assert!(supports_custom_resource(
            Provider::Gemini,
            LinkableResource::Command
        ));
        assert!(supports_custom_resource(
            Provider::QwenCode,
            LinkableResource::Command
        ));
        assert!(!supports_custom_resource(
            Provider::KimiCode,
            LinkableResource::Command
        ));
    }

    #[test]
    fn reports_custom_agent_support() {
        assert!(supports_custom_resource(
            Provider::Claude,
            LinkableResource::Agent
        ));
        assert!(supports_custom_resource(
            Provider::KimiCode,
            LinkableResource::Agent
        ));
        assert!(supports_custom_resource(
            Provider::Gemini,
            LinkableResource::Agent
        ));
    }

    #[test]
    fn counts_hook_attach_points() {
        assert_eq!(supported_hook_count(Provider::Claude), 13);
        assert_eq!(supported_hook_count(Provider::Codex), 1);
        assert_eq!(supported_hook_count(Provider::Gemini), 10);
        assert_eq!(supported_hook_count(Provider::OpenCode), 13);
        assert_eq!(supported_hook_count(Provider::Goose), 0);
        assert_eq!(supported_hook_count(Provider::KimiCode), 0);
        assert_eq!(supported_hook_count(Provider::QwenCode), 0);
    }

    #[test]
    fn mapping_rows_project_every_source_kind() {
        let payload = serde_json::json!({
            "provider_scope": "all",
            "fields": [
                {
                    "field": "slug",
                    "source": { "kind": "roster", "key": "slug" },
                    "expected": ["string"],
                    "coercion": "string_literal",
                    "optional": false,
                    "description": "Canonical slug",
                },
                {
                    "field": "supports_skills",
                    "source": { "kind": "research", "topic": "skills", "path": "support" },
                    "expected": ["enum(subset of X)"],
                    "coercion": "skill_support_to_bool",
                    "optional": false,
                    "description": "Skill support",
                },
                {
                    "field": "stream_protocol",
                    "source": { "kind": "facts", "key": "stream_protocol" },
                    "expected": ["string", "record(requires )"],
                    "coercion": "stream_protocol_wire",
                    "optional": true,
                    "description": "Stream format",
                },
            ],
        });
        let rows = mapping_rows(&payload).expect("well-formed payload");
        assert_eq!(
            rows[0],
            MappingRow {
                field: "slug".into(),
                source: "roster:slug".into(),
                expected: "string".into(),
                coercion: "string_literal".into(),
                description: "Canonical slug".into(),
            }
        );
        assert_eq!(rows[1].source, "research:skills.support");
        // Alternatives join with ` | `; optional fields carry the suffix.
        assert_eq!(rows[2].source, "facts:stream_protocol");
        assert_eq!(rows[2].expected, "string | record(requires ) (optional)");
    }

    #[test]
    fn mapping_rows_reject_malformed_payloads() {
        assert!(mapping_rows(&serde_json::json!({})).is_err());
        assert!(
            mapping_rows(&serde_json::json!({ "fields": [ { "field": "x" } ] })).is_err(),
            "entry without source/coercion must error"
        );
    }

    #[test]
    fn describe_json_serializes_all_providers() {
        let payload: Vec<&claudine::provider::ProviderInfo> = all_providers().collect();
        let json = serde_json::to_value(&payload).expect("ProviderInfo serializes");
        let array = json.as_array().expect("payload is a JSON array");
        assert_eq!(array.len(), PROVIDERS_DISPLAY_ORDER.len());
        for (index, provider) in PROVIDERS_DISPLAY_ORDER.into_iter().enumerate() {
            let entry = &array[index];
            assert_eq!(
                entry["provider"],
                serde_json::to_value(provider).unwrap(),
                "entry {index} has unexpected provider"
            );
            assert!(
                entry.get("display_name").is_some(),
                "entry {index} missing display_name"
            );
            assert!(entry.get("slug").is_some(), "entry {index} missing slug");
            assert!(
                entry.get("docs_url").is_some(),
                "entry {index} missing docs_url"
            );
            // Typed catalog half plus resource portability. The legacy
            // AgentCapabilities facade must stay out of structured JSON.
            for key in [
                "event_mapping",
                "system_prompt",
                "yolo",
                "reasoning",
                "known_gaps",
                "acp",
                "output_formats",
                "entrypoints",
                "prompt_arg_conventions",
                "resource_support",
                "display_policy",
            ] {
                assert!(
                    entry.get(key).is_some(),
                    "entry {index} ({provider:?}) missing typed catalog field {key:?}"
                );
            }
            assert!(
                entry.get("capabilities").is_none(),
                "entry {index} ({provider:?}) unexpectedly serialized legacy capabilities"
            );
        }
    }
}
