use std::fs;

use super::*;

fn empirical_research(empirical: Option<EmpiricalEvidence>) -> ResearchVocabulary {
    ResearchVocabulary {
        kind_buckets: vec![ResearchBucket {
            kind: "api_remote".into(),
            needles: vec![ResearchNeedle {
                text: "overloaded".into(),
                evidence: "empirical".into(),
                source: Some("https://example.com/provider/capture-contract".into()),
                empirical,
            }],
        }],
        ..ResearchVocabulary::default()
    }
}

fn provenance_findings(report: &FindingsReport) -> Vec<&Finding> {
    report
        .findings
        .iter()
        .filter(|finding| finding.check == Check::ProvenanceCoherence)
        .collect()
}

#[test]
fn empirical_provenance_requires_a_resolvable_scoped_fixture_and_capture_notes() {
    let topic = tempfile::tempdir().expect("topic tempdir");
    let fixtures = topic.path().join("_fixtures");
    fs::create_dir(&fixtures).expect("fixture directory");
    fs::write(fixtures.join("capacity.json"), b"{\"error\":\"scrubbed\"}\n")
        .expect("scrubbed fixture");

    let valid = empirical_research(Some(EmpiricalEvidence {
        fixture: "./_fixtures/capacity.json".into(),
        capture_notes: "Captured from provider v1.2; credentials and IDs removed.".into(),
    }));
    let report = evaluate_with_fixture_base("codex", None, &valid, Some(topic.path()));
    assert!(provenance_findings(&report).is_empty(), "{:?}", report.findings);

    let invalid = [
        empirical_research(None),
        empirical_research(Some(EmpiricalEvidence {
            fixture: "./_fixtures/capacity.json".into(),
            capture_notes: " ".into(),
        })),
        empirical_research(Some(EmpiricalEvidence {
            fixture: "./_fixtures/../raw.json".into(),
            capture_notes: "Captured and scrubbed.".into(),
        })),
        empirical_research(Some(EmpiricalEvidence {
            fixture: "./_fixtures/missing.json".into(),
            capture_notes: "Captured and scrubbed.".into(),
        })),
    ];
    for research in invalid {
        let report =
            evaluate_with_fixture_base("codex", None, &research, Some(topic.path()));
        assert!(!provenance_findings(&report).is_empty(), "{:?}", report.findings);
    }
}

#[test]
fn motivating_class_reads_numeric_code_values() {
    for code in [429, 503] {
        let research = ResearchVocabulary {
            code_buckets: vec![ResearchCodeBucket {
                kind: "api_remote".into(),
                codes: vec![ResearchCode {
                    code,
                    name: None,
                    evidence: "documented".into(),
                    source: Some("https://example.com/provider/errors".into()),
                    empirical: None,
                }],
            }],
            ..ResearchVocabulary::default()
        };
        let mut findings = Vec::new();
        check_motivating_class(&research, &mut findings);
        assert!(findings.is_empty(), "numeric {code} should cover the class");
    }

    let research = ResearchVocabulary {
        code_buckets: vec![ResearchCodeBucket {
            kind: "api_remote".into(),
            codes: vec![ResearchCode {
                code: 500,
                name: None,
                evidence: "documented".into(),
                source: Some("https://example.com/provider/errors".into()),
                empirical: None,
            }],
        }],
        ..ResearchVocabulary::default()
    };
    let mut findings = Vec::new();
    check_motivating_class(&research, &mut findings);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].check, Check::MotivatingClass);
}

#[test]
fn empirical_schema_fixture_loads_and_resolves() {
    let area = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen crate lives under the claudine package area");
    let topic = area.join("docs/research/agent-errors");
    let frontmatter = inputs::load_validated_frontmatter(
        &topic.join("_fixtures/research-shaped.md"),
    )
    .expect("empirical fixture should satisfy the sidecar");
    let research = parse_research("fixture", &frontmatter).expect("typed empirical fixture");
    let empirical = research.msg_buckets[1].needles[3]
        .empirical
        .as_ref()
        .expect("empirical capture object");

    let mut findings = Vec::new();
    check_empirical_provenance(
        Branch::Msg,
        "temporarily unavailable",
        Some(empirical),
        Some(&topic),
        &mut findings,
    );
    assert!(findings.is_empty(), "{findings:?}");
}
