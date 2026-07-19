use std::future::Future;

use biscuit_file::FetchPolicy;
use sniff::filesystem::git::{ResolvedRemote, resolve_remote_at};
use sniff::remote::FocusedProviderClient;

use super::ResolutionContext;
use crate::markdown::compose::expression::ExpressionError;
use crate::markdown::compose::remote_fetch::{SharedExecutorError, block_on_shared_executor};

pub(super) fn client(
    context: &ResolutionContext,
    remote: Option<&str>,
    function: &str,
) -> Result<FocusedProviderClient, ExpressionError> {
    let resolved = resolve(context, remote, function)?;
    build_client(resolved, context.remote_policy(), function)
}

/// Resolves the caller repository's remote for a provider call.
///
/// Shared with `cicd`, which needs the resolved identity to build a job
/// reference before it builds a client.
pub(super) fn resolve(
    context: &ResolutionContext,
    remote: Option<&str>,
    function: &str,
) -> Result<ResolvedRemote, ExpressionError> {
    #[allow(unused_mut)]
    let mut resolved = resolve_remote_at(context.caller_dir(), remote)
        .map_err(|error| provider_error(function, error))?
        .ok_or_else(|| ExpressionError::Other {
            function: function.to_string(),
            message: "the caller repository has no usable configured remote".to_string(),
        })?;
    #[cfg(test)]
    if let Some(transport) = test_transport::current() {
        resolved.api_flavor = transport.flavor;
    }
    Ok(resolved)
}

/// Constructs the focused client for an already-resolved remote.
///
/// Every provider function funnels through here so the client is built one way
/// regardless of whether the remote came from repository resolution or from a
/// canonical provider URL.
pub(super) fn build_client(
    #[allow(unused_mut)] mut resolved: ResolvedRemote,
    policy: FetchPolicy,
    function: &str,
) -> Result<FocusedProviderClient, ExpressionError> {
    #[cfg(test)]
    if let Some(transport) = test_transport::current() {
        resolved.api_flavor = transport.flavor;
        return FocusedProviderClient::with_api_base(resolved, policy, &transport.api_base)
            .map_err(|error| provider_error(function, error));
    }
    FocusedProviderClient::new(resolved, policy)
        .map_err(|error| provider_error(function, error))
}

/// Test-only injection point for the provider transport.
///
/// Two production facts make a loopback mock server unaddressable through any
/// real constructor, and both are properties of Sniff rather than of the code
/// under test here:
///
/// - `FocusedProviderClient::new` builds `https://{host}/api/...` from
///   `ResolvedRemote::host`, which is stored without a port, so neither the
///   scheme nor the port of a `127.0.0.1:<port>` server can be expressed; and
/// - `ApiFlavor` is derived purely from host-name patterns, so a numeric
///   loopback host is always `SelfHosted`/`Unknown` and no provider flavor can
///   be selected for it.
///
/// The endpoint-host check inside the client additionally requires the API host
/// to equal the remote host, so the mock cannot be reached under a friendly
/// hostname either. Overriding both fields together is therefore the narrowest
/// way to exercise the compose → client → HTTP → formatter path; flavor
/// detection and API-base derivation keep their own unit coverage in Sniff.
///
/// Neither this module nor the branch that reads it exists in a non-test build,
/// so this adds no production surface and no environment variable. The cell is
/// process-wide rather than thread-local because compose evaluates its surfaces
/// on whichever thread the caller supplies, so tests that install an override
/// must be `#[serial_test::serial(provider_transport)]`.
#[cfg(test)]
pub(in crate::markdown::compose) mod test_transport {
    use std::sync::{Mutex, OnceLock};

    use sniff::filesystem::git::ApiFlavor;

    #[derive(Clone)]
    pub(in crate::markdown::compose) struct Transport {
        pub(in crate::markdown::compose) api_base: String,
        pub(in crate::markdown::compose) flavor: ApiFlavor,
    }

    fn cell() -> &'static Mutex<Option<Transport>> {
        static CELL: OnceLock<Mutex<Option<Transport>>> = OnceLock::new();
        CELL.get_or_init(|| Mutex::new(None))
    }

    pub(in crate::markdown::compose) fn current() -> Option<Transport> {
        cell().lock().expect("provider transport override lock").clone()
    }

    /// Installs an override until the returned guard drops.
    pub(in crate::markdown::compose) fn install(
        api_base: impl Into<String>,
        flavor: ApiFlavor,
    ) -> Guard {
        *cell().lock().expect("provider transport override lock") =
            Some(Transport { api_base: api_base.into(), flavor });
        Guard
    }

    pub(in crate::markdown::compose) struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            if let Ok(mut slot) = cell().lock() {
                *slot = None;
            }
        }
    }
}

/// Reads the authored `remote` key out of a canonical list query.
///
/// The canonical vocabulary admits an exact configured remote name or nothing
/// at all, so a non-string or blank value is an authoring error rather than a
/// request for the preferred remote — silently coercing it would send the query
/// to a repository the author never named.
///
/// ## Errors
///
/// Returns [`ExpressionError::Other`] naming `remote` for any present value
/// that is not a non-blank string.
pub(super) fn authored_remote(
    function: &str,
    value: Option<serde_json::Value>,
) -> Result<Option<String>, ExpressionError> {
    match value {
        None => Ok(None),
        Some(serde_json::Value::String(name)) if !name.trim().is_empty() => Ok(Some(name)),
        Some(_) => Err(ExpressionError::Other {
            function: function.to_string(),
            message: "remote must be a non-empty string naming a configured remote".to_string(),
        }),
    }
}

/// Bridges one asynchronous provider query into the synchronous expression
/// evaluator.
///
/// The future runs on the compose run's shared executor rather than on a
/// runtime built for this call; see
/// [`block_on_shared_executor`](crate::markdown::compose::remote_fetch::block_on_shared_executor)
/// for why blocking here is safe inside a caller's own Tokio runtime.
pub(super) fn run<T, Fut>(function: &'static str, future: Fut) -> Result<T, ExpressionError>
where
    T: Send + 'static,
    Fut: Future<Output = Result<T, sniff::SniffError>> + Send + 'static,
{
    match block_on_shared_executor(future) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(provider_error(function, error)),
        Err(SharedExecutorError::Panicked) => Err(ExpressionError::Other {
            function: function.to_string(),
            message: "provider query worker panicked".to_string(),
        }),
        Err(SharedExecutorError::Unavailable) => Err(provider_error(
            function,
            sniff::SniffError::RemoteInit {
                provider: "runtime".to_string(),
                message: "no provider executor is available".to_string(),
            },
        )),
    }
}

fn provider_error(function: &str, error: sniff::SniffError) -> ExpressionError {
    ExpressionError::Other { function: function.to_string(), message: error.to_string() }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::thread::ThreadId;

    use super::*;
    use crate::markdown::compose::remote_fetch::executor_builds;

    fn message(error: &ExpressionError) -> String {
        match error {
            ExpressionError::Other { message, .. } => message.clone(),
            other => panic!("expected ExpressionError::Other, got {other:?}"),
        }
    }

    fn function_name(error: &ExpressionError) -> String {
        match error {
            ExpressionError::Other { function, .. } => function.clone(),
            other => panic!("expected ExpressionError::Other, got {other:?}"),
        }
    }

    /// The executor must be shared, not per call. A runtime built per query
    /// would put each of these futures on its own freshly spawned thread, so
    /// the distinct-thread count discriminates the two designs directly.
    #[test]
    fn provider_runs_share_one_executor() {
        const QUERIES: usize = 12;

        // Warm up so the one-time lazy build is not counted against the loop.
        run("probe", async { Ok(()) }).expect("warm-up succeeds");

        let builds_before = executor_builds();
        let mut threads: HashSet<ThreadId> = HashSet::new();
        for _ in 0..QUERIES {
            let observed: ThreadId =
                run("probe", async { Ok(std::thread::current().id()) }).expect("query succeeds");
            threads.insert(observed);
        }

        assert!(
            threads.len() < QUERIES,
            "{QUERIES} queries landed on {} distinct threads; a shared executor \
             has a fixed worker pool, so this indicates a runtime per query",
            threads.len()
        );
        assert_eq!(
            executor_builds() - builds_before,
            0,
            "no additional executor may be constructed once one already exists"
        );
    }

    /// The thread-per-query bridge existed to keep `block_on` off a caller's
    /// live runtime. Spawning onto a foreign executor must preserve that.
    #[tokio::test]
    async fn provider_run_works_inside_an_active_runtime() {
        let value = run("probe", async { Ok(7_u32) }).expect("query succeeds inside a runtime");
        assert_eq!(value, 7);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn provider_run_works_inside_a_multi_thread_runtime() {
        let value = run("probe", async { Ok(7_u32) }).expect("query succeeds inside a runtime");
        assert_eq!(value, 7);
    }

    #[test]
    fn sniff_errors_surface_as_focused_expression_errors() {
        let error = run::<(), _>("pr", async {
            Err(sniff::SniffError::RemoteInit {
                provider: "github".to_string(),
                message: "token rejected".to_string(),
            })
        })
        .expect_err("provider failure must not be swallowed");

        assert_eq!(function_name(&error), "pr");
        assert!(
            message(&error).contains("token rejected"),
            "focused provider detail was lost: {}",
            message(&error)
        );
    }

    #[test]
    fn panicking_query_becomes_an_error_rather_than_unwinding() {
        let error = run::<(), _>("cicd", async { panic!("provider exploded") })
            .expect_err("a panicking query must not unwind into the compose run");

        assert_eq!(function_name(&error), "cicd");
        assert_eq!(message(&error), "provider query worker panicked");
    }
}
