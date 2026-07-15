//! Subcommand dispatch for the `md` CLI.

use crate::args::{Cli, Command as CliCommand, OutputFormat, SchemaTarget};
use crate::io::load_markdown;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::toc::TocTree;
use tracing::instrument;

mod code_block;
mod compose;
mod clean;
mod frontmatter;
mod graph;
mod hash;
mod render;
pub mod schema;
mod validate;

use code_block::run_code_block;
use compose::{ComposeAllowFlags, build_remote_read_config, parse_compose_positionals, run_compose};
use frontmatter::{run_edit, run_get, run_rm, run_set};
use hash::run_hash;

pub use clean::run_clean;
pub use render::run_render;

/// Validates that top-level CLI options are not combined with subcommands.
pub fn validate_subcommand_usage(cli: &Cli) -> Result<()> {
    let mut conflicts = Vec::new();

    if cli.input.is_some() {
        conflicts.push("[INPUT]");
    }
    if cli.output != OutputFormat::Auto {
        conflicts.push("--output");
    }
    if cli.show {
        conflicts.push("--show");
    }
    if cli.save {
        conflicts.push("--save");
    }

    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(eyre!(
            "subcommands cannot be combined with top-level options: {}",
            conflicts.join(", ")
        ))
    }
}

#[instrument(skip_all)]
pub fn run_subcommand(command: CliCommand, cli: &Cli) -> Result<()> {
    match command {
        CliCommand::Render {
            input,
            output,
            show,
            indent,
        } => {
            run_render(input.as_ref(), output, show, indent, cli)?;
        }
        CliCommand::Clean {
            input,
            save,
            indent,
            compact,
            loose,
            fixed_width,
            ignore_incidental_newlines,
        } => {
            let mode = clean::resolve_list_spacing(compact, loose);
            run_clean(
                input.as_ref(),
                save,
                indent,
                mode,
                fixed_width,
                ignore_incidental_newlines,
                cli.verbose > 0,
            )?;
        }
        CliCommand::Compose {
            args,
            state,
            set,
            output,
            show,
            frontmatter,
            compact,
            loose,
            indent,
            allow_missing_hyperlinks,
            allow_missing_image_refs,
            allow_missing_transclusions,
            allow_any_missing_reference,
            allow_ctx_override,
            allow_invalid_frontmatter_assignment,
            allow_reassigned_frontmatter_property,
            baseline_schema,
            no_baseline_schema,
            no_trigger_schemas,
            timeout,
            allow_shell_timeout,
            shell,
            perf,
            allow_host,
            remote_concurrency,
            remote_ttl,
            remote_refresh,
            remote_freshness,
            cache_root,
        } => {
            let parsed = parse_compose_positionals(&args)?;
            let mode = clean::resolve_list_spacing(compact, loose);
            let allow = ComposeAllowFlags {
                hyperlinks: allow_missing_hyperlinks || allow_any_missing_reference,
                image_refs: allow_missing_image_refs || allow_any_missing_reference,
                transclusions: allow_missing_transclusions || allow_any_missing_reference,
            };
            let remote_config = build_remote_read_config(
                &allow_host,
                remote_concurrency,
                remote_ttl,
                remote_refresh,
                remote_freshness,
            );
            run_compose(
                parsed.input.as_ref(),
                state.as_deref(),
                set.as_deref(),
                parsed.shorthand_setters,
                output,
                show,
                frontmatter,
                mode,
                indent,
                &allow,
                allow_ctx_override,
                allow_invalid_frontmatter_assignment,
                allow_reassigned_frontmatter_property,
                baseline_schema.as_ref(),
                no_baseline_schema,
                no_trigger_schemas,
                timeout,
                allow_shell_timeout,
                shell,
                perf,
                remote_config,
                cache_root.as_ref(),
                cli,
            )?;
        }
        CliCommand::Toc { input, json } => {
            let md = load_markdown(input.as_ref())?;
            let toc = md.toc();

            if json {
                println!("{}", serde_json::to_string_pretty(&toc)?);
            } else {
                // Detect the terminal only on the human-rendered branch so
                // `md toc --json` never pays terminal detection (finding 3).
                let term = Terminal::new();
                let mut tree = TocTree::new(toc);
                if cli.verbose > 0 {
                    tree = tree.verbose();
                }
                print!("{}", tree.render(&term));
            }
        }
        CliCommand::Delta {
            base,
            updated,
            json,
        } => {
            let base_md = load_markdown(Some(&base))
                .wrap_err_with(|| format!("Failed to read base file: {:?}", base))?;
            let updated_md = load_markdown(Some(&updated))
                .wrap_err_with(|| format!("Failed to read updated file: {:?}", updated))?;
            let delta = base_md.delta(&updated_md);

            if json {
                println!("{}", serde_json::to_string_pretty(&delta)?);
            } else {
                use darkmatter::markdown::delta::DeltaReport;

                let mut report =
                    DeltaReport::new(delta).with_documents(base_md.clone(), updated_md.clone());
                if cli.verbose > 0 {
                    report = report.verbose();
                }
                print!("{}", report.render(&Terminal::new()));
            }
        }
        CliCommand::Get {
            input,
            props,
            json5,
            yaml,
            toml,
            raw,
            compact,
        } => {
            run_get(&input, &props, json5, yaml, toml, raw, compact)?;
        }
        CliCommand::Set {
            input,
            prop,
            value,
            save,
        } => {
            run_set(&input, &prop, &value, save)?;
        }
        CliCommand::Rm { input, props, json } => {
            run_rm(&input, &props, json, cli)?;
        }
        CliCommand::Edit { file } => {
            run_edit(&file)?;
        }
        CliCommand::Hash {
            input,
            kind,
            body,
            frontmatter,
            save,
            diff,
            strict,
        } => {
            run_hash(
                input.as_ref(),
                kind.map(Into::into),
                body,
                frontmatter,
                save,
                diff,
                strict,
            )?;
        }
        CliCommand::Validate { target } => {
            validate::run_validate(target)?;
        }
        CliCommand::Graph {
            input,
            follow,
            validate,
            json,
        } => {
            graph::run_graph(&input, follow, validate, json)?;
        }
        CliCommand::CodeBlock {
            input,
            file,
            content,
            language,
            theme,
            title,
            line_numbering,
            highlight,
            output,
        } => {
            run_code_block(
                &input,
                file,
                content,
                language.as_deref(),
                theme,
                title.as_deref(),
                line_numbering,
                highlight.as_deref(),
                output,
                cli,
            )?;
        }
        CliCommand::Schema { target } => match target {
            SchemaTarget::Validate {
                inputs,
                schema,
                format,
                quiet,
                no_trigger_schemas,
            } => {
                schema::run_validate(
                    &inputs,
                    schema.as_deref(),
                    format,
                    quiet,
                    no_trigger_schemas,
                )?;
            }
            SchemaTarget::Detect {
                files,
                format,
                merge,
            } => {
                schema::run_detect(&files, format, merge)?;
            }
            SchemaTarget::About => {
                schema::run_about(cli.verbose > 0, cli.code_block.into())?;
            }
            SchemaTarget::Triggers { file } => schema::run_triggers(&file)?,
        },
    }

    Ok(())
}
