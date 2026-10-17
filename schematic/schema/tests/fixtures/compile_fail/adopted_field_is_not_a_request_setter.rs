// `per_page` is adopted by the pager: the request no longer carries it.
use schematic_schema::github::{GitHub, ListReleasesRequest};

fn main() {
    let client = GitHub::new();
    let _ = client.list_releases("octo", "widgets").per_page(5);
    let _ = ListReleasesRequest { per_page: Some(5), ..ListReleasesRequest::new("octo", "widgets") };
}
