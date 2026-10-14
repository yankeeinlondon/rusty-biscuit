// GitLab `ListRepositoryTree` models a `keyset` alternate, which generates no caller API.
use schematic_schema::gitlab::GitLab;

fn main() {
    let client = GitLab::new();
    let _ = client.list_repository_tree("1").alternate("keyset");
    let _ = client.list_repository_tree("1").pager().keyset();
}
