/// The `ApiAuthMethod` expresses
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum ApiAuthMethod {
    BearerToken,
    ApiKey(String),
    /// API key passed as query parameter (e.g., Gemini uses `?key=API_KEY`)
    QueryParam(String),
    None,
}
