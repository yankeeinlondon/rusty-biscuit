//! Gate and pipeline tests over composed fixture areas.
//!
//! Each test writes a minimal claudine-area layout (roster, facts,
//! research doc + sidecar, optional overrides) into a tempdir and drives
//! the same [`claudine_gen::generate_for_area`] pipeline the CLI uses.

use std::fs;
use std::path::Path;

use claudine_gen::{GenError, Provenance, generate_for_area};

/// Mirrors the real agent-models sidecar's mapped subset.
const BASELINE_SIDECAR: &str = r#"$schema:
    last_updated: date(required)
    agent: string(required)
    model: string(required)
    model_selection: "{ method: enum(cli_flag,env_var,config_file,interactive_command,wire_envelope; required), site: string(required), example: string, notes: string }[]"
    dynamic_listing: "{ available: boolean(required), method: string, example: string }"
    requires_claudine_update: boolean(required)
"#;

/// Doctored sidecar: `dynamic_listing.available` became an `enum(...)`
/// whose members are NOT a subset of `ModelCatalogSource`'s variants.
const DOCTORED_SIDECAR: &str = r#"$schema:
    last_updated: date(required)
    agent: string(required)
    model: string(required)
    model_selection: "{ method: enum(cli_flag,env_var,config_file,interactive_command,wire_envelope; required), site: string(required), example: string, notes: string }[]"
    dynamic_listing: "{ available: enum(static, dynamic, telepathic; required), method: string }"
    requires_claudine_update: boolean(required)
"#;

const RESEARCH_DOC: &str = r#"---
$schema: ./_schema.yaml
last_updated: 2026-07-03
agent: test
model: default
model_selection:
  - method: env_var
    site: "ANTHROPIC_MODEL"
  - method: env_var
    site: "COMPOUND_A / COMPOUND_B"
  - method: cli_flag
    site: "--model"
dynamic_listing:
  available: false
requires_claudine_update: false
---

Fixture body.
"#;

/// Valid against the doctored sidecar (`available` is a member string), so
/// the ONLY possible failure is the schema-compatibility gate — proving it
/// fires before value mapping.
const RESEARCH_DOC_ENUM: &str = r#"---
$schema: ./_schema.yaml
last_updated: 2026-07-03
agent: test
model: default
model_selection:
  - method: env_var
    site: "ANTHROPIC_MODEL"
dynamic_listing:
  available: static
requires_claudine_update: false
---

Fixture body.
"#;

const ROSTER: &str = r#"kind: sequence
list:
    - name: Claude Code
      file: claude.md
      slug: claude
      binary: claude
      repo_dir: ".claude"
"#;

const FACTS: &str = "supports_skills: true\n";

struct Fixture {
    sidecar: &'static str,
    research: &'static str,
    facts: String,
    overrides: Option<String>,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            sidecar: BASELINE_SIDECAR,
            research: RESEARCH_DOC,
            facts: FACTS.to_string(),
            overrides: None,
        }
    }
}

fn write_area(dir: &Path, fixture: &Fixture) {
    let write = |rel: &str, content: &str| {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    };
    write("docs/providers.yaml", ROSTER);
    write("docs/providers/facts/claude.yaml", &fixture.facts);
    write("docs/research/agent-models/_schema.yaml", fixture.sidecar);
    write("docs/research/agent-models/claude.md", fixture.research);
    if let Some(overrides) = &fixture.overrides {
        write("docs/providers/overrides/claude.yaml", overrides);
    }
}

fn generate(fixture: &Fixture) -> Result<claudine_gen::Generation, GenError> {
    let dir = tempfile::tempdir().unwrap();
    write_area(dir.path(), fixture);
    generate_for_area(dir.path(), "claude")
}

/// `skip_research: true` keeps the entry's identity in the roster but makes
/// generating it a loud, typed failure — never a silent skip.
#[test]
fn skip_research_roster_entry_is_rejected_loudly() {
    let dir = tempfile::tempdir().unwrap();
    write_area(dir.path(), &Fixture::default());
    let skipped_roster = ROSTER.replace(
        "      repo_dir: \".claude\"\n",
        "      repo_dir: \".claude\"\n      skip_research: true\n",
    );
    fs::write(dir.path().join("docs/providers.yaml"), skipped_roster).unwrap();
    let err = generate_for_area(dir.path(), "claude").unwrap_err();
    match err {
        GenError::RosterEntrySkipped { slug, .. } => assert_eq!(slug, "claude"),
        other => panic!("expected RosterEntrySkipped, got: {other}"),
    }
}

#[test]
fn skeleton_generates_from_all_declared_sources() {
    let generation = generate(&Fixture::default()).unwrap();
    let fragment = &generation.fragment;
    assert!(fragment.contains("    slug: \"claude\",\n"));
    assert!(fragment.contains("    binary: \"claude\",\n"));
    assert!(fragment.contains("    agent_offset: \".claude\",\n"));
    assert!(fragment.contains("    supports_skills: true,\n"));
    assert!(fragment.contains("    dynamic_source: ModelCatalogSource::Static,\n"));
    // env-var coercion: `env_var` records only, bare identifiers only.
    assert!(fragment.contains("    model_env_vars: &[\"ANTHROPIC_MODEL\"],\n"));
}

/// A compound `site` ("A / B") fails the bare-identifier rule; the drop must
/// be collected for the report, never silent (Checkpoint A ruling,
/// 2026-07-04) — even though the field still generates successfully.
#[test]
fn compound_env_var_site_is_skipped_loudly() {
    let generation = generate(&Fixture::default()).unwrap();
    assert_eq!(
        generation.skips,
        vec![claudine_gen::CoercionSkip {
            field: "model_env_vars",
            reason: "site is not a single env-var identifier",
            records: vec!["COMPOUND_A / COMPOUND_B".to_string()],
        }]
    );
}

#[test]
fn doctored_sidecar_enum_not_subset_fails_before_value_mapping() {
    let err = generate(&Fixture {
        sidecar: DOCTORED_SIDECAR,
        research: RESEARCH_DOC_ENUM,
        ..Fixture::default()
    })
    .unwrap_err();
    match err {
        GenError::EnumNotSubset {
            field,
            rust_enum,
            offending,
            ..
        } => {
            assert_eq!(field, "dynamic_source");
            assert_eq!(rust_enum, "ModelCatalogSource");
            // `static` IS a variant; the two invented members are named.
            assert!(offending.contains("dynamic"));
            assert!(offending.contains("telepathic"));
            assert!(!offending.contains("static"));
        }
        other => panic!("expected EnumNotSubset, got: {other}"),
    }
}

#[test]
fn facts_value_for_research_declared_field_is_a_source_collision() {
    let err = generate(&Fixture {
        facts: format!("{FACTS}dynamic_source: static\n"),
        ..Fixture::default()
    })
    .unwrap_err();
    match err {
        GenError::SourceCollision {
            field,
            declared,
            offending,
        } => {
            assert_eq!(field, "dynamic_source");
            assert_eq!(declared, "research");
            assert_eq!(offending, "facts");
        }
        other => panic!("expected SourceCollision, got: {other}"),
    }
}

#[test]
fn facts_key_with_no_registry_entry_is_rejected() {
    let err = generate(&Fixture {
        facts: format!("{FACTS}made_up_field: 1\n"),
        ..Fixture::default()
    })
    .unwrap_err();
    assert!(matches!(err, GenError::UnknownFactsKey { key } if key == "made_up_field"));
}

#[test]
fn override_wins_and_reports_the_suppressed_source_value() {
    let generation = generate(&Fixture {
        overrides: Some(
            "model_env_vars:\n    value: [\"CLAUDE_MODEL\", \"ANTHROPIC_MODEL\"]\n    reason: pin legacy pair\n"
                .to_string(),
        ),
        ..Fixture::default()
    })
    .unwrap();
    assert!(
        generation
            .fragment
            .contains("    model_env_vars: &[\"CLAUDE_MODEL\", \"ANTHROPIC_MODEL\"],\n")
    );
    let field = generation
        .fields
        .iter()
        .find(|f| f.field == "model_env_vars")
        .unwrap();
    match &field.provenance {
        Provenance::Override {
            suppressed, stale, ..
        } => {
            assert_eq!(
                suppressed.as_ref().unwrap(),
                &serde_json::json!(["ANTHROPIC_MODEL"])
            );
            assert!(!stale, "override differs from research; not stale");
        }
        other => panic!("expected Override provenance, got: {other:?}"),
    }
}

#[test]
fn override_equal_to_source_value_is_flagged_stale() {
    let generation = generate(&Fixture {
        overrides: Some(
            "model_env_vars:\n    value: [\"ANTHROPIC_MODEL\"]\n    reason: now redundant\n"
                .to_string(),
        ),
        ..Fixture::default()
    })
    .unwrap();
    let field = generation
        .fields
        .iter()
        .find(|f| f.field == "model_env_vars")
        .unwrap();
    assert!(matches!(
        &field.provenance,
        Provenance::Override { stale: true, .. }
    ));
}

#[test]
fn override_for_unmapped_field_is_rejected() {
    let err = generate(&Fixture {
        overrides: Some("made_up:\n    value: 1\n    reason: nope\n".to_string()),
        ..Fixture::default()
    })
    .unwrap_err();
    assert!(matches!(err, GenError::UnknownOverrideField { field } if field == "made_up"));
}

#[test]
fn override_without_reason_is_rejected() {
    let err = generate(&Fixture {
        overrides: Some("model_env_vars:\n    value: [\"X\"]\n".to_string()),
        ..Fixture::default()
    })
    .unwrap_err();
    assert!(matches!(err, GenError::OverrideMissingReason { field } if field == "model_env_vars"));
}

#[test]
fn research_frontmatter_violating_its_sidecar_fails_loudly() {
    let err = generate(&Fixture {
        // `requires_claudine_update` (required boolean) is missing.
        research: "---\n$schema: ./_schema.yaml\nlast_updated: 2026-07-03\nagent: test\nmodel: default\ndynamic_listing:\n  available: false\n---\n\nBody.\n",
        ..Fixture::default()
    })
    .unwrap_err();
    assert!(matches!(err, GenError::ResearchInvalid { .. }), "got: {err}");
}
