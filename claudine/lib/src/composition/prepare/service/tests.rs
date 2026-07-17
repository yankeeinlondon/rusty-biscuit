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
        ..CallerInputLayers::default()
    };
    let options = || layers.apply_to(PrepareOptions::default());

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

/// The entry reason is recorded on the result, so a downstream stage reads the
/// stage row instead of re-deriving it from loop-local state. Re-deriving is
/// how the two routes drifted.
#[test]
fn the_prepared_document_carries_its_entry_reason() {
    let dir = TempDir::new().unwrap();
    let source = source_at(dir.path(), "doc.md", "---\na: 1\n---\nbody\n");

    for entry in DocumentEntryReason::ALL {
        let prepared = prepare(*entry, &source, PrepareOptions::default());
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
        ..CallerInputLayers::default()
    };
    layers
        .env_overrides
        .insert("AGENT".to_string(), "codex".to_string());

    let prepared = prepare(
        DocumentEntryReason::Direct,
        &source,
        layers.apply_to(PrepareOptions::default()),
    );

    assert_eq!(prepared.prompt.trim(), "codex");
    assert_eq!(
        prepared.compose_context.env().get("AGENT").map(String::as_str),
        Some("codex"),
        "the stored snapshot is the one the body composed against, env layer \
         included — not a fresh capture"
    );
}

/// `current.ctx.*` stays a live, event-time, lifecycle-only surface — and is
/// explicitly *not* a fallback for the prepared `ctx.*`.
///
/// The two answer different questions: `ctx.*` is what the document was
/// composed against, `current.ctx.*` is what is true now. Letting one satisfy a
/// lookup of the other would make a prepared document's body silently
/// time-dependent, which is precisely the drift R5 stores the snapshot to
/// prevent. So the two roots are disjoint at prepare time: `ctx.*` resolves
/// from the stored snapshot, and `current.*` — which has no meaning before an
/// event fires — resolves to nothing rather than borrowing `ctx`'s answer.
#[test]
fn current_is_not_a_prepare_time_fallback_for_the_stored_context() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "doc.md",
        "---\na: 1\n---\nprepared=[{{ ctx.agent }}] current=[{{ current.ctx.agent }}]\n",
    );
    let mut layers = CallerInputLayers {
        file_ref_fallback_dir: Some(dir.path().to_path_buf()),
        ..CallerInputLayers::default()
    };
    layers
        .env_overrides
        .insert("AGENT".to_string(), "codex".to_string());

    let prepared = prepare(
        DocumentEntryReason::Direct,
        &source,
        layers.apply_to(PrepareOptions::default()),
    );

    assert_eq!(
        prepared.prompt.trim(),
        "prepared=[codex] current=[]",
        "`ctx.agent` comes from the stored snapshot; `current.ctx.agent` is \
         event-time and must not be backfilled from it at prepare time"
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
        prompt_source: PromptSource::Supplied("fix the build".to_string()),
        options: PrepareOptions::default(),
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
        options: PrepareOptions::default(),
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
        ..CallerInputLayers::default()
    };
    layers
        .env_overrides
        .insert("MODEL".to_string(), "gpt-5".to_string());
    layers.add_approved_commands(["basename x.md".to_string()]);

    let options = layers.apply_to(PrepareOptions {
        // A target-specific field the layers must not clobber.
        shell_working_directory: Some(PathBuf::from("/repo")),
        ..PrepareOptions::default()
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
    let mut layers = CallerInputLayers::default();
    assert!(layers.pre_approved_commands.is_none());

    layers.add_approved_commands(Vec::new());
    assert!(
        layers.pre_approved_commands.is_none(),
        "approving nothing must not materialize an empty set"
    );

    layers.add_approved_commands(["git status".to_string()]);
    layers.add_approved_commands(["basename x.md".to_string()]);

    let approved = layers.pre_approved_commands.unwrap();
    assert!(approved.contains("git status") && approved.contains("basename x.md"));
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
    let area = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("claudine/lib has a parent")
        .to_path_buf();
    let dir = TempDir::new().unwrap();
    let source = source_at(dir.path(), "doc.md", "---\na: 1\n---\n[{{ ctx.area }}]\n");

    let options = || PrepareOptions {
        // The caller's launch area — an immutable invocation input.
        file_ref_fallback_dir: Some(area.clone()),
        ..PrepareOptions::default()
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
    let area = Path::new(env!("CARGO_MANIFEST_DIR"))
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
            ..PrepareOptions::default()
        },
    );
    let elsewhere = prepare(
        DocumentEntryReason::Direct,
        &source,
        PrepareOptions {
            file_ref_fallback_dir: Some(dir.path().to_path_buf()),
            ..PrepareOptions::default()
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
