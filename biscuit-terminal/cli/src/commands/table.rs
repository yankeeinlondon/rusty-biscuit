use crate::commands::color_parse::parse_color;
use crate::commands::shared::detect_terminal_honoring_force_color;
use crate::commands::{CliContext, Run};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use clap::Args as ClapArgs;
use renderable::tree::{RenderNode, RenderStrictness};

/// Render a table through the render tree
#[derive(ClapArgs, Debug, Clone)]
pub struct TableArgs {
    /// Comma-separated column headers.
    #[arg(long, required = true)]
    pub columns: String,

    /// A comma-separated row of cells; repeat for multiple rows.
    #[arg(long = "row")]
    pub rows: Vec<String>,

    /// Stripe alternate rows with a background color.
    #[arg(long)]
    pub striped: bool,

    /// Stripe background color (named or #rrggbb).
    #[arg(long = "stripe-bg")]
    pub stripe_bg: Option<String>,

    /// Stripe text color (named or #rrggbb).
    #[arg(long = "stripe-text")]
    pub stripe_text: Option<String>,
}

impl Run for TableArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let headers: Vec<&str> = self
            .columns
            .split(',')
            .map(str::trim)
            .filter(|h| !h.is_empty())
            .collect();
        if headers.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No columns provided. Usage: bt table --columns \"Name,Score\" --row \"Ann,90\""
            ));
        }

        let columns: Vec<TableColumn> =
            headers.iter().map(|h| TableColumn::new(*h)).collect();

        let data: Vec<Vec<TableCellContent>> = self
            .rows
            .iter()
            .map(|row| {
                row.split(',')
                    .map(|cell| TableCellContent::from(cell.trim().to_string()))
                    .collect()
            })
            .collect();

        let mut table = Table::new().with_columns(columns).with_data(data);

        if self.striped {
            table = table.alternate_background_color();
        }
        if let Some(color) = &self.stripe_bg {
            table = table.with_stripe_bg(parse_color(color)?);
        }
        if let Some(color) = &self.stripe_text {
            table = table.with_stripe_text(parse_color(color)?);
        }

        let node = table.render_tree_node().ok_or_else(|| {
            color_eyre::eyre::eyre!("Table component produced no render-tree node")
        })?;
        let root = RenderNode::root(vec![node]);

        let term = detect_terminal_honoring_force_color();
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        let rendered = render_terminal_node(&root, &opts)
            .map_err(|e| color_eyre::eyre::eyre!("render failed: {e}"))?;

        println!("{}", rendered.output);
        Ok(())
    }
}
