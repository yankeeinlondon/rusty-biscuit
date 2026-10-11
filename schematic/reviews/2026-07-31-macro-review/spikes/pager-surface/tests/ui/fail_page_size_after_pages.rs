use pager_surface::Client;
fn main() {
    let client = Client::new("http://127.0.0.1:1");
    let _ = client.list_issues().pages().page_size(10);
}
