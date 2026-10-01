use super::*;
use darkmatter::markdown::MarkdownError;
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

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap());

    let on_disk = std::fs::read_to_string(&file).unwrap();
    assert_eq!(on_disk, artifact.text);
    let markdown: Markdown = on_disk.clone().into();
    let options = inline_hash_options();
    let stored = markdown.stored_hash(&options)
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
fn stamps_the_utc_date_when_the_local_date_is_ahead() {
    let dir = TempDir::new().unwrap();
    let agent_wrote = AUTHORED_DOCUMENT.replace("Old body\n", "Agent body\n");
    let (file, plan) = agent_run(&dir, AUTHORED_DOCUMENT, &agent_wrote);
    // 23:30 UTC on the 6th is already the 7th at UTC+10.
    let evening_utc: chrono::DateTime<chrono::Utc> = "2026-09-06T23:30:00Z".parse().unwrap();

    written(reconcile_inline_artifact(&plan, evening_utc).unwrap());

    let on_disk = std::fs::read_to_string(&file).unwrap();
    assert!(on_disk.contains("last_updated: '2026-09-06'"), "{on_disk}");
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

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap());

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

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap());

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

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap());

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

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap());

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

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap());

    assert!(artifact.body_cleaned);
    let on_disk = std::fs::read_to_string(&file).unwrap();
    assert!(on_disk.contains("# Title\n\nNo blank line"));
    let markdown: Markdown = on_disk.into();
    let options = inline_hash_options();
    let stored = markdown.stored_hash(&options)
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

        written(reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap());

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

    let outcome = reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap();

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

    let outcome = reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap();

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

    let outcome = reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap();

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

    let outcome = reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap();

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

    let error = reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap_err();

    assert!(matches!(error, CompositionError::InlineHashMalformed(_)));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), agent_wrote);
}

#[test]
fn reports_a_duplicate_owned_key_without_mutating_the_document() {
    let dir = TempDir::new().unwrap();
    let original = "---\nprompt: test\n---\nOld body\n";
    let agent_wrote = "---\nprompt: one\nprompt: two\n---\nAgent body\n";
    let (file, plan) = agent_run(&dir, original, agent_wrote);

    let error = reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap_err();

    // The agent wrote the second `prompt` line, so the refusal names it.
    let CompositionError::InlineAgentFrontmatterRejected { rejection, .. } = &error else {
        panic!("expected an agent-attributed rejection, got {error:?}");
    };
    assert_eq!(rejection.line, Some(3), "{rejection}");
    assert!(rejection.agent_edit, "{rejection}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), agent_wrote);
}

#[test]
fn reports_a_document_the_agent_deleted() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let plan = plan(&file, "---\nprompt: test\n---\nOld body\n");

    let error = reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap_err();

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

    written(reconcile_inline_artifact(&guard, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap());
    let first = std::fs::read_to_string(&file).unwrap();

    // The written artifact becomes the next run's baseline; an agent that
    // changes nothing is refused, and the bytes do not move.
    let second_guard = plan(&file, &first);
    let outcome = reconcile_inline_artifact(&second_guard, "2026-09-07T12:00:00Z".parse().unwrap()).unwrap();
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
        written(reconcile_inline_artifact(&plan(&file, original), "2026-09-06T12:00:00Z".parse().unwrap()).unwrap());
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
            reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap(),
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
        reconcile_inline_artifact_with_evidence(&plan, "2026-09-06T12:00:00Z".parse().unwrap(), true).unwrap(),
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
        reconcile_inline_artifact_with_evidence(&plan, "2026-09-06T12:00:00Z".parse().unwrap(), true).unwrap(),
        InlineReconciliation::Rejected(BodyRejection::Empty)
    ));
}

// -- agent-written frontmatter: repair and literal tokens (R3, R4) ----------

/// The spec's inline case through the closure: the agent adds four values,
/// two of which YAML misreads and two of which would read as instructions.
/// The file is valid YAML, `summary` and `cmd` are stored as tokens, `note`
/// and `title` as quoted strings; the delta the completion schema sees holds
/// the decoded text; and the stamped hash agrees with the written bytes.
#[test]
fn agent_values_are_repaired_encoded_hashed_and_reported_decoded() {
    use darkmatter::markdown::literal_token::encode_yaml_scalar;

    let dir = TempDir::new().unwrap();
    let original = "---\nprompt: write it\narea_note: \"in {{ area }}\"\n---\nOld body\n";
    let agent_wrote = concat!(
        "---\n",
        "prompt: write it\n",
        "area_note: \"in {{ area }}\"\n",
        "summary: fixed {{…}} parsing\n",
        "note: see issue #42\n",
        "cmd: \"$(echo X)\"\n",
        "title: Fix: colons\n",
        "---\n",
        "New body\n",
    );
    let (file, plan) = agent_run(&dir, original, agent_wrote);

    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap());

    let on_disk = std::fs::read_to_string(&file).unwrap();
    assert_eq!(on_disk, artifact.text);
    for line in [
        "area_note: \"in {{ area }}\"\n".to_string(),
        format!("summary: {}\n", encode_yaml_scalar("fixed {{…}} parsing")),
        "note: \"see issue #42\"\n".to_string(),
        format!("cmd: {}\n", encode_yaml_scalar("$(echo X)")),
        "title: \"Fix: colons\"\n".to_string(),
    ] {
        assert!(on_disk.contains(&line), "missing {line:?} in:\n{on_disk}");
    }

    let reported: Vec<(String, serde_json::Value)> = artifact
        .frontmatter_delta
        .entries
        .iter()
        .map(|entry| match entry {
            FrontmatterDeltaEntry::Addition { property, value } => {
                (property.clone(), value.clone())
            }
            other => panic!("unexpected delta entry {other:?}"),
        })
        .collect();
    assert_eq!(
        reported,
        vec![
            ("summary".to_string(), serde_json::json!("fixed {{…}} parsing")),
            ("note".to_string(), serde_json::json!("see issue #42")),
            ("cmd".to_string(), serde_json::json!("$(echo X)")),
            ("title".to_string(), serde_json::json!("Fix: colons")),
        ]
    );

    let markdown: Markdown = on_disk.into();
    let options = inline_hash_options();
    let stored = markdown.stored_hash(&options).unwrap().unwrap();
    let comparison = markdown.compare_hash(&stored, &options).unwrap();
    assert!(!comparison.frontmatter_changed && !comparison.body_changed);
}

/// Read, write, read: a second closure over the first run's output, with the
/// agent leaving the stored tokens alone, keeps their bytes.
#[test]
fn stored_tokens_survive_a_second_run_byte_for_byte() {
    let dir = TempDir::new().unwrap();
    let original = "---\nprompt: write it\n---\nOld body\n";
    let first = "---\nprompt: write it\nsummary: fixed {{…}} parsing\n---\nFirst body\n";
    let (file, plan) = agent_run(&dir, original, first);
    let first_text = written(reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap()).text;
    let token_line = first_text.lines().find(|line| line.starts_with("summary:")).unwrap().to_string();

    let second = first_text.replace("First body", "Second body");
    let (_, plan) = agent_run(&dir, &first_text, &second);
    let artifact = written(reconcile_inline_artifact(&plan, "2026-09-07T12:00:00Z".parse().unwrap()).unwrap());

    assert!(artifact.text.contains(&format!("{token_line}\n")), "{}", artifact.text);
    assert!(artifact.frontmatter_delta.is_empty(), "{:?}", artifact.frontmatter_delta);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), artifact.text);
}

/// An unrepairable edit is refused before anything is written, with the line
/// and the agent named. The caller's rollback then restores the baseline.
#[test]
fn an_unrepairable_edit_is_refused_without_writing() {
    let dir = TempDir::new().unwrap();
    let original = "---\nprompt: write it\ntitle: t\n---\nOld body\n";
    for (agent_wrote, line) in [
        ("---\nprompt: write it\ntitle: t\ntitle: again\n---\nNew body\n", 4),
        ("---\nprompt: write it\ntitle: t\nmeta:\n  a: b: c\n---\nNew body\n", 5),
        ("---\r\nprompt: write it\r\ntitle: t\r\nmeta:\r\n  a: b: c\r\n---\r\nNew body\r\n", 5),
    ] {
        let (file, plan) = agent_run(&dir, original, agent_wrote);
        let error = reconcile_inline_artifact(&plan, "2026-09-06T12:00:00Z".parse().unwrap()).unwrap_err();
        let CompositionError::InlineAgentFrontmatterRejected { rejection, .. } = &error else {
            panic!("expected an agent-attributed rejection, got {error:?}");
        };
        assert_eq!(rejection.line, Some(line), "{rejection}");
        assert!(rejection.agent_edit, "{rejection}");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), agent_wrote, "nothing written");

        restore_inline_baseline(&plan).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
    }
}

/// Stamping the new hash replaces the anchored `hash` node; `mirror` would then
/// resolve to `earlier`'s same-named anchor. The write-back refuses before the
/// atomic write.
#[test]
fn a_hash_stamp_that_would_repoint_an_alias_is_refused_without_writing() {
    let dir = TempDir::new().unwrap();
    let original = concat!(
        "---\n",
        "prompt: test\n",
        "earlier: &h before\n",
        "hash: &h a000000000000000-b000000000000000\n",
        "mirror: *h\n",
        "---\n",
        "Old body\n",
    );
    let agent_wrote = original.replace("Old body\n", "Agent body\n");
    let (file, plan) = agent_run(&dir, original, &agent_wrote);

    let error = reconcile_inline_artifact(&plan, "2026-09-06").unwrap_err();

    let CompositionError::InlineHashMalformed(MarkdownError::FrontmatterTextEdit { reason }) =
        &error
    else {
        panic!("expected a refused frontmatter edit, got {error:?}");
    };
    assert!(reason.contains("`mirror`"), "{reason}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), agent_wrote, "nothing written");
}

/// Restoring the owned `hash` drops the anchor the agent's new `mirror` alias
/// resolved to, so `mirror` would silently take `earlier`'s value.
#[test]
fn an_owned_restoration_that_would_repoint_an_alias_is_refused_without_writing() {
    let dir = TempDir::new().unwrap();
    let original = concat!(
        "---\n",
        "prompt: test\n",
        "earlier: &h before\n",
        "hash: a000000000000000-b000000000000000\n",
        "---\n",
        "Old body\n",
    );
    let agent_wrote = concat!(
        "---\n",
        "prompt: test\n",
        "earlier: &h before\n",
        "hash: &h a000000000000000-b000000000000000\n",
        "mirror: *h\n",
        "---\n",
        "Agent body\n",
    );
    let (file, plan) = agent_run(&dir, original, agent_wrote);

    let error = reconcile_inline_artifact(&plan, "2026-09-06").unwrap_err();

    let CompositionError::InlineArtifactEditFailed(MarkdownError::FrontmatterTextEdit { reason }) =
        &error
    else {
        panic!("expected a refused frontmatter edit, got {error:?}");
    };
    assert!(reason.contains("`mirror`"), "{reason}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), agent_wrote, "nothing written");
}

/// An indented comment under `last_updated` is not part of its value: the
/// closure stamps the date and keeps the comment and every terminator.
#[test]
fn stamps_a_date_above_an_indented_comment_and_keeps_every_byte() {
    let document = |date_line: &str, hash: &str, body: &str, newline: &str| {
        [
            "---",
            "prompt: test",
            &format!("hash: {hash}"),
            date_line,
            "  # set this when the body changes",
            "author: A",
            "---",
            body,
            "",
        ]
        .join(newline)
    };
    let stale = "a000000000000000-b000000000000000";
    for newline in ["\n", "\r\n", "\r"] {
        for authored in ["last_updated: 2026-01-01", "last_updated:"] {
            let dir = TempDir::new().unwrap();
            let original = document(authored, stale, "Old body", newline);
            let agent_wrote = document(authored, stale, "Agent body", newline);
            let (file, plan) = agent_run(&dir, &original, &agent_wrote);

            let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06").unwrap());

            let on_disk = std::fs::read_to_string(&file).unwrap();
            assert_eq!(on_disk, artifact.text);
            let hash = on_disk
                .split(newline)
                .find_map(|line| line.strip_prefix("hash: "))
                .unwrap_or_else(|| panic!("{authored:?} {newline:?}: no hash in {on_disk:?}"));
            // The body goes through the closure's cleanup, which owns its
            // terminators; every frontmatter byte must match exactly.
            let expected = document("last_updated: 2026-09-06", hash, "Agent body", newline);
            let frontmatter_end = expected.find("Agent body").unwrap();
            assert_eq!(
                on_disk[..frontmatter_end],
                expected[..frontmatter_end],
                "{authored:?} {newline:?}"
            );
            assert_eq!(on_disk[frontmatter_end..].trim_end(), "Agent body");
        }
    }
}

#[test]
fn stamps_a_date_after_a_quote_inside_a_plain_value_and_keeps_its_comment() {
    let document = |date: &str, hash: &str, body: &str, newline: &str| {
        [
            "---",
            "prompt: test",
            &format!("hash: {hash}"),
            &format!("last_updated: {date}   # keep this explanation"),
            "author: A",
            "---",
            body,
            "",
        ]
        .join(newline)
    };
    let stale = "a000000000000000-b000000000000000";
    for newline in ["\n", "\r\n", "\r"] {
        for authored in ["yesterday's date", "unknown \"date"] {
            let dir = TempDir::new().unwrap();
            let original = document(authored, stale, "Old body", newline);
            let agent_wrote = document(authored, stale, "Agent body", newline);
            let (file, plan) = agent_run(&dir, &original, &agent_wrote);

            let artifact = written(reconcile_inline_artifact(&plan, "2026-09-06").unwrap());

            let on_disk = std::fs::read_to_string(&file).unwrap();
            assert_eq!(on_disk, artifact.text);
            let hash = on_disk
                .split(newline)
                .find_map(|line| line.strip_prefix("hash: "))
                .unwrap_or_else(|| panic!("{authored:?} {newline:?}: no hash in {on_disk:?}"));
            let expected = document("2026-09-06", hash, "Agent body", newline);
            let frontmatter_end = expected.find("Agent body").unwrap();
            assert_eq!(
                on_disk[..frontmatter_end],
                expected[..frontmatter_end],
                "{authored:?} {newline:?}"
            );
            assert_eq!(on_disk[frontmatter_end..].trim_end(), "Agent body");
        }
    }
}

/// A `:` inside a plain flow sibling or key (`a:'b`) is content, and so is the
/// quote after it: an agent-written quoted target beside it is still encoded
/// in place, its siblings keep their bytes, and the document stamps.
#[test]
fn encodes_a_quoted_flow_target_beside_a_content_colon() {
    // `{T}` marks the quoted target; each row is the value of a new `q`.
    let rows = [
        // Controls: ordinary and `don't` siblings, genuinely quoted colons,
        // and adjacent JSON-like keys.
        ("[ordinary, {T}]", "\"{{x}}\""),
        ("[don't, {T}]", "'{{x}}'"),
        ("['a:''b', {T}]", "\"{{x}}\""),
        ("{\"a\":b, c: {T}}", "'{{x}}'"),
        ("[http://x, {T}]", "\"{{x}}\""),
        // Both quote spellings after a content colon, balanced spellings,
        // plain keys, mapping values, and nested collections.
        ("[a:'b, {T}]", "\"{{x}}\""),
        ("[a:\"b, {T}]", "'{{x}}'"),
        ("[a:'b, c:'d, {T}]", "\"{{x}}\""),
        ("[a:\"b, c:\"d, {T}]", "'{{x}}'"),
        ("{a:'b: {T}}", "\"{{x}}\""),
        ("{a:\"b: {T}}", "'{{x}}'"),
        ("{a: a:'b, b: {T}}", "\"{{x}}\""),
        ("{a: a:\"b, b: {T}}", "'{{x}}'"),
        ("{list: [a:'b, {T}]}", "\"{{x}}\""),
        ("[{k: a:'b}, {k: {T}}]", "'{{x}}'"),
    ];
    assert_quoted_flow_targets_persist_exactly(&rows);
}

/// Runs each `(flow, target)` row, where `{T}` in `flow` marks the quoted
/// `target`, through [`reconcile_inline_artifact`] as the value of a new `q`
/// under LF and CRLF, and asserts the exact persisted bytes: the target
/// replaced by its encoded literal token, siblings byte-identical, one
/// `Addition` delta equal to the authored parse, and a clean stored hash.
fn assert_quoted_flow_targets_persist_exactly(rows: &[(&str, &str)]) {
    use darkmatter::markdown::literal_token::encode_yaml_scalar;

    let encoded = encode_yaml_scalar("{{x}}");
    for newline in ["\n", "\r\n"] {
        for &(flow, target) in rows {
            let document = |extra: Option<&str>, body: &str| {
                let mut lines = vec!["---", "prompt: test"];
                lines.extend(extra);
                lines.extend(["after: z", "---", body, ""]);
                lines.join(newline)
            };
            let authored = format!("q: {}", flow.replace("{T}", target));
            let dir = TempDir::new().unwrap();
            let original = document(None, "Old body");
            let agent_wrote = document(Some(&authored), "Agent body");
            let (file, plan) = agent_run(&dir, &original, &agent_wrote);

            let artifact = written(
                reconcile_inline_artifact(&plan, "2026-09-06")
                    .unwrap_or_else(|error| panic!("{authored:?} {newline:?}: {error}")),
            );

            let on_disk = std::fs::read_to_string(&file).unwrap();
            assert_eq!(on_disk, artifact.text);
            let hash = on_disk
                .split(newline)
                .find_map(|line| line.strip_prefix("hash: "))
                .unwrap_or_else(|| panic!("{authored:?}: no hash in {on_disk:?}"));
            let stored_q = flow.replace("{T}", &encoded);
            let expected = [
                "---",
                "prompt: test",
                &format!("q: {stored_q}"),
                "after: z",
                &format!("hash: {hash}"),
                "last_updated: 2026-09-06",
                "---",
                "",
            ]
            .join(newline);
            // The body is the agent's, through Darkmatter's cleanup pass.
            let body = darkmatter::markdown::cleanup::cleanup_content(&format!("Agent body{newline}"));
            assert_eq!(on_disk, format!("{expected}{body}"), "{authored:?} {newline:?}");

            let authored_value: serde_json::Value =
                biscuit_file::serde_yaml_ng::from_str::<serde_json::Value>(&authored).unwrap()["q"].clone();
            let FrontmatterDeltaEntry::Addition { property, value } =
                &artifact.frontmatter_delta.entries[..]
                    .first()
                    .unwrap_or_else(|| panic!("{authored:?}: empty delta"))
            else {
                panic!("{authored:?}: expected an addition");
            };
            assert_eq!(artifact.frontmatter_delta.entries.len(), 1, "{authored:?}");
            assert_eq!((property.as_str(), value), ("q", &authored_value), "{authored:?}");

            let markdown: Markdown = on_disk.into();
            let options = inline_hash_options();
            let stored = parse_inline_stored_hash(&markdown, &options).unwrap().unwrap();
            let comparison = markdown.compare_hash(&stored, &options).unwrap();
            assert!(!comparison.frontmatter_changed && !comparison.body_changed, "{authored:?}");
        }
    }
}

/// A parenthesis is plain-scalar content in YAML, so it never hides the `,`
/// before an agent-written quoted target, even when a later entry closes it.
#[test]
fn encodes_a_quoted_flow_target_beside_parentheses() {
    // `{T}` marks the quoted target; each row is the value of a new `q`.
    let rows = [
        // Controls: parentheses inside quotes and balanced in one entry.
        ("['a(b', {T}]", "\"{{x}}\""),
        ("[\"a(b, c)d\", {T}]", "'{{x}}'"),
        ("[a(b)c, {T}]", "\"{{x}}\""),
        // Unbalanced and cross-entry balanced parentheses, mappings, and
        // nested collections.
        ("[a(b, {T}]", "\"{{x}}\""),
        ("[a)b, {T}]", "'{{x}}'"),
        ("[a(b, c)d, {T}]", "\"{{x}}\""),
        ("{a: a(b, b: {T}}", "\"{{x}}\""),
        ("{a(b: x, c)d: {T}}", "'{{x}}'"),
        ("{list: [a(b, {T}]}", "\"{{x}}\""),
        ("[{k: a(b}, {k: c)d}, {T}]", "'{{x}}'"),
        ("[[a(b, c)d], {T}]", "\"{{x}}\""),
    ];
    assert_quoted_flow_targets_persist_exactly(&rows);
}

/// A quote inside a plain flow sibling or key is content, so an agent-written
/// quoted target beside it is still encoded in place and the document stamps.
#[test]
fn encodes_a_quoted_flow_target_beside_a_plain_scalar_holding_a_quote() {
    use darkmatter::markdown::literal_token::encode_yaml_scalar;

    // `{T}` marks the quoted target; each row is the value of a new `q`.
    let rows = [
        ("[ordinary, {T}]", "\"{{x}}\""),
        ("['don''t', {T}]", "'{{x}}'"),
        ("{'don''t': {T}}", "\"{{x}}\""),
        ("[don't, {T}]", "\"{{x}}\""),
        ("[say \"hi, {T}]", "'{{x}}'"),
        ("[don't, can't, {T}]", "\"{{x}}\""),
        ("{don't: {T}}", "\"{{x}}\""),
        ("{say \"hi: {T}}", "'{{x}}'"),
        ("{a: don't, b: {T}}", "\"{{x}}\""),
        ("{a: say \"hi, b: {T}}", "'{{x}}'"),
        ("{list: [don't, {T}], z: 1}", "\"{{x}}\""),
        ("[{k: don't}, {k: {T}}]", "'{{x}}'"),
        ("[don't, {can't: {T}}]", "\"{{x}}\""),
    ];
    let encoded = encode_yaml_scalar("{{x}}");
    for newline in ["\n", "\r\n"] {
        for (flow, target) in rows {
            let document = |extra: Option<&str>, body: &str| {
                let mut lines = vec!["---", "prompt: test"];
                lines.extend(extra);
                lines.extend(["after: z", "---", body, ""]);
                lines.join(newline)
            };
            let authored = format!("q: {}", flow.replace("{T}", target));
            let dir = TempDir::new().unwrap();
            let original = document(None, "Old body");
            let agent_wrote = document(Some(&authored), "Agent body");
            let (file, plan) = agent_run(&dir, &original, &agent_wrote);

            let artifact = written(
                reconcile_inline_artifact(&plan, "2026-09-06")
                    .unwrap_or_else(|error| panic!("{authored:?} {newline:?}: {error}")),
            );

            let on_disk = std::fs::read_to_string(&file).unwrap();
            assert_eq!(on_disk, artifact.text);
            let stored_line = format!("q: {}{newline}", flow.replace("{T}", &encoded));
            let expected_start = format!("---{newline}prompt: test{newline}{stored_line}after: z{newline}");
            assert!(
                on_disk.starts_with(&expected_start),
                "{authored:?} {newline:?}: {on_disk:?}"
            );
            assert!(on_disk.contains(&format!("last_updated: 2026-09-06{newline}")), "{on_disk:?}");

            let authored_value: serde_json::Value =
                biscuit_file::serde_yaml_ng::from_str::<serde_json::Value>(&authored).unwrap()["q"].clone();
            let FrontmatterDeltaEntry::Addition { property, value } =
                &artifact.frontmatter_delta.entries[..]
                    .first()
                    .unwrap_or_else(|| panic!("{authored:?}: empty delta"))
            else {
                panic!("{authored:?}: expected an addition");
            };
            assert_eq!(artifact.frontmatter_delta.entries.len(), 1, "{authored:?}");
            assert_eq!((property.as_str(), value), ("q", &authored_value), "{authored:?}");

            let markdown: Markdown = on_disk.into();
            let options = inline_hash_options();
            let stored = parse_inline_stored_hash(&markdown, &options).unwrap().unwrap();
            let comparison = markdown.compare_hash(&stored, &options).unwrap();
            assert!(!comparison.frontmatter_changed && !comparison.body_changed, "{authored:?}");
        }
    }
}
