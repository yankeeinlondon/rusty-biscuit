//! Gate and pipeline tests over composed fixture areas.
//!
//! Each test copies the REAL claude inputs (roster, facts, research docs +
//! sidecars for every registry topic) into a tempdir, doctors the piece
//! under test, and drives the same [`claudine_gen::generate_for_area`]
//! pipeline the CLI uses. Using the real inputs keeps the fixtures honest:
//! they must satisfy the same sidecar schemas production does.

use std::fs;
use std::path::{Path, PathBuf};

use claudine_gen::{GenError, Provenance, generate_for_area};
use darkmatter::markdown::compose::find_git_root_from;

/// Research topics the registry consumes (fixture copy set).
const TOPICS: &[&str] = &[
    "acp",
    "agent-cli",
    "agent-logging",
    "agent-models",
    "model-config",
    "non-interactive-sessions",
    "resume",
    "skills",
];

/// The unchained-ai artifact's workspace-relative location.
const ARTIFACT_REL: &str = "unchained-ai/artifacts/models-catalog.json";

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
    /// Copies the real claude inputs into a fresh workspace-shaped tempdir
    /// (deliberately WITHOUT the overrides file — override behavior is
    /// exercised explicitly per test). The area lives at `<tmp>/claudine`
    /// so the workspace-relative unchained-ai artifact resolves at
    /// `<tmp>/unchained-ai/artifacts/models-catalog.json`.
    fn new() -> Self {
        Self::from_dir(tempfile::tempdir().unwrap())
    }

    fn new_in(parent: &Path) -> Self {
        Self::from_dir(tempfile::tempdir_in(parent).unwrap())
    }

    fn from_dir(dir: tempfile::TempDir) -> Self {
        let real = real_area();
        let copy = |rel: &str| {
            let to = dir.path().join("claudine").join(rel);
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::copy(real.join(rel), to)
                .unwrap_or_else(|err| panic!("fixture copy of `{rel}` failed: {err}"));
        };
        copy("docs/providers.yaml");
        copy("docs/providers/facts/claude.yaml");
        for topic in TOPICS {
            copy(&format!("docs/research/{topic}/_schema.yaml"));
            copy(&format!("docs/research/{topic}/claude.md"));
        }
        let artifact_to = dir.path().join(ARTIFACT_REL);
        fs::create_dir_all(artifact_to.parent().unwrap()).unwrap();
        fs::copy(
            real.parent()
                .expect("area lives under the workspace root")
                .join(ARTIFACT_REL),
            artifact_to,
        )
        .unwrap_or_else(|err| panic!("fixture copy of `{ARTIFACT_REL}` failed: {err}"));
        Self { dir }
    }

    fn area(&self) -> PathBuf {
        self.dir.path().join("claudine")
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.area().join(rel)
    }

    fn append(&self, rel: &str, extra: &str) {
        let path = self.path(rel);
        let mut content = fs::read_to_string(&path).unwrap();
        content.push_str(extra);
        fs::write(path, content).unwrap();
    }

    fn replace(&self, rel: &str, from: &str, to: &str) {
        let path = self.path(rel);
        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.contains(from),
            "fixture replace: `{from}` not found in {rel}"
        );
        fs::write(path, content.replace(from, to)).unwrap();
    }

    fn write(&self, rel: &str, content: &str) {
        let path = self.path(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn generate(&self) -> Result<claudine_gen::Generation, GenError> {
        generate_for_area(&self.area(), "claude")
    }
}

/// `skip_research: true` keeps the entry's identity in the roster but makes
/// generating it a loud, typed failure — never a silent skip.
#[test]
fn skip_research_roster_entry_is_rejected_loudly() {
    let fixture = Fixture::new();
    fixture.replace(
        "docs/providers.yaml",
        "      slug: claude\n",
        "      slug: claude\n      skip_research: true\n",
    );
    let err = fixture.generate().unwrap_err();
    match err {
        GenError::RosterEntrySkipped { slug, .. } => assert_eq!(slug, "claude"),
        other => panic!("expected RosterEntrySkipped, got: {other}"),
    }
}

#[test]
fn pipeline_generates_from_all_declared_sources() {
    let fixture = Fixture::new();
    assert!(
        find_git_root_from(&fixture.area()).is_none(),
        "the no-repository fixture exercises the discovery-compatible fallback"
    );
    let generation = fixture.generate().unwrap();
    let data_rs = &generation.data_rs;
    // Roster-fed identity.
    assert!(data_rs.contains("    slug: \"claude\",\n"));
    assert!(data_rs.contains("    binary: \"claude\",\n"));
    assert!(data_rs.contains("    agent_offset: \".claude\",\n"));
    // Research-fed values (skills enum→bool, agent-models listing + env
    // vars — no override in the fixture, so the research value shows).
    assert!(data_rs.contains("    supports_skills: true,\n"));
    assert!(data_rs.contains("    model_catalog_source: ModelCatalogSource::None,\n"));
    assert!(data_rs.contains("\"ANTHROPIC_MODEL\""));
    // Research + artifact join: expected offerings carry classification
    // and identity-key joins; local runners become offering sources.
    assert!(data_rs.contains("id: \"claude-opus-4-8\","));
    assert!(data_rs.contains("catalog_id: Some(\"anthropic/claude-opus@4.8\"),"));
    assert!(data_rs.contains("class: OfferingClass::VendorApi,"));
    assert!(data_rs.contains("prefix: \"ollama\","));
    assert!(data_rs.contains("integration: Some(LocalRunnerIntegration::FirstClass),"));
    // Facts-fed values.
    assert!(data_rs.contains("    stream_protocol: Some(StreamProtocol::StreamJson),\n"));
    assert!(data_rs.contains("pub(in crate::provider) static CLAUDE_EVENT_MAPPING"));
    // Mixed-source acp record: research-fed server_mode (acp topic
    // `support`) joined with the facts client/events halves.
    assert!(data_rs.contains("        server_mode: AcpServerMode::Adapter,\n"));
    assert!(data_rs.contains("        client_supported: false,\n"));
}

#[test]
fn relative_area_inside_a_repository_uses_one_absolute_resolution_boundary() {
    let cwd = std::env::current_dir().expect("current directory");
    let fixture = Fixture::new_in(&cwd);
    fs::create_dir(fixture.dir.path().join(".git")).expect("temporary repository marker");
    let relative_area = fixture
        .area()
        .strip_prefix(&cwd)
        .expect("fixture was created below the current directory")
        .to_path_buf();

    let generation = generate_for_area(&relative_area, "claude")
        .expect("relative area and schema paths must share one absolute boundary");

    assert!(generation.data_rs.contains("    slug: \"claude\",\n"));
}

/// The unchained-ai artifact is a hard generation input: absence and a
/// schema_version bump both fail loudly at the pipeline level.
#[test]
fn missing_or_mismatched_artifact_fails_loudly() {
    let fixture = Fixture::new();
    let artifact_path = fixture.dir.path().join(ARTIFACT_REL);

    let current = format!(
        "\"schema_version\": {}",
        claudine_gen::artifact::EXPECTED_SCHEMA_VERSION
    );
    let original = fs::read_to_string(&artifact_path).unwrap();
    let doctored = original.replace(&current, "\"schema_version\": 99");
    assert_ne!(
        original, doctored,
        "fixture artifact does not carry the expected schema_version marker"
    );
    fs::write(&artifact_path, doctored).unwrap();
    let err = fixture.generate().unwrap_err();
    assert!(
        matches!(
            err,
            GenError::ArtifactSchemaVersion {
                found: 99,
                expected: claudine_gen::artifact::EXPECTED_SCHEMA_VERSION,
                ..
            }
        ),
        "expected ArtifactSchemaVersion, got: {err}"
    );

    fs::remove_file(&artifact_path).unwrap();
    let err = fixture.generate().unwrap_err();
    assert!(
        matches!(err, GenError::ArtifactMissing { .. }),
        "expected ArtifactMissing, got: {err}"
    );
}

/// Compound `site` records ("A / B") fail the bare-identifier rule; the
/// drop must be collected for the report, never silent (Checkpoint A
/// ruling, 2026-07-04) — even though the field still generates.
#[test]
fn compound_env_var_sites_are_skipped_loudly() {
    let generation = Fixture::new().generate().unwrap();
    let skip = generation
        .skips
        .iter()
        .find(|skip| skip.field == "model_env_vars")
        .expect("claude research carries compound env-var sites");
    assert_eq!(skip.reason, "site is not a single env-var identifier");
    assert!(!skip.records.is_empty());
}

#[test]
fn doctored_sidecar_enum_not_subset_fails_before_value_mapping() {
    let fixture = Fixture::new();
    // Doctor the sidecar: `dynamic_listing.available` becomes an enum whose
    // members are NOT a subset of ModelCatalogSource's variants — and keep
    // the research doc valid against it (`available: none`) so the ONLY
    // possible failure is the schema-compatibility gate.
    fixture.replace(
        "docs/research/agent-models/_schema.yaml",
        "dynamic_listing: \"{ available: boolean(required)",
        "dynamic_listing: \"{ available: enum(none, dynamic, telepathic; required)",
    );
    fixture.replace(
        "docs/research/agent-models/claude.md",
        "  available: false",
        "  available: none",
    );
    let err = fixture.generate().unwrap_err();
    match err {
        GenError::EnumNotSubset {
            field,
            rust_enum,
            offending,
            ..
        } => {
            assert_eq!(field, "model_catalog_source");
            assert_eq!(rust_enum, "ModelCatalogSource");
            // `none` IS a variant; the two invented members are named.
            assert!(offending.contains("dynamic"));
            assert!(offending.contains("telepathic"));
            assert!(!offending.contains("none"));
        }
        other => panic!("expected EnumNotSubset, got: {other}"),
    }
}

#[test]
fn facts_value_for_research_declared_field_is_a_source_collision() {
    let fixture = Fixture::new();
    fixture.append("docs/providers/facts/claude.yaml", "model_catalog_source: none\n");
    let err = fixture.generate().unwrap_err();
    match err {
        GenError::SourceCollision {
            field,
            declared,
            offending,
        } => {
            assert_eq!(field, "model_catalog_source");
            assert_eq!(declared, "research");
            assert_eq!(offending, "facts");
        }
        other => panic!("expected SourceCollision, got: {other}"),
    }
}

/// The graduation gate (delete-on-graduate): `supports_skills` moved
/// facts → research at v1, so a facts file still carrying the key is a
/// loud source collision, not a silently shadowed value.
#[test]
fn graduated_supports_skills_in_facts_is_a_source_collision() {
    let fixture = Fixture::new();
    fixture.append("docs/providers/facts/claude.yaml", "supports_skills: true\n");
    let err = fixture.generate().unwrap_err();
    match err {
        GenError::SourceCollision {
            field,
            declared,
            offending,
        } => {
            assert_eq!(field, "supports_skills");
            assert_eq!(declared, "research");
            assert_eq!(offending, "facts");
        }
        other => panic!("expected SourceCollision, got: {other}"),
    }
}

/// The acp graduation gate (2026-07-05, delete-on-graduate for the
/// sub-field): a facts `acp` record still carrying `server_mode` is a loud
/// source collision, not a silently shadowed value.
#[test]
fn graduated_acp_server_mode_in_facts_is_a_source_collision() {
    let fixture = Fixture::new();
    fixture.replace(
        "docs/providers/facts/claude.yaml",
        "acp:\n  client_supported: false",
        "acp:\n  server_mode: adapter\n  client_supported: false",
    );
    let err = fixture.generate().unwrap_err();
    match err {
        GenError::SourceCollision {
            field,
            declared,
            offending,
        } => {
            assert_eq!(field, "acp.server_mode");
            assert_eq!(declared, "research");
            assert_eq!(offending, "facts");
        }
        other => panic!("expected SourceCollision, got: {other}"),
    }
}

#[test]
fn facts_key_with_no_registry_entry_is_rejected() {
    let fixture = Fixture::new();
    fixture.append("docs/providers/facts/claude.yaml", "made_up_field: 1\n");
    let err = fixture.generate().unwrap_err();
    assert!(matches!(err, GenError::UnknownFactsKey { key } if key == "made_up_field"));
}

#[test]
fn override_wins_and_reports_the_suppressed_source_value() {
    let fixture = Fixture::new();
    fixture.write(
        "docs/providers/overrides/claude.yaml",
        "model_env_vars:\n    value: [\"CLAUDE_MODEL\", \"ANTHROPIC_MODEL\"]\n    reason: pin legacy pair\n",
    );
    let generation = fixture.generate().unwrap();
    assert!(
        generation
            .data_rs
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
            let suppressed = suppressed.as_ref().expect("research value resolvable");
            let suppressed: Vec<&str> = suppressed
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            assert!(suppressed.contains(&"ANTHROPIC_MODEL"));
            assert!(!stale, "override differs from research; not stale");
        }
        other => panic!("expected Override provenance, got: {other:?}"),
    }
}

#[test]
fn override_equal_to_source_value_is_flagged_stale() {
    let fixture = Fixture::new();
    // First resolve the research value, then pin an identical override.
    let baseline = fixture.generate().unwrap();
    let research_value = &baseline
        .fields
        .iter()
        .find(|f| f.field == "model_env_vars")
        .unwrap()
        .value;
    fixture.write(
        "docs/providers/overrides/claude.yaml",
        &format!(
            "model_env_vars:\n    value: {research_value}\n    reason: now redundant\n"
        ),
    );
    let generation = fixture.generate().unwrap();
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
    let fixture = Fixture::new();
    fixture.write(
        "docs/providers/overrides/claude.yaml",
        "made_up:\n    value: 1\n    reason: nope\n",
    );
    let err = fixture.generate().unwrap_err();
    assert!(matches!(err, GenError::UnknownOverrideField { field } if field == "made_up"));
}

#[test]
fn override_without_reason_is_rejected() {
    let fixture = Fixture::new();
    fixture.write(
        "docs/providers/overrides/claude.yaml",
        "model_env_vars:\n    value: [\"X\"]\n",
    );
    let err = fixture.generate().unwrap_err();
    assert!(matches!(err, GenError::OverrideMissingReason { field } if field == "model_env_vars"));
}

#[test]
fn research_frontmatter_violating_its_sidecar_fails_loudly() {
    let fixture = Fixture::new();
    // `requires_claudine_update` is a required boolean in every topic
    // sidecar; dropping it must fail validation before any mapping.
    fixture.replace(
        "docs/research/agent-models/claude.md",
        "requires_claudine_update:",
        "requires_claudine_update_renamed:",
    );
    let err = fixture.generate().unwrap_err();
    assert!(matches!(err, GenError::ResearchInvalid { .. }), "got: {err}");
}

/// `dynamic_listing.available: true` cannot select a catalog mechanism;
/// with no override pinning one, generation fails loudly (the
/// codex/kimi/opencode situation before their overrides).
#[test]
fn dynamic_listing_true_without_override_fails_loudly() {
    let fixture = Fixture::new();
    fixture.replace(
        "docs/research/agent-models/claude.md",
        "  available: false",
        "  available: true",
    );
    let err = fixture.generate().unwrap_err();
    assert!(
        matches!(err, GenError::UnmappableValue { field, .. } if field == "model_catalog_source"),
        "expected UnmappableValue for model_catalog_source"
    );
}

/// A `shell_command` override authored in the externally tagged object
/// form emits the struct-variant expression with `program` and `args`.
#[test]
fn shell_command_override_emits_struct_variant() {
    let fixture = Fixture::new();
    fixture.write(
        "docs/providers/overrides/claude.yaml",
        "model_catalog_source:\n    value:\n        shell_command:\n            program: opencode\n            args: [\"models\"]\n    reason: exercise the data variant\n",
    );
    let generation = fixture.generate().unwrap();
    assert!(
        generation.data_rs.contains(
            "    model_catalog_source: ModelCatalogSource::ShellCommand {\n        \
             program: \"opencode\",\n        args: &[\"models\"],\n    },\n"
        ),
        "struct-variant expression missing from:\n{}",
        generation.data_rs
    );
}

/// `shell_command` as a bare member string is rejected loudly — only the
/// object form can carry `program`/`args`.
#[test]
fn bare_shell_command_override_fails_loudly() {
    let fixture = Fixture::new();
    fixture.write(
        "docs/providers/overrides/claude.yaml",
        "model_catalog_source:\n    value: shell_command\n    reason: wrong shape on purpose\n",
    );
    let err = fixture.generate().unwrap_err();
    assert!(
        matches!(err, GenError::UnmappableValue { field, .. } if field == "model_catalog_source"),
        "expected UnmappableValue for bare shell_command"
    );
}
