use crate::args::{Cli, Command as CliCommand, OutputFormat};
use crate::output::{
    emit_or_show_artifact, html_artifact, json_artifact, markdown_artifact, open_output_artifact,
    print_delta, print_toc_tree, render_terminal_output, OutputArtifact,
};
use biscuit_hash::xx_hash;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::highlighting::{detect_code_theme, detect_color_mode, detect_prose_theme};
use darkmatter::markdown::transform::TransformOptions;
use darkmatter::markdown::{fs::collect_markdown_files, Markdown};
use rayon::prelude::*;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

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

    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(eyre!(
            "subcommands cannot be combined with top-level render options: {}",
            conflicts.join(", ")
        ))
    }
}

pub fn run_subcommand(command: CliCommand, cli: &Cli) -> Result<()> {
    match command {
        CliCommand::Read {
            input,
            output,
            show,
        } => {
            run_read(input.as_ref(), output, show, cli)?;
        }
        CliCommand::Clean { input } => {
            let mut md = load_markdown(input.as_ref())?;
            md.cleanup();
            println!("{}", md.as_string());
        }
        CliCommand::Compose {
            input,
            state,
            output,
            show,
        } => {
            run_compose(input.as_ref(), state.as_deref(), output, show, cli)?;
        }
        CliCommand::Toc { input, json } => {
            let md = load_markdown(Some(&input))?;
            let toc = md.toc();

            if json {
                println!("{}", serde_json::to_string_pretty(&toc)?);
            } else {
                print_toc_tree(&toc, cli.verbose > 0, None);
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
                print_delta(&delta, cli.verbose > 0, &base_md, &updated_md);
            }
        }
        CliCommand::Get {
            input,
            props,
            json5,
            yaml,
            toml,
        } => {
            run_get(&input, &props, json5, yaml, toml)?;
        }
        CliCommand::Hash {
            input,
            body,
            frontmatter,
            strict,
        } => {
            run_hash(input.as_ref(), body, frontmatter, strict)?;
        }
    }

    Ok(())
}

/// Shared read/render logic for both implicit (no subcommand) and explicit `read` subcommand.
pub fn run_read(input: Option<&PathBuf>, output: OutputFormat, show: bool, cli: &Cli) -> Result<()> {
    let md = load_markdown(input)?;

    let prose_theme = cli.theme.unwrap_or_else(detect_prose_theme);
    let code_theme = cli
        .code_theme
        .unwrap_or_else(|| detect_code_theme(prose_theme));
    let color_mode = detect_color_mode();
    let stdout_is_tty = io::stdout().is_terminal();

    match output {
        OutputFormat::Auto => {
            if stdout_is_tty {
                render_terminal_output(&md, input, cli, prose_theme, code_theme, color_mode)?;
                if show {
                    open_output_artifact(&markdown_artifact(&md))?;
                }
            } else {
                emit_or_show_artifact(markdown_artifact(&md), show)?;
            }
        }
        OutputFormat::Markdown => {
            emit_or_show_artifact(markdown_artifact(&md), show)?;
        }
        OutputFormat::Html => {
            let artifact = html_artifact(&md, prose_theme, code_theme, color_mode)?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Json => {
            let artifact = json_artifact(&md)?;
            emit_or_show_artifact(artifact, show)?;
        }
    }

    Ok(())
}

/// Run the compose (transform) pipeline.
pub fn run_compose(
    input: Option<&PathBuf>,
    state_json: Option<&str>,
    output: OutputFormat,
    show: bool,
    cli: &Cli,
) -> Result<()> {
    let md = load_markdown(input)?;

    let mut options = TransformOptions::new();

    // Parse --state as JSON if provided
    if let Some(json_str) = state_json {
        let state: serde_json::Value =
            serde_json::from_str(json_str).wrap_err("Invalid JSON in --state argument")?;
        if !state.is_object() {
            return Err(eyre!(
                "Invalid --state argument: expected a JSON object like {{\"name\":\"Alice\"}}"
            ));
        }
        options = options.with_external_state(state);
    }

    // Set source file for relative transclusion resolution
    if let Some(path) = input
        && path.to_str() != Some("-")
    {
        options = options.with_source_file(path);
    }

    let (transformed, _report) = md
        .transform_with(options)
        .map_err(|e| eyre!("Transform failed: {}", e))?;

    let prose_theme = cli.theme.unwrap_or_else(detect_prose_theme);
    let code_theme = cli
        .code_theme
        .unwrap_or_else(|| detect_code_theme(prose_theme));
    let color_mode = detect_color_mode();

    match output {
        OutputFormat::Auto | OutputFormat::Markdown => {
            // Frontmatter drives the pipeline; once composition is complete, discard it.
            let content = transformed.content().to_string();
            if show {
                let artifact = OutputArtifact {
                    content: content.clone(),
                    extension: "md",
                    label: "markdown",
                };
                println!("{content}");
                open_output_artifact(&artifact)?;
            } else {
                println!("{content}");
            }
        }
        OutputFormat::Html => {
            let artifact = html_artifact(&transformed, prose_theme, code_theme, color_mode)?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Json => {
            let artifact = json_artifact(&transformed)?;
            emit_or_show_artifact(artifact, show)?;
        }
    }

    Ok(())
}

/// Get frontmatter properties from a markdown document.
pub fn run_get(
    input: &PathBuf,
    props: &[String],
    json5: bool,
    yaml: bool,
    toml: bool,
) -> Result<()> {
    let md = load_markdown(Some(input))?;
    let fm = md.frontmatter();

    let value = if props.len() == 1 {
        // Single property: return the raw value (or empty string if missing)
        fm.as_map()
            .get(&props[0])
            .cloned()
            .unwrap_or(serde_json::Value::String(String::new()))
    } else {
        // Multiple properties: return a dictionary of requested keys
        let mut map = serde_json::Map::new();
        for prop in props {
            let v = fm
                .as_map()
                .get(prop)
                .cloned()
                .unwrap_or(serde_json::Value::String(String::new()));
            map.insert(prop.clone(), v);
        }
        serde_json::Value::Object(map)
    };

    let output = format_value(&value, json5, yaml, toml)?;
    println!("{output}");

    Ok(())
}

/// Format a `serde_json::Value` according to the requested output format.
fn format_value(
    value: &serde_json::Value,
    json5: bool,
    yaml: bool,
    toml: bool,
) -> Result<String> {
    if json5 {
        let json_str = serde_json::to_string(value)?;
        let j5 = biscuit_file::Json5::from_str(&json_str)
            .map_err(|e| eyre!("JSON5 conversion failed: {e}"))?;
        Ok(j5.as_json5())
    } else if yaml {
        let json_str = serde_json::to_string(value)?;
        let j5 = biscuit_file::Json5::from_str(&json_str)
            .map_err(|e| eyre!("YAML conversion failed: {e}"))?;
        let mut yaml_str = j5
            .as_yaml()
            .map_err(|e| eyre!("YAML conversion failed: {e}"))?;
        // serde_yaml appends a trailing newline; trim for consistent output
        if yaml_str.ends_with('\n') {
            yaml_str.truncate(yaml_str.len() - 1);
        }
        Ok(yaml_str)
    } else if toml {
        // TOML requires a table at the top level; wrap scalar values
        let toml_value = if value.is_object() {
            value.clone()
        } else {
            serde_json::json!({ "value": value })
        };
        let json_str = serde_json::to_string(&toml_value)?;
        let j5 = biscuit_file::Json5::from_str(&json_str)
            .map_err(|e| eyre!("TOML conversion failed: {e}"))?;
        let mut toml_str = j5
            .as_toml()
            .map_err(|e| eyre!("TOML conversion failed: {e}"))?;
        // Trim trailing newline for consistent output
        if toml_str.ends_with('\n') {
            toml_str.truncate(toml_str.len() - 1);
        }
        Ok(toml_str)
    } else {
        // Default: JSON
        Ok(serde_json::to_string_pretty(value)?)
    }
}

/// Hash a markdown document's frontmatter and/or body.
///
/// When the input is a directory, recursively finds all markdown files,
/// hashes each in parallel, concatenates the per-file hashes, and
/// produces a single aggregate hash.
pub fn run_hash(
    input: Option<&PathBuf>,
    body_only: bool,
    frontmatter_only: bool,
    strict: bool,
) -> Result<()> {
    // Directory mode: aggregate hash over all markdown files
    if let Some(path) = input
        && path.is_dir()
    {
        let mut paths = collect_markdown_files(path)?;
        paths.sort();

        let per_file_hashes: Vec<String> = paths
            .par_iter()
            .map(|p| {
                let md = Markdown::try_from(p.as_path())
                    .unwrap_or_else(|_| Markdown::from(String::new()));
                md.hash(body_only, frontmatter_only, strict)
            })
            .collect();

        let combined = per_file_hashes.join("\n");
        let aggregate = xx_hash(&combined);

        if body_only || frontmatter_only {
            println!("{:016x}", aggregate);
        } else {
            // Split each per-file "fm-body" hash, aggregate fm and body separately
            let (fm_parts, body_parts): (Vec<&str>, Vec<&str>) = per_file_hashes
                .iter()
                .filter_map(|h| h.split_once('-'))
                .unzip();

            let fm_aggregate = xx_hash(&fm_parts.join("\n"));
            let body_aggregate = xx_hash(&body_parts.join("\n"));
            println!("{:016x}-{:016x}", fm_aggregate, body_aggregate);
        }

        return Ok(());
    }

    // Single file / stdin mode
    let md = load_markdown(input)?;
    println!("{}", md.hash(body_only, frontmatter_only, strict));
    Ok(())
}

/// Loads markdown from a file path or stdin.
pub fn load_markdown(path: Option<&PathBuf>) -> Result<Markdown> {
    if let Some(p) = path {
        if p.to_str() == Some("-") {
            // Explicit stdin marker
            read_from_stdin()
        } else {
            Markdown::try_from(p.as_path())
                .wrap_err_with(|| format!("Failed to read file: {:?}", p))
        }
    } else {
        // No path provided - check if stdin has data
        if io::stdin().is_terminal() {
            // Interactive terminal - no input available
            Err(eyre!("No input file provided. Use `md --help` for usage."))
        } else {
            // Piped input available
            read_from_stdin()
        }
    }
}

/// Reads markdown content from stdin.
fn read_from_stdin() -> Result<Markdown> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .wrap_err("Failed to read from stdin")?;
    Ok(buffer.into())
}
