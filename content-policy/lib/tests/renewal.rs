//! Renewal through the public API: planning, byte-exact editing, refused
//! shapes, the safety net, file application, and the lifecycle example.

use std::fs;

use chrono::NaiveDate;
use content_policy::{
    Action, BaselineTarget, ChangeKind, ConflictKind, DiagnosticCode, EvaluationContext,
    NaiveDateText, Policy, PolicyOptions, PolicySource, RefusalReason, RenewalContext,
    RenewalError, RenewalPlan, ResultKind, Status, apply_renewal, evaluate_document,
    plan_fingerprint, plan_renewal,
};

mod common;

const TODAY: &str = "2026-09-28";

fn date(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
}

fn context(today: &str) -> RenewalContext {
    RenewalContext::new(date(today))
}

fn plan_at(source: &str, context: &RenewalContext) -> RenewalPlan {
    plan_renewal(source.as_bytes(), context).unwrap_or_else(|error| panic!("{error}\n{source:?}"))
}

fn plan(source: &str) -> RenewalPlan {
    plan_at(source, &context(TODAY))
}

fn apply(plan: &RenewalPlan, source: &str) -> String {
    String::from_utf8(plan.apply_to(source.as_bytes()).unwrap()).unwrap()
}

/// The renewed text of `source` on [`TODAY`].
fn renewed(source: &str) -> String {
    apply(&plan(source), source)
}

fn refusals(source: &str) -> Vec<(RefusalReason, String)> {
    match plan_renewal(source.as_bytes(), &context(TODAY)) {
        Err(RenewalError::Refused { refusals, .. }) => {
            refusals.into_iter().map(|refusal| (refusal.reason, refusal.message)).collect()
        }
        other => panic!("expected a refusal for {source:?}, got {other:?}"),
    }
}

fn refusal(source: &str) -> (RefusalReason, String) {
    let mut all = refusals(source);
    assert_eq!(all.len(), 1, "{all:?}");
    all.remove(0)
}

fn evaluates(source: &str) {
    evaluate_document(source.as_bytes(), &EvaluationContext::new(date(TODAY).and_hms_opt(0, 0, 0).unwrap().and_utc()))
        .unwrap_or_else(|error| panic!("{error}\n{source:?}"));
}

fn property(name: &str) -> BaselineTarget {
    BaselineTarget::Property {
        name: name.to_string(),
    }
}

// --- Planning (AC 6, 19, 25) --------------------------------------------------

/// Every renewable entry advances together: inline dates in place, `@name`
/// properties, and the shorthand's default property, written once when shared.
/// Durations, actions, the deadline, and every other byte stay.
#[test]
fn every_renewable_entry_is_renewed_and_nothing_else_changes() {
    let source = "---\n\
        title: Notes # keep\n\
        last_updated: 2026-01-01\n\
        reviewed: '2026-02-01'\n\
        content_policy:\n  \
          - ValidFor(3mo)\n  \
          - ValidFor(1yr, @reviewed)\n  \
          - ValidFor(6mo,  2026-03-01)   # inline\n  \
          - rule: ValidFor(2wk, @last_updated)\n    \
            action: archive\n  \
          - rule: ValidUntil(2027-01-01)\n    \
            action: remove\n  \
          - TimeSensitive\n\
        ---\n\
        Body 2026-01-01\n";
    let plan = plan(source);
    let summary: Vec<_> = plan
        .changes
        .iter()
        .map(|change| (change.target.clone(), change.entries.clone(), change.kind, change.previous))
        .collect();
    assert_eq!(
        summary,
        vec![
            (property("last_updated"), vec![0, 3], ChangeKind::Renewed, Some(NaiveDateText(date("2026-01-01")))),
            (property("reviewed"), vec![1], ChangeKind::Renewed, Some(NaiveDateText(date("2026-02-01")))),
            (BaselineTarget::Inline { entry: 2 }, vec![2], ChangeKind::Renewed, Some(NaiveDateText(date("2026-03-01")))),
        ]
    );
    assert!(plan.changes.iter().all(|change| change.value == NaiveDateText(date(TODAY))));
    assert_eq!(plan.policy.source, PolicySource::Declared);
    assert!(plan.tab_repair.is_empty());
    let expected = source
        .replace("last_updated: 2026-01-01", "last_updated: 2026-09-28")
        .replace("reviewed: '2026-02-01'", "reviewed: '2026-09-28'")
        .replace("ValidFor(6mo,  2026-03-01)", "ValidFor(6mo,  2026-09-28)");
    assert_eq!(apply(&plan, source), expected);
}

/// Renewal never changes the policy's identity (AC 23), and the renewed
/// document evaluates fresh on the update date.
#[test]
fn renewal_keeps_the_policy_identity() {
    let source = "---\ncontent_policy:\n  - ValidFor(3mo, 2026-01-01)\n  - ValidFor(1yr, @reviewed)\n---\n";
    let at = EvaluationContext::new(date(TODAY).and_hms_opt(0, 0, 0).unwrap().and_utc());
    let before = evaluate_document(source.as_bytes(), &at).unwrap();
    let plan = plan(source);
    assert_eq!(plan.policy.identity, before.policy.identity);
    let after = evaluate_document(apply(&plan, source).as_bytes(), &at).unwrap();
    assert_eq!(after.policy.identity, before.policy.identity);
    assert_eq!(after.status, Status::Fresh);
}

#[test]
fn the_update_date_defaults_to_today_and_rejects_the_future() {
    let source = "---\nlast_updated: 2026-01-01\n---\n";
    assert_eq!(plan(source).update_date, NaiveDateText(date(TODAY)));

    let earlier = context(TODAY).on(date("2026-09-01"));
    let plan = plan_at(source, &earlier);
    assert_eq!(apply(&plan, source), "---\nlast_updated: 2026-09-01\n---\n");

    let future = context(TODAY).on(date("2026-09-29"));
    let error = plan_renewal(source.as_bytes(), &future).unwrap_err();
    assert!(
        matches!(error, RenewalError::FutureDate { on, today, .. } if on == date("2026-09-29") && today == date(TODAY)),
        "{error:?}"
    );
    assert!(error.to_string().contains("after today's UTC date"), "{error}");
}

/// A missing or `null` target is a first capture ("new baseline"), distinct
/// from a renewed value; each spelling of `null` is filled in place.
#[test]
fn missing_and_null_targets_are_new_baselines() {
    let cases = [
        ("---\ntitle: x\n---\n", "---\ntitle: x\nlast_updated: 2026-09-28\n---\n"),
        ("---\nlast_updated:\ntitle: x\n---\n", "---\nlast_updated: 2026-09-28\ntitle: x\n---\n"),
        ("---\nlast_updated: ~\n---\n", "---\nlast_updated: 2026-09-28\n---\n"),
        ("---\nlast_updated: null\n---\n", "---\nlast_updated: 2026-09-28\n---\n"),
        ("---\nlast_updated: # todo\n---\n", "---\nlast_updated: 2026-09-28 # todo\n---\n"),
        ("---\nlast_updated: ~ # todo\n---\n", "---\nlast_updated: 2026-09-28 # todo\n---\n"),
    ];
    for (source, expected) in cases {
        let plan = plan(source);
        assert_eq!(plan.changes.len(), 1, "{source:?}");
        assert_eq!(plan.changes[0].kind, ChangeKind::NewBaseline, "{source:?}");
        assert_eq!(plan.changes[0].previous, None, "{source:?}");
        assert_eq!(apply(&plan, source), expected, "{source:?}");
    }
}

#[test]
fn a_baseline_already_at_the_update_date_is_unchanged() {
    let source = "---\nlast_updated: 2026-09-28\ncontent_policy:\n  - ValidFor(3mo)\n  - ValidFor(1yr, 2026-09-28)\n---\n";
    let plan = plan(source);
    assert!(plan.changes.iter().all(|change| change.kind == ChangeKind::Unchanged), "{plan:?}");
    assert!(plan.edits.is_empty());
    assert!(!plan.nothing_to_renew());
    assert_eq!(apply(&plan, source), source);
}

/// A present malformed baseline needs correction; renewal does not repair it.
#[test]
fn a_present_malformed_baseline_is_an_error() {
    let cases = [
        ("last_updated: Sept 28", DiagnosticCode::InvalidDate),
        ("last_updated: 2026-02-30", DiagnosticCode::InvalidDate),
        ("last_updated: \"\"", DiagnosticCode::InvalidDate),
        ("last_updated: 2026-09-28T10:00:00Z", DiagnosticCode::DatesOnly),
        ("last_updated: 5", DiagnosticCode::WrongType),
        ("last_updated: [2026-01-01]", DiagnosticCode::WrongType),
        ("reviewed: {at: 2026-01-01}\ncontent_policy:\n  - ValidFor(3mo, @reviewed)", DiagnosticCode::WrongType),
    ];
    for (frontmatter, code) in cases {
        let source = format!("---\n{frontmatter}\n---\n");
        match plan_renewal(source.as_bytes(), &context(TODAY)) {
            Err(RenewalError::Invalid(invalid)) => assert!(invalid.has(code), "{source:?}: {invalid}"),
            other => panic!("{source:?}: expected invalid, got {other:?}"),
        }
    }
}

#[test]
fn invalid_declarations_and_duplicate_keys_are_errors() {
    for (source, code) in [
        ("---\ncontent_policy: []\n---\n", DiagnosticCode::EmptyPolicy),
        ("---\ncontent_policy:\n  - Duration(3mo)\n---\n", DiagnosticCode::UnknownRule),
        ("---\nlast_updated: 2026-01-01\nlast_updated: 2026-02-01\n---\n", DiagnosticCode::DuplicateKey),
    ] {
        match plan_renewal(source.as_bytes(), &context(TODAY).with_document("doc.md")) {
            Err(RenewalError::Invalid(invalid)) => {
                assert!(invalid.has(code), "{source:?}: {invalid}");
                assert_eq!(invalid.document.as_deref(), Some("doc.md"));
            }
            other => panic!("{source:?}: expected invalid, got {other:?}"),
        }
    }
    // The unquoted flow-list reference is malformed YAML (AC 17).
    let unquoted = "---\nlast_updated: 2026-01-01\ncontent_policy: [ValidFor(3mo, @last_updated)]\n---\n";
    assert!(matches!(
        plan_renewal(unquoted.as_bytes(), &context(TODAY)),
        Err(RenewalError::Read { .. })
    ));
}

/// A renewed property that is also a `ValidUntil` deadline would extend the
/// deadline; that is a conflict, not a silent write.
#[test]
fn renewing_a_referenced_deadline_is_a_conflict() {
    let source = "---\nreview: 2026-01-01\ncontent_policy:\n  - ValidFor(3mo, @review)\n  - rule: ValidUntil(@review)\n    action: archive\n---\n";
    match plan_renewal(source.as_bytes(), &context(TODAY)) {
        Err(RenewalError::Conflict { conflicts, .. }) => {
            assert_eq!(conflicts.len(), 1);
            assert_eq!(conflicts[0].kind, ConflictKind::MovesDeadline);
            assert_eq!(conflicts[0].property, "review");
            assert_eq!(conflicts[0].entries, vec![0, 1]);
        }
        other => panic!("expected a conflict, got {other:?}"),
    }
    // A deadline on a different property is not affected.
    let separate = "---\nreview: 2026-01-01\nretire: 2027-01-01\ncontent_policy:\n  - ValidFor(3mo, @review)\n  - ValidUntil(@retire)\n---\n";
    assert_eq!(
        renewed(separate),
        separate.replace("review: 2026-01-01", "review: 2026-09-28")
    );
}

/// `Evergreen`, `TimeSensitive`, and `ValidUntil`-only policies have nothing
/// to renew; the plan is empty, lists no tab repair, and applies to the same
/// bytes.
#[test]
fn policies_without_a_renewable_entry_have_nothing_to_renew() {
    for policy in [
        "  - Evergreen\n",
        "  - TimeSensitive\n",
        "  - ValidUntil(2027-01-01)\n",
        "  - ValidUntil(@retire)\n  - TimeSensitive\n",
    ] {
        let source = format!("---\nlast_updated: 2026-01-01\ncontent_policy:\n{policy}---\n");
        let plan = plan(&source);
        assert!(plan.nothing_to_renew(), "{source:?}");
        assert!(plan.edits.is_empty() && plan.tab_repair.is_empty());
        assert_eq!(apply(&plan, &source), source);
    }
    let tabs = "---\nnote: |-\n\tx\ncontent_policy:\n  - Evergreen\n---\n";
    let plan = plan(tabs);
    assert!(plan.nothing_to_renew() && plan.tab_repair.is_empty());

    let time_sensitive = PolicyOptions::default().with_default_policy(Policy::from_text("TimeSensitive").unwrap());
    let plan = plan_at("# No frontmatter\n", &context(TODAY).with_options(time_sensitive));
    assert!(plan.nothing_to_renew());
    assert_eq!(plan.policy.source, PolicySource::Defaulted);
}

/// A document with no frontmatter falls back to the default policy; renewal
/// creates a block holding the date property, after any BOM, with the body's
/// line terminator, and leaves the body byte-for-byte unchanged (AC 19).
#[test]
fn a_document_without_frontmatter_gets_a_new_block() {
    let cases = [
        ("# Title\n\nBody\n", "---\nlast_updated: 2026-09-28\n---\n# Title\n\nBody\n"),
        ("# Title\r\n\r\nBody\r\n", "---\r\nlast_updated: 2026-09-28\r\n---\r\n# Title\r\n\r\nBody\r\n"),
        ("# Title\rBody\r", "---\rlast_updated: 2026-09-28\r---\r# Title\rBody\r"),
        ("\u{feff}# Title\n", "\u{feff}---\nlast_updated: 2026-09-28\n---\n# Title\n"),
        ("", "---\nlast_updated: 2026-09-28\n---\n"),
        ("no terminator", "---\nlast_updated: 2026-09-28\n---\nno terminator"),
    ];
    for (source, expected) in cases {
        let plan = plan(source);
        assert_eq!(plan.policy.source, PolicySource::Defaulted);
        assert_eq!(plan.changes[0].kind, ChangeKind::NewBaseline);
        assert_eq!(apply(&plan, source), expected, "{source:?}");
    }

    // The configured date property and a default policy's `@name` reference.
    let options = PolicyOptions::default()
        .with_date_property("reviewed_on")
        .with_default_policy(Policy::from_text(r#"["ValidFor(3mo)", "ValidFor(1yr, @audited)"]"#).unwrap());
    let plan = plan_at("Body\n", &context(TODAY).with_options(options));
    assert_eq!(
        apply(&plan, "Body\n"),
        "---\nreviewed_on: 2026-09-28\naudited: 2026-09-28\n---\nBody\n"
    );
}

#[test]
fn a_default_policy_inline_baseline_is_not_in_the_document() {
    let options = PolicyOptions::default().with_default_policy(Policy::from_text("ValidFor(3mo, 2026-01-01)").unwrap());
    let error = plan_renewal(b"Body\n", &context(TODAY).with_options(options)).unwrap_err();
    let RenewalError::Refused { refusals, .. } = error else {
        panic!("{error:?}");
    };
    assert_eq!(refusals[0].reason, RefusalReason::NotInDocument);
}

#[test]
fn a_configured_policy_key_is_renewed() {
    let source = "---\npolicy:\n  - ValidFor(3mo, 2026-01-01)\ncontent_policy:\n  - ValidFor(3mo, 2026-01-01)\n---\n";
    let options = PolicyOptions::default().with_key("policy");
    let plan = plan_at(source, &context(TODAY).with_options(options));
    assert_eq!(
        apply(&plan, source),
        "---\npolicy:\n  - ValidFor(3mo, 2026-09-28)\ncontent_policy:\n  - ValidFor(3mo, 2026-01-01)\n---\n"
    );
}

#[test]
fn the_plan_fingerprints_the_bytes_it_read() {
    let source = "---\nlast_updated: 2026-01-01\n---\n";
    let plan = plan(source);
    assert_eq!(plan.fingerprint, plan_fingerprint(source.as_bytes()));
    let hex = plan.fingerprint.strip_prefix("xxh64:").unwrap();
    assert!(hex.len() == 16 && hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    assert_ne!(plan.fingerprint, plan_fingerprint(b"---\nlast_updated: 2026-01-02\n---\n"));
}

// --- Byte-exact editing (AC 12, 29, 30) -------------------------------------

/// One case per row of the spec's comparison with Darkmatter's writer.
#[test]
fn darkmatter_comparison_table() {
    // `last_updated:` with no value.
    assert_eq!(
        renewed("---\nlast_updated:\ntitle: x\n---\nBody\n"),
        "---\nlast_updated: 2026-09-28\ntitle: x\n---\nBody\n"
    );
    // A comment after an empty value keeps its spacing.
    assert_eq!(
        renewed("---\nhash: abc\nlast_updated:   # todo\n---\n"),
        "---\nhash: abc\nlast_updated: 2026-09-28   # todo\n---\n"
    );
    // An LF line in a file that contains CRLF keeps its LF.
    assert_eq!(
        renewed("---\r\nhash: abc\r\nlast_updated: 2026-01-01\n---\r\nBody\r\n"),
        "---\r\nhash: abc\r\nlast_updated: 2026-09-28\n---\r\nBody\r\n"
    );
    // Lone-CR lines keep their CR.
    assert_eq!(
        renewed("---\rhash: abc\rlast_updated: 2026-01-01\r---\rBody\r"),
        "---\rhash: abc\rlast_updated: 2026-09-28\r---\rBody\r"
    );
    // A BOM and no frontmatter: the new block goes after the BOM.
    assert_eq!(
        renewed("\u{feff}# Title\n\nBody\n"),
        "\u{feff}---\nlast_updated: 2026-09-28\n---\n# Title\n\nBody\n"
    );
    // An anchored target is refused.
    let (reason, message) = refusal("---\nlast_updated: &lu 2026-01-01\nreviewed: *lu\n---\n");
    assert_eq!(reason, RefusalReason::AnchorAliasOrTag);
    assert!(message.contains("&lu"), "{message}");
}

/// Each untouched line keeps its own terminator, and an appended property
/// takes the terminator of the line before it.
#[test]
fn mixed_line_endings_are_kept_per_line() {
    let edit = "---\nhash: abc\r\nlast_updated: 2026-01-01\ntitle: x\r\n---\nBody\r\nmore\n";
    assert_eq!(renewed(edit), edit.replace("2026-01-01", "2026-09-28"));
    assert_eq!(
        renewed("---\nhash: abc\ntitle: x\r\n---\nBody\n"),
        "---\nhash: abc\ntitle: x\r\nlast_updated: 2026-09-28\r\n---\nBody\n"
    );
    assert_eq!(
        renewed("---\r\ntitle: x\n---\r\nBody\r\n"),
        "---\r\ntitle: x\nlast_updated: 2026-09-28\n---\r\nBody\r\n"
    );
}

/// Accepted shapes from the frontmatter-reader spike's fixture matrix, each
/// with its byte-exact result.
#[test]
fn spike_matrix_accepted_shapes() {
    let inline = |item: &str| format!("---\ntitle: t\ncontent_policy:\n{item}\n---\nBody\n");
    let cases: Vec<(&str, String, String)> = vec![
        ("list item plain", inline("  - ValidFor(3mo, 2026-01-01)"), inline("  - ValidFor(3mo, 2026-09-28)")),
        ("list item single-quoted", inline("  - 'ValidFor(3mo, 2026-01-01)'"), inline("  - 'ValidFor(3mo, 2026-09-28)'")),
        ("list item double-quoted", inline("  - \"ValidFor(3mo, 2026-01-01)\""), inline("  - \"ValidFor(3mo, 2026-09-28)\"")),
        ("zero-indent list (AC 29)", inline("- ValidFor(3mo, 2026-01-01)"), inline("- ValidFor(3mo, 2026-09-28)")),
        ("second entry", inline("  - TimeSensitive\n  - ValidFor(3mo, 2026-01-01)"), inline("  - TimeSensitive\n  - ValidFor(3mo, 2026-09-28)")),
        ("rule plain", inline("  - rule: ValidFor(3mo, 2026-01-01)\n    action: archive"), inline("  - rule: ValidFor(3mo, 2026-09-28)\n    action: archive")),
        ("rule single-quoted", inline("  - rule: 'ValidFor(3mo, 2026-01-01)'\n    action: archive"), inline("  - rule: 'ValidFor(3mo, 2026-09-28)'\n    action: archive")),
        ("rule double-quoted", inline("  - rule: \"ValidFor(3mo, 2026-01-01)\"\n    action: archive"), inline("  - rule: \"ValidFor(3mo, 2026-09-28)\"\n    action: archive")),
        ("rule as second key", inline("  - action: archive\n    rule: ValidFor(3mo, 2026-01-01)"), inline("  - action: archive\n    rule: ValidFor(3mo, 2026-09-28)")),
        ("zero-indent long form", inline("- rule: ValidFor(3mo, 2026-01-01)\n  action: archive"), inline("- rule: ValidFor(3mo, 2026-09-28)\n  action: archive")),
        ("item trailing comment", inline("  - ValidFor(3mo, 2026-01-01)  # renewed in place"), inline("  - ValidFor(3mo, 2026-09-28)  # renewed in place")),
        ("rule trailing comment", inline("  - rule: ValidFor(3mo, 2026-01-01) # c\n    action: archive"), inline("  - rule: ValidFor(3mo, 2026-09-28) # c\n    action: archive")),
        ("comment line after key", inline("  # reviewed quarterly\n  - ValidFor(3mo, 2026-01-01)"), inline("  # reviewed quarterly\n  - ValidFor(3mo, 2026-09-28)")),
        (
            "comment on key line",
            "---\ncontent_policy: # note\n  - ValidFor(3mo, 2026-01-01)\n---\n".to_string(),
            "---\ncontent_policy: # note\n  - ValidFor(3mo, 2026-09-28)\n---\n".to_string(),
        ),
        (
            "CRLF rule with comment",
            "---\r\ntitle: t\r\ncontent_policy:\r\n  - rule: 'ValidFor(3mo, 2026-01-01)' # c\r\n    action: archive\r\n---\r\nBody\r\n".to_string(),
            "---\r\ntitle: t\r\ncontent_policy:\r\n  - rule: 'ValidFor(3mo, 2026-09-28)' # c\r\n    action: archive\r\n---\r\nBody\r\n".to_string(),
        ),
    ];
    let properties: Vec<(&str, &str, &str)> = vec![
        ("prop trailing comment", "---\nhash: abc-def\nlast_updated: 2026-01-01 # reviewed\n---\n", "---\nhash: abc-def\nlast_updated: 2026-09-28 # reviewed\n---\n"),
        ("prop quoted and comment", "---\nlast_updated: \"2026-01-01\"   # c\n---\n", "---\nlast_updated: \"2026-09-28\"   # c\n---\n"),
        ("prop trailing spaces", "---\nlast_updated: 2026-01-01   \n---\n", "---\nlast_updated: 2026-09-28   \n---\n"),
        ("prop single-quoted", "---\nlast_updated: '2026-01-01'\n---\n", "---\nlast_updated: '2026-09-28'\n---\n"),
        ("prop after multibyte", "---\ntitle: café ☕ 日本\nlast_updated: 2026-01-01\n---\n", "---\ntitle: café ☕ 日本\nlast_updated: 2026-09-28\n---\n"),
        ("prop only nested", "---\nmeta:\n  last_updated: 2026-01-01\n---\n", "---\nmeta:\n  last_updated: 2026-01-01\nlast_updated: 2026-09-28\n---\n"),
        ("prop key quoted", "---\n\"last_updated\": 2026-01-01\n---\n", "---\n\"last_updated\": 2026-09-28\n---\n"),
        ("CRLF null value", "---\r\nhash: abc\r\nlast_updated:\r\n---\r\n", "---\r\nhash: abc\r\nlast_updated: 2026-09-28\r\n---\r\n"),
        ("CRLF insert", "---\r\nhash: abc\r\n---\r\nBody\r\n", "---\r\nhash: abc\r\nlast_updated: 2026-09-28\r\n---\r\nBody\r\n"),
        ("closing fence at EOF", "---\nlast_updated: 2026-01-01\n---", "---\nlast_updated: 2026-09-28\n---"),
        ("closing fence at EOF, insert", "---\nhash: abc\n---", "---\nhash: abc\nlast_updated: 2026-09-28\n---"),
        ("empty block", "---\n---\nBody\n", "---\nlast_updated: 2026-09-28\n---\nBody\n"),
        ("blank-line block", "---\n\n---\nBody\n", "---\n\nlast_updated: 2026-09-28\n---\nBody\n"),
        ("BOM prop", "\u{feff}---\nlast_updated: 2026-01-01\n---\nBody\n", "\u{feff}---\nlast_updated: 2026-09-28\n---\nBody\n"),
        // A trailing clip or keep block scalar would gain a line break from a
        // line after it, so the property goes in front of that entry.
        ("last key is a clip block scalar", "---\ntitle: t\nnote: |\n  text\n---\n", "---\ntitle: t\nlast_updated: 2026-09-28\nnote: |\n  text\n---\n"),
        ("only key is a keep block scalar", "---\nnote: >+ # c\n  text\n\n---\n", "---\nlast_updated: 2026-09-28\nnote: >+ # c\n  text\n\n---\n"),
        ("nested clip block scalar", "---\nmeta:\n  items:\n    - |\n      text\n---\n", "---\nlast_updated: 2026-09-28\nmeta:\n  items:\n    - |\n      text\n---\n"),
        ("last key is a strip block scalar", "---\nnote: |-\n  text\n---\n", "---\nnote: |-\n  text\nlast_updated: 2026-09-28\n---\n"),
        ("CRLF clip block scalar", "---\r\ntitle: t\r\nnote: |\r\n  text\r\n---\r\n", "---\r\ntitle: t\r\nlast_updated: 2026-09-28\r\nnote: |\r\n  text\r\n---\r\n"),
        (
            "quoted flow-list reference (AC 14, 17)",
            "---\nlast_updated: 2026-01-01\ncontent_policy: [\"ValidFor(3mo, @last_updated)\"]\n---\n",
            "---\nlast_updated: 2026-09-28\ncontent_policy: [\"ValidFor(3mo, @last_updated)\"]\n---\n",
        ),
        (
            "flow-mapping entry, property target",
            "---\nlast_updated: 2026-01-01\ncontent_policy:\n  - {rule: \"ValidFor(3mo, @last_updated)\", action: archive}\n---\n",
            "---\nlast_updated: 2026-09-28\ncontent_policy:\n  - {rule: \"ValidFor(3mo, @last_updated)\", action: archive}\n---\n",
        ),
    ];
    let all = cases
        .iter()
        .map(|(name, source, expected)| (*name, source.as_str(), expected.as_str()))
        .chain(properties);
    for (name, source, expected) in all {
        let plan = plan_renewal(source.as_bytes(), &context(TODAY)).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(apply(&plan, source), expected, "{name}");
    }
}

/// A flow-style policy renews when every edit lies outside the brackets, and
/// is refused when the date to change is inside (AC 14).
#[test]
fn flow_lists_renew_only_outside_the_brackets() {
    let block = "---\nlast_updated: 2026-09-01\ncontent_policy:\n  - ValidFor(3mo, @last_updated)\n---\n";
    let flow = "---\nlast_updated: 2026-09-01\ncontent_policy: [\"ValidFor(3mo, @last_updated)\"]\n---\n";
    assert_eq!(renewed(block), block.replace("2026-09-01", "2026-09-28"));
    assert_eq!(renewed(flow), flow.replace("2026-09-01", "2026-09-28"));

    let inside = "---\ncontent_policy: [\"ValidFor(3mo, 2026-09-01)\"]\n---\n";
    let (reason, message) = refusal(inside);
    assert_eq!(reason, RefusalReason::FlowList);
    assert!(message.contains("block list") && message.contains("- ValidFor(3mo, 2026-09-01)"), "{message}");
    evaluates(inside);
}

// --- Refused shapes (AC 14, 31) ---------------------------------------------

/// Every refused shape names its reason and writes nothing; the first three
/// name the block-list form, and evaluation of the same documents succeeds.
#[test]
fn refused_shapes_name_their_reason() {
    let policy = |list: &str| format!("---\ntitle: t\ncontent_policy:\n{list}\n---\nBody\n");
    let cases: Vec<(&str, String, RefusalReason, bool)> = vec![
        ("flow list", "---\ncontent_policy: [\"ValidFor(3mo, 2026-01-01)\"]\n---\n".to_string(), RefusalReason::FlowList, true),
        ("multi-line flow list", "---\ncontent_policy: [\n  \"ValidFor(3mo, 2026-01-01)\"\n]\n---\n".to_string(), RefusalReason::FlowList, true),
        ("block scalar item", policy("  - >-\n    ValidFor(3mo, 2026-01-01)"), RefusalReason::MultiLineValue, true),
        ("literal block rule", policy("  - rule: |-\n      ValidFor(3mo, 2026-01-01)\n    action: refresh"), RefusalReason::MultiLineValue, true),
        ("multi-line plain item", policy("  - ValidFor(3mo,\n    2026-01-01)"), RefusalReason::MultiLineValue, true),
        ("multi-line plain, date on first line", policy("  - ValidFor(3mo, 2026-01-01\n    )"), RefusalReason::MultiLineValue, true),
        ("multi-line quoted item", policy("  - \"ValidFor(3mo,\n    2026-01-01)\""), RefusalReason::MultiLineValue, true),
        ("escaped double-quoted item", policy("  - \"ValidFor(3mo,\\x202026-01-01)\""), RefusalReason::EscapedString, true),
        ("escaped double-quoted rule", policy("  - rule: \"ValidFor(3mo,\\x202026-01-01)\"\n    action: archive"), RefusalReason::EscapedString, true),
        ("anchored rule", policy("  - &r ValidFor(3mo, 2026-01-01)"), RefusalReason::AnchorAliasOrTag, false),
        ("tagged rule", policy("  - !!str ValidFor(3mo, 2026-01-01)"), RefusalReason::AnchorAliasOrTag, false),
        ("anchored policy list", "---\ncontent_policy: &p\n  - ValidFor(3mo, 2026-01-01)\n---\n".to_string(), RefusalReason::AnchorAliasOrTag, false),
        ("anchored property", "---\nlast_updated: &lu 2026-01-01\n---\n".to_string(), RefusalReason::AnchorAliasOrTag, false),
        ("aliased property", "---\nbase: &d 2026-01-01\nlast_updated: *d\n---\n".to_string(), RefusalReason::AnchorAliasOrTag, false),
        ("tagged property", "---\nlast_updated: !!str 2026-01-01\n---\n".to_string(), RefusalReason::AnchorAliasOrTag, false),
        ("flow-mapping entry", policy("  - {rule: \"ValidFor(3mo, 2026-01-01)\", action: archive}"), RefusalReason::FlowMapping, false),
        ("block-scalar property", "---\nlast_updated: >-\n  2026-01-01\n---\n".to_string(), RefusalReason::MultiLineValue, false),
    ];
    for (name, source, reason, names_block_list) in cases {
        let (actual, message) = refusal(&source);
        assert_eq!(actual, reason, "{name}: {message}");
        assert!(!message.is_empty());
        if names_block_list {
            assert!(message.contains("block list"), "{name}: {message}");
        }
        evaluates(&source);
    }

    for (source, reason) in [
        ("---\nlast_updated: 2026-01-01\n", RefusalReason::UnterminatedBlock),
        ("---\nlast_updated: 2026-01-01\n...\nBody\n", RefusalReason::UnterminatedBlock),
        ("----\nlast_updated: 2026-01-01\n----\nBody\n", RefusalReason::NearMissFence),
    ] {
        let (actual, message) = refusal(source);
        assert_eq!(actual, reason, "{source:?}");
        assert!(message.contains("second block"), "{message}");
    }
    let (_, message) = refusal("---\na: 1\n...\n");
    assert!(message.contains("`...`"), "{message}");
}

/// Planning covers every entry before any edit: one refused target means no
/// plan at all, and every refusal is listed.
#[test]
fn one_refused_target_means_no_partial_plan() {
    let source = "---\nlast_updated: &a 2026-01-01\ncontent_policy: [\"ValidFor(3mo)\", \"ValidFor(1yr, 2026-01-01)\"]\n---\n";
    let all = refusals(source);
    let reasons: Vec<_> = all.iter().map(|(reason, _)| *reason).collect();
    assert_eq!(reasons, vec![RefusalReason::AnchorAliasOrTag, RefusalReason::FlowList]);

    let partly = "---\nlast_updated: 2026-01-01\ncontent_policy: [\"ValidFor(3mo)\", \"ValidFor(1yr, 2026-01-01)\"]\n---\n";
    assert_eq!(refusal(partly).0, RefusalReason::FlowList);
}

// --- Tab repair (AC 27, renewal half) ---------------------------------------

/// Tab-indented frontmatter gets the tab repair as its own listed edit;
/// applying writes it together with the date edits and changes nothing else.
#[test]
fn tab_indented_frontmatter_lists_and_applies_the_repair() {
    let source = "---\nprompt: |-\n\tLine one\n\t\tLine two\nnested:\n\tkey: value # c\nlast_updated: 2026-02-27\ncontent_policy:\n\t- ValidFor(6mo)\n\t- ValidFor(1yr, 2026-02-27)\n---\nBody\n\tindented body\n";
    let plan = plan(source);
    assert_eq!(plan.tab_repair.len(), 5);
    assert_eq!(plan.edits.len(), 2);
    for repair in &plan.tab_repair {
        assert!(source[repair.span.clone()].contains('\t'), "{repair:?}");
    }
    assert_eq!(
        apply(&plan, source),
        "---\nprompt: |-\n  Line one\n    Line two\nnested:\n  key: value # c\nlast_updated: 2026-09-28\ncontent_policy:\n  - ValidFor(6mo)\n  - ValidFor(1yr, 2026-09-28)\n---\nBody\n\tindented body\n"
    );
}

// --- Safety net and application (AC 12, 31) ---------------------------------

#[test]
fn the_safety_net_refuses_a_corrupted_edit() {
    let source = "---\ntitle: x\nlast_updated: 2026-01-01\n---\nBody\n";
    let good = plan(source);

    let mut extra_key = good.clone();
    extra_key.edits[0].replacement = "2026-09-28\nextra: 1".to_string();
    let mut other_value = good.clone();
    other_value.edits[0].span = 11..12;
    other_value.edits[0].replacement = "y".to_string();
    let mut body = good.clone();
    body.edits[0].span = source.len() - 2..source.len() - 1;
    body.edits[0].replacement = "Y".to_string();
    let mut overlapping = good.clone();
    overlapping.edits.push(overlapping.edits[0].clone());
    let mut broken = good.clone();
    broken.edits[0].replacement = "[".to_string();

    for (name, corrupted, detail) in [
        ("extra key", extra_key, "`extra`"),
        ("other value", other_value, "`title`"),
        ("body", body, "outside the frontmatter"),
        ("overlap", overlapping, "overlaps"),
        ("unparseable", broken, "does not read"),
    ] {
        match corrupted.apply_to(source.as_bytes()) {
            Err(RenewalError::SafetyNet { detail: actual, .. }) => {
                assert!(actual.contains(detail), "{name}: {actual}");
            }
            other => panic!("{name}: expected the safety net, got {other:?}"),
        }
    }

    // Through the file helper, a refused plan leaves the file untouched.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("note.md");
    fs::write(&path, source).unwrap();
    let mut corrupted = good.clone();
    corrupted.edits[0].replacement = "2026-09-28\nextra: 1".to_string();
    assert!(matches!(apply_renewal(&path, &corrupted), Err(RenewalError::SafetyNet { .. })));
    assert_eq!(fs::read_to_string(&path).unwrap(), source);
}

#[test]
fn apply_writes_the_planned_bytes_and_refuses_a_changed_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("note.md");
    let source = "---\nlast_updated: 2026-01-01\n---\nBody\n";
    fs::write(&path, source).unwrap();

    let plan = plan(&fs::read_to_string(&path).unwrap());
    apply_renewal(&path, &plan).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "---\nlast_updated: 2026-09-28\n---\nBody\n");
    let entries = fs::read_dir(directory.path()).unwrap().count();
    assert_eq!(entries, 1, "no temporary file is left behind");

    // The file changed since planning (here, by the first apply): nothing is
    // written, and the error says why.
    let edited = "---\nlast_updated: 2026-09-28\n---\nBody edited by hand\n";
    fs::write(&path, edited).unwrap();
    let error = apply_renewal(&path, &plan).unwrap_err();
    assert!(matches!(error, RenewalError::ModifiedSincePlan { .. }), "{error:?}");
    assert!(error.to_string().contains("changed after the renewal was planned"));
    assert_eq!(fs::read_to_string(&path).unwrap(), edited);

    let missing = directory.path().join("missing.md");
    assert!(matches!(apply_renewal(&missing, &plan), Err(RenewalError::Io { .. })));
    assert!(!missing.exists());
}

#[test]
fn a_plan_with_nothing_to_renew_writes_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("note.md");
    let source = "---\ncontent_policy:\n  - Evergreen\n---\n";
    fs::write(&path, source).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let plan = plan(source);
    apply_renewal(&path, &plan).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), source);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
}

// --- Lifecycle (AC 7, library half) -----------------------------------------

const LIFECYCLE: &str = "---\nlast_updated: 2026-09-28\ncontent_policy:\n  - rule: ValidFor(3mo, @last_updated)\n    action: refresh\n  - rule: ValidUntil(2027-01-01)\n    action: archive\n---\n\n# Body\n";

fn check(bytes: &str, when: &str) -> (Status, Option<Action>, Vec<ResultKind>) {
    let at = date(when).and_hms_opt(0, 0, 0).unwrap().and_utc();
    let report = evaluate_document(bytes.as_bytes(), &EvaluationContext::new(at)).unwrap();
    (report.status, report.action, report.results.iter().map(|result| result.outcome.kind()).collect())
}

/// All eight steps of the spec's lifecycle example, with renewal's clock
/// injected so each `--on` date is not in the future.
#[test]
fn lifecycle_steps_through_the_library() {
    use ResultKind::{NotTriggered, Triggered, Unknown};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("lifecycle.md");
    fs::write(&path, LIFECYCLE).unwrap();
    let read = || fs::read_to_string(&path).unwrap();
    let renew = |on: &str| {
        let bytes = fs::read(&path).unwrap();
        let plan = plan_renewal(&bytes, &context(on).on(date(on))).unwrap();
        apply_renewal(&path, &plan).unwrap();
    };

    // 1 and 2.
    assert_eq!(check(&read(), "2026-12-27"), (Status::Fresh, None, vec![NotTriggered, NotTriggered]));
    assert_eq!(check(&read(), "2026-12-28"), (Status::Stale, Some(Action::Refresh), vec![Triggered, NotTriggered]));
    // 3: only `last_updated` changes.
    renew("2026-12-29");
    assert_eq!(read(), LIFECYCLE.replace("last_updated: 2026-09-28", "last_updated: 2026-12-29"));
    // 4: fresh; the new deadline is 2027-03-29.
    assert_eq!(check(&read(), "2026-12-29"), (Status::Fresh, None, vec![NotTriggered, NotTriggered]));
    // 5: the baseline is in that date's future.
    assert_eq!(check(&read(), "2026-12-28"), (Status::Unknown, None, vec![Unknown, NotTriggered]));
    // 6: the deadline expires the document.
    assert_eq!(check(&read(), "2027-01-01"), (Status::Expired, Some(Action::Archive), vec![NotTriggered, Triggered]));
    // 7: renewal does not move the deadline.
    renew("2027-01-01");
    assert_eq!(read(), LIFECYCLE.replace("last_updated: 2026-09-28", "last_updated: 2027-01-01"));
    assert_eq!(check(&read(), "2027-01-01"), (Status::Expired, Some(Action::Archive), vec![NotTriggered, Triggered]));
    // 8: a `remove` action on the deadline; both entries are still reported.
    let removal = read().replace("action: archive", "action: remove");
    assert_eq!(check(&removal, "2027-01-01"), (Status::Expired, Some(Action::Remove), vec![NotTriggered, Triggered]));
}

// --- Migrated repository documents ------------------------------------------

/// Renewing each of the 23 migrated documents in a temporary copy changes
/// only the `last_updated` value, plus the listed tab repair in the eight
/// tab-indented documents.
#[test]
fn migrated_documents_renew_byte_exactly() {
    let directory = tempfile::tempdir().unwrap();
    let mut tab_repaired = 0;
    for (name, bytes) in common::MIGRATED_DOCUMENTS {
        let original = std::str::from_utf8(bytes).unwrap();
        let path = directory.path().join(format!("{}.md", name.replace('/', "-")));
        fs::write(&path, bytes).unwrap();
        let plan = plan_renewal(bytes, &context("2026-09-29").with_document(name)).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(plan.changes.len(), 1, "{name}");
        assert_eq!(plan.changes[0].target, property("last_updated"), "{name}");
        assert_eq!(plan.changes[0].kind, ChangeKind::Renewed, "{name}");
        apply_renewal(&path, &plan).unwrap();
        let renewed = fs::read_to_string(&path).unwrap();

        if !plan.tab_repair.is_empty() {
            tab_repaired += 1;
        }
        let before: Vec<&str> = original.split_inclusive('\n').collect();
        let after: Vec<&str> = renewed.split_inclusive('\n').collect();
        assert_eq!(before.len(), after.len(), "{name}");
        let mut dated = 0;
        for (old, new) in before.iter().zip(&after) {
            if old == new {
                continue;
            }
            if old.starts_with("last_updated: ") {
                assert_eq!(*new, "last_updated: 2026-09-29\n", "{name}");
                dated += 1;
                continue;
            }
            let indent = old.len() - old.trim_start_matches([' ', '\t']).len();
            let repaired = old[..indent].replace('\t', "  ") + &old[indent..];
            assert!(old.contains('\t') && *new == repaired, "{name}: {old:?} -> {new:?}");
        }
        assert_eq!(dated, 1, "{name}");
        let at = date("2026-09-29").and_hms_opt(0, 0, 0).unwrap().and_utc();
        let report = evaluate_document(renewed.as_bytes(), &EvaluationContext::new(at)).unwrap();
        assert_eq!(report.status, Status::Fresh, "{name}");
        assert!(report.warnings.is_empty(), "{name}: the repaired copy needs no repair");
    }
    assert_eq!(tab_repaired, 8);
}
