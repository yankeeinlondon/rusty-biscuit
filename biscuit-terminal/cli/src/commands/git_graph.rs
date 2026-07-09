use crate::args::LayoutArgs;
use crate::commands::mermaid::{build_mermaid_diagram, display_mermaid};
use crate::commands::shared::{print_example_command_with_terminal, terminal_for_render};
use crate::commands::{CliContext, Run};
use clap::Args as ClapArgs;
use std::io::Write;

/// Example data for git-graph --example
const GIT_GRAPH_EXAMPLE: &[&str] = &[
    "commit",
    "commit",
    "branch feature",
    "checkout feature",
    "commit",
    "commit",
    "checkout main",
    "commit",
    "merge feature",
    "commit",
];
const GIT_GRAPH_EXAMPLE_CMD: &str = r#"bt git-graph "commit" "commit" "branch feature" "checkout feature" "commit" "commit" "checkout main" "commit" "merge feature" "commit""#;

/// Render a git graph from git commands
#[derive(ClapArgs, Debug, Clone)]
pub struct GitGraphArgs {
    #[arg(long)]
    pub inverse: bool,

    #[arg(long, short = 't')]
    pub title: Option<String>,

    #[arg(long, short = 'w')]
    pub width: Option<crate::types::WidthSpec>,

    #[command(flatten)]
    pub layout: LayoutArgs,

    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(long)]
    pub meta: bool,

    #[arg(value_name = "COMMANDS", required_unless_present = "example")]
    pub commands: Vec<String>,
}

impl Run for GitGraphArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let _ = std::io::stdout().flush();

        let commands: Vec<String> = if self.example {
            GIT_GRAPH_EXAMPLE.iter().map(|s| s.to_string()).collect()
        } else {
            self.commands
        };

        let body = commands
            .iter()
            .map(|cmd| format!("    {}", cmd))
            .collect::<Vec<_>>()
            .join("\n");

        let instructions = if let Some(title) = &self.title {
            format!("---\ntitle: {}\n---\ngitGraph\n{}", title, body)
        } else {
            format!("gitGraph\n{}", body)
        };

        if ctx.json {
            let output = serde_json::json!({
                "type": "git-graph",
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
            "git-graph",
            &self.layout,
            self.meta,
            false,
            &terminal,
        )?;

        if self.example {
            print_example_command_with_terminal(GIT_GRAPH_EXAMPLE_CMD, &terminal);
        }

        Ok(())
    }
}
