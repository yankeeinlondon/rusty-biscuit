// GitHub `ListReleases` authors no previous-page source.
use schematic_schema::github::GitHub;

async fn used(client: GitHub) {
    let mut pages = client.list_releases("octo", "widgets").pages();
    let _ = pages.prev().await;
}

fn main() {
    let _ = used(GitHub::new());
}
