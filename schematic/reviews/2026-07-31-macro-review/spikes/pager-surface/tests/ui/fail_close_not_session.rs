use pager_surface::Client;
async fn run(client: Client) {
    let mut pages = client.list_roles().pages();
    let _ = pages.close().await;
}
fn main() { let _ = run; }
