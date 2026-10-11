use pager_surface::{Client, PagerError};
use pager_surface::encoding_b::list_issues_b;

async fn run(client: Client) -> Result<(), PagerError> {
    // page number: page_size + page
    let mut issues = client.list_issues().page_size(50)?.page(2).deadline(std::time::Duration::from_secs(5)).pages();
    let _ = issues.next().await?;
    // resumable bidirectional cursor: page_size + after + before + prev
    let mut batches = client.list_batches().page_size(10)?.after("x").pages();
    let _ = batches.prev().await?;
    let _ = client.list_batches().before("y").pages();
    // resumable forward-only cursor: after
    let _ = client.list_events().after("e").pages();
    // opaque cursor: only pages/next/items
    let _ = client.list_roles().pages().items();
    // next link: body URL with size; Link header with prev
    let _ = client.list_repos().page_size(100)?.pages();
    let mut commits = client.list_commits().pages();
    let _ = commits.prev().await?;
    // session: page_size + keep_alive + close + with_session
    let mut scroll = client.search_scroll().page_size(1000)?.keep_alive("2m").pages();
    let _ = scroll.next().await?;
    scroll.close().await?;
    client.search_scroll().with_session(async |pages| { pages.next().await?; Ok(()) }).await?;
    // encoding B
    let _ = list_issues_b().page_size(10).unwrap().page(3);
    Ok(())
}
fn main() { let _ = run; }
