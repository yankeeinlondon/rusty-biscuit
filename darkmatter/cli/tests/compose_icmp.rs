//! ICMP consent through the normal `md compose` path (AC6, AC33).
//!
//! `--allow-host` is one entry point for two disjoint policies. Every case
//! here uses targets no entry grants, or the packet-free `--shell` preflight,
//! so the tests send no ICMP and open no socket.

mod common;

use common::CliProcessFixture;
use predicates::prelude::*;

/// The documentation range `10.77.0.0/16` is granted; `10.99.0.0/16` is not.
const DOCUMENT: &str = "inside={{ ping(\"10.77.1.5\", 250) }} outside={{ ping_under(\"10.99.1.5\", 20, 4) }}\n";

/// AC6: preflight reports both probes as typed effects with the grant verdict
/// the CIDR entry decided, and sends nothing to either.
#[test]
fn preflight_reports_planned_icmp_effects_and_their_cidr_grant() {
    let fixture = CliProcessFixture::named("preflight_reports_planned_icmp_effects_and_their_cidr_grant");
    let document = fixture.write_file("cwd/doc.md", DOCUMENT);

    fixture
        .command()
        .args(["compose", "--shell", "--allow-host", "10.77.0.0/16"])
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("ICMP probes discovered: 2"))
        .stdout(predicate::str::contains("| ping | 10.77.1.5 | 250 | 1 | true |"))
        .stdout(predicate::str::contains("| ping_under | 10.99.1.5 | 20 | 4 | false |"));
}

/// AC6: composing the same document denies only the ungranted target, with one
/// warning naming it, and resolves the call to nothing rather than leaving the
/// `{{ … }}` behind.
#[test]
fn an_ungranted_target_is_denied_with_one_warning() {
    let fixture = CliProcessFixture::named("an_ungranted_target_is_denied_with_one_warning");
    // Grant a range containing neither target so neither probe can send.
    let document = fixture.write_file("cwd/doc.md", "outside={{ ping(\"10.99.1.5\") }}\n");

    fixture
        .command()
        .args(["compose", "--allow-host", "10.77.0.0/16"])
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("{{").not())
        .stderr(predicate::str::contains("10.99.1.5"))
        .stderr(predicate::str::contains("--allow-host"));
}

/// AC33: a CIDR grant is ICMP-only. The same entry does not put any address in
/// its range on the HTTP allowlist, so a remote read inside it is still denied
/// by policy — no connection is attempted.
#[test]
fn a_cidr_grant_does_not_authorize_http_to_that_range() {
    let fixture = CliProcessFixture::named("a_cidr_grant_does_not_authorize_http_to_that_range");
    let document = fixture.write_file("cwd/doc.md", "::file https://10.77.1.5/remote.md\n");

    fixture
        .command()
        .args(["compose", "--allow-host", "10.77.0.0/16"])
        .arg(&document)
        .assert()
        .failure()
        .stderr(predicate::str::contains("10.77.1.5"));
}
