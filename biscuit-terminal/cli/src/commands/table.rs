use crate::commands::color_parse::parse_color;
use crate::commands::shared::{print_example_command, terminal_for_render};
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::table::types::{ColumnType, Currency};
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

/// Maps one `--column-types` token to its [`ColumnType`].
///
/// `int`/`integer`, `float`, and the currency codes `usd`/`gbp`/`eur` declare a
/// right-aligned typed column; an empty token or `string`/`text` is a
/// left-aligned text column.
///
/// Column types live in their own flag rather than as a `Header:type` suffix on
/// `--columns` so that literal header text — including a header that happens to
/// end in a type word, such as `Revenue: USD` — is never reinterpreted as a
/// type.
///
/// ## Errors
///
/// Returns an error for any other token: `--column-types` is an explicit type
/// declaration, so an unrecognized token is a user mistake, not header text.
fn parse_column_type(token: &str) -> color_eyre::Result<ColumnType> {
    let token = token.trim();
    if token.is_empty() || token.eq_ignore_ascii_case("string") || token.eq_ignore_ascii_case("text")
    {
        return Ok(ColumnType::String);
    }
    recognized_column_type(token).ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "unknown column type {token:?}; expected int, integer, float, usd, gbp, eur, string, or text"
        )
    })
}

/// Maps a recognized `--column-types` type token to its [`ColumnType`], or
/// `None` for any token that is not a declared numeric/currency type.
fn recognized_column_type(token: &str) -> Option<ColumnType> {
    Some(match token.to_ascii_lowercase().as_str() {
        "int" | "integer" => ColumnType::Integer,
        "float" => ColumnType::Float,
        "usd" => ColumnType::Currency(Currency::USD),
        "gbp" => ColumnType::Currency(Currency::GBP),
        "eur" => ColumnType::Currency(Currency::EUR),
        _ => return None,
    })
}

/// Builds the [`TableCellContent`] for one `--mixed-row` cell from its column's
/// type: a numeric column parses the value into the matching typed cell; any
/// other column parses the cell as [`Prose`] markup (`\n` → hard line break, as
/// in `--prose-row`).
///
/// ## Errors
///
/// Returns an error when a numeric column's cell does not parse as the expected
/// number.
fn typed_cell(value: &str, column_type: &ColumnType) -> color_eyre::Result<TableCellContent> {
    let cell = match column_type {
        ColumnType::String => TableCellContent::from(Prose::new(value.replace("\\n", "\n"))),
        ColumnType::Integer => TableCellContent::Integer(
            value
                .parse()
                .map_err(|e| color_eyre::eyre::eyre!("invalid integer cell {value:?}: {e}"))?,
        ),
        ColumnType::Float => TableCellContent::Float(
            value
                .parse()
                .map_err(|e| color_eyre::eyre::eyre!("invalid float cell {value:?}: {e}"))?,
        ),
        ColumnType::Currency(currency) => TableCellContent::Currency(
            currency.clone(),
            value
                .parse()
                .map_err(|e| color_eyre::eyre::eyre!("invalid currency cell {value:?}: {e}"))?,
        ),
    };
    Ok(cell)
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

    /// Comma-separated column headers. Header text is literal; declare typed
    /// columns with the separate `--column-types` flag. A bare table is all
    /// left-aligned text columns.
    #[arg(long, required_unless_present = "example")]
    pub columns: Option<String>,

    /// Optional comma-separated column types, positionally aligned with
    /// `--columns`: `int`/`integer`, `float`, a currency code (`usd`, `gbp`,
    /// `eur`), or empty / `string` for a left-aligned text column. A numeric or
    /// currency type right-aligns the column and formats its `--mixed-row`
    /// cells. Kept separate from `--columns` so header text is never
    /// reinterpreted as a type.
    #[arg(long = "column-types")]
    pub column_types: Option<String>,

    /// A comma-separated row of cells; repeat for multiple rows.
    #[arg(long = "row")]
    pub rows: Vec<String>,

    /// A comma-separated row whose cells are parsed as [`Prose`] markup, so
    /// inline tags (`<b>`, `<red>`, `<a href=…>`, …) become capability-resolved
    /// `StyledProse` cells. Repeat for multiple rows; appended after `--row`.
    ///
    /// A literal `\n` in a cell becomes a hard line break inside that styled
    /// run, so one cell can span multiple visual lines.
    #[arg(long = "prose-row")]
    pub prose_rows: Vec<String>,

    /// A comma-separated row whose cells are interpreted by their column's
    /// declared type (see `--columns`): a numeric column parses the value into
    /// a typed, right-aligned cell, while every other column parses the cell as
    /// [`Prose`] markup (a left-aligned `StyledProse` cell, `\n` → hard line
    /// break). Lets one row mix a styled Prose cell with typed numeric cells.
    /// Repeat for multiple rows; appended after `--prose-row`.
    #[arg(long = "mixed-row")]
    pub mixed_rows: Vec<String>,

    /// Cap every column to this character width so long cells wrap onto
    /// multiple visual lines. Useful for exercising wrapped styled cells.
    #[arg(long = "col-width")]
    pub col_width: Option<usize>,

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
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let columns_arg = self
            .columns
            .unwrap_or_else(|| TABLE_EXAMPLE_COLUMNS.to_string());
        let headers: Vec<String> = columns_arg
            .split(',')
            .map(str::trim)
            .filter(|h| !h.is_empty())
            .map(str::to_string)
            .collect();
        // Types are declared positionally in their own flag; a column with no
        // matching entry stays a left-aligned text column.
        let column_types: Vec<ColumnType> = match self.column_types.as_deref() {
            Some(spec) => spec
                .split(',')
                .map(parse_column_type)
                .collect::<color_eyre::Result<_>>()?,
            None => Vec::new(),
        };
        let specs: Vec<(String, ColumnType)> = headers
            .into_iter()
            .enumerate()
            .map(|(idx, header)| (header, column_types.get(idx).cloned().unwrap_or_default()))
            .collect();
        if specs.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No columns provided. Usage: bt table --columns \"Name,Score\" --row \"Ann,90\""
            ));
        }

        let columns: Vec<TableColumn> = specs
            .iter()
            .map(|(header, column_type)| {
                let column = TableColumn::new(header.clone()).with_type(column_type.clone());
                match self.col_width {
                    Some(width) => column.with_max_width(width),
                    None => column,
                }
            })
            .collect();

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
                .map(|cell| {
                    // A literal `\n` in the cell becomes a hard line break inside
                    // the styled run; shells cannot pass a raw newline as one
                    // argument word.
                    let markup = cell.trim().replace("\\n", "\n");
                    TableCellContent::from(Prose::new(markup))
                })
                .collect();
            data.push(cells);
        }

        // `--mixed-row` cells take their kind from the matching column's type,
        // so one row can carry a left-aligned styled Prose cell next to typed,
        // right-aligned numeric cells. Appended after the Prose rows.
        for row in &self.mixed_rows {
            let cells: Vec<TableCellContent> = row
                .split(',')
                .enumerate()
                .map(|(idx, cell)| {
                    let column_type = specs
                        .get(idx)
                        .map(|(_, column_type)| column_type.clone())
                        .unwrap_or_default();
                    typed_cell(cell.trim(), &column_type)
                })
                .collect::<color_eyre::Result<_>>()?;
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
            let term = terminal_for_render(ctx.plain);
            println!("{}", table.render(&term));
        }

        if self.example {
            print_example_command(TABLE_EXAMPLE_CMD);
        }
        Ok(())
    }
}
