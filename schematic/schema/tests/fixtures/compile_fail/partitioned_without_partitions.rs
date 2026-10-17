// GitHub `ListReleases` authors no partitions.
use schematic_schema::github::GitHub;

fn main() {
    let client = GitHub::new();
    let _ = client.list_releases("octo", "widgets").pager().partitioned(2);
}
