use std::future::Future;

use sniff::filesystem::git::resolve_remote_at;
use sniff::remote::FocusedProviderClient;

use super::ResolutionContext;
use crate::markdown::compose::expression::ExpressionError;

pub(super) fn client(
    context: &ResolutionContext,
    remote: Option<&str>,
    function: &str,
) -> Result<FocusedProviderClient, ExpressionError> {
    let resolved = resolve_remote_at(context.caller_dir(), remote)
        .map_err(|error| provider_error(function, error))?
        .ok_or_else(|| ExpressionError::Other {
            function: function.to_string(),
            message: "the caller repository has no usable configured remote".to_string(),
        })?;
    FocusedProviderClient::new(resolved, context.remote_policy())
        .map_err(|error| provider_error(function, error))
}

pub(super) fn run<T, Fut>(function: &'static str, future: Fut) -> Result<T, ExpressionError>
where
    T: Send + 'static,
    Fut: Future<Output = Result<T, sniff::SniffError>> + Send + 'static,
{
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| sniff::SniffError::RemoteInit {
                provider: "runtime".to_string(),
                message: error.to_string(),
            })?
            .block_on(future)
    })
    .join()
    .map_err(|_| ExpressionError::Other {
        function: function.to_string(),
        message: "provider query worker panicked".to_string(),
    })?
    .map_err(|error| provider_error(function, error))
}

fn provider_error(function: &str, error: sniff::SniffError) -> ExpressionError {
    ExpressionError::Other { function: function.to_string(), message: error.to_string() }
}
