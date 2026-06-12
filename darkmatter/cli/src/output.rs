use crate::args::{Cli, CliFill};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::layout::{DarkmatterPage, PageComponent};
use darkmatter::markdown::highlighting::{ColorMode, ThemePair};
use darkmatter::markdown::output::MermaidMode;
use darkmatter::markdown::output::terminal::TerminalImageMode;
use darkmatter::markdown::{Markdown, MarkdownDelta, MarkdownToc, MarkdownTocNode};
use darkmatter::markdown::block::scan_inline_hr_warnings;
use darkmatter::style::{
    BespokeStyleOverrides, ComponentStyleOverrides, HrStyleOverrides, ListStyleOverrides,
    PageStyleOverrides, StyleWarning, StyleWarningKind, apply_bespoke_style, apply_color_style,
    apply_component_style, apply_hr_style, apply_list_style, apply_page_style, from_frontmatter,
    into_strict,
};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct OutputArtifact {
    pub content: String,
    pub extension: &'static str,
    pub label: &'static str,
}

pub fn render_terminal_output(
    md: &Markdown,
    input_path: Option<&PathBuf>,
    cli: &Cli,
    term: Terminal,
    prose_theme: ThemePair,
    code_theme: ThemePair,
    color_mode: ColorMode,
) -> Result<()> {
    // The `Terminal` is now passed in by the caller (`run_render` in
    // `commands.rs`) so the page surface and the nested code-block panel
    // resolve against the same source — Phase 2's "single `Terminal` is
    // the source of truth" invariant. The `color_mode` parameter is kept
    // for the `DarkmatterPage::with_color_mode` call so the page's
    // `LayoutContext::from_page` uses it as the fallback for
    // `ColorMode::Unknown` (matching the layout context's `surface_mode`
    // resolution).
    let mut page = DarkmatterPage::new(&term)
        .with_prose_theme(prose_theme.kebab_name())
        .with_code_theme(code_theme.kebab_name())
        .with_color_mode(color_mode)
        .with_code_block_mode(cli.code_block.into())
        .with_image_mode(terminal_image_mode_from_env())
        .with_mermaid_mode(if cli.mermaid {
            MermaidMode::Image
        } else {
            MermaidMode::Off
        });

    if let Some(path) = input_path
        && path.to_str() != Some("-")
    {
        page = page.with_base_path(path.parent().map(|p| p.to_path_buf()).unwrap_or_default());
    }

    // Apply layout flags from CLI.
    page = apply_cli_layout_flags(page, cli);

    // Apply page-level frontmatter style after CLI flags so CLI wins on
    // overlapping fields via PageStyleOverrides.
    page = apply_style_frontmatter(page, md, cli, input_path)?;

    // Handle line numbers: CLI flag overrides default.
    if let Some(on) = cli.line_numbers {
        page = page.with_line_numbers(on);
    }

    let output = page
        .render(md)
        .context("Failed to render markdown for terminal")?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(output.as_bytes())
        .context("Failed to write terminal output")?;

    Ok(())
}

/// Apply CLI layout flags to a [`DarkmatterPage`].
///
/// Precedence: margin shorthand → axis → side-specific.
/// Same for padding. Alignment: global → component-specific.
/// Fill: global → component-specific.
pub fn apply_cli_layout_flags(page: DarkmatterPage, cli: &Cli) -> DarkmatterPage {
    let mut page = page;

    // Margin precedence: all > axis > side
    if let Some(n) = cli.margin {
        page = page.with_margin(n);
    }
    if let Some(n) = cli.mx {
        page = page.with_margin_x(n);
    }
    if let Some(n) = cli.my {
        page = page.with_margin_y(n);
    }
    if let Some(n) = cli.mt {
        page = page.with_margin_top(n);
    }
    if let Some(n) = cli.mb {
        page = page.with_margin_bottom(n);
    }
    if let Some(n) = cli.ml {
        page = page.with_margin_left(n);
    }
    if let Some(n) = cli.mr {
        page = page.with_margin_right(n);
    }

    // Padding precedence: all > axis > side
    if let Some(n) = cli.padding {
        page = page.with_padding(n);
    }
    if let Some(n) = cli.px {
        page = page.with_padding_x(n);
    }
    if let Some(n) = cli.py {
        page = page.with_padding_y(n);
    }
    if let Some(n) = cli.pt {
        page = page.with_padding_top(n);
    }
    if let Some(n) = cli.pb {
        page = page.with_padding_bottom(n);
    }
    if let Some(n) = cli.pl {
        page = page.with_padding_left(n);
    }
    if let Some(n) = cli.pr {
        page = page.with_padding_right(n);
    }

    // Page background
    if let Some(bg) = cli.page_bg {
        page = page.with_page_background(bg.into());
    }

    // Explicit page background color (overrides the computed `PageBackground`).
    if let Some(color) = cli.page_bg_color {
        page = page.with_page_bg_color(color);
    }

    // Max width
    if let Some(n) = cli.max_width {
        page = page.with_max_width(n);
    }

    // Alignment precedence: global > component-specific
    if let Some(align) = cli.alignment {
        for component in PageComponent::ALL {
            page = apply_component_alignment(page, component, align.into());
        }
    }
    if let Some(align) = cli.align_images {
        page = apply_component_alignment(page, PageComponent::Images, align.into());
    }
    if let Some(align) = cli.align_lists {
        for component in [PageComponent::Ul, PageComponent::Ol, PageComponent::Li] {
            page = apply_component_alignment(page, component, align.into());
        }
    }
    if let Some(align) = cli.align_ul {
        page = apply_component_alignment(page, PageComponent::Ul, align.into());
    }
    if let Some(align) = cli.align_ol {
        page = apply_component_alignment(page, PageComponent::Ol, align.into());
    }
    if let Some(align) = cli.align_li {
        page = apply_component_alignment(page, PageComponent::Li, align.into());
    }
    if let Some(align) = cli.align_block_quotes {
        page = apply_component_alignment(page, PageComponent::BlockQuotes, align.into());
    }
    if let Some(align) = cli.align_tables {
        page = apply_component_alignment(page, PageComponent::Tables, align.into());
    }
    if let Some(align) = cli.align_code_blocks {
        page = apply_component_alignment(page, PageComponent::CodeBlocks, align.into());
    }

    // Fill precedence: global > component-specific
    if let Some(ref fill) = cli.fill {
        for component in PageComponent::ALL {
            page = apply_component_fill(page, component, fill);
        }
    }
    if let Some(ref fill) = cli.fill_images {
        page = apply_component_fill(page, PageComponent::Images, fill);
    }
    if let Some(ref fill) = cli.fill_lists {
        for component in [PageComponent::Ul, PageComponent::Ol, PageComponent::Li] {
            page = apply_component_fill(page, component, fill);
        }
    }
    if let Some(ref fill) = cli.fill_ul {
        page = apply_component_fill(page, PageComponent::Ul, fill);
    }
    if let Some(ref fill) = cli.fill_ol {
        page = apply_component_fill(page, PageComponent::Ol, fill);
    }
    if let Some(ref fill) = cli.fill_li {
        page = apply_component_fill(page, PageComponent::Li, fill);
    }
    if let Some(ref fill) = cli.fill_block_quotes {
        page = apply_component_fill(page, PageComponent::BlockQuotes, fill);
    }
    if let Some(ref fill) = cli.fill_tables {
        page = apply_component_fill(page, PageComponent::Tables, fill);
    }
    if let Some(ref fill) = cli.fill_code_blocks {
        page = apply_component_fill(page, PageComponent::CodeBlocks, fill);
    }

    page
}

/// Set `alignment` on `component`, merging with any existing [`ComponentPolicy`].
fn apply_component_alignment(
    page: DarkmatterPage,
    component: PageComponent,
    alignment: renderable::layout::Alignment,
) -> DarkmatterPage {
    let mut policy = page.component_policy(component).cloned().unwrap_or_default();
    policy.layout.alignment = alignment;
    page.with_component_policy(component, policy)
}

/// Apply a [`CliFill`] to `component`, merging with any existing [`ComponentPolicy`].
fn apply_component_fill(
    page: DarkmatterPage,
    component: PageComponent,
    fill: &CliFill,
) -> DarkmatterPage {
    use renderable::layout::{Edges, TargetValue, Width};

    let mut policy = page.component_policy(component).cloned().unwrap_or_default();
    match fill {
        CliFill::Full => {
            policy.layout.width = Width::Auto;
            policy.layout.max_width = None;
            policy.layout.padding = Edges::default();
        }
        CliFill::Pad(length) => {
            policy.layout.padding = Edges::x(length.clone());
        }
        CliFill::Indent(length) => {
            policy.layout.padding = match policy.layout.alignment {
                renderable::layout::Alignment::Left => Edges {
                    left: TargetValue::universal(length.clone()),
                    ..Edges::default()
                },
                renderable::layout::Alignment::Right => Edges {
                    right: TargetValue::universal(length.clone()),
                    ..Edges::default()
                },
                renderable::layout::Alignment::Center => Edges::x(length.clone()),
            };
        }
        CliFill::Max(length) => {
            policy.layout.max_width = Some(TargetValue::universal(length.clone()));
        }
        CliFill::Explicit(length) => {
            policy.layout.width = Width::Fixed(TargetValue::universal(length.clone()));
        }
    }
    page.with_component_policy(component, policy)
}

/// Build a [`PageStyleOverrides`] reflecting which `style.page.*` fields the
/// CLI has already claimed.
///
/// Mirrors the shorthand expansion rules in [`apply_cli_layout_flags`]:
/// `--margin` claims all four sides, `--mx` claims left/right, `--my` claims
/// top/bottom. Padding follows the same pattern. `--max-width`, `--page-bg`,
/// and `--alignment` each claim their corresponding page-level field. The
/// component-specific alignment flags (`--align-images`, `--align-lists`,
/// `--align-block-quotes`, `--align-tables`, `--align-code-blocks`) each
/// claim their component so the `style.page.alignment` broadcast does not
/// silently overwrite them.
pub fn page_style_overrides_from_cli(cli: &Cli) -> PageStyleOverrides {
    let margin_all = cli.margin.is_some();
    let mx = cli.mx.is_some();
    let my = cli.my.is_some();
    let padding_all = cli.padding.is_some();
    let px = cli.px.is_some();
    let py = cli.py.is_some();

    PageStyleOverrides {
        margin_top: margin_all || my || cli.mt.is_some(),
        margin_right: margin_all || mx || cli.mr.is_some(),
        margin_bottom: margin_all || my || cli.mb.is_some(),
        margin_left: margin_all || mx || cli.ml.is_some(),
        padding_top: padding_all || py || cli.pt.is_some(),
        padding_right: padding_all || px || cli.pr.is_some(),
        padding_bottom: padding_all || py || cli.pb.is_some(),
        padding_left: padding_all || px || cli.pl.is_some(),
        max_width: cli.max_width.is_some(),
        background: cli.page_bg.is_some(),
        background_color: cli.page_bg_color.is_some(),
        alignment: cli.alignment.is_some(),
        align_images: cli.align_images.is_some(),
        align_lists: cli.align_lists.is_some(),
        align_ul: cli.align_ul.is_some(),
        align_ol: cli.align_ol.is_some(),
        align_li: cli.align_li.is_some(),
        align_block_quotes: cli.align_block_quotes.is_some(),
        align_tables: cli.align_tables.is_some(),
        align_code_blocks: cli.align_code_blocks.is_some(),
    }
}

/// Build a [`ListStyleOverrides`] reflecting which list-bucket frontmatter
/// fields the CLI has already claimed.
///
/// Mirrors the global/component-specific precedence in
/// [`apply_cli_layout_flags`]: the global `--alignment` flag claims every
/// `*_alignment` field, the global `--fill` flag claims every `*_fill` field,
/// the broadcast `--align-lists` / `--fill-lists` flags claim all three list
/// components, and the granular flags (`--align-ul`, `--fill-ol`, etc.) each
/// claim only their own field.
pub fn list_style_overrides_from_cli(cli: &Cli) -> ListStyleOverrides {
    let alignment_all = cli.alignment.is_some();
    let fill_all = cli.fill.is_some();
    let align_lists_broadcast = cli.align_lists.is_some();
    let fill_lists_broadcast = cli.fill_lists.is_some();

    ListStyleOverrides {
        ul_alignment: alignment_all || align_lists_broadcast || cli.align_ul.is_some(),
        ul_fill: fill_all || fill_lists_broadcast || cli.fill_ul.is_some(),
        ul_left_margin: false,
        ol_alignment: alignment_all || align_lists_broadcast || cli.align_ol.is_some(),
        ol_fill: fill_all || fill_lists_broadcast || cli.fill_ol.is_some(),
        li_alignment: alignment_all || align_lists_broadcast || cli.align_li.is_some(),
        li_fill: fill_all || fill_lists_broadcast || cli.fill_li.is_some(),
    }
}

/// Build a [`ComponentStyleOverrides`] reflecting which component-bucket
/// frontmatter fields the CLI has already claimed.
///
/// Mirrors the global/component-specific precedence in
/// [`apply_cli_layout_flags`]: the global `--alignment` flag claims every
/// `*_alignment` field, the global `--fill` flag claims every `*_fill` field,
/// and the component-specific flags (`--align-tables`, `--align-images`,
/// `--align-block-quotes`, `--fill-tables`, `--fill-images`,
/// `--fill-block-quotes`) each claim only their own field.
pub fn component_style_overrides_from_cli(cli: &Cli) -> ComponentStyleOverrides {
    let alignment_all = cli.alignment.is_some();
    let fill_all = cli.fill.is_some();

    ComponentStyleOverrides {
        tables_alignment: alignment_all || cli.align_tables.is_some(),
        tables_fill: fill_all || cli.fill_tables.is_some(),
        images_alignment: alignment_all || cli.align_images.is_some(),
        images_fill: fill_all || cli.fill_images.is_some(),
        block_quotes_alignment: alignment_all || cli.align_block_quotes.is_some(),
        block_quotes_fill: fill_all || cli.fill_block_quotes.is_some(),
    }
}

/// Build an [`HrStyleOverrides`] reflecting which HR frontmatter fields the CLI
/// has already claimed.
///
/// There are currently no HR-specific CLI flags, so this always returns the
/// default (no overrides). It exists for symmetry with the other override
/// helpers and to make adding HR CLI flags a one-line change.
pub fn hr_style_overrides_from_cli(_cli: &Cli) -> HrStyleOverrides {
    HrStyleOverrides::default()
}

/// Build a [`BespokeStyleOverrides`] reflecting which bespoke frontmatter
/// fields the CLI has already claimed.
///
/// Mirrors the CLI precedence in [`apply_cli_layout_flags`]: `--code-theme`
/// claims the `style.page.code.theme` field so the frontmatter value is
/// skipped.
pub fn bespoke_style_overrides_from_cli(cli: &Cli) -> BespokeStyleOverrides {
    BespokeStyleOverrides {
        code_theme: cli.code_theme.is_some(),
    }
}

/// Parse the `style:` frontmatter (if any), promote schema warnings to errors
/// when `--strict-style` is set, log remaining warnings, and apply the
/// page-level subset to `page` using the CLI's override summary.
pub fn apply_style_frontmatter(
    page: DarkmatterPage,
    md: &Markdown,
    cli: &Cli,
    input_path: Option<&PathBuf>,
) -> Result<DarkmatterPage> {
    let (style, all_warnings) =
        from_frontmatter(md.frontmatter()).context("Failed to parse `style:` frontmatter")?;

    // `--strict-style` promotes schema issues (UnknownKey / Deprecated) to
    // errors, but informational `KnownButInactive` warnings must still flow
    // through `log_style_warnings` so `RUST_LOG=darkmatter=info` users see
    // future-phase keys regardless of strict mode.
    let (style, warnings) = if cli.strict_style {
        let (mut schema, informational): (Vec<_>, Vec<_>) = all_warnings
            .into_iter()
            .partition(StyleWarning::is_schema_issue);
        // Also reject inline HR deprecation warnings in strict mode.
        let inline_hr_warnings = scan_inline_hr_warnings(md.content());
        schema.extend(inline_hr_warnings);
        let style = into_strict((style, schema))
            .context("`style:` frontmatter rejected by --strict-style")?;
        (style, informational)
    } else {
        (style, all_warnings)
    };

    log_style_warnings(&warnings);

    let page_overrides = page_style_overrides_from_cli(cli);
    let page = apply_page_style(page, &style, page_overrides)
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))?;

    let component_overrides = component_style_overrides_from_cli(cli);
    let page = apply_component_style(page, &style, component_overrides)
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))?;

    let list_overrides = list_style_overrides_from_cli(cli);
    let page = apply_list_style(page, &style, list_overrides)
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))?;

    let hr_overrides = hr_style_overrides_from_cli(cli);
    let page = apply_hr_style(page, &style, hr_overrides)
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))?;

    let page = apply_color_style(page, &style)
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))?;

    let bespoke_overrides = bespoke_style_overrides_from_cli(cli);
    let source_path = input_path
        .filter(|p| p.to_str() != Some("-"))
        .map(|p| p.as_path());
    apply_bespoke_style(page, &style, bespoke_overrides, source_path)
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))
}

/// Log non-fatal style warnings via `tracing`.
///
/// `KnownButInactive` is informational; `UnknownKey` and `Deprecated` are
/// warnings (but only when not promoted to errors by `--strict-style`).
fn log_style_warnings(warnings: &[StyleWarning]) {
    for w in warnings {
        match &w.kind {
            StyleWarningKind::UnknownKey => {
                tracing::warn!(path = %w.path, "unknown style key");
            }
            StyleWarningKind::Deprecated { replacement } => {
                tracing::warn!(
                    path = %w.path,
                    replacement = %replacement,
                    "deprecated style key",
                );
            }
            StyleWarningKind::KnownButInactive { sub_spec } => {
                tracing::info!(
                    path = %w.path,
                    sub_spec = sub_spec,
                    "style key parsed but not yet wired",
                );
            }
        }
    }
}

pub fn markdown_artifact(md: &Markdown) -> OutputArtifact {
    OutputArtifact {
        content: md.as_string(),
        extension: "md",
        label: "markdown",
    }
}

pub fn html_artifact(
    md: &Markdown,
    prose_theme: ThemePair,
    code_theme: ThemePair,
    color_mode: ColorMode,
    cli: &Cli,
    input_path: Option<&PathBuf>,
) -> Result<OutputArtifact> {
    let term = Terminal::new_optimistic(120);
    let mut page = apply_cli_layout_flags(
        DarkmatterPage::new(&term)
            .with_prose_theme(prose_theme.kebab_name())
            .with_code_theme(code_theme.kebab_name())
            .with_color_mode(color_mode),
        cli,
    );

    page = apply_style_frontmatter(page, md, cli, input_path)?;

    let content = page
        .render_to_browser(md)
        .context("Failed to convert to HTML")?;

    Ok(OutputArtifact {
        content,
        extension: "html",
        label: "html",
    })
}

pub fn json_artifact(md: &Markdown) -> Result<OutputArtifact> {
    let ast = md.as_ast().context("Failed to generate AST")?;
    let content = serde_json::to_string_pretty(&ast)?;
    Ok(OutputArtifact {
        content,
        extension: "json",
        label: "json",
    })
}

pub fn emit_or_show_artifact(artifact: OutputArtifact, show: bool) -> Result<()> {
    if show {
        open_output_artifact(&artifact)
    } else {
        println!("{}", artifact.content);
        Ok(())
    }
}

pub fn open_output_artifact(artifact: &OutputArtifact) -> Result<()> {
    let temp_path = write_output_artifact_file(artifact)?;

    // MD_DRY_RUN=1 writes the temp file but skips launching the viewer.
    // Useful in tests and CI where opening a GUI app is undesirable.
    if std::env::var("MD_DRY_RUN").is_ok_and(|v| !v.is_empty()) {
        return Ok(());
    }

    // Non-blocking open, graceful error handling
    if let Err(error) = open::that(&temp_path) {
        eprintln!("Failed to open {} output: {}", artifact.label, error);
        eprintln!("Preview available at: {}", temp_path.display());
    }

    Ok(())
}

fn write_output_artifact_file(artifact: &OutputArtifact) -> Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let filename = format!(
        "md-output-{}-{}.{}",
        std::process::id(),
        timestamp,
        artifact.extension
    );
    let path = std::env::temp_dir().join(filename);

    std::fs::write(&path, &artifact.content)
        .wrap_err_with(|| format!("Failed to write {} output file", artifact.label))?;

    Ok(path)
}

pub fn terminal_image_mode_from_env() -> TerminalImageMode {
    let Ok(raw) = std::env::var("TERMINAL_IMAGES") else {
        return TerminalImageMode::Auto;
    };

    match parse_bool_env(&raw) {
        Some(true) => TerminalImageMode::Force,
        Some(false) => TerminalImageMode::Never,
        None => {
            tracing::warn!(value = %raw, "Invalid TERMINAL_IMAGES value; falling back to auto mode");
            TerminalImageMode::Auto
        }
    }
}

pub fn parse_bool_env(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "y" => Some(true),
        "0" | "false" | "no" | "off" | "n" => Some(false),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TOC Tree Output
// ─────────────────────────────────────────────────────────────────────────────

/// Prints the table of contents as a text-based tree.
///
/// If `filename` is provided, it will be displayed in bold after the document icon.
pub fn print_toc_tree(toc: &MarkdownToc, verbose: bool, filename: Option<&str>) {
    // ANSI escape codes
    const BOLD: &str = "\x1b[1m";
    const RESET: &str = "\x1b[0m";

    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut out = stdout.lock();
    let mut err = stderr.lock();

    // Breathing room: blank line to stderr before TOC
    writeln!(err).ok();

    // Print document icon, optionally with filename in bold
    if toc.title.is_some() {
        if let Some(name) = filename {
            writeln!(out, "📄 {BOLD}{name}{RESET}").ok();
        } else {
            writeln!(out, "📄").ok();
        }
        if verbose {
            writeln!(
                err,
                "   Page hash: {:016x} (trimmed: {:016x})",
                toc.page_hash, toc.page_hash_trimmed
            )
            .ok();
        }
    }

    // Print the tree structure
    for (i, node) in toc.structure.iter().enumerate() {
        let is_last = i == toc.structure.len() - 1;
        print_toc_node(&mut out, node, "", is_last, verbose);
    }

    // Breathing room: blank line to stderr after TOC
    writeln!(err).ok();

    // Print summary only in verbose mode, to stderr
    if verbose {
        writeln!(
            err,
            "Total: {} heading{}",
            toc.heading_count(),
            if toc.heading_count() == 1 { "" } else { "s" }
        )
        .ok();

        if !toc.code_blocks.is_empty() {
            writeln!(err, "Code blocks: {}", toc.code_blocks.len()).ok();
        }

        if !toc.internal_links.is_empty() {
            let broken_count = toc.broken_links().len();
            if broken_count > 0 {
                writeln!(
                    err,
                    "Internal links: {} ({} broken)",
                    toc.internal_links.len(),
                    broken_count
                )
                .ok();
            } else {
                writeln!(err, "Internal links: {}", toc.internal_links.len()).ok();
            }
        }
    }
}

/// Recursively prints a TOC node with tree characters.
fn print_toc_node<W: Write>(
    out: &mut W,
    node: &MarkdownTocNode,
    prefix: &str,
    is_last: bool,
    verbose: bool,
) {
    // Tree connector characters
    let connector = if is_last { "└── " } else { "├── " };
    let child_prefix = if is_last { "    " } else { "│   " };

    if verbose {
        // Show semantic content hash (used for whitespace-insensitive comparison)
        writeln!(
            out,
            "{}{}{} ({:016x})",
            prefix,
            connector,
            node.title,
            node.prelude_hash_normalized()
        )
        .ok();
    } else {
        writeln!(out, "{}{}{}", prefix, connector, node.title).ok();
    }

    // Print children
    let new_prefix = format!("{}{}", prefix, child_prefix);
    for (i, child) in node.children.iter().enumerate() {
        let child_is_last = i == node.children.len() - 1;
        print_toc_node(out, child, &new_prefix, child_is_last, verbose);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Delta Output
// ─────────────────────────────────────────────────────────────────────────────

// ANSI escape codes
const INVERSE: &str = "\x1b[7m";
const BOLD: &str = "\x1b[1m";
const ITALIC: &str = "\x1b[3m";
const RESET: &str = "\x1b[0m";

/// Formats a code block change with ANSI styling.
///
/// Format: `{inverse}lang{reset} code block in {bold}section{reset} {description}`
pub fn format_code_block_change(lang: &str, section_path: &str, description: &str) -> String {
    // Parse the description to determine change type and format accordingly
    if let Some(rest) = description.strip_prefix("Language: ") {
        // Language change: "Language: none → text"
        format!(
            "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} 
             changed its {BOLD}language{RESET} setting: {rest}"
        )
    } else if let Some(rest) = description.strip_prefix("'") {
        // Property change: "'title': "old" → "new"" -> "title property: "old" → "new""
        if let Some((prop_name, value_part)) = rest.split_once("':") {
            format!(
                "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} 
                 changed its {BOLD}{prop_name}{RESET} property:{value_part}"
            )
        } else {
            // Fallback if parsing fails
            format!(
                "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} 
                 changed: {description}"
            )
        }
    } else if description.starts_with("Modified") {
        // Content modified
        format!(
            "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} 
             was {BOLD}modified{RESET}"
        )
    } else if description.starts_with("Added") {
        // Added code block
        format!("{INVERSE}{lang}{RESET} code block added in {BOLD}{section_path}{RESET}")
    } else if description.starts_with("Removed") {
        // Removed code block
        format!("{INVERSE}{lang}{RESET} code block removed from {BOLD}{section_path}{RESET}")
    } else {
        // Fallback for other descriptions
        format!("{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET}: {description}")
    }
}

/// Prints the delta comparison results.
pub fn print_delta(delta: &MarkdownDelta, verbose: bool, original: &Markdown, updated: &Markdown) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Blank line before output for visual separation
    writeln!(handle).ok();

    // Print classification header
    let (classification_symbol, classification_name) = match delta.classification {
        darkmatter::markdown::DocumentChange::NoChange => ("✓", "No changes"),
        darkmatter::markdown::DocumentChange::WhitespaceOnly => ("~", "Whitespace changes only"),
        darkmatter::markdown::DocumentChange::FrontmatterOnly => ("◈", "Frontmatter only"),
        darkmatter::markdown::DocumentChange::FrontmatterAndWhitespace => {
            ("◈", "Frontmatter and whitespace")
        }
        darkmatter::markdown::DocumentChange::StructuralOnly => ("⊕", "Structural only"),
        darkmatter::markdown::DocumentChange::ContentMinor => ("△", "Minor changes"),
        darkmatter::markdown::DocumentChange::ContentModerate => ("◐", "Moderate changes"),
        darkmatter::markdown::DocumentChange::ContentMajor => ("◉", "Major changes"),
        darkmatter::markdown::DocumentChange::Rewritten => ("★", "Rewritten"),
    };

    writeln!(
        handle,
        "{} {} ({:.1}% changed)",
        classification_symbol,
        classification_name,
        delta.statistics.content_change_ratio * 100.0
    )
    .ok();
    writeln!(handle).ok();

    // Print frontmatter changes
    if delta.frontmatter_changed {
        writeln!(handle, "Frontmatter:").ok();
        if delta.frontmatter_formatting_only {
            writeln!(handle, "  (formatting changes only)").ok();
        } else {
            for change in &delta.frontmatter_changes {
                let symbol = match change.action {
                    darkmatter::markdown::ChangeAction::PropertyAdded => "+",
                    darkmatter::markdown::ChangeAction::PropertyRemoved => "-",
                    darkmatter::markdown::ChangeAction::PropertyUpdated => "~",
                    _ => "?",
                };
                writeln!(
                    handle,
                    "  {} {}: {}",
                    symbol, change.key, change.description
                )
                .ok();
            }
        }
        writeln!(handle).ok();
    }

    // Print preamble changes
    if delta.preamble_changed {
        if delta.preamble_whitespace_only {
            writeln!(handle, "Preamble: whitespace changes only").ok();
        } else {
            writeln!(handle, "Preamble: modified").ok();
        }
        writeln!(handle).ok();
    }

    // Print added sections
    if !delta.added.is_empty() {
        writeln!(handle, "Added ({}):", delta.added.len()).ok();
        for change in &delta.added {
            let path_str = change
                .new_path
                .as_ref()
                .map(|p| p.join(" > "))
                .unwrap_or_default();
            if verbose {
                writeln!(
                    handle,
                    "  + {} (line {})",
                    path_str,
                    change.new_line.unwrap_or(0)
                )
                .ok();
            } else {
                writeln!(handle, "  + {}", path_str).ok();
            }
        }
        writeln!(handle).ok();
    }

    // Print removed sections
    if !delta.removed.is_empty() {
        writeln!(handle, "Removed ({}):", delta.removed.len()).ok();
        for change in &delta.removed {
            let path_str = change
                .original_path
                .as_ref()
                .map(|p| p.join(" > "))
                .unwrap_or_default();
            if verbose {
                writeln!(
                    handle,
                    "  - {} (was line {})",
                    path_str,
                    change.original_line.unwrap_or(0)
                )
                .ok();
            } else {
                writeln!(handle, "  - {}", path_str).ok();
            }
        }
        writeln!(handle).ok();
    }

    // Separate content changes from whitespace-only changes
    let content_changes: Vec<_> = delta
        .modified
        .iter()
        .filter(|c| !matches!(c.action, darkmatter::markdown::ChangeAction::WhitespaceOnly))
        .collect();
    let whitespace_changes: Vec<_> = delta
        .modified
        .iter()
        .filter(|c| matches!(c.action, darkmatter::markdown::ChangeAction::WhitespaceOnly))
        .collect();

    // Print content modifications (the important ones)
    if !content_changes.is_empty() {
        writeln!(handle, "Modified ({}):", content_changes.len()).ok();
        for change in &content_changes {
            writeln!(handle, "  - {}", change.description).ok();
        }
        writeln!(handle).ok();
    }

    // Print moved sections
    if !delta.moved.is_empty() {
        writeln!(handle, "Moved ({}):", delta.moved.len()).ok();
        for moved in &delta.moved {
            let from = moved.original_path.join(" > ");
            let to = moved.new_path.join(" > ");
            let level_change = if moved.level_delta < 0 {
                format!(" (promoted by {})", -moved.level_delta)
            } else if moved.level_delta > 0 {
                format!(" (demoted by {})", moved.level_delta)
            } else {
                String::new()
            };
            writeln!(handle, "  ↷ {} → {}{}", from, to, level_change).ok();
        }
        writeln!(handle).ok();
    }

    // Print code block changes (always show, not just verbose)
    if !delta.code_block_changes.is_empty() {
        writeln!(handle, "Code blocks:").ok();
        for change in &delta.code_block_changes {
            let lang = change.language.as_deref().unwrap_or("plain");
            // Skip H1 in section path (start from index 1 if it exists)
            let section_path = if change.section_path.len() > 1 {
                change.section_path[1..].join(" > ")
            } else if !change.section_path.is_empty() {
                change.section_path[0].clone()
            } else {
                String::from("(preamble)")
            };

            // Format with ANSI styling based on change type
            // inverse=\x1b[7m, bold=\x1b[1m, italic=\x1b[3m, reset=\x1b[0m
            let formatted = format_code_block_change(lang, &section_path, &change.description);
            writeln!(handle, "  - {}", formatted).ok();
        }
        writeln!(handle).ok();
    }

    // Print broken links
    if !delta.broken_links.is_empty() {
        writeln!(handle, "⚠ Broken links ({}):", delta.broken_links.len()).ok();
        for link in &delta.broken_links {
            write!(
                handle,
                "  ✗ #{} at line {}",
                link.target_slug, link.line_number
            )
            .ok();
            if let Some(ref suggestion) = link.suggested_replacement {
                writeln!(handle, " → did you mean #{}?", suggestion).ok();
            } else {
                writeln!(handle).ok();
            }
        }
        writeln!(handle).ok();
    }

    // Print whitespace-only changes at the end (less important)
    if !whitespace_changes.is_empty() {
        writeln!(handle, "Whitespace only ({}):", whitespace_changes.len()).ok();
        for change in &whitespace_changes {
            // Skip H1 in section path (start from index 1 if it exists)
            let path_str = change
                .original_path
                .as_ref()
                .map(|p| {
                    if p.len() > 1 {
                        p[1..].join(" > ")
                    } else if !p.is_empty() {
                        p[0].clone()
                    } else {
                        String::from("(preamble)")
                    }
                })
                .unwrap_or_default();
            // description contains the whitespace type(s) - show in italics
            writeln!(
                handle,
                "  - {}: {ITALIC}{}{RESET}",
                path_str, change.description
            )
            .ok();
        }
        // Dim italic note after the list
        writeln!(handle).ok();
        writeln!(
            handle,
            "  \x1b[2m\x1b[3mwhitespace only changes have no visual effect when rendered\x1b[0m"
        )
        .ok();
        writeln!(handle).ok();
    }

    // Print summary statistics if verbose
    if verbose {
        let stats = &delta.statistics;
        writeln!(handle, "Statistics:").ok();
        writeln!(
            handle,
            "  Bytes: {} → {} ({} changed)",
            stats.original_bytes, stats.new_bytes, stats.bytes_changed
        )
        .ok();
        writeln!(
            handle,
            "  Sections: {} → {} ({} unchanged)",
            stats.original_section_count, stats.new_section_count, stats.sections_unchanged
        )
        .ok();
        writeln!(handle).ok();

        // Visual diff output
        use darkmatter::diff::visual::{VisualDiffInput, VisualDiffOptions, render_visual_diff};

        let options = VisualDiffOptions::default();

        // Frontmatter visual diff (if changed)
        if delta.frontmatter_changed && !delta.frontmatter_formatting_only {
            let fm_orig =
                serde_yaml::to_string(original.frontmatter().as_map()).unwrap_or_default();
            let fm_upd = serde_yaml::to_string(updated.frontmatter().as_map()).unwrap_or_default();

            if !fm_orig.is_empty() || !fm_upd.is_empty() {
                writeln!(handle, "{BOLD}Frontmatter Visual Diff:{RESET}").ok();
                writeln!(
                    handle,
                    "{}",
                    render_visual_diff(
                        VisualDiffInput {
                            original: &fm_orig,
                            updated: &fm_upd,
                            label_original: "original",
                            label_updated: "updated",
                        },
                        &options,
                    )
                    .rendered
                )
                .ok();
            }
        }

        // Content body visual diff (if has content changes)
        let has_content_changes = !delta.added.is_empty()
            || !delta.removed.is_empty()
            || !delta.modified.is_empty()
            || delta.preamble_changed;

        if has_content_changes {
            writeln!(
                handle,
                "{}",
                render_visual_diff(
                    VisualDiffInput {
                        original: original.content(),
                        updated: updated.content(),
                        label_original: "original",
                        label_updated: "updated",
                    },
                    &options,
                )
                .rendered
            )
            .ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parse_bool_env_supports_truthy_values() {
        for value in ["1", "true", "TRUE", "yes", "on", "y"] {
            assert_eq!(parse_bool_env(value), Some(true), "value: {value}");
        }
    }

    #[test]
    fn parse_bool_env_supports_falsy_values() {
        for value in ["0", "false", "FALSE", "no", "off", "n"] {
            assert_eq!(parse_bool_env(value), Some(false), "value: {value}");
        }
    }

    #[test]
    fn parse_bool_env_rejects_unknown_values() {
        for value in ["", "maybe", "2", "enable", "disable"] {
            assert_eq!(parse_bool_env(value), None, "value: {value}");
        }
    }

    fn cli_from(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("CLI should parse")
    }

    #[test]
    fn strict_style_flag_defaults_false() {
        let cli = cli_from(&["md", "doc.md"]);
        assert!(!cli.strict_style);
    }

    #[test]
    fn strict_style_flag_parses() {
        let cli = cli_from(&["md", "doc.md", "--strict-style"]);
        assert!(cli.strict_style);
    }

    #[test]
    fn overrides_default_empty_when_no_layout_flags() {
        let cli = cli_from(&["md", "doc.md"]);
        let o = page_style_overrides_from_cli(&cli);
        assert_eq!(o, PageStyleOverrides::default());
    }

    #[test]
    fn margin_shorthand_claims_all_four_sides() {
        let cli = cli_from(&["md", "doc.md", "--margin", "4"]);
        let o = page_style_overrides_from_cli(&cli);
        assert!(o.margin_top && o.margin_right && o.margin_bottom && o.margin_left);
        assert!(!o.padding_top && !o.padding_left);
    }

    #[test]
    fn mx_claims_left_and_right_only() {
        let cli = cli_from(&["md", "doc.md", "--mx", "2"]);
        let o = page_style_overrides_from_cli(&cli);
        assert!(o.margin_left && o.margin_right);
        assert!(!o.margin_top && !o.margin_bottom);
    }

    #[test]
    fn my_claims_top_and_bottom_only() {
        let cli = cli_from(&["md", "doc.md", "--my", "1"]);
        let o = page_style_overrides_from_cli(&cli);
        assert!(o.margin_top && o.margin_bottom);
        assert!(!o.margin_left && !o.margin_right);
    }

    #[test]
    fn ml_claims_only_left_margin() {
        let cli = cli_from(&["md", "doc.md", "--ml", "2"]);
        let o = page_style_overrides_from_cli(&cli);
        assert!(o.margin_left);
        assert!(!o.margin_right && !o.margin_top && !o.margin_bottom);
    }

    #[test]
    fn padding_shorthand_claims_all_four_sides() {
        let cli = cli_from(&["md", "doc.md", "--padding", "2"]);
        let o = page_style_overrides_from_cli(&cli);
        assert!(o.padding_top && o.padding_right && o.padding_bottom && o.padding_left);
    }

    #[test]
    fn px_claims_left_and_right_padding() {
        let cli = cli_from(&["md", "doc.md", "--px", "1"]);
        let o = page_style_overrides_from_cli(&cli);
        assert!(o.padding_left && o.padding_right);
        assert!(!o.padding_top && !o.padding_bottom);
    }

    #[test]
    fn max_width_flag_claims_max_width() {
        let cli = cli_from(&["md", "doc.md", "--max-width", "80"]);
        let o = page_style_overrides_from_cli(&cli);
        assert!(o.max_width);
    }

    #[test]
    fn page_bg_flag_claims_background() {
        let cli = cli_from(&["md", "doc.md", "--page-bg", "subtle"]);
        let o = page_style_overrides_from_cli(&cli);
        assert!(o.background);
    }

    #[test]
    fn alignment_flag_claims_alignment() {
        let cli = cli_from(&["md", "doc.md", "--alignment", "center"]);
        let o = page_style_overrides_from_cli(&cli);
        assert!(o.alignment);
    }

    #[test]
    fn component_specific_alignment_flags_claim_their_component() {
        let cli = cli_from(&[
            "md",
            "doc.md",
            "--align-images",
            "left",
            "--align-lists",
            "right",
            "--align-block-quotes",
            "center",
            "--align-tables",
            "right",
            "--align-code-blocks",
            "left",
        ]);
        let o = page_style_overrides_from_cli(&cli);
        assert!(o.align_images && o.align_lists && o.align_block_quotes);
        assert!(o.align_tables && o.align_code_blocks);
        // The global `--alignment` was not set, so the broadcast field stays clear.
        assert!(!o.alignment);
    }

    #[test]
    fn granular_list_alignment_flags_claim_their_component() {
        let cli = cli_from(&[
            "md",
            "doc.md",
            "--align-ul",
            "left",
            "--align-ol",
            "center",
            "--align-li",
            "right",
        ]);
        let o = page_style_overrides_from_cli(&cli);
        assert!(o.align_ul && o.align_ol && o.align_li);
        assert!(!o.align_lists);
    }

    #[test]
    fn component_alignment_unclaimed_when_no_flags() {
        let cli = cli_from(&["md", "doc.md"]);
        let o = page_style_overrides_from_cli(&cli);
        assert!(
            !o.align_images
                && !o.align_lists
                && !o.align_ul
                && !o.align_ol
                && !o.align_li
                && !o.align_block_quotes
                && !o.align_tables
                && !o.align_code_blocks,
        );
    }

    // -----------------------------------------------------------------
    // Bespoke style overrides
    // -----------------------------------------------------------------

    #[test]
    fn bespoke_overrides_default_empty_when_no_flags() {
        let cli = cli_from(&["md", "doc.md"]);
        let o = bespoke_style_overrides_from_cli(&cli);
        assert!(!o.code_theme);
    }

    #[test]
    fn code_theme_flag_claims_code_theme() {
        let cli = cli_from(&["md", "doc.md", "--code-theme", "dracula"]);
        let o = bespoke_style_overrides_from_cli(&cli);
        assert!(o.code_theme);
    }

    // -----------------------------------------------------------------
    // Strict-style preserves informational `KnownButInactive` warnings
    // -----------------------------------------------------------------

    /// Build a `DarkmatterPage` at a deterministic width for warning-log tests.
    fn test_page() -> DarkmatterPage {
        DarkmatterPage::new(&Terminal::new_optimistic(80))
    }

    #[test]
    #[tracing_test::traced_test]
    fn strict_style_succeeds_on_schema_clean_sub_spec_7_key() {
        // After sub-spec #7, `page.stylesheet` is wired and produces no
        // `KnownButInactive` warning. `--strict-style` must still succeed
        // because the document is schema-clean.
        let raw = "---\n\
style:\n\
\x20   page:\n\
\x20       stylesheet: https://example.com/main.css\n\
---\n\n# Doc\n";
        let md = Markdown::try_from_content(raw).unwrap();
        let cli = cli_from(&["md", "doc.md", "--strict-style"]);
        apply_style_frontmatter(test_page(), &md, &cli, None)
            .expect("strict-style must succeed on schema-clean wired key");
    }

    #[test]
    #[tracing_test::traced_test]
    fn non_strict_style_applies_sub_spec_7_key_silently() {
        // After sub-spec #7, `page.stylesheet` is wired and silent.
        let raw = "---\n\
style:\n\
\x20   page:\n\
\x20       stylesheet: https://example.com/main.css\n\
---\n\n# Doc\n";
        let md = Markdown::try_from_content(raw).unwrap();
        let cli = cli_from(&["md", "doc.md"]);
        let page = apply_style_frontmatter(test_page(), &md, &cli, None).expect("apply");
        assert!(page.stylesheet().is_some(), "sub-spec #7 key should be applied");
    }
}
