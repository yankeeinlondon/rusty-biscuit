// No shipped strategy marks its page size adjustable mid-traversal.
use schematic_schema::bitbucket::Bitbucket;

fn main() {
    let client = Bitbucket::new();
    let mut pages = client.list_tags("team", "widgets").pages();
    pages.set_page_size(5);
}
