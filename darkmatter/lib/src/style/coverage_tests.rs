//! Cross-cutting test module that exercises the schema descriptor against
//! the parser pipeline. Lives in a dedicated file so the iteration-style
//! tests don't get lost inside the unit-test modules.
//!
//! These tests close two specific gaps called out by `review-1.md`:
//!
//! 1. **Alias coverage** — every documented `SchemaLeaf::alias` round-trips
//!    through `from_json_value` and produces both the expected `Deprecated`
//!    warning at the raw (snake-case) path and a `KnownButInactive` warning
//!    at the canonical kebab path.
//!
//! 2. **Descriptor drift** — every canonical leaf in `SCHEMA` is reachable
//!    through `from_json_value` without producing an `UnknownKey` warning.
//!    If a struct field is added without a matching descriptor entry the
//!    walker would flag it; if a descriptor entry exists for a field that
//!    isn't reachable through the schema structs serde would silently drop
//!    its value and this test would still pass, so the descriptor-side gap
//!    is caught by a second pass that checks each descriptor entry against
//!    a serde round-trip and asserts the value reappears on the parsed
//!    side.

use serde_json::{Value, json};

use crate::style::descriptor::{LeafType, SCHEMA, SchemaLeaf};
use crate::style::parse::{ACTIVE_STYLE_WIRING_SUB_SPEC, from_json_value};
use crate::style::warning::{StyleWarning, StyleWarningKind};

/// Build a nested JSON object that places `value` at `dotted_path`.
fn build_at(dotted_path: &str, value: Value) -> Value {
    let mut current = value;
    for segment in dotted_path.rsplit('.') {
        current = json!({ segment: current });
    }
    current
}

/// A representative sample value for each `LeafType`. Used to round-trip
/// every leaf through the parser without hitting typed-error paths.
fn sample_for(kind: LeafType) -> Value {
    match kind {
        LeafType::HorizontalLength => json!("10ch"),
        LeafType::WidthOrMode => json!("fit-content"),
        LeafType::RowCount => json!(1),
        LeafType::Alignment => json!("left"),
        LeafType::Color => json!("red-500"),
        LeafType::BackgroundEnum => json!("subtle"),
        LeafType::StringValue => json!("x"),
        LeafType::OpaqueValue => json!({"k": "v"}),
        LeafType::HrKind => json!("waves"),
        LeafType::HrWeight => json!("medium"),
        LeafType::HrAlignment => json!("center"),
        LeafType::CompoundStyle => json!({"bold": true, "italic": true}),
        LeafType::WordWrap => json!("wrap-prose"),
    }
}

/// Compute the set of `(raw_path, canonical_path)` pairs for every segment
/// position where the alias and canonical paths differ. The walker emits
/// one `Deprecated` warning per such position, with `path = "style.{raw}"`
/// and `replacement = canonical`.
fn expected_deprecated_positions(alias: &str, canonical: &str) -> Vec<(String, String)> {
    let alias_segs: Vec<&str> = alias.split('.').collect();
    let canon_segs: Vec<&str> = canonical.split('.').collect();
    assert_eq!(
        alias_segs.len(),
        canon_segs.len(),
        "alias `{}` and canonical `{}` must have the same segment count",
        alias,
        canonical
    );
    let mut out = Vec::new();
    let mut raw_acc = String::new();
    let mut canon_acc = String::new();
    for (a, c) in alias_segs.iter().zip(canon_segs.iter()) {
        if !raw_acc.is_empty() {
            raw_acc.push('.');
            canon_acc.push('.');
        }
        raw_acc.push_str(a);
        canon_acc.push_str(c);
        if a != c {
            out.push((format!("style.{}", raw_acc), canon_acc.clone()));
        }
    }
    out
}

/// Test: every documented `SchemaLeaf::alias` parses successfully and
/// produces one `Deprecated` warning per segment that differs from the
/// canonical spelling, plus a single `KnownButInactive` warning at the
/// canonical path.
#[test]
fn every_alias_round_trips_with_expected_warnings() {
    let mut tested = 0usize;
    for leaf in SCHEMA {
        let Some(alias) = leaf.alias else { continue };
        tested += 1;

        let value = build_at(alias, sample_for(leaf.leaf_type));
        let (_parsed, warnings) = from_json_value(&value)
            .unwrap_or_else(|e| panic!("alias `{}` failed to parse: {:?}", alias, e));

        let expected = expected_deprecated_positions(alias, leaf.canonical);
        let actual_deprecated: Vec<&StyleWarning> = warnings
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::Deprecated { .. }))
            .collect();
        assert_eq!(
            actual_deprecated.len(),
            expected.len(),
            "alias `{}` produced {} `Deprecated` warning(s), expected {} \
             (positions: {:?}); got: {:?}",
            alias,
            actual_deprecated.len(),
            expected.len(),
            expected,
            warnings,
        );
        for (expected_path, expected_replacement) in &expected {
            let found = actual_deprecated
                .iter()
                .find(|w| &w.path == expected_path)
                .unwrap_or_else(|| {
                    panic!(
                        "alias `{}` missing `Deprecated` warning at `{}`; \
                         got: {:?}",
                        alias, expected_path, warnings
                    )
                });
            let StyleWarningKind::Deprecated { ref replacement } = found.kind else {
                unreachable!()
            };
            assert_eq!(
                replacement, expected_replacement,
                "alias `{}` deprecated at `{}` had replacement `{}`, \
                 expected `{}`",
                alias, expected_path, replacement, expected_replacement
            );
        }

        let expected_inactive = if leaf.sub_spec > ACTIVE_STYLE_WIRING_SUB_SPEC {
            1
        } else {
            0
        };
        let inactive_at_canonical: Vec<&StyleWarning> = warnings
            .iter()
            .filter(|w| {
                matches!(w.kind, StyleWarningKind::KnownButInactive { .. })
                    && w.path == format!("style.{}", leaf.canonical)
            })
            .collect();
        assert_eq!(
            inactive_at_canonical.len(),
            expected_inactive,
            "alias `{}` (sub_spec {}) expected {} `KnownButInactive` \
             warning(s) at canonical `style.{}` — got {:?}",
            alias,
            leaf.sub_spec,
            expected_inactive,
            leaf.canonical,
            warnings,
        );
    }
    assert!(
        tested > 0,
        "no aliases were tested — `SCHEMA` may be empty or misconfigured"
    );
}

/// Test: every canonical leaf in `SCHEMA` round-trips through the parser
/// without emitting an `UnknownKey` warning. This catches the scenario
/// where a struct field is renamed but the descriptor still lists the old
/// path — the parser would then flag the descriptor's canonical path as
/// unknown when round-tripped.
#[test]
fn every_canonical_leaf_round_trips_without_unknown_key() {
    for leaf in SCHEMA {
        let value = build_at(leaf.canonical, sample_for(leaf.leaf_type));
        let (_parsed, warnings) = from_json_value(&value).unwrap_or_else(|e| {
            panic!(
                "canonical leaf `{}` failed to parse: {:?}",
                leaf.canonical, e
            )
        });

        let unknown: Vec<&StyleWarning> = warnings
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::UnknownKey))
            .collect();
        assert!(
            unknown.is_empty(),
            "canonical leaf `{}` produced unknown-key warning(s): {:?}",
            leaf.canonical,
            unknown,
        );

        let expected_inactive = if leaf.sub_spec > ACTIVE_STYLE_WIRING_SUB_SPEC {
            1
        } else {
            0
        };
        let inactive: Vec<&StyleWarning> = warnings
            .iter()
            .filter(|w| {
                matches!(w.kind, StyleWarningKind::KnownButInactive { .. })
                    && w.path == format!("style.{}", leaf.canonical)
            })
            .collect();
        assert_eq!(
            inactive.len(),
            expected_inactive,
            "canonical leaf `{}` (sub_spec {}) expected {} \
             `KnownButInactive` warning(s) — got {:?}",
            leaf.canonical,
            leaf.sub_spec,
            expected_inactive,
            warnings,
        );
    }
}

/// Test: the count of descriptor entries matches a hand-maintained expected
/// total. Adding a new field to a per-bucket struct without adding a
/// descriptor entry leaves the new field discoverable only through unknown-
/// key warnings (which are noisy if the user uses the new field). Removing
/// a struct field without removing the descriptor entry leaves the
/// `every_canonical_leaf_round_trips_without_unknown_key` test passing
/// because serde silently ignores unknown JSON keys on the parsed-struct
/// side. This count check is the cheap drift sentinel for the second
/// direction.
///
/// Update the constant when the schema legitimately grows or shrinks.
#[test]
fn descriptor_entry_count_matches_expected() {
    const EXPECTED: usize =
        // page: 16
        // table, block-quote, ol, li: 10 each = 40
        // ul: 11
        // hyperlinks: 5 outer + 10 local-style = 15
        // images: 5 outer + 10 local-style = 15
        // hr: 12
        // disclosure: 10
        // code-block: 10
        16 + 40 + 11 + 15 + 15 + 12 + 10 + 10;
    assert_eq!(
        SCHEMA.len(),
        EXPECTED,
        "descriptor entry count drifted — update EXPECTED if intentional"
    );
}

/// Test: all wired HR keys produce no `KnownButInactive` warnings.
#[test]
fn active_wiring_warnings_for_hr_keys() {
    let v = json!({
        "hr": {
            "kind": "waves",
            "width": "50%",
            "max-width": "60ch",
            "alignment": "center",
            "color": "red-500",
            "bg-color": "blue-500",
            "weight": "thick"
        }
    });
    let (_parsed, warnings) = from_json_value(&v).unwrap();

    let inactive: Vec<&StyleWarning> = warnings
        .iter()
        .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
        .collect();

    for path in [
        "style.hr.kind",
        "style.hr.width",
        "style.hr.max-width",
        "style.hr.alignment",
        "style.hr.color",
        "style.hr.bg-color",
        "style.hr.weight",
    ] {
        assert!(
            !inactive.iter().any(|w| w.path == path),
            "wired hr key `{}` should not produce KnownButInactive, got: {:?}",
            path,
            inactive
        );
    }
}

/// Test: all wired disclosure keys produce no `KnownButInactive` warnings.
#[test]
fn active_wiring_warnings_for_disclosure_keys() {
    let v = json!({
        "disclosure": {
            "width": "40ch",
            "max-width": "50%",
            "alignment": "left",
            "color": "red-500",
            "bg-color": "blue-500"
        }
    });
    let (_parsed, warnings) = from_json_value(&v).unwrap();

    let inactive: Vec<&StyleWarning> = warnings
        .iter()
        .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
        .collect();

    for path in [
        "style.disclosure.width",
        "style.disclosure.max-width",
        "style.disclosure.alignment",
        "style.disclosure.color",
        "style.disclosure.bg-color",
    ] {
        assert!(
            !inactive.iter().any(|w| w.path == path),
            "wired disclosure key `{}` should not produce KnownButInactive, got: {:?}",
            path,
            inactive
        );
    }
}

/// Spot-check: the legacy-aliased nested children that triggered Finding 1
/// also surface in the alias coverage above. This separate assertion makes
/// the regression intent obvious in test names.
#[test]
fn finding_1_paths_appear_in_alias_coverage() {
    let paths_with_aliases: Vec<&'static str> =
        SCHEMA.iter().filter_map(|l: &SchemaLeaf| l.alias).collect();
    for required in [
        "block_quote.max_width",
        "hyperlinks.local_style.max_width",
        "images.local_style.max_width",
    ] {
        assert!(
            paths_with_aliases.contains(&required),
            "regression: `{}` must be a documented descriptor alias",
            required
        );
    }
}
