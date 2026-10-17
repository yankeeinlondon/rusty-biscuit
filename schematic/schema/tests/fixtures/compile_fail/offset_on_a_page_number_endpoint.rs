// GitHub `ListReleases` authors page numbers, not offsets.
use schematic_schema::github::GitHub;

fn main() {
    let client = GitHub::new();
    let _ = client.list_releases("octo", "widgets").pager().offset(20);
}
