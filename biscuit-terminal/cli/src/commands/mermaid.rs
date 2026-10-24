use crate::args::{LayoutArgs, MermaidFeatureArgs};
use crate::commands::shared::*;
use crate::output::RenderMeta;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::components::terminal_image::TerminalImage;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::LayoutTerminalExt;
use biscuit_visualized::components::mermaid::{MermaidDiagram, MermaidPresentation, MermaidRenderError};
use renderable::tree::{RenderNode, RenderStrictness};
use std::time::Instant;

/// Build a [`MermaidDiagram`] from instructions, applying common CLI options.
///
/// Handles inverse mode, width parsing, layout application, and the
/// fallback policy selected by `--feature`.
pub fn build_mermaid_diagram(
    instructions: &str,
    inverse: bool,
    width: Option<&str>,
    layout: &LayoutArgs,
    features: &MermaidFeatureArgs,
) -> color_eyre::Result<MermaidDiagram> {
    let mut diagram = if inverse {
        MermaidDiagram::new(instructions).inverted(is_dark_mode())
    } else {
        MermaidDiagram::new(instructions)
    };
    diagram = diagram.with_fallback_policy(features.fallback_policy());

    // Apply width
    if let Some(w) = width {
        use biscuit_terminal::components::terminal_image::parse_width_spec;
        let image_width = parse_width_spec(w).map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
        diagram = diagram.with_width(image_width);
    }

    // Apply layout
    apply_renderable_layout(&mut diagram, layout);

    Ok(diagram)
}

pub fn with_mermaid_frontmatter_title(body: &str, title: Option<&str>) -> String {
    match title {
        Some(title) => format!("---\ntitle: {title}\n---\n{body}"),
        None => body.to_string(),
    }
}

/// Display a [`MermaidDiagram`] and optionally output metadata.
///
/// When `terminal` can show images, uses the fallible
/// [`MermaidDiagram::try_render()`], so a failed image export is an error.
/// Otherwise prints the text drawing or the source that
/// [`MermaidDiagram::present`] chooses, highlighted (see
/// [`render_mermaid_fallback`]); `meta` and `debug` describe images only.
///
/// When `debug` is true, emits a `--- mermaid debug ---` block to
/// stderr containing the resolved `image width: <N> cells` line.
/// Level-2 tests parse this to verify pane-column geometry.
pub fn display_mermaid(
    diagram: &MermaidDiagram,
    instructions: &str,
    diagram_type: &str,
    layout: &LayoutArgs,
    meta: bool,
    debug: bool,
    terminal: &Terminal,
) -> color_eyre::Result<()> {
    let start_time = Instant::now();

    let rendered = if TerminalImage::route_has_pixels(terminal) {
        diagram.try_render(terminal)
    } else {
        Err(MermaidRenderError::NoImageSupport)
    };
    let result = match rendered {
        Ok(result) => result,
        Err(e) => {
            return handle_mermaid_error(e, diagram, instructions, diagram_type, layout, terminal);
        }
    };

    let render_time_ms = start_time.elapsed().as_millis() as u64;

    emit_vertical_margins(layout, || emit_image_output(&result.output))?;

    // Output metadata if requested (goes to stderr, outside layout margins)
    if meta {
        let file_size_bytes = std::fs::metadata(&result.png_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let render_meta = RenderMeta {
            filename: result.png_path.to_string_lossy().to_string(),
            cache_hit: result.cache_hit,
            file_size_bytes,
            render_time_ms,
        };

        output_render_meta(&render_meta)?;
    }

    if debug {
        eprintln!("--- mermaid debug ---");
        eprintln!("image width: {} cells", result.width_cells);
        eprintln!("term width: {} cells", terminal.width());
        eprintln!("render time: {render_time_ms} ms");
    }

    // Let terminal settle after image rendering
    settle_terminal();

    Ok(())
}

/// The diagram source as a fenced `mermaid` block.
///
/// A fenced block's body is literal, so `instructions` is not escaped:
/// `Prose::escape_text` here would show its backslashes.
fn mermaid_source_block(instructions: &str) -> Prose {
    Prose::new(format!("```mermaid\n{instructions}\n```"))
}

/// `diagram` as [`MermaidDiagram::present`] shows it on `term`, with code
/// blocks highlighted by darkmatter: the text drawing (a `txt` block, plus
/// the partial note) or the `mermaid` source.
///
/// This is the component's own layout (its text columns, then its margins
/// and alignment) with a code renderer installed; without color the code
/// renderer declines and the blocks are plain fences.
pub fn render_mermaid_fallback(diagram: &MermaidDiagram, term: &Terminal) -> String {
    let nodes = match diagram.present(term) {
        MermaidPresentation::Image(result) => return result.output,
        presentation => presentation.nodes().unwrap_or_default().to_vec(),
    };
    let mut options =
        TerminalRenderOptions::new(term, RenderStrictness::Lossy).with_code_renderer(darkmatter_code_renderer(term));
    options.context = options.context.with_width(diagram.text_columns(term.width()));
    let output = render_terminal_node(&RenderNode::root(nodes), &options)
        .map(|rendered| rendered.output)
        .unwrap_or_else(|_| diagram.fallback_code_block());
    diagram.layout().apply_block_layout(&output, term.width())
}

/// Handle Mermaid rendering errors with user-friendly output.
///
/// No image support is not a failure: the fallback is printed (see
/// [`render_mermaid_fallback`]) with the route hint. A failed image export
/// is reported and returned as an error.
pub fn handle_mermaid_error(
    error: MermaidRenderError,
    diagram: &MermaidDiagram,
    instructions: &str,
    diagram_type: &str,
    layout: &LayoutArgs,
    term: &Terminal,
) -> color_eyre::Result<()> {
    match error {
        MermaidRenderError::NoImageSupport => {
            let output = render_mermaid_fallback(diagram, term);
            emit_vertical_margins(layout, || {
                println!("{output}");
                Ok(())
            })?;
            crate::hints::emit_terminal_route_hint(term);
            Ok(())
        }
        MermaidRenderError::Visualization(ref viz_err) => {
            let mut section = Section::new(HeadingLevel::h3, "Error");
            section.push(Prose::new(format!(
                "<red><b>Error:</b></red> {}",
                Prose::escape_text_outside_code_spans(&viz_err.to_string())
            )));
            section.push(Prose::new(format!(
                "<dim>Mermaid {} was defined as:</dim>",
                diagram_type
            )));
            section.push(mermaid_source_block(instructions));
            eprintln!("{}", section.render(term));
            Err(color_eyre::eyre::eyre!("{}", viz_err))
        }
        MermaidRenderError::DisplayError(ref msg) => {
            let mut section = Section::new(HeadingLevel::h3, "Error");
            section.push(Prose::new(format!(
                "<red><b>Error:</b></red> Failed to display image: {}",
                Prose::escape_text_outside_code_spans(msg)
            )));
            eprintln!("{}", section.render(term));
            Err(color_eyre::eyre::eyre!("Failed to display image: {}", msg))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::discovery::detection::ColorDepth;

    #[test]
    fn mermaid_source_block_shows_the_diagram_source_verbatim() {
        let instructions = "flowchart LR\n    A[Start] --> B{Ok?}\n    B -->|yes| C(Done_1)";

        let term = Terminal::builder()
            .width(120)
            .color_depth(ColorDepth::None)
            .build();

        let rendered = mermaid_source_block(instructions).render(&term);

        assert!(rendered.contains(instructions), "source not verbatim: {rendered:?}");
        assert!(!rendered.contains('\\'), "stray escape backslash: {rendered:?}");
    }

    use biscuit_terminal::discovery::detection::ImageSupport;
    use biscuit_visualized::components::{MermaidFallbackPolicy, PARTIAL_NOTE_ID, TextFallback};

    const PIE: &str = "pie\n    \"Dogs\" : 386\n    \"Cats\" : 85";
    const FLOWCHART: &str = "flowchart LR\n    A --> B";
    /// A 24-bit foreground SGR: syntect's token colors. The plain code block
    /// only ever dims, so this marks darkmatter's highlighter.
    const TRUECOLOR_FOREGROUND: &str = "\x1b[38;2;";

    /// A terminal with images off and the given color depth.
    fn no_image_terminal(color_depth: ColorDepth) -> Terminal {
        Terminal::builder()
            .width(100)
            .color_depth(color_depth)
            .image_support(ImageSupport::None)
            .supports_unicode(true)
            .is_tty(false)
            .build()
    }

    /// `source` as `bt` builds it, with text allowed explicitly so the
    /// host's `MERMAID_FALLBACK` cannot change the route.
    fn diagram(source: &str) -> MermaidDiagram {
        build_mermaid_diagram(source, false, None, &LayoutArgs::default(), &MermaidFeatureArgs::default())
            .expect("diagram")
            .with_fallback_policy(MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Allowed))
    }

    #[test]
    fn a_source_fallback_is_highlighted_by_darkmatter() {
        let term = no_image_terminal(ColorDepth::TrueColor);
        assert!(!TerminalImage::route_has_pixels(&term));
        let pie = diagram(PIE);
        assert!(matches!(pie.present(&term), MermaidPresentation::Source(_)));

        let output = render_mermaid_fallback(&pie, &term);

        assert!(output.contains(TRUECOLOR_FOREGROUND), "no highlighting: {output:?}");
        assert!(output.contains("Dogs"), "source missing: {output:?}");
        assert!(!output.contains("```"), "a plain fence, not darkmatter's block: {output:?}");
    }

    #[test]
    fn a_text_fallback_is_highlighted_by_darkmatter() {
        let term = no_image_terminal(ColorDepth::TrueColor);
        let flowchart = diagram(FLOWCHART);
        assert!(matches!(flowchart.present(&term), MermaidPresentation::Text(_)));

        let output = render_mermaid_fallback(&flowchart, &term);

        assert!(output.contains(TRUECOLOR_FOREGROUND), "no highlighting: {output:?}");
        assert!(output.contains('┌') && output.contains(" A "), "drawing missing: {output:?}");
        assert!(output.contains(PARTIAL_NOTE_ID), "note missing: {output:?}");
        assert!(!output.contains("```"), "a plain fence, not darkmatter's block: {output:?}");
    }

    /// The source block's body is byte-for-byte darkmatter's `CodeBlock`
    /// for a `mermaid` fence: the same highlighter and Mermaid syntax.
    /// `CodeBlock` also prints a language header row, which the
    /// component's code node does not ask for.
    #[test]
    fn a_source_fallback_matches_darkmatters_code_block() {
        let term = no_image_terminal(ColorDepth::TrueColor);

        let output = render_mermaid_fallback(&diagram(PIE), &term);
        let code_block = darkmatter::markdown::CodeBlock::new(PIE)
            .with_fence_language("mermaid")
            .render(&term);

        assert!(
            code_block.trim_end().ends_with(output.trim_end()),
            "bt: {output:?}\nCodeBlock: {code_block:?}"
        );
    }

    #[test]
    fn without_color_fallbacks_are_plain_fences() {
        let term = no_image_terminal(ColorDepth::None);

        let source = render_mermaid_fallback(&diagram(PIE), &term);
        let text = render_mermaid_fallback(&diagram(FLOWCHART), &term);

        assert!(source.starts_with("```mermaid\n"), "{source:?}");
        assert!(text.starts_with("```txt\n"), "{text:?}");
        assert!(!source.contains('\x1b') && !text.contains('\x1b'), "escapes without color");
    }

    /// A failed image export stays an error; only missing image support
    /// prints a fallback. (Driving a real export failure loads the host's
    /// fonts, which takes seconds, so the handler is tested directly.)
    #[test]
    fn an_image_export_failure_is_still_an_error() {
        let term = no_image_terminal(ColorDepth::None);
        let pie = diagram(PIE);

        let result = handle_mermaid_error(
            MermaidRenderError::DisplayError("encoder refused".to_string()),
            &pie,
            PIE,
            "pie chart",
            &LayoutArgs::default(),
            &term,
        );

        assert!(result.is_err(), "an export failure became success");
    }
}
