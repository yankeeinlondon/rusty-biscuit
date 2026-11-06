//! Subcommand dispatch for the `md` CLI.

use crate::args::policy::{self, PolicyCommand};
use crate::args::{Cli, Command as CliCommand, SchemaTarget};
use crate::output::report::ReportSink;
use crate::output::{OutputEnvironment, resolve_output, stderr_warning};
use biscuit_terminal::output::WidthRequest;
use crate::request::MdRequest;
use clap::CommandFactory as _;
use color_eyre::eyre::Result;
use tracing::instrument;

mod code_block;
pub(crate) mod compose;
mod clean;
mod delta;
mod frontmatter;
mod graph;
mod hash;
mod render;
pub mod schema;
mod toc;
mod trigger_findings;
mod validate;

use code_block::{CodeBlockFlags, run_code_block};
use compose::{CollectionLimits, ComposeOutputWidth, build_remote_read_config, parse_compose_positionals, run_compose};
use frontmatter::{run_edit, run_get, run_rm, run_set};
use hash::{HashFlags, run_hash};

pub use clean::{CleanOptions, CleanSchemaFlags, run_clean};
pub use crate::output::report::ReportedExit;
use render::run_render;

/// Validates that top-level CLI options are not combined with subcommands.
pub fn validate_subcommand_usage(cli: &Cli) -> Result<()> {
    let mut conflicts = Vec::new();

    if cli.render.input.is_some() {
        conflicts.push("[INPUT]");
    }
    if cli.render.output.is_some() {
        conflicts.push("--output");
    }
    if cli.render.indent.is_some() {
        conflicts.push("--indent");
    }
    if cli.render.show {
        conflicts.push("--show");
    }
    if cli.save {
        conflicts.push("--save");
    }
    if !cli.render.deny.is_empty() {
        conflicts.push("--deny");
    }

    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(Cli::command()
            .error(
                clap::error::ErrorKind::ArgumentConflict,
                format!("subcommands cannot be combined with top-level options: {}", conflicts.join(", ")),
            )
            .into())
    }
}

#[instrument(skip_all)]
pub fn run_subcommand(command: CliCommand, cli: &Cli, request: &MdRequest) -> Result<()> {
    match command {
        CliCommand::Render(options) => run_render(&options, cli, request)?,
        CliCommand::Clean {
            input,
            save,
            indent,
            compact,
            loose,
            output,
            schema,
            baseline_schema,
            disable,
        } => {
            let policy = policy::resolve_for(
                PolicyCommand::Clean,
                &[],
                &[],
                &disable,
                baseline_schema.is_some().then_some("--baseline-schema <PATH>"),
            )?;
            let options = CleanOptions {
                save,
                indent,
                list_item_padding: clean::resolve_list_spacing(compact, loose),
                verbose: cli.verbose > 0,
                output: Some(output),
                schema: CleanSchemaFlags {
                    schema,
                    baseline_schema,
                    no_baseline_schema: policy.disables("baseline-schema"),
                    no_schema_triggers: policy.disables("schema-triggers"),
                },
            };
            run_clean(input.as_ref(), &options, cli, request)?;
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
            baseline_schema,
            allow,
            deny,
            disable,
            timeout,
            max_iteration_visits,
            max_expansion_bytes,
            shell,
            perf,
            remote_concurrency,
            remote_ttl,
            remote_refresh,
            remote_freshness,
            cache_root,
        } => {
            let mode = clean::resolve_list_spacing(compact, loose)
                .unwrap_or(darkmatter::markdown::cleanup::rules::ListItemPadding::Preserve);
            let policy = policy::resolve_for(
                PolicyCommand::Compose,
                &allow,
                &deny,
                &disable,
                baseline_schema.is_some().then_some("--baseline-schema <PATH>"),
            )?;
            let remote_config = build_remote_read_config(
                policy.hosts(),
                remote_concurrency,
                remote_ttl,
                remote_refresh,
                remote_freshness,
            );
            let resolved = resolve_output(
                output,
                cli.width,
                WidthRequest::Source,
                cli.verbose,
                &OutputEnvironment::detect(),
            )?;
            if output.is_data() {
                compose::reject_report_conflicts(output, shell)?;
            }
            let sink = output.is_data().then(|| {
                let file = parse_compose_positionals(&args)
                    .ok()
                    .and_then(|parsed| parsed.input)
                    .filter(|path| path.to_str() != Some("-"))
                    .map(|path| path.display().to_string());
                ReportSink::new(&resolved, file, show, perf)
            });
            if sink.is_none()
                && let Some(warning) = &resolved.width_warning
            {
                stderr_warning(warning);
            }
            let result = parse_compose_positionals(&args).and_then(|parsed| {
                run_compose(
                    parsed.input.as_ref(),
                    state.as_deref(),
                    set.as_deref(),
                    parsed.shorthand_setters,
                    ComposeOutputWidth::from_resolved(&resolved),
                    show,
                    frontmatter,
                    mode,
                    indent,
                    &policy,
                    baseline_schema.as_ref(),
                    timeout,
                    CollectionLimits { max_iteration_visits, max_expansion_bytes },
                    shell,
                    perf,
                    remote_config,
                    cache_root.as_ref(),
                    cli,
                    request,
                    sink.as_ref(),
                )
            });
            match sink {
                Some(sink) => sink.finish(result)?,
                None => result?,
            }
        }
        CliCommand::Toc { input, output } => toc::run_toc(input.as_ref(), output, cli, request)?,
        CliCommand::Delta { base, updated, output } => delta::run_delta(&base, &updated, output, cli, request)?,
        CliCommand::Get { input, props, output, compact } => run_get(&input, &props, output, compact, cli, request)?,
        CliCommand::Set { input, prop, value, save, output } => run_set(&input, &prop, &value, save, output, cli, request)?,
        CliCommand::Rm { input, props, output } => run_rm(&input, &props, output, cli, request)?,
        CliCommand::Edit { file, output } => run_edit(&file, output, cli, request)?,
        CliCommand::Hash { input, kind, body, frontmatter, save, diff, strict, output } => {
            let flags = HashFlags { kind: kind.map(Into::into), body, frontmatter, save, diff, strict };
            run_hash(input.as_ref(), flags, output, cli, request)?;
        }
        CliCommand::Validate { target } => validate::run_validate(target, cli, request)?,
        CliCommand::Graph { input, follow, validate, output } => graph::run_graph(&input, follow, validate, output, cli, request)?,
        CliCommand::CodeBlock { input, file, content, language, theme, title, line_numbering, highlight, output } => {
            let flags = CodeBlockFlags { force_file: file, force_content: content, language, theme, title, line_numbering, highlight };
            run_code_block(&input, &flags, output, cli, request)?;
        }
        CliCommand::Schema { target } => match target {
            SchemaTarget::Validate { inputs, schema, output, quiet, disable } => {
                let policy = policy::resolve_for(
                    PolicyCommand::SchemaValidate,
                    &[],
                    &[],
                    &disable,
                    schema.is_some().then_some("--schema <PATH>"),
                )?;
                let options = schema::ValidateOptions {
                    schema: schema.as_deref(),
                    no_baseline_schema: policy.disables("baseline-schema"),
                    no_schema_triggers: policy.disables("schema-triggers"),
                    quiet,
                    output,
                };
                schema::run_validate(&inputs, &options, cli, request)?;
            }
            SchemaTarget::Detect { files, output, merge } => schema::run_detect(&files, output, merge, cli, request)?,
            SchemaTarget::About { output } => schema::run_about(output, cli.code_block.into(), cli)?,
            SchemaTarget::Triggers { file, output } => schema::run_triggers(&file, output, cli, request)?,
        },
    }

    Ok(())
}
