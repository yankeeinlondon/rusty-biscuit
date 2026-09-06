use super::*;
use darkmatter::markdown::hash::{FrontmatterDeltaEntry, StoredHashValue};
use std::path::Path;
use tempfile::TempDir;

/// The AC12 byte-preservation fixture: a `|-` block scalar with four-space
/// indentation, a trailing space, a blank line, and escaped quotes, plus a
/// `structured` stored hash the closure must downgrade to `Simple`.
const AUTHORED_DOCUMENT: &str = concat!(
    "---\n",
    "prompt: |-\n",
    "    Keep four spaces  \n",
    "\n",
    "    and literal \\\"quotes\\\"\n",
    "hash:\n",
    "  kind: structured\n",
    "  value: a000000000000000-b000000000000000-c000000000000000-d000000000000000\n",
    "last_updated: '2026-01-01'\n",
    "---\n",
    "Old body\n",
);

/// Build the inline guard for a document whose pre-run text is `original`.
fn plan(path: &Path, original: &str) -> InlineClosurePlan {
    let markdown: Markdown = original.to_string().into();
    InlineClosurePlan {
        document_path: path.to_path_buf(),
        original_document_text: original.to_string(),
        original_hash: markdown.compute_hash(MdHashKind::Simple, &inline_hash_options()),
    }
}

/// Seed `doc.md` with `original` (the guard's baseline) and then overwrite it
/// with `agent_wrote`, mimicking a provider that edited the file in place.
fn agent_run(dir: &TempDir, original: &str, agent_wrote: &str) -> (std::path::PathBuf, InlineClosurePlan) {
    let file = dir.path().join("doc.md");
    let plan = plan(&file, original);
    std::fs::write(&file, agent_wrote).unwrap();
    (file, plan)
}

fn written(reconciliation: InlineReconciliation) -> InlineArtifact {
    match reconciliation {
        InlineReconciliation::Written(artifact) => *artifact,
        InlineReconciliation::Rejected(reason) => {
            panic!("expected an accepted artifact, got a {reason} rejection")
        }
    }
}

// -- accepted artifacts -----------------------------------------------------

#[test]
fn accepts_the_agents_body_and_stamps_a_coherent_hash() {
    let dir = TempDir::new().unwrap();
    let agent_wrote = AUTHORED_DOCUMENT.replace("Old body\n", "Agent body\n");
    let (file, plan) = agent_run(&dir, AUTHORED_DOCUMENT, &agent_wrote);

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06").unwrap());

    let on_disk = std::fs::read_to_string(&file).unwrap();
    assert_eq!(on_disk, artifact.text);
    let markdown: Markdown = on_disk.clone().into();
    let options = inline_hash_options();
    let stored = parse_inline_stored_hash(&markdown, &options)
        .unwrap()
        .unwrap();
    let StoredHashValue::Flat(hash_value) = &stored.value else {
        panic!("inline closure must downgrade the managed hash to Simple")
    };
    // Every authored byte outside the two managed nodes survives: the four-space
    // block scalar, the trailing space, the blank line, and the escaped quotes.
    let expected = format!(
        concat!(
            "---\n",
            "prompt: |-\n",
            "    Keep four spaces  \n",
            "\n",
            "    and literal \\\"quotes\\\"\n",
            "hash: {}\n",
            "last_updated: '2026-09-06'\n",
            "---\n",
            "Agent body\n",
        ),
        hash_value
    );
    assert_eq!(on_disk, expected);
    assert_eq!(stored.kind, MdHashKind::Simple);
    // AC9b's L1 half: the stamped hash agrees with the document it was
    // stamped against, which is what `md hash --diff` reports on.
    let comparison = markdown.compare_hash(&stored, &options).unwrap();
    assert!(!comparison.frontmatter_changed && !comparison.body_changed);
    assert!(artifact.restored_properties.is_empty());
    assert!(artifact.frontmatter_delta.is_empty());
}

#[test]
fn restores_every_owned_property_the_agent_touched_and_warns_once_each() {
    let dir = TempDir::new().unwrap();
    let agent_wrote = concat!(
        "---\n",
        "prompt: \"Keep four spaces\\n\\nand literal quotes\"\n",
        "last_updated: never\n",
        "---\n",
        "Agent body\n",
    );
    let (file, plan) = agent_run(&dir, AUTHORED_DOCUMENT, agent_wrote);

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06").unwrap());

    // `hash` was deleted by the agent, so it is restored too, then re-stamped.
    assert_eq!(
        artifact.restored_properties,
        ["prompt", "hash", "last_updated"]
    );
    let on_disk = std::fs::read_to_string(&file).unwrap();
    assert!(
        on_disk.contains("prompt: |-\n    Keep four spaces  \n\n    and literal \\\"quotes\\\"\n"),
        "the authored `prompt` bytes must be restored verbatim; got:\n{on_disk}"
    );
    assert!(!on_disk.contains("prompt: \""));
    assert!(on_disk.contains("last_updated: '2026-09-06'"));
    assert!(!on_disk.contains("last_updated: never"));
}

#[test]
fn owned_restoration_is_silent_when_the_agent_leaves_them_alone() {
    let dir = TempDir::new().unwrap();
    let original = "---\nprompt: test\nlast_updated: '2026-01-01'\n---\nOld body\n";
    let (_file, plan) = agent_run(
        &dir,
        original,
        "---\nprompt: test\nlast_updated: '2026-01-01'\n---\nAgent body\n",
    );

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06").unwrap());

    assert!(artifact.restored_properties.is_empty());
}

#[test]
fn reports_the_agents_semantic_delta_excluding_owned_properties() {
    let dir = TempDir::new().unwrap();
    let original = concat!(
        "---\n",
        "prompt: test\n",
        "researched_by: null\n",
        "owner: Human\n",
        "last_updated: '2026-01-01'\n",
        "---\n",
        "Old body\n",
    );
    let agent_wrote = concat!(
        "---\n",
        "prompt: rewritten\n",
        "researched_by: opencode\n",
        "products:\n",
        "  uk: 10\n",
        "last_updated: never\n",
        "---\n",
        "Agent body\n",
    );
    let (_file, plan) = agent_run(&dir, original, agent_wrote);

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06").unwrap());

    assert_eq!(
        artifact.frontmatter_delta.entries,
        vec![
            FrontmatterDeltaEntry::Replacement {
                property: "researched_by".into(),
                previous_value: serde_json::Value::Null,
                value: serde_json::json!("opencode"),
            },
            FrontmatterDeltaEntry::Addition {
                property: "products".into(),
                value: serde_json::json!({ "uk": 10 }),
            },
            FrontmatterDeltaEntry::Deletion {
                property: "owner".into(),
                previous_value: serde_json::json!("Human"),
            },
        ]
    );
}

#[test]
fn value_preserving_reformatting_is_not_a_semantic_change() {
    let dir = TempDir::new().unwrap();
    let original = "---\nprompt: test\ntitle: |-\n  First\n  Second\n---\nOld body\n";
    let (_file, plan) = agent_run(
        &dir,
        original,
        "---\nprompt: test\ntitle: \"First\\nSecond\"\n---\nAgent body\n",
    );

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06").unwrap());

    assert!(
        artifact.frontmatter_delta.is_empty(),
        "reformatting a value must not register as an agent edit: {:?}",
        artifact.frontmatter_delta
    );
}

#[test]
fn cleans_the_agents_body_and_hashes_the_cleaned_text() {
    let dir = TempDir::new().unwrap();
    let original = "---\nprompt: test\nlast_updated: '2026-01-01'\n---\nOld body\n";
    let (file, plan) = agent_run(
        &dir,
        original,
        "---\nprompt: test\nlast_updated: '2026-01-01'\n---\n# Title\nNo blank line\n",
    );

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06").unwrap());

    assert!(artifact.body_cleaned);
    let on_disk = std::fs::read_to_string(&file).unwrap();
    assert!(on_disk.contains("# Title\n\nNo blank line"));
    let markdown: Markdown = on_disk.into();
    let options = inline_hash_options();
    let stored = parse_inline_stored_hash(&markdown, &options)
        .unwrap()
        .unwrap();
    let comparison = markdown.compare_hash(&stored, &options).unwrap();
    assert!(!comparison.frontmatter_changed && !comparison.body_changed);
}

#[test]
fn preserves_crlf_and_the_authored_last_updated_quote_style() {
    for (label, quote) in [("double", '"'), ("single", '\'')] {
        let dir = TempDir::new().unwrap();
        // A quote character is not a legal file name on Windows.
        let file = dir.path().join(format!("quoted-{label}.md"));
        let original = format!(
            "---\r\nprompt: |-\r\n  Keep\r\nlast_updated: {quote}2026-01-01{quote}\r\n---\r\nOld body\r\n"
        );
        let plan = plan(&file, &original);
        std::fs::write(&file, original.replace("Old body", "Agent body")).unwrap();

        written(reconcile_inline_artifact(&plan, "2026-09-06").unwrap());

        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert!(on_disk.contains("prompt: |-\r\n  Keep\r\n"));
        assert!(on_disk.contains(&format!("last_updated: {quote}2026-09-06{quote}\r\n")));
    }
}

// -- refused candidates -----------------------------------------------------

#[test]
fn refuses_an_untouched_document_without_writing() {
    let dir = TempDir::new().unwrap();
    let (file, plan) = agent_run(&dir, AUTHORED_DOCUMENT, AUTHORED_DOCUMENT);

    let outcome = reconcile_inline_artifact(&plan, "2026-09-06").unwrap();

    assert!(matches!(
        outcome,
        InlineReconciliation::Rejected(BodyRejection::Unchanged)
    ));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), AUTHORED_DOCUMENT);
}

#[test]
fn refuses_a_whitespace_only_body_change() {
    let dir = TempDir::new().unwrap();
    let original = "---\nprompt: test\n---\nOld body\n";
    let (file, plan) = agent_run(&dir, original, "---\nprompt: test\n---\n\n  Old body  \n\n\n");

    let outcome = reconcile_inline_artifact(&plan, "2026-09-06").unwrap();

    assert!(matches!(
        outcome,
        InlineReconciliation::Rejected(BodyRejection::Unchanged)
    ));
    assert!(std::fs::read_to_string(&file).unwrap().contains("Old body"));
}

#[test]
fn refuses_an_empty_body_even_when_frontmatter_changed() {
    let dir = TempDir::new().unwrap();
    let original = "---\nprompt: test\n---\nOld body\n";
    let (file, plan) = agent_run(&dir, original, "---\nprompt: test\nresearched_by: x\n---\n   \n");

    let outcome = reconcile_inline_artifact(&plan, "2026-09-06").unwrap();

    assert!(matches!(
        outcome,
        InlineReconciliation::Rejected(BodyRejection::Empty)
    ));
    let on_disk = std::fs::read_to_string(&file).unwrap();
    assert!(!on_disk.contains("hash:"));
    assert!(!on_disk.contains("last_updated:"));
}

#[test]
fn refuses_a_baseline_that_was_never_cleanup_stable() {
    // A body cleanup would rewrite (no blank line after the heading) must not
    // be mistaken for the agent's work.
    let dir = TempDir::new().unwrap();
    let original = "---\nprompt: test\n---\n# Title\nNo blank line\n";
    let (_file, plan) = agent_run(&dir, original, original);

    let outcome = reconcile_inline_artifact(&plan, "2026-09-06").unwrap();

    assert!(matches!(
        outcome,
        InlineReconciliation::Rejected(BodyRejection::Unchanged)
    ));
}

// -- typed failures ---------------------------------------------------------

#[test]
fn reports_a_malformed_stored_hash_without_mutating_the_document() {
    let dir = TempDir::new().unwrap();
    let original = "---\nprompt: test\nhash: not-a-hash\n---\nOld body\n";
    let agent_wrote = "---\nprompt: test\nhash: not-a-hash\n---\nAgent body\n";
    let (file, plan) = agent_run(&dir, original, agent_wrote);

    let error = reconcile_inline_artifact(&plan, "2026-09-06").unwrap_err();

    assert!(matches!(error, CompositionError::InlineHashMalformed(_)));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), agent_wrote);
}

#[test]
fn reports_a_duplicate_owned_key_without_mutating_the_document() {
    let dir = TempDir::new().unwrap();
    let original = "---\nprompt: test\n---\nOld body\n";
    let agent_wrote = "---\nprompt: one\nprompt: two\n---\nAgent body\n";
    let (file, plan) = agent_run(&dir, original, agent_wrote);

    let error = reconcile_inline_artifact(&plan, "2026-09-06").unwrap_err();

    assert!(
        matches!(error, CompositionError::InlineArtifactEditFailed { .. }),
        "expected a typed edit failure, got {error:?}"
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), agent_wrote);
}

#[test]
fn reports_a_document_the_agent_deleted() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let plan = plan(&file, "---\nprompt: test\n---\nOld body\n");

    let error = reconcile_inline_artifact(&plan, "2026-09-06").unwrap_err();

    assert!(
        matches!(error, CompositionError::InlineArtifactUnreadable { .. }),
        "expected a typed read failure, got {error:?}"
    );
}

// -- round trip -------------------------------------------------------------

#[test]
fn read_write_read_is_stable_and_the_second_pass_refuses() {
    let dir = TempDir::new().unwrap();
    let agent_wrote = AUTHORED_DOCUMENT.replace("Old body\n", "Agent body\n");
    let (file, guard) = agent_run(&dir, AUTHORED_DOCUMENT, &agent_wrote);

    written(reconcile_inline_artifact(&guard, "2026-09-06").unwrap());
    let first = std::fs::read_to_string(&file).unwrap();

    // The written artifact becomes the next run's baseline; an agent that
    // changes nothing is refused, and the bytes do not move.
    let second_guard = plan(&file, &first);
    let outcome = reconcile_inline_artifact(&second_guard, "2026-09-07").unwrap();
    assert!(matches!(
        outcome,
        InlineReconciliation::Rejected(BodyRejection::Unchanged)
    ));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), first);
}

#[test]
fn repeated_reconciliation_of_identical_inputs_is_byte_deterministic() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\nprompt: test\nlast_updated: '2026-01-01'\n---\nOld body\n";
    let agent_wrote = "---\nprompt: test\nlast_updated: '2026-01-01'\n---\nAgent body\n";
    let run = || {
        std::fs::write(&file, agent_wrote).unwrap();
        written(reconcile_inline_artifact(&plan(&file, original), "2026-09-06").unwrap());
        std::fs::read_to_string(&file).unwrap()
    };
    assert_eq!(run(), run());
}

// -- rollback ---------------------------------------------------------------

/// The guard restores its captured text byte-for-byte, including the stored
/// hash and date it carried before the run — a rollback stamps nothing.
#[test]
fn restoring_the_baseline_rewrites_the_captured_bytes_exactly() {
    let dir = TempDir::new().unwrap();
    let (file, plan) = agent_run(
        &dir,
        AUTHORED_DOCUMENT,
        "---\nprompt: mangled\n---\nHalf-written agent output\n",
    );

    restore_inline_baseline(&plan).expect("restoring an ordinary file succeeds");

    assert_eq!(std::fs::read_to_string(&file).unwrap(), AUTHORED_DOCUMENT);
}

/// Restoration is idempotent: a second rollback of an already-restored
/// document is a no-op rather than a second, differently stamped write.
#[test]
fn restoring_twice_is_byte_identical() {
    let dir = TempDir::new().unwrap();
    let (file, plan) = agent_run(&dir, AUTHORED_DOCUMENT, "---\nprompt: x\n---\nAgent\n");

    restore_inline_baseline(&plan).unwrap();
    let first = std::fs::read_to_string(&file).unwrap();
    restore_inline_baseline(&plan).unwrap();

    assert_eq!(std::fs::read_to_string(&file).unwrap(), first);
}

/// A restoring write that cannot land is a typed failure naming the document,
/// so the caller can attach it as the rollback cause instead of claiming the
/// file was put back.
#[cfg(unix)]
#[test]
fn a_restoration_that_cannot_write_reports_the_typed_rollback_failure() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let sealed = dir.path().join("sealed");
    std::fs::create_dir(&sealed).unwrap();
    let file = sealed.join("doc.md");
    std::fs::write(&file, "---\nprompt: x\n---\nAgent body\n").unwrap();
    let plan = plan(&file, AUTHORED_DOCUMENT);
    std::fs::set_permissions(&sealed, std::fs::Permissions::from_mode(0o555)).unwrap();

    let error = restore_inline_baseline(&plan).expect_err("a sealed directory refuses the write");
    std::fs::set_permissions(&sealed, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        matches!(error, CompositionError::InlineRollbackFailed { .. }),
        "unexpected error: {error}"
    );
    assert!(error.to_string().contains("doc.md"), "{error}");
    assert!(
        error.to_string().contains("could not restore"),
        "the wording must not imply the document was restored: {error}"
    );
}

// -- carried body-change evidence -------------------------------------------

/// Without evidence a metadata-only edit is refused as unchanged; with the
/// operation's carried evidence the same candidate is accepted, stamped, and
/// written (AC18).
#[test]
fn carried_body_change_evidence_accepts_a_metadata_only_candidate() {
    let dir = TempDir::new().unwrap();
    let agent_wrote = AUTHORED_DOCUMENT.replace(
        "last_updated: '2026-01-01'\n",
        "last_updated: '2026-01-01'\nresearched_by: goose\n",
    );
    let (file, plan) = agent_run(&dir, AUTHORED_DOCUMENT, &agent_wrote);

    assert!(
        matches!(
            reconcile_inline_artifact(&plan, "2026-09-06").unwrap(),
            InlineReconciliation::Rejected(BodyRejection::Unchanged)
        ),
        "without evidence an unchanged body is refused"
    );
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        agent_wrote,
        "a refusal writes nothing"
    );

    let artifact = written(
        reconcile_inline_artifact_with_evidence(&plan, "2026-09-06", true).unwrap(),
    );

    assert!(artifact.text.contains("researched_by: goose"), "{}", artifact.text);
    assert!(artifact.text.contains("last_updated: '2026-09-06'"), "{}", artifact.text);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), artifact.text);
}

/// Evidence never rescues an empty body: a blank document is not a deliverable
/// however much earlier work the operation produced.
#[test]
fn carried_evidence_still_refuses_an_empty_body() {
    let dir = TempDir::new().unwrap();
    let (_, plan) = agent_run(
        &dir,
        AUTHORED_DOCUMENT,
        &AUTHORED_DOCUMENT.replace("Old body\n", "   \n"),
    );

    assert!(matches!(
        reconcile_inline_artifact_with_evidence(&plan, "2026-09-06", true).unwrap(),
        InlineReconciliation::Rejected(BodyRejection::Empty)
    ));
}
