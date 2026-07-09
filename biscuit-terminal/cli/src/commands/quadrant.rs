use crate::args::LayoutArgs;
use crate::commands::mermaid::{build_mermaid_diagram, display_mermaid};
use crate::commands::shared::{
    format_axis_label, print_example_command_with_terminal, terminal_for_render,
};
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::mermaid::QuadrantTheme;
use clap::Args as ClapArgs;
use std::io::Write;

/// Example data for quadrant --example
const QUADRANT_EXAMPLE: &[&str] = &[
    "Campaign A: [0.3, 0.78]",
    "Campaign B: [0.45, 0.23]",
    "Campaign C: [0.57, 0.69]",
    "Campaign D: [0.78, 0.34]",
    "Campaign E: [0.40, 0.34]",
    "Campaign F: [0.65, 0.78]",
];
const QUADRANT_EXAMPLE_CMD: &str = r#"bt quadrant --title "Campaign Analysis" --x-axis "Low Reach --> High Reach" --y-axis "Low Engagement --> High Engagement" "Campaign A: [0.3, 0.78]" "Campaign B: [0.45, 0.23]" "Campaign C: [0.57, 0.69]" "Campaign D: [0.78, 0.34]" "Campaign E: [0.40, 0.34]" "Campaign F: [0.65, 0.78]""#;

/// Render a quadrant chart from data points
#[derive(ClapArgs, Debug, Clone)]
pub struct QuadrantArgs {
    #[arg(long = "x-axis", short = 'x', allow_hyphen_values = true)]
    pub x_axis: Option<String>,

    #[arg(long = "y-axis", short = 'y', allow_hyphen_values = true)]
    pub y_axis: Option<String>,

    #[arg(long, short = 't', allow_hyphen_values = true)]
    pub title: Option<String>,

    #[arg(
        long = "top-left",
        short = 'l',
        visible_alias = "tl",
        allow_hyphen_values = true
    )]
    pub top_left: Option<String>,

    #[arg(
        long = "top-right",
        short = 'r',
        visible_alias = "tr",
        allow_hyphen_values = true
    )]
    pub top_right: Option<String>,

    #[arg(long = "bottom-left", visible_alias = "bl", allow_hyphen_values = true)]
    pub bottom_left: Option<String>,

    #[arg(
        long = "bottom-right",
        visible_alias = "br",
        allow_hyphen_values = true
    )]
    pub bottom_right: Option<String>,

    #[arg(long)]
    pub inverse: bool,

    #[arg(long, short = 'w')]
    pub width: Option<crate::types::WidthSpec>,

    #[command(flatten)]
    pub layout: LayoutArgs,

    #[arg(long)]
    pub point_radius: Option<u32>,

    #[arg(long)]
    pub label_size: Option<u32>,

    #[arg(long, value_enum, default_value_t = QuadrantTheme::Default)]
    pub theme: QuadrantTheme,

    #[arg(long = "q1-fill")]
    pub q1_fill: Option<crate::types::HexColor>,

    #[arg(long = "q2-fill")]
    pub q2_fill: Option<crate::types::HexColor>,

    #[arg(long = "q3-fill")]
    pub q3_fill: Option<crate::types::HexColor>,

    #[arg(long = "q4-fill")]
    pub q4_fill: Option<crate::types::HexColor>,

    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(long)]
    pub meta: bool,

    #[arg(value_name = "POINTS", required_unless_present = "example")]
    pub points: Vec<String>,
}

impl Run for QuadrantArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let _ = std::io::stdout().flush();

        let (title, x_axis, y_axis, points): (
            Option<&str>,
            Option<&str>,
            Option<&str>,
            Vec<String>,
        ) = if self.example {
            (
                Some("Campaign Analysis"),
                Some("Low Reach --> High Reach"),
                Some("Low Engagement --> High Engagement"),
                QUADRANT_EXAMPLE.iter().map(|s| s.to_string()).collect(),
            )
        } else {
            (
                self.title.as_deref(),
                self.x_axis.as_deref(),
                self.y_axis.as_deref(),
                self.points,
            )
        };

        let mut body_lines = Vec::new();

        if let Some(t) = title {
            body_lines.push(format!("    title \"{}\"", t));
        }

        if let Some(x) = x_axis {
            body_lines.push(format!("    x-axis {}", format_axis_label(x)));
        }
        if let Some(y) = y_axis {
            body_lines.push(format!("    y-axis {}", format_axis_label(y)));
        }

        if let Some(tl) = &self.top_left {
            body_lines.push(format!("    quadrant-1 \"{}\"", tl));
        }
        if let Some(tr) = &self.top_right {
            body_lines.push(format!("    quadrant-2 \"{}\"", tr));
        }
        if let Some(bl) = &self.bottom_left {
            body_lines.push(format!("    quadrant-3 \"{}\"", bl));
        }
        if let Some(br) = &self.bottom_right {
            body_lines.push(format!("    quadrant-4 \"{}\"", br));
        }

        for point in &points {
            body_lines.push(format!("    {}", point));
        }

        let body = body_lines.join("\n");
        let instructions = format!("quadrantChart\n{}", body);

        if ctx.json {
            let output = serde_json::json!({
                "type": "quadrant",
                "x_axis": x_axis,
                "y_axis": y_axis,
                "title": title,
                "top_left": self.top_left,
                "top_right": self.top_right,
                "bottom_left": self.bottom_left,
                "bottom_right": self.bottom_right,
                "inverse": self.inverse,
                "width": self.width,
                "point_radius": self.point_radius,
                "label_size": self.label_size,
                "theme": self.theme.as_str(),
                "q1_fill": self.q1_fill,
                "q2_fill": self.q2_fill,
                "q3_fill": self.q3_fill,
                "q4_fill": self.q4_fill,
                "instructions": instructions,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            return Ok(());
        }

        let config = {
            use biscuit_terminal::components::mermaid::MermaidConfig;
            let mut cfg = MermaidConfig::new();

            if let Some(r) = self.point_radius {
                cfg = cfg.with_point_radius(r);
            }
            let effective_label_size =
                self.label_size
                    .unwrap_or(if points.len() <= 6 { 18 } else { 15 });
            cfg = cfg.with_point_label_font_size(effective_label_size);

            cfg = self
                .theme
                .apply(cfg, crate::commands::shared::is_dark_mode());

            if let Some(color) = &self.q1_fill {
                cfg = cfg.with_quadrant_fill(1, color.as_str());
            }
            if let Some(color) = &self.q2_fill {
                cfg = cfg.with_quadrant_fill(2, color.as_str());
            }
            if let Some(color) = &self.q3_fill {
                cfg = cfg.with_quadrant_fill(3, color.as_str());
            }
            if let Some(color) = &self.q4_fill {
                cfg = cfg.with_quadrant_fill(4, color.as_str());
            }

            cfg
        };

        let width_str = self.width.as_ref().map(|w| w.to_string());
        let diagram = build_mermaid_diagram(
            &instructions,
            self.inverse,
            width_str.as_deref(),
            &self.layout,
        )?
        .with_config(config);
        let terminal = terminal_for_render(ctx.plain);
        display_mermaid(
            &diagram,
            &instructions,
            "quadrant chart",
            &self.layout,
            self.meta,
            false,
            &terminal,
        )?;

        if self.example {
            print_example_command_with_terminal(QUADRANT_EXAMPLE_CMD, &terminal);
        }

        Ok(())
    }
}
