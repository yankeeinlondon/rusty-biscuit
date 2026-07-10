//! Sidecar↔Rust enum mirror test for the signals research topic.
//!
//! `docs/research/signals/_schema.yaml` embeds the signal-taxonomy member
//! lists inside its `records`/`extractions` quoted inline-object type
//! strings; the canonical definitions live in `claudine-catalog-types`
//! (`signal.rs` / `vocab.rs`). Per
//! `features/2026-07-02-provider-metadata/design/signal-detection.md`
//! §"Shared vocab enums", this test mechanically closes three-copy drift:
//! each sidecar `enum(...)` member list must equal the corresponding
//! `VARIANTS` slice exactly (same members, same order).

use std::path::Path;

use claudine_catalog_types::{
    Confidence, DetectionMode, MatchOp, SignalKind, SignalSource, Unit, Zone,
};
use strum::VariantNames;

/// The claudine package-area root (parent of this crate's manifest dir).
fn area() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen crate lives under the claudine package area")
}

/// The quoted inline-object type string for a top-level sidecar list field
/// (`records` or `extractions`).
fn type_string<'a>(sidecar: &'a str, list: &str) -> &'a str {
    let needle = format!("{list}: \"");
    let start = sidecar
        .find(&needle)
        .unwrap_or_else(|| panic!("sidecar has no `{list}: \"...\"` type string"))
        + needle.len();
    let len = sidecar[start..]
        .find('"')
        .unwrap_or_else(|| panic!("unterminated `{list}` type string in sidecar"));
    &sidecar[start..start + len]
}

/// The member list of `<field>: enum(...)` inside one inline-object type
/// string, with any trailing `; required` marker stripped.
fn enum_members<'a>(type_string: &'a str, list: &str, field: &str) -> Vec<&'a str> {
    let needle = format!("{field}: enum(");
    let start = type_string
        .find(&needle)
        .unwrap_or_else(|| panic!("`{list}` type string has no `{field}: enum(...)` block"))
        + needle.len();
    let len = type_string[start..]
        .find(')')
        .unwrap_or_else(|| panic!("unclosed `{field}: enum(` block in `{list}` type string"));
    let inner = type_string[start..start + len]
        .split(';')
        .next()
        .expect("split yields at least one segment");
    let members: Vec<&str> = inner.split(',').map(str::trim).collect();
    assert!(
        members.iter().all(|member| !member.is_empty()),
        "empty member in `{list}.{field}` enum block: {members:?}"
    );
    members
}

#[test]
fn signals_sidecar_enums_mirror_rust_variants() {
    let path = area().join("docs/research/signals/_schema.yaml");
    let sidecar = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

    let records = type_string(&sidecar, "records");
    let extractions = type_string(&sidecar, "extractions");

    let cases: [(&str, &str, &str, &[&str]); 7] = [
        ("records", records, "signal", SignalKind::VARIANTS),
        ("records", records, "source", SignalSource::VARIANTS),
        ("records", records, "match_op", MatchOp::VARIANTS),
        ("records", records, "detection", DetectionMode::VARIANTS),
        ("records", records, "confidence", Confidence::VARIANTS),
        ("extractions", extractions, "unit", Unit::VARIANTS),
        ("extractions", extractions, "zone", Zone::VARIANTS),
    ];

    for (list, section, field, expected) in cases {
        let sidecar_members = enum_members(section, list, field);
        assert_eq!(
            sidecar_members, expected,
            "sidecar `{list}.{field}` enum drifted from Rust VARIANTS\n  \
             sidecar: {sidecar_members:?}\n  rust:    {expected:?}"
        );
    }
}
