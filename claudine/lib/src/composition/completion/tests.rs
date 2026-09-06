use super::*;
use crate::composition::schema::PropertyState;
use crate::diagnostics::Diagnostic;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// The voip.md schema as authored: `prompt` eager, three runtime obligations,
/// one optional property.
const VOIP_SCHEMA: &str = concat!(
    "---\n",
    "$schema:\n",
    "  prompt: 'string(required;eager)'\n",
    "  last_updated: 'string(required)'\n",
    "  researched_by: 'string(required)'\n",
    "  products: 'object(required)'\n",
    "  notes: 'string'\n",
    "---\n",
    "Body\n",
);

fn launch_schema(document: &str) -> LaunchSchema {
    let markdown: Markdown = document.to_string().into();
    let effective = DarkmatterSchemas::new()
        .effective_for(&markdown)
        .expect("the fixture schema should resolve")
        .expect("the fixture declares a SimplifiedSchema");
    LaunchSchema {
        effective,
        phase: Some(SchemaPhase::Launch),
        report: None,
    }
}

fn doc_path() -> PathBuf {
    PathBuf::from("/tmp/voip.md")
}

fn satisfied_instance() -> serde_json::Value {
    serde_json::json!({
        "prompt": "research voip",
        "last_updated": "2026-09-06",
        "researched_by": "opencode",
        "products": { "uk": 10 },
    })
}

fn property_state(verdict: &CompletionVerdict, name: &str) -> PropertyState {
    verdict
        .status
        .as_ref()
        .expect("a SimplifiedSchema exposes a property table")
        .required
        .iter()
        .chain(
            verdict
                .status
                .as_ref()
                .unwrap()
                .optional
                .iter(),
        )
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("{name} should appear in the status report"))
        .state
}

// -- the pure evaluator -----------------------------------------------------

#[test]
fn a_document_without_a_schema_completes_when_its_body_changed() {
    let verdict = evaluate_completion(
        None,
        &serde_json::json!({ "anything": 1 }),
        BodyEvidence::Changed,
        &doc_path(),
    );

    assert!(verdict.is_satisfied());
    assert!(verdict.status.is_none());
    assert!(verdict.problems.is_empty());
    assert!(verdict.into_error().is_none());
}

#[test]
fn a_satisfied_instance_completes_and_reports_every_property() {
    let schema = launch_schema(VOIP_SCHEMA);

    let verdict = evaluate_completion(
        Some(&schema),
        &satisfied_instance(),
        BodyEvidence::Changed,
        &doc_path(),
    );

    assert!(verdict.is_satisfied(), "{:?}", verdict.problems);
    // The author sees the whole schema, not only the failures.
    for name in ["prompt", "last_updated", "researched_by", "products"] {
        assert_eq!(property_state(&verdict, name), PropertyState::Valid);
    }
    assert_eq!(property_state(&verdict, "notes"), PropertyState::Missing);
}

#[test]
fn a_required_property_the_agent_never_set_fails_completion() {
    let schema = launch_schema(VOIP_SCHEMA);
    let mut instance = satisfied_instance();
    instance.as_object_mut().unwrap().remove("products");

    let verdict = evaluate_completion(
        Some(&schema),
        &instance,
        BodyEvidence::Changed,
        &doc_path(),
    );

    assert!(!verdict.is_satisfied());
    assert_eq!(
        verdict
            .problems
            .iter()
            .map(|problem| (problem.property.as_str(), problem.kind))
            .collect::<Vec<_>>(),
        [("products", CompletionProblemKind::Missing)]
    );
    assert_eq!(property_state(&verdict, "products"), PropertyState::Missing);
    assert_eq!(
        property_state(&verdict, "researched_by"),
        PropertyState::Valid
    );

    let error = verdict.into_error().expect("a failed verdict carries an err");
    assert_eq!(error.code(), "composition.completion_schema");
    let detail = error.detail();
    assert_eq!(detail["properties"][0]["property"], "products");
    assert_eq!(detail["properties"][0]["kind"], "missing");
}

#[test]
fn an_explicit_null_is_absence_at_completion() {
    let schema = launch_schema(VOIP_SCHEMA);
    let mut instance = satisfied_instance();
    instance.as_object_mut().unwrap()["researched_by"] = serde_json::Value::Null;

    let verdict = evaluate_completion(
        Some(&schema),
        &instance,
        BodyEvidence::Changed,
        &doc_path(),
    );

    assert!(!verdict.is_satisfied());
    assert_eq!(verdict.problems[0].property, "researched_by");
}

#[test]
fn a_value_of_the_wrong_type_fails_completion_with_the_type_message() {
    let schema = launch_schema(VOIP_SCHEMA);
    let mut instance = satisfied_instance();
    instance.as_object_mut().unwrap()["products"] = serde_json::json!("a string");

    let verdict = evaluate_completion(
        Some(&schema),
        &instance,
        BodyEvidence::Changed,
        &doc_path(),
    );

    assert!(!verdict.is_satisfied());
    assert_eq!(verdict.problems[0].property, "products");
    assert_eq!(verdict.problems[0].kind, CompletionProblemKind::Invalid);
    assert_eq!(property_state(&verdict, "products"), PropertyState::Invalid);
    let error = verdict.into_error().unwrap();
    assert_eq!(error.code(), "composition.completion_schema");
    assert_eq!(error.detail()["properties"][0]["kind"], "invalid");
}

#[test]
fn problems_follow_the_status_tables_declaration_order() {
    let schema = launch_schema(VOIP_SCHEMA);
    // A mix of a missing property and a present-but-wrong one, so the
    // validator's own emission order is not the table's order.
    let instance = serde_json::json!({
        "prompt": "research voip",
        "products": "a string",
    });

    let verdict = evaluate_completion(
        Some(&schema),
        &instance,
        BodyEvidence::Changed,
        &doc_path(),
    );

    let table: Vec<String> = verdict
        .status
        .as_ref()
        .unwrap()
        .required
        .iter()
        .chain(verdict.status.as_ref().unwrap().optional.iter())
        .map(|entry| entry.name.clone())
        .collect();
    let reported: Vec<String> = verdict
        .problems
        .iter()
        .map(|problem| problem.property.clone())
        .collect();
    assert_eq!(reported.len(), 3, "{reported:?}");
    let positions: Vec<usize> = reported
        .iter()
        .map(|name| table.iter().position(|entry| entry == name).unwrap())
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "problems {reported:?} must follow the table order {table:?}"
    );
}

#[test]
fn a_transient_caller_value_satisfies_completion_and_removing_it_does_not() {
    let schema = launch_schema(VOIP_SCHEMA);
    // `researched_by` came from `--set` / sequence state / `proxy.with`; it is
    // in the live effective map and never in the file.
    let with_overlay = satisfied_instance();
    let mut without_overlay = with_overlay.clone();
    without_overlay
        .as_object_mut()
        .unwrap()
        .remove("researched_by");

    assert!(
        evaluate_completion(
            Some(&schema),
            &with_overlay,
            BodyEvidence::Changed,
            &doc_path()
        )
        .is_satisfied()
    );
    assert!(
        !evaluate_completion(
            Some(&schema),
            &without_overlay,
            BodyEvidence::Changed,
            &doc_path()
        )
        .is_satisfied()
    );
}

#[test]
fn body_rejection_outranks_schema_problems() {
    let schema = launch_schema(VOIP_SCHEMA);
    let instance = serde_json::json!({});

    let verdict = evaluate_completion(
        Some(&schema),
        &instance,
        BodyEvidence::Rejected(BodyRejection::Unchanged),
        &doc_path(),
    );

    assert!(!verdict.is_satisfied());
    assert!(!verdict.problems.is_empty(), "the schema is still reported");
    let error = verdict.into_error().unwrap();
    assert_eq!(error.code(), "composition.body_unchanged");
    assert_eq!(error.detail()["reason"], "unchanged");
}

#[test]
fn an_empty_body_is_reported_as_its_own_reason() {
    let verdict = evaluate_completion(
        None,
        &serde_json::json!({}),
        BodyEvidence::Rejected(BodyRejection::Empty),
        &doc_path(),
    );

    let error = verdict.into_error().unwrap();
    assert_eq!(error.code(), "composition.body_unchanged");
    assert_eq!(error.detail()["reason"], "empty");
}

#[test]
fn validation_performs_no_filesystem_work_for_an_eager_file_property() {
    // An eager `file` property is a launch-time existence check. At completion
    // the validator is passive, so a reference to a file that does not exist
    // must not be probed — and must not fail.
    let schema = launch_schema(concat!(
        "---\n",
        "$schema:\n",
        "  spec: 'file(required;eager)'\n",
        "---\n",
        "Body\n",
    ));

    let verdict = evaluate_completion(
        Some(&schema),
        &serde_json::json!({ "spec": "./definitely-not-here-9e1f.md" }),
        BodyEvidence::Changed,
        &doc_path(),
    );

    assert!(verdict.is_satisfied(), "{:?}", verdict.problems);
}

#[test]
fn a_nested_property_failure_names_its_full_path() {
    let schema = launch_schema(concat!(
        "---\n",
        "$schema:\n",
        "  products: '{ uk_price: number(required), name: string }[](required)'\n",
        "---\n",
        "Body\n",
    ));

    let missing = evaluate_completion(
        Some(&schema),
        &serde_json::json!({ "products": [{ "name": "handset" }] }),
        BodyEvidence::Changed,
        &doc_path(),
    );
    assert!(!missing.is_satisfied());
    assert_eq!(missing.problems[0].property, "products[0].uk_price");
    assert_eq!(missing.problems[0].kind, CompletionProblemKind::Missing);

    let wrong_type = evaluate_completion(
        Some(&schema),
        &serde_json::json!({ "products": [{ "uk_price": 10 }, { "uk_price": "ten" }] }),
        BodyEvidence::Changed,
        &doc_path(),
    );
    assert!(!wrong_type.is_satisfied());
    assert_eq!(wrong_type.problems[0].property, "products[1].uk_price");
    assert_eq!(wrong_type.problems[0].kind, CompletionProblemKind::Invalid);

    assert!(
        evaluate_completion(
            Some(&schema),
            &serde_json::json!({ "products": [{ "uk_price": 10, "name": "handset" }] }),
            BodyEvidence::Changed,
            &doc_path(),
        )
        .is_satisfied()
    );
}

/// AC15: raw JSON Schema has no phase vocabulary, so its authored `required`
/// entries are enforced at completion exactly as they are at launch. There is
/// no `eager`/`required` split to express, which is why the spec directs
/// authors who need one to SimplifiedSchema.
#[test]
fn raw_json_schema_required_is_enforced_at_completion() {
    let dir = TempDir::new().expect("temp dir");
    let sidecar = dir.path().join("raw.json");
    std::fs::write(
        &sidecar,
        r#"{"type":"object","properties":{"researched_by":{"type":"string"}},"required":["researched_by"]}"#,
    )
    .expect("write raw JSON Schema");
    let schema = launch_schema(&format!(
        "---\n$schema: {}\n---\nBody\n",
        sidecar.display()
    ));

    let missing = evaluate_completion(
        Some(&schema),
        &serde_json::json!({}),
        BodyEvidence::Changed,
        &doc_path(),
    );
    assert!(!missing.is_satisfied());
    assert_eq!(missing.problems[0].property, "researched_by");
    assert_eq!(missing.problems[0].kind, CompletionProblemKind::Missing);
    // The per-property status table is projected from SimplifiedSchema, so a
    // raw JSON Schema contributes no rows and the failure travels through the
    // problem list alone.
    let status = missing.status.as_ref().expect("a status report is still built");
    assert!(status.required.is_empty() && status.optional.is_empty());

    let wrong_type = evaluate_completion(
        Some(&schema),
        &serde_json::json!({ "researched_by": 7 }),
        BodyEvidence::Changed,
        &doc_path(),
    );
    assert!(!wrong_type.is_satisfied());
    assert_eq!(wrong_type.problems[0].kind, CompletionProblemKind::Invalid);

    assert!(
        evaluate_completion(
            Some(&schema),
            &serde_json::json!({ "researched_by": "opencode" }),
            BodyEvidence::Changed,
            &doc_path(),
        )
        .is_satisfied()
    );
}

/// AC16: `generated; required` keeps its authoring exemption, but the host's
/// supply opportunity has passed by the completion seam, so an absent value is
/// a failure and a supplied one passes. A `generated` property that is not
/// `required` stays optional at both ends.
#[test]
fn a_generated_required_property_must_be_supplied_by_completion() {
    let schema = launch_schema(concat!(
        "---\n",
        "$schema:\n",
        "  started_at: 'datetime(generated; required)'\n",
        "  note: 'string(generated)'\n",
        "---\n",
        "Body\n",
    ));

    let absent = evaluate_completion(
        Some(&schema),
        &serde_json::json!({}),
        BodyEvidence::Changed,
        &doc_path(),
    );
    assert!(!absent.is_satisfied());
    assert_eq!(
        absent
            .problems
            .iter()
            .map(|problem| problem.property.as_str())
            .collect::<Vec<_>>(),
        ["started_at"],
        "an optional generated property must not be demanded"
    );
    assert_eq!(absent.problems[0].kind, CompletionProblemKind::Missing);

    assert!(
        evaluate_completion(
            Some(&schema),
            &serde_json::json!({ "started_at": "2026-09-06T09:00:00Z" }),
            BodyEvidence::Changed,
            &doc_path(),
        )
        .is_satisfied()
    );

    // Explicit null is absence here too.
    let nulled = evaluate_completion(
        Some(&schema),
        &serde_json::json!({ "started_at": serde_json::Value::Null }),
        BodyEvidence::Changed,
        &doc_path(),
    );
    assert!(!nulled.is_satisfied());
    assert_eq!(nulled.problems[0].property, "started_at");
}

#[test]
fn dotted_pointer_renders_nested_and_indexed_paths() {
    assert_eq!(dotted_pointer(""), "");
    assert_eq!(dotted_pointer("/researched_by"), "researched_by");
    assert_eq!(dotted_pointer("/products/2/uk_price"), "products[2].uk_price");
    assert_eq!(dotted_pointer("/a~1b"), "a/b");
}

// -- the shared orchestration entry point -----------------------------------

fn inline_guard(path: &Path, original: &str) -> InlineClosurePlan {
    let markdown: Markdown = original.to_string().into();
    InlineClosurePlan {
        document_path: path.to_path_buf(),
        original_document_text: original.to_string(),
        original_hash: markdown.compute_hash(
            darkmatter::markdown::hash::MdHashKind::Simple,
            &super::super::closure::inline_hash_options(),
        ),
    }
}

#[test]
fn direct_compose_judges_the_live_instance_and_never_touches_the_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let on_disk = "---\nprompt: research voip\n---\nUnrelated body\n";
    std::fs::write(&file, on_disk).unwrap();
    let schema = launch_schema(VOIP_SCHEMA);
    let live = satisfied_instance();

    let outcome = complete_active_document(CompletionContext {
        mode: CompositionMode::ChainedDocument,
        active_path: &file,
        launch_schema: Some(&schema),
        live_frontmatter: &live,
        inline_guard: None,
        prior_body_change: false,
        today: "2026-09-06",
    })
    .unwrap();

    assert!(outcome.verdict.is_satisfied());
    assert_eq!(outcome.verdict.body, BodyEvidence::NotApplicable);
    assert!(outcome.artifact.is_none());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), on_disk);
}

#[test]
fn inline_layers_the_agent_delta_over_the_live_effective_instance() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = concat!(
        "---\n",
        "prompt: research voip\n",
        "last_updated: '2026-01-01'\n",
        "notes: authored\n",
        "---\n",
        "Old body\n",
    );
    // The agent set `products` and deleted `notes`; `researched_by` came from
    // the caller and is only ever in the effective map.
    std::fs::write(
        &file,
        concat!(
            "---\n",
            "prompt: research voip\n",
            "last_updated: '2026-01-01'\n",
            "products:\n",
            "  uk: 10\n",
            "---\n",
            "Agent body\n",
        ),
    )
    .unwrap();
    let guard = inline_guard(&file, original);
    let schema = launch_schema(VOIP_SCHEMA);
    let live = serde_json::json!({
        "prompt": "research voip",
        "last_updated": "2026-01-01",
        "notes": "authored",
        "researched_by": "opencode",
    });

    let outcome = complete_active_document(CompletionContext {
        mode: CompositionMode::InlineFrontmatterPrompt,
        active_path: &file,
        launch_schema: Some(&schema),
        live_frontmatter: &live,
        inline_guard: Some(&guard),
        prior_body_change: false,
        today: "2026-09-06",
    })
    .unwrap();

    assert!(outcome.verdict.is_satisfied(), "{:?}", outcome.verdict.problems);
    assert_eq!(outcome.verdict.body, BodyEvidence::Changed);
    let artifact = outcome.artifact.expect("an accepted inline run writes");
    // The transient caller value satisfied the schema without being persisted.
    assert!(!artifact.text.contains("researched_by"));
    // The closure's fresh stamp — not the pre-run effective value — is what the
    // schema saw for `last_updated`.
    assert!(artifact.text.contains("last_updated: '2026-09-06'"));
    assert!(artifact.text.contains("hash: "));
}

#[test]
fn an_agent_deletion_is_observable_at_completion() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = concat!(
        "---\n",
        "prompt: research voip\n",
        "researched_by: opencode\n",
        "products:\n",
        "  uk: 10\n",
        "---\n",
        "Old body\n",
    );
    // The agent removed the property the schema requires.
    std::fs::write(
        &file,
        "---\nprompt: research voip\nproducts:\n  uk: 10\n---\nAgent body\n",
    )
    .unwrap();
    let guard = inline_guard(&file, original);
    let schema = launch_schema(VOIP_SCHEMA);
    let live = serde_json::json!({
        "prompt": "research voip",
        "researched_by": "opencode",
        "products": { "uk": 10 },
    });

    let outcome = complete_active_document(CompletionContext {
        mode: CompositionMode::InlineFrontmatterPrompt,
        active_path: &file,
        launch_schema: Some(&schema),
        live_frontmatter: &live,
        inline_guard: Some(&guard),
        prior_body_change: false,
        today: "2026-09-06",
    })
    .unwrap();

    assert!(!outcome.verdict.is_satisfied());
    assert_eq!(outcome.verdict.problems[0].property, "researched_by");
    // AC9b: the artifact the agent produced is kept despite the failed verdict.
    assert!(outcome.artifact.is_some());
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("Agent body")
    );
}

#[test]
fn inline_and_direct_reach_identical_status_for_the_same_instance() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\nprompt: research voip\n---\nOld body\n";
    std::fs::write(&file, "---\nprompt: research voip\n---\nAgent body\n").unwrap();
    let guard = inline_guard(&file, original);
    let schema = launch_schema(VOIP_SCHEMA);
    let live = serde_json::json!({ "prompt": "research voip" });

    let inline = complete_active_document(CompletionContext {
        mode: CompositionMode::InlineFrontmatterPrompt,
        active_path: &file,
        launch_schema: Some(&schema),
        live_frontmatter: &live,
        inline_guard: Some(&guard),
        prior_body_change: false,
        today: "2026-09-06",
    })
    .unwrap();

    // The direct instance is the inline instance the closure ended up with, so
    // the two verdicts must render the same rows.
    let inline_instance = serde_json::json!({
        "prompt": "research voip",
        "last_updated": "2026-09-06",
        "hash": hash_of(&file),
    });
    let direct = complete_active_document(CompletionContext {
        mode: CompositionMode::ChainedDocument,
        active_path: &file,
        launch_schema: Some(&schema),
        live_frontmatter: &inline_instance,
        inline_guard: None,
        prior_body_change: false,
        today: "2026-09-06",
    })
    .unwrap();

    assert_eq!(inline.verdict.status, direct.verdict.status);
    assert_eq!(inline.verdict.problems, direct.verdict.problems);
}

fn hash_of(path: &Path) -> String {
    let markdown: Markdown = std::fs::read_to_string(path).unwrap().into();
    markdown.frontmatter().as_map()["hash"]
        .as_str()
        .expect("the closure stamps a flat Simple hash")
        .to_string()
}

#[test]
fn an_untouched_document_yields_a_body_rejection_and_no_artifact() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\nprompt: research voip\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();
    let guard = inline_guard(&file, original);
    let schema = launch_schema(VOIP_SCHEMA);
    let live = satisfied_instance();

    let outcome = complete_active_document(CompletionContext {
        mode: CompositionMode::InlineFrontmatterPrompt,
        active_path: &file,
        launch_schema: Some(&schema),
        live_frontmatter: &live,
        inline_guard: Some(&guard),
        prior_body_change: false,
        today: "2026-09-06",
    })
    .unwrap();

    assert!(!outcome.verdict.is_satisfied());
    assert!(outcome.artifact.is_none());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
    assert_eq!(
        outcome.verdict.into_error().unwrap().code(),
        "composition.body_unchanged"
    );
}

#[test]
fn inline_completion_without_a_guard_is_a_typed_wiring_error() {
    let live = serde_json::json!({});
    let error = complete_active_document(CompletionContext {
        mode: CompositionMode::InlineFrontmatterPrompt,
        active_path: &doc_path(),
        launch_schema: None,
        live_frontmatter: &live,
        inline_guard: None,
        prior_body_change: false,
        today: "2026-09-06",
    })
    .unwrap_err();

    assert!(matches!(
        error,
        CompositionError::InlineGuardMissing { .. }
    ));
    assert_eq!(error.code(), "usage.invalid_argument");
}

/// AC13/AC14: a `prompt` supplied by the caller and deliberately never written
/// to the file still satisfies completion.
///
/// The owned-property overlay reports what landed on disk, but an *absent*
/// owned property is not an absence — the transient value the launch gate
/// accepted is still the run's effective `prompt`.
#[test]
fn a_transient_caller_prompt_survives_the_owned_property_overlay() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    // The authored document carries no `prompt`: the caller supplied it.
    let original = "---\nresearched_by: opencode\n---\nOld body\n";
    std::fs::write(
        &file,
        "---\nresearched_by: opencode\nproducts:\n  uk: 10\n---\nAgent body\n",
    )
    .unwrap();
    let guard = inline_guard(&file, original);
    let schema = launch_schema(VOIP_SCHEMA);
    let live = serde_json::json!({
        "prompt": "transient caller prompt",
        "researched_by": "opencode",
    });

    let outcome = complete_active_document(CompletionContext {
        mode: CompositionMode::InlineFrontmatterPrompt,
        active_path: &file,
        launch_schema: Some(&schema),
        live_frontmatter: &live,
        inline_guard: Some(&guard),
        prior_body_change: false,
        today: "2026-09-06",
    })
    .unwrap();

    assert!(
        outcome.verdict.is_satisfied(),
        "a transient prompt must not read as absent: {:?}",
        outcome.verdict.problems
    );
    assert!(
        !std::fs::read_to_string(&file).unwrap().contains("transient caller prompt"),
        "and it must still never be persisted"
    );
}

/// Carried body-change evidence reaches the closure through the completion
/// context, so a metadata-only recovery attempt is judged on its schema rather
/// than refused as an unchanged body.
#[test]
fn carried_body_change_evidence_reaches_the_inline_closure() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\nprompt: research voip\n---\nOld body\n";
    // The recovery attempt repaired frontmatter only; the body is the baseline.
    std::fs::write(
        &file,
        "---\nprompt: research voip\nresearched_by: opencode\nproducts:\n  uk: 10\n---\nOld body\n",
    )
    .unwrap();
    let guard = inline_guard(&file, original);
    let schema = launch_schema(VOIP_SCHEMA);
    let live = serde_json::json!({ "prompt": "research voip" });

    let refused = complete_active_document(CompletionContext {
        mode: CompositionMode::InlineFrontmatterPrompt,
        active_path: &file,
        launch_schema: Some(&schema),
        live_frontmatter: &live,
        inline_guard: Some(&guard),
        prior_body_change: false,
        today: "2026-09-06",
    })
    .unwrap();
    assert_eq!(
        refused.verdict.body,
        BodyEvidence::Rejected(BodyRejection::Unchanged)
    );

    let recovered = complete_active_document(CompletionContext {
        mode: CompositionMode::InlineFrontmatterPrompt,
        active_path: &file,
        launch_schema: Some(&schema),
        live_frontmatter: &live,
        inline_guard: Some(&guard),
        prior_body_change: true,
        today: "2026-09-06",
    })
    .unwrap();

    assert_eq!(recovered.verdict.body, BodyEvidence::Changed);
    assert!(
        recovered.verdict.is_satisfied(),
        "the repaired frontmatter satisfies the schema: {:?}",
        recovered.verdict.problems
    );
    assert!(recovered.artifact.is_some(), "the recovery is written once");
}
