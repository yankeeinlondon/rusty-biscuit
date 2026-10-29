//! Deferred source positions on document-level validation.
//!
//! [`DarkmatterSchemas::validate`] and its siblings build a document's
//! [`PositionMap`] only when the settled report holds a problem, once per
//! report. Each parity case checks that the deferred report is identical to
//! judging the same frontmatter against the eagerly built map
//! ([`positions_for`] with [`EffectiveSchema::validate_with_positions`]);
//! the build counts run with the `work-counters` feature.

use biscuit_file::FileResolutionContext;
use serde_json::json;

use super::*;
use crate::markdown::Markdown;
use crate::markdown::compose::ComposeSource;

fn api(baseline_yaml: &str) -> DarkmatterSchemas {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(baseline_yaml).expect("baseline yaml");
    let baseline = super::simplified::parse_yaml_schema(&value).expect("baseline schema");
    DarkmatterSchemas::new(FileResolutionContext::new(std::env::temp_dir()))
        .with_baseline(baseline)
        .expect("baseline converts")
}

fn bare_api() -> DarkmatterSchemas {
    DarkmatterSchemas::new(FileResolutionContext::new(std::env::temp_dir()))
}

fn doc(text: &str) -> Markdown {
    text.to_string().into()
}

/// The deferred report for `md`, asserted identical to the eager one.
fn assert_matches_eager(api: &DarkmatterSchemas, md: &Markdown) -> ValidationReport {
    let deferred = api.validate(md).expect("validate");
    let effective = api.effective_for(md).expect("effective").expect("schema");
    let mut eager = effective.validate_with_positions(&frontmatter_as_json(md), &positions_for(md));
    eager.trigger_findings = api.discovery_findings().to_vec();
    assert_eq!(format!("{deferred:?}"), format!("{eager:?}"));

    let phased = api
        .validate_for_phase(md, SchemaPhase::Completion)
        .expect("phase");
    let eager_phased = effective
        .validate_for_phase_with_positions(
            &frontmatter_as_json(md),
            &positions_for(md),
            SchemaPhase::Completion,
        )
        .expect("eager phase");
    assert_eq!(
        format!("{:?}", phased.problems),
        format!("{:?}", eager_phased.problems)
    );
    deferred
}

#[test]
fn several_problems_locate_as_the_eager_map_does() {
    let api = api("n: number\nm: \"{ a: number }\"\ntitle: string(required)\n");
    let report = assert_matches_eager(&api, &doc("---\nn: oops\nm:\n  a: x\n  b: 1\n---\n"));
    assert_eq!(report.problems.len(), 3, "{:#?}", report.problems);
    assert!(
        report.problems.iter().all(|problem| problem.line.is_some()),
        "{:#?}",
        report.problems
    );
}

#[test]
fn crlf_aliases_and_root_unions_locate_as_the_eager_map_does() {
    let api_with_alias = api("base: \"{ x: number }\"\nother: \"{ x: number }\"\nn: number\n");
    assert_matches_eager(
        &api_with_alias,
        &doc("---\nbase: &b\n  x: 1\nother: *b\nn:   oops\n---\n"),
    );
    assert_matches_eager(
        &api("m: \"{ a: number }\"\n"),
        &doc("---\r\nm:\r\n  a: x\r\n---\r\n"),
    );
    assert_matches_eager(
        &bare_api(),
        &doc(concat!(
            "---\n",
            "$schema:\n",
            "  - kind: literal(a)\n",
            "    m: \"{ x: number }\"\n",
            "  - kind: literal(b)\n",
            "    n: number(required)\n",
            "kind: a\n",
            "m:\n",
            "  x: oops\n",
            "---\n",
        )),
    );
}

/// Assigned values keep their authored source; a constructed document uses
/// its serialized view; unparsed frontmatter has no positions.
#[test]
fn original_constructed_and_unparsed_sources_locate_as_the_eager_map_does() {
    let api = api(
        "tags: number[]\nm: \"{ a: number, b: number }\"\nextra: number\ntitle: string(required)\n",
    );
    let mut assigned = doc("---\ntags:\n  - 1\n  - 2\nm:\n  a: 1\n---\n");
    let map = assigned.frontmatter_mut().as_map_mut();
    map.insert("tags".into(), json!([1, "replaced", "new"]));
    map.insert("m".into(), json!({ "a": 1, "b": "new" }));
    map.insert("extra".into(), json!("new"));
    let report = assert_matches_eager(&api, &assigned);
    assert!(report.problems.iter().any(|problem| problem.line.is_some()));

    let mut constructed = Markdown::new("body");
    constructed
        .frontmatter_mut()
        .as_map_mut()
        .insert("m".into(), json!({ "a": 1, "c": 2 }));
    assert_matches_eager(&api, &constructed);

    assert_matches_eager(&api, &doc("---\ntitle: [unclosed\n---\nbody\n"));
}

/// A named type imported at two properties reports each use at its own
/// authored value, as the eager map does.
#[test]
fn shared_definitions_locate_each_use_as_the_eager_map_does() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical tempdir");
    std::fs::write(
        root.join("types.yaml"),
        "kind: schema\ntypes:\n  entry: \"{ name: string(required), size: number(min(0)) }\"\n",
    )
    .expect("write types");
    let path = root.join("doc.md");
    std::fs::write(
        &path,
        concat!(
            "---\n",
            "$schema:\n",
            "  first: \"entry@./types.yaml\"\n",
            "  second: \"entry@./types.yaml\"\n",
            "first:\n",
            "  size: -1\n",
            "second:\n",
            "  name: x\n",
            "  size: -2\n",
            "---\n",
        ),
    )
    .expect("write document");
    let md = Markdown::try_from(path.as_path())
        .expect("document")
        .with_source(ComposeSource::File(path));
    let api = DarkmatterSchemas::new(FileResolutionContext::new(&root));

    let report = assert_matches_eager(&api, &md);
    let located: Vec<_> = report
        .problems
        .iter()
        .map(|problem| (problem.path.as_str(), problem.line, problem.column))
        .collect();
    assert!(
        located.contains(&("/first/size", Some(6), Some(9))),
        "{located:?}"
    );
    assert!(
        located.contains(&("/second/size", Some(9), Some(9))),
        "{located:?}"
    );
}

#[cfg(feature = "work-counters")]
mod counts {
    use crate::work_counters::{Work, snapshot};

    use super::*;

    /// Position maps built while `run` runs.
    fn position_builds<T>(run: impl FnOnce() -> T) -> (T, u64) {
        let before = snapshot();
        let value = run();
        (value, snapshot().since(&before).get(Work::PositionMapBuild))
    }

    #[test]
    fn a_valid_baseline_only_document_builds_no_position_map() {
        let api = bare_api()
            .with_darkmatter_baseline_json_schema()
            .expect("embedded baseline");
        let md = doc("---\ntitle: plain\ndescription: a page\n---\nbody\n");
        let ((report, summary), builds) =
            position_builds(|| api.validate_summarized(&md).expect("validate"));
        assert!(report.valid, "{:#?}", report.problems);
        assert!(summary.has_schema);
        assert_eq!(builds, 0);
    }

    #[test]
    fn a_document_without_a_schema_builds_no_position_map() {
        let md = doc("---\ntitle: plain\n---\nbody\n");
        let ((report, summary), builds) =
            position_builds(|| bare_api().validate_summarized(&md).expect("validate"));
        assert!(report.valid);
        assert!(!summary.has_schema);
        assert_eq!(builds, 0);
    }

    #[test]
    fn several_problems_share_one_position_map() {
        let api = api("n: number\nm: \"{ a: number }\"\ntitle: string(required)\n");
        let md = doc("---\nn: oops\nm:\n  a: x\n  b: 1\n---\n");
        let (report, builds) = position_builds(|| api.validate(&md).expect("validate"));
        assert_eq!(report.problems.len(), 3, "{:#?}", report.problems);
        assert_eq!(builds, 1);
        let (phased, builds) = position_builds(|| {
            api.validate_for_phase(&md, SchemaPhase::Launch)
                .expect("phase")
        });
        assert!(!phased.valid);
        assert_eq!(builds, 1);
    }

    /// The first arm's problems are discarded once the second accepts, so
    /// they are never located.
    #[test]
    fn a_root_union_accepted_by_a_later_arm_builds_no_position_map() {
        let md =
            doc("---\n$schema:\n  - a: number(required)\n  - b: number(required)\nb: 1\n---\n");
        let (report, builds) = position_builds(|| bare_api().validate(&md).expect("validate"));
        assert!(report.valid, "{:#?}", report.problems);
        assert_eq!(builds, 0);
    }

    #[test]
    fn a_pending_shell_value_builds_no_position_map() {
        let md = doc("---\n$schema:\n  when: string(required)\nwhen: $(date)\n---\n");
        let (report, builds) = position_builds(|| bare_api().validate(&md).expect("validate"));
        assert!(report.valid, "{:#?}", report.problems);
        assert_eq!(builds, 0);
    }

    #[test]
    fn a_preparation_failure_builds_no_position_map() {
        let md = doc("---\n$schema: ./missing-lazy-positions-schema.yaml\ntitle: 1\n---\n");
        let (result, builds) = position_builds(|| bare_api().validate(&md));
        assert!(result.is_err());
        assert_eq!(builds, 0);
    }
}
