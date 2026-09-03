use super::*;
use tempfile::TempDir;

const AC1_ORIGINAL_DOCUMENT: &str = concat!(
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

fn plan(original: &str, allowed: &[&str]) -> InlineClosurePlan {
    let markdown: darkmatter::markdown::Markdown = original.to_string().into();
    InlineClosurePlan {
        original_document_text: original.to_string(),
        original_hash: markdown.compute_hash(MdHashKind::Simple, &inline_hash_options()),
        response_frontmatter: allowed.iter().map(|key| (*key).to_string()).collect(),
    }
}

#[test]
fn replacement_parts_returns_trimmed_body_without_frontmatter() {
    let parts = extract_replacement_parts("  Hello world  ").unwrap();
    assert_eq!(parts.body, "Hello world");
    assert!(parts.frontmatter.is_none());
}

#[test]
fn replacement_parts_preserves_unclosed_delimiter_as_body() {
    let parts = extract_replacement_parts("---\ntitle: test\nNo closing fence\n").unwrap();
    assert_eq!(parts.body, "---\ntitle: test\nNo closing fence");
    assert!(parts.frontmatter.is_none());
}

#[test]
fn replacement_parts_treats_whitespace_prefixed_delimiter_as_body() {
    let parts = extract_replacement_parts("  ---\nnot: metadata\n---\nBody\n").unwrap();
    assert_eq!(parts.body, "---\nnot: metadata\n---\nBody");
    assert!(parts.frontmatter.is_none());
}

#[test]
fn replacement_parts_parses_mapping_and_source_lines() {
    let parts = extract_replacement_parts(
        "---\naccess_points:\n  - Office\n'generated:by': inventory\n---\nNew body\n",
    )
    .unwrap();
    let frontmatter = parts.frontmatter.unwrap();
    assert_eq!(frontmatter["access_points"].line, 2);
    assert_eq!(frontmatter["generated:by"].line, 4);
    assert_eq!(parts.body, "New body");
}

#[test]
fn exact_response_frontmatter_allows_blank_lines_before_an_unauthorized_key() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\nprompt: test\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();
    let replacement = extract_replacement_parts("---\n\n\nundeclared: value\n---\nNew body\n")
        .unwrap();

    let result = apply_inline_closure(&plan(original, &[]), &replacement, &file, "2026-09-01")
        .unwrap();

    assert_eq!(
        result.ignored_properties,
        [InlinePropertyNotice {
            key: "undeclared".into(),
            line: 4,
        }]
    );
    assert!(std::fs::read_to_string(file).unwrap().ends_with("---\nNew body\n"));
}

#[test]
fn unauthorized_property_warning_line_counts_every_response_protocol_line() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\nprompt: test\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();
    let replacement = extract_replacement_parts(
        "---\n\n# provider note\n\nundeclared: value\n---\nNew body\n",
    )
    .unwrap();

    let result = apply_inline_closure(&plan(original, &[]), &replacement, &file, "2026-09-01")
        .unwrap();

    assert_eq!(
        result.ignored_properties,
        [InlinePropertyNotice {
            key: "undeclared".into(),
            line: 5,
        }]
    );
}

#[test]
fn replacement_parts_rejects_invalid_frontmatter_shapes() {
    for response in [
        "---\n[one, two]\n---\nBody\n",
        "---\nkey: [\n---\nBody\n",
        "---\nkey: one\nkey: two\n---\nBody\n",
        "---\nkey: one\n---\n",
    ] {
        assert!(
            matches!(
                extract_replacement_parts(response),
                Err(
                    CompositionError::InvalidInlineResponse(_)
                        | CompositionError::InlineResponseFrontmatterYaml { .. }
                )
            ),
            "response should be invalid: {response:?}"
        );
    }
}

#[test]
fn rewrite_replaces_whole_existing_nodes_and_preserves_adjacent_comments() {
    let original = concat!(
        "---\n",
        "prompt: |-\n",
        "  Preserve this\n",
        "access_points:\n",
        "  old: value\n",
        "# keep this comment\n",
        "owner: human\n",
        "last_updated: '2026-01-01'\n",
        "---\n",
        "Old body\n",
    );
    let mut harvested = IndexMap::new();
    harvested.insert(
        "access_points".into(),
        "access_points:\n- Office\n- Studio\n".into(),
    );
    let rewritten = rewrite_inline_document(
        original,
        "New body\n",
        &harvested,
        &["access_points".into()],
    )
    .unwrap();
    assert!(rewritten.contains("access_points:\n- Office\n- Studio\n# keep this comment\n"));
    assert!(rewritten.contains("prompt: |-\n  Preserve this\n"));
    assert!(rewritten.contains("owner: human\n"));
    assert!(rewritten.ends_with("---\nNew body\n"));
}

#[test]
fn rewrite_inserts_missing_nodes_in_declaration_order_before_last_updated() {
    let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld\n";
    let mut harvested = IndexMap::new();
    harvested.insert("generated:by".into(), "'generated:by': test\n".into());
    harvested.insert("access_points".into(), "access_points:\n- Office\n".into());
    let rewritten = rewrite_inline_document(
        original,
        "New\n",
        &harvested,
        &["access_points".into(), "generated:by".into()],
    )
    .unwrap();
    let access = rewritten.find("access_points:").unwrap();
    let generated = rewritten.find("'generated:by':").unwrap();
    let updated = rewritten.find("last_updated:").unwrap();
    assert!(access < generated && generated < updated);
    let parsed: darkmatter::markdown::Markdown = rewritten.into();
    assert_eq!(parsed.frontmatter().as_map()["generated:by"], "test");
}

#[test]
fn apply_closure_harvests_only_authorized_properties() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = concat!(
        "---\n",
        "prompt: test\n",
        "access_points:\n",
        "  - Old\n",
        "last_updated: 2026-01-01\n",
        "---\n",
        "Old body\n",
    );
    std::fs::write(&file, original).unwrap();
    let replacement = extract_replacement_parts(concat!(
        "---\n",
        "access_points: [Office, Studio]\n",
        "generated_by: inventory\n",
        "title: rejected\n",
        "hash: rejected\n",
        "last_updated: rejected\n",
        "---\n",
        "New body\n",
    ))
    .unwrap();
    let result = apply_inline_closure(
        &plan(original, &["access_points", "generated_by", "missing"]),
        &replacement,
        &file,
        "2026-09-01",
    )
    .unwrap();
    assert_eq!(result.inserted_properties, ["generated_by"]);
    assert_eq!(result.refreshed_properties, ["access_points"]);
    assert_eq!(result.missing_properties, ["missing"]);
    assert_eq!(
        result.ignored_properties,
        [InlinePropertyNotice {
            key: "title".into(),
            line: 4,
        }]
    );
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("generated_by: inventory\n"));
    assert!(!written.contains("title: rejected"));
    assert!(!written.contains("hash: rejected"));
}

#[test]
fn removing_response_authorization_leaves_generated_value_untouched_on_later_run() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = concat!(
        "---\n",
        "prompt: test\n",
        "response_frontmatter: [generated_by]\n",
        "last_updated: 2026-01-01\n",
        "---\n",
        "Old body\n",
    );
    std::fs::write(&file, original).unwrap();

    let first_replacement = extract_replacement_parts(
        "---\ngenerated_by: first-run\n---\nFirst generated body\n",
    )
    .unwrap();
    apply_inline_closure(
        &plan(original, &["generated_by"]),
        &first_replacement,
        &file,
        "2026-09-01",
    )
    .unwrap();

    let without_authorization = std::fs::read_to_string(&file)
        .unwrap()
        .replace("response_frontmatter: [generated_by]\n", "");
    std::fs::write(&file, &without_authorization).unwrap();
    let second_replacement = extract_replacement_parts(
        "---\ngenerated_by: second-run\n---\nSecond generated body\n",
    )
    .unwrap();
    let result = apply_inline_closure(
        &plan(&without_authorization, &[]),
        &second_replacement,
        &file,
        "2026-09-01",
    )
    .unwrap();

    assert_eq!(
        result.ignored_properties,
        [InlinePropertyNotice {
            key: "generated_by".into(),
            line: 2,
        }]
    );
    let written = std::fs::read_to_string(file).unwrap();
    assert!(written.contains("generated_by: first-run\n"));
    assert!(!written.contains("generated_by: second-run\n"));
    assert!(!written.contains("response_frontmatter:"));
    assert!(written.contains("Second generated body\n"));
}

#[test]
fn apply_closure_preserves_authored_frontmatter_bytes_and_stamps_clean_hash() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, AC1_ORIGINAL_DOCUMENT).unwrap();
    let replacement = extract_replacement_parts("New body\n").unwrap();
    apply_inline_closure(
        &plan(AC1_ORIGINAL_DOCUMENT, &[]),
        &replacement,
        &file,
        "2026-09-01",
    )
    .unwrap();
    let written = std::fs::read_to_string(&file).unwrap();
    let markdown: darkmatter::markdown::Markdown = written.clone().into();
    let options = inline_hash_options();
    let stored = parse_inline_stored_hash(&markdown, &options)
        .unwrap()
        .unwrap();
    let darkmatter::markdown::hash::StoredHashValue::Flat(hash_value) = &stored.value else {
        panic!("inline closure must downgrade the managed hash to Simple")
    };
    let expected = format!(
        concat!(
            "---\n",
            "prompt: |-\n",
            "    Keep four spaces  \n",
            "\n",
            "    and literal \\\"quotes\\\"\n",
            "hash: {}\n",
            "last_updated: '2026-09-01'\n",
            "---\n",
            "New body\n",
        ),
        hash_value
    );
    assert_eq!(written, expected);
    assert!(!written.contains("prompt: \""));
    assert_eq!(stored.kind, MdHashKind::Simple);
    let comparison = markdown.compare_hash(&stored, &options).unwrap();
    assert!(!comparison.frontmatter_changed && !comparison.body_changed);
}

#[test]
fn apply_closure_reports_value_drift_but_silences_reformat_only_drift() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\nprompt: |-\n  First\n  Second\ntitle: Mine\n---\nOld body\n";
    std::fs::write(
        &file,
        "---\nprompt: \"First\\nSecond\"\ntitle: Theirs\n---\nChanged body\n",
    )
    .unwrap();
    let replacement = extract_replacement_parts("New body\n").unwrap();
    let result = apply_inline_closure(
        &plan(original, &[]),
        &replacement,
        &file,
        "2026-09-01",
    )
    .unwrap();
    assert_eq!(result.restored_frontmatter_properties, ["title"]);
    assert!(!result.unclassified_frontmatter_drift_restored);
    assert!(result.body_drift_restored);
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("prompt: |-\n  First\n  Second\n"));
    assert!(written.contains("title: Mine\n"));
    assert!(written.contains("New body"));
}

#[test]
fn apply_closure_reports_added_and_removed_frontmatter_properties() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = concat!(
        "---\n",
        "prompt: test\n",
        "title: Mine\n",
        "owner: Human\n",
        "---\n",
        "Old body\n",
    );
    std::fs::write(
        &file,
        "---\nprompt: test\ntitle: Mine\nadded: New\n---\nOld body\n",
    )
    .unwrap();

    let replacement = extract_replacement_parts("New body\n").unwrap();
    let result = apply_inline_closure(
        &plan(original, &[]),
        &replacement,
        &file,
        "2026-09-01",
    )
    .unwrap();

    assert_eq!(
        result.restored_frontmatter_properties,
        ["owner", "added"]
    );
    assert!(!result.unclassified_frontmatter_drift_restored);
    assert!(!result.body_drift_restored);
}

#[test]
fn apply_closure_reports_unclassified_frontmatter_drift_without_body_drift() {
    let original = "---\nprompt: test\ntitle: Mine\n---\nOld body\n";
    for current in [
        "---\nprompt: test\ntitle: [\n---\nOld body\n",
        "---\n- prompt\n- title\n---\nOld body\n",
        "--\nprompt: test\ntitle: Mine\n---\nOld body\n",
        "---\nprompt: test\ntitle: Mine\n--\nOld body\n",
    ] {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("doc.md");
        std::fs::write(&file, current).unwrap();

        let replacement = extract_replacement_parts("New body\n").unwrap();
        let result = apply_inline_closure(
            &plan(original, &[]),
            &replacement,
            &file,
            "2026-09-01",
        )
        .unwrap();

        assert!(
            result.unclassified_frontmatter_drift_restored,
            "frontmatter drift should be reported for {current:?}"
        );
        assert!(result.restored_frontmatter_properties.is_empty());
        assert!(
            !result.body_drift_restored,
            "unchanged body should not be reported as drift for {current:?}"
        );
    }
}

#[test]
fn apply_closure_rejects_unchanged_body_even_with_authorized_metadata() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\nprompt: test\nresponse_frontmatter: [generated_by]\n---\nOriginal body\n";
    std::fs::write(&file, original).unwrap();
    let replacement = extract_replacement_parts(
        "---\ngenerated_by: inventory\n---\nOriginal body\n",
    )
    .unwrap();
    let error = apply_inline_closure(
        &plan(original, &["generated_by"]),
        &replacement,
        &file,
        "2026-09-01",
    )
    .unwrap_err();
    assert!(matches!(error, CompositionError::InvalidInlineResponse(_)));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
}

#[test]
fn replacement_parts_rejects_empty_input() {
    for response in ["", "   "] {
        assert!(matches!(
            extract_replacement_parts(response),
            Err(CompositionError::InvalidInlineResponse(_))
        ));
    }
}

#[test]
fn rewrite_preserves_crlf_and_authored_schema() {
    let original = concat!(
        "---\r\n",
        "$schema:\r\n",
        "  title: 'string(required)'\r\n",
        "prompt: |-\r\n",
        "  Keep this formatting\r\n",
        "title: Authored\r\n",
        "last_updated: \"2026-01-01\"\r\n",
        "---\r\n",
        "Old body\r\n",
    );
    let rewritten = rewrite_inline_document(original, "New body\r\n", &IndexMap::new(), &[])
        .unwrap();
    assert!(rewritten.contains("$schema:\r\n  title: 'string(required)'\r\n"));
    assert!(rewritten.contains("prompt: |-\r\n  Keep this formatting\r\n"));
    assert!(rewritten.contains("title: Authored\r\n"));
    assert!(rewritten.ends_with("---\r\nNew body\r\n"));
}

#[test]
fn property_serialization_round_trips_scalar_sequence_mapping_and_significant_key() {
    for (key, value) in [
        ("title", serde_json::json!("key: value # intact")),
        ("count", serde_json::json!(42)),
        ("draft", serde_json::json!(true)),
        ("removed", serde_json::Value::Null),
        ("tags", serde_json::json!(["rust", "markdown"])),
        ("meta", serde_json::json!({"version": "1.0"})),
        ("generated:by", serde_json::json!("stub")),
    ] {
        let fragment = serialize_frontmatter_property(key, &value).unwrap();
        let parsed: serde_json::Value = biscuit_file::serde_yaml_ng::from_str(&fragment).unwrap();
        assert_eq!(parsed.as_object().unwrap().get(key), Some(&value));
    }
}

#[test]
fn apply_closure_rejects_empty_body() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\nprompt: test\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();
    let replacement = InlineReplacementParts {
        body: "  ".into(),
        frontmatter: None,
    };
    assert!(matches!(
        apply_inline_closure(&plan(original, &[]), &replacement, &file, "2026-09-01"),
        Err(CompositionError::InvalidInlineResponse(_))
    ));
    assert_eq!(std::fs::read_to_string(file).unwrap(), original);
}

#[test]
fn apply_closure_cleans_body_and_hashes_the_cleaned_text() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();
    let replacement = extract_replacement_parts(
        "# Generated Title\nParagraph without blank line\n",
    )
    .unwrap();
    let result = apply_inline_closure(
        &plan(original, &[]),
        &replacement,
        &file,
        "2026-09-01",
    )
    .unwrap();
    assert!(result.body_cleaned);
    let written = std::fs::read_to_string(file).unwrap();
    assert!(written.contains("# Generated Title\n\nParagraph without blank line"));
    let markdown: darkmatter::markdown::Markdown = written.into();
    let options = inline_hash_options();
    let stored = parse_inline_stored_hash(&markdown, &options).unwrap().unwrap();
    let comparison = markdown.compare_hash(&stored, &options).unwrap();
    assert!(!comparison.frontmatter_changed && !comparison.body_changed);
}

#[test]
fn apply_closure_preserves_quoted_last_updated_style() {
    for (label, quote, expected) in [
        ("double", '"', "last_updated: \"2026-09-01\""),
        ("single", '\'', "last_updated: '2026-09-01'"),
    ] {
        let dir = TempDir::new().unwrap();
        // A quote character is not a legal file name on Windows.
        let file = dir.path().join(format!("quoted-{label}.md"));
        let original = format!(
            "---\nprompt: test\nlast_updated: {quote}2026-01-01{quote}\n---\nOld body\n"
        );
        std::fs::write(&file, &original).unwrap();
        let replacement = extract_replacement_parts("New body\n").unwrap();
        apply_inline_closure(
            &plan(&original, &[]),
            &replacement,
            &file,
            "2026-09-01",
        )
        .unwrap();
        assert!(std::fs::read_to_string(file).unwrap().contains(expected));
    }
}

#[test]
fn apply_closure_preserves_crlf_frontmatter_through_hash_stamp() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\r\nprompt: |-\r\n  Keep\r\nlast_updated: 2026-01-01\r\n---\r\nOld body\r\n";
    std::fs::write(&file, original).unwrap();
    let replacement = extract_replacement_parts("New body\r\n").unwrap();
    apply_inline_closure(&plan(original, &[]), &replacement, &file, "2026-09-01").unwrap();
    let written = std::fs::read_to_string(file).unwrap();
    assert!(written.contains("prompt: |-\r\n  Keep\r\n"));
    assert!(written.contains("last_updated: 2026-09-01\r\n"));
}

#[test]
fn apply_closure_rejects_malformed_hash_without_mutation() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\nprompt: test\nhash: not-a-hash\nlast_updated: 2026-01-01\n---\nOld body\n";
    std::fs::write(&file, original).unwrap();
    let replacement = extract_replacement_parts("New body\n").unwrap();
    let error = apply_inline_closure(
        &plan(original, &[]),
        &replacement,
        &file,
        "2026-09-01",
    )
    .unwrap_err();
    assert!(matches!(error, CompositionError::InlineHashMalformed(_)));
    assert_eq!(std::fs::read_to_string(file).unwrap(), original);
}

#[test]
fn apply_closure_is_deterministic_for_fixed_inputs() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
    let replacement = extract_replacement_parts("New body\n").unwrap();
    let run = || {
        std::fs::write(&file, original).unwrap();
        apply_inline_closure(
            &plan(original, &[]),
            &replacement,
            &file,
            "2026-09-01",
        )
        .unwrap();
        std::fs::read_to_string(&file).unwrap()
    };
    assert_eq!(run(), run());
}

#[test]
fn apply_closure_second_identical_run_is_byte_idempotent() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, AC1_ORIGINAL_DOCUMENT).unwrap();
    let replacement = extract_replacement_parts("New body\n").unwrap();
    apply_inline_closure(
        &plan(AC1_ORIGINAL_DOCUMENT, &[]),
        &replacement,
        &file,
        "2026-09-01",
    )
    .unwrap();
    let first = std::fs::read_to_string(&file).unwrap();
    let error = apply_inline_closure(
        &plan(&first, &[]),
        &replacement,
        &file,
        "2026-09-01",
    )
    .unwrap_err();
    assert!(matches!(error, CompositionError::InvalidInlineResponse(_)));
    assert_eq!(std::fs::read_to_string(file).unwrap(), first);
}

#[test]
fn frontmatter_changed_reports_generated_nodes_but_not_body_only_changes() {
    let dir = TempDir::new().unwrap();
    let body_only_file = dir.path().join("body.md");
    let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
    std::fs::write(&body_only_file, original).unwrap();
    let body = extract_replacement_parts("New body\n").unwrap();
    let body_result = apply_inline_closure(
        &plan(original, &[]),
        &body,
        &body_only_file,
        "2026-09-01",
    )
    .unwrap();
    assert!(!body_result.frontmatter_changed);

    let generated_file = dir.path().join("generated.md");
    std::fs::write(&generated_file, original).unwrap();
    let generated = extract_replacement_parts(
        "---\ngenerated_by: stub\n---\nNew body\n",
    )
    .unwrap();
    let generated_result = apply_inline_closure(
        &plan(original, &["generated_by"]),
        &generated,
        &generated_file,
        "2026-09-01",
    )
    .unwrap();
    assert!(generated_result.frontmatter_changed);
}
