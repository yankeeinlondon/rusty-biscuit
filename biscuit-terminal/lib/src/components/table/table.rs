use crate::components::renderable::Renderable;

#[derive(Debug)]
pub enum TableCellContent {
    /// Text (which can include escape characters)
    Text(String),
}

#[derive(Debug)]
pub enum TableColumn {}

#[derive(Debug)]
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
