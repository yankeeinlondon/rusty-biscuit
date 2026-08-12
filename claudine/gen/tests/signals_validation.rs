//! Signals compilation gates over doctored fixture corpora.
//!
//! Each test copies the REAL signals corpus (all nine docs + the sidecar)
//! into a tempdir area, doctors the piece under test, and drives the same
//! [`claudine_gen::build_signals`] path the CLI uses — malformed input must
//! be a loud typed error naming the doc (and record), never a silent skip.
//! Evidence-file existence is deliberately unchecked here (deferred to the
//! fixture-replay `signals check`), so the fixtures/ tree is not copied.

use std::fs;
use std::path::{Path, PathBuf};

use claudine_gen::{GenError, SIGNAL_SLUGS, build_signals};

/// The real claudine package-area root.
fn real_area() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen crate lives under the claudine package area")
}

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    /// Copies the nine signals docs plus the sidecar schema into a fresh
    /// tempdir area.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let real = real_area();
        let copy = |rel: &str| {
            let to = dir.path().join(rel);
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::copy(real.join(rel), to)
                .unwrap_or_else(|err| panic!("fixture copy of `{rel}` failed: {err}"));
        };
        copy("docs/research/signals/_schema.yaml");
        for slug in SIGNAL_SLUGS {
            copy(&format!("docs/research/signals/{slug}.md"));
        }
        Self { dir }
    }

    fn replace(&self, slug: &str, from: &str, to: &str) {
        let path: PathBuf = self
            .dir
            .path()
            .join(format!("docs/research/signals/{slug}.md"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.contains(from),
            "fixture replace: `{from}` not found in {slug}.md"
        );
        fs::write(path, content.replace(from, to)).unwrap();
    }

    fn build(&self) -> Result<String, GenError> {
        build_signals(self.dir.path())
    }
}

/// Untampered corpus compiles, twice, byte-identically (the determinism
/// contract behind the drift test).
#[test]
fn real_corpus_builds_deterministically() {
    let fixture = Fixture::new();
    let first = fixture.build().expect("real corpus must compile");
    let second = fixture.build().expect("real corpus must compile twice");
    assert_eq!(first, second, "build_signals must be byte-deterministic");
    assert!(first.ends_with("];\n") && !first.ends_with("\n\n"));
    // Every provider table plus the roster are present.
    for slug in SIGNAL_SLUGS {
        assert!(first.contains(&format!("static {}_SIGNALS", slug.to_uppercase())));
    }
}

/// A wildcard breaks the restricted JSONPath subset — error names the doc,
/// the record, and the offending path.
#[test]
fn wildcard_match_path_is_rejected() {
    let fixture = Fixture::new();
    fixture.replace(
        "claude",
        "match_path: rate_limit_info.rateLimitType",
        "match_path: \"rate_limit_info[*].rateLimitType\"",
    );
    match fixture.build().unwrap_err() {
        GenError::SignalRecordInvalid {
            path,
            record,
            message,
        } => {
            assert!(path.ends_with("claude.md"), "{}", path.display());
            // First cap record in document order carries the doctored path.
            assert_eq!(record, "stream-usage_cap_approaching-usage");
            assert!(message.contains("rate_limit_info[*].rateLimitType"), "{message}");
            assert!(message.contains("not numeric"), "{message}");
        }
        other => panic!("expected SignalRecordInvalid, got: {other}"),
    }
}

/// `op: regex` values must compile with the regex crate at generate time.
#[test]
fn non_compiling_regex_is_rejected() {
    let fixture = Fixture::new();
    fixture.replace("kimi", "match_value: \"^-32004$\"", "match_value: \"^(\"");
    match fixture.build().unwrap_err() {
        GenError::SignalRecordInvalid {
            path, message, ..
        } => {
            assert!(path.ends_with("kimi.md"), "{}", path.display());
            assert!(message.contains("does not compile"), "{message}");
        }
        other => panic!("expected SignalRecordInvalid, got: {other}"),
    }
}

/// Two records in one provider×source group may not share a priority.
#[test]
fn duplicate_priority_in_a_source_group_is_rejected() {
    let fixture = Fixture::new();
    // claude stream cap priorities are 10, 11, 12, ... — collide 11 into 10.
    fixture.replace("claude", "priority: 11", "priority: 10");
    match fixture.build().unwrap_err() {
        GenError::SignalRecordInvalid {
            record, message, ..
        } => {
            assert_eq!(record, "stream-usage_cap_approaching-seven_day");
            assert!(
                message.contains("duplicates record `stream-usage_cap_approaching-usage`"),
                "{message}"
            );
            assert!(message.contains("`stream` source group"), "{message}");
        }
        other => panic!("expected SignalRecordInvalid, got: {other}"),
    }
}

/// Every extractions[].record must join to an existing records[].id.
#[test]
fn dangling_extraction_reference_is_rejected() {
    let fixture = Fixture::new();
    fixture.replace(
        "claude",
        "- record: stream-usage_cap_approaching-usage",
        "- record: stream-usage_cap_approaching-vanished",
    );
    match fixture.build().unwrap_err() {
        GenError::SignalDocInvalid { path, message } => {
            assert!(path.ends_with("claude.md"), "{}", path.display());
            assert!(
                message.contains("stream-usage_cap_approaching-vanished"),
                "{message}"
            );
            assert!(message.contains("does not exist"), "{message}");
        }
        other => panic!("expected SignalDocInvalid, got: {other}"),
    }
}

/// `exists` carries no match terms; a stray `match_value` is an error.
#[test]
fn exists_with_match_value_is_rejected() {
    let fixture = Fixture::new();
    fixture.replace(
        "claude",
        "match_path: apiKeySource\n    match_op: exists\n",
        "match_path: apiKeySource\n    match_op: exists\n    match_value: ANTHROPIC_API_KEY\n",
    );
    match fixture.build().unwrap_err() {
        GenError::SignalRecordInvalid {
            record, message, ..
        } => {
            assert_eq!(record, "stream-auth_kind_detected-init");
            assert!(
                message.contains("carries no `match_value`"),
                "{message}"
            );
        }
        other => panic!("expected SignalRecordInvalid, got: {other}"),
    }
}

/// An unknown enum member fails loudly. The sidecar schema is the first
/// gate (darkmatter validation → ResearchInvalid); the in-module strum
/// parse is the second line of defense against sidecar↔Rust drift.
#[test]
fn unknown_signal_member_is_rejected() {
    let fixture = Fixture::new();
    fixture.replace(
        "claude",
        "signal: usage_cap_approaching",
        "signal: usage_cap_telepathy",
    );
    match fixture.build().unwrap_err() {
        GenError::ResearchInvalid { path, problems } => {
            assert!(path.ends_with("claude.md"), "{}", path.display());
            assert!(problems.contains("usage_cap_telepathy"), "{problems}");
        }
        other => panic!("expected ResearchInvalid, got: {other}"),
    }
}

/// Priorities are u16 — an out-of-range YAML number is an error, not a
/// wrap.
#[test]
fn out_of_range_priority_is_rejected() {
    let fixture = Fixture::new();
    fixture.replace("claude", "priority: 110", "priority: 70000");
    match fixture.build().unwrap_err() {
        GenError::SignalRecordInvalid {
            record, message, ..
        } => {
            assert_eq!(record, "stream-session_tainted-result-error");
            assert!(message.contains("not a u16"), "{message}");
        }
        other => panic!("expected SignalRecordInvalid, got: {other}"),
    }
}

/// A declarative record without match terms is malformed.
#[test]
fn declarative_record_without_match_op_is_rejected() {
    let fixture = Fixture::new();
    fixture.replace(
        "claude",
        "match_path: is_throttled\n    match_op: eq\n    match_value: \"true\"\n",
        "match_path: is_throttled\n",
    );
    match fixture.build().unwrap_err() {
        GenError::SignalRecordInvalid {
            record, message, ..
        } => {
            assert_eq!(record, "stream-rate_limited-throttled");
            assert!(
                message.contains("require both `match_path` and `match_op`"),
                "{message}"
            );
        }
        other => panic!("expected SignalRecordInvalid, got: {other}"),
    }
}
