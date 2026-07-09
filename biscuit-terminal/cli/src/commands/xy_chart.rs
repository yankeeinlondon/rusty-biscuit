use crate::args::LayoutArgs;
use crate::commands::mermaid::{build_mermaid_diagram, display_mermaid};
use crate::commands::shared::{print_example_command_with_terminal, terminal_for_render};
use crate::commands::{CliContext, Run};
use clap::Args as ClapArgs;
use std::io::Write;

/// XY chart type selector
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XyChartType {
    Bar,
    Line,
}

/// Example data for bar-chart --example
const BAR_CHART_EXAMPLE: &[&str] = &["12", "28", "45", "38", "22", "55"];
const BAR_CHART_EXAMPLE_CMD: &str = "bt bar-chart --title \"Monthly Revenue\" --x-axis \"Jan,Feb,Mar,Apr,May,Jun\" --y-axis \"$ (thousands)\" 12 28 45 38 22 55";

/// Example data for line-chart --example
const LINE_CHART_EXAMPLE: &[&str] = &["20", "22", "19", "23", "25", "21", "24"];
const LINE_CHART_EXAMPLE_CMD: &str = "bt line-chart --title \"Weekly Temperature\" --x-axis \"Mon,Tue,Wed,Thu,Fri,Sat,Sun\" --y-axis \"°C\" 20 22 19 23 25 21 24";

/// Render a bar chart from data values
#[derive(ClapArgs, Debug, Clone)]
pub struct BarChartArgs {
    #[arg(long, short = 't')]
    pub title: Option<String>,

    #[arg(long = "x-axis", short = 'x')]
    pub x_axis: Option<String>,

    #[arg(long = "y-axis", short = 'y')]
    pub y_axis: Option<String>,

    #[arg(long, short = 'w')]
    pub width: Option<crate::types::WidthSpec>,

    #[command(flatten)]
    pub layout: LayoutArgs,

    #[arg(long)]
    pub horizontal: bool,

    #[arg(long)]
    pub show_data_label: bool,

    #[arg(long)]
    pub aspect_ratio: Option<crate::types::PositiveF32>,

    #[arg(long)]
    pub line: bool,

    #[arg(long)]
    pub inverse: bool,

    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(long)]
    pub meta: bool,

    #[arg(value_name = "DATA", required_unless_present = "example")]
    pub data: Vec<String>,
}

impl Run for BarChartArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        render_xy_chart(
            XyChartType::Bar,
            self.title.as_deref(),
            self.x_axis.as_deref(),
            self.y_axis.as_deref(),
            self.width.as_ref().map(|w| w.to_string()).as_deref(),
            &self.layout,
            self.horizontal,
            self.show_data_label,
            self.aspect_ratio.map(|a| a.value()),
            self.line,
            false,
            self.inverse,
            self.example,
            self.meta,
            &self.data,
            ctx.json,
            ctx.plain,
        )
    }
}

/// Render a line chart from data values
#[derive(ClapArgs, Debug, Clone)]
pub struct LineChartArgs {
    #[arg(long, short = 't')]
    pub title: Option<String>,

    #[arg(long = "x-axis", short = 'x')]
    pub x_axis: Option<String>,

    #[arg(long = "y-axis", short = 'y')]
    pub y_axis: Option<String>,

    #[arg(long, short = 'w')]
    pub width: Option<crate::types::WidthSpec>,

    #[command(flatten)]
    pub layout: LayoutArgs,

    #[arg(long)]
    pub horizontal: bool,

    #[arg(long)]
    pub show_data_label: bool,

    #[arg(long)]
    pub aspect_ratio: Option<crate::types::PositiveF32>,

    #[arg(long)]
    pub bar: bool,

    #[arg(long)]
    pub inverse: bool,

    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(long)]
    pub meta: bool,

    #[arg(value_name = "DATA", required_unless_present = "example")]
    pub data: Vec<String>,
}

impl Run for LineChartArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        render_xy_chart(
            XyChartType::Line,
            self.title.as_deref(),
            self.x_axis.as_deref(),
            self.y_axis.as_deref(),
            self.width.as_ref().map(|w| w.to_string()).as_deref(),
            &self.layout,
            self.horizontal,
            self.show_data_label,
            self.aspect_ratio.map(|a| a.value()),
            false,
            self.bar,
            self.inverse,
            self.example,
            self.meta,
            &self.data,
            ctx.json,
            ctx.plain,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_xy_chart(
    chart_type: XyChartType,
    title: Option<&str>,
    x_axis: Option<&str>,
    y_axis: Option<&str>,
    width: Option<&str>,
    layout: &LayoutArgs,
    horizontal: bool,
    show_data_label: bool,
    aspect_ratio: Option<f32>,
    add_line: bool,
    add_bar: bool,
    inverse: bool,
    example: bool,
    meta: bool,
    data: &[String],
    json: bool,
    plain: bool,
) -> color_eyre::Result<()> {
    let _ = std::io::stdout().flush();

    let (data, use_example_labels): (Vec<String>, bool) = if example {
        let example_data = match chart_type {
            XyChartType::Bar => BAR_CHART_EXAMPLE,
            XyChartType::Line => LINE_CHART_EXAMPLE,
        };
        (example_data.iter().map(|s| s.to_string()).collect(), true)
    } else {
        (data.to_vec(), false)
    };

    let values = parse_xy_data(&data)?;

    if values.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "No valid data values provided. Use format: \"1 2 3\" or \"[1,2,3]\" or \"1,2,3\""
        ));
    }

    let (eff_title, eff_x_axis, eff_y_axis) = if use_example_labels {
        match chart_type {
            XyChartType::Bar => (
                Some("Monthly Revenue"),
                Some("Jan,Feb,Mar,Apr,May,Jun"),
                Some("$ (thousands)"),
            ),
            XyChartType::Line => (
                Some("Weekly Temperature"),
                Some("Mon,Tue,Wed,Thu,Fri,Sat,Sun"),
                Some("°C"),
            ),
        }
    } else {
        (title, x_axis, y_axis)
    };

    let aspect = aspect_ratio.unwrap_or(1.5);
    let init_config = format!(
        "%%{{init: {{\"xychart\": {{\"showTitle\": {}, \"xAxis\": {{\"showLabel\": {}}}, \"yAxis\": {{\"showLabel\": {}}}}}}}}}%%",
        eff_title.is_some(),
        eff_x_axis.is_some(),
        eff_y_axis.is_some()
    );

    let orientation = if horizontal { "horizontal" } else { "" };
    let chart_decl = format!("xychart-beta {}", orientation).trim().to_string();

    let x_axis_line = if let Some(labels) = eff_x_axis {
        if labels.contains(',') {
            let cats: Vec<&str> = labels.split(',').map(|s| s.trim()).collect();
            format!("    x-axis [{}]", cats.join(", "))
        } else {
            format!("    x-axis \"{}\"", labels)
        }
    } else {
        let default_labels: Vec<String> = (1..=values.len()).map(|i| i.to_string()).collect();
        format!("    x-axis [{}]", default_labels.join(", "))
    };

    let y_axis_line = if let Some(label) = eff_y_axis {
        format!("    y-axis \"{}\"", label)
    } else {
        String::new()
    };

    let title_line = eff_title
        .map(|t| format!("    title \"{}\"", t))
        .unwrap_or_default();

    let data_str: String = values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let primary_series = match chart_type {
        XyChartType::Bar => format!("    bar [{}]", data_str),
        XyChartType::Line => format!("    line [{}]", data_str),
    };

    let secondary_series = if add_line && chart_type == XyChartType::Bar {
        format!("\n    line [{}]", data_str)
    } else if add_bar && chart_type == XyChartType::Line {
        format!("\n    bar [{}]", data_str)
    } else {
        String::new()
    };

    let mut parts = vec![init_config, chart_decl];
    if !title_line.is_empty() {
        parts.push(title_line);
    }
    parts.push(x_axis_line);
    if !y_axis_line.is_empty() {
        parts.push(y_axis_line);
    }
    parts.push(primary_series);
    if !secondary_series.is_empty() {
        parts.push(secondary_series.trim().to_string());
    }

    let instructions = parts.join("\n");

    if json {
        let output = serde_json::json!({
            "type": match chart_type {
                XyChartType::Bar => "bar-chart",
                XyChartType::Line => "line-chart",
            },
            "inverse": inverse,
            "title": eff_title,
            "x_axis": eff_x_axis,
            "y_axis": eff_y_axis,
            "horizontal": horizontal,
            "show_data_label": show_data_label,
            "aspect_ratio": aspect,
            "add_line": add_line,
            "add_bar": add_bar,
            "values": values,
            "instructions": instructions,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let diagram = build_mermaid_diagram(&instructions, inverse, width, layout)?;
    let chart_name = match chart_type {
        XyChartType::Bar => "bar chart",
        XyChartType::Line => "line chart",
    };
    let terminal = terminal_for_render(plain);
    display_mermaid(
        &diagram,
        &instructions,
        chart_name,
        layout,
        meta,
        false,
        &terminal,
    )?;

    if example {
        let cmd = match chart_type {
            XyChartType::Bar => BAR_CHART_EXAMPLE_CMD,
            XyChartType::Line => LINE_CHART_EXAMPLE_CMD,
        };
        print_example_command_with_terminal(cmd, &terminal);
    }

    Ok(())
}

/// Parse XY chart data from various input formats.
pub fn parse_xy_data(data: &[String]) -> color_eyre::Result<Vec<f64>> {
    let mut values = Vec::new();

    for item in data {
        let trimmed = item.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let inner = &trimmed[1..trimmed.len() - 1];
            for part in inner.split(',') {
                let v: f64 = part.trim().parse().map_err(|_| {
                    color_eyre::eyre::eyre!("Invalid number in JSON array: '{}'", part.trim())
                })?;
                values.push(v);
            }
            continue;
        }

        if trimmed.contains(',') {
            for part in trimmed.split(',') {
                let v: f64 = part
                    .trim()
                    .parse()
                    .map_err(|_| color_eyre::eyre::eyre!("Invalid number: '{}'", part.trim()))?;
                values.push(v);
            }
            continue;
        }

        let v: f64 = trimmed
            .parse()
            .map_err(|_| color_eyre::eyre::eyre!("Invalid number: '{}'", trimmed))?;
        values.push(v);
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_column_width() {
        assert!(matches!(
            crate::commands::shared::parse_column_width("50").unwrap(),
            biscuit_terminal::components::two_column::ColumnWidth::Fixed(50)
        ));
        assert!(matches!(
            crate::commands::shared::parse_column_width("80ch").unwrap(),
            biscuit_terminal::components::two_column::ColumnWidth::Fixed(80)
        ));

        let width = crate::commands::shared::parse_column_width("40%").unwrap();
        if let biscuit_terminal::components::two_column::ColumnWidth::Percent(p) = width {
            assert!((p - 0.4).abs() < f32::EPSILON);
        } else {
            panic!("Expected Percent");
        }

        assert!(crate::commands::shared::parse_column_width("invalid").is_err());
        assert!(crate::commands::shared::parse_column_width("150%").is_err());
    }
}
