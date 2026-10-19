//! Claudine's launch and completion gates follow Darkmatter's effective
//! presence end to end (acceptance criterion 12 of
//! `2026-10-16-core-schema-types`): through a real `claudine compose`, a
//! null-admitting whole value (`null`, `unknown`, a union with a `null` arm)
//! is optional whatever its `required` marker, while a concrete `required`
//! property and a required list of `unknown` items stay required. A value
//! that fails a nullable union's concrete arm is handled as any invalid
//! optional value is (dropped with a warning).
//!
//! Each row launches a stub provider that records that it ran, so the row
//! shows which gate decided: a launch-gate failure never starts the provider;
//! a completion-gate failure starts it and then fails the run. Darkmatter's
//! own surfaces are covered by its `schemas_effective_presence` suite.

use std::path::Path;

use crate::common;
use common::{CliProcessFixture, write};

const SCHEMA: &str = concat!(
    "$schema:\n",
    "  a: 'null(required)'\n",
    "  b: 'unknown(required)'\n",
    "  c: ['string(required; min(3))', 'null']\n",
    "  d: 'string(required)'\n",
    "  e: 'unknown[](required)'\n",
);

#[cfg(unix)]
fn write_provider_stub(bin_dir: &Path) {
    common::write_executable(
        &bin_dir.join("goose"),
        "#!/bin/sh\n: > \"$PROVIDER_MARKER\"\nprintf 'done\\n'\nexit 0\n",
    );
}

// A built binary, not a `.cmd` shim: Rust refuses to pass the multi-line
// prompt argument to a batch file ("batch file arguments are invalid"). Its
// marker mode honors `PROVIDER_MARKER` as the Unix stub does.
#[cfg(windows)]
fn write_provider_stub(bin_dir: &Path) {
    std::fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        bin_dir.join("goose.exe"),
    )
    .unwrap();
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// Both gates passed and the run succeeded.
    Succeeded,
    /// The launch gate passed, the provider ran, and completion failed.
    FailedAtCompletion,
    /// The launch gate failed; the provider never started.
    FailedAtLaunch,
}

fn compose(values: &str) -> (Outcome, String) {
    let fixture = CliProcessFixture::named("compose-presence-parity");
    write_provider_stub(fixture.bin_dir());
    let marker = fixture.cwd().join("provider-ran");
    let document = fixture.cwd().join("prompt.md");
    write(&document, &format!("---\n{SCHEMA}{values}---\nBody.\n"));

    let output = fixture
        .command()
        .env("PROVIDER_MARKER", &marker)
        .args(["compose", "--goose"])
        .arg(&document)
        .output()
        .expect("claudine runs");
    let stderr = common::strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(!fixture.audio_spool().exists(), "no lifecycle audio is published");
    let outcome = match (output.status.success(), marker.exists()) {
        (true, true) => Outcome::Succeeded,
        (false, true) => Outcome::FailedAtCompletion,
        (false, false) => Outcome::FailedAtLaunch,
        (true, false) => panic!("the run succeeded without starting the provider:\n{stderr}"),
    };
    (outcome, stderr)
}

/// One test per row: each spawns one `claudine compose`.
macro_rules! presence_rows {
    ($($name:ident: $values:expr => $expected:expr, $property:expr;)*) => {$(
        #[test]
        fn $name() {
            let (outcome, stderr) = compose($values);
            assert_eq!(outcome, $expected, "stderr:\n{stderr}");
            if let Some(property) = $property {
                assert!(stderr.contains(property), "stderr lacks `{property}`:\n{stderr}");
            }
        }
    )*};
}

/// A `start` effect that rewrites values between the two gates, so the
/// completion gate judges what the launch gate never saw.
const CLEAR_AT_START: &str = "start:\n    - {set: {a: null, b: null, c: null}}\n";

presence_rows! {
    // Control row: every concrete requirement met, every nullable omitted.
    null_admitting_properties_may_be_omitted:
        "d: present\ne: []\n" => Outcome::Succeeded, None::<&str>;
    null_admitting_properties_may_be_explicitly_null:
        "a: null\nb: null\nc: null\nd: present\ne: [1, x, null]\n" => Outcome::Succeeded, None::<&str>;
    // Direct `compose` has no agent to supply a property, so its launch gate
    // already demands every effectively required one.
    an_omitted_required_string_fails_before_launch:
        "e: []\n" => Outcome::FailedAtLaunch, Some("missing properties");
    an_omitted_required_unknown_list_fails_before_launch:
        "d: present\n" => Outcome::FailedAtLaunch, Some("missing properties");
    nullables_cleared_by_start_pass_completion:
        &format!("a: 1\nb: x\nc: abc\nd: present\ne: []\n{CLEAR_AT_START}") => Outcome::Succeeded, None::<&str>;
    a_required_string_cleared_by_start_fails_completion:
        "d: present\ne: []\nstart:\n    - {set: {d: null}}\n" => Outcome::FailedAtCompletion, Some("completion schema");
    a_required_unknown_list_cleared_by_start_fails_completion:
        "d: present\ne: []\nstart:\n    - {set: {e: null}}\n" => Outcome::FailedAtCompletion, Some("completion schema");
    // Claudine's consumer policy for an invalid *optional* value: it is
    // dropped with a warning rather than failing the run. The nullable union
    // is optional, so `ab` (rejected by Darkmatter's validator) is dropped.
    a_short_value_in_a_nullable_union_is_dropped_as_optional:
        "c: ab\nd: present\ne: []\n" => Outcome::Succeeded, Some("dropped optional schema property `c`");
}
