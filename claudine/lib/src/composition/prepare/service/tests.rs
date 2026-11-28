//! Canonical-preparation service tests.
//!
//! The load-bearing claim of this phase is that direct and transitioned entries
//! are prepared by the *same* code, so "proxying to a document behaves like
//! invoking it directly" is structural rather than aspirational. These tests
//! assert the observable consequence of that: same source plus same input
//! layers produces the same prepared document whatever brought it here.

use super::*;
use crate::composition::types::CallerInputLayers;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn source_at(dir: &Path, name: &str, text: &str) -> ResolvedCompositionSource {
    let file = dir.join(name);
    fs::write(&file, text).unwrap();
    let original_text = fs::read_to_string(&file).unwrap();
    ResolvedCompositionSource {
        original_ref: file.display().to_string(),
        resolved_path: file,
        original_text: original_text.clone(),
        markdown: original_text.into(),
    }
}

fn prepare(
    entry: DocumentEntryReason,
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
) -> PreparedComposition {
    prepare_document(DocumentPreparation {
        entry,
        mode: CompositionMode::ChainedDocument,
        source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::Validate,
        options,
    })
    .expect("canonical preparation succeeds")
}

/// The equivalence contract. Given the same resolved source and the same
/// assembled input layers, a proxy target and a direct document prepare to the
/// same prompt, the same effective frontmatter, and the same selection hints.
///
/// Before the canonical service existed these ran through different composers
/// with independently hand-rolled options, and drifted.
#[test]
fn direct_and_proxy_entry_prepare_equivalent_documents() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "doc.md",
        "---\nagent: codex\nmodel: gpt-5\nphase: '{{ n }}'\n---\nrun phase {{ n }}\n",
    );
    let layers = CallerInputLayers {
        set_overrides: Some(serde_json::json!({ "n": 3 })),
        file_ref_fallback_dir: Some(dir.path().to_path_buf()),
        ..CallerInputLayers::new(crate::test_support::context())
    };
    let options = || layers.apply_to(PrepareOptions::new(crate::test_support::context()));

    let direct = prepare(DocumentEntryReason::Direct, &source, options());
    let proxied = prepare(DocumentEntryReason::ProxyTarget, &source, options());

    assert_eq!(direct.prompt, proxied.prompt, "same delivered prompt");
    assert_eq!(
        direct.effective_frontmatter, proxied.effective_frontmatter,
        "same effective frontmatter — the `--set` layer reached both"
    );
    assert_eq!(
        direct.selection_hints.agent, proxied.selection_hints.agent,
        "a proxy target selects its own authored provider, as a direct \
         invocation would"
    );
    assert_eq!(direct.selection_hints.model, proxied.selection_hints.model);
    assert_eq!(direct.resolved_path, proxied.resolved_path);
    assert_eq!(direct.prompt.trim(), "run phase 3");
}

#[test]
fn caller_input_layers_round_trip_raw_values_and_origins() {
    let launch = TempDir::new().unwrap();
    let origin = biscuit_file::FileResolutionContext::new(launch.path());
    let raw = serde_json::json!({
        "spec": "fixes/example/spec.md",
        "optional": null,
        "iterations": 3
    });
    let layers = CallerInputLayers::from_caller_overrides(Some(raw.clone()), origin.clone());

    assert_eq!(layers.set_overrides.as_ref(), Some(&raw));
    assert_eq!(layers.caller_input_records.len(), 3);
    for (property, value) in raw.as_object().unwrap() {
        let record = layers
            .caller_input_records
            .get(property)
            .expect("every explicit caller property retains a record");
        assert_eq!(record.raw(), value);
        assert_eq!(record.origin(), &origin);
    }

    let round_trip = CallerInputLayers::from_options(
        &layers.apply_to(PrepareOptions::new(crate::test_support::context())),
    );
    assert_eq!(round_trip.set_overrides, layers.set_overrides);
    assert_eq!(round_trip.caller_input_records, layers.caller_input_records);
}

/// The entry reason is recorded on the result, so a downstream stage reads the
/// stage row instead of re-deriving it from loop-local state. Re-deriving is
/// how the two routes drifted.
#[test]
fn the_prepared_document_carries_its_entry_reason() {
    let dir = TempDir::new().unwrap();
    let source = source_at(dir.path(), "doc.md", "---\na: 1\n---\nbody\n");

    for entry in DocumentEntryReason::ALL {
        let prepared = prepare(*entry, &source, PrepareOptions::new(crate::test_support::context()));
        assert_eq!(prepared.entry, *entry);
        assert_eq!(
            prepared.entry.stages(),
            entry.stages(),
            "the stage row travels with the document"
        );
    }
}

/// R5: the snapshot the document was composed against is stored, not recaptured.
/// A consumer that recaptured would silently answer `ctx.*` from wherever the
/// process CWD had drifted to by the time it asked.
#[test]
fn the_prepared_document_stores_the_context_it_composed_against() {
    let dir = TempDir::new().unwrap();
    let source = source_at(dir.path(), "doc.md", "---\na: 1\n---\n{{ ctx.agent }}\n");
    let mut layers = CallerInputLayers {
        file_ref_fallback_dir: Some(dir.path().to_path_buf()),
        ..CallerInputLayers::new(crate::test_support::context())
    };
    layers
        .env_overrides
        .insert("AGENT".to_string(), "codex".to_string());

    let prepared = prepare(
        DocumentEntryReason::Direct,
        &source,
        layers.apply_to(PrepareOptions::new(crate::test_support::context())),
    );

    assert_eq!(prepared.prompt.trim(), "codex");
    assert_eq!(
        prepared.compose_context.env().get("AGENT").map(String::as_str),
        Some("codex"),
        "the stored snapshot is the one the body composed against, env layer \
         included — not a fresh capture"
    );
}

/// `current.*` observes a fact when the reference is reached — and is
/// explicitly *not* a fallback for the prepared `ctx.*`.
///
/// The two answer different questions: `ctx.*` is what the document was
/// composed against, `current.*` is what is true now. Letting one satisfy a
/// lookup of the other would make a prepared document's body silently
/// time-dependent, which is precisely the drift R5 stores the snapshot to
/// prevent. So the two roots are disjoint: `ctx.*` resolves from the stored
/// snapshot, and `current.*` resolves through the invocation's refresh
/// capability.
///
/// This preparation supplies no invocation, so it holds no capability at all
/// and every `current.<key>` fails closed — a literal empty, not the host's
/// ambient answer and not `ctx`'s (decision D5).
#[test]
fn current_is_not_a_prepare_time_fallback_for_the_stored_context() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "doc.md",
        "---\na: 1\n---\nprepared=[{{ ctx.agent }}] current=[{{ current.agent }}]\n",
    );
    let mut layers = CallerInputLayers {
        file_ref_fallback_dir: Some(dir.path().to_path_buf()),
        ..CallerInputLayers::new(crate::test_support::context())
    };
    layers
        .env_overrides
        .insert("AGENT".to_string(), "codex".to_string());

    let prepared = prepare(
        DocumentEntryReason::Direct,
        &source,
        layers.apply_to(PrepareOptions::new(crate::test_support::context())),
    );

    assert_eq!(
        prepared.prompt.trim(),
        "prepared=[codex] current=[]",
        "`ctx.agent` comes from the stored snapshot; `current.agent` has no \
         supplied capability and must render empty rather than borrowing it"
    );
}

/// A passthrough document is composed for its effective frontmatter only: the
/// prompt came from argv or stdin, and the body is a provider memory file's
/// context. Its emptiness therefore says nothing about whether the caller
/// supplied a request, and must not fail preparation.
#[test]
fn a_supplied_prompt_is_delivered_and_an_empty_body_is_not_an_error() {
    let dir = TempDir::new().unwrap();
    let source = source_at(dir.path(), "CLAUDE.md", "---\nteam: platform\n---\n");

    let prepared = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::Direct,
        mode: CompositionMode::ChainedDocument,
        source: &source,
        schema: SchemaStage::Validate,
        prompt_source: PromptSource::Supplied("fix the build".to_string()),
        options: PrepareOptions::new(crate::test_support::context()),
    })
    .expect("an empty memory-file body is not an empty prompt");

    assert_eq!(prepared.prompt, "fix the build");
    assert_eq!(
        prepared.effective_frontmatter.get("team").unwrap(),
        &serde_json::json!("platform"),
        "the document is still composed for its frontmatter"
    );
}

/// The counterpart: when the composed body *is* the prompt, an empty one is
/// still the typed error it has always been.
#[test]
fn an_empty_composed_body_still_fails_when_it_is_the_prompt() {
    let dir = TempDir::new().unwrap();
    let source = source_at(dir.path(), "doc.md", "---\na: 1\n---\n");

    let err = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::ProxyTarget,
        mode: CompositionMode::ChainedDocument,
        source: &source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::Validate,
        options: PrepareOptions::new(crate::test_support::context()),
    })
    .expect_err("an empty composed body is an empty prompt");

    assert!(
        matches!(err, CompositionError::ComposedBodyEmpty { .. }),
        "expected the typed empty-body error; got {err:?}"
    );
}

/// The input layers are exactly four, and a round trip through the assembly
/// point preserves each. This is the guard the plan asks for in prose: no
/// source-specific field may be added to the caller's layers, because they are
/// invocation-scoped and reapplied at every document.
#[test]
fn caller_input_layers_round_trip_through_the_assembly_point() {
    let mut layers = CallerInputLayers {
        set_overrides: Some(serde_json::json!({ "spec": "x.md" })),
        file_ref_fallback_dir: Some(PathBuf::from("/launch/area")),
        ..CallerInputLayers::new(crate::test_support::context())
    };
    layers
        .env_overrides
        .insert("MODEL".to_string(), "gpt-5".to_string());
    layers.add_approved_commands(["basename x.md".to_string()].into_iter().collect());

    let options = layers.apply_to(PrepareOptions {
        // A target-specific field the layers must not clobber.
        shell_working_directory: Some(PathBuf::from("/repo")),
        ..PrepareOptions::new(crate::test_support::context())
    });

    assert_eq!(options.set_overrides, layers.set_overrides);
    assert_eq!(options.file_ref_fallback_dir, layers.file_ref_fallback_dir);
    assert_eq!(options.pre_approved_commands, layers.pre_approved_commands);
    assert_eq!(options.env_overrides, layers.env_overrides);
    assert_eq!(
        options.shell_working_directory,
        Some(PathBuf::from("/repo")),
        "the layers carry no target-specific state and must not overwrite it"
    );

    let recovered = CallerInputLayers::from_options(&options);
    assert_eq!(recovered.set_overrides, layers.set_overrides);
    assert_eq!(recovered.file_ref_fallback_dir, layers.file_ref_fallback_dir);
    assert_eq!(recovered.pre_approved_commands, layers.pre_approved_commands);
    assert_eq!(recovered.env_overrides, layers.env_overrides);
}

/// The in-flight fold the old `preflight_proxy_target` performed: a fresh
/// target's own approvals join the invocation-wide set without displacing the
/// approvals already in it.
#[test]
fn newly_approved_commands_extend_rather_than_replace() {
    let mut layers = CallerInputLayers::new(crate::test_support::context());
    assert!(layers.pre_approved_commands.is_none());

    layers.add_approved_commands(Default::default());
    assert!(
        layers.pre_approved_commands.is_none(),
        "approving nothing must not materialize an empty set"
    );

    layers.add_approved_commands(["git status".to_string()].into_iter().collect());
    layers.add_approved_commands(["basename x.md".to_string()].into_iter().collect());

    let approved = layers.pre_approved_commands.unwrap();
    assert!(approved.contains_exact("git status") && approved.contains_exact("basename x.md"));
}

/// Restores the process CWD on drop, panic included.
struct CwdGuard(PathBuf);

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

/// R5's regression test. The wrapper mutates the parent CWD to the repo root
/// before dispatch by design, so a preparation that captured its context
/// ambiently would answer `ctx.area` from wherever the process was last pointed
/// — and would answer differently depending on *when* it ran.
///
/// CWD is process-global, so this is serialized and restores on panic.
#[test]
#[serial_test::serial(cwd)]
fn context_derivation_ignores_a_later_process_cwd_change() {
    let area = biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("claudine/lib has a parent")
        .to_path_buf();
    let dir = TempDir::new().unwrap();
    let source = source_at(dir.path(), "doc.md", "---\na: 1\n---\n[{{ ctx.area }}]\n");

    let options = || PrepareOptions {
        // The caller's launch area — an immutable invocation input.
        file_ref_fallback_dir: Some(area.clone()),
        ..PrepareOptions::new(crate::test_support::context())
    };

    let guard = CwdGuard(std::env::current_dir().unwrap());
    std::env::set_current_dir(&area).unwrap();
    let before = prepare(DocumentEntryReason::Direct, &source, options());

    // Exactly what the wrapper does to the parent process before dispatch.
    std::env::set_current_dir(dir.path()).unwrap();
    let after = prepare(DocumentEntryReason::ProxyTarget, &source, options());
    drop(guard);

    assert_eq!(
        before.prompt.trim(),
        "[claudine]",
        "the launch-area anchor drives `ctx.area`"
    );
    assert_eq!(
        after.prompt, before.prompt,
        "`ctx.area` is a function of the launch anchor, not of the process CWD \
         at the moment preparation happened to run"
    );
}

/// The negative half: the anchor is what decides, so a different anchor gives a
/// different answer. Without this, the test above would pass for a snapshot
/// that ignored its inputs entirely.
#[test]
#[serial_test::serial(cwd)]
fn a_different_launch_anchor_derives_a_different_context() {
    let area = biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("claudine/lib has a parent")
        .to_path_buf();
    let dir = TempDir::new().unwrap();
    let source = source_at(dir.path(), "doc.md", "---\na: 1\n---\n[{{ ctx.area }}]\n");

    let guard = CwdGuard(std::env::current_dir().unwrap());
    // Stand the process in the claudine area, so an ambient capture would say
    // "claudine" for both preparations below.
    std::env::set_current_dir(&area).unwrap();

    let anchored = prepare(
        DocumentEntryReason::Direct,
        &source,
        PrepareOptions {
            file_ref_fallback_dir: Some(area.clone()),
            ..PrepareOptions::new(crate::test_support::context())
        },
    );
    let elsewhere = prepare(
        DocumentEntryReason::Direct,
        &source,
        PrepareOptions {
            file_ref_fallback_dir: Some(dir.path().to_path_buf()),
            ..PrepareOptions::new(crate::test_support::context())
        },
    );
    drop(guard);

    assert_eq!(anchored.prompt.trim(), "[claudine]");
    assert_ne!(
        elsewhere.prompt, anchored.prompt,
        "a non-area anchor must not report the CWD's area — that would mean the \
         snapshot was captured ambiently"
    );
}

/// AC6/AC8/AC10: every document-entry route receives target identity and
/// launch facts from its epoch snapshot while schema and file references stay
/// owned by the active source document.
#[test]
fn every_entry_keeps_launch_values_separate_from_source_files_and_schema() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("repo");
    let launch_dir = root.join("alpha/lib");
    let source_dir = root.join("beta/lib");
    fs::create_dir_all(&launch_dir).unwrap();
    fs::create_dir_all(&source_dir).unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&root)
        .status()
        .unwrap();
    assert!(status.success());
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"alpha/lib\", \"beta/lib\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    for (path, name) in [(&launch_dir, "alpha"), (&source_dir, "beta")] {
        fs::write(
            path.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
        )
        .unwrap();
    }

    fs::write(
        launch_dir.join("schema.yaml"),
        "$schema:\n  launch_only: string(required)\n",
    )
    .unwrap();
    fs::write(
        source_dir.join("schema.yaml"),
        "$schema:\n  source_marker: string(required)\n  spec: 'file(eager; required)'\n",
    )
    .unwrap();
    fs::write(source_dir.join("spec.md"), "SOURCE-SPEC\n").unwrap();
    let source_path = source_dir.join("probe.md");
    fs::write(
        &source_path,
        concat!(
            "---\n",
            "$schema: ./schema.yaml\n",
            "source_marker: source-owned\n",
            "spec: spec.md\n",
            "prepared_area: '{{ ctx.area }}'\n",
            "prepared_agent: '{{ ctx.agent }}'\n",
            "prepared_model: '{{ ctx.model }}'\n",
            "---\n",
            "AREA={{ ctx.area }} AGENT={{ ctx.agent }} MODEL={{ ctx.model }} ",
            "ENV={{ env.AGENT }}/{{ env.MODEL }} FILE={{ file_exists(spec) }}\n",
        ),
    )
    .unwrap();
    let source = crate::composition::resolve_composition_source(source_path.to_string_lossy().as_ref(), &crate::test_support::snapshot())
    .unwrap();

    let invocation = crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), &launch_dir).unwrap();
    let source_context = invocation.derive_source(&source.resolved_path).unwrap();
    let requirements =
        darkmatter::markdown::compose::ContextRequirements::for_document(&source.markdown);
    let env_overrides = std::collections::BTreeMap::from([
        ("AGENT".to_string(), "codex".to_string()),
        ("MODEL".to_string(), "gpt-5".to_string()),
    ]);

    for entry in DocumentEntryReason::ALL {
        let mut context = invocation.capture_launch_context(&requirements);
        context.env_mut().extend(env_overrides.clone());
        let prepared = prepare(
            *entry,
            &source,
            PrepareOptions {
                env_overrides: env_overrides.clone(),
                prepared_context: Some(context),
                file_ref_fallback_dir: Some(launch_dir.clone()),
                invocation_context: Some(invocation.clone()),
                ..PrepareOptions::new(source_context.file_resolution_context().clone())
            },
        );

        assert!(
            prepared.prompt.contains("AREA=alpha AGENT=codex MODEL=gpt-5 ENV=codex/gpt-5 FILE=true"),
            "entry {entry:?} did not use the launch snapshot and target overrides: {}",
            prepared.prompt
        );
        assert_eq!(
            prepared.effective_frontmatter.get("prepared_area"),
            Some(&serde_json::json!("alpha")),
            "entry {entry:?} did not prepare frontmatter from the launch snapshot"
        );
        assert_eq!(
            prepared.effective_frontmatter.get("prepared_agent"),
            Some(&serde_json::json!("codex"))
        );
        assert_eq!(
            prepared.effective_frontmatter.get("prepared_model"),
            Some(&serde_json::json!("gpt-5"))
        );
        assert_eq!(
            prepared.effective_frontmatter.get("source_marker"),
            Some(&serde_json::json!("source-owned")),
            "entry {entry:?} must validate against the source-side schema"
        );
        assert_eq!(
            prepared
                .effective_frontmatter
                .get("spec")
                .and_then(serde_json::Value::as_str),
            // The eager-`file` rewrite projects the value repository-relative;
            // `beta/` (not `alpha/`) proves it resolved from the source document.
            Some("beta/lib/spec.md"),
            "entry {entry:?} must preserve the source-owned file reference"
        );
        assert!(source_dir.join("spec.md").is_file());
        assert!(
            !launch_dir.join("spec.md").exists(),
            "entry {entry:?} could only have passed eager validation by resolving from the source"
        );
    }
}

// -- cross-route typed identity ---------------------------------------------
//
// Phase 12 of `features/2026-07-13-proxy-with`. These lock the claim the
// service's docs make: preparation runs through the schema layer, so a
// `$schema`-declaring document reaches the same typed error whichever route
// reached it. Before this, `prepare_document` called `prepare_direct_with_prompt`
// directly while `compose` and `sequence` called `prepare_direct_with_schema` —
// so the *harness* route (every proxied, retried, resumed, and loop-refreshed
// document) surfaced an uncategorized `ComposeFailed(SchemaValidationFailed)`
// where the identical document invoked directly surfaced the typed variant.

/// A document whose schema requires a property the frontmatter supplies with
/// the wrong type.
///
/// Uses the inline `$schema` map rather than a sidecar reference: the subject
/// here is which *typed error* each route surfaces, and an inline schema keeps
/// that independent of file resolution.
fn invalid_required_source(dir: &Path) -> ResolvedCompositionSource {
    source_at(
        dir,
        "doc.md",
        "---\n$schema:\n  count: 'number(required)'\ncount: not-a-number\n---\nbody\n",
    )
}

fn options_in(dir: &Path) -> PrepareOptions {
    PrepareOptions {
        file_ref_fallback_dir: Some(dir.to_path_buf()),
        ..PrepareOptions::new(crate::test_support::context())
    }
}

/// The checkpoint's headline gate, at the layer that decides it.
///
/// The same document failing the same way must produce the same typed variant
/// whether it was named directly or reached by a transition. Both proxy entries
/// are covered because they are the two proxy routes the plan names: a proxy
/// from `initialize` and a proxy from terminal recovery both adopt their target
/// as `ProxyTarget`, and retry/resume re-prepare the active document.
#[test]
fn a_schema_failure_has_one_typed_identity_across_every_entry() {
    let dir = TempDir::new().unwrap();
    let source = invalid_required_source(dir.path());

    let direct = crate::composition::prepare_direct_with_schema(&source, options_in(dir.path()))
        .expect_err("an invalid required value fails");

    for entry in [
        DocumentEntryReason::Direct,
        DocumentEntryReason::ProxyTarget,
        DocumentEntryReason::Retry,
        DocumentEntryReason::Resume,
        DocumentEntryReason::LoopIteration,
    ] {
        let err = prepare_document(DocumentPreparation {
            entry,
            mode: CompositionMode::ChainedDocument,
            source: &source,
            prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::Validate,
            options: options_in(dir.path()),
        })
        .expect_err("an invalid required value fails on every entry");

        assert!(
            matches!(err, CompositionError::SchemaValidation { .. }),
            "entry {entry:?} must surface the typed `SchemaValidation`, not an \
             uncategorized compose failure; got {err:?}"
        );
        assert_eq!(
            err.to_string(),
            direct.to_string(),
            "entry {entry:?} must render identically to the direct route"
        );
    }
}

/// The missing-required case is a *different* typed variant, and the harness
/// route must reach that one too — categorization, not just "some typed error".
#[test]
fn a_missing_required_property_is_typed_on_the_harness_route() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "doc.md",
        "---\n$schema:\n  count: 'number(required)'\n---\nbody\n",
    );

    let err = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::ProxyTarget,
        mode: CompositionMode::ChainedDocument,
        source: &source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::Validate,
        options: options_in(dir.path()),
    })
    .expect_err("a missing required value fails");

    assert!(
        matches!(&err, CompositionError::MissingProperties { missing, .. } if
            missing.iter().any(|property| property.name == "count")),
        "the proxy route must reach `MissingProperties` and name the property, \
         which is what drives the interactive collection prompt; got {err:?}"
    );
}

/// Invalid-**optional** drop-and-retry is the other half of what the schema
/// layer adds, and it is a behavior change on the harness route rather than
/// only a typing change: preparation now succeeds, removes the bad value, and
/// lets composition replace the document-declared optional binding with null.
#[test]
fn an_invalid_optional_is_dropped_and_recorded_on_the_harness_route() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "doc.md",
        "---\n$schema:\n  note: 'string(required)'\n  count: 'number'\n\
         note: hello\ncount: not-a-number\n---\nbody\n",
    );

    let prepared = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::ProxyTarget,
        mode: CompositionMode::ChainedDocument,
        source: &source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::Validate,
        options: options_in(dir.path()),
    })
    .expect("an invalid optional is dropped rather than fatal");

    assert!(
        prepared
            .dropped_optionals
            .iter()
            .any(|dropped| dropped.property == "count"),
        "the drop must be recorded so the CLI can warn about it — off the schema \
         path this vector was always empty, so a proxied target could not warn \
         about a silently dropped value even in principle; got {:?}",
        prepared.dropped_optionals
    );
    assert_eq!(
        prepared.effective_frontmatter.get("count"),
        None,
        "the invalid value is dropped and the declared but unset key stays absent",
    );
}

/// `PromptSource::Supplied` must survive the schema layer.
///
/// This is the one thing that made the unification non-mechanical: the schema
/// path hardcoded `ComposedBody`, but the passthrough case composes the
/// document only for its frontmatter and must skip the body-emptiness check.
/// Threading it through is what lets the harness route use the schema layer at
/// all, so a regression here would silently push that route back off it.
#[test]
fn a_supplied_prompt_survives_the_schema_layer_with_an_empty_body() {
    // Empty body: fatal for `ComposedBody`, expected for `Supplied`.
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "doc.md",
        "---\n$schema:\n  note: 'string(required)'\nnote: hi\n---\n",
    );

    let prepared = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::Direct,
        mode: CompositionMode::ChainedDocument,
        source: &source,
        schema: SchemaStage::Validate,
        prompt_source: PromptSource::Supplied("the caller's prompt".to_string()),
        options: options_in(dir.path()),
    })
    .expect("a supplied prompt does not require a composed body");

    assert_eq!(prepared.prompt, "the caller's prompt");

    let composed = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::Direct,
        mode: CompositionMode::ChainedDocument,
        source: &source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::Validate,
        options: options_in(dir.path()),
    });
    assert!(
        composed.is_err(),
        "the same document with a composed-body prompt must still fail the \
         emptiness check — threading `PromptSource` must not disable it"
    );
}

/// R4: the read taken before a document's own `initialize` must not reach a
/// schema verdict — `initialize` is exactly the stage that can still supply the
/// missing value.
///
/// The same source, judged, is the control: without it this test would pass on a
/// document that simply satisfies its schema.
#[test]
fn a_deferred_read_withholds_the_verdict_the_validating_read_reaches() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "doc.md",
        "---\n$schema:\n    count: 'number(required)'\n---\nbody\n",
    );

    let deferred = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::ProxyTarget,
        mode: CompositionMode::ChainedDocument,
        source: &source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::DeferToStabilizedReread,
        options: PrepareOptions::new(crate::test_support::context()),
    })
    .expect("a pre-`initialize` read does not judge the document");

    assert!(
        deferred.schema_verdict_deferred,
        "the deferral must be recorded on the prepared document: a downstream \
         stage still owes the stabilized reread that judges it"
    );

    let err = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::ProxyTarget,
        mode: CompositionMode::ChainedDocument,
        source: &source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::Validate,
        options: PrepareOptions::new(crate::test_support::context()),
    })
    .expect_err("the validating read reaches the verdict");

    assert!(
        matches!(err, CompositionError::MissingProperties { .. }),
        "and reaches it as the typed variant, not a raw Darkmatter error; got {err:?}"
    );
}

/// A deferred read still *coerces* frontmatter to the declared types. Only the
/// verdict is withheld, so a value the schema recognizes reaches `initialize`
/// with its real type rather than as the authored string.
#[test]
fn a_deferred_read_still_coerces_declared_types() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "doc.md",
        "---\n$schema:\n    enabled: boolean\n    count: 'number(required)'\nenabled: \"true\"\n---\nbody\n",
    );

    let prepared = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::ProxyTarget,
        mode: CompositionMode::ChainedDocument,
        source: &source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::DeferToStabilizedReread,
        options: PrepareOptions::new(crate::test_support::context()),
    })
    .expect("the missing required `count` is not this read's verdict to reach");

    assert_eq!(
        prepared.effective_frontmatter.get("enabled"),
        Some(&serde_json::json!(true)),
        "a lifecycle condition reading `enabled` must see a real boolean"
    );
}

/// An ordinary later loop pass composes its retained source without reaching
/// the launch verdict and without leaving one pending.
///
/// The control is the same source under `Validate`, which refuses the missing
/// `count`. The skipped read still applies the schema's defaults (`level`) and
/// keeps the resolved schema for the completion verdict, so a pending flag left
/// `true` here would hand a second verdict to the harness.
#[test]
fn loop_iteration_skips_the_launch_verdict_without_deferring_it() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "doc.md",
        "---\n$schema:\n    count: 'number(required)'\n    level: 'number(default(2))'\n---\nlevel={{ level }}\n",
    );

    let err = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::LoopIteration,
        mode: CompositionMode::ChainedDocument,
        source: &source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::Validate,
        options: options_in(dir.path()),
    })
    .expect_err("the validating read refuses the missing required value");
    assert!(
        matches!(&err, CompositionError::MissingProperties { missing, .. }
            if missing.iter().any(|property| property.name == "count")),
        "the control reaches the typed verdict; got {err:?}"
    );

    let prepared = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::LoopIteration,
        mode: CompositionMode::ChainedDocument,
        source: &source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::SkipLaunchVerdict,
        options: options_in(dir.path()),
    })
    .expect("a later loop pass reaches no launch verdict");

    assert_eq!(prepared.entry, DocumentEntryReason::LoopIteration);
    assert!(
        !prepared.schema_verdict_deferred,
        "a later pass owes no verdict, so none may be marked pending"
    );
    assert!(prepared.launch_schema.is_some(), "the schema stays attached for completion");
    assert_eq!(
        prepared.effective_frontmatter.get("level"),
        Some(&serde_json::json!(2)),
        "defaults still apply on the skipped read"
    );
    assert_eq!(prepared.prompt.trim(), "level=2");
}

/// An ordinary later inline pass takes its reconciliation baseline from the
/// file as it reads now, while its prompt still comes from the retained source.
///
/// The retained source and the file on disk differ in body here, exactly as
/// after a pass whose provider edited the deliverable.
#[test]
fn a_later_inline_pass_baselines_on_the_live_deliverable() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "doc.md",
        "---\nprompt: 'edit the body'\n---\nretained body\n",
    );
    let live = "---\nprompt: 'edit the body'\n---\nfirst pass deliverable\n";
    fs::write(&source.resolved_path, live).unwrap();

    let prepared = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::LoopIteration,
        mode: CompositionMode::InlineFrontmatterPrompt,
        source: &source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::SkipLaunchVerdict,
        options: options_in(dir.path()),
    })
    .expect("the later inline pass prepares");

    let crate::composition::types::CompositionClosurePlan::Inline(plan) = &prepared.closure else {
        panic!("an inline pass carries an inline closure plan");
    };
    assert_eq!(plan.original_document_text, live, "the baseline is the live deliverable");
    assert_eq!(
        plan.original_hash,
        darkmatter::markdown::Markdown::from(live).compute_hash(
            darkmatter::markdown::hash::MdHashKind::Simple,
            &crate::composition::closure::inline_hash_options(),
        ),
        "the baseline hash is the live deliverable's"
    );
    assert!(prepared.prompt.contains("edit the body"), "the prompt comes from the retained source");

    fs::remove_file(&source.resolved_path).unwrap();
    let err = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::LoopIteration,
        mode: CompositionMode::InlineFrontmatterPrompt,
        source: &source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::SkipLaunchVerdict,
        options: options_in(dir.path()),
    })
    .expect_err("an unreadable deliverable fails before composition");
    assert!(
        matches!(&err, CompositionError::InlineArtifactUnreadable { path, .. } if *path == source.resolved_path),
        "the unreadable baseline names the document; got {err:?}"
    );
}

/// A composed prompt body resolves `current.<key>` through the invocation's
/// launch evidence, while `ctx.<key>` stays the stored snapshot.
///
/// This is the seam AC28 rides on: an author writes `{{ current.branch }}` in a
/// prompt and gets the branch as it stands, without the compose path
/// rediscovering a repository from wherever the process CWD drifted to.
#[test]
fn a_composed_prompt_resolves_current_through_the_invocation() {
    use std::process::Command;

    let dir = TempDir::new().unwrap();
    for args in [
        vec!["init", "--initial-branch=main"],
        vec!["commit", "-q", "--allow-empty", "-m", "fixture"],
    ] {
        let status = Command::new("git")
            .args([
                "-c",
                "user.name=Claudine Test",
                "-c",
                "user.email=claudine@example.invalid",
            ])
            .args(&args)
            .current_dir(dir.path())
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    let source = source_at(
        dir.path(),
        "doc.md",
        "---\na: 1\n---\nsnapshot=[{{ ctx.branch }}] now=[{{ current.branch }}]\n",
    );
    let invocation = crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), dir.path()).unwrap();
    let options = PrepareOptions {
        invocation_context: Some(invocation),
        file_ref_fallback_dir: Some(dir.path().to_path_buf()),
        ..PrepareOptions::new(crate::test_support::context())
    };

    let prepared = prepare(DocumentEntryReason::Direct, &source, options);

    assert_eq!(
        prepared.prompt.trim(),
        "snapshot=[main] now=[main]",
        "both roots answer from the invocation's launch repository"
    );
}

/// R4: the late verdict on a caller-supplied file reference names the
/// directory the caller's value was resolved from, never the document's.
///
/// The document sits in `<launch>/prompts`, and `spec=fix` was typed at
/// `<launch>`. Darkmatter's own verdict resolves the value in document context
/// and would name `<launch>/prompts`.
#[test]
fn late_caller_file_verdict_names_the_callers_base_directory() {
    let launch = TempDir::new().unwrap();
    // The verdict names the origin exactly as the caller's record holds it.
    let launch_dir = launch.path().to_path_buf();
    fs::create_dir_all(launch_dir.join("prompts")).unwrap();
    fs::create_dir_all(launch_dir.join("features/a")).unwrap();
    fs::write(launch_dir.join("features/a/spec.md"), "---\ntitle: a\n---\n").unwrap();
    let origin = biscuit_file::FileResolutionContext::new(&launch_dir);
    let caller_base = format!("while resolving from `{}`", launch_dir.display());
    let document_base = format!("`{}`", launch_dir.join("prompts").display());

    for (label, schema) in [
        (
            // The arm is only decidable after composition, and the arms
            // disagree on `spec`'s array shape, so the early supplied-file
            // pass has no single glob and leaves `spec` to this verdict.
            "union arms disagreeing on array shape",
            "$schema:\n  - kind: 'literal(feature)'\n    spec: 'file(required;match(**/*spec*.md);eager)'\n  \
             - kind: 'literal(note)'\n    spec: 'file(required;match(**/*note*.md);eager)[]'\n\
             kind: \"{{ 'feature' }}\"\n",
        ),
        (
            // A bare `file` has no glob, so there is nothing to complete early.
            "union with a bare eager file",
            "$schema:\n  - kind: 'literal(feature)'\n    spec: 'file(required;eager)'\n  \
             - kind: 'literal(note)'\n    plan: 'file(required;eager)'\n\
             kind: \"{{ 'feature' }}\"\n",
        ),
        (
            "single schema",
            "$schema:\n  spec: 'file(required;match(**/*spec*.md);eager)'\n",
        ),
    ] {
        let source = source_at(
            &launch_dir.join("prompts"),
            "p.md",
            &format!("---\n{schema}---\nSpec: {{{{spec}}}}\n"),
        );
        let options = |spec: &str| {
            CallerInputLayers {
                file_ref_fallback_dir: Some(launch_dir.clone()),
                ..CallerInputLayers::from_caller_overrides(
                    Some(serde_json::json!({ "spec": spec })),
                    origin.clone(),
                )
            }
            .apply_to(PrepareOptions {
                // As the CLI does: the document-scoped request context.
                ..PrepareOptions::new(origin.for_source(&source.resolved_path))
            })
        };
        let prepare_with = |spec: &str| {
            prepare_document(DocumentPreparation {
                entry: DocumentEntryReason::Direct,
                mode: CompositionMode::ChainedDocument,
                source: &source,
                prompt_source: PromptSource::ComposedBody,
                schema: SchemaStage::Validate,
                options: options(spec),
            })
        };

        // Control: a literal the caller's origin resolves composes.
        let prepared = prepare_with("features/a/spec.md")
            .unwrap_or_else(|err| panic!("{label}: resolvable spec failed: {err}"));
        assert!(prepared.prompt.contains("features/a/spec.md"), "{label}: {}", prepared.prompt);

        let err = prepare_with("fix").expect_err("`fix` names no file");
        // A union reaches Claudine's translated verdict; a single schema keeps
        // Darkmatter's typed caller-file problem, which already carries the
        // caller's origin. Both must name the caller's base directory.
        let message = match &err {
            CompositionError::SchemaValidation { message, problems, .. } => {
                assert_eq!(problems, &vec!["/spec".to_string()], "{label}");
                assert!(
                    message.starts_with("/spec: no existing file matched reference `fix`"),
                    "{label}: {message}"
                );
                message.clone()
            }
            CompositionError::ComposeFailed(darkmatter::markdown::MarkdownError::SchemaValidationFailed {
                problems,
                ..
            }) if label == "single schema" => {
                assert!(problems.iter().all(|problem| problem.caller_file.is_some()), "{label}");
                problems.iter().map(|problem| problem.message.clone()).collect::<Vec<_>>().join("; ")
            }
            other => panic!("{label}: unexpected error: {other}"),
        };
        let caller_base = if label == "single schema" {
            format!("`{}`", launch_dir.display())
        } else {
            caller_base.clone()
        };
        assert!(message.contains(&caller_base), "{label}: {message}");
        assert!(!message.contains(&document_base), "{label}: {message}");
    }
}

/// A document's `loop:` block is not one of the deferred event keys, so a
/// schema completion may fill `no_error: false` at the loop root and in the
/// gate's actions. The lifecycle of such a completed value, parsed with the
/// document's authored values (`authored_lifecycle_values`) as its source, is
/// the lifecycle of the document as written: the filled flag is never a loose
/// key beside `gate`, and the authored sibling flag counts once.
#[test]
fn a_completed_loop_block_parses_with_the_document_as_its_authored_source() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "doc.md",
        "---\nloop:\n  until: \"true\"\n  max: 2\n  gate:\n    - info: { text: pass }\n      no_error: true\n---\nrun\n",
    );
    let options = PrepareOptions::new(crate::test_support::context());
    let authored = super::super::authored_lifecycle_values(&source, &options);
    let completed = serde_json::json!({ "loop": {
        "until": "true",
        "max": 2,
        "gate": [{ "info": { "text": "pass", "no_error": false }, "no_error": true }],
        "no_error": false,
    } });

    let parse = |value: &serde_json::Value, authored: Option<&serde_json::Value>| {
        crate::composition::lifecycle::parse_lifecycle_config_with_orders(value, &source.resolved_path, None, authored)
    };
    let from_completed = parse(&completed, Some(&authored)).expect("the filled flags are not authored");
    assert_eq!(from_completed, parse(&authored, None).expect("the document parses"));
    let gate = &from_completed.stacks.loop_gate;
    let darkmatter::lifecycle::actions::LifecycleNode::Action(action) = &gate[0] else {
        panic!("one gate action, got {gate:?}");
    };
    assert!(action.no_error, "the authored sibling flag is the one placement");
    // Read as authored, the filled root flag sits beside `gate`.
    assert!(parse(&completed, None).is_err());
}

/// The same through the whole preparation. `loop` is a deferred lifecycle
/// key, so the schema stage judges it (the launch schema is selected) but
/// writes no default into it: the effective block is the authored one, the
/// gate action's unwritten `no_error` is the parser's `false`, and the
/// lifecycle is the one the document was written with. An authored flag the
/// parser refuses stops preparation, so nothing of the document runs.
#[test]
fn a_loop_document_prepares_as_authored_and_an_invalid_loop_flag_is_refused() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "doc.md",
        "---\nloop:\n  until: \"true\"\n  max: 2\n  gate:\n    - info: { text: pass }\n---\nrun\n",
    );
    let prepared = prepare(DocumentEntryReason::Direct, &source, PrepareOptions::new(crate::test_support::context()));
    let authored = super::super::authored_lifecycle_values(&source, &PrepareOptions::new(crate::test_support::context()));
    let as_authored =
        crate::composition::lifecycle::parse_lifecycle_config_with_orders(&authored, &source.resolved_path, None, None)
            .expect("the document parses");
    assert!(prepared.launch_schema.is_some());
    assert_eq!(prepared.effective_frontmatter["loop"], authored["loop"], "no default is written into the block");
    assert_eq!(prepared.lifecycle, as_authored);
    let darkmatter::lifecycle::actions::LifecycleNode::Action(action) = &prepared.lifecycle.stacks.loop_gate[0] else {
        panic!("one gate action, got {:?}", prepared.lifecycle.stacks.loop_gate);
    };
    assert!(!action.no_error, "the filled flag is `false`");

    let refused = source_at(
        dir.path(),
        "refused.md",
        "---\nloop:\n  until: \"true\"\n  max: 2\n  gate:\n    - info: { text: pass }\n      no_error: 5\n---\nrun\n",
    );
    let error = prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::Direct,
        mode: CompositionMode::ChainedDocument,
        source: &refused,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::Validate,
        options: PrepareOptions::new(crate::test_support::context()),
    })
    .expect_err("a numeric `no_error` is not a flag");
    assert!(error.to_string().contains("no_error"), "{error}");
}
