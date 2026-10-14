// GitHub `GetRepository` is not a list: no pager, through the call or a generic send.
use schematic_schema::github::{GetRepositoryRequest, GitHub};

fn main() {
    let client = GitHub::new();
    let _ = client.send(GetRepositoryRequest::new("octo", "widgets")).pager();
    let _ = client.get_repository("octo", "widgets").pager();
}
