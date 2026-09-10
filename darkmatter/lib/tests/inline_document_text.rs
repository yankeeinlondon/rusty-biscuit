use darkmatter::markdown::hash::{
    FrontmatterDeltaEntry, restore_properties_text,
};
use darkmatter::markdown::{Markdown, MarkdownError};

const SHIPPED_DIALECT_DOCUMENTS: &[(&str, &str)] = &[
    (
        "claudine",
        include_str!("../../tests/fixtures/schema-triggers/docs/claudine.md"),
    ),
    (
        "inline-compose",
        include_str!("../../tests/fixtures/schema-triggers/docs/inline-compose.md"),
    ),
    (
        "plain",
        include_str!("../../tests/fixtures/schema-triggers/docs/plain.md"),
    ),
    (
        "sequence",
        include_str!("../../tests/fixtures/schema-triggers/docs/sequence.md"),
    ),
];

#[test]
fn shipped_dialect_document_corpus_is_passively_readable() {
    for (name, document) in SHIPPED_DIALECT_DOCUMENTS {
        let restored = restore_properties_text(document, document, &[])
            .unwrap_or_else(|error| panic!("shipped {name} document failed: {error}"));
        assert_eq!(restored.text, *document, "shipped {name} document changed");
        assert!(restored.restored_properties.is_empty());
        assert!(restored.frontmatter_delta.is_empty());
    }
}

#[test]
fn shipped_inline_compose_document_runs_through_the_public_restore_path() {
    let snapshot = SHIPPED_DIALECT_DOCUMENTS
        .iter()
        .find_map(|(name, document)| (*name == "inline-compose").then_some(*document))
        .unwrap();
    let current = snapshot
        .replace("prompt: Explain the result.", "prompt: agent changed this")
        .replace(
            "Inline compose dialect fixture.",
            "Agent-authored inline result.",
        );

    let restored = restore_properties_text(&current, snapshot, &["prompt"]).unwrap();

    assert_eq!(restored.restored_properties, ["prompt"]);
    assert_eq!(
        restored.text,
        snapshot.replace(
            "Inline compose dialect fixture.",
            "Agent-authored inline result."
        )
    );
    assert!(restored.frontmatter_delta.is_empty());
}

#[test]
fn public_restore_api_preserves_the_candidate_body_and_is_a_fixed_point() {
    let snapshot = concat!(
        "---\r\n",
        "prompt: |-\r\n",
        "    Preserve me.  \r\n",
        r"source: C:\Users\Ken Snyder\input.md",
        "\r\n---\r\n",
        "Original body.\r\n",
    );
    let candidate = concat!(
        "---\r\n",
        "prompt: overwritten\r\n",
        r"source: C:\Users\Ken Snyder\output.md",
        "\r\nproducts:\r\n",
        "    - UDM Pro\r\n",
        "---\r\n",
        "Agent-authored body.  \r\n",
    );

    let first = restore_properties_text(candidate, snapshot, &["prompt"]).unwrap();
    assert_eq!(first.restored_properties, ["prompt"]);
    assert!(first.text.ends_with("---\r\nAgent-authored body.  \r\n"));
    assert_eq!(
        first.frontmatter_delta.entries,
        [
            FrontmatterDeltaEntry::Replacement {
                property: "source".into(),
                previous_value: serde_json::json!(r"C:\Users\Ken Snyder\input.md"),
                value: serde_json::json!(r"C:\Users\Ken Snyder\output.md"),
            },
            FrontmatterDeltaEntry::Addition {
                property: "products".into(),
                value: serde_json::json!(["UDM Pro"]),
            },
        ]
    );

    let reparsed: Markdown = first.text.clone().into();
    assert_eq!(
        reparsed.fm_get::<String>("prompt").unwrap().as_deref(),
        Some("Preserve me.  ")
    );
    assert_eq!(reparsed.content(), "Agent-authored body.  ");

    let second = restore_properties_text(&first.text, snapshot, &["prompt"]).unwrap();
    assert_eq!(second.text, first.text);
    assert!(second.restored_properties.is_empty());
    assert_eq!(second.frontmatter_delta, first.frontmatter_delta);
}

#[test]
fn public_restore_api_returns_typed_errors_for_ambiguous_input() {
    let error = restore_properties_text(
        "---\nprompt: one\n'prompt': two\n---\nBody\n",
        "---\nprompt: original\n---\nOld body\n",
        &["prompt"],
    )
    .unwrap_err();

    assert!(matches!(error, MarkdownError::FrontmatterTextEdit { .. }));
}
