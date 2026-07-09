use crate::args::LayoutArgs;
use crate::commands::mermaid::{build_mermaid_diagram, display_mermaid};
use crate::commands::shared::{print_example_command_with_terminal, terminal_for_render};
use crate::commands::{CliContext, Run};
use clap::Args as ClapArgs;
use std::io::Write;

/// Example data for timeline --example
const TIMELINE_EXAMPLE: &[&str] = &[
    "2002: LinkedIn",
    "2004: Facebook",
    "2005: YouTube",
    "2006: Twitter",
    "2010: Instagram",
    "2011: Snapchat",
];
const TIMELINE_EXAMPLE_CMD: &str = "bt timeline --title \"Social Media History\" \"2002: LinkedIn\" \"2004: Facebook\" \"2005: YouTube\" \"2006: Twitter\" \"2010: Instagram\" \"2011: Snapchat\"";

/// Render a timeline diagram
#[derive(ClapArgs, Debug, Clone)]
pub struct TimelineArgs {
    #[arg(long, short = 't')]
    pub title: Option<String>,

    #[arg(long, short = 'w')]
    pub width: Option<crate::types::WidthSpec>,

    #[command(flatten)]
    pub layout: LayoutArgs,

    #[arg(long, short = 's', action = clap::ArgAction::Append)]
    pub section: Vec<String>,

    #[arg(long)]
    pub inverse: bool,

    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(long)]
    pub meta: bool,

    #[arg(value_name = "EVENTS", required_unless_present = "example")]
    pub events: Vec<String>,
}

impl Run for TimelineArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let _ = std::io::stdout().flush();

        let (events, eff_title): (Vec<String>, Option<&str>) = if self.example {
            (
                TIMELINE_EXAMPLE.iter().map(|s| s.to_string()).collect(),
                Some("Social Media History"),
            )
        } else {
            (self.events, self.title.as_deref())
        };

        if events.is_empty() && self.section.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No events provided. Use format: \"YYYY: Event description\""
            ));
        }

        for event in &events {
            if !event.contains(':') {
                return Err(color_eyre::eyre::eyre!(
                    "Invalid event format '{}'. Expected 'YYYY: Description'",
                    event
                ));
            }
        }

        let mut lines = vec!["timeline".to_string()];

        if let Some(t) = eff_title {
            lines.push(format!("    title {}", t));
        }

        if self.section.is_empty() {
            for event in &events {
                lines.push(format!("    {}", event));
            }
        } else {
            for (i, section) in self.section.iter().enumerate() {
                lines.push(format!("    section {}", section));
                let events_per_section = events.len().div_ceil(self.section.len());
                let start = i * events_per_section;
                let end = ((i + 1) * events_per_section).min(events.len());
                for event in events.get(start..end).unwrap_or(&[]) {
                    lines.push(format!("        {}", event));
                }
            }
        }

        use crate::commands::mermaid::with_mermaid_frontmatter_title;
        let instructions = with_mermaid_frontmatter_title(&lines.join("\n"), eff_title);

        if ctx.json {
            let output = serde_json::json!({
                "type": "timeline",
                "inverse": self.inverse,
                "title": eff_title,
                "sections": self.section,
                "events": events,
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
            "timeline",
            &self.layout,
            self.meta,
            false,
            &terminal,
        )?;

        if self.example {
            print_example_command_with_terminal(TIMELINE_EXAMPLE_CMD, &terminal);
        }

        Ok(())
    }
}
