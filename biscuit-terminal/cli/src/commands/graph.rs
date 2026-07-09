use crate::args::LayoutArgs;
use crate::commands::shared::*;
use crate::commands::{CliContext, Run};
use crate::output::RenderMeta;
use biscuit_terminal::components::graph_expression::GraphExpression;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::terminal::Terminal;
use clap::Args as ClapArgs;
use std::io::Write;
use std::time::Instant;

/// Graph input syntax argument for CLI.
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum GraphInputSyntaxArg {
    Auto,
    Expression,
    Dot,
}

impl GraphInputSyntaxArg {
    pub fn as_cli_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Expression => "expression",
            Self::Dot => "dot",
        }
    }
}

impl From<GraphInputSyntaxArg>
    for biscuit_terminal::components::graph_expression::GraphInputSyntax
{
    fn from(arg: GraphInputSyntaxArg) -> Self {
        match arg {
            GraphInputSyntaxArg::Auto => Self::Auto,
            GraphInputSyntaxArg::Expression => Self::Expression,
            GraphInputSyntaxArg::Dot => Self::Dot,
        }
    }
}

/// Graph orientation argument for CLI.
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum GraphOrientationArg {
    LeftToRight,
    TopToBottom,
}

impl GraphOrientationArg {
    pub fn as_cli_str(&self) -> &'static str {
        match self {
            Self::LeftToRight => "left-to-right",
            Self::TopToBottom => "top-to-bottom",
        }
    }
}

impl From<GraphOrientationArg>
    for biscuit_terminal::components::graph_expression::GraphOrientation
{
    fn from(arg: GraphOrientationArg) -> Self {
        match arg {
            GraphOrientationArg::LeftToRight => Self::LeftToRight,
            GraphOrientationArg::TopToBottom => Self::TopToBottom,
        }
    }
}

/// Example data for graph-expression --example
const GRAPH_EXPRESSION_EXAMPLE: &str = "Start -> Validate -> Render; Validate -> Retry";
const GRAPH_EXPRESSION_EXAMPLE_CMD: &str =
    r#"bt graph-expression "Start -> Validate -> Render; Validate -> Retry""#;

/// Render a graph expression as a diagram
#[derive(ClapArgs, Debug, Clone)]
pub struct GraphExpressionArgs {
    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(long, default_value = "auto", value_enum)]
    pub syntax: GraphInputSyntaxArg,

    #[arg(long, short = 't')]
    pub title: Option<String>,

    #[arg(long, short = 'w')]
    pub width: Option<crate::types::WidthSpec>,

    #[arg(long)]
    pub inverse: bool,

    #[arg(long, add = clap_complete::engine::ArgValueCompleter::new(crate::font_completer))]
    pub font: Option<String>,

    #[arg(long, value_enum, default_value = "top-to-bottom")]
    pub orientation: GraphOrientationArg,

    #[command(flatten)]
    pub layout: LayoutArgs,

    #[arg(long)]
    pub meta: bool,

    #[arg(value_name = "EXP", required_unless_present = "example")]
    pub content: Vec<String>,
}

impl Run for GraphExpressionArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let _ = std::io::stdout().flush();

        let source = if self.example {
            GRAPH_EXPRESSION_EXAMPLE.to_string()
        } else {
            self.content.join(" ")
        };

        if source.trim().is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No graph expression provided. Use format: \"a -> b -> c\""
            ));
        }

        if ctx.json {
            let output = serde_json::json!({
                "type": "graph-expression",
                "syntax": self.syntax.as_cli_str(),
                "orientation": self.orientation.as_cli_str(),
                "inverse": self.inverse,
                "title": self.title,
                "width": self.width,
                "font": self.font,
                "source": source,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            return Ok(());
        }

        let mut graph = if self.inverse {
            GraphExpression::inverted_for_terminal(&source, self.syntax.into())
                .map_err(|e| color_eyre::eyre::eyre!("{}", e))?
        } else {
            GraphExpression::for_terminal(&source, self.syntax.into())
                .map_err(|e| color_eyre::eyre::eyre!("{}", e))?
        };

        if let Some(f) = &self.font {
            graph = graph.with_font_family(f);
        }

        if let Some(t) = &self.title {
            graph = graph.with_title(t);
        }
        graph = graph.with_orientation(self.orientation.into());

        if let Some(w) = &self.width {
            use biscuit_terminal::components::terminal_image::parse_width_spec;
            let image_width =
                parse_width_spec(&w.to_string()).map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
            graph = graph.with_width(image_width);
        }

        apply_renderable_layout(&mut graph, &self.layout);

        let terminal = terminal_for_render(ctx.plain);
        display_graph(&graph, &source, &self.layout, self.meta, &terminal)?;

        if self.example {
            print_example_command_with_terminal(GRAPH_EXPRESSION_EXAMPLE_CMD, &terminal);
        }

        Ok(())
    }
}

/// Display a [`GraphExpression`] and optionally output metadata.
pub fn display_graph(
    graph: &GraphExpression,
    source: &str,
    layout: &LayoutArgs,
    meta: bool,
    terminal: &Terminal,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::graph_expression::GraphRenderError;
    let start_time = Instant::now();

    let result = match graph.try_render(terminal) {
        Ok(result) => result,
        Err(GraphRenderError::NoImageSupport) => {
            emit_vertical_margins(layout, || {
                print!("{}", graph.render(terminal));
                Ok(())
            })?;
            return Ok(());
        }
        Err(error) => return handle_graph_error(error, &graph.fallback_code_block(), source, terminal),
    };

    let render_time_ms = start_time.elapsed().as_millis() as u64;

    emit_vertical_margins(layout, || emit_image_output(&result.output))?;

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

    settle_terminal();

    Ok(())
}

/// Handle graph rendering errors with user-friendly output.
pub fn handle_graph_error(
    error: biscuit_terminal::components::graph_expression::GraphRenderError,
    fallback: &str,
    source: &str,
    term: &Terminal,
) -> color_eyre::Result<()> {
    use biscuit_terminal::components::graph_expression::GraphRenderError;

    match error {
        GraphRenderError::NoImageSupport => {
            println!("{fallback}");
            Ok(())
        }
        GraphRenderError::Visualization(ref viz_err) => {
            let mut section = Section::new(HeadingLevel::h3, "Error");
            section.push(Prose::new(format!(
                "<red><b>Error:</b></red> {}",
                Prose::escape_text(&viz_err.to_string())
            )));
            section.push(Prose::new("<dim>Graph expression was defined as:</dim>"));
            section.push(Prose::new(Prose::escape_text(fallback)));
            eprintln!("{}", section.render(term));
            Err(color_eyre::eyre::eyre!("{}", viz_err))
        }
        GraphRenderError::DisplayError(ref msg) => {
            let mut section = Section::new(HeadingLevel::h3, "Error");
            section.push(Prose::new(format!(
                "<red><b>Error:</b></red> Failed to display image: {}",
                Prose::escape_text(msg)
            )));
            section.push(Prose::new("<dim>Graph expression source was:</dim>"));
            section.push(Prose::new(Prose::escape_text(source)));
            eprintln!("{}", section.render(term));
            Err(color_eyre::eyre::eyre!("Failed to display image: {}", msg))
        }
    }
}
