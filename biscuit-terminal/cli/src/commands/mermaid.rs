use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::output::RenderMeta;
use biscuit_terminal::components::mermaid::MermaidDiagram;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::terminal::Terminal;
use std::time::Instant;

/// Build a [`MermaidDiagram`] from instructions, applying common CLI options.
///
/// Handles inverse mode, width parsing, and layout application.
pub fn build_mermaid_diagram(
    instructions: &str,
    inverse: bool,
    width: Option<&str>,
    layout: &LayoutArgs,
) -> color_eyre::Result<MermaidDiagram> {
    let mut diagram = if inverse {
        MermaidDiagram::new(instructions).inverted(is_dark_mode())
    } else {
        MermaidDiagram::new(instructions)
    };

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
/// Uses [`MermaidDiagram::try_render()`] for proper error reporting.
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

    let result = match diagram.try_render(terminal) {
        Ok(result) => result,
        Err(e) => {
            return handle_mermaid_error(e, instructions, diagram_type, terminal);
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

/// Handle Mermaid rendering errors with user-friendly output.
pub fn handle_mermaid_error(
    error: biscuit_terminal::components::mermaid::MermaidRenderError,
    instructions: &str,
    diagram_type: &str,
    term: &Terminal,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::mermaid::MermaidRenderError;

    match error {
        MermaidRenderError::NoImageSupport => {
            // Graceful degradation: output the diagram as a fenced code block
            // so piped/non-TTY consumers still get useful output.
            println!("```mermaid\n{}\n```", instructions);
            Ok(())
        }
        MermaidRenderError::Visualization(ref viz_err) => {
            let mut section = Section::new(HeadingLevel::h3, "Error");
            section.push(Prose::new(format!(
                "<red><b>Error:</b></red> {}",
                Prose::escape_text(&viz_err.to_string())
            )));
            section.push(Prose::new(format!(
                "<dim>Mermaid {} was defined as:</dim>",
                diagram_type
            )));
            section.push(Prose::new(format!(
                "```mermaid\n{}\n```",
                Prose::escape_text(instructions)
            )));
            eprintln!("{}", section.render(term));
            Err(color_eyre::eyre::eyre!("{}", viz_err))
        }
        MermaidRenderError::DisplayError(ref msg) => {
            let mut section = Section::new(HeadingLevel::h3, "Error");
            section.push(Prose::new(format!(
                "<red><b>Error:</b></red> Failed to display image: {}",
                Prose::escape_text(msg)
            )));
            eprintln!("{}", section.render(term));
            Err(color_eyre::eyre::eyre!("Failed to display image: {}", msg))
        }
    }
}
