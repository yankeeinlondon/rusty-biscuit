use pager_surface::Client;
fn main() {
    let client = Client::new("http://127.0.0.1:1");
    let _ = client.list_issues().keep_alive("1m");
}
