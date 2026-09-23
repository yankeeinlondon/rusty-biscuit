//! The Darkmatter half of the invalid-frontmatter safety gate.
//!
//! `biscuit-file` owns the `serde_yaml_ng::Value`-equality half; this suite
//! owns the schema half. The unit tests in
//! `markdown/schemas/tests/clean_quoting.rs` pin the behavior on chosen
//! inputs — this suite proves the *contract* holds across generated ones.
//!
//! Two properties carry the safety argument:
//!
//! 1. [`schema_result_set_identical`] must see a type-changing edit as a
//!    difference. The raw, non-coercing half is what makes this work:
//!    coercion maps an authored `42` and an authored `"42"` onto the same
//!    problem set, so a coerced-only comparison would wave a type change
//!    through as "unchanged". A prior task made identity required on both
//!    halves; these tests are what stop that from silently regressing.
//!
//! 2. The auto-apply path may narrow a scalar's YAML type to string — that
//!    is the whole point of schema-proven quoting — but it must never change
//!    the scalar's *text*. Quoting `1.20` yields the string `1.20`, never
//!    `1.2`. Every other kind of type change must never auto-apply at all.

use biscuit_file::{YamlCertainty, apply_edit_set};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, SchemaCleanAnalysis, analyze_frontmatter,
    schema_result_set_identical,
};
use proptest::prelude::*;

// ── Helpers ──────────────────────────────────────────────────────────────

/// Resolves the effective schema a frontmatter block declares for itself.
fn effective_for(yaml_body: &str) -> EffectiveSchema {
    let content = format!("---\n{yaml_body}---\nbody\n");
    let md: Markdown = content.as_str().into();
    DarkmatterSchemas::new()
        .effective_for(&md)
        .expect("schema resolution must succeed")
        .expect("document must resolve an effective schema")
}

/// Applies every auto-apply-eligible repair the schema tier produced,
/// mirroring what `md clean` does in `frontmatter_repair.rs`.
fn auto_apply(analysis: &SchemaCleanAnalysis, yaml_source: &str) -> String {
    let repairs: Vec<biscuit_file::YamlRepair> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.classification.is_auto_apply_eligible())
        .flat_map(|diagnostic| diagnostic.repairs.iter().cloned())
        .collect();
    apply_edit_set(yaml_source, &repairs).source
}

/// The scalar at `key`, rendered as the text a reader would see: quotes
/// stripped, everything else verbatim. This is the projection the quoting
/// tier must preserve.
fn scalar_text(yaml: &str, key: &str) -> Option<String> {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).ok()?;
    let node = value.get(key)?;
    Some(match node {
        serde_yaml_ng::Value::String(text) => text.clone(),
        serde_yaml_ng::Value::Number(number) => number.to_string(),
        serde_yaml_ng::Value::Bool(flag) => flag.to_string(),
        _ => return None,
    })
}

// ── Property 1: the result-set check sees type changes ───────────────────

/// Scalars whose text form looks numeric or boolean. Some of these already
/// parse *as strings* — `007` keeps its leading zeros, `0x1F` is not a YAML
/// 1.2 integer — which is precisely why the type-sensitive properties below
/// have to test rather than assume it.
fn scalar_strategy() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec![
        "42", "1.20", "0", "true", "false", "null", "1e3", "0x1F", "007",
    ])
}

/// True when the scalar parses as a bool or a number — the S2 mismatch
/// family `schema_proven_quoting` recognizes.
///
/// Null is deliberately excluded: against a non-required `string` an absent
/// value is not a type error, so quoting `null` changes nothing the
/// validator can see.
fn is_non_string_scalar(scalar: &str) -> bool {
    matches!(
        serde_yaml_ng::from_str::<serde_yaml_ng::Value>(scalar),
        Ok(serde_yaml_ng::Value::Number(_) | serde_yaml_ng::Value::Bool(_))
    )
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        .. ProptestConfig::default()
    })]

    /// Quoting a scalar under a `string`-requiring schema always changes the
    /// raw problem set (one mismatch → none), so the S1 check must report a
    /// difference. If it ever returned `true` here, a parse-equivalence
    /// candidate could smuggle a type change past the gate.
    #[test]
    fn quoting_is_never_seen_as_schema_identical(scalar in scalar_strategy()) {
        // A scalar that already parses as a string satisfies the schema as
        // authored, so quoting it changes nothing and the check correctly
        // reports "identical". The property only bites on real mismatches.
        prop_assume!(is_non_string_scalar(scalar));

        let body = format!("$schema:\n  release: string\nrelease: {scalar}\n");
        let effective = effective_for(&body);
        let quoted = format!("$schema:\n  release: string\nrelease: \"{scalar}\"\n");
        prop_assert!(
            !schema_result_set_identical(&effective, &body, &quoted),
            "quoting {scalar:?} must register as a schema-result change"
        );
    }

    /// The check must be reflexive: a source is always identical to itself.
    /// A false negative here would block legitimate whitespace repairs.
    #[test]
    fn identical_sources_always_compare_identical(scalar in scalar_strategy()) {
        let body = format!("$schema:\n  release: string\nrelease: {scalar}\n");
        let effective = effective_for(&body);
        prop_assert!(schema_result_set_identical(&effective, &body, &body));
    }

    /// Whitespace-only edits are the S1 tier's whole purpose: they preserve
    /// the value, so they must preserve the schema result set too.
    #[test]
    fn whitespace_only_edits_compare_identical(scalar in scalar_strategy()) {
        let body = format!("$schema:\n  release: string\nrelease: {scalar}\n");
        let effective = effective_for(&body);
        let padded = format!("$schema:\n  release: string\nrelease:   {scalar}  \n");
        prop_assert!(
            schema_result_set_identical(&effective, &body, &padded),
            "padding whitespace around {scalar:?} must not change the result set"
        );
    }
}

/// The raw half is load-bearing. Under a `number` schema, an authored `42`
/// and an authored `"42"` both *coerce* to a valid number, so the coerced
/// problem sets are both empty and match. Only the raw comparison sees that
/// one was authored as a string. Deleting `validate_raw` from the check
/// would make this test fail — which is exactly why it exists.
#[test]
fn raw_half_catches_a_type_change_coercion_hides() {
    let body = "$schema:\n  count: number\ncount: 42\n";
    let effective = effective_for(body);
    let stringified = "$schema:\n  count: number\ncount: \"42\"\n";

    assert!(
        !schema_result_set_identical(&effective, body, stringified),
        "number → quoted-number must not compare identical; the raw half is what sees it"
    );
}

/// The coerced half is also load-bearing, for the mirror-image reason: two
/// candidates can be raw-identical (both string-typed against a `number`
/// schema) yet diverge under coercion, because only one is numeric text.
#[test]
fn coerced_half_catches_a_divergence_the_raw_half_misses() {
    let body = "$schema:\n  count: number\ncount: \"42\"\n";
    let effective = effective_for(body);
    let non_numeric = "$schema:\n  count: number\ncount: \"abc\"\n";

    assert!(
        !schema_result_set_identical(&effective, body, non_numeric),
        "\"42\" → \"abc\" must not compare identical; the coerced half is what sees it"
    );
}

/// Scalars that merely *look* numeric are already strings, so a
/// string-requiring schema is already satisfied and there is nothing to
/// repair. Quoting them anyway would be churn on documents that are correct.
///
/// This is the "Norway problem" family seen from the other side: the danger
/// is not only that `no` becomes `false`, but that a repair tier over-fires
/// on `007` and rewrites files that were never broken.
#[test]
fn string_lookalike_scalars_need_no_repair() {
    // `0x1F` is deliberately NOT here: serde_yaml_ng resolves it as a number,
    // so it belongs to the mismatch family rather than the lookalike one.
    for scalar in ["007", "1.2.3", "2026-07-18-invalid-frontmatter", "1.20.3"] {
        assert!(
            !is_non_string_scalar(scalar),
            "{scalar:?} is expected to parse as a string"
        );

        let body = format!("$schema:\n  release: string\nrelease: {scalar}\n");
        let effective = effective_for(&body);
        let analysis = analyze_frontmatter(&effective, &body);

        assert!(
            analysis.is_clean(),
            "{scalar:?} already satisfies the schema; got {:?}",
            analysis.diagnostics()
        );
        assert_eq!(
            auto_apply(&analysis, &body),
            body,
            "{scalar:?} must not be rewritten"
        );
    }
}

/// Any parse failure fails the proof outright — the gate is closed by
/// default rather than open.
#[test]
fn unparseable_either_side_fails_the_proof() {
    let body = "$schema:\n  title: string\ntitle: hi\n";
    let effective = effective_for(body);

    assert!(!schema_result_set_identical(&effective, body, "title: [unclosed\n"));
    assert!(!schema_result_set_identical(&effective, "title: [unclosed\n", body));
    assert!(!schema_result_set_identical(&effective, body, ""));
}

// ── Property 2: auto-apply never changes a scalar's text ─────────────────

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        .. ProptestConfig::default()
    })]

    /// The headline auto-apply guarantee. When the quoting tier fires, the
    /// scalar's text must survive byte-for-byte — `1.20` must not become
    /// `1.2`, `007` must not become `7`, `0x1F` must not become `31`.
    #[test]
    fn auto_apply_preserves_scalar_text(scalar in scalar_strategy()) {
        let body = format!("$schema:\n  release: string\nrelease: {scalar}\n");
        let effective = effective_for(&body);
        let analysis = analyze_frontmatter(&effective, &body);
        let applied = auto_apply(&analysis, &body);

        // Whether or not a repair fired, the text a reader sees at `release`
        // must be the authored lexeme.
        if let Some(text) = scalar_text(&applied, "release") {
            prop_assert_eq!(
                text.as_str(),
                scalar,
                "auto-apply changed the scalar text of {:?}",
                scalar
            );
        }
    }

    /// After the quoting tier runs, re-running it must be a no-op. Without
    /// this, `md clean` would not be a fixed point on schema-repaired files.
    #[test]
    fn schema_quoting_is_idempotent(scalar in scalar_strategy()) {
        let body = format!("$schema:\n  release: string\nrelease: {scalar}\n");
        let effective = effective_for(&body);
        let once = auto_apply(&analyze_frontmatter(&effective, &body), &body);
        let twice = auto_apply(&analyze_frontmatter(&effective, &once), &once);
        prop_assert_eq!(&twice, &once, "schema quoting is not a fixed point");
    }

    /// An already-correct string must never be touched. The tier exists to
    /// fix type mismatches, and there is no mismatch here.
    #[test]
    fn already_quoted_values_are_untouched(scalar in scalar_strategy()) {
        let body = format!("$schema:\n  release: string\nrelease: \"{scalar}\"\n");
        let effective = effective_for(&body);
        let analysis = analyze_frontmatter(&effective, &body);
        prop_assert_eq!(
            auto_apply(&analysis, &body),
            body.clone(),
            "a schema-satisfying value must not be edited"
        );
    }

    /// A key the schema says nothing about gets no auto-applied edit. This
    /// is the spec's unconstrained-key silence (D-8).
    #[test]
    fn unconstrained_keys_are_never_auto_repaired(scalar in scalar_strategy()) {
        let body = format!("$schema:\n  other: string\nrelease: {scalar}\n");
        let effective = effective_for(&body);
        let analysis = analyze_frontmatter(&effective, &body);
        prop_assert_eq!(
            auto_apply(&analysis, &body),
            body.clone(),
            "unconstrained key {:?} must not be auto-repaired",
            scalar
        );
    }
}

/// A type change the schema did *not* prove must never auto-apply. Here the
/// schema wants a number and got a string: "fixing" that means unquoting,
/// which would change the text. v1 reports it and stops.
#[test]
fn unproven_type_change_never_auto_applies() {
    let body = "$schema:\n  count: number\ncount: \"abc\"\n";
    let effective = effective_for(body);
    let analysis = analyze_frontmatter(&effective, body);

    assert_eq!(
        auto_apply(&analysis, body),
        body,
        "a string → number repair is not deterministic and must not auto-apply"
    );
    for diagnostic in analysis.diagnostics() {
        assert_ne!(
            diagnostic.classification,
            YamlCertainty::Deterministic,
            "string → number must not be classified deterministic: {diagnostic:?}"
        );
    }
}

/// More than one raw problem means the sole-mismatch proof cannot run, so
/// nothing auto-applies even though each problem looks individually fixable.
#[test]
fn multiple_problems_block_auto_apply() {
    let body = "$schema:\n  release: string\n  name: string\nrelease: 1.20\nname: 42\n";
    let effective = effective_for(body);
    let analysis = analyze_frontmatter(&effective, body);

    assert_eq!(
        auto_apply(&analysis, body),
        body,
        "the sole-problem precondition must gate auto-apply"
    );
}

/// Report-only diagnostics must carry no repairs at all, so a caller that
/// naively drains `repairs` cannot mutate on a non-deterministic finding.
#[test]
fn report_only_diagnostics_carry_no_repairs() {
    let body = "$schema:\n  count: number\ncount: \"abc\"\n";
    let effective = effective_for(body);

    for diagnostic in analyze_frontmatter(&effective, body).diagnostics() {
        if diagnostic.classification != YamlCertainty::Deterministic {
            assert!(
                diagnostic.repairs.is_empty(),
                "report-only diagnostic must carry no repairs: {diagnostic:?}"
            );
        }
    }
}

/// The flagship schema case, end to end, with its neighbours pinned: only
/// the proven scalar moves.
#[test]
fn proven_quoting_edits_only_its_own_scalar() {
    let body = "$schema:\n  release: string\nname: example\nrelease: 1.20\nkeep: 7\n";
    let effective = effective_for(body);
    let applied = auto_apply(&analyze_frontmatter(&effective, body), body);

    assert_eq!(
        applied, "$schema:\n  release: string\nname: example\nrelease: \"1.20\"\nkeep: 7\n",
        "only the schema-proven scalar may change"
    );
}
