use pager_surface::{encoding_a1::Pager1, generated::*};
fn main() {
    let _ = Pager1::<ListIssues>::new().page_size(10).page(2).pages();
    let _ = Pager1::<ListBatches>::new().after("x").pages().prev();
    Pager1::<SearchScroll>::new().keep_alive("1m").pages().close();
}
