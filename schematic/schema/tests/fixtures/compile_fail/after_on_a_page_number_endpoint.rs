// GitHub `ListReleases` authors page numbers: no cursor to resume after.
use schematic_schema::github::GitHub;

fn main() {
    let client = GitHub::new();
    let _ = client.list_releases("octo", "widgets").pager().after("x");
    let _ = client.list_releases("octo", "widgets").after("x");
}
