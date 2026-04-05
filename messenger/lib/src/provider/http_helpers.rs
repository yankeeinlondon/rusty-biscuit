use serde::de::DeserializeOwned;

use crate::error::MessengerError;
use crate::receipt::ProviderKind;

/// Handle common HTTP response patterns: rate limiting (429), server errors (5xx),
/// and JSON body parsing.
///
/// Provider-specific error handling (e.g., checking `ok` in Slack or `error` in WhatsApp)
/// should be done by the caller on the returned value.
pub async fn handle_http_response<T: DeserializeOwned>(
    response: reqwest::Response,
    provider: ProviderKind,
) -> Result<T, MessengerError> {
    let status = response.status();

    if status.as_u16() == 429 {
        let retry_after = response
            .headers()
            .get("Retry-After")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(|s| s * 1000);
        tracing::warn!(
            provider = %provider,
            retry_after_ms = ?retry_after,
            "rate limited"
        );
        return Err(MessengerError::RateLimited {
            provider,
            retry_after_ms: retry_after,
        });
    }

    if status.is_server_error() {
        tracing::warn!(provider = %provider, status = %status, "server error");
        return Err(provider.transport_error(format!("server error: {status}")));
    }

    response
        .json()
        .await
        .map_err(|e| provider.transport_error(e))
}
