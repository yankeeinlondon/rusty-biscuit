use crate::args::{
    Cli, Command as CliCommand, GraphFormat, OutputFormat, SchemaTarget, ValidateOutputFormat,
    ValidateTarget,
};
use crate::output::{
    emit_or_show_artifact, html_artifact, json_artifact, markdown_artifact, markdown_plus_artifact,
    open_output_artifact, print_delta, print_toc_tree, render_terminal_output,
};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::cleanup::ListSpacingMode;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;
use tracing::{debug, info, instrument};

mod code_block;
mod compose;
mod frontmatter;
mod hash;
pub mod schema;

use code_block::run_code_block;

use compose::{ComposeAllowFlags, build_remote_read_config, parse_compose_positionals, run_compose};
use frontmatter::{run_edit, run_get, run_rm, run_set};
use hash::run_hash;

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
        } => {
            let mode = resolve_list_spacing(compact, loose);
            run_clean(input.as_ref(), save, indent, mode, cli.verbose > 0)?;
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
            let mode = resolve_list_spacing(compact, loose);
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
            run_validate(target)?;
        }
        CliCommand::Graph {
            input,
            follow,
            validate,
            json,
        } => {
            run_graph(&input, follow, validate, json)?;
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
            } => {
                schema::run_validate(&inputs, schema.as_deref(), format, quiet)?;
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
        },
    }

    Ok(())
}

/// Clean markdown formatting, optionally saving in place and printing a delta report.
/// Converts CLI flags to a `ListSpacingMode`.
fn resolve_list_spacing(compact: bool, loose: bool) -> ListSpacingMode {
    match (compact, loose) {
        (true, _) => ListSpacingMode::Compact,
        (_, true) => ListSpacingMode::Loose,
        _ => ListSpacingMode::Normal,
    }
}

#[instrument(skip_all)]
pub fn run_clean(
    input: Option<&PathBuf>,
    save: bool,
    indent: Option<usize>,
    list_spacing: ListSpacingMode,
    verbose: bool,
) -> Result<()> {
    if !save {
        let mut md = load_markdown(input)?;
        apply_cleanup(&mut md, indent, list_spacing);
        println!("{}", md.as_string());
        return Ok(());
    }

    let input_path = input
        .ok_or_else(|| eyre!("--save requires an input file path (stdin is not supported)"))?;
    if input_path.to_str() == Some("-") {
        return Err(eyre!(
            "--save requires an input file path (stdin is not supported)"
        ));
    }

    let resolved = resolve_file_path(input_path)?;
    let original = load_markdown(Some(input_path))?;
    let mut cleaned = original.clone();
    apply_cleanup(&mut cleaned, indent, list_spacing);

    let delta = original.delta(&cleaned);
    if !delta.is_unchanged() {
        std::fs::write(&resolved, cleaned.as_string())
            .wrap_err_with(|| format!("Failed to write cleaned markdown to {:?}", resolved))?;
    }

    print_delta(&delta, verbose, &original, &cleaned);
    Ok(())
}

fn apply_cleanup(md: &mut Markdown, indent: Option<usize>, mode: ListSpacingMode) {
    match (indent, mode) {
        (Some(size), ListSpacingMode::Compact) => md.cleanup_with_indent_compact(size),
        (Some(size), ListSpacingMode::Loose) => md.cleanup_with_indent_loose(size),
        (Some(size), ListSpacingMode::Normal) => md.cleanup_with_indent(size),
        (None, ListSpacingMode::Compact) => md.cleanup_compact(),
        (None, ListSpacingMode::Loose) => md.cleanup_loose(),
        (None, ListSpacingMode::Normal) => md.cleanup(),
    };
}

/// Shared render logic for both implicit (no subcommand) and explicit `render` subcommand.
#[instrument(skip_all, fields(command = "render"))]
pub fn run_render(
    input: Option<&PathBuf>,
    output: OutputFormat,
    show: bool,
    indent: Option<usize>,
    cli: &Cli,
) -> Result<()> {
    debug!("rendering document");
    let mut md = load_markdown(input)?;

    // Apply cleanup with the specified or default indentation
    let indent_size = indent.unwrap_or(darkmatter::markdown::cleanup::DEFAULT_INDENT);
    md.cleanup_with_indent(indent_size);

    // Only terminal rendering needs a real `Terminal`; MarkdownPlus and HTML
    // construct their own optimistic terminal inside the artifact builders so
    // they can resolve a single theme from the same terminal that renders the
    // page. Markdown, JSON, and non-TTY auto paths never resolve a theme.
    let stdout_is_tty = io::stdout().is_terminal();

    match output {
        OutputFormat::Auto => {
            if stdout_is_tty {
                let term = Terminal::new();
                render_terminal_output(
                    &md, input, cli, term,
                )?;
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
        OutputFormat::MarkdownPlus => {
            let artifact = markdown_plus_artifact(&md, cli, input,
            )?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Html => {
            let artifact = html_artifact(&md, cli, input,
            )?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Json => {
            let artifact = json_artifact(&md)?;
            emit_or_show_artifact(artifact, show)?;
        }
    }

    Ok(())
}

/// Loads markdown from a file path or stdin.
///
/// Paths are resolved through biscuit-file's `FileReference` system, which
/// supports `@`-prefixed magic paths (e.g. `@prompts/feature.md`), vault
/// references, and other reference syntaxes. Plain paths and `-` (stdin)
/// are handled as before.
pub fn load_markdown(path: Option<&PathBuf>) -> Result<Markdown> {
    if let Some(p) = path {
        if p.to_str() == Some("-") {
            // Explicit stdin marker
            read_from_stdin()
        } else {
            let resolved = resolve_file_path(p)?;
            Markdown::try_from(resolved.as_path())
                .wrap_err_with(|| format!("Failed to read file: {:?}", resolved))
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

/// Resolves a file path through biscuit-file's `FileReference` system.
///
/// If the path contains `@`-prefixed magic references or other FileReference
/// syntax, it will be resolved accordingly. Plain paths are returned as-is
/// (made absolute if relative).
fn resolve_file_path(raw_path: &PathBuf) -> Result<PathBuf> {
    use biscuit_file::FileReference;

    let raw = raw_path.to_string_lossy();
    match FileReference::new(&raw) {
        Ok(file_ref) => {
            let resolved = file_ref
                .resolve()
                .wrap_err_with(|| format!("Failed to resolve file reference: {:?}", raw_path))?;
            match resolved {
                Some(p) => Ok(p),
                None => {
                    // FileReference couldn't resolve it — fall back to raw path
                    Err(eyre!("Failed to load file: {:?}", raw_path))
                }
            }
        }
        Err(_) => {
            // Not a valid file reference syntax — treat as plain path
            Ok(raw_path.clone())
        }
    }
}

/// Reads markdown content from stdin.
fn read_from_stdin() -> Result<Markdown> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .wrap_err("Failed to read from stdin")?;
    Markdown::try_from_content(buffer).map_err(Into::into)
}

#[instrument(skip_all, fields(command = "validate"))]
fn run_validate(target: ValidateTarget) -> Result<()> {
    info!("starting reference validation");
    use darkmatter::markdown::reference::ReferenceGraphOptions;
    use darkmatter::markdown::reference::validate::ReferenceValidationOptions;

    match target {
        ValidateTarget::Refs {
            input,
            remote,
            fragments,
            timeout,
            fail_fast,
            format,
            show_all,
            graph,
        } => {
            let md = Markdown::try_from(input.as_path())
                .wrap_err_with(|| format!("Failed to load {}", input.display()))?;

            // If --graph requested, print graph and exit
            if let Some(graph_format) = graph {
                let graph_options = ReferenceGraphOptions::default();
                let ref_graph = md
                    .reference_graph(graph_options)
                    .wrap_err("Failed to build reference graph")?;

                match graph_format {
                    GraphFormat::Mermaid => println!("{}", ref_graph.to_mermaid()),
                    GraphFormat::Dot => println!("{}", ref_graph.to_dot()),
                }
                return Ok(());
            }

            let options = ReferenceValidationOptions {
                graph: ReferenceGraphOptions::default(),
                validate_remote: remote,
                remote_timeout: std::time::Duration::from_secs(timeout),
                validate_fragments: fragments,
                fail_fast,
            };

            let report = md
                .validate_references(options)
                .wrap_err("Reference validation failed")?;

            match format {
                ValidateOutputFormat::Text => {
                    print_validation_report_text(&report, &input, show_all);
                }
                ValidateOutputFormat::Json => {
                    print_validation_report_json(&report)?;
                }
            }

            if report.is_valid() {
                Ok(())
            } else {
                Err(eyre!("{} error(s) found", report.error_count()))
            }
        }
    }
}

fn print_validation_report_text(
    report: &darkmatter::markdown::reference::validate::ReferenceValidationReport,
    input: &std::path::Path,
    show_all: bool,
) {
    use darkmatter::markdown::reference::validate::ReferenceSeverity;

    println!("References scanned: {}", report.references_scanned);
    println!("Valid: {}", report.references_valid);
    println!("Issues: {}", report.issues.len());

    if !report.issues.is_empty() {
        println!();
    }

    for issue in &report.issues {
        let severity = match issue.severity {
            ReferenceSeverity::Error => "ERROR",
            ReferenceSeverity::Warning => "WARN ",
            ReferenceSeverity::Info => "INFO ",
        };

        let source = match &issue.origin.source {
            darkmatter::markdown::compose::ComposeSource::File(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| input.display().to_string()),
            _ => input.display().to_string(),
        };

        println!(
            "{severity}  {source}:{line}  {message}",
            line = issue.origin.line,
            message = issue.message
        );
    }

    if show_all && !report.warnings.is_empty() {
        println!();
        for warning in &report.warnings {
            println!("WARN   {warning}");
        }
    }
}

fn print_validation_report_json(
    report: &darkmatter::markdown::reference::validate::ReferenceValidationReport,
) -> Result<()> {
    use darkmatter::markdown::reference::validate::ReferenceSeverity;

    let issues: Vec<serde_json::Value> = report
        .issues
        .iter()
        .map(|i| {
            serde_json::json!({
                "code": format!("{:?}", i.code),
                "message": i.message,
                "severity": match i.severity {
                    ReferenceSeverity::Error => "error",
                    ReferenceSeverity::Warning => "warning",
                    ReferenceSeverity::Info => "info",
                },
                "reference_id": i.reference_id,
                "line": i.origin.line,
            })
        })
        .collect();

    let json = serde_json::json!({
        "references_scanned": report.references_scanned,
        "references_valid": report.references_valid,
        "issues": issues,
        "warnings": report.warnings,
        "is_valid": report.is_valid(),
    });

    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

/// Category label for a reference kind used in grouped validation output.
fn reference_kind_category_label(
    kind: darkmatter::markdown::reference::types::ReferenceKind,
) -> &'static str {
    use darkmatter::markdown::reference::types::ReferenceKind;
    match kind {
        ReferenceKind::Hyperlink => "Invalid Hyperlink(s)",
        ReferenceKind::Image => "Invalid Image Reference(s)",
        ReferenceKind::HtmlVideo | ReferenceKind::HtmlAudio | ReferenceKind::HtmlSource => {
            "Invalid Media Reference(s)"
        }
        ReferenceKind::HtmlIframe => "Invalid Iframe(s)",
        ReferenceKind::Transclusion => "Invalid Transclusion Target(s)",
        ReferenceKind::CssImport | ReferenceKind::InlineCss => "Invalid CSS Import(s)",
        ReferenceKind::ScriptImport | ReferenceKind::InlineScript => "Invalid Script Import(s)",
        ReferenceKind::FontImport => "Invalid Font Import(s)",
        ReferenceKind::MetaTag => "Invalid Meta Tag(s)",
    }
}

/// Formats grouped validation issues as a styled string.
///
/// Issues are grouped by [`ReferenceKind`] category, then listed by source
/// document with the broken reference highlighted in red. Returns an empty
/// string if there are no error-severity issues.
fn format_validation_issues(
    report: &darkmatter::markdown::reference::validate::ReferenceValidationReport,
    term: &biscuit_terminal::terminal::Terminal,
) -> String {
    use biscuit_terminal::components::list::UnorderedList;
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::TerminalRenderable as _;
    use darkmatter::markdown::compose::ComposeSource;
    use darkmatter::markdown::reference::types::ReferenceKind;
    use darkmatter::markdown::reference::validate::ReferenceSeverity;
    use std::collections::BTreeMap;

    // Only show error-severity issues in grouped output
    let errors: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.severity == ReferenceSeverity::Error)
        .collect();

    if errors.is_empty() {
        return String::new();
    }

    let mut out = String::new();

    // Group by kind, preserving discovery order within each group
    let mut groups: BTreeMap<u8, (ReferenceKind, Vec<_>)> = BTreeMap::new();
    let kind_order = |k: ReferenceKind| -> u8 {
        match k {
            ReferenceKind::Transclusion => 0,
            ReferenceKind::Hyperlink => 1,
            ReferenceKind::Image => 2,
            ReferenceKind::HtmlVideo | ReferenceKind::HtmlAudio | ReferenceKind::HtmlSource => 3,
            ReferenceKind::HtmlIframe => 4,
            ReferenceKind::CssImport | ReferenceKind::InlineCss => 5,
            ReferenceKind::ScriptImport | ReferenceKind::InlineScript => 6,
            ReferenceKind::FontImport => 7,
            ReferenceKind::MetaTag => 8,
        }
    };

    for issue in &errors {
        let key = kind_order(issue.kind);
        groups
            .entry(key)
            .or_insert_with(|| (issue.kind, Vec::new()))
            .1
            .push(*issue);
    }

    out.push('\n');

    for (kind, issues) in groups.values() {
        let label = reference_kind_category_label(*kind);
        let header = format!("<red-500><b>{label}</b></red-500>");
        out.push_str(&Prose::new(header).render(term));
        out.push('\n');

        let mut list = UnorderedList::empty();
        for issue in issues {
            let source_name = match &issue.origin.source {
                ComposeSource::File(p) => p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                ComposeSource::Url(u) => u.to_string(),
                ComposeSource::Unknown => "unknown".to_string(),
            };

            let source_href = match &issue.origin.source {
                ComposeSource::File(p) => {
                    format!("file://{}", p.display())
                }
                _ => String::new(),
            };

            let item_text = if source_href.is_empty() {
                format!(
                    "the <blue-500>{source_name}</blue-500> reference to <red-500>{ref_display}</red-500> is not valid",
                    ref_display = issue.reference_display,
                )
            } else {
                format!(
                    "the <a href=\"{source_href}\"><blue-500>{source_name}</blue-500></a> reference to <red-500>{ref_display}</red-500> is not valid",
                    ref_display = issue.reference_display,
                )
            };

            list.add(Prose::new(item_text));
        }

        out.push_str(&list.render(term));
        out.push('\n');
    }

    // Summary line
    let issue_count = errors.len();
    let summary = format!(
        "{} references scanned, {} valid, <red-500><b>{} issues</b></red-500>",
        report.references_scanned, report.references_valid, issue_count
    );
    out.push_str(&Prose::new(summary).render(term));
    out.push('\n');

    out
}

// ── Graph JSON serialization ──────────────────────────────────────────

/// Serialize a `ComposeSource` to a JSON-friendly path string.
fn source_to_json(source: &darkmatter::markdown::compose::ComposeSource) -> serde_json::Value {
    use darkmatter::markdown::compose::ComposeSource;
    match source {
        ComposeSource::File(p) => serde_json::Value::String(p.display().to_string()),
        ComposeSource::Url(u) => serde_json::Value::String(u.to_string()),
        ComposeSource::Unknown => serde_json::Value::Null,
    }
}

/// Serialize a `ReferenceKind` to a snake_case string.
fn kind_to_json(kind: darkmatter::markdown::reference::types::ReferenceKind) -> &'static str {
    use darkmatter::markdown::reference::types::ReferenceKind;
    match kind {
        ReferenceKind::Hyperlink => "hyperlink",
        ReferenceKind::Image => "image",
        ReferenceKind::HtmlVideo => "html_video",
        ReferenceKind::HtmlAudio => "html_audio",
        ReferenceKind::HtmlSource => "html_source",
        ReferenceKind::HtmlIframe => "html_iframe",
        ReferenceKind::Transclusion => "transclusion",
        ReferenceKind::CssImport => "css_import",
        ReferenceKind::InlineCss => "inline_css",
        ReferenceKind::ScriptImport => "script_import",
        ReferenceKind::InlineScript => "inline_script",
        ReferenceKind::FontImport => "font_import",
        ReferenceKind::MetaTag => "meta_tag",
    }
}

/// Serialize a `ReferenceTarget` to a JSON object with `type` and `raw`.
fn target_to_json(
    target: &darkmatter::markdown::reference::types::ReferenceTarget,
) -> serde_json::Value {
    use darkmatter::markdown::reference::types::ReferenceTarget;
    match target {
        ReferenceTarget::LocalPath { raw } => {
            serde_json::json!({ "type": "local_path", "raw": raw })
        }
        ReferenceTarget::RemoteUrl { raw } => {
            serde_json::json!({ "type": "remote_url", "raw": raw })
        }
        ReferenceTarget::Fragment { raw } => {
            serde_json::json!({ "type": "fragment", "raw": raw })
        }
        ReferenceTarget::DataUri { raw } => {
            serde_json::json!({ "type": "data_uri", "raw": raw })
        }
        ReferenceTarget::OtherScheme { raw, scheme } => {
            serde_json::json!({ "type": "other_scheme", "raw": raw, "scheme": scheme })
        }
        ReferenceTarget::Inline => {
            serde_json::json!({ "type": "inline" })
        }
    }
}

/// Serialize a `ReferenceSyntax` to a snake_case string.
fn syntax_to_json(syntax: darkmatter::markdown::reference::types::ReferenceSyntax) -> &'static str {
    use darkmatter::markdown::reference::types::ReferenceSyntax;
    match syntax {
        ReferenceSyntax::MarkdownLink => "markdown_link",
        ReferenceSyntax::HtmlAnchor => "html_anchor",
        ReferenceSyntax::MarkdownImage => "markdown_image",
        ReferenceSyntax::HtmlImage => "html_image",
        ReferenceSyntax::HtmlVideoTag => "html_video_tag",
        ReferenceSyntax::HtmlAudioTag => "html_audio_tag",
        ReferenceSyntax::HtmlSourceTag => "html_source_tag",
        ReferenceSyntax::HtmlIframeTag => "html_iframe_tag",
        ReferenceSyntax::DirectiveFile => "directive_file",
        ReferenceSyntax::DirectiveUrl => "directive_url",
        ReferenceSyntax::DirectiveCode => "directive_code",
        ReferenceSyntax::DirectiveTocLinking => "directive_toc_linking",
        ReferenceSyntax::DirectiveFileLinks => "directive_file_links",
        ReferenceSyntax::FrontmatterPrologue => "frontmatter_prologue",
        ReferenceSyntax::FrontmatterEpilogue => "frontmatter_epilogue",
        ReferenceSyntax::HtmlLinkTag => "html_link_tag",
        ReferenceSyntax::HtmlScriptTag => "html_script_tag",
        ReferenceSyntax::HtmlStyleTag => "html_style_tag",
        ReferenceSyntax::CssAtImport => "css_at_import",
        ReferenceSyntax::CssFontFaceSrc => "css_font_face_src",
        ReferenceSyntax::HtmlMetaTag => "html_meta_tag",
    }
}

/// Serialize a transclusion directive kind to a snake_case string.
fn directive_kind_to_json(
    syntax: darkmatter::markdown::reference::types::ReferenceSyntax,
) -> &'static str {
    use darkmatter::markdown::reference::types::ReferenceSyntax;
    match syntax {
        ReferenceSyntax::DirectiveFile => "file",
        ReferenceSyntax::DirectiveUrl => "url",
        ReferenceSyntax::DirectiveCode => "code",
        ReferenceSyntax::DirectiveTocLinking => "toc_linking",
        ReferenceSyntax::DirectiveFileLinks => "file_links",
        ReferenceSyntax::FrontmatterPrologue => "prologue",
        ReferenceSyntax::FrontmatterEpilogue => "epilogue",
        other => syntax_to_json(other),
    }
}

/// Serialize a single `ReferenceRecord` to JSON.
fn reference_record_to_json(
    record: &darkmatter::markdown::reference::types::ReferenceRecord,
) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "id": record.id,
        "kind": kind_to_json(record.kind),
        "target": target_to_json(&record.target),
        "syntax": syntax_to_json(record.origin.syntax),
        "line": record.origin.line,
    });

    if !record.attributes.is_empty() {
        obj["attributes"] = serde_json::Value::Object(record.attributes.clone());
    }

    obj
}

/// Serialize a `ReferenceInsertion` (transclusion) to JSON, optionally expanding child nodes.
fn insertion_to_json(
    insertion: &darkmatter::markdown::reference::types::ReferenceInsertion,
    graph: &darkmatter::markdown::reference::types::ReferenceGraph,
    follow: bool,
) -> serde_json::Value {
    let kind_str = insertion
        .context
        .directive_kind
        .map(directive_kind_to_json)
        .unwrap_or("unknown");

    // Find the target path from the child node
    let child_node = graph.node_by_id(insertion.child_node_id.as_ref());
    let target = child_node
        .map(|n| source_to_json(&n.source))
        .unwrap_or(serde_json::Value::Null);

    let mut obj = serde_json::json!({
        "kind": kind_str,
        "target": target,
        "line": insertion.directive_line,
        "followable": insertion.context.directive_kind
            .map(|s| s.is_followable_transclusion())
            .unwrap_or(false),
    });

    if let Some(ref heading) = insertion.context.section_heading_text {
        obj["section"] = serde_json::Value::String(heading.clone());
    }
    if let Some(level) = insertion.context.section_heading_level {
        obj["section_level"] = serde_json::Value::Number(level.as_u8().into());
    }

    // Recursively expand child node when following
    if follow && let Some(child) = child_node {
        obj["node"] = graph_node_to_json(child, graph, true);
    }

    obj
}

/// Serialize a single graph node to JSON.
fn graph_node_to_json(
    node: &darkmatter::markdown::reference::types::ReferenceGraphNode,
    graph: &darkmatter::markdown::reference::types::ReferenceGraph,
    follow: bool,
) -> serde_json::Value {
    use darkmatter::markdown::compose::ComposeSource;

    let file_name = match &node.source {
        ComposeSource::File(p) => p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        ComposeSource::Url(u) => u.to_string(),
        ComposeSource::Unknown => String::new(),
    };

    let references: Vec<_> = node
        .local_references
        .records
        .iter()
        // Exclude transclusion records (they appear under "transclusions")
        .filter(|r| r.kind != darkmatter::markdown::reference::types::ReferenceKind::Transclusion)
        .map(reference_record_to_json)
        .collect();

    let transclusions: Vec<_> = node
        .child_insertions
        .iter()
        .map(|ins| insertion_to_json(ins, graph, follow))
        .collect();

    serde_json::json!({
        "file": file_name,
        "source": source_to_json(&node.source),
        "references": references,
        "transclusions": transclusions,
    })
}

/// Serialize validation report to JSON.
fn validation_report_to_json(
    report: &darkmatter::markdown::reference::validate::ReferenceValidationReport,
) -> serde_json::Value {
    use darkmatter::markdown::compose::ComposeSource;
    use darkmatter::markdown::reference::validate::ReferenceSeverity;

    let issues: Vec<_> = report
        .issues
        .iter()
        .map(|i| {
            let source_file = match &i.origin.source {
                ComposeSource::File(p) => serde_json::Value::String(p.display().to_string()),
                ComposeSource::Url(u) => serde_json::Value::String(u.to_string()),
                ComposeSource::Unknown => serde_json::Value::Null,
            };
            serde_json::json!({
                "code": format!("{:?}", i.code),
                "message": i.message,
                "severity": match i.severity {
                    ReferenceSeverity::Error => "error",
                    ReferenceSeverity::Warning => "warning",
                    ReferenceSeverity::Info => "info",
                },
                "kind": kind_to_json(i.kind),
                "reference": i.reference_display,
                "line": i.origin.line,
                "source": source_file,
            })
        })
        .collect();

    serde_json::json!({
        "valid": report.is_valid(),
        "references_scanned": report.references_scanned,
        "references_valid": report.references_valid,
        "issues": issues,
        "warnings": report.warnings,
    })
}

#[instrument(skip_all)]
fn run_graph(input: &PathBuf, follow: bool, validate: bool, json: bool) -> Result<()> {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;
    use darkmatter::markdown::reference::file_tree::FileTree;

    let resolved = resolve_file_path(input)?;
    let mut tree = FileTree::new(&resolved).map_err(|e| eyre!("{e}"))?;

    if follow {
        tree = tree.follow_transclusions();
    }
    if validate {
        tree = tree.validate();
    }

    tree.ensure_built().map_err(|e| eyre!("{e}"))?;

    // ── JSON output ──────────────────────────────────────────────────
    if json {
        if let Some(graph) = tree.graph() {
            let mut root_json = graph_node_to_json(&graph.root, graph, follow);

            if validate && let Some(report) = tree.validation_report() {
                root_json["validation"] = validation_report_to_json(report);
            }

            println!("{}", serde_json::to_string_pretty(&root_json)?);
        }
        // JSON mode always exits 0
        return Ok(());
    }

    // ── Terminal tree output ─────────────────────────────────────────
    let term = Terminal::default();
    print!("{}", tree.display(&term));

    // Validation summary footer
    if validate && let Some(report) = tree.validation_report() {
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::renderable::TerminalRenderable as _;

        let formatted = format_validation_issues(report, &term);
        if !formatted.is_empty() {
            println!("{formatted}");
        } else {
            let summary = format!(
                "{} references scanned, {} valid, 0 issues",
                report.references_scanned, report.references_valid,
            );
            println!("\n{}", Prose::new(summary).render(&term));
        }

        // Exit code 2 for validation errors
        if !report.is_valid() {
            std::process::exit(2);
        }
    }

    Ok(())
}

