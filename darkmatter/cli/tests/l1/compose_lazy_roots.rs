//! The reserved `current` / `current_env` roots through the normal
//! `md compose` path (AC27, AC32).
//!
//! `md compose` owns its context, so it installs the anchored ambient refresh:
//! a `current.<key>` read re-observes the key's group at the directory the
//! request captured, never at the process CWD at reference time. These cases
//! pin the spelling and the clean break; the mutation and memo-scope contracts
//! are proven with controlled in-process providers in the library suite.

use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;

/// A date/time key is the one fact a hermetic fixture can compare without a
/// repository: the eager snapshot and the lazy refresh observe the same day.
#[test]
fn a_lazy_root_resolves_through_md_compose() {
    let fixture = CliProcessFixture::named("a_lazy_root_resolves_through_md_compose");
    let document = fixture.write_file(
        "cwd/doc.md",
        "eager={{ ctx.today }} lazy={{ current.today }} same={{ ctx.today == current.today }}\n",
    );

    fixture
        .command()
        .arg("compose")
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("same=true"))
        .stdout(predicate::str::contains("{{").not());
}

/// `current_env.<KEY>` reads the live process environment, so a value handed to
/// the child reaches the composed document through the reserved root.
#[test]
fn current_env_reads_the_composing_process_environment() {
    let fixture = CliProcessFixture::named("current_env_reads_the_composing_process_environment");
    let document = fixture.write_file(
        "cwd/doc.md",
        "live=[{{ current_env.LAZY_ROOT_PROBE_VALUE }}] missing=[{{ current_env.LAZY_ROOT_PROBE_ABSENT }}]\n",
    );

    fixture
        .command()
        .env("LAZY_ROOT_PROBE_VALUE", "supplied")
        .arg("compose")
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("live=[supplied] missing=[]"));
}

/// R33's clean break, end to end: the removed nesting is an unknown path, not
/// an alias for `ctx`, and it fails the compose instead of rendering empty.
#[test]
fn the_removed_nesting_fails_the_compose() {
    let fixture = CliProcessFixture::named("the_removed_nesting_fails_the_compose");
    let document = fixture.write_file("cwd/doc.md", "today={{ current.ctx.today }}\n");

    fixture
        .command()
        .arg("compose")
        .arg(&document)
        .assert()
        .failure()
        .stderr(predicate::str::contains("current.ctx.today"))
        .stderr(predicate::str::contains("is not a member of the reserved"));
}
