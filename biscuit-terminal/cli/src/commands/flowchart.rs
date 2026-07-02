use crate::args::LayoutArgs;
use crate::commands::mermaid::{build_mermaid_diagram, display_mermaid};
use crate::commands::{CliContext, Run};
use clap::Args as ClapArgs;
use std::io::Write;

/// Example data for flowchart --example
/// Note: Each element is joined with newlines in the flowchart body
const FLOWCHART_EXAMPLE: &[&str] = &[
    "A[Start] --> B{Decision}",
    "B -->|Yes| C[Success]",
    "B -->|No| D[Retry]",
    "D --> B",
];
const FLOWCHART_EXAMPLE_CMD: &str = r#"bt flowchart "A[Start] --> B{Decision}" "B -->|Yes| C[Success]" "B -->|No| D[Retry]" "D --> B""#;

/// Render a flowchart from node definitions
///
/// Creates a Mermaid flowchart and renders it to the terminal.
/// Default direction is left-to-right (LR).
#[derive(ClapArgs, Debug, Clone)]
pub struct FlowchartArgs {
    /// Render top-down instead of left-right
    #[arg(long)]
    pub vertical: bool,

    /// Use inverted colors with solid background
    #[arg(long)]
    pub inverse: bool,

    /// Add a title above the diagram
    #[arg(long, short = 't')]
    pub title: Option<String>,

    /// Display width: percentage (e.g., "50%"), characters (e.g., "80ch" or "80"), or "fill"
    #[arg(long, short = 'w')]
    pub width: Option<crate::types::WidthSpec>,

    #[command(flatten)]
    pub layout: LayoutArgs,

    /// Render an example diagram and show the command used
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Output rendering metadata to stderr
    #[arg(long)]
    pub meta: bool,

    /// Flowchart node and edge definitions
    #[arg(value_name = "CONTENT", required_unless_present = "example")]
    pub content: Vec<String>,
}

impl Run for FlowchartArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let _ = std::io::stdout().flush();

        let content: Vec<String> = if self.example {
            FLOWCHART_EXAMPLE.iter().map(|s| s.to_string()).collect()
        } else {
            self.content
        };

        let direction = if self.vertical { "TD" } else { "LR" };
        let body = content.join("\n    ");

        let instructions = if let Some(title) = &self.title {
            format!(
                "---\ntitle: {}\n---\nflowchart {}\n    {}",
                title, direction, body
            )
        } else {
            format!("flowchart {}\n    {}", direction, body)
        };

        if ctx.json {
            let output = serde_json::json!({
                "type": "flowchart",
                "direction": direction,
                "inverse": self.inverse,
                "title": self.title,
                "width": self.width,
                "instructions": instructions,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            return Ok(());
        }

        let width_str = self.width.as_ref().map(|w| w.to_string());
        let diagram = build_mermaid_diagram(
            &instructions,
            self.inverse,
            width_str.as_deref(),
            &self.layout,
        )?;
        let terminal = terminal_for_render(ctx.plain);
        display_mermaid(
            &diagram,
            &instructions,
            "flowchart",
            &self.layout,
            self.meta,
            false,
            &terminal,
        )?;

        if self.example {
            print_example_command_with_terminal(FLOWCHART_EXAMPLE_CMD, &terminal);
        }

        Ok(())
    }
}

use crate::commands::shared::{print_example_command_with_terminal, terminal_for_render};
