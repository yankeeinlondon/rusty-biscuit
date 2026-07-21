use crate::filesystem::git::ApiFlavor;

pub(crate) fn provider_token(flavor: ApiFlavor) -> (Option<String>, &'static str) {
    let names: &[&str] = match flavor {
        ApiFlavor::GitHub => &["GH_TOKEN", "GITHUB_TOKEN"],
        ApiFlavor::GitLab => &["GITLAB_TOKEN", "GITLAB_PRIVATE_TOKEN"],
        ApiFlavor::Gitea | ApiFlavor::Forgejo => {
            &["GITEA_TOKEN", "FORGEJO_TOKEN", "CODEBERG_TOKEN"]
        }
        ApiFlavor::Bitbucket | ApiFlavor::BitbucketDataCenter => &["BITBUCKET_TOKEN"],
        ApiFlavor::AzureDevOps => &["AZURE_DEVOPS_TOKEN"],
        _ => &[],
    };
    (
        names.iter().find_map(|name| std::env::var(name).ok()),
        names.first().copied().unwrap_or("PROVIDER_TOKEN"),
    )
}
