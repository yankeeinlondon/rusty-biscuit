// GitHub `ListReleases` authors no total.
use schematic_schema::github::GitHub;

fn main() {
    let client = GitHub::new();
    let pages = client.list_releases("octo", "widgets").pages();
    let _ = pages.total();
}
