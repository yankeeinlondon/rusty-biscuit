use crate::components::renderable::Renderable;

pub enum TableCellContent {
    /// Text (which can include escape characters)
    Text(String),
}

pub enum TableColumn {}

#[allow(dead_code)]
pub struct Table {
    title: Option<String>,
    columns: Option<Vec<TableColumn>>,
    data: Option<Vec<Vec<TableCellContent>>>,
}

impl Renderable for Table {
    fn render() -> String {
        todo!()
    }

    fn fallback_render(_term: &crate::terminal::Terminal) -> String {
        todo!()
    }
}
