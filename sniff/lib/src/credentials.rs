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

pub(crate) fn host_bound_provider_token(
    flavor: ApiFlavor,
    host: &str,
) -> (Option<String>, String) {
    let variable = host_bound_provider_variable(flavor, host);
    (std::env::var(&variable).ok(), variable)
}

fn host_bound_provider_variable(flavor: ApiFlavor, host: &str) -> String {
    let provider = match flavor {
        ApiFlavor::GitHub => "GITHUB",
        ApiFlavor::GitLab => "GITLAB",
        ApiFlavor::Gitea => "GITEA",
        ApiFlavor::Forgejo => "FORGEJO",
        ApiFlavor::Bitbucket | ApiFlavor::BitbucketDataCenter => "BITBUCKET",
        ApiFlavor::AzureDevOps => "AZURE_DEVOPS",
        _ => "PROVIDER",
    };
    let host = host.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() {
            encoded.push(char::from(byte).to_ascii_uppercase());
        } else {
            use std::fmt::Write;
            write!(encoded, "_{byte:02X}_").expect("writing to a String cannot fail");
        }
        encoded
    });
    format!("SNIFF_{provider}_{host}_TOKEN")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_bound_variables_preserve_exact_provider_and_host_identity() {
        assert_eq!(
            host_bound_provider_variable(ApiFlavor::GitLab, "git.example"),
            "SNIFF_GITLAB_GIT_2E_EXAMPLE_TOKEN"
        );
        assert_eq!(
            host_bound_provider_variable(ApiFlavor::Gitea, "git-example"),
            "SNIFF_GITEA_GIT_2D_EXAMPLE_TOKEN"
        );
        assert_ne!(
            host_bound_provider_variable(ApiFlavor::GitLab, "git.example"),
            host_bound_provider_variable(ApiFlavor::GitLab, "git-example")
        );
    }
}
