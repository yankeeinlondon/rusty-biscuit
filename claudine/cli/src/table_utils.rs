use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::utils::layout::Margin;

pub(crate) fn base_table(columns: Vec<TableColumn>) -> Table {
    let mut table = Table::new().with_columns(columns);
    table.layout_mut().left_margin = Margin::Chars(1);
    table
}
