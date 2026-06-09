use crate::commands::color_parse::parse_color;
use crate::commands::shared::{detect_terminal_honoring_force_color, print_example_command};
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use clap::Args as ClapArgs;
use renderable::markdown::MarkdownRenderable;

/// Wraps a [`Color`](renderable::color::Color) into the opaque universal
/// `TargetValue<PerMode<PaintColor>>` shape a [`Style`](renderable::style::Style)
/// color slot expects.
fn color_style_value(
    color: renderable::color::Color,
) -> renderable::layout::TargetValue<renderable::style::PerMode<renderable::style::PaintColor>> {
    renderable::layout::TargetValue::universal(renderable::style::PerMode::universal(color))
}

const TABLE_EXAMPLE_COLUMNS: &str = "Area,Status,Owner";
const TABLE_EXAMPLE_ROWS: &[&str] = &[
    "Parser,Green,Lin",
    "Renderer,Review,Mara",
    "Docs,Updated,Noor",
];
const TABLE_EXAMPLE_CMD: &str = r#"bt table --columns "Area,Status,Owner" --row "Parser,Green,Lin" --row "Renderer,Review,Mara" --row "Docs,Updated,Noor" --striped"#;

/// Render a table through the canonical render tree.
///
/// The default target is the terminal. Switch to a different render target
/// with one of the mutually-exclusive `--html`, `--md`, or `--md-plus` flags.
#[derive(ClapArgs, Debug, Clone)]
pub struct TableArgs {
    /// Render an example and show the command used
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Comma-separated column headers.
    #[arg(long, required_unless_present = "example")]
    pub columns: Option<String>,

    /// A comma-separated row of cells; repeat for multiple rows.
    #[arg(long = "row")]
    pub rows: Vec<String>,

    /// A comma-separated row whose cells are parsed as [`Prose`] markup, so
    /// inline tags (`<b>`, `<red>`, `<a href=…>`, …) become capability-resolved
    /// `StyledProse` cells. Repeat for multiple rows; appended after `--row`.
    #[arg(long = "prose-row")]
    pub prose_rows: Vec<String>,

    /// Render through the cursor-alignment bespoke path
    /// (`Table::prefer_cursor_alignment`) instead of the standard tree layout.
    /// Only meaningful for the terminal target on a TTY.
    #[arg(long = "cursor-align")]
    pub cursor_align: bool,

    /// Optional table title. Renders as a caption above the table on every
    /// target (terminal caption line, HTML `<caption>`, Markdown plain-text
    /// line preceding the table).
    #[arg(long)]
    pub title: Option<String>,

    /// Stripe alternate rows with a background color.
    #[arg(long)]
    pub striped: bool,

    /// Stripe background color (named or #rrggbb).
    #[arg(long = "stripe-bg")]
    pub stripe_bg: Option<String>,

    /// Stripe text color (named or #rrggbb).
    #[arg(long = "stripe-text")]
    pub stripe_text: Option<String>,

    /// Render every column header in bold.
    #[arg(long = "bold-header")]
    pub bold_header: bool,

    /// Header text color (named or #rrggbb).
    #[arg(long = "header-color")]
    pub header_color: Option<String>,

    /// Body (data cell) text color (named or #rrggbb).
    #[arg(long = "body-color")]
    pub body_color: Option<String>,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with_all = ["md", "md_plus"])]
    pub html: bool,

    /// Render to portable Markdown (GFM) instead of the terminal.
    #[arg(long, conflicts_with_all = ["html", "md_plus"])]
    pub md: bool,

    /// Render to MarkdownPlus (inline HTML allowed) instead of the terminal.
    #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
    pub md_plus: bool,
}

impl Run for TableArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let columns_arg = self
            .columns
            .unwrap_or_else(|| TABLE_EXAMPLE_COLUMNS.to_string());
        let headers: Vec<&str> = columns_arg
            .split(',')
            .map(str::trim)
            .filter(|h| !h.is_empty())
            .collect();
        if headers.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No columns provided. Usage: bt table --columns \"Name,Score\" --row \"Ann,90\""
            ));
        }

        let columns: Vec<TableColumn> = headers.iter().map(|h| TableColumn::new(*h)).collect();

        let rows = if self.example && self.rows.is_empty() {
            TABLE_EXAMPLE_ROWS
                .iter()
                .map(|row| row.to_string())
                .collect()
        } else {
            self.rows
        };

        let mut data: Vec<Vec<TableCellContent>> = rows
            .iter()
            .map(|row| {
                row.split(',')
                    .map(|cell| TableCellContent::from(cell.trim().to_string()))
                    .collect()
            })
            .collect();

        // `--prose-row` cells are parsed as Prose markup so inline tags become
        // capability-resolved `StyledProse` cells. Appended after the plain rows.
        for row in &self.prose_rows {
            let cells: Vec<TableCellContent> = row
                .split(',')
                .map(|cell| TableCellContent::from(Prose::new(cell.trim())))
                .collect();
            data.push(cells);
        }

        let mut table = Table::new().with_columns(columns).with_data(data);

        if self.cursor_align {
            table = table.prefer_cursor_alignment();
        }

        if let Some(title) = self.title.as_ref() {
            table = table.with_title(title.clone());
        }

        if self.striped || self.example {
            table = table.alternate_background_color();
        }
        if let Some(color) = &self.stripe_bg {
            table = table.with_stripe_bg(parse_color(color)?);
        }
        if let Some(color) = &self.stripe_text {
            table = table.with_stripe_text(parse_color(color)?);
        }

        // Typed header / body slot styling (Spec B D5).
        let mut header_style = renderable::style::Style::default();
        if self.bold_header {
            header_style.emphasis.bold = true;
        }
        if let Some(color) = &self.header_color {
            header_style.color = Some(color_style_value(parse_color(color)?));
        }
        if !header_style.is_empty() {
            table = table.with_header_style(header_style);
        }
        if let Some(color) = &self.body_color {
            let body_style = renderable::style::Style {
                color: Some(color_style_value(parse_color(color)?)),
                ..renderable::style::Style::default()
            };
            table = table.with_body_style(body_style);
        }

        // Dispatch by selected target. Mutual exclusion is enforced by clap.
        if self.html {
            println!("{}", table.render_html_fragment().render());
        } else if self.md {
            println!("{}", table.render_markdown());
        } else if self.md_plus {
            println!("{}", table.render_markdown_plus());
        } else {
            let term = detect_terminal_honoring_force_color();
            println!("{}", table.render(&term));
        }

        if self.example {
            print_example_command(TABLE_EXAMPLE_CMD);
        }
        Ok(())
    }
}
