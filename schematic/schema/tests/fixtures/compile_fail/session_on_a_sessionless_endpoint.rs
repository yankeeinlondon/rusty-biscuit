// No shipped endpoint authors a session: no scope can be opened, and no keep-alive set.
use schematic_schema::github::GitHub;

fn main() {
    let client = GitHub::new();
    let _ = client
        .list_releases("octo", "widgets")
        .pager()
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }));
    let _ = client.list_releases("octo", "widgets").pager().keep_alive("5m");
}
