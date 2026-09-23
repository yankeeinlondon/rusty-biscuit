//! The request's ICMP effect authority behind `ping` and `ping_under` (R6).
//!
//! ICMP is a separate grant from HTTP fetch permission even though both are
//! spelled with `--allow-host`. An entry that parses as an IP literal or a
//! strict CIDR is an ICMP grant; a hostname or wildcard never authorizes an
//! address indirectly, and a CIDR never widens
//! [`FetchPolicy`](biscuit_file::FetchPolicy) (see
//! [`is_icmp_only_entry`], which keeps CIDR entries out of the HTTP allowlist).
//!
//! The authority is request-owned state carried on `ComposeOptions` and
//! projected into every `ResolutionContext` the request builds, so a nested
//! `as_markdown` child inherits the root's consent rather than acquiring its
//! own. It has three modes:
//!
//! - **execute** — check the grant, then send through the host transport or an
//!   injected one;
//! - **discovery** — record the planned probe for preflight and send nothing,
//!   which is how `ping` reaches [`ComposePreflightReport`] without a packet;
//! - **denied** — the default. No grants, so every target is refused with one
//!   warning and no packet.
//!
//! [`ComposePreflightReport`]: super::preflight::ComposePreflightReport

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use ipnet::IpNet;
use sniff::network::ScopedIpAddr;
use sniff::network::icmp::{self, EchoProbe, PingVerdict, ProbeBudget};

use super::ComposeWarning;
use super::expression::ExpressionError;

/// The warning stage ICMP denials are reported under.
const STAGE: &str = "icmp";

/// One `--allow-host` entry that named an ICMP capability.
#[derive(Debug, Clone, PartialEq, Eq)]
enum IcmpGrant {
    /// An exact address. A grant that spelled a zone (`fe80::1%en0`) permits
    /// only that zone; an unscoped grant permits any.
    Exact(ScopedIpAddr),
    /// Every address whose bits fall inside this network, of that family only.
    Network(IpNet),
}

impl IcmpGrant {
    /// The grant an `--allow-host` entry names, or `None` for a hostname or
    /// wildcard pattern.
    fn parse(entry: &str) -> Option<Self> {
        if let Ok(network) = entry.parse::<IpNet>() {
            return Some(Self::Network(network));
        }
        entry.parse::<ScopedIpAddr>().ok().map(Self::Exact)
    }

    fn permits(&self, target: &ScopedIpAddr) -> bool {
        match self {
            Self::Exact(granted) => {
                granted.address() == target.address()
                    && (granted.scope().is_none() || granted.scope() == target.scope())
            }
            Self::Network(network) => target.is_within(network),
        }
    }
}

/// Whether `entry` is an ICMP-only grant, which the HTTP allowlist must drop.
///
/// A bare IP literal keeps its long-standing meaning as an exact HTTP host and
/// is therefore *not* ICMP-only; only a CIDR is.
pub(crate) fn is_icmp_only_entry(entry: &str) -> bool {
    entry.parse::<IpNet>().is_ok()
}

/// One ICMP echo series a document could run, discovered without sending a
/// packet.
///
/// Preflight reports these as typed records rather than command strings: the
/// orchestrator approves a target, timeout, and attempt count, and the runtime
/// revalidates the address it actually resolved against the grant before
/// sending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedIcmpProbe {
    /// The expression function that would send it, `ping` or `ping_under`.
    pub function: String,
    /// The validated literal target, scope included.
    pub target: ScopedIpAddr,
    /// Per-attempt timeout.
    pub timeout: Duration,
    /// Number of sequential attempts.
    pub attempts: u32,
    /// Whether the request's grants already permit this target. A planned probe
    /// that is not granted is the one an approver must act on.
    pub granted: bool,
}

/// One echo transport shared by every call in a request.
type SharedProbe = Arc<Mutex<dyn EchoProbe + Send>>;

/// The request's ICMP authority: its grants, its transport, and where denials
/// and planned probes are recorded.
///
/// The default authority grants nothing and owns no transport, so a
/// `ResolutionContext` that was never given one cannot reach the network.
#[derive(Clone, Default)]
pub(crate) struct IcmpAuthority {
    grants: Arc<Vec<IcmpGrant>>,
    /// `None` uses the host transport.
    transport: Option<SharedProbe>,
    /// `Some` suppresses sending and records what would have been sent.
    discovered: Option<Arc<Mutex<Vec<PlannedIcmpProbe>>>>,
    warnings: Arc<Mutex<Vec<ComposeWarning>>>,
}

impl std::fmt::Debug for IcmpAuthority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IcmpAuthority")
            .field("grants", &self.grants)
            .field("transport", &self.transport.as_ref().map(|_| "injected"))
            .field("discovery", &self.is_discovery())
            .finish()
    }
}

impl IcmpAuthority {
    /// This authority's grants replaced by the ones `entries` names, sharing
    /// the same warning sink, transport, and discovery mode.
    pub(crate) fn with_grants<S: AsRef<str>>(&self, entries: &[S]) -> Self {
        Self {
            grants: Arc::new(
                entries
                    .iter()
                    .filter_map(|entry| IcmpGrant::parse(entry.as_ref()))
                    .collect(),
            ),
            ..self.clone()
        }
    }

    /// A discovery sink: every call records its plan and sends nothing.
    pub(crate) fn discovering(&self) -> Self {
        Self {
            discovered: Some(Arc::default()),
            ..self.clone()
        }
    }

    /// Installs a deterministic transport in place of the host's.
    #[cfg(test)]
    pub(crate) fn with_transport(&self, probe: impl EchoProbe + Send + 'static) -> Self {
        Self {
            transport: Some(Arc::new(Mutex::new(probe))),
            ..self.clone()
        }
    }

    pub(crate) fn is_discovery(&self) -> bool {
        self.discovered.is_some()
    }

    pub(crate) fn has_injected_transport(&self) -> bool {
        self.transport.is_some()
    }

    /// Drains the probes recorded in discovery mode.
    pub(crate) fn take_discovered(&self) -> Vec<PlannedIcmpProbe> {
        match &self.discovered {
            Some(sink) => std::mem::take(&mut *lock(sink)),
            None => Vec::new(),
        }
    }

    /// Drains the denial warnings raised so far in the request.
    pub(crate) fn take_warnings(&self) -> Vec<ComposeWarning> {
        std::mem::take(&mut *lock(&self.warnings))
    }

    fn permits(&self, target: &ScopedIpAddr) -> bool {
        self.grants.iter().any(|grant| grant.permits(target))
    }

    /// Runs one echo series for `function`.
    ///
    /// ## Returns
    ///
    /// `None` when nothing was sent — the target is not granted (one warning is
    /// recorded) or this is a discovery walk (the plan is recorded). Otherwise
    /// the verdict of the completed series.
    ///
    /// ## Errors
    ///
    /// [`ExpressionError::ContractViolation`] when the host could not send. No
    /// reply is an outcome, not an error, and a send failure aborts the series
    /// even after earlier replies.
    pub(crate) fn run(
        &self,
        function: &str,
        target: &ScopedIpAddr,
        budget: ProbeBudget,
    ) -> Result<Option<PingVerdict>, ExpressionError> {
        let granted = self.permits(target);
        if let Some(sink) = &self.discovered {
            lock(sink).push(PlannedIcmpProbe {
                function: function.to_string(),
                target: target.clone(),
                timeout: budget.per_attempt(),
                attempts: budget.attempts(),
                granted,
            });
            return Ok(None);
        }
        if !granted {
            lock(&self.warnings).push(ComposeWarning::new(
                STAGE,
                format!(
                    "<inverse>{function}()</inverse> did not probe <blue>{target}</blue>: no <dim>--allow-host</dim> entry grants ICMP to that address"
                ),
            ));
            return Ok(None);
        }
        let report = match &self.transport {
            Some(probe) => icmp::ping_with(&mut *lock(probe), target, budget),
            None => icmp::ping(target, budget),
        }
        .map_err(|error| ExpressionError::ContractViolation {
            function: function.to_string(),
            message: error.to_string(),
        })?;
        Ok(Some(report.verdict()))
    }
}

/// A poisoned ICMP mutex means a handler panicked mid-probe; the recorded
/// state is still sound, so the request continues rather than cascading.
fn lock<T: ?Sized>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Deterministic transport and grant fixtures shared by the ICMP tests.
///
/// Sending real ICMP from a test would depend on the host's privileges
/// (`net.ipv4.ping_group_range` on Linux) and on the network; every outcome
/// this feature maps is scripted here instead.
#[cfg(test)]
pub(crate) mod test_support {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use sniff::network::ScopedIpAddr;
    use sniff::network::icmp::{EchoOutcome, EchoProbe, IcmpError};

    use super::{IcmpAuthority, lock};

    /// Every target a scripted probe was actually asked to send to.
    #[derive(Clone, Default)]
    pub(crate) struct ProbeLog(Arc<Mutex<Vec<String>>>);

    impl ProbeLog {
        pub(crate) fn sent(&self) -> Vec<String> {
            lock(&self.0).clone()
        }

        pub(crate) fn count(&self) -> usize {
            lock(&self.0).len()
        }
    }

    /// An echo transport that replays a fixed script, one entry per attempt.
    pub(crate) struct ScriptedProbe {
        outcomes: VecDeque<Result<EchoOutcome, IcmpError>>,
        log: ProbeLog,
    }

    impl ScriptedProbe {
        pub(crate) fn new(log: &ProbeLog, outcomes: Vec<Result<EchoOutcome, IcmpError>>) -> Self {
            Self {
                outcomes: outcomes.into(),
                log: log.clone(),
            }
        }
    }

    impl EchoProbe for ScriptedProbe {
        fn echo(
            &mut self,
            target: &ScopedIpAddr,
            _timeout: Duration,
        ) -> Result<EchoOutcome, IcmpError> {
            lock(&self.log.0).push(target.to_string());
            self.outcomes
                .pop_front()
                .unwrap_or(Ok(EchoOutcome::NoReply))
        }
    }

    /// A reply after `millis`.
    pub(crate) fn reply(millis: u64) -> Result<EchoOutcome, IcmpError> {
        Ok(EchoOutcome::Reply {
            round_trip: Duration::from_millis(millis),
        })
    }

    pub(crate) fn no_reply() -> Result<EchoOutcome, IcmpError> {
        Ok(EchoOutcome::NoReply)
    }

    /// The host refusing to send, which aborts a series.
    pub(crate) fn send_failure() -> Result<EchoOutcome, IcmpError> {
        Err(IcmpError::NotPermitted {
            detail: "scripted refusal".to_string(),
        })
    }

    /// An authority granting `entries` and probing through `outcomes`.
    pub(crate) fn authority<S: AsRef<str>>(
        entries: &[S],
        log: &ProbeLog,
        outcomes: Vec<Result<EchoOutcome, IcmpError>>,
    ) -> IcmpAuthority {
        IcmpAuthority::default()
            .with_transport(ScriptedProbe::new(log, outcomes))
            .with_grants(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(text: &str) -> ScopedIpAddr {
        text.parse().expect("fixture address")
    }

    #[test]
    fn hostnames_and_wildcards_are_not_icmp_grants() {
        let authority = IcmpAuthority::default().with_grants(&["example.com", "*.example.com", ""]);
        assert!(authority.grants.is_empty());
        assert!(!authority.permits(&address("93.184.216.34")));
    }

    #[test]
    fn an_exact_grant_normalizes_spelling_and_keeps_an_explicit_zone() {
        let authority = IcmpAuthority::default().with_grants(&["2001:0db8:0000::0001", "fe80::1%en0"]);
        assert!(authority.permits(&address("2001:db8::1")));
        assert!(authority.permits(&address("fe80::1%en0")));
        // The grant named a zone, so another zone is a different capability.
        assert!(!authority.permits(&address("fe80::1%en1")));
        assert!(!authority.permits(&address("fe80::1")));
    }

    #[test]
    fn an_unscoped_grant_permits_any_zone_of_the_same_address() {
        let authority = IcmpAuthority::default().with_grants(&["fe80::1"]);
        assert!(authority.permits(&address("fe80::1")));
        assert!(authority.permits(&address("fe80::1%en0")));
    }

    #[test]
    fn a_cidr_grant_matches_bits_only_and_never_crosses_family() {
        let authority = IcmpAuthority::default().with_grants(&["10.1.2.3/16", "2001:db8::/32"]);
        assert!(authority.permits(&address("10.1.99.4")));
        assert!(!authority.permits(&address("10.2.0.1")));
        assert!(authority.permits(&address("2001:db8::99%en0")));
        // An IPv4-mapped IPv6 address is not inside an IPv4 network.
        assert!(!authority.permits(&address("::ffff:10.1.0.1")));
    }

    #[test]
    fn only_a_cidr_entry_is_withheld_from_the_http_allowlist() {
        assert!(is_icmp_only_entry("10.0.0.0/8"));
        assert!(is_icmp_only_entry("2001:db8::/32"));
        assert!(!is_icmp_only_entry("10.0.0.1"));
        assert!(!is_icmp_only_entry("example.com"));
        // A malformed prefix length is not a CIDR, so it stays an HTTP entry
        // and grants no ICMP.
        assert!(!is_icmp_only_entry("10.0.0.0/99"));
        assert!(IcmpGrant::parse("10.0.0.0/99").is_none());
    }
}
