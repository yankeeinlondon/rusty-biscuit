use super::*;

// -- extract_replacement_body -------------------------------------------

#[test]
fn extract_body_returns_trimmed_content() {
    let body = extract_replacement_body("  Hello world  ").unwrap();
    assert_eq!(body, "Hello world");
}

#[test]
fn extract_body_rejects_empty_input() {
    assert!(extract_replacement_body("").is_err());
    assert!(extract_replacement_body("   ").is_err());
}

#[test]
fn extract_body_strips_accidental_frontmatter() {
    let output = "---\ntitle: oops\n---\nActual body content\n";
    let body = extract_replacement_body(output).unwrap();
    assert_eq!(body, "Actual body content");
}

#[test]
fn extract_body_rejects_frontmatter_only() {
    let output = "---\ntitle: oops\n---\n";
    assert!(extract_replacement_body(output).is_err());
}

#[test]
fn apply_inline_closure_rejects_unchanged_body() {
    let original = "---\nprompt: write\n---\nOriginal body\n";
    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let original_hash = original_markdown
        .compute_hash(MdHashKind::Simple, &inline_hash_options());
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_hash,
    };

    let err = apply_inline_closure(
        &plan,
        "Original body",
        Path::new("/tmp/nonexistent"),
        "2026-03-27",
        None,
    )
    .unwrap_err();

    assert!(matches!(err, CompositionError::InvalidInlineResponse(_)));
    assert!(err.to_string().contains("unchanged"));

    // Port verification: the captured body segment is a non-empty 16-hex string.
    assert_eq!(simple_body(&plan.original_hash).len(), 16);
    assert!(!simple_body(&plan.original_hash).is_empty());
    assert!(simple_body(&plan.original_hash).chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn extract_body_preserves_dashes_that_are_not_frontmatter() {
    let output = "Some text\n---\nMore text\n";
    let body = extract_replacement_body(output).unwrap();
    assert_eq!(body, "Some text\n---\nMore text");
}

// -- rewrite_inline_document --------------------------------------------

#[test]
fn rewrite_preserves_block_scalar_frontmatter_layout() {
    let original = concat!(
        "---\n",
        "prompt: |-\n",
        "  First line\n",
        "  Second line\n",
        "last_updated: 2026-03-18\n",
        "---\n",
        "Old body\n",
    );

    let rewritten =
        rewrite_inline_document(original, "Fresh body\n", "2026-03-19", &[]).unwrap();

    assert!(rewritten.contains("prompt: |-"));
    assert!(rewritten.contains("  First line\n  Second line\n"));
    assert!(rewritten.contains("last_updated: 2026-03-19"));
    assert!(rewritten.ends_with("---\nFresh body\n"));
}

#[test]
fn rewrite_adds_last_updated_without_reserializing_frontmatter() {
    let original = concat!(
        "---\n",
        "prompt: |-\n",
        "  Keep this formatting\n",
        "---\n",
        "Body\n",
    );

    let rewritten =
        rewrite_inline_document(original, "Updated body\n", "2026-03-19", &[]).unwrap();

    assert!(rewritten.contains("prompt: |-"));
    assert!(rewritten.contains("  Keep this formatting\n"));
    assert!(rewritten.contains("last_updated: 2026-03-19\n---\nUpdated body\n"));
}

#[test]
fn rewrite_updates_quoted_last_updated() {
    let original = "---\nlast_updated: \"2026-01-01\"\n---\nBody\n";
    let rewritten = rewrite_inline_document(original, "New body\n", "2026-03-27", &[]).unwrap();
    assert!(rewritten.contains("last_updated: \"2026-03-27\""));
}

#[test]
fn rewrite_updates_single_quoted_last_updated() {
    let original = "---\nlast_updated: '2026-01-01'\n---\nBody\n";
    let rewritten = rewrite_inline_document(original, "New body\n", "2026-03-27", &[]).unwrap();
    assert!(rewritten.contains("last_updated: '2026-03-27'"));
}

#[test]
fn rewrite_preserves_schema_property_with_inline_value() {
    // Phase 5 Task 2: `$schema` must survive the inline rewrite so the
    // document continues to validate on subsequent runs.
    let original = concat!(
        "---\n",
        "$schema:\n",
        "  title: 'string(required)'\n",
        "prompt: |-\n",
        "  write a title\n",
        "title: Hello\n",
        "---\n",
        "Old body\n",
    );

    let rewritten =
        rewrite_inline_document(original, "Fresh body\n", "2026-05-26", &[]).unwrap();

    assert!(rewritten.contains("$schema:"));
    assert!(rewritten.contains("title: 'string(required)'"));
    assert!(rewritten.contains("last_updated: 2026-05-26"));
    assert!(rewritten.ends_with("---\nFresh body\n"));
}

#[test]
fn rewrite_does_not_persist_set_only_keys() {
    // Phase 5 Task 2: interactive-collected values flow through
    // `--set` overrides during composition only; they must not appear
    // in the rewritten document. The rewrite reuses the original
    // frontmatter text and only adds `last_updated` + any new keys
    // explicitly handed to it. Pass an empty `new_properties` slice
    // to simulate the inline closure path.
    let original = "---\n$schema:\n  title: 'string(required)'\n---\nOld\n";
    let rewritten = rewrite_inline_document(original, "New body\n", "2026-05-26", &[]).unwrap();

    // The collected `title` value is never in the original document
    // and should not be inserted on the way out.
    assert!(!rewritten.contains("title: Plan"));
    assert!(rewritten.contains("$schema:"));
    assert!(rewritten.contains("last_updated: 2026-05-26"));
}

#[test]
fn rewrite_preserves_crlf_line_endings() {
    let original = "---\r\nlast_updated: 2026-01-01\r\n---\r\nBody\r\n";
    let rewritten =
        rewrite_inline_document(original, "New body\r\n", "2026-03-27", &[]).unwrap();
    assert!(rewritten.contains("last_updated: 2026-03-27\r\n"));
}

// -- apply_inline_closure -----------------------------------------------

#[test]
fn apply_closure_rejects_empty_body() {
    let original = "---\nprompt: test\n---\nOld\n";
    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let original_hash = original_markdown
        .compute_hash(MdHashKind::Simple, &inline_hash_options());
    let plan = InlineClosurePlan {
        original_document_text: original.into(),
        original_hash,
    };
    let err = apply_inline_closure(
        &plan,
        "  ",
        Path::new("/tmp/nonexistent"),
        "2026-03-27",
        None,
    );
    assert!(err.is_err());
}

// -- apply_inline_closure with post-run frontmatter -----------------------

#[test]
fn rewrite_merges_new_properties_before_last_updated() {
    let original = "---\nprompt: test\nlast_updated: 2026-03-18\n---\nOld body\n";
    let new_props = vec![("tags".to_string(), "tags: research\n".to_string())];
    let rewritten =
        rewrite_inline_document(original, "New body\n", "2026-04-02", &new_props).unwrap();
    assert!(rewritten.contains("prompt: test\n"));
    assert!(rewritten.contains("tags: research\n"));
    assert!(rewritten.contains("last_updated: 2026-04-02\n"));
    assert!(rewritten.contains("New body\n"));
    let tags_pos = rewritten.find("tags:").unwrap();
    let lu_pos = rewritten.find("last_updated:").unwrap();
    assert!(tags_pos < lu_pos);
}

#[test]
fn rewrite_no_new_properties_backward_compatible() {
    let original = "---\nprompt: test\nlast_updated: 2026-03-18\n---\nOld body\n";
    let rewritten = rewrite_inline_document(original, "New body\n", "2026-04-02", &[]).unwrap();
    assert!(rewritten.contains("prompt: test\n"));
    assert!(rewritten.contains("last_updated: 2026-04-02\n"));
    assert!(rewritten.contains("New body\n"));
    // No extra properties injected
    assert!(!rewritten.contains("tags:"));
}

// -- strip_leading_frontmatter ------------------------------------------

#[test]
fn strip_frontmatter_removes_leading_fences() {
    let text = "---\ntitle: test\n---\nBody here\n";
    assert_eq!(strip_leading_frontmatter(text), "Body here\n");
}

#[test]
fn strip_frontmatter_preserves_text_without_fences() {
    let text = "Just plain text\nwith lines\n";
    assert_eq!(strip_leading_frontmatter(text), text);
}

#[test]
fn strip_frontmatter_preserves_unclosed_fences() {
    let text = "---\ntitle: test\nNo closing fence\n";
    assert_eq!(strip_leading_frontmatter(text), text);
}

// -- serialize_frontmatter_property ----------------------------------------

#[test]
fn serialize_string_property() {
    let value = serde_json::json!("hello world");
    let result = serialize_frontmatter_property("title", &value);
    assert_eq!(result, "title: hello world\n");
}

#[test]
fn serialize_string_with_special_chars_is_quoted() {
    let value = serde_json::json!("key: value");
    let result = serialize_frontmatter_property("note", &value);
    assert!(
        result.contains("'key: value'") || result.contains("\"key: value\""),
        "expected quoted YAML value, got: {result}"
    );
}

#[test]
fn serialize_string_with_colon_hash() {
    let value = serde_json::json!("item #1: top");
    let result = serialize_frontmatter_property("desc", &value);
    assert!(
        result.contains('#'),
        "hash should be preserved in quoted value: {result}"
    );
}

#[test]
fn serialize_number_property() {
    let result = serialize_frontmatter_property("count", &serde_json::json!(42));
    assert_eq!(result, "count: 42\n");
}

#[test]
fn serialize_bool_property() {
    let result = serialize_frontmatter_property("draft", &serde_json::json!(true));
    assert_eq!(result, "draft: true\n");
}

#[test]
fn serialize_null_property() {
    let result = serialize_frontmatter_property("removed", &serde_json::json!(null));
    assert_eq!(result, "removed: null\n");
}

#[test]
fn serialize_array_property() {
    let value = serde_json::json!(["rust", "markdown"]);
    let result = serialize_frontmatter_property("tags", &value);
    assert!(result.starts_with("tags:\n"));
    assert!(result.contains("  - rust"));
    assert!(result.contains("  - markdown"));
}

#[test]
fn serialize_object_property() {
    let value = serde_json::json!({"version": "1.0"});
    let result = serialize_frontmatter_property("meta", &value);
    assert!(result.starts_with("meta:\n"));
    assert!(result.contains("version"));
}

// -- upsert_last_updated ------------------------------------------------

#[test]
fn upsert_adds_when_missing() {
    let yaml = "prompt: test\n";
    let result = upsert_last_updated_in_frontmatter(yaml, "2026-03-27", "\n", &[]);
    assert!(result.contains("last_updated: 2026-03-27\n"));
    assert!(result.contains("prompt: test\n"));
}

#[test]
fn upsert_replaces_existing() {
    let yaml = "last_updated: 2026-01-01\nprompt: test\n";
    let result = upsert_last_updated_in_frontmatter(yaml, "2026-03-27", "\n", &[]);
    assert!(result.contains("last_updated: 2026-03-27\n"));
    assert!(!result.contains("2026-01-01"));
}

#[test]
fn upsert_ignores_indented_last_updated() {
    let yaml = "  last_updated: 2026-01-01\n";
    let result = upsert_last_updated_in_frontmatter(yaml, "2026-03-27", "\n", &[]);
    // Indented line should not be rewritten (it's nested YAML)
    assert!(result.contains("  last_updated: 2026-01-01"));
    // A new top-level entry should be appended
    assert!(result.contains("last_updated: 2026-03-27\n"));
}

// -- upsert with new properties -------------------------------------------

#[test]
fn upsert_injects_new_properties_before_last_updated() {
    let yaml = "prompt: test\nlast_updated: 2026-01-01\n";
    let new_props = vec!["tags: research\n".to_string()];
    let result = upsert_last_updated_in_frontmatter(yaml, "2026-04-02", "\n", &new_props);
    assert!(result.contains("prompt: test\n"));
    assert!(result.contains("tags: research\n"));
    assert!(result.contains("last_updated: 2026-04-02\n"));
    // tags must appear before last_updated
    let tags_pos = result.find("tags:").unwrap();
    let lu_pos = result.find("last_updated:").unwrap();
    assert!(
        tags_pos < lu_pos,
        "new property should appear before last_updated"
    );
}

#[test]
fn upsert_injects_new_properties_when_last_updated_missing() {
    let yaml = "prompt: test\n";
    let new_props = vec!["tags: research\n".to_string()];
    let result = upsert_last_updated_in_frontmatter(yaml, "2026-04-02", "\n", &new_props);
    assert!(result.contains("tags: research\n"));
    assert!(result.contains("last_updated: 2026-04-02\n"));
    let tags_pos = result.find("tags:").unwrap();
    let lu_pos = result.find("last_updated:").unwrap();
    assert!(tags_pos < lu_pos);
}

#[test]
fn upsert_empty_new_properties_behaves_as_before() {
    let yaml = "prompt: test\nlast_updated: 2026-01-01\n";
    let no_props: Vec<String> = vec![];
    let result = upsert_last_updated_in_frontmatter(yaml, "2026-04-02", "\n", &no_props);
    assert_eq!(result, "prompt: test\nlast_updated: 2026-04-02\n");
}

// -- end-to-end integration tests -----------------------------------------

#[test]
fn apply_closure_merges_new_and_reports_reverted() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: original prompt\ntitle: My Doc\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let original_hash = original_markdown
        .compute_hash(MdHashKind::Simple, &inline_hash_options());
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_hash,
    };

    // Simulate post-run state: title changed, tags added
    let mut post_run_fm = indexmap::IndexMap::new();
    post_run_fm.insert("prompt".to_string(), serde_json::json!("original prompt"));
    post_run_fm.insert("title".to_string(), serde_json::json!("Changed Title"));
    post_run_fm.insert("tags".to_string(), serde_json::json!("research"));

    let result = apply_inline_closure(
        &plan,
        "Brand new body content\n",
        &file,
        "2026-04-02",
        Some(&post_run_fm),
    )
    .unwrap();

    assert_eq!(result.new_properties, vec!["tags"]);
    assert_eq!(result.reverted_properties, vec!["title"]);

    let written = std::fs::read_to_string(&file).unwrap();
    // Original title preserved
    assert!(written.contains("title: My Doc\n"));
    // New property merged
    assert!(written.contains("tags: research\n"));
    // last_updated set
    assert!(written.contains("last_updated: 2026-04-02\n"));
    // New body applied
    assert!(written.contains("Brand new body content\n"));
    // tags appears before last_updated
    let tags_pos = written.find("tags:").unwrap();
    let lu_pos = written.find("last_updated:").unwrap();
    assert!(tags_pos < lu_pos);
}

#[test]
fn apply_closure_none_post_run_backward_compatible() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let original_hash = original_markdown
        .compute_hash(MdHashKind::Simple, &inline_hash_options());
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_hash,
    };

    let result =
        apply_inline_closure(&plan, "Updated body\n", &file, "2026-04-02", None).unwrap();

    assert!(result.new_properties.is_empty());
    assert!(result.reverted_properties.is_empty());

    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("last_updated: 2026-04-02\n"));
    assert!(written.contains("Updated body\n"));
}

#[test]
fn apply_closure_writes_cleaned_body_and_consistent_hash() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let opts = inline_hash_options();
    let original_hash = original_markdown.compute_hash(MdHashKind::Simple, &opts);
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_hash,
    };

    // Dirty provider body: header with no following blank line.
    let dirty_body = "# Generated Title\nParagraph without blank line\n";
    let result = apply_inline_closure(&plan, dirty_body, &file, "2026-04-02", None).unwrap();
    assert!(result.body_cleaned, "dirty body must report body_cleaned");

    let written = std::fs::read_to_string(&file).unwrap();
    // The cleaned body (blank line inserted) is the on-disk body, NOT the
    // raw provider body.
    assert!(
        written.contains("# Generated Title\n\nParagraph without blank line"),
        "on-disk body must be the cleaned body; got:\n{written}"
    );
    assert!(
        !written.contains("# Generated Title\nParagraph without blank line"),
        "raw dirty body must not survive to disk; got:\n{written}"
    );

    // The stored hash describes the FINAL document: `md hash --diff` would
    // exit 0 (neither frontmatter nor body reported as changed).
    let written_md: darkmatter::markdown::Markdown = written.clone().into();
    let stored = parse_inline_stored_hash(&written_md, &opts)
        .unwrap()
        .expect("written file should carry a stored hash");
    let comparison = written_md.compare_hash(&stored, &opts).unwrap();
    assert!(
        !comparison.frontmatter_changed && !comparison.body_changed,
        "stored hash must match the cleaned on-disk document; got:\n{written}"
    );
}

#[test]
fn apply_closure_body_cleaned_flag_reflects_cleanup() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let opts = inline_hash_options();

    let run = |body: &str| {
        let file = dir.path().join("flag.md");
        let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
        std::fs::write(&file, original).unwrap();
        let original_md: darkmatter::markdown::Markdown = original.to_string().into();
        let plan = InlineClosurePlan {
            original_document_text: original.to_string(),
            original_hash: original_md.compute_hash(MdHashKind::Simple, &opts),
        };
        apply_inline_closure(&plan, body, &file, "2026-04-02", None)
            .unwrap()
            .body_cleaned
    };

    // Dirty: header glued to paragraph → cleanup rewrites it.
    assert!(run("# Title\nParagraph\n"));
    // Already clean: cleanup is a no-op.
    assert!(!run("# Title\n\nParagraph\n"));
}

#[test]
fn apply_closure_writes_simple_hash() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let original_hash = original_markdown
        .compute_hash(MdHashKind::Simple, &inline_hash_options());
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_hash,
    };

    apply_inline_closure(&plan, "New body\n", &file, "2026-04-02", None).unwrap();

    let written = std::fs::read_to_string(&file).unwrap();
    let written_md: darkmatter::markdown::Markdown = written.clone().into();
    let hash_value = written_md
        .frontmatter()
        .as_map()
        .get("hash")
        .expect("hash property should be stamped");
    let hash_str = hash_value.as_str().expect("hash should be a string");

    let parts: Vec<&str> = hash_str.split('-').collect();
    assert_eq!(parts.len(), 2, "hash should have two 16-hex segments: {hash_str}");
    assert_eq!(parts[0].len(), 16);
    assert_eq!(parts[1].len(), 16);
    assert!(
        parts.iter().all(|p| p.bytes().all(|b| b.is_ascii_hexdigit())),
        "hash segments should be lowercase hex: {hash_str}"
    );

    // The managed keys are excluded from the fm segment, so the hash remains
    // stable across save round-trips.
    assert!(written.contains("last_updated: 2026-04-02\n"));
    assert!(written.contains("New body\n"));
}

#[test]
fn apply_closure_hash_is_self_referentially_stable() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let opts = inline_hash_options();
    let original_hash = original_markdown.compute_hash(MdHashKind::Simple, &opts);
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_hash,
    };

    apply_inline_closure(&plan, "New body\n", &file, "2026-04-02", None).unwrap();

    let written = std::fs::read_to_string(&file).unwrap();
    let written_md: darkmatter::markdown::Markdown = written.into();
    let stored = parse_inline_stored_hash(&written_md, &opts)
        .unwrap()
        .expect("written file should carry a stored hash");
    let computed = written_md.compute_hash(MdHashKind::Simple, &opts);

    assert_eq!(stored.kind, MdHashKind::Simple);
    assert_eq!(
        computed.to_stored_value(),
        stored.value,
        "re-computed hash must equal the stored value byte-for-byte"
    );
}

#[test]
fn apply_closure_downgrades_structured_hash_to_simple() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    // Valid structured stored hash, but the forced-Simple closure must
    // normalize it to the Simple shorthand on the next run.
    let original = concat!(
        "---\n",
        "prompt: test\n",
        "last_updated: 2026-01-01\n",
        "hash:\n",
        "  kind: structured\n",
        "  value: a000000000000000-b000000000000000-c000000000000000-d000000000000000\n",
        "---\n",
        "Old body\n",
    );
    std::fs::write(&file, original).unwrap();

    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let opts = inline_hash_options();
    let original_hash = original_markdown.compute_hash(MdHashKind::Simple, &opts);
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_hash,
    };

    apply_inline_closure(&plan, "New body\n", &file, "2026-04-02", None).unwrap();

    let written = std::fs::read_to_string(&file).unwrap();
    let written_md: darkmatter::markdown::Markdown = written.into();
    let stored = parse_inline_stored_hash(&written_md, &opts)
        .unwrap()
        .expect("written file should carry a stored hash");

    assert_eq!(
        stored.kind,
        MdHashKind::Simple,
        "non-Simple stored hash should be downgraded to Simple"
    );
    assert!(
        matches!(stored.value, darkmatter::markdown::hash::StoredHashValue::Flat(_)),
        "Simple hash should serialize as a flat shorthand string"
    );

    // Equivalent to `md hash --diff` exiting 0.
    let comparison = written_md.compare_hash(&stored, &opts).unwrap();
    assert!(
        !comparison.frontmatter_changed && !comparison.body_changed,
        "stored Simple hash should match the written document"
    );
}

#[test]
fn apply_closure_rejects_malformed_hash_and_preserves_file() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: test\nhash: not-a-hash\nlast_updated: 2026-01-01\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let original_hash = original_markdown
        .compute_hash(MdHashKind::Simple, &inline_hash_options());
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_hash,
    };

    let err = apply_inline_closure(&plan, "New body\n", &file, "2026-04-02", None).unwrap_err();

    assert!(
        matches!(err, CompositionError::InlineHashMalformed(_)),
        "expected InlineHashMalformed, got: {err}"
    );

    // The failure path runs before atomic_write, so the file must be unchanged.
    let on_disk = std::fs::read_to_string(&file).unwrap();
    assert_eq!(
        on_disk, original,
        "malformed hash must abort before writing to disk"
    );
}

// -- frontmatter-changed signal (Phase 3) --------------------------------

#[test]
fn apply_closure_reports_frontmatter_changed_when_new_key_added() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: original prompt\ntitle: My Doc\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let original_hash = original_markdown
        .compute_hash(MdHashKind::Simple, &inline_hash_options());
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_hash,
    };

    let mut post_run_fm = indexmap::IndexMap::new();
    post_run_fm.insert("prompt".to_string(), serde_json::json!("original prompt"));
    post_run_fm.insert("title".to_string(), serde_json::json!("My Doc"));
    post_run_fm.insert("tags".to_string(), serde_json::json!("research"));

    let result = apply_inline_closure(
        &plan,
        "Brand new body content\n",
        &file,
        "2026-04-02",
        Some(&post_run_fm),
    )
    .unwrap();

    assert!(result.frontmatter_changed, "adding a new key should flip frontmatter_changed");
}

#[test]
fn apply_closure_reports_frontmatter_unchanged_when_modified_key_reverted() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: original prompt\ntitle: My Doc\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let original_hash = original_markdown
        .compute_hash(MdHashKind::Simple, &inline_hash_options());
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_hash,
    };

    // Agent tried to change `title`, but the closure reverts it to the
    // original value. No other keys are added or removed.
    let mut post_run_fm = indexmap::IndexMap::new();
    post_run_fm.insert("prompt".to_string(), serde_json::json!("original prompt"));
    post_run_fm.insert("title".to_string(), serde_json::json!("Changed Title"));

    let result = apply_inline_closure(
        &plan,
        "Brand new body content\n",
        &file,
        "2026-04-02",
        Some(&post_run_fm),
    )
    .unwrap();

    assert!(
        !result.frontmatter_changed,
        "reverting a modified key should leave frontmatter_changed false"
    );
    assert_eq!(result.reverted_properties, vec!["title"]);
}

#[test]
fn apply_closure_hash_save_is_idempotent_when_stored_hash_matches() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_md: darkmatter::markdown::Markdown = original.to_string().into();
    let opts = inline_hash_options();
    let original_hash = original_md.compute_hash(MdHashKind::Simple, &opts);

    // Pre-compute and write a valid Simple hash that matches the document.
    let baseline = original_md.compute_hash(MdHashKind::Simple, &opts);
    let doc_with_hash = format!(
        "---\nprompt: test\nhash: {}\nlast_updated: 2026-01-01\n---\nOld body\n",
        baseline.flat_string().unwrap()
    );
    std::fs::write(&file, &doc_with_hash).unwrap();

    let plan = InlineClosurePlan {
        original_document_text: doc_with_hash.clone(),
        original_hash,
    };

    // A body-only change would normally bump last_updated, but if the
    // replacement body equals the original body the closure rejects before
    // writing. Directly verify the save decision is idempotent when the
    // stored hash already matches the document.
    let md: darkmatter::markdown::Markdown = doc_with_hash.clone().into();
    let parsed = parse_inline_stored_hash(&md, &opts).unwrap();
    let decision = md.plan_hash_save(parsed.as_ref(), &opts).unwrap();
    assert!(
        decision.new_stored.is_none(),
        "matching stored hash should not rewrite the file"
    );
    assert!(
        !decision.bump_last_updated,
        "matching stored hash should not bump last_updated"
    );

    // Also verify the closure rejection path leaves the file untouched.
    let err = apply_inline_closure(&plan, "Old body", &file, "2026-04-02", None).unwrap_err();
    assert!(matches!(err, CompositionError::InvalidInlineResponse(_)));
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        doc_with_hash,
        "unchanged-body rejection must not mutate the file"
    );
}

#[test]
fn apply_closure_is_deterministic_for_fixed_inputs() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_md: darkmatter::markdown::Markdown = original.to_string().into();
    let opts = inline_hash_options();
    let original_hash = original_md.compute_hash(MdHashKind::Simple, &opts);

    let run = || {
        let plan = InlineClosurePlan {
            original_document_text: original.to_string(),
            original_hash: original_hash.clone(),
        };
        apply_inline_closure(&plan, "New body\n", &file, "2026-04-02", None).unwrap();
        let bytes = std::fs::read_to_string(&file).unwrap();
        std::fs::write(&file, original).unwrap();
        bytes
    };

    let first = run();
    let second = run();
    assert_eq!(
        first, second,
        "two apply_inline_closure invocations with identical inputs must be byte-identical"
    );
}

#[test]
fn apply_closure_reports_frontmatter_unchanged_for_body_only_change() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();

    let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
    let original_hash = original_markdown
        .compute_hash(MdHashKind::Simple, &inline_hash_options());
    let plan = InlineClosurePlan {
        original_document_text: original.to_string(),
        original_hash,
    };

    let result =
        apply_inline_closure(&plan, "Updated body\n", &file, "2026-04-02", None).unwrap();

    assert!(
        !result.frontmatter_changed,
        "a body-only change should leave frontmatter_changed false"
    );
}
