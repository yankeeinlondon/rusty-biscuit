//! Crate-boundary coverage for interface addresses, gateways, and ICMP.
//!
//! `real_` tests touch the host's network stack and run in the real-resource
//! tier (`just test-real`). The loopback ICMP round trips are the local form of
//! AC14; they fail, rather than skip, when the host cannot send ICMP, and the
//! failure names the missing capability.

use std::time::{Duration, Instant};

use sniff::network::icmp::{self, IcmpError, PingVerdict, ProbeBudget};
use sniff::network::{
    ScopedIpAddr, contains_cgnat_address, detect_default_gateways, detect_network_with_request,
    host_addresses,
};
use sniff::request::NetworkRequest;

#[test]
fn host_addresses_from_real_interfaces_are_unique_sorted_and_reparseable() {
    let info = detect_network_with_request(&NetworkRequest::interfaces_only()).unwrap();
    let addresses = host_addresses(&info.interfaces);

    let mut sorted = addresses.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(addresses, sorted);

    for address in &addresses {
        let spelling = address.to_string();
        assert_eq!(spelling.parse::<ScopedIpAddr>().unwrap(), *address);
        assert_eq!(
            address.scope().is_some(),
            address.address().is_ipv6() && address.is_link_local(),
            "{spelling}: exactly the IPv6 link-local addresses are zoned"
        );
    }

    let expected_cgnat = info
        .ip_addresses
        .v4
        .iter()
        .any(|address| address.address.starts_with("100.") && {
            let second: u8 = address.address.split('.').nth(1).unwrap().parse().unwrap();
            (64..128).contains(&second)
        });
    assert_eq!(contains_cgnat_address(&addresses), expected_cgnat);
}

fn loopback_series(target: &str) -> icmp::PingReport {
    let target: ScopedIpAddr = target.parse().unwrap();
    let budget = ProbeBudget::from_millis(1000.0, 3.0).unwrap();
    let started = Instant::now();
    let report = icmp::ping(&target, budget).unwrap_or_else(|error| {
        panic!(
            "ICMP to {target} could not be sent; this environment lacks the \
             unprivileged ICMP capability AC14 requires: {error}"
        )
    });
    assert!(started.elapsed() < budget.deadline_bound());
    report
}

#[test]
fn real_icmp_ipv4_loopback_replies() {
    let report = loopback_series("127.0.0.1");
    assert_eq!(report.verdict(), PingVerdict::AllReplied, "{report:?}");
}

#[test]
fn real_icmp_ipv6_loopback_replies() {
    let report = loopback_series("::1");
    assert_eq!(report.verdict(), PingVerdict::AllReplied, "{report:?}");
}

#[test]
fn real_icmp_unanswered_target_is_no_reply_within_the_bound() {
    // TEST-NET-1 (RFC 5737) is never assigned, so nothing answers.
    let target: ScopedIpAddr = "192.0.2.1".parse().unwrap();
    let budget = ProbeBudget::new(Duration::from_millis(150), 2).unwrap();
    let started = Instant::now();
    let report = icmp::ping(&target, budget).unwrap();
    let elapsed = started.elapsed();
    assert_eq!(report.verdict(), PingVerdict::NoneReplied, "{report:?}");
    assert!(elapsed < budget.deadline_bound(), "{elapsed:?}");
}

#[test]
fn real_icmp_unknown_scope_fails_before_sending() {
    let target: ScopedIpAddr = "fe80::1%sniff-no-such-if".parse().unwrap();
    let budget = ProbeBudget::from_millis(100.0, 3.0).unwrap();
    let started = Instant::now();
    let error = icmp::ping(&target, budget).unwrap_err();
    assert!(matches!(error, IcmpError::UnknownScope { .. }), "{error:?}");
    assert!(started.elapsed() < Duration::from_millis(100));
}

#[test]
fn real_default_gateways_are_readable_and_well_formed() {
    let gateways = detect_default_gateways().unwrap();
    if let Some(v4) = gateways.v4 {
        assert!(!v4.is_unspecified());
    }
    if let Some(v6) = &gateways.v6 {
        assert!(!v6.address().is_unspecified());
        assert_eq!(v6.scope().is_some(), v6.is_link_local(), "{v6}");
    }
    let json = serde_json::to_string(&gateways).unwrap();
    assert_eq!(
        serde_json::from_str::<sniff::network::DefaultGateways>(&json).unwrap(),
        gateways
    );
}
