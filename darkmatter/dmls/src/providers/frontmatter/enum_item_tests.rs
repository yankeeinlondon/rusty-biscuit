//! Completion of one item of an array whose item type is an enum: the
//! sandboxing schema's `os_support: os[]` with `os: enum(...)` declared under
//! `types:` in a separate schema file, plus the inline `enum(...)[]` spelling.
//!
//! The schema here is static; a dynamic `enum({{ … }})` list resolves to the
//! same concrete member list and reaches completion through the same path.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use lsp_types::{CompletionItemKind, InitializeParams, Uri};

use super::completion;
use crate::capabilities::ClientProfile;
use crate::config::DmlsConfig;
use crate::graph::WorkspaceGraph;
use crate::overlay::OverlayState;
use crate::providers::DocumentContext;
use crate::source_map::{PositionEncoding, SourceMap};
use crate::workspace::file_path_to_uri;

/// The shipped sandboxing schema's shape with its `os` enum written out.
const SANDBOX_SCHEMA: &str = "kind: schema\n\
description: sandbox-shaped schema with a static os enum\n\
$schema:\n    \
    schema_revision: literal(5; required)\n    \
    has_native_support: boolean(required) -> whether the provider sandboxes natively\n    \
    os_support: os[](required) -> which operating systems support it\n\
types:\n    \
    os: enum(macOS, Linux, Windows; required) -> a supported operating system\n";

/// Writes [`SANDBOX_SCHEMA`] and a provider document holding `document` into a
/// fresh repository folder, then runs `f` with the document's context.
fn with_provider<R>(document: &str, f: impl FnOnce(&DocumentContext) -> R) -> R {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::write(root.join("sandboxing.yaml"), SANDBOX_SCHEMA).expect("schema");
    let path = root.join("provider.md");
    std::fs::write(&path, document).expect("document");
    with_document(&root, &path, document, f)
}

fn with_document<R>(root: &Path, path: &Path, text: &str, f: impl FnOnce(&DocumentContext) -> R) -> R {
    let uri: Uri = file_path_to_uri(path).expect("document uri");
    let config = DmlsConfig::default();
    let roots = [root.to_path_buf()];
    let resolution = crate::context::test_support::resolution_in_repository(root, path);
    let overlay = OverlayState::default().for_document(&uri, text, path, &config, &roots, &resolution);
    let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
    let graph = WorkspaceGraph::build(&BTreeMap::new(), 1, &crate::context::test_support::workspace_contexts());
    let profile = ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
    let ctx = DocumentContext {
        uri: &uri,
        path,
        text,
        source_map: &source_map,
        graph: &graph,
        doc_id: None,
        config: &config,
        profile: &profile,
        overlay: overlay.as_ref(),
        resolution: &resolution,
    };
    f(&ctx)
}

/// The document with `os_support` written as `os_line`, the cursor's offset
/// at the end of `cursor_after` (which must occur in `os_line`).
fn provider_document(os_line: &str, cursor_after: &str) -> (String, usize) {
    let text = format!(
        "---\n$schema: ./sandboxing.yaml\nschema_revision: 5\nhas_native_support: true\n{os_line}\n---\n\n# Provider\n"
    );
    let line_start = text.find(os_line).expect("os line in document");
    let offset = line_start + os_line.find(cursor_after).expect("cursor text in os line") + cursor_after.len();
    (text, offset)
}

/// The labels of the enum-member items offered at `offset`.
fn member_labels(ctx: &DocumentContext, offset: usize) -> Vec<String> {
    completion(ctx, offset)
        .into_iter()
        .filter(|item| item.kind == Some(CompletionItemKind::ENUM_MEMBER))
        .map(|item| item.label)
        .collect()
}

#[test]
fn empty_block_item_of_named_enum_array_offers_every_member() {
    let (text, offset) = provider_document("os_support:\n  - ", "  - ");
    with_provider(&text, |ctx| {
        assert_eq!(member_labels(ctx, offset), ["macOS", "Linux", "Windows"]);
    });
}

#[test]
fn bare_dash_item_of_named_enum_array_offers_every_member() {
    // A `-` with no space yet: the item text replaces from after the marker.
    let (text, offset) = provider_document("os_support:\n  -", "  -");
    with_provider(&text, |ctx| {
        assert_eq!(member_labels(ctx, offset), ["macOS", "Linux", "Windows"]);
    });
}

#[test]
fn partial_block_item_filters_members_and_replaces_the_partial() {
    let (text, offset) = provider_document("os_support:\n  - macOS\n  - Li", "  - Li");
    with_provider(&text, |ctx| {
        let items: Vec<_> = completion(ctx, offset)
            .into_iter()
            .filter(|item| item.kind == Some(CompletionItemKind::ENUM_MEMBER))
            .collect();
        let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, ["Linux"]);
        let Some(lsp_types::CompletionTextEdit::Edit(edit)) = &items[0].text_edit else {
            panic!("a text edit replaces the partial item: {:?}", items[0]);
        };
        assert_eq!(edit.new_text, "Linux");
        assert_eq!(edit.range.start.character, 4, "the edit starts after `  - `");
        assert_eq!(edit.range.end.character, 6, "the edit ends at the cursor");
    });
}

#[test]
fn flow_array_elements_offer_members() {
    // Brackets are closed as an editor auto-closes them; an unclosed `[`
    // leaves the frontmatter unparseable, which offers no completion at all.
    for (os_line, cursor_after) in [
        ("os_support: []", "["),
        ("os_support: [macOS, ]", "[macOS, "),
        ("os_support: [macOS, W]", "[macOS, W"),
    ] {
        let (text, offset) = provider_document(os_line, cursor_after);
        with_provider(&text, |ctx| {
            let labels = member_labels(ctx, offset);
            let expected: &[&str] = if cursor_after.ends_with('W') {
                &["Windows"]
            } else {
                &["macOS", "Linux", "Windows"]
            };
            assert_eq!(labels, expected, "members offered in `{os_line}`");
        });
    }
}

#[test]
fn inline_enum_array_item_offers_members() {
    // The same gap without a named type: `enum(...)[]` declared inline.
    let text = "---\n$schema:\n  modes: enum(fit, fill)[]\nmodes:\n  - \n---\n\nbody\n";
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    let path = root.join("doc.md");
    std::fs::write(&path, text).expect("document");
    let offset = text.find("  - \n").expect("item line") + "  - ".len();
    with_document(&root, &path, text, |ctx| {
        assert_eq!(member_labels(ctx, offset), ["fit", "fill"]);
    });
}

#[test]
fn scalar_enum_value_keeps_offering_members() {
    // Control: a non-array use of the same named type already completes.
    let schema = SANDBOX_SCHEMA.replace("os_support: os[](required)", "os_support: os(required)");
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::write(root.join("sandboxing.yaml"), schema).expect("schema");
    let (text, offset) = provider_document("os_support: ", "os_support: ");
    let path = root.join("provider.md");
    std::fs::write(&path, &text).expect("document");
    with_document(&root, &path, &text, |ctx| {
        assert_eq!(member_labels(ctx, offset), ["macOS", "Linux", "Windows"]);
    });
}

