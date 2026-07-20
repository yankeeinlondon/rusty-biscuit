use std::future::Future;

use biscuit_file::FetchPolicy;
use sniff::filesystem::git::{ResolvedRemote, resolve_remote_at};
use sniff::remote::FocusedProviderClient;

use super::ResolutionContext;
use crate::markdown::compose::expression::{ExpressionError, ProviderFailureKind};
use crate::markdown::compose::remote_fetch::{SharedExecutorError, block_on_shared_executor};

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
/// canonical provider URL. Async because a neutral-hostname self-managed
/// server is identified by Sniff's bounded discovery probe
/// ([`FocusedProviderClient::discover`]) before the client can exist; it runs
/// inside the [`run`] bridge on the shared executor.
pub(super) async fn connect(
    #[allow(unused_mut)] mut resolved: ResolvedRemote,
    policy: FetchPolicy,
) -> Result<FocusedProviderClient, sniff::SniffError> {
    #[cfg(test)]
    if let Some(transport) = test_transport::current() {
        resolved.api_flavor = transport.flavor;
        return FocusedProviderClient::with_api_base(resolved, policy, &transport.api_base);
    }
    FocusedProviderClient::discover(resolved, policy).await
}

/// Test-only injection point for the provider transport.
///
/// The production path can now address a loopback endpoint on its own:
/// `ResolvedRemote` retains the configured scheme/host/port and
/// `FocusedProviderClient::discover` identifies a neutral host through the
/// bounded version-endpoint probe. This override remains because the compose
/// fixtures assert exact provider request counts (single-flight/memoization),
/// and the discovery probe issues its own version requests per constructed
/// client, which would pollute those counts; pinning the flavor and API base
/// keeps every recorded request a query under test. Sniff's Wiremock suite
/// covers the discovery path itself against the production constructor.
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

/// Translates the authored `direction` (and, for `pr_list`, its `sort`
/// companion) into the internal newest-first flag.
///
/// `direction` is only meaningful relative to a sort key, so combining it with
/// `sort: "provider-default"` — which asks for the provider's own order
/// verbatim — is an invalid filter combination rather than a value to ignore
/// (D25 forbids silently ignoring an authored field).
///
/// ## Errors
///
/// Returns [`ExpressionError::Other`] for a direction outside
/// `ascending`/`descending` or for `direction` combined with
/// `sort: "provider-default"`.
pub(super) fn authored_direction(
    function: &str,
    direction: Option<&str>,
    sort: Option<&str>,
) -> Result<bool, ExpressionError> {
    let descending = match direction {
        None => true,
        Some("ascending") => false,
        Some("descending") => true,
        Some(_) => {
            return Err(ExpressionError::Other {
                function: function.to_string(),
                message: "direction must be ascending or descending".to_string(),
            });
        }
    };
    if direction.is_some() && sort == Some("provider-default") {
        return Err(ExpressionError::Other {
            function: function.to_string(),
            message: "direction cannot be combined with sort: \"provider-default\"; \
                      the provider's own order is preserved as returned"
                .to_string(),
        });
    }
    Ok(descending)
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

/// Converts a Sniff provider failure into the focused provider error variant.
///
/// Every provider-facing function funnels Sniff errors through here so the
/// [`ProviderFailureKind`] classification is attached in exactly one place.
/// The classification (not the message) is what makes the failure
/// authoring-fatal on all three expression surfaces; the message keeps
/// Sniff's actionable detail verbatim.
pub(super) fn provider_error(function: &str, error: sniff::SniffError) -> ExpressionError {
    ExpressionError::Provider {
        function: function.to_string(),
        kind: classify_provider_failure(&error),
        message: error.to_string(),
    }
}

/// Maps a [`sniff::SniffError`] onto the focused failure vocabulary.
///
/// The remote-provider variants carry the distinction natively; everything
/// else reaching this boundary (I/O, Git, client initialization, a non-404
/// HTTP status) is a transport-class failure: the provider could not produce
/// a usable answer and the cause is neither an authoring mistake nor a
/// capability gap.
fn classify_provider_failure(error: &sniff::SniffError) -> ProviderFailureKind {
    use sniff::SniffError as E;
    match error {
        E::RemotePolicyDenied { .. } => ProviderFailureKind::DeniedHost,
        E::MissingCredentials { .. }
        | E::InvalidCredentials { .. }
        | E::RemoteForbidden { .. } => ProviderFailureKind::Authentication,
        E::RateLimited { .. } => ProviderFailureKind::RateLimit,
        E::UnsupportedProvider { .. }
        | E::UnsupportedRemoteCapability { .. }
        | E::UnsupportedRemoteFilter { .. } => ProviderFailureKind::UnsupportedCapability,
        E::IncompleteRemoteDomain { .. } => ProviderFailureKind::IncompleteDomain,
        E::ShorthandNotFound { .. } => ProviderFailureKind::NotFound,
        E::RemoteApi { status: 404, .. } => ProviderFailureKind::NotFound,
        _ => ProviderFailureKind::Transport,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::thread::ThreadId;

    use super::*;
    use crate::markdown::compose::remote_fetch::executor_builds;

    fn message(error: &ExpressionError) -> String {
        match error {
            ExpressionError::Other { message, .. } | ExpressionError::Provider { message, .. } => {
                message.clone()
            }
            other => panic!("expected a message-carrying error, got {other:?}"),
        }
    }

    fn function_name(error: &ExpressionError) -> String {
        match error {
            ExpressionError::Other { function, .. } | ExpressionError::Provider { function, .. } => {
                function.clone()
            }
            other => panic!("expected a function-carrying error, got {other:?}"),
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

    /// Every Sniff remote-provider variant must land on its documented focused
    /// kind; the classification — not string matching on the message — is what
    /// downstream surfaces and tests key on.
    #[test]
    fn sniff_errors_classify_into_the_focused_vocabulary() {
        use sniff::SniffError as E;

        let cases: Vec<(E, ProviderFailureKind)> = vec![
            (
                E::RemotePolicyDenied { host: "git.example".to_string() },
                ProviderFailureKind::DeniedHost,
            ),
            (
                E::MissingCredentials {
                    provider: "GitHub".to_string(),
                    env_var: "GITHUB_TOKEN".to_string(),
                },
                ProviderFailureKind::Authentication,
            ),
            (
                E::InvalidCredentials {
                    provider: "GitHub".to_string(),
                    message: "bad token".to_string(),
                },
                ProviderFailureKind::Authentication,
            ),
            (
                E::RemoteForbidden {
                    provider: "GitLab".to_string(),
                    message: "denied".to_string(),
                },
                ProviderFailureKind::Authentication,
            ),
            (
                E::RateLimited { provider: "Gitea".to_string(), retry_after: None },
                ProviderFailureKind::RateLimit,
            ),
            (
                E::UnsupportedProvider { url: "https://sr.ht/x".to_string() },
                ProviderFailureKind::UnsupportedCapability,
            ),
            (
                E::UnsupportedRemoteCapability {
                    capability: "provider queries",
                    target: "SelfHosted".to_string(),
                },
                ProviderFailureKind::UnsupportedCapability,
            ),
            (
                E::UnsupportedRemoteFilter {
                    field: "assignee",
                    provider: "Gitea".to_string(),
                },
                ProviderFailureKind::UnsupportedCapability,
            ),
            (
                E::IncompleteRemoteDomain {
                    provider: "Gitea".to_string(),
                    bound: "parent executions",
                    limit: 20,
                },
                ProviderFailureKind::IncompleteDomain,
            ),
            (
                E::ShorthandNotFound {
                    owner: "a".to_string(),
                    repo: "b".to_string(),
                    providers_tried: "GitHub".to_string(),
                },
                ProviderFailureKind::NotFound,
            ),
            (
                E::RemoteApi {
                    provider: "GitHub".to_string(),
                    status: 404,
                    message: "missing".to_string(),
                },
                ProviderFailureKind::NotFound,
            ),
            (
                E::RemoteApi {
                    provider: "GitHub".to_string(),
                    status: 500,
                    message: "boom".to_string(),
                },
                ProviderFailureKind::Transport,
            ),
            (
                E::RemoteUnreachable {
                    url: "https://git.example".to_string(),
                    message: "connection refused".to_string(),
                },
                ProviderFailureKind::Transport,
            ),
            (
                E::RemoteInit {
                    provider: "runtime".to_string(),
                    message: "no executor".to_string(),
                },
                ProviderFailureKind::Transport,
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(
                classify_provider_failure(&error),
                expected,
                "{error} classified incorrectly"
            );
        }
    }

    /// The converted error is the typed provider variant (fatal on every
    /// surface), never the generic catch-all.
    #[test]
    fn provider_error_produces_the_typed_provider_variant() {
        let error = provider_error(
            "pr",
            sniff::SniffError::RateLimited {
                provider: "Gitea".to_string(),
                retry_after: Some(30),
            },
        );
        match error {
            ExpressionError::Provider { function, kind, message } => {
                assert_eq!(function, "pr");
                assert_eq!(kind, ProviderFailureKind::RateLimit);
                assert!(message.contains("rate limited"), "{message}");
                assert!(message.contains("30s"), "{message}");
            }
            other => panic!("expected ExpressionError::Provider, got {other:?}"),
        }
    }
}
