//! `ping` / `ping_under` through the real compose pipeline and preflight walk
//! (AC6-AC8, AC33, and the ICMP portions of AC20/AC32).
//!
//! These run inside the crate so the request's ICMP transport can be replaced
//! with a scripted one. No test needs a network or ICMP privileges.

use super::*;

use crate::markdown::compose::icmp::IcmpAuthority;
use crate::markdown::compose::icmp::test_support::{ProbeLog, ScriptedProbe, no_reply, reply};

/// Options granting `hosts`, probing through a scripted transport, and reading
/// no ambient host state.
fn options(hosts: &[&str], log: &ProbeLog, outcomes: Vec<Result<sniff::network::icmp::EchoOutcome, sniff::network::icmp::IcmpError>>) -> ComposeOptions {
    let mut options = ComposeOptions::new();
    for host in hosts {
        options = options.with_allowed_host(*host);
    }
    options.icmp = IcmpAuthority::default().with_transport(ScriptedProbe::new(log, outcomes));
    options
}

fn compose(document: &str, options: ComposeOptions) -> (Markdown, ComposeReport) {
    Markdown::from(document)
        .compose_with(options)
        .unwrap_or_else(|error| panic!("compose must succeed: {error}"))
}

/// A granted target probes once through the request's transport and the
/// boolean reaches the body.
#[test]
fn a_granted_ping_resolves_through_the_pipeline() {
    let log = ProbeLog::default();
    let (composed, report) = compose(
        "Reachable: {{ ping(\"10.1.2.3\", 250) }}\n",
        options(&["10.1.0.0/16"], &log, vec![reply(4)]),
    );

    assert_eq!(composed.content().trim(), "Reachable: true");
    assert_eq!(log.sent(), vec!["10.1.2.3".to_string()]);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

/// AC6: an ungranted target is denied once, with the warning reaching the
/// compose report rather than the document.
#[test]
fn ac6_an_ungranted_ping_warns_once_and_sends_nothing() {
    let log = ProbeLog::default();
    let (_composed, report) = compose(
        "Reachable: {{ ping(\"10.9.9.9\") }}\n",
        options(&["10.1.0.0/16"], &log, vec![reply(4)]),
    );

    assert_eq!(log.count(), 0);
    let icmp: Vec<_> = report.warnings.iter().filter(|w| w.stage == "icmp").collect();
    assert_eq!(icmp.len(), 1, "{:?}", report.warnings);
    assert!(icmp[0].message.contains("10.9.9.9"), "{}", icmp[0].message);
}

/// AC8: a mixed series through a frontmatter value keeps `"unstable"` as a
/// string, not a coerced boolean.
#[test]
fn ac8_ping_under_reaches_frontmatter_as_its_literal_verdict() {
    let log = ProbeLog::default();
    let (composed, _report) = compose(
        "---\nlink: '{{ ping_under(\"10.1.2.3\", 50, 3) }}'\n---\nLink is {{ link }}\n",
        options(&["10.1.2.3"], &log, vec![reply(1), no_reply(), reply(2)]),
    );

    assert_eq!(composed.content().trim(), "Link is unstable");
    assert_eq!(log.count(), 3);
}

/// AC6/AC32: a statically known ping appears in the root preflight as a typed
/// effect, and collecting it sends nothing.
#[test]
fn ac6_preflight_lists_a_planned_ping_without_sending_it() {
    let log = ProbeLog::default();
    let document = Markdown::from("Reachable: {{ ping(\"10.9.9.9\", 250) }}\n");
    let options = options(&["10.1.0.0/16"], &log, vec![reply(4)]);

    let report = document.compose_preflight(&options).expect("preflight succeeds");

    assert_eq!(log.count(), 0, "preflight must not send ICMP");
    assert_eq!(report.icmp_probes.len(), 1, "{:?}", report.icmp_probes);
    let probe = &report.icmp_probes[0];
    assert_eq!(probe.function, "ping");
    assert_eq!(probe.target.to_string(), "10.9.9.9");
    assert_eq!(probe.timeout, std::time::Duration::from_millis(250));
    assert_eq!(probe.attempts, 1);
    assert!(!probe.granted, "an ungranted probe is the one an approver acts on");
}

/// AC32: a ping the condition-aware pass would never evaluate — a false page
/// block and an untaken ternary branch — is still discovered, and a granted
/// one is reported as already approved.
#[test]
fn ac32_preflight_is_condition_blind_and_records_the_grant_verdict() {
    let log = ProbeLog::default();
    let document = Markdown::from(
        "{{ ping(\"10.1.0.1\") }}\n\n\
         ::block when=\"false\"\n{{ ping(\"10.1.0.2\") }}\n::end-block\n\n\
         {{ false ? ping_under(\"10.1.0.3\", 20, 2) : \"\" }}\n",
    );

    let report = document
        .compose_preflight(&options(&["10.1.0.0/16"], &log, vec![reply(4)]))
        .expect("preflight succeeds");

    // Discovery order follows the interpolation walk, which is not source
    // order; the claim is coverage, not sequence.
    let mut targets: Vec<String> = report
        .icmp_probes
        .iter()
        .map(|probe| probe.target.to_string())
        .collect();
    targets.sort();
    assert_eq!(targets, ["10.1.0.1", "10.1.0.2", "10.1.0.3"]);
    assert!(report.icmp_probes.iter().all(|probe| probe.granted));
    let untaken = report
        .icmp_probes
        .iter()
        .find(|probe| probe.target.to_string() == "10.1.0.3")
        .expect("the untaken branch is discovered");
    assert_eq!((untaken.function.as_str(), untaken.attempts), ("ping_under", 2));
    assert_eq!(log.count(), 0);
}

/// AC32 (nesting): a ping inside `as_markdown` content appears in the *root*
/// preflight, and the nested child sends it under the root's consent rather
/// than acquiring its own.
#[test]
fn ac32_a_nested_ping_appears_in_root_preflight_and_inherits_root_consent() {
    let log = ProbeLog::default();
    let document = Markdown::from("{{ as_markdown(\"Nested: {{ ping(\\\"10.1.2.3\\\") }}\") }}\n");

    let report = document
        .compose_preflight(&options(&["10.1.0.0/16"], &log, vec![reply(4)]))
        .expect("preflight succeeds");
    assert_eq!(log.count(), 0);
    assert_eq!(
        report.icmp_probes.iter().map(|p| p.target.to_string()).collect::<Vec<_>>(),
        ["10.1.2.3"]
    );

    let composed_log = ProbeLog::default();
    let (composed, compose_report) = compose(
        "{{ as_markdown(\"Nested: {{ ping(\\\"10.1.2.3\\\") }}\") }}\n",
        options(&["10.1.0.0/16"], &composed_log, vec![reply(4)]),
    );
    assert_eq!(composed.content().trim(), "Nested: true");
    assert_eq!(composed_log.sent(), vec!["10.1.2.3".to_string()]);
    assert!(compose_report.warnings.is_empty(), "{:?}", compose_report.warnings);
}

/// A nested child cannot widen the root's grant: the denial and its single
/// warning are the same as they would be at the root.
#[test]
fn a_nested_ping_cannot_escape_the_root_grant() {
    let log = ProbeLog::default();
    let (_composed, report) = compose(
        "{{ as_markdown(\"Nested: {{ ping(\\\"10.9.9.9\\\") }}\") }}\n",
        options(&["10.1.0.0/16"], &log, vec![reply(4)]),
    );

    assert_eq!(log.count(), 0);
    assert_eq!(report.warnings.iter().filter(|w| w.stage == "icmp").count(), 1);
}

/// AC33: a CIDR grant authorizes ICMP and nothing else. The same entry leaves
/// the HTTP allowlist untouched, so a URL inside that range is still denied.
#[test]
fn ac33_a_cidr_grant_never_authorizes_http_to_that_range() {
    let config = crate::markdown::compose::RemoteReadConfig {
        allowed_hosts: vec!["10.1.0.0/16".to_string(), "example.com".to_string()],
        ..Default::default()
    };

    assert!(!config.is_host_allowed("10.1.2.3"));
    assert!(!config.is_host_allowed("10.1.0.0/16"));
    assert!(config.is_host_allowed("example.com"));
    assert_eq!(config.http_hosts().collect::<Vec<_>>(), [&"example.com".to_string()]);
}

/// A target computed during evaluation is revalidated against the grant before
/// anything is sent, so it can only use a capability that was already
/// approved — evaluating the expression never acquires one.
#[test]
fn a_dynamic_target_is_revalidated_against_the_grant() {
    let log = ProbeLog::default();
    let (composed, report) = compose(
        "---\ninside: 10.1.2.3\noutside: 10.9.9.9\n---\nin={{ ping(inside) }} out={{ ping(outside) }}\n",
        options(&["10.1.0.0/16"], &log, vec![reply(4)]),
    );

    assert!(composed.content().contains("in=true"), "{}", composed.content());
    assert_eq!(log.sent(), vec!["10.1.2.3".to_string()]);
    assert_eq!(report.warnings.iter().filter(|w| w.stage == "icmp").count(), 1);
}
