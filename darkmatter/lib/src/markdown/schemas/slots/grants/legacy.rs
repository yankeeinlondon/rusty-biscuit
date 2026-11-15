//! The retired user-level grant store (`<home>/.config/darkmatter/schema-read-policy`).
//!
//! Schema-slot reads are no longer decided by it: `permissions.file-read`
//! and this run's approvals decide them. What remains is what the one-time
//! migration into `permissions.file-read` (`config::migration`) reads: the
//! store's location and the grammar of one line's target.

use std::path::PathBuf;

use super::GrantTarget;

/// The retired store's location below the user's home, as path components.
pub const GRANT_STORE_RELATIVE_PATH: [&str; 3] = [".config", "darkmatter", "schema-read-policy"];

/// The target a store line's kind word and argument name.
pub(super) fn target_from_words(kind: &str, argument: &str) -> Result<GrantTarget, String> {
    Ok(match kind {
        "file" => GrantTarget::File(stored_path(argument)?),
        "dir" => GrantTarget::Dir(stored_path(argument)?),
        "url" => GrantTarget::Url(http_url(argument)?),
        "url-any-query" => {
            let url = http_url(argument)?;
            if url.query().is_some() {
                return Err(format!("`{argument}` has a query; a `url-any-query` grant is written without one"));
            }
            GrantTarget::UrlAnyQuery(url)
        }
        "site" => site(argument)?,
        other => {
            return Err(format!("unknown kind `{other}`; expected file, dir, url, url-any-query, or site"));
        }
    })
}

fn stored_path(argument: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(argument);
    if !path.is_absolute() {
        return Err(format!("`{argument}` is not an absolute path"));
    }
    // A grant for a path that does not exist (yet) keeps its written form;
    // only an existing path has a canonical one to compare by.
    match biscuit_file::canonicalize_simplified(&path) {
        Ok(canonical) => Ok(canonical),
        Err(_) => Ok(path),
    }
}

fn http_url(argument: &str) -> Result<url::Url, String> {
    let url = url::Url::parse(argument).map_err(|error| format!("`{argument}` is not a URL: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("`{argument}` is not an http or https URL"));
    }
    if url.fragment().is_some() {
        return Err(format!("`{argument}` has a fragment; a grant URL has none"));
    }
    Ok(url)
}

fn site(argument: &str) -> Result<GrantTarget, String> {
    let url = http_url(argument)?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(format!("`{argument}` has user information; a site is `scheme://host[:port]`"));
    }
    if url.path() != "/" || url.query().is_some() {
        return Err(format!("`{argument}` has a path or query; a site is `scheme://host[:port]`"));
    }
    let Some(host) = url.host_str() else {
        return Err(format!("`{argument}` has no host"));
    };
    Ok(GrantTarget::Site { scheme: url.scheme().to_string(), host: host.to_string(), port: url.port() })
}
