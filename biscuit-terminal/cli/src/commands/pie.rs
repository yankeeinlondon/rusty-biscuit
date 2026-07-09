use crate::args::LayoutArgs;
use crate::commands::color_parse::extract_color;
use crate::commands::mermaid::{build_mermaid_diagram, display_mermaid};
use crate::commands::shared::{print_example_command_with_terminal, terminal_for_render};
use crate::commands::{CliContext, Run};
use clap::Args as ClapArgs;
use std::io::Write;

/// A parsed pie chart entry with optional color.
pub struct PieEntry {
    /// The Mermaid-formatted data line (e.g., `"Label" : value`)
    pub line: String,
    /// Optional hex color for this slice (e.g., `#3178c6`)
    pub color: Option<String>,
}

/// Parses pie chart data from various input formats.
pub fn parse_pie_data(data: &[String]) -> Vec<PieEntry> {
    let mut result = Vec::new();

    for item in data {
        if item.contains(';') {
            for part in item.split(';') {
                let part = part.trim();
                if !part.is_empty()
                    && let Some(parsed) = parse_single_pie_entry(part)
                {
                    result.push(parsed);
                }
            }
        } else {
            if let Some(parsed) = parse_single_pie_entry(item) {
                result.push(parsed);
            }
        }
    }

    result
}

/// Parses a single pie chart entry into Mermaid format with optional color.
pub fn parse_single_pie_entry(entry: &str) -> Option<PieEntry> {
    let entry = entry.trim();
    if entry.is_empty() {
        return None;
    }

    let (entry_without_color, color) = extract_color(entry);

    if let Some(stripped) = entry_without_color.strip_prefix('"')
        && let Some(close_quote_idx) = stripped.find('"')
    {
        let label = &stripped[..close_quote_idx];
        let rest = &stripped[close_quote_idx + 1..];

        if let Some(colon_idx) = rest.find(':') {
            let value = rest[colon_idx + 1..].trim();
            if !value.is_empty() {
                return Some(PieEntry {
                    line: format!("\"{}\" : {}", label, value),
                    color,
                });
            }
        }
    }

    if let Some(colon_idx) = entry_without_color.find(':') {
        let label = entry_without_color[..colon_idx].trim();
        let value = entry_without_color[colon_idx + 1..].trim();

        if !label.is_empty() && !value.is_empty() {
            let label = label.trim_matches('"');
            return Some(PieEntry {
                line: format!("\"{}\" : {}", label, value),
                color,
            });
        }
    }

    None
}

/// Builds the Mermaid init directive for pie chart colors.
pub fn build_pie_init_directive(entries: &[PieEntry]) -> Option<String> {
    let color_vars: Vec<String> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, entry)| {
            entry
                .color
                .as_ref()
                .map(|c| format!("'pie{}': '{}'", i + 1, c))
        })
        .collect();

    if color_vars.is_empty() {
        None
    } else {
        Some(format!(
            "%%{{init: {{'themeVariables': {{{}}}}}}}%%",
            color_vars.join(", ")
        ))
    }
}

/// Example data for pie-chart --example
const PIE_CHART_EXAMPLE: &[&str] = &["TypeScript: 45 #3178C6", "Rust: 35 #A72145", "Python: 20"];
const PIE_CHART_EXAMPLE_CMD: &str =
    r#"bt pie-chart "TypeScript: 45 #3178C6" "Rust: 35 #A72145" "Python: 20""#;

/// Render a pie chart from data values
#[derive(ClapArgs, Debug, Clone)]
pub struct PieChartArgs {
    #[arg(long)]
    pub inverse: bool,

    #[arg(long, short = 't')]
    pub title: Option<String>,

    #[arg(long, short = 'w')]
    pub width: Option<crate::types::WidthSpec>,

    #[command(flatten)]
    pub layout: LayoutArgs,

    #[arg(long)]
    pub show_data: bool,

    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(long)]
    pub meta: bool,

    #[arg(long)]
    pub debug: bool,

    #[arg(value_name = "DATA", required_unless_present = "example")]
    pub data: Vec<String>,
}

impl Run for PieChartArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let _ = std::io::stdout().flush();

        let data: Vec<String> = if self.example {
            PIE_CHART_EXAMPLE.iter().map(|s| s.to_string()).collect()
        } else {
            self.data
        };

        let parsed_entries = parse_pie_data(&data);

        if parsed_entries.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No valid data points provided. Use format: \"Label: value\""
            ));
        }

        let init_directive = build_pie_init_directive(&parsed_entries);

        let show_data_str = if self.show_data { " showData" } else { "" };
        let title_line = self
            .title
            .as_ref()
            .map(|t| format!("    title {}", t))
            .unwrap_or_default();

        let data_lines: String = parsed_entries
            .iter()
            .map(|e| format!("    {}", e.line))
            .collect::<Vec<_>>()
            .join("\n");

        let mut instructions_parts = Vec::new();

        if let Some(ref init) = init_directive {
            instructions_parts.push(init.clone());
        }

        if title_line.is_empty() {
            instructions_parts.push(format!("pie{}\n{}", show_data_str, data_lines));
        } else {
            instructions_parts.push(format!(
                "pie{}\n{}\n{}",
                show_data_str, title_line, data_lines
            ));
        }

        let instructions = instructions_parts.join("\n");

        if ctx.json {
            let output = serde_json::json!({
                "type": "pie-chart",
                "inverse": self.inverse,
                "title": self.title,
                "width": self.width,
                "show_data": self.show_data,
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
            "pie chart",
            &self.layout,
            self.meta,
            self.debug,
            &terminal,
        )?;

        if self.example {
            print_example_command_with_terminal(PIE_CHART_EXAMPLE_CMD, &terminal);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::color_parse::parse_hex_color;

    #[test]
    fn test_extract_color() {
        assert_eq!(
            extract_color("Label: 10 #ff0000"),
            ("Label: 10", Some("#ff0000".to_string()))
        );
        assert_eq!(
            extract_color("Label: 10 color: #00ff00"),
            ("Label: 10", Some("#00ff00".to_string()))
        );
        assert_eq!(extract_color("Label: 10"), ("Label: 10", None));
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#fff"), Some("#fff".to_string()));
        assert_eq!(parse_hex_color("#ff0000"), Some("#ff0000".to_string()));
        assert_eq!(parse_hex_color("ff0000"), None);
        assert_eq!(parse_hex_color("invalid"), None);
    }

    #[test]
    fn test_parse_single_pie_entry() {
        let entry = parse_single_pie_entry("Dogs: 386").unwrap();
        assert_eq!(entry.line, "\"Dogs\" : 386");
        assert_eq!(entry.color, None);

        let entry = parse_single_pie_entry("Cats: 85 #00ff00").unwrap();
        assert_eq!(entry.line, "\"Cats\" : 85");
        assert_eq!(entry.color, Some("#00ff00".to_string()));

        let entry = parse_single_pie_entry("\"Some Label\" : 10.5").unwrap();
        assert_eq!(entry.line, "\"Some Label\" : 10.5");
        assert_eq!(entry.color, None);
    }
}
