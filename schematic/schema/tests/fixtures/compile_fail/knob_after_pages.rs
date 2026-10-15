// Knobs configure a traversal before it starts; `Pages` has none.
use schematic_schema::github::GitHub;

fn main() {
    let client = GitHub::new();
    let _ = client.list_releases("octo", "widgets").pages().page_size(5);
}
