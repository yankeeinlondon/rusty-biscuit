use pager_surface::{Client, Pager, generated::ListModels};
fn main() {
    let client = Client::new("http://127.0.0.1:1");
    let _ = Pager::<ListModels>::new(&client);
}
