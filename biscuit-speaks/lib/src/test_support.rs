//! Gating helpers shared by biscuit-speaks' real-resource (`real_*`) unit tests.
//!
//! A `real_*` test drives a genuine synthesis backend, so it must skip on a host
//! that does not have that backend installed. A skip is not evidence, so a
//! provisioned host or a CI leg needs a way to turn one into a failure. Two
//! switches do that and they compose:
//!
//! - `PLAYA_REAL_AUDIO_REQUIRED=1` — the repository-wide, all-or-nothing switch
//!   already honored by `lib/tests/real_detached_phase4.rs` and
//!   `playa/lib/tests/real_playback_reports.rs`. It requires *every* real audio
//!   resource in the run.
//! - `BISCUIT_SPEAKS_REQUIRED_PROVIDERS=echogarden,gtts` — names the individual
//!   providers whose absence must be fatal while every other provider still
//!   skips cleanly.
//!
//! The pair mirrors `BISCUIT_TEST_LEVEL_REQUIRED` and
//! `BISCUIT_TEST_REQUIRED_BACKENDS` in `test_toolkit`, and for the same reason:
//! a runner that can host `echogarden` may have no route to Google's TTS
//! endpoint, and an all-or-nothing switch would force it to demand both or
//! neither.

use std::fmt::Display;

/// Environment variable naming the providers whose absence must be a hard
/// failure rather than a clean skip.
///
/// Comma-separated, case-insensitive, whitespace around each entry ignored.
/// Unset or all-whitespace means "no provider is required". Entries are matched
/// exactly against [`KNOWN_REAL_PROVIDERS`]; an unrecognized entry panics rather
/// than being silently treated as "not required", because a typo would
/// otherwise disable the very switch it was meant to enable.
pub(crate) const REQUIRED_PROVIDERS_VAR: &str = "BISCUIT_SPEAKS_REQUIRED_PROVIDERS";

/// Playa's repository-wide switch: any real audio resource is required.
const GLOBAL_REQUIRED_VAR: &str = "PLAYA_REAL_AUDIO_REQUIRED";

/// Playa's dry-run switch. Set means no real playback may occur.
///
/// Only the playback tests consult it, and those exist only with `playa`.
#[cfg(feature = "playa")]
const DRY_RUN_VAR: &str = "PLAYA_DRY_RUN";

/// Identifier for the EchoGarden provider.
pub(crate) const ECHOGARDEN: &str = "echogarden";

/// Identifier for the gTTS provider.
pub(crate) const GTTS: &str = "gtts";

/// Every identifier [`REQUIRED_PROVIDERS_VAR`] accepts.
///
/// Add an identifier here when a provider gains its first `real_*` test; the
/// list is deliberately not the whole `HostTtsProvider` enum, so that naming a
/// provider with no real coverage is reported as the mistake it is.
pub(crate) const KNOWN_REAL_PROVIDERS: &[&str] = &[ECHOGARDEN, GTTS];

/// Whether a real-resource test for `provider` must fail instead of skipping.
pub(crate) fn real_provider_required(provider: &str) -> bool {
    if std::env::var(GLOBAL_REQUIRED_VAR).as_deref() == Ok("1") {
        return true;
    }
    required_by_list(
        std::env::var(REQUIRED_PROVIDERS_VAR).ok().as_deref(),
        provider,
    )
}

/// Whether Playa's dry-run switch is set, in any of the spellings Playa accepts.
#[cfg(feature = "playa")]
pub(crate) fn dry_run_enabled() -> bool {
    matches!(
        std::env::var(DRY_RUN_VAR).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

/// Record a skip for `provider`, or fail when that provider is required.
///
/// Every skip branch of a `real_*` test goes through this function, so a host
/// that declares a provider available can never quietly report a skip as a pass.
///
/// ## Panics
///
/// When `provider` is required by either switch, or when
/// [`REQUIRED_PROVIDERS_VAR`] names an unknown provider.
pub(crate) fn skip_or_require(provider: &str, reason: impl Display) {
    assert!(
        !real_provider_required(provider),
        "{provider}: real-resource coverage is required on this host, but {reason}"
    );
    eprintln!("SKIP: {provider}: {reason}");
}

/// Membership test for one parsed [`REQUIRED_PROVIDERS_VAR`] value.
///
/// Split out from [`real_provider_required`] so the parsing rules are testable
/// without mutating process-global environment.
fn required_by_list(raw: Option<&str>, provider: &str) -> bool {
    let Some(raw) = raw else {
        return false;
    };
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_ascii_lowercase)
        .map(|entry| {
            assert!(
                KNOWN_REAL_PROVIDERS.contains(&entry.as_str()),
                "{REQUIRED_PROVIDERS_VAR}=\"{raw}\" names unknown provider \"{entry}\"; \
                 known providers: {}",
                KNOWN_REAL_PROVIDERS.join(", ")
            );
            entry
        })
        .any(|entry| entry == provider)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_and_blank_lists_require_nothing() {
        assert!(!required_by_list(None, ECHOGARDEN));
        assert!(!required_by_list(Some(""), ECHOGARDEN));
        assert!(!required_by_list(Some("  , \t "), ECHOGARDEN));
    }

    #[test]
    fn a_named_provider_is_required_and_others_still_skip() {
        assert!(required_by_list(Some("echogarden"), ECHOGARDEN));
        assert!(!required_by_list(Some("echogarden"), GTTS));
        assert!(required_by_list(Some("echogarden,gtts"), GTTS));
    }

    #[test]
    fn entries_tolerate_whitespace_and_case() {
        assert!(required_by_list(Some(" EchoGarden , GTTS "), GTTS));
    }

    #[test]
    #[should_panic(expected = "names unknown provider \"echogrden\"")]
    fn a_misspelled_provider_is_an_error_not_a_near_miss() {
        required_by_list(Some("echogrden"), ECHOGARDEN);
    }
}
