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
}

/// Arguments accepted by `claudine providers generate`.
#[derive(Debug, Args)]
pub struct GenerateArgs {
    /// Print the generator's mapping registry (field -> source -> coercion)
    /// as JSON. Without this flag, runs the report-only drift check.
    #[arg(long)]
    pub mapping: bool,
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
    if let Some(ProvidersCommand::Generate(generate)) = args.command {
        return run_generate(generate);
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

/// Shells out to the `claudine-gen` binary (raw stdout pass-through; CLI
/// rendering of the mapping arrives with generator v1 in Phase B).
fn run_generate(args: GenerateArgs) -> Result<()> {
    let gen_arg = if args.mapping { "mapping" } else { "check" };
    let mut command = match resolve_gen_binary() {
        Some(binary) => Command::new(binary),
        None => match repo_root_for_cargo_fallback() {
            Some(root) => {
                let mut cargo = Command::new("cargo");
                cargo
                    .args(["run", "-p", "claudine-gen", "--quiet", "--"])
                    .current_dir(root);
                cargo
            }
            None => bail!(
                "could not find the `claudine-gen` binary (looked next to this \
                 executable and on PATH) and no repo root is detectable for a \
                 `cargo run -p claudine-gen` fallback.\n\
                 Install it with `cargo install --path claudine/gen` or run from \
                 inside the rusty-biscuit repo."
            ),
        },
    };
    let status = command
        .arg(gen_arg)
        .status()
        .wrap_err("failed to launch claudine-gen")?;
    if !status.success() {
        bail!("claudine-gen {gen_arg} exited with {status}");
    }
    Ok(())
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
