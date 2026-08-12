use crate::args::LayoutArgs;
use crate::commands::mermaid::{build_mermaid_diagram, display_mermaid};
use crate::commands::shared::{print_example_command_with_terminal, terminal_for_render};
use crate::commands::{CliContext, Run};
use clap::Args as ClapArgs;
use std::io::Write;

/// Example data for state-diagram --example
const STATE_DIAGRAM_EXAMPLE: &[&str] = &[
    "[*] --> Idle",
    "Idle --> Running: start",
    "Running --> Idle: stop",
    "Running --> Error: failure",
    "Error --> Idle: reset",
    "Idle --> [*]: shutdown",
];
const STATE_DIAGRAM_EXAMPLE_CMD: &str = "bt state-diagram --title \"Process States\" \"[*] --> Idle\" \"Idle --> Running: start\" \"Running --> Idle: stop\" \"Running --> Error: failure\" \"Error --> Idle: reset\" \"Idle --> [*]: shutdown\"";

/// Render a state diagram
#[derive(ClapArgs, Debug, Clone)]
#[command(after_long_help = "\nExamples:\n  [*]           Start/end state\n")]
pub struct StateDiagramArgs {
    #[arg(long, short = 't')]
    pub title: Option<String>,

    #[arg(long, short = 'w')]
    pub width: Option<crate::types::WidthSpec>,

    #[command(flatten)]
    pub layout: LayoutArgs,

    #[arg(long)]
    pub inverse: bool,

    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(long)]
    pub meta: bool,

    #[arg(value_name = "TRANSITIONS", required_unless_present = "example")]
    pub transitions: Vec<String>,
}

impl Run for StateDiagramArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let _ = std::io::stdout().flush();

        let (transitions, eff_title): (Vec<String>, Option<&str>) = if self.example {
            (
                STATE_DIAGRAM_EXAMPLE
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                Some("Process States"),
            )
        } else {
            (self.transitions, self.title.as_deref())
        };

        if transitions.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No transitions provided. Use format: \"State1 --> State2\" or \"[*] --> State\""
            ));
        }

        let mut lines = vec!["stateDiagram-v2".to_string()];

        for transition in &transitions {
            lines.push(format!("    {}", transition));
        }

        use crate::commands::mermaid::with_mermaid_frontmatter_title;
        let instructions = with_mermaid_frontmatter_title(&lines.join("\n"), eff_title);

        if ctx.json {
            let output = serde_json::json!({
                "type": "state-diagram",
                "inverse": self.inverse,
                "title": eff_title,
                "transitions": transitions,
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
            "state diagram",
            &self.layout,
            self.meta,
            false,
            &terminal,
        )?;

        if self.example {
            print_example_command_with_terminal(STATE_DIAGRAM_EXAMPLE_CMD, &terminal);
        }

        Ok(())
    }
}
