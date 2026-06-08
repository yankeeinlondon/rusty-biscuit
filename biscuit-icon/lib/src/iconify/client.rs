use reqwest::Url;
use serde::Deserialize;

use crate::body::IconBody;
use crate::error::{IconError, Result};

const DEFAULT_BASE: &str = "https://api.iconify.design";

/// Allowed characters in an Iconify prefix or name: ASCII alphanumeric,
/// hyphen, and underscore.
fn is_valid_id_part(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// A thin async client over the Iconify JSON API.
#[derive(Debug, Clone)]
pub struct IconifyClient {
    http: reqwest::Client,
    base: String,
}

/// Splits a `prefix:name` identifier into its parts.
///
/// # Errors
/// Returns [`IconError::InvalidIdentifier`] when there is not exactly one `:`
/// with non-empty, syntactically valid parts on both sides.
pub fn parse_id(id: &str) -> Result<(String, String)> {
    let (prefix, name) = id.split_once(':').ok_or_else(|| IconError::InvalidIdentifier(id.to_string()))?;
    if !is_valid_id_part(prefix) || !is_valid_id_part(name) || name.contains(':') {
        return Err(IconError::InvalidIdentifier(id.to_string()));
    }
    Ok((prefix.to_string(), name.to_string()))
}

#[derive(Deserialize)]
struct CollectionResponse {
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    icons: std::collections::HashMap<String, IconEntry>,
}

#[derive(Deserialize)]
struct IconEntry {
    body: String,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}

/// License metadata for an Iconify collection.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct License {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub spdx: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Deserialize)]
struct CollectionMeta {
    #[serde(default)]
    name: String,
    #[serde(default)]
    license: Option<License>,
}

#[derive(Deserialize)]
struct SearchResponse {
    icons: Vec<String>,
    #[serde(default)]
    _total: usize,
    #[serde(default)]
    _limit: usize,
}

impl IconifyClient {
    /// Builds a client targeting the public Iconify API.
    #[must_use]
    pub fn new() -> Self {
        Self { http: reqwest::Client::new(), base: DEFAULT_BASE.to_string() }
    }

    /// Builds a client targeting a custom base URL (used in tests).
    #[must_use]
    pub fn with_base(base: impl Into<String>) -> Self {
        Self { http: reqwest::Client::new(), base: base.into() }
    }

    /// Fetches a single icon body by `prefix:name`.
    ///
    /// # Errors
    /// - [`IconError::InvalidIdentifier`] for malformed ids.
    /// - [`IconError::Fetch`] on transport/HTTP failure.
    /// - [`IconError::NotFound`] when the icon is absent from the response.
    pub async fn fetch_body(&self, id: &str) -> Result<IconBody> {
        let (prefix, name) = parse_id(id)?;
        let mut url = Url::parse(&self.base)
            .map_err(|e| IconError::Fetch(e.to_string()))?
            .join(&format!("{}.json", prefix))
            .map_err(|e| IconError::Fetch(e.to_string()))?;
        url.query_pairs_mut().append_pair("icons", &name);
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| IconError::Fetch(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(IconError::Fetch(format!("HTTP {}", resp.status())));
        }
        let data: CollectionResponse = resp
            .json()
            .await
            .map_err(|e| IconError::Fetch(e.to_string()))?;
        let entry = data.icons.get(&name).ok_or_else(|| IconError::NotFound(id.to_string()))?;
        let width = entry.width.or(data.width).unwrap_or(16);
        let height = entry.height.or(data.height).unwrap_or(16);
        Ok(IconBody::new(entry.body.clone(), width, height))
    }

    /// Fetches the list of Iconify set prefixes, each with its human title
    /// and optional license.
    ///
    /// # Errors
    /// [`IconError::Fetch`] on transport/HTTP/parse failure.
    pub async fn fetch_collections(&self) -> Result<Vec<(String, String, Option<License>)>> {
        let url = Url::parse(&self.base)
            .map_err(|e| IconError::Fetch(e.to_string()))?
            .join("collections")
            .map_err(|e| IconError::Fetch(e.to_string()))?;
        let resp = self.http.get(url).send().await.map_err(|e| IconError::Fetch(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(IconError::Fetch(format!("HTTP {}", resp.status())));
        }
        let map: std::collections::BTreeMap<String, CollectionMeta> =
            resp.json().await.map_err(|e| IconError::Fetch(e.to_string()))?;
        Ok(map.into_iter().map(|(prefix, meta)| (prefix, meta.name, meta.license)).collect())
    }

    /// Searches the Iconify catalog for icons matching `query`.
    ///
    /// Returns up to `limit` `prefix:name` identifiers (default 20).
    ///
    /// # Errors
    /// [`IconError::Fetch`] on transport/HTTP/parse failure.
    pub async fn search_icons(&self, query: &str, limit: Option<usize>) -> Result<Vec<String>> {
        let mut url = Url::parse(&self.base)
            .map_err(|e| IconError::Fetch(e.to_string()))?
            .join("search")
            .map_err(|e| IconError::Fetch(e.to_string()))?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("query", query);
            qp.append_pair("limit", &limit.unwrap_or(20).to_string());
        }
        let resp = self.http.get(url).send().await.map_err(|e| IconError::Fetch(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(IconError::Fetch(format!("HTTP {}", resp.status())));
        }
        let data: SearchResponse = resp.json().await.map_err(|e| IconError::Fetch(e.to_string()))?;
        Ok(data.icons)
    }
}

impl Default for IconifyClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn parse_id_rejects_missing_colon() {
        assert!(matches!(parse_id("mdihome"), Err(IconError::InvalidIdentifier(_))));
    }

    #[test]
    fn parse_id_rejects_extra_colon() {
        assert!(matches!(parse_id("mdi:home:extra"), Err(IconError::InvalidIdentifier(_))));
    }

    #[test]
    fn parse_id_rejects_invalid_characters() {
        assert!(matches!(parse_id("mdi:home/home"), Err(IconError::InvalidIdentifier(_))));
    }

    #[test]
    fn parse_id_accepts_prefix_name() {
        assert_eq!(parse_id("mdi:home").unwrap(), ("mdi".into(), "home".into()));
    }

    #[tokio::test]
    async fn fetch_body_parses_collection_response() {
        let server = MockServer::start().await;
        let json = serde_json::json!({
            "prefix": "mdi",
            "width": 24,
            "height": 24,
            "icons": { "home": { "body": "<path d=\"M0 0\"/>" } }
        });
        Mock::given(method("GET"))
            .and(path("/mdi.json"))
            .and(query_param("icons", "home"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json))
            .mount(&server)
            .await;

        let client = IconifyClient::with_base(server.uri());
        let body = client.fetch_body("mdi:home").await.unwrap();
        assert_eq!(body.body, "<path d=\"M0 0\"/>");
        assert_eq!(body.width, 24);
    }

    #[tokio::test]
    async fn fetch_body_missing_icon_is_not_found() {
        let server = MockServer::start().await;
        let json = serde_json::json!({ "prefix": "mdi", "icons": {} });
        Mock::given(method("GET"))
            .and(path("/mdi.json"))
            .and(query_param("icons", "ghost"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json))
            .mount(&server)
            .await;
        let client = IconifyClient::with_base(server.uri());
        assert!(matches!(client.fetch_body("mdi:ghost").await, Err(IconError::NotFound(_))));
    }

    #[tokio::test]
    async fn fetch_collections_lists_prefixes_titles_and_licenses() {
        let server = MockServer::start().await;
        let json = serde_json::json!({
            "mdi": { "name": "Material Design Icons", "license": { "title": "Apache License 2.0", "spdx": "Apache-2.0", "url": "https://github.com/Templarian/MaterialDesign/blob/master/LICENSE" } },
            "lucide": { "name": "Lucide" }
        });
        Mock::given(method("GET"))
            .and(path("/collections"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json))
            .mount(&server)
            .await;
        let client = IconifyClient::with_base(server.uri());
        let sets = client.fetch_collections().await.unwrap();
        assert!(sets.contains(&(
            "mdi".into(),
            "Material Design Icons".into(),
            Some(License { title: "Apache License 2.0".into(), spdx: "Apache-2.0".into(), url: Some("https://github.com/Templarian/MaterialDesign/blob/master/LICENSE".into()) })
        )));
        assert!(sets.contains(&("lucide".into(), "Lucide".into(), None)));
    }

    #[tokio::test]
    async fn search_icons_parses_response() {
        let server = MockServer::start().await;
        let json = serde_json::json!({
            "icons": ["mdi:home", "lucide:home"],
            "total": 2,
            "limit": 20,
        });
        Mock::given(method("GET"))
            .and(path("/search"))
            .and(query_param("query", "home"))
            .and(query_param("limit", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json))
            .mount(&server)
            .await;
        let client = IconifyClient::with_base(server.uri());
        let hits = client.search_icons("home", Some(10)).await.unwrap();
        assert_eq!(hits, vec!["mdi:home", "lucide:home"]);
    }
}
