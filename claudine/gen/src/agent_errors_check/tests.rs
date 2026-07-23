use super::*;
use crate::vocabulary::{CodeBucket, KeywordBucket};

fn seed() -> ErrorVocabulary {
    ErrorVocabulary {
        kind_buckets: vec![
            KeywordBucket {
                kind: "api_remote".into(),
                needles: vec!["rate".into(), "quota".into()],
            },
            KeywordBucket {
                kind: "configuration".into(),
                needles: vec!["auth".into()],
            },
            KeywordBucket {
                kind: "api_remote".into(),
                needles: vec!["upstream".into()],
            },
        ],
        msg_buckets: vec![KeywordBucket {
            kind: "configuration".into(),
            needles: vec!["api key".into()],
        }],
        code_buckets: vec![
            CodeBucket {
                code: -32001,
                kind: "configuration".into(),
            },
            CodeBucket {
                code: -32002,
                kind: "api_remote".into(),
            },
        ],
    }
}

fn needle(text: &str, evidence: &str, source: Option<&str>) -> ResearchNeedle {
    ResearchNeedle {
        text: text.into(),
        evidence: evidence.into(),
        source: source.map(str::to_string),
        empirical: None,
    }
}

/// A research doc that preserves every seed, cites non-seed evidence, and
/// covers the capacity class — no findings.
fn clean_research() -> ResearchVocabulary {
    ResearchVocabulary {
        kind_buckets: vec![
            ResearchBucket {
                kind: "api_remote".into(),
                needles: vec![
                    needle("rate", "seed", None),
                    needle("quota", "seed", None),
                    needle("overloaded", "documented", Some("https://example/docs")),
                ],
            },
            ResearchBucket {
                kind: "configuration".into(),
                needles: vec![needle("auth", "seed", None)],
            },
            ResearchBucket {
                kind: "api_remote".into(),
                needles: vec![needle("upstream", "seed", None)],
            },
        ],
        msg_buckets: vec![ResearchBucket {
            kind: "configuration".into(),
            needles: vec![needle("api key", "seed", None)],
        }],
        code_buckets: vec![
            ResearchCodeBucket {
                kind: "configuration".into(),
                codes: vec![ResearchCode {
                    code: -32001,
                    name: Some("AUTH_EXPIRED".into()),
                    evidence: "seed".into(),
                    source: None,
                    empirical: None,
                }],
            },
            ResearchCodeBucket {
                kind: "api_remote".into(),
                codes: vec![ResearchCode {
                    code: -32002,
                    name: Some("REMOTE_ERROR".into()),
                    evidence: "seed".into(),
                    source: None,
                    empirical: None,
                }],
            },
        ],
        gaps: vec![],
    }
}

#[test]
fn clean_research_yields_no_findings() {
    let report = evaluate("codex", Some(&seed()), &clean_research());
    assert!(report.is_clean(), "unexpected findings: {:?}", report.findings);
}

#[test]
fn seed_row_changes_are_classified_and_seed_claims_are_identity_bound() {
    enum Mutation {
        Removal,
        CrossKind,
        RepeatedKindBucketMove,
        IntraBucketReorder,
        BucketReorder,
        NumericRekind,
        NumericReorder,
    }

    let cases = [
        ("removal", Mutation::Removal, Check::SeedRemoval, "quota", false),
        ("cross-kind move", Mutation::CrossKind, Check::SeedRekind, "rate", true),
        (
            "repeated-kind bucket move",
            Mutation::RepeatedKindBucketMove,
            Check::SeedReorder,
            "rate",
            true,
        ),
        (
            "intra-bucket reorder",
            Mutation::IntraBucketReorder,
            Check::SeedReorder,
            "rate",
            true,
        ),
        (
            "bucket reorder",
            Mutation::BucketReorder,
            Check::SeedReorder,
            "rate",
            true,
        ),
        (
            "numeric-code re-kind",
            Mutation::NumericRekind,
            Check::SeedRekind,
            "-32001",
            true,
        ),
        (
            "numeric-code reorder",
            Mutation::NumericReorder,
            Check::SeedReorder,
            "-32001",
            true,
        ),
    ];

    for (name, mutation, expected_check, target, expects_invented_seed) in cases {
        let mut research = clean_research();
        match mutation {
            Mutation::Removal => {
                research.kind_buckets[0].needles.retain(|needle| needle.text != target);
            }
            Mutation::CrossKind => research.kind_buckets[0].kind = "configuration".into(),
            Mutation::RepeatedKindBucketMove => {
                let moved = research.kind_buckets[0].needles.remove(0);
                research.kind_buckets[2].needles.push(moved);
            }
            Mutation::IntraBucketReorder => research.kind_buckets[0].needles.swap(0, 1),
            Mutation::BucketReorder => research.kind_buckets.swap(0, 2),
            Mutation::NumericRekind => {
                research.code_buckets[0].kind = "api_remote".into();
            }
            Mutation::NumericReorder => research.code_buckets.swap(0, 1),
        }

        let report = evaluate("codex", Some(&seed()), &research);
        assert!(
            report.findings.iter().any(|finding| {
                finding.check == expected_check && finding.detail.contains(target)
            }),
            "{name}: expected {expected_check:?} for `{target}`, got {:?}",
            report.findings
        );
        assert_eq!(
            report.findings.iter().any(|finding| {
                finding.check == Check::InventedSeed && finding.detail.contains(target)
            }),
            expects_invented_seed,
            "{name}: moved seed provenance must be checked against the complete row identity"
        );
    }
}

#[test]
fn uppercase_and_whitespace_needles_are_flagged() {
    let mut research = clean_research();
    research.msg_buckets[0]
        .needles
        .push(needle("API Error", "documented", Some("https://x")));
    research.msg_buckets[0]
        .needles
        .push(needle(" padded ", "documented", Some("https://x")));
    let report = evaluate("codex", Some(&seed()), &research);
    let hygiene = report
        .findings
        .iter()
        .filter(|f| f.check == Check::NeedleHygiene)
        .count();
    assert!(hygiene >= 2, "expected hygiene findings, got {:?}", report.findings);
}

#[test]
fn non_seed_needle_without_source_is_flagged() {
    let mut research = clean_research();
    research.kind_buckets[0]
        .needles
        .push(needle("upstream", "source_code", None));
    let report = evaluate("codex", Some(&seed()), &research);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.check == Check::ProvenanceCoherence && f.detail.contains("upstream")),
        "expected a provenance finding, got {:?}",
        report.findings
    );
}

#[test]
fn invented_seed_evidence_is_flagged() {
    let mut research = clean_research();
    // `billing` is NOT in the seed but claims to be a seed.
    research.kind_buckets[0]
        .needles
        .push(needle("billing", "seed", None));
    let report = evaluate("codex", Some(&seed()), &research);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.check == Check::InventedSeed && f.detail.contains("billing")),
        "expected an invented-seed finding, got {:?}",
        report.findings
    );
}

#[test]
fn missing_capacity_class_is_flagged_but_a_gap_satisfies_it() {
    let mut research = clean_research();
    research.kind_buckets[0].needles.retain(|n| n.text != "overloaded");
    let report = evaluate("codex", Some(&seed()), &research);
    assert!(
        report.findings.iter().any(|f| f.check == Check::MotivatingClass),
        "expected a motivating-class finding, got {:?}",
        report.findings
    );

    // Acknowledging the gap clears it, even with no capacity needle.
    research.gaps.push(Gap {
        area: "capacity".into(),
        notes: "could not confirm the 503 overload phrasing in the CLI source".into(),
    });
    let report = evaluate("codex", Some(&seed()), &research);
    assert!(
        !report.findings.iter().any(|f| f.check == Check::MotivatingClass),
        "gap should satisfy motivating-class, got {:?}",
        report.findings
    );
}

#[test]
fn outcome_replacement_covers_findings_then_clean() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("nested").join("codex.json");

    let dirty = FindingsReport {
        status: GateStatus::Findings,
        provider: "codex".into(),
        findings: vec![Finding {
            check: Check::MotivatingClass,
            branch: None,
            detail: "x".into(),
        }],
        error: None,
        error_scope: None,
    };
    write_outcome_report(&path, &dirty).expect("write findings");
    let dirty_text = fs::read_to_string(&path).unwrap();
    assert!(dirty_text.contains("status: findings"));
    assert!(!dirty_text.contains("error_scope:"));

    // A subsequent clean run atomically replaces findings with an explicit
    // clean outcome; clean is never represented by absence.
    let clean = FindingsReport {
        status: GateStatus::Clean,
        provider: "codex".into(),
        findings: vec![],
        error: None,
        error_scope: None,
    };
    write_outcome_report(&path, &clean).expect("clean replace");
    let clean_text = fs::read_to_string(&path).unwrap();
    assert!(clean_text.contains("status: clean"));
    assert!(!clean_text.contains("status: findings"));
    assert!(!clean_text.contains("error_scope:"));
}
