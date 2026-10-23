//! Network functions: `ipv4`/`ipv6` address filters over the request's
//! captured interface addresses (R14), and the `ping`/`ping_under` ICMP
//! reachability probes governed by the request's ICMP grants (R6).

use ipnet::IpNet;
use serde_json::Value;
use sniff::network::ScopedIpAddr;
use sniff::network::icmp::{PingVerdict, ProbeBudget};

use super::{BoundCall, FunctionBinding, ResolutionContext, required_text};
use crate::markdown::compose::context::ContextGroup;
use crate::markdown::compose::expression::ExpressionError;

/// `ping`'s per-attempt timeout when the caller supplies none.
const DEFAULT_PING_TIMEOUT_MS: f64 = 100.0;

/// `ping_under`'s attempt count when the caller supplies none.
const DEFAULT_PING_UNDER_ATTEMPTS: f64 = 3.0;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::context("ipv4", &[], ipv4_fn),
    FunctionBinding::context("ipv6", &[], ipv6_fn),
    FunctionBinding::context("ping", &[], ping_fn),
    FunctionBinding::context("ping_under", &[], ping_under_fn),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    V4,
    V6,
}

fn ipv4_fn(call: &BoundCall<'_>, context: &ResolutionContext) -> Result<Value, ExpressionError> {
    addresses("ipv4", Family::V4, call, context)
}

fn ipv6_fn(call: &BoundCall<'_>, context: &ResolutionContext) -> Result<Value, ExpressionError> {
    addresses("ipv6", Family::V6, call, context)
}

/// An omitted `filter` selects the default set.
fn addresses(
    function: &'static str,
    family: Family,
    call: &BoundCall<'_>,
    context: &ResolutionContext,
) -> Result<Value, ExpressionError> {
    let filter = call.args().text(0);
    let observed = context.observations.addresses().ok_or_else(|| {
        ExpressionError::FunctionContextNotCaptured {
            function: function.to_string(),
            group: ContextGroup::Network,
        }
    })?;
    Ok(Value::Array(
        select(observed, family, filter)
            .map(|address| Value::String(address.to_string()))
            .collect(),
    ))
}

/// Applies the R14 filter rule to the addresses of one family.
///
/// A filter is a CIDR only when the whole string parses as one; a CIDR selects
/// by address bits (zone ignored) and may include loopback or link-local. A
/// CIDR of the other family selects nothing. Any other filter is a substring
/// match over the default set, which excludes loopback and link-local.
fn select<'a>(
    observed: &'a [ScopedIpAddr],
    family: Family,
    filter: Option<&'a str>,
) -> impl Iterator<Item = &'a ScopedIpAddr> {
    let network = filter.and_then(|filter| filter.parse::<IpNet>().ok());
    observed.iter().filter(move |address| {
        if family_of(address) != family {
            return false;
        }
        match (network, filter) {
            (Some(network), _) => address.is_within(&network),
            (None, filter) => {
                !address.is_loopback()
                    && !address.is_link_local()
                    && filter.is_none_or(|filter| address.to_string().contains(filter))
            }
        }
    })
}

fn family_of(address: &ScopedIpAddr) -> Family {
    if address.address().is_ipv4() { Family::V4 } else { Family::V6 }
}

/// `ping(address, timeout?)`: one echo, `true` when it replies in time.
fn ping_fn(call: &BoundCall<'_>, context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let target = target("ping", call)?;
    let timeout = call.args().number(1).unwrap_or(DEFAULT_PING_TIMEOUT_MS);
    let budget = budget("ping", timeout, 1.0)?;
    // One attempt can only be all-replied or none-replied.
    Ok(match context.icmp.run("ping", &target, budget)? {
        None => Value::Null,
        Some(verdict) => Value::Bool(verdict == PingVerdict::AllReplied),
    })
}

/// `ping_under(address, timeout, attempts?)`: a sequential series, `true` when
/// every reply beat the threshold, `false` when none did, `"unstable"` between.
fn ping_under_fn(call: &BoundCall<'_>, context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let target = target("ping_under", call)?;
    // Required, so always bound; NaN could only fail the budget check below.
    let timeout = call.args().number(1).unwrap_or(f64::NAN);
    let attempts = call.args().number(2).unwrap_or(DEFAULT_PING_UNDER_ATTEMPTS);
    let budget = budget("ping_under", timeout, attempts)?;
    Ok(match context.icmp.run("ping_under", &target, budget)? {
        None => Value::Null,
        Some(PingVerdict::AllReplied) => Value::Bool(true),
        Some(PingVerdict::NoneReplied) => Value::Bool(false),
        Some(PingVerdict::Unstable) => Value::String("unstable".to_string()),
    })
}

/// The bound `ip-address` as a scoped address.
///
/// Nothing here resolves DNS: binding already refused anything that is not an
/// IPv4, IPv6, or scoped IPv6 literal, so a parse failure here means the
/// `ip-address` pattern admitted a spelling the parser does not.
fn target(function: &'static str, call: &BoundCall<'_>) -> Result<ScopedIpAddr, ExpressionError> {
    let text = required_text(function, call, 0)?;
    text.parse().map_err(|error| ExpressionError::ContractViolation {
        function: function.to_string(),
        message: format!("`{text}` is not an IP address literal: {error}"),
    })
}

/// The validated budget, or the compose error its rejection carries.
///
/// [`ProbeBudget::from_millis`] range-checks before converting, so a
/// non-finite, non-positive, fractional, or overflowing value is rejected
/// rather than truncated.
fn budget(
    function: &'static str,
    timeout_ms: f64,
    attempts: f64,
) -> Result<ProbeBudget, ExpressionError> {
    ProbeBudget::from_millis(timeout_ms, attempts).map_err(|error| {
        ExpressionError::ContractViolation {
            function: function.to_string(),
            message: error.to_string(),
        }
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::super::call_function;
    use crate::markdown::compose::context::capture::CapturedObservations;
    use crate::markdown::compose::context::ContextGroup;
    use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};

    /// AC20's fixture plus an IPv6 set with a scoped link-local address.
    fn context() -> ResolutionContext {
        let mut addresses: Vec<_> = [
            "127.0.0.1",
            "169.254.3.4",
            "192.168.10.5",
            "10.1.2.3",
            "100.101.102.103",
            "::1",
            "fe80::1%en0",
            "fd00::5",
            "2001:db8::10",
        ]
        .iter()
        .map(|text| text.parse().expect("fixture address"))
        .collect();
        // Capture delivers `sniff::network::host_addresses` order.
        addresses.sort();
        ResolutionContext::default()
            .with_observations(CapturedObservations::for_test_addresses(&addresses))
    }

    fn call(name: &str, args: &[Value]) -> Result<Value, ExpressionError> {
        call_function(name, args, Some(&context())).expect("registered context function")
    }

    #[test]
    fn default_sets_exclude_loopback_link_local_and_the_other_family() {
        assert_eq!(call("ipv4", &[]).unwrap(), json!(["10.1.2.3", "100.101.102.103", "192.168.10.5"]));
        assert_eq!(call("ipv6", &[]).unwrap(), json!(["2001:db8::10", "fd00::5"]));
        // Ledger row 69: omission selects the default set; `null` is not
        // omission and is a type error.
        assert!(matches!(call("ipv4", &[Value::Null]), Err(ExpressionError::Call(_))));
    }

    /// AC20: a covering CIDR includes loopback, a partial address is a
    /// substring, and an invalid prefix length falls back to a substring match
    /// that nothing contains.
    #[test]
    fn ac20_cidr_substring_and_malformed_cidr_fallback() {
        assert_eq!(call("ipv4", &[json!("127.0.0.0/8")]).unwrap(), json!(["127.0.0.1"]));
        assert_eq!(call("ipv4", &[json!("192.168.10")]).unwrap(), json!(["192.168.10.5"]));
        assert_eq!(call("ipv4", &[json!("192.168.10/99")]).unwrap(), json!([]));
        assert_eq!(call("ipv4", &[json!("169.254.0.0/16")]).unwrap(), json!(["169.254.3.4"]));
        // A substring never reaches the excluded loopback address.
        assert_eq!(call("ipv4", &[json!("127.0")]).unwrap(), json!([]));
        // Host bits in a CIDR select by its network.
        assert_eq!(call("ipv4", &[json!("192.168.10.99/24")]).unwrap(), json!(["192.168.10.5"]));
        assert_eq!(call("ipv4", &[json!("0.0.0.0/0")]).unwrap().as_array().unwrap().len(), 5);
    }

    #[test]
    fn ipv6_cidrs_compare_bits_and_keep_the_returned_scope() {
        assert_eq!(call("ipv6", &[json!("::1/128")]).unwrap(), json!(["::1"]));
        assert_eq!(call("ipv6", &[json!("fe80::/10")]).unwrap(), json!(["fe80::1%en0"]));
        assert_eq!(call("ipv6", &[json!("fd00")]).unwrap(), json!(["fd00::5"]));
        // A scoped spelling is not a CIDR, and substring matching skips link-local.
        assert_eq!(call("ipv6", &[json!("fe80::%en0/64")]).unwrap(), json!([]));
        assert_eq!(call("ipv6", &[json!("%en0")]).unwrap(), json!([]));
    }

    #[test]
    fn a_valid_cidr_of_the_other_family_selects_nothing() {
        assert_eq!(call("ipv4", &[json!("::/0")]).unwrap(), json!([]));
        assert_eq!(call("ipv6", &[json!("0.0.0.0/0")]).unwrap(), json!([]));
        assert_eq!(call("ipv6", &[json!("::ffff:192.168.10.5/128")]).unwrap(), json!([]));
    }

    #[test]
    fn empty_capture_is_empty_and_uncaptured_network_is_fatal() {
        let empty = ResolutionContext::default()
            .with_observations(CapturedObservations::for_test_addresses(&[]));
        assert_eq!(call_function("ipv4", &[], Some(&empty)).unwrap().unwrap(), json!([]));

        let error = call_function("ipv6", &[], Some(&ResolutionContext::default())).unwrap().unwrap_err();
        assert!(matches!(
            error,
            ExpressionError::FunctionContextNotCaptured { group: ContextGroup::Network, .. }
        ));
        assert!(error.is_authoring_fatal());
    }

    /// Ledger rows 69-70: a number filter converts to its text (a substring
    /// filter); a list filter or an extra argument fails at binding.
    #[test]
    fn filters_bind_as_text_and_extra_arguments_are_errors() {
        assert_eq!(call("ipv4", &[json!(10)]).unwrap(), call("ipv4", &[json!("10")]).unwrap());
        assert!(matches!(call("ipv4", &[json!(["10"])]), Err(ExpressionError::Call(_))));
        assert!(matches!(call("ipv6", &[json!("a"), json!("b")]), Err(ExpressionError::Call(_))));
    }
}

#[cfg(test)]
mod ping_tests {
    //! Dispatch-level coverage for `ping` and `ping_under` (AC6-AC8, AC33).
    //!
    //! Every probe runs through a scripted transport, so no test needs a
    //! network, ICMP privileges, or a live host.

    use serde_json::{Value, json};

    use super::super::call_function;
    use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};
    use crate::markdown::compose::icmp::IcmpAuthority;
    use crate::markdown::compose::icmp::test_support::{
        ProbeLog, authority, no_reply, reply, send_failure,
    };

    fn context(authority: &IcmpAuthority) -> ResolutionContext {
        ResolutionContext {
            icmp: authority.clone(),
            ..ResolutionContext::default()
        }
    }

    fn call(name: &str, args: &[Value], authority: &IcmpAuthority) -> Result<Value, ExpressionError> {
        call_function(name, args, Some(&context(authority))).expect("registered context function")
    }

    /// AC6: an ungranted target answers `null` with exactly one warning, and
    /// nothing leaves the host.
    #[test]
    fn ac6_a_denied_target_is_null_plus_one_warning_and_sends_nothing() {
        let log = ProbeLog::default();
        let grants = authority(&["10.0.0.1"], &log, vec![reply(1)]);

        assert_eq!(call("ping", &[json!("10.0.0.2")], &grants).unwrap(), Value::Null);

        assert_eq!(log.count(), 0, "a denial must send no packet");
        let warnings = grants.take_warnings();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].stage, "icmp");
        assert!(warnings[0].message.contains("10.0.0.2"), "{}", warnings[0].message);
        assert!(warnings[0].message.contains("--allow-host"), "{}", warnings[0].message);
    }

    /// AC33: a denied multi-attempt call returns one `null` and one diagnostic
    /// and sends nothing — it does not consume its attempt budget first.
    #[test]
    fn ac33_a_denied_multi_attempt_call_sends_no_packets() {
        let log = ProbeLog::default();
        let grants = authority(&["192.0.2.0/24"], &log, vec![reply(1), reply(1), reply(1)]);

        let value = call("ping_under", &[json!("198.51.100.7"), json!(50), json!(3)], &grants);

        assert_eq!(value.unwrap(), Value::Null);
        assert_eq!(log.count(), 0);
        assert_eq!(grants.take_warnings().len(), 1);
    }

    /// A CIDR grant governs by address bits: inside probes, outside is denied,
    /// and a valid CIDR of the other family never matches.
    #[test]
    fn a_cidr_grant_admits_its_own_family_only() {
        let log = ProbeLog::default();
        let grants = authority(&["10.1.0.0/16", "::/0"], &log, vec![reply(1), reply(1)]);

        assert_eq!(call("ping", &[json!("10.1.250.9")], &grants).unwrap(), json!(true));
        assert_eq!(call("ping", &[json!("10.2.0.1")], &grants).unwrap(), Value::Null);
        // `::/0` covers every IPv6 address but no IPv4 one.
        assert_eq!(call("ping", &[json!("2001:db8::9")], &grants).unwrap(), json!(true));

        assert_eq!(log.sent(), vec!["10.1.250.9".to_string(), "2001:db8::9".to_string()]);
    }

    /// AC33: a scoped IPv6 target keeps its zone through validation, the grant
    /// comparison, and the transport.
    #[test]
    fn ac33_a_scoped_ipv6_target_keeps_its_zone() {
        let log = ProbeLog::default();
        let grants = authority(&["fe80::1%en0"], &log, vec![reply(1), reply(1)]);

        assert_eq!(call("ping", &[json!("fe80::1%en0")], &grants).unwrap(), json!(true));
        // The grant is zone-specific, so another interface is a different
        // capability even though the address bits match.
        assert_eq!(call("ping", &[json!("fe80::1%en1")], &grants).unwrap(), Value::Null);

        assert_eq!(log.sent(), vec!["fe80::1%en0".to_string()]);
    }

    /// AC7: a granted but silent target is `false`, not `null` and not an error.
    #[test]
    fn ac7_no_reply_within_the_timeout_is_false() {
        let log = ProbeLog::default();
        let grants = authority(&["10.0.0.1"], &log, vec![no_reply()]);

        assert_eq!(call("ping", &[json!("10.0.0.1")], &grants).unwrap(), json!(false));
        assert_eq!(log.count(), 1);
        assert!(grants.take_warnings().is_empty());
    }

    /// A reply *at* the threshold is late, so it is no reply.
    #[test]
    fn a_reply_at_the_threshold_counts_as_late() {
        let log = ProbeLog::default();
        let grants = authority(&["10.0.0.1"], &log, vec![reply(50), reply(49)]);

        assert_eq!(call("ping", &[json!("10.0.0.1"), json!(50)], &grants).unwrap(), json!(false));
        assert_eq!(call("ping", &[json!("10.0.0.1"), json!(50)], &grants).unwrap(), json!(true));
    }

    /// AC7: the host refusing to send is a compose error that no surface
    /// demotes, and it is distinct from "no reply".
    #[test]
    fn ac7_a_send_failure_is_a_never_demoted_compose_error() {
        let log = ProbeLog::default();
        let grants = authority(&["10.0.0.1"], &log, vec![send_failure()]);

        let error = call("ping", &[json!("10.0.0.1")], &grants).unwrap_err();

        assert!(matches!(error, ExpressionError::ContractViolation { ref function, .. } if function == "ping"));
        assert!(error.is_authoring_fatal());
        assert!(grants.take_warnings().is_empty(), "a send failure is not a denial");
    }

    /// AC33: a send failure after an earlier success aborts the series rather
    /// than reporting the replies it already had.
    #[test]
    fn ac33_a_send_failure_after_a_success_aborts_the_series() {
        let log = ProbeLog::default();
        let grants = authority(&["10.0.0.1"], &log, vec![reply(1), send_failure(), reply(1)]);

        let error = call("ping_under", &[json!("10.0.0.1"), json!(50), json!(3)], &grants).unwrap_err();

        assert!(matches!(error, ExpressionError::ContractViolation { .. }));
        assert_eq!(log.count(), 2, "the third attempt must not run");
    }

    /// AC8: the three `ping_under` verdicts, each from a scripted mix.
    #[test]
    fn ac8_ping_under_maps_all_some_and_no_replies() {
        let all = ProbeLog::default();
        let grants = authority(&["10.0.0.1"], &all, vec![reply(1), reply(2), reply(3)]);
        assert_eq!(
            call("ping_under", &[json!("10.0.0.1"), json!(50)], &grants).unwrap(),
            json!(true)
        );
        assert_eq!(all.count(), 3, "the default attempt count is three");

        let mixed = ProbeLog::default();
        let grants = authority(&["10.0.0.1"], &mixed, vec![reply(1), no_reply(), reply(3)]);
        assert_eq!(
            call("ping_under", &[json!("10.0.0.1"), json!(50)], &grants).unwrap(),
            json!("unstable")
        );

        let none = ProbeLog::default();
        let grants = authority(&["10.0.0.1"], &none, vec![no_reply(), no_reply()]);
        assert_eq!(
            call("ping_under", &[json!("10.0.0.1"), json!(50), json!(2)], &grants).unwrap(),
            json!(false)
        );
        assert_eq!(none.count(), 2);
    }

    /// AC33: every malformed address is rejected before a packet is sent, and
    /// no spelling reaches DNS. The `ip-address` type rejects each one at
    /// binding (ledger row 71), so the handler is never entered.
    #[test]
    fn ac33_malformed_addresses_are_rejected_without_probing() {
        use crate::markdown::compose::expression::functions::test_hooks::{handler_entries, reset_handler_entries};

        reset_handler_entries();
        let log = ProbeLog::default();
        // A grant wide enough that only validation can stop these.
        let grants = authority(&["0.0.0.0/0", "::/0"], &log, vec![reply(1)]);

        for spelling in [
            "localhost",
            "example.com",
            "10.0.0.256",
            "10.0.0",
            "[::1]",
            " 10.0.0.1",
            "10.0.0.1%en0",
            "fe80::1%",
            "",
        ] {
            let error = call("ping", &[json!(spelling)], &grants)
                .unwrap_err_or_panic(spelling);
            assert!(matches!(error, ExpressionError::Call(_)), "{spelling}: {error}");
        }
        assert_eq!(handler_entries("ping"), 0);
        assert_eq!(log.count(), 0);
    }

    /// AC33: non-finite, non-positive, overflowing, and fractional numbers are
    /// range-checked before conversion rather than truncated. `timeout` is a
    /// plain `number`, so its range stays the handler's check; `attempts` is
    /// `number(integer; min(1); max(100))`, so binding rejects it.
    #[test]
    fn ac33_invalid_time_and_attempt_budgets_are_rejected() {
        let log = ProbeLog::default();
        let grants = authority(&["10.0.0.1"], &log, vec![reply(1)]);

        for timeout in [json!(0), json!(-5), json!(60_001), json!(1e300)] {
            let error = call("ping", &[json!("10.0.0.1"), timeout.clone()], &grants)
                .unwrap_err_or_panic(&timeout.to_string());
            assert!(matches!(error, ExpressionError::ContractViolation { .. }), "{timeout}: {error}");
        }

        for attempts in [json!(0), json!(2.5), json!(-1), json!(101), json!(1e18)] {
            let error = call("ping_under", &[json!("10.0.0.1"), json!(50), attempts.clone()], &grants)
                .unwrap_err_or_panic(&attempts.to_string());
            assert!(matches!(error, ExpressionError::Call(_)), "{attempts}: {error}");
        }
        assert_eq!(log.count(), 0);
    }

    /// An absent optional argument takes the default. Ledger rows 71-72:
    /// `null` is not omission and is a type error; numeric text converts.
    #[test]
    fn optional_arguments_default_and_wrong_types_are_binding_errors() {
        let log = ProbeLog::default();
        let grants = authority(&["10.0.0.1"], &log, vec![reply(1), reply(1), reply(1), reply(1)]);

        assert_eq!(call("ping", &[json!("10.0.0.1")], &grants).unwrap(), json!(true));
        assert_eq!(call("ping", &[json!("10.0.0.1"), json!("50")], &grants).unwrap(), json!(true));

        for (name, args) in [
            ("ping", vec![json!("10.0.0.1"), Value::Null]),
            ("ping", vec![json!(10)]),
            // `ping_under`'s timeout is required, unlike `ping`'s.
            ("ping_under", vec![json!("10.0.0.1")]),
            ("ping_under", vec![json!("10.0.0.1"), Value::Null]),
        ] {
            assert!(matches!(call(name, &args, &grants), Err(ExpressionError::Call(_))), "{name}{args:?}");
        }
        assert_eq!(log.count(), 2);
    }

    /// The default authority grants nothing, so a surface that was never handed
    /// a request's consent cannot reach the network.
    #[test]
    fn the_default_authority_denies_every_target() {
        let grants = IcmpAuthority::default();
        assert_eq!(call("ping", &[json!("127.0.0.1")], &grants).unwrap(), Value::Null);
        assert_eq!(grants.take_warnings().len(), 1);
    }

    /// A discovery authority records the plan and answers `null` without
    /// sending, which is what keeps preflight passive.
    #[test]
    fn a_discovery_authority_records_the_plan_and_sends_nothing() {
        let log = ProbeLog::default();
        let grants = authority(&["10.0.0.1"], &log, vec![reply(1)]).discovering();

        assert_eq!(call("ping", &[json!("10.0.0.1"), json!(250)], &grants).unwrap(), Value::Null);
        assert_eq!(call("ping_under", &[json!("10.9.9.9"), json!(20), json!(4)], &grants).unwrap(), Value::Null);

        let planned = grants.take_discovered();
        assert_eq!(log.count(), 0);
        assert_eq!(planned.len(), 2);
        assert_eq!(planned[0].function, "ping");
        assert_eq!(planned[0].timeout, std::time::Duration::from_millis(250));
        assert_eq!(planned[0].attempts, 1);
        assert!(planned[0].granted);
        assert_eq!(planned[1].function, "ping_under");
        assert_eq!(planned[1].attempts, 4);
        assert!(!planned[1].granted, "an ungranted plan is the approvable one");
        // Discovery raises no denial warning; preflight is passive.
        assert!(grants.take_warnings().is_empty());
    }

    /// `Result::unwrap_err` with the offending input in the panic message.
    trait UnwrapErrOrPanic {
        fn unwrap_err_or_panic(self, input: &str) -> ExpressionError;
    }

    impl UnwrapErrOrPanic for Result<Value, ExpressionError> {
        fn unwrap_err_or_panic(self, input: &str) -> ExpressionError {
            match self {
                Ok(value) => panic!("`{input}` must be rejected, got {value}"),
                Err(error) => error,
            }
        }
    }
}
