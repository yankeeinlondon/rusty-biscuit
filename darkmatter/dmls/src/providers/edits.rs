//! Shared `WorkspaceEdit` assembly for the editing providers (rename, code
//! actions).
//!
//! Editors differ in what a [`WorkspaceEdit`] may carry (R-7): `documentChanges`
//! with resource operations and change annotations are gated on the client
//! profile. [`EditBuilder`] accumulates per-document text edits plus create-file
//! operations and lowers them into the richest form the profile allows — falling
//! back to plain `changes` (text edits only) when resource operations are
//! unsupported.

use std::collections::BTreeMap;

use lsp_types::{
    CreateFile, DocumentChangeOperation, DocumentChanges, OneOf,
    OptionalVersionedTextDocumentIdentifier, ResourceOp, TextDocumentEdit, TextEdit, Uri,
    WorkspaceEdit,
};

use crate::capabilities::ClientProfile;

/// Accumulates text edits (grouped by document) and create-file operations,
/// then lowers to a [`WorkspaceEdit`] honoring the client profile.
#[derive(Default)]
pub struct EditBuilder {
    /// URI string → (URI, edits). A `BTreeMap` keeps document groups in a
    /// deterministic order regardless of insertion sequence.
    edits: BTreeMap<String, (Uri, Vec<TextEdit>)>,
    creates: Vec<CreateFile>,
}

impl EditBuilder {
    /// A fresh, empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether nothing has been accumulated.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty() && self.creates.is_empty()
    }

    /// Records a text edit against `uri`.
    pub fn edit(&mut self, uri: Uri, edit: TextEdit) {
        self.edits
            .entry(uri.as_str().to_string())
            .or_insert_with(|| (uri, Vec::new()))
            .1
            .push(edit);
    }

    /// Records a create-file operation (only surfaced when the profile supports
    /// resource operations).
    pub fn create_file(&mut self, uri: Uri) {
        self.creates.push(CreateFile {
            uri,
            options: None,
            annotation_id: None,
        });
    }

    /// Lowers to a [`WorkspaceEdit`].
    ///
    /// With resource-operation support, `documentChanges` carries every create
    /// before the edits (so a new file exists before it is written). Without it,
    /// only `changes` (text edits) are emitted — callers must gate create-file
    /// actions on [`ClientProfile::supports_resource_operations`] so a
    /// create-bearing edit is never silently dropped.
    // `WorkspaceEdit::changes` is keyed by `Uri`, which clippy flags as a
    // mutable key type; the URIs here are never mutated after insertion.
    #[allow(clippy::mutable_key_type)]
    pub fn build(self, profile: &ClientProfile) -> WorkspaceEdit {
        if profile.supports_resource_operations {
            let mut ops: Vec<DocumentChangeOperation> = Vec::new();
            for create in self.creates {
                ops.push(DocumentChangeOperation::Op(ResourceOp::Create(create)));
            }
            for (_, (uri, edits)) in self.edits {
                ops.push(DocumentChangeOperation::Edit(TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier { uri, version: None },
                    edits: edits.into_iter().map(OneOf::Left).collect(),
                }));
            }
            WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(ops)),
                ..Default::default()
            }
        } else {
            let changes = self
                .edits
                .into_iter()
                .map(|(_, (uri, edits))| (uri, edits))
                .collect();
            WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }
        }
    }
}
