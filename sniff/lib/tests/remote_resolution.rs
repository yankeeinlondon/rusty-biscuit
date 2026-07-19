use sniff::SniffError;
use sniff::filesystem::git::{ApiFlavor, GitRepo, resolve_remote_at};
use sniff::filesystem::preferred_remote_url;

fn repository() -> (tempfile::TempDir, git2::Repository) {
    let directory = tempfile::tempdir().unwrap();
    let repository = git2::Repository::init(directory.path()).unwrap();
    (directory, repository)
}

#[test]
fn preferred_remote_ignores_url_less_entries_and_uses_contract_order() {
    let (directory, repository) = repository();
    repository.remote("upstream", "https://gitlab.com/group/upstream.git").unwrap();
    repository.remote("zebra", "https://github.com/acme/zebra.git").unwrap();
    repository.remote("alpha", "https://github.com/acme/alpha.git").unwrap();
    repository.config().unwrap().set_str("remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*").unwrap();

    let resolved = resolve_remote_at(directory.path(), None).unwrap().unwrap();
    assert_eq!(resolved.name, "alpha");

    repository.remote("origin", "https://github.com/acme/origin.git").unwrap();
    let resolved = resolve_remote_at(directory.path(), None).unwrap().unwrap();
    assert_eq!(resolved.name, "origin");
}

#[test]
fn exact_remote_resolution_preserves_urls_and_nested_identity() {
    let (directory, repository) = repository();
    repository.remote("source", "git@gitlab.com:group/nested/project.git").unwrap();
    repository.config().unwrap()
        .set_str("remote.source.pushurl", "ssh://git@gitlab.com/group/nested/project-write.git")
        .unwrap();

    let resolved = resolve_remote_at(directory.path(), Some("source")).unwrap().unwrap();
    assert_eq!(resolved.fetch_url, "git@gitlab.com:group/nested/project.git");
    assert_eq!(resolved.push_url, "ssh://git@gitlab.com/group/nested/project-write.git");
    assert_eq!(resolved.host.as_deref(), Some("gitlab.com"));
    assert_eq!(resolved.namespace.as_deref(), Some("group/nested"));
    assert_eq!(resolved.repository.as_deref(), Some("project"));
    assert_eq!(resolved.api_flavor, ApiFlavor::GitLab);
}

#[test]
fn azure_ssh_remote_is_classified_without_network_access() {
    let (directory, repository) = repository();
    repository
        .remote("origin", "git@ssh.dev.azure.com:v3/acme/widgets/project")
        .unwrap();

    let resolved = resolve_remote_at(directory.path(), None).unwrap().unwrap();
    assert_eq!(resolved.api_flavor, ApiFlavor::AzureDevOps);
    assert_eq!(resolved.host.as_deref(), Some("ssh.dev.azure.com"));
}

/// AC19: the aggregate `GitRepo` projection and the shared resolver must
/// select the same remote. A URL-less `origin` used to win the aggregate
/// selection and then yield no org/repo at all, while `resolve_remote_at`
/// skipped it and reported the usable remote.
#[test]
fn aggregate_projection_and_resolver_agree_when_origin_has_no_url() {
    let (directory, repository) = repository();
    repository
        .config()
        .unwrap()
        .set_str("remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*")
        .unwrap();
    repository.remote("alpha", "https://github.com/acme/alpha.git").unwrap();

    let resolved = resolve_remote_at(directory.path(), None).unwrap().unwrap();
    assert_eq!(resolved.name, "alpha");
    assert_eq!(
        preferred_remote_url(directory.path()).unwrap().as_deref(),
        Some("https://github.com/acme/alpha.git")
    );

    let repo = GitRepo::discover(directory.path()).unwrap().unwrap();
    assert_eq!(
        repo.org_and_repo(),
        (Some("acme".to_string()), Some("alpha".to_string()))
    );
}

#[test]
fn explicit_missing_and_url_less_remotes_are_distinct_errors() {
    let (directory, repository) = repository();
    repository.config().unwrap().set_str("remote.empty.fetch", "+refs/heads/*:refs/remotes/empty/*").unwrap();

    assert!(matches!(
        resolve_remote_at(directory.path(), Some("missing")),
        Err(SniffError::RemoteNotConfigured { name }) if name == "missing"
    ));
    assert!(matches!(
        resolve_remote_at(directory.path(), Some("empty")),
        Err(SniffError::RemoteUrlMissing { name }) if name == "empty"
    ));
}
