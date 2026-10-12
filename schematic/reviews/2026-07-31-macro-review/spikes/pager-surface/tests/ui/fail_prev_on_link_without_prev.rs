use pager_surface::Client;
async fn run(client: Client) {
    let mut pages = client.list_repos().pages();
    let _ = pages.prev().await;
}
fn main() { let _ = run; }
