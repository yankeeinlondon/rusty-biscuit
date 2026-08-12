use std::process::ExitStatus;
use std::time::{Duration, Instant};

/// Cleanup type returned by the MCP injector after a session.
pub(crate) type McpCleanup = Option<(
    Box<dyn claudine::mcp::inject::McpInjector>,
    claudine::mcp::inject::InjectionResult,
)>;

/// Remove temp files written during MCP injection.
pub(crate) fn cleanup_mcp_injection(mcp_cleanup: McpCleanup) {
    if let Some((injector, injection_result)) = mcp_cleanup
        && let Err(e) = injector.cleanup(&injection_result)
    {
        tracing::warn!("MCP injector cleanup failed: {e}");
    }
}

/// Resolve first-response latency from three observed timestamps.
///
/// Preferred precedence: semantic stdout > raw stdout > stderr.
pub(crate) fn resolve_first_response(
    semantic: Option<Instant>,
    raw_stdout: Option<Instant>,
    stderr: Option<Instant>,
    spawned_at: Instant,
) -> Option<Duration> {
    semantic
        .or(raw_stdout)
        .or(stderr)
        .map(|t| t.saturating_duration_since(spawned_at))
}

pub(crate) fn exit_code_from_status(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }

    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_response_preference_semantic_over_raw_over_stderr() {
        // Offsets are synthesized rather than slept, so the three candidates stay
        // exactly 5/10/15ms apart no matter how the host scheduler behaves. A
        // sleep-based version asserted an upper bound of 15ms and failed on CI
        // whenever the first 5ms sleep overshot.
        let spawned_at = Instant::now();
        let semantic = Some(spawned_at + Duration::from_millis(5));
        let raw = Some(spawned_at + Duration::from_millis(10));
        let stderr = Some(spawned_at + Duration::from_millis(15));

        let resolved = resolve_first_response(semantic, raw, stderr, spawned_at);
        // Semantic wins even though it is the earliest.
        assert_eq!(resolved, Some(Duration::from_millis(5)));
    }

    #[test]
    fn first_response_falls_back_to_raw_when_no_semantic() {
        let spawned_at = Instant::now();
        std::thread::sleep(Duration::from_millis(5));
        let raw = Some(Instant::now());

        let resolved = resolve_first_response(None, raw, None, spawned_at);
        assert!(resolved.is_some());
        assert!(resolved.unwrap() >= Duration::from_millis(5));
    }

    #[test]
    fn first_response_falls_back_to_stderr_when_nothing_else() {
        let spawned_at = Instant::now();
        std::thread::sleep(Duration::from_millis(5));
        let stderr = Some(Instant::now());

        let resolved = resolve_first_response(None, None, stderr, spawned_at);
        assert!(resolved.is_some());
        assert!(resolved.unwrap() >= Duration::from_millis(5));
    }

    #[test]
    fn first_response_none_when_no_output_at_all() {
        let spawned_at = Instant::now();
        let resolved = resolve_first_response(None, None, None, spawned_at);
        assert!(resolved.is_none());
    }
}
