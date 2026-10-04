use crate::filesystem::git::ApiFlavor;

pub(crate) trait ProviderRequestBuilder: Sized {
    fn header(self, name: &'static str, value: String) -> Self;
    fn basic_auth(self, token: &str) -> Self;
    fn bearer_auth(self, token: &str) -> Self;
}

impl ProviderRequestBuilder for reqwest::RequestBuilder {
    fn header(self, name: &'static str, value: String) -> Self {
        self.header(name, value)
    }

    fn basic_auth(self, token: &str) -> Self {
        self.basic_auth("", Some(token))
    }

    fn bearer_auth(self, token: &str) -> Self {
        self.bearer_auth(token)
    }
}

impl ProviderRequestBuilder for reqwest::blocking::RequestBuilder {
    fn header(self, name: &'static str, value: String) -> Self {
        self.header(name, value)
    }

    fn basic_auth(self, token: &str) -> Self {
        self.basic_auth("", Some(token))
    }

    fn bearer_auth(self, token: &str) -> Self {
        self.bearer_auth(token)
    }
}

pub(crate) fn authenticate_provider_request<R: ProviderRequestBuilder>(
    request: R,
    flavor: ApiFlavor,
    token: &str,
) -> R {
    match flavor {
        ApiFlavor::GitLab => request.header("PRIVATE-TOKEN", token.to_string()),
        ApiFlavor::Gitea | ApiFlavor::Forgejo => {
            request.header("Authorization", format!("token {token}"))
        }
        ApiFlavor::AzureDevOps => request.basic_auth(token),
        _ => request.bearer_auth(token),
    }
}

/// Environment variables that hold a provider-scoped token, in the order
/// [`provider_token`] consults them.
pub(crate) fn provider_token_variables(flavor: ApiFlavor) -> &'static [&'static str] {
    match flavor {
        ApiFlavor::GitHub => &["GH_TOKEN", "GITHUB_TOKEN"],
        ApiFlavor::GitLab => &["GITLAB_TOKEN", "GITLAB_PRIVATE_TOKEN"],
        ApiFlavor::Gitea | ApiFlavor::Forgejo => {
            &["GITEA_TOKEN", "FORGEJO_TOKEN", "CODEBERG_TOKEN"]
        }
        ApiFlavor::Bitbucket | ApiFlavor::BitbucketDataCenter => &["BITBUCKET_TOKEN"],
        ApiFlavor::AzureDevOps => &["AZURE_DEVOPS_TOKEN"],
        _ => &[],
    }
}

/// The first set variable of [`provider_token_variables`] as
/// `(name, value)`, and the variable to name when none is set.
///
/// A variable set to the empty string is unset: an empty token is never sent,
/// and it does not hide a later variable.
pub(crate) fn provider_token(flavor: ApiFlavor) -> (Option<(&'static str, String)>, &'static str) {
    let names = provider_token_variables(flavor);
    (
        names
            .iter()
            .find_map(|name| token_in(name).map(|token| (*name, token))),
        names.first().copied().unwrap_or("PROVIDER_TOKEN"),
    )
}

/// The host-bound `SNIFF_{PROVIDER}_{HOST}_TOKEN` override and its name; an
/// empty value is unset, as for [`provider_token`].
pub(crate) fn host_bound_provider_token(flavor: ApiFlavor, host: &str) -> (Option<String>, String) {
    let variable = host_bound_provider_variable(flavor, host);
    (token_in(&variable), variable)
}

fn token_in(variable: &str) -> Option<String> {
    std::env::var(variable).ok().filter(|token| !token.is_empty())
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
