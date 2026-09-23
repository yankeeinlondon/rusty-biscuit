//! ICMP echo reachability probes.
//!
//! [`ping`] sends sequential echo requests to one IPv4 or scoped IPv6 literal
//! without DNS, privilege escalation, or a subprocess. Each attempt has its own
//! monotonic deadline; a reply at or after that deadline counts as no reply.
//! "No reply" is an outcome, never an error. An error means the host could not
//! send at all (not permitted, unknown scope, a local API failure) and aborts
//! the whole series, even after earlier replies.
//!
//! | Platform | Transport |
//! |---|---|
//! | macOS, Linux, WSL2 | unprivileged datagram ICMP socket; Linux requires the process group inside `net.ipv4.ping_group_range` |
//! | Windows | `IcmpSendEcho2` / `Icmp6SendEcho2` |
//!
//! [`EchoProbe`] is the seam for deterministic tests: [`ping_with`] applies
//! the same budget, deadline, and abort rules to any probe.

use std::time::{Duration, Instant};

use super::ScopedIpAddr;

#[cfg(any(unix, test))]
mod packet;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

/// Largest per-attempt timeout.
pub const MAX_TIMEOUT: Duration = Duration::from_secs(60);

/// Largest number of attempts in one series.
pub const MAX_ATTEMPTS: u32 = 100;

/// Allowance on top of [`ProbeBudget::total`] for opening and closing the
/// transport. A series that completes normally returns within
/// [`ProbeBudget::deadline_bound`].
pub const SETUP_ALLOWANCE: Duration = Duration::from_millis(500);

/// Why an ICMP probe could not run.
#[derive(Debug, thiserror::Error)]
pub enum IcmpError {
    /// The timeout or attempt count is outside the supported range.
    #[error("invalid ICMP budget: {0}")]
    InvalidBudget(String),
    /// The operating system refused to open or use an ICMP endpoint.
    #[error("ICMP echo is not permitted on this host: {detail}")]
    NotPermitted {
        /// What was refused and, where known, how to allow it.
        detail: String,
    },
    /// The zone of a scoped IPv6 target names no interface on this host.
    #[error("`{target}` names an unknown interface `{scope}`")]
    UnknownScope {
        /// The scoped target.
        target: String,
        /// The unresolved zone.
        scope: String,
    },
    /// The echo request could not be sent or its reply could not be read.
    #[error("cannot send ICMP echo to {target}: {source}")]
    Send {
        /// The target.
        target: String,
        /// The operating-system error.
        #[source]
        source: std::io::Error,
    },
    /// This platform has no ICMP transport.
    #[error("ICMP echo is not supported on this platform")]
    Unsupported,
}

/// A validated timeout and attempt count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeBudget {
    per_attempt: Duration,
    attempts: u32,
}

impl ProbeBudget {
    /// A budget of `attempts` sequential attempts, each waiting up to
    /// `per_attempt`.
    ///
    /// ## Errors
    ///
    /// [`IcmpError::InvalidBudget`] unless `per_attempt` is positive and at
    /// most [`MAX_TIMEOUT`] and `attempts` is `1..=`[`MAX_ATTEMPTS`].
    pub fn new(per_attempt: Duration, attempts: u32) -> Result<Self, IcmpError> {
        if per_attempt.is_zero() || per_attempt > MAX_TIMEOUT {
            return Err(IcmpError::InvalidBudget(format!(
                "timeout must be greater than 0 and at most {} ms, got {} ms",
                MAX_TIMEOUT.as_millis(),
                per_attempt.as_secs_f64() * 1000.0
            )));
        }
        if attempts == 0 || attempts > MAX_ATTEMPTS {
            return Err(IcmpError::InvalidBudget(format!(
                "attempts must be between 1 and {MAX_ATTEMPTS}, got {attempts}"
            )));
        }
        Ok(Self {
            per_attempt,
            attempts,
        })
    }

    /// [`ProbeBudget::new`] from caller-supplied numbers: a millisecond timeout,
    /// which may be fractional, and a whole attempt count.
    ///
    /// ## Errors
    ///
    /// [`IcmpError::InvalidBudget`] for a non-finite, non-positive, or
    /// out-of-range value, or a fractional attempt count. Values are range
    /// checked before conversion, so nothing is truncated.
    pub fn from_millis(per_attempt_ms: f64, attempts: f64) -> Result<Self, IcmpError> {
        let max_ms = MAX_TIMEOUT.as_secs_f64() * 1000.0;
        if !per_attempt_ms.is_finite() || per_attempt_ms <= 0.0 || per_attempt_ms > max_ms {
            return Err(IcmpError::InvalidBudget(format!(
                "timeout must be a finite number of milliseconds greater than 0 and at most {max_ms}, got {per_attempt_ms}"
            )));
        }
        if !attempts.is_finite()
            || attempts.fract() != 0.0
            || attempts < 1.0
            || attempts > f64::from(MAX_ATTEMPTS)
        {
            return Err(IcmpError::InvalidBudget(format!(
                "attempts must be a whole number between 1 and {MAX_ATTEMPTS}, got {attempts}"
            )));
        }
        let per_attempt = Duration::try_from_secs_f64(per_attempt_ms / 1000.0)
            .map_err(|error| IcmpError::InvalidBudget(error.to_string()))?;
        // Range checked above: the cast cannot truncate.
        Self::new(per_attempt, attempts as u32)
    }

    #[allow(missing_docs)]
    pub fn per_attempt(&self) -> Duration {
        self.per_attempt
    }

    #[allow(missing_docs)]
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Every attempt's timeout combined.
    pub fn total(&self) -> Duration {
        self.per_attempt * self.attempts
    }

    /// [`ProbeBudget::total`] plus [`SETUP_ALLOWANCE`].
    pub fn deadline_bound(&self) -> Duration {
        self.total() + SETUP_ALLOWANCE
    }
}

/// The result of one echo attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoOutcome {
    /// A reply arrived before the attempt's deadline.
    Reply {
        /// Time from starting the attempt to receiving the reply.
        round_trip: Duration,
    },
    /// No reply before the deadline, including an unreachable target.
    NoReply,
}

/// One echo transport.
pub trait EchoProbe {
    /// Sends one echo request to `target` and waits at most `timeout` for the
    /// matching reply.
    ///
    /// ## Errors
    ///
    /// An [`IcmpError`] when the request cannot be sent. A missing reply is
    /// [`EchoOutcome::NoReply`], not an error.
    fn echo(&mut self, target: &ScopedIpAddr, timeout: Duration) -> Result<EchoOutcome, IcmpError>;
}

/// Every attempt's outcome, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingReport {
    outcomes: Vec<EchoOutcome>,
}

/// How a series of attempts went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PingVerdict {
    /// Every attempt replied in time.
    AllReplied,
    /// No attempt replied in time.
    NoneReplied,
    /// Some attempts replied in time and some did not.
    Unstable,
}

impl PingReport {
    #[allow(missing_docs)]
    pub fn outcomes(&self) -> &[EchoOutcome] {
        &self.outcomes
    }

    #[allow(missing_docs)]
    pub fn verdict(&self) -> PingVerdict {
        let replied = self
            .outcomes
            .iter()
            .filter(|outcome| matches!(outcome, EchoOutcome::Reply { .. }))
            .count();
        match replied {
            0 => PingVerdict::NoneReplied,
            n if n == self.outcomes.len() => PingVerdict::AllReplied,
            _ => PingVerdict::Unstable,
        }
    }
}

/// Probes `target` with the host's ICMP transport.
///
/// ## Errors
///
/// See [`ping_with`].
pub fn ping(target: &ScopedIpAddr, budget: ProbeBudget) -> Result<PingReport, IcmpError> {
    let mut probe = SystemProbe::new();
    ping_with(&mut probe, target, budget)
}

/// Runs `budget.attempts()` sequential attempts through `probe`.
///
/// An attempt counts as a reply only when both the probe's reported round trip
/// and the elapsed monotonic time are below the per-attempt timeout.
///
/// ## Errors
///
/// The first [`IcmpError`] from the probe, which stops the series.
pub fn ping_with(
    probe: &mut dyn EchoProbe,
    target: &ScopedIpAddr,
    budget: ProbeBudget,
) -> Result<PingReport, IcmpError> {
    let per_attempt = budget.per_attempt();
    let mut outcomes = Vec::with_capacity(budget.attempts() as usize);
    for _ in 0..budget.attempts() {
        let started = Instant::now();
        let outcome = probe.echo(target, per_attempt)?;
        let elapsed = started.elapsed();
        outcomes.push(match outcome {
            EchoOutcome::Reply { round_trip }
                if round_trip < per_attempt && elapsed < per_attempt =>
            {
                EchoOutcome::Reply { round_trip }
            }
            _ => EchoOutcome::NoReply,
        });
    }
    Ok(PingReport { outcomes })
}

/// The host's ICMP transport. Endpoints open on first use and close on drop,
/// so a series pays setup once per address family.
pub struct SystemProbe {
    #[cfg(unix)]
    inner: unix::UnixProbe,
    #[cfg(windows)]
    inner: windows::WindowsProbe,
}

impl SystemProbe {
    #[allow(missing_docs, clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            #[cfg(unix)]
            inner: unix::UnixProbe::default(),
            #[cfg(windows)]
            inner: windows::WindowsProbe::default(),
        }
    }
}

impl EchoProbe for SystemProbe {
    fn echo(&mut self, target: &ScopedIpAddr, timeout: Duration) -> Result<EchoOutcome, IcmpError> {
        #[cfg(unix)]
        {
            self.inner.echo(target, timeout)
        }
        #[cfg(windows)]
        {
            self.inner.echo(target, timeout)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (target, timeout);
            Err(IcmpError::Unsupported)
        }
    }
}

/// Resolves a zone to an interface index: a number is used as-is, a name is
/// looked up. An unscoped target is index 0.
#[cfg(any(unix, windows))]
fn scope_index(target: &ScopedIpAddr) -> Result<u32, IcmpError> {
    let Some(scope) = target.scope() else {
        return Ok(0);
    };
    if let Ok(index) = scope.parse::<u32>() {
        return Ok(index);
    }
    getifaddrs::if_nametoindex(scope)
        .ok()
        .filter(|index| *index != 0)
        .ok_or_else(|| IcmpError::UnknownScope {
            target: target.to_string(),
            scope: scope.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    const MS: Duration = Duration::from_millis(1);

    /// Replays scripted outcomes and counts calls.
    struct Scripted {
        script: VecDeque<Result<EchoOutcome, IcmpError>>,
        calls: usize,
    }

    impl Scripted {
        fn new(script: Vec<Result<EchoOutcome, IcmpError>>) -> Self {
            Self {
                script: script.into(),
                calls: 0,
            }
        }
    }

    impl EchoProbe for Scripted {
        fn echo(&mut self, _: &ScopedIpAddr, _: Duration) -> Result<EchoOutcome, IcmpError> {
            self.calls += 1;
            self.script.pop_front().expect("probe called more often than scripted")
        }
    }

    fn reply(ms: u64) -> Result<EchoOutcome, IcmpError> {
        Ok(EchoOutcome::Reply {
            round_trip: Duration::from_millis(ms),
        })
    }

    fn target() -> ScopedIpAddr {
        "192.0.2.1".parse().unwrap()
    }

    fn run(script: Vec<Result<EchoOutcome, IcmpError>>, per_attempt_ms: u64) -> (Result<PingReport, IcmpError>, usize) {
        let attempts = script.len() as u32;
        let mut probe = Scripted::new(script);
        let budget = ProbeBudget::new(Duration::from_millis(per_attempt_ms), attempts).unwrap();
        let result = ping_with(&mut probe, &target(), budget);
        (result, probe.calls)
    }

    #[test]
    fn all_fast_attempts_are_all_replied() {
        let (report, calls) = run(vec![reply(1), reply(5), reply(9)], 10);
        let report = report.unwrap();
        assert_eq!(calls, 3);
        assert_eq!(report.verdict(), PingVerdict::AllReplied);
        assert_eq!(
            report.outcomes()[1],
            EchoOutcome::Reply {
                round_trip: 5 * MS
            }
        );
    }

    #[test]
    fn all_slow_attempts_are_none_replied() {
        let (report, _) = run(vec![reply(50), Ok(EchoOutcome::NoReply), reply(11)], 10);
        let report = report.unwrap();
        assert_eq!(report.verdict(), PingVerdict::NoneReplied);
        assert!(report.outcomes().iter().all(|o| *o == EchoOutcome::NoReply));
    }

    #[test]
    fn mixed_attempts_are_unstable() {
        let (report, _) = run(vec![reply(2), reply(80), Ok(EchoOutcome::NoReply)], 10);
        assert_eq!(report.unwrap().verdict(), PingVerdict::Unstable);
    }

    #[test]
    fn a_reply_exactly_at_the_threshold_is_late() {
        let (report, _) = run(vec![reply(10)], 10);
        let report = report.unwrap();
        assert_eq!(report.outcomes(), &[EchoOutcome::NoReply]);
        assert_eq!(report.verdict(), PingVerdict::NoneReplied);
    }

    #[test]
    fn a_probe_that_overruns_the_deadline_is_late_whatever_it_reports() {
        struct Dawdler;
        impl EchoProbe for Dawdler {
            fn echo(&mut self, _: &ScopedIpAddr, timeout: Duration) -> Result<EchoOutcome, IcmpError> {
                std::thread::sleep(timeout + 5 * MS);
                Ok(EchoOutcome::Reply { round_trip: MS })
            }
        }
        let budget = ProbeBudget::new(20 * MS, 2).unwrap();
        let report = ping_with(&mut Dawdler, &target(), budget).unwrap();
        assert_eq!(report.verdict(), PingVerdict::NoneReplied);
    }

    #[test]
    fn a_send_failure_after_an_earlier_success_aborts_the_series() {
        let failure = Err(IcmpError::Send {
            target: "192.0.2.1".to_string(),
            source: std::io::Error::from(std::io::ErrorKind::AddrNotAvailable),
        });
        let mut probe = Scripted::new(vec![reply(1), failure, reply(1)]);
        let budget = ProbeBudget::new(10 * MS, 3).unwrap();
        let error = ping_with(&mut probe, &target(), budget).unwrap_err();
        assert!(matches!(error, IcmpError::Send { .. }), "{error:?}");
        assert_eq!(probe.calls, 2, "no attempt runs after the failure");
    }

    #[test]
    fn a_single_attempt_timeout_is_an_outcome_not_an_error() {
        let (report, _) = run(vec![Ok(EchoOutcome::NoReply)], 100);
        assert_eq!(report.unwrap().verdict(), PingVerdict::NoneReplied);
    }

    #[test]
    fn a_series_of_timeouts_stays_within_the_deadline_bound() {
        struct Timeout;
        impl EchoProbe for Timeout {
            fn echo(&mut self, _: &ScopedIpAddr, timeout: Duration) -> Result<EchoOutcome, IcmpError> {
                std::thread::sleep(timeout);
                Ok(EchoOutcome::NoReply)
            }
        }
        let budget = ProbeBudget::new(15 * MS, 3).unwrap();
        let started = Instant::now();
        ping_with(&mut Timeout, &target(), budget).unwrap();
        let elapsed = started.elapsed();
        assert!(elapsed >= budget.total(), "{elapsed:?}");
        assert!(elapsed < budget.deadline_bound(), "{elapsed:?}");
    }

    #[test]
    fn budget_accepts_the_boundaries() {
        let budget = ProbeBudget::from_millis(0.5, 1.0).unwrap();
        assert_eq!(budget.per_attempt(), Duration::from_micros(500));
        let budget = ProbeBudget::from_millis(60_000.0, 100.0).unwrap();
        assert_eq!(budget.per_attempt(), MAX_TIMEOUT);
        assert_eq!(budget.attempts(), MAX_ATTEMPTS);
        assert_eq!(budget.total(), Duration::from_secs(6000));
        assert_eq!(
            ProbeBudget::from_millis(100.0, 3.0).unwrap().deadline_bound(),
            Duration::from_millis(300) + SETUP_ALLOWANCE
        );
    }

    #[test]
    fn budget_rejects_invalid_and_overflowing_numbers_without_truncation() {
        let invalid = [
            (f64::NAN, 1.0),
            (f64::INFINITY, 1.0),
            (f64::NEG_INFINITY, 1.0),
            (0.0, 1.0),
            (-0.0, 1.0),
            (-5.0, 1.0),
            (60_000.001, 1.0),
            (1e300, 1.0),
            (100.0, 0.0),
            (100.0, -1.0),
            (100.0, 2.5),
            (100.0, 0.999),
            (100.0, 101.0),
            (100.0, f64::NAN),
            (100.0, f64::INFINITY),
            // 2^32 + 1 would truncate to 1 as u32.
            (100.0, 4_294_967_297.0),
            (100.0, 1e20),
        ];
        for (ms, attempts) in invalid {
            assert!(
                matches!(
                    ProbeBudget::from_millis(ms, attempts),
                    Err(IcmpError::InvalidBudget(_))
                ),
                "({ms}, {attempts})"
            );
        }
        assert!(matches!(
            ProbeBudget::new(Duration::ZERO, 1),
            Err(IcmpError::InvalidBudget(_))
        ));
        assert!(matches!(
            ProbeBudget::new(MAX_TIMEOUT + Duration::from_nanos(1), 1),
            Err(IcmpError::InvalidBudget(_))
        ));
        assert!(matches!(
            ProbeBudget::new(MS, u32::MAX),
            Err(IcmpError::InvalidBudget(_))
        ));
    }

    #[test]
    fn numeric_zones_resolve_without_a_lookup_and_unknown_names_fail() {
        let numeric: ScopedIpAddr = "fe80::1%7".parse().unwrap();
        assert_eq!(scope_index(&numeric).unwrap(), 7);
        assert_eq!(scope_index(&"::1".parse().unwrap()).unwrap(), 0);

        let unknown: ScopedIpAddr = "fe80::1%no-such-interface-9".parse().unwrap();
        match scope_index(&unknown) {
            Err(IcmpError::UnknownScope { target, scope }) => {
                assert_eq!(target, "fe80::1%no-such-interface-9");
                assert_eq!(scope, "no-such-interface-9");
            }
            other => panic!("expected UnknownScope, got {other:?}"),
        }
    }
}
