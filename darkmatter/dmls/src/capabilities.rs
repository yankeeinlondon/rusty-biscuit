//! Server capability advertisement, position-encoding negotiation, and the
//! per-client capability profile.
//!
//! The capability advertisement grows phase by phase: only capabilities that
//! are actually implemented are ever advertised. Providers consult
//! [`ClientProfile`] instead of sniffing raw client capabilities ad hoc; the
//! gate fields and their per-editor reality come from the R-7 research
//! (`darkmatter/dmls/design/research/r7-editor-capability-matrix.md`).

use lsp_types::{
    ClientCapabilities, CodeActionProviderCapability, CompletionOptions, DocumentLinkOptions,
    FileOperationFilter, FileOperationPattern, FileOperationPatternKind,
    FileOperationRegistrationOptions, FoldingRangeProviderCapability, HoverProviderCapability,
    InitializeParams, OneOf, PositionEncodingKind, RenameOptions, SaveOptions,
    SemanticTokensFullOptions, SemanticTokensOptions, SemanticTokensServerCapabilities,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    WorkspaceFileOperationsServerCapabilities, WorkspaceServerCapabilities,
    WorkDoneProgressOptions,
};

use crate::source_map::PositionEncoding;

/// Selects the position encoding for a client.
///
/// DMLS supports UTF-8 and UTF-16: the first client-offered encoding DMLS
/// supports wins; UTF-16 is forced when only UTF-16 is offered, when the
/// offer list contains nothing DMLS supports, or when negotiation is absent
/// (the LSP 3.17 default).
pub fn negotiate_position_encoding(capabilities: &ClientCapabilities) -> PositionEncoding {
    capabilities
        .general
        .as_ref()
        .and_then(|general| general.position_encodings.as_ref())
        .and_then(|offered| {
            offered.iter().find_map(|kind| {
                if *kind == PositionEncodingKind::UTF8 {
                    Some(PositionEncoding::Utf8)
                } else if *kind == PositionEncodingKind::UTF16 {
                    Some(PositionEncoding::Utf16)
                } else {
                    None
                }
            })
        })
        .unwrap_or(PositionEncoding::Utf16)
}

/// Builds the server capability advertisement.
///
/// The advertisement grows phase by phase: only implemented capabilities are
/// advertised. Phase 4 adds the Layer-0 Markdown surface — symbols, navigation,
/// links, references/highlights, hover, completion, and folding — on top of the
/// Phase-2 document-sync lifecycle. Folding is gated on the client profile
/// (omitted for Helix, which has no LSP folding); diagnostics are push-based
/// (`publishDiagnostics`) and need no capability advertisement. Phase 10 adds the
/// editing surface — rename (with prepare support), quick-fix code actions, and
/// whole-document formatting — plus `workspace/willRenameFiles` gated on the
/// client's file-operation support (R-7: not Neovim). The semantic-tokens
/// surface (full + range, no delta) is advertised only when the client requests
/// semantic tokens, publishing the frozen V1 legend.
pub fn server_capabilities(profile: &ClientProfile) -> ServerCapabilities {
    ServerCapabilities {
        workspace: workspace_capabilities(profile),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        position_encoding: Some(match profile.position_encoding {
            PositionEncoding::Utf8 => PositionEncodingKind::UTF8,
            PositionEncoding::Utf16 => PositionEncodingKind::UTF16,
        }),
        text_document_sync: Some(TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
            open_close: Some(true),
            change: Some(TextDocumentSyncKind::FULL),
            will_save: None,
            will_save_wait_until: None,
            save: Some(
                SaveOptions {
                    include_text: Some(false),
                }
                .into(),
            ),
        })),
        document_symbol_provider: Some(OneOf::Left(true)),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        document_link_provider: Some(DocumentLinkOptions {
            // Targets are cheap lexical joins, resolved eagerly.
            resolve_provider: Some(false),
            work_done_progress_options: Default::default(),
        }),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![
                "/".to_string(),
                "(".to_string(),
                "#".to_string(),
                // `ctx.` auto-completes inside `{{ }}`; outside an open
                // interpolation the provider's guard offers nothing (D3).
                ".".to_string(),
            ]),
            ..Default::default()
        }),
        folding_range_provider: profile
            .supports_folding
            .then_some(FoldingRangeProviderCapability::Simple(true)),
        semantic_tokens_provider: profile
            .supports_semantic_tokens
            .then(semantic_tokens_capabilities),
        ..Default::default()
    }
}

/// The semantic-token server capability: the frozen V1 legend with full and
/// range support and no delta (delta is explicitly out of V1).
fn semantic_tokens_capabilities() -> SemanticTokensServerCapabilities {
    SemanticTokensOptions {
        work_done_progress_options: WorkDoneProgressOptions::default(),
        legend: crate::semantic_legend::legend(),
        range: Some(true),
        full: Some(SemanticTokensFullOptions::Bool(true)),
    }
    .into()
}

/// Workspace-scoped server capabilities: `willRenameFiles` for Markdown files,
/// advertised only when the client honors file operations (R-7 gate — not
/// Neovim, which reaches file rename through a code action instead).
fn workspace_capabilities(profile: &ClientProfile) -> Option<WorkspaceServerCapabilities> {
    if !profile.supports_file_operations {
        return None;
    }
    let filter = FileOperationFilter {
        scheme: Some("file".to_string()),
        pattern: FileOperationPattern {
            glob: "**/*.{md,markdown}".to_string(),
            matches: Some(FileOperationPatternKind::File),
            options: None,
        },
    };
    Some(WorkspaceServerCapabilities {
        workspace_folders: None,
        file_operations: Some(WorkspaceFileOperationsServerCapabilities {
            will_rename: Some(FileOperationRegistrationOptions {
                filters: vec![filter],
            }),
            ..Default::default()
        }),
    })
}

/// How rich hover content may be for this client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HoverMediaProfile {
    /// Text-first Markdown; no images or HTML tables (terminal clients and
    /// GUI clients whose hover rendering is conservative).
    #[default]
    TextFirst,
    /// Images and HTML tables may enhance hover (VS Code only) — never
    /// load-bearing.
    RichMedia,
}

/// Editors DMLS keys capability defaults on when advertisement under-reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KnownClient {
    VsCode,
    Zed,
    Neovim,
    Helix,
    Other,
}

impl KnownClient {
    fn from_name(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower.contains("visual studio code") || lower.contains("vscode") {
            Self::VsCode
        } else if lower.contains("zed") {
            Self::Zed
        } else if lower.contains("neovim") {
            Self::Neovim
        } else if lower.contains("helix") {
            Self::Helix
        } else {
            Self::Other
        }
    }
}

/// Per-client gates computed once at `initialize`.
///
/// Fields are derived from advertised capabilities first, then adjusted by
/// keyed defaults for clients whose advertisement under-reports (or whose
/// documented behavior contradicts it — e.g. Zed's completion resolve
/// deliberately never populates `textEdit`).
#[derive(Debug, Clone, PartialEq)]
pub struct ClientProfile {
    /// Client name from `clientInfo`, when sent.
    pub client_name: Option<String>,
    /// Client version from `clientInfo`, when sent.
    pub client_version: Option<String>,
    /// Negotiated position encoding. Nothing may be emitted before
    /// `initialize` completes — clients like Helix cannot decode positions
    /// until the encoding is known.
    pub position_encoding: PositionEncoding,
    /// `WorkspaceEdit` may carry create/rename/delete resource operations.
    pub supports_resource_operations: bool,
    /// `ChangeAnnotation`s render usefully (VS Code, Neovim). Elsewhere the
    /// code-action title must carry the explanation.
    pub supports_change_annotations: bool,
    /// `codeAction/resolve` may lazily populate expensive edits.
    pub supports_code_action_resolve: bool,
    /// `completionItem/resolve` may lazily populate documentation/details.
    pub supports_completion_resolve: bool,
    /// Whether completion resolve may supply `textEdit`. Zed deliberately
    /// never resolves `textEdit`, so DMLS must never defer it to resolve.
    pub resolve_provides_text_edit: bool,
    /// Snippet syntax (tab stops) allowed in completion insert text.
    pub supports_snippets: bool,
    /// Client honors dynamic `workspace/didChangeWatchedFiles` registration.
    pub client_watches_files: bool,
    /// DMLS keeps a server-side rescan/hash fallback (no watchers, or
    /// Neovim-on-Linux where client watching is limited/disabled).
    pub needs_watch_fallback: bool,
    /// `workspace/willRenameFiles` works (VS Code, Zed, Helix — not Neovim).
    pub supports_file_operations: bool,
    /// Client answers `workspace/configuration` requests.
    pub supports_workspace_configuration: bool,
    /// Client requests semantic tokens (`textDocument.semanticTokens`).
    pub supports_semantic_tokens: bool,
    /// Client honors a server-sent `workspace/semanticTokens/refresh`
    /// (`workspace.semanticTokens.refreshSupport`).
    pub supports_semantic_tokens_refresh: bool,
    /// Client supports LSP folding ranges at all (Helix does not).
    pub supports_folding: bool,
    /// Folding ranges must be line-only (Neovim).
    pub folding_line_only: bool,
    /// LSP selection ranges supported (disable for Helix).
    pub supports_selection_range: bool,
    /// Linked editing ranges supported (disable for Helix).
    pub supports_linked_editing: bool,
    /// `window/workDoneProgress` is rendered.
    pub supports_work_done_progress: bool,
    /// Hover media richness.
    pub hover_media: HoverMediaProfile,
    /// The IWES-documented Helix quirk: a one-character selection on a
    /// selection-sensitive code action means an empty selection at the
    /// start character. Applied only to selection-sensitive actions.
    pub helix_one_char_selection_is_empty: bool,
}

impl ClientProfile {
    /// Computes the profile from `initialize` params and the negotiated
    /// encoding.
    pub fn from_initialize(params: &InitializeParams, encoding: PositionEncoding) -> Self {
        let caps = &params.capabilities;
        let workspace = caps.workspace.as_ref();
        let text_document = caps.text_document.as_ref();
        let window = caps.window.as_ref();

        let workspace_edit = workspace.and_then(|ws| ws.workspace_edit.as_ref());
        let completion_item = text_document
            .and_then(|td| td.completion.as_ref())
            .and_then(|completion| completion.completion_item.as_ref());
        let completion_resolve = completion_item.and_then(|item| item.resolve_support.as_ref());
        let folding = text_document.and_then(|td| td.folding_range.as_ref());

        let mut profile = Self {
            client_name: params.client_info.as_ref().map(|info| info.name.clone()),
            client_version: params
                .client_info
                .as_ref()
                .and_then(|info| info.version.clone()),
            position_encoding: encoding,
            supports_resource_operations: workspace_edit
                .and_then(|edit| edit.resource_operations.as_ref())
                .is_some_and(|ops| !ops.is_empty()),
            supports_change_annotations: workspace_edit
                .is_some_and(|edit| edit.change_annotation_support.is_some()),
            supports_code_action_resolve: text_document
                .and_then(|td| td.code_action.as_ref())
                .is_some_and(|action| action.resolve_support.is_some()),
            supports_completion_resolve: completion_resolve.is_some(),
            resolve_provides_text_edit: completion_resolve
                .is_some_and(|resolve| resolve.properties.iter().any(|p| p == "textEdit")),
            supports_snippets: completion_item
                .and_then(|item| item.snippet_support)
                .unwrap_or(false),
            client_watches_files: workspace
                .and_then(|ws| ws.did_change_watched_files.as_ref())
                .and_then(|watch| watch.dynamic_registration)
                .unwrap_or(false),
            needs_watch_fallback: false,
            supports_file_operations: workspace
                .and_then(|ws| ws.file_operations.as_ref())
                .and_then(|ops| ops.will_rename)
                .unwrap_or(false),
            supports_workspace_configuration: workspace
                .and_then(|ws| ws.configuration)
                .unwrap_or(false),
            supports_semantic_tokens: text_document
                .is_some_and(|td| td.semantic_tokens.is_some()),
            supports_semantic_tokens_refresh: workspace
                .and_then(|ws| ws.semantic_tokens.as_ref())
                .and_then(|st| st.refresh_support)
                .unwrap_or(false),
            supports_folding: folding.is_some(),
            folding_line_only: folding
                .and_then(|f| f.line_folding_only)
                .unwrap_or(false),
            supports_selection_range: text_document
                .is_some_and(|td| td.selection_range.is_some()),
            supports_linked_editing: text_document
                .is_some_and(|td| td.linked_editing_range.is_some()),
            supports_work_done_progress: window
                .and_then(|w| w.work_done_progress)
                .unwrap_or(false),
            hover_media: HoverMediaProfile::TextFirst,
            helix_one_char_selection_is_empty: false,
        };

        profile.needs_watch_fallback = !profile.client_watches_files;
        profile.apply_keyed_defaults();
        profile
    }

    /// Adjusts gates for clients whose capabilities under-report (R-7).
    fn apply_keyed_defaults(&mut self) {
        let known = self
            .client_name
            .as_deref()
            .map(KnownClient::from_name)
            .unwrap_or(KnownClient::Other);
        match known {
            KnownClient::VsCode => {
                self.hover_media = HoverMediaProfile::RichMedia;
            }
            KnownClient::Zed => {
                // Zed's completion resolve deliberately never populates
                // `textEdit`, whatever resolveSupport advertises.
                self.resolve_provides_text_edit = false;
            }
            KnownClient::Neovim => {
                // Neovim's capability table sets file operations off; keep
                // rename reachable via a code action/command path instead.
                self.supports_file_operations = false;
                self.folding_line_only = true;
                // Client-side watching is limited/disabled on Linux.
                self.needs_watch_fallback = true;
            }
            KnownClient::Helix => {
                self.supports_folding = false;
                self.supports_selection_range = false;
                self.supports_linked_editing = false;
                self.supports_change_annotations = false;
                self.helix_one_char_selection_is_empty = true;
            }
            KnownClient::Other => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn caps(value: serde_json::Value) -> ClientCapabilities {
        serde_json::from_value(value).unwrap()
    }

    fn init_params(value: serde_json::Value) -> InitializeParams {
        serde_json::from_value(value).unwrap()
    }

    // --- R-2 negotiation matrix ---

    #[test]
    fn test_negotiation_vscode_utf16_only() {
        let caps = caps(json!({ "general": { "positionEncodings": ["utf-16"] } }));
        assert_eq!(negotiate_position_encoding(&caps), PositionEncoding::Utf16);
    }

    #[test]
    fn test_negotiation_neovim_prefers_first_supported() {
        let caps = caps(json!({
            "general": { "positionEncodings": ["utf-8", "utf-16", "utf-32"] }
        }));
        assert_eq!(negotiate_position_encoding(&caps), PositionEncoding::Utf8);
    }

    #[test]
    fn test_negotiation_helix_order_utf8_first() {
        let caps = caps(json!({
            "general": { "positionEncodings": ["utf-8", "utf-32", "utf-16"] }
        }));
        assert_eq!(negotiate_position_encoding(&caps), PositionEncoding::Utf8);
    }

    #[test]
    fn test_negotiation_utf32_first_selects_next_supported() {
        let caps = caps(json!({
            "general": { "positionEncodings": ["utf-32", "utf-16"] }
        }));
        assert_eq!(negotiate_position_encoding(&caps), PositionEncoding::Utf16);
    }

    #[test]
    fn test_negotiation_missing_defaults_to_utf16() {
        let absent = caps(json!({}));
        assert_eq!(negotiate_position_encoding(&absent), PositionEncoding::Utf16);
        let empty_general = caps(json!({ "general": {} }));
        assert_eq!(
            negotiate_position_encoding(&empty_general),
            PositionEncoding::Utf16
        );
    }

    #[test]
    fn test_negotiation_unsupported_only_falls_back_to_utf16() {
        let caps = caps(json!({ "general": { "positionEncodings": ["utf-32"] } }));
        assert_eq!(negotiate_position_encoding(&caps), PositionEncoding::Utf16);
    }

    // --- capability advertisement ---

    fn profile_with(encoding: PositionEncoding, capabilities: serde_json::Value) -> ClientProfile {
        let params = init_params(json!({ "capabilities": capabilities }));
        ClientProfile::from_initialize(&params, encoding)
    }

    #[test]
    fn test_server_capabilities_advertise_layer0_surface() {
        let profile = profile_with(
            PositionEncoding::Utf8,
            json!({ "textDocument": { "foldingRange": {} } }),
        );
        let advertised = server_capabilities(&profile);
        assert_eq!(
            advertised.position_encoding,
            Some(PositionEncodingKind::UTF8)
        );
        let Some(TextDocumentSyncCapability::Options(sync)) = advertised.text_document_sync
        else {
            panic!("expected sync options");
        };
        assert_eq!(sync.open_close, Some(true));
        assert_eq!(sync.change, Some(TextDocumentSyncKind::FULL));
        // Phase 4 Layer-0 surface is advertised.
        assert!(advertised.hover_provider.is_some());
        assert!(advertised.completion_provider.is_some());
        assert!(advertised.definition_provider.is_some());
        assert!(advertised.references_provider.is_some());
        assert!(advertised.document_highlight_provider.is_some());
        assert!(advertised.document_symbol_provider.is_some());
        assert!(advertised.workspace_symbol_provider.is_some());
        assert!(advertised.document_link_provider.is_some());
        assert!(advertised.folding_range_provider.is_some());
        // Phase 10 editing surface: rename (with prepare), code actions, and
        // whole-document formatting.
        assert!(matches!(
            advertised.rename_provider,
            Some(OneOf::Right(RenameOptions {
                prepare_provider: Some(true),
                ..
            }))
        ));
        assert!(advertised.code_action_provider.is_some());
        assert!(advertised.document_formatting_provider.is_some());
    }

    #[test]
    fn test_will_rename_files_gated_on_file_operations() {
        // A client that honors file operations gets willRenameFiles advertised.
        let with_ops = profile_with(
            PositionEncoding::Utf8,
            json!({ "workspace": { "fileOperations": { "willRename": true } } }),
        );
        let advertised = server_capabilities(&with_ops);
        let ops = advertised
            .workspace
            .and_then(|ws| ws.file_operations)
            .and_then(|fo| fo.will_rename);
        assert!(ops.is_some());

        // A client without file operations (e.g. Neovim) does not.
        let neovim = ClientProfile::from_initialize(
            &init_params(json!({
                "clientInfo": { "name": "Neovim", "version": "0.11.0" },
                "capabilities": {}
            })),
            PositionEncoding::Utf8,
        );
        assert!(server_capabilities(&neovim).workspace.is_none());
    }

    #[test]
    fn test_folding_omitted_for_helix_profile() {
        let profile = profile_with(
            PositionEncoding::Utf8,
            json!({ "textDocument": { "foldingRange": {} } }),
        );
        // Helix keys off `clientInfo`; simulate it directly on the profile.
        let helix = ClientProfile::from_initialize(
            &init_params(json!({
                "clientInfo": { "name": "helix", "version": "25.07" },
                "capabilities": { "textDocument": { "foldingRange": {} } }
            })),
            PositionEncoding::Utf8,
        );
        assert!(server_capabilities(&profile).folding_range_provider.is_some());
        assert!(server_capabilities(&helix).folding_range_provider.is_none());
    }

    // --- ClientProfile gates ---

    #[test]
    fn test_profile_from_capabilities() {
        let params = init_params(json!({
            "capabilities": {
                "workspace": {
                    "configuration": true,
                    "workspaceEdit": {
                        "resourceOperations": ["create", "rename", "delete"],
                        "changeAnnotationSupport": { "groupsOnLabel": true }
                    },
                    "didChangeWatchedFiles": { "dynamicRegistration": true },
                    "fileOperations": { "willRename": true },
                    "semanticTokens": { "refreshSupport": true }
                },
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true,
                            "resolveSupport": { "properties": ["documentation", "textEdit"] }
                        }
                    },
                    "codeAction": { "resolveSupport": { "properties": ["edit"] } },
                    "foldingRange": { "lineFoldingOnly": false },
                    "selectionRange": {},
                    "linkedEditingRange": {},
                    "semanticTokens": {
                        "requests": { "full": true, "range": true },
                        "tokenTypes": [],
                        "tokenModifiers": [],
                        "formats": []
                    }
                },
                "window": { "workDoneProgress": true }
            }
        }));
        let profile = ClientProfile::from_initialize(&params, PositionEncoding::Utf16);
        assert!(profile.supports_semantic_tokens);
        assert!(profile.supports_semantic_tokens_refresh);
        assert!(profile.supports_resource_operations);
        assert!(profile.supports_change_annotations);
        assert!(profile.supports_code_action_resolve);
        assert!(profile.supports_completion_resolve);
        assert!(profile.resolve_provides_text_edit);
        assert!(profile.supports_snippets);
        assert!(profile.client_watches_files);
        assert!(!profile.needs_watch_fallback);
        assert!(profile.supports_file_operations);
        assert!(profile.supports_workspace_configuration);
        assert!(profile.supports_folding);
        assert!(!profile.folding_line_only);
        assert!(profile.supports_selection_range);
        assert!(profile.supports_linked_editing);
        assert!(profile.supports_work_done_progress);
        assert_eq!(profile.hover_media, HoverMediaProfile::TextFirst);
        assert!(!profile.helix_one_char_selection_is_empty);
    }

    #[test]
    fn test_profile_zed_never_defers_text_edit_to_resolve() {
        let params = init_params(json!({
            "clientInfo": { "name": "Zed", "version": "1.9.0" },
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "resolveSupport": { "properties": ["documentation", "textEdit"] }
                        }
                    }
                }
            }
        }));
        let profile = ClientProfile::from_initialize(&params, PositionEncoding::Utf16);
        assert!(profile.supports_completion_resolve);
        assert!(!profile.resolve_provides_text_edit);
    }

    #[test]
    fn test_profile_helix_quirks() {
        let params = init_params(json!({
            "clientInfo": { "name": "helix", "version": "25.07.1" },
            "capabilities": {
                "textDocument": { "foldingRange": {}, "selectionRange": {} }
            }
        }));
        let profile = ClientProfile::from_initialize(&params, PositionEncoding::Utf8);
        assert!(!profile.supports_folding);
        assert!(!profile.supports_selection_range);
        assert!(!profile.supports_linked_editing);
        assert!(!profile.supports_change_annotations);
        assert!(profile.helix_one_char_selection_is_empty);
    }

    #[test]
    fn test_profile_neovim_keyed_defaults() {
        let params = init_params(json!({
            "clientInfo": { "name": "Neovim", "version": "0.11.0" },
            "capabilities": {
                "workspace": { "fileOperations": { "willRename": true } },
                "textDocument": { "foldingRange": { "lineFoldingOnly": true } }
            }
        }));
        let profile = ClientProfile::from_initialize(&params, PositionEncoding::Utf8);
        assert!(!profile.supports_file_operations);
        assert!(profile.folding_line_only);
        assert!(profile.needs_watch_fallback);
    }

    #[test]
    fn test_profile_vscode_rich_hover() {
        let params = init_params(json!({
            "clientInfo": { "name": "Visual Studio Code", "version": "1.127.0" },
            "capabilities": {}
        }));
        let profile = ClientProfile::from_initialize(&params, PositionEncoding::Utf16);
        assert_eq!(profile.hover_media, HoverMediaProfile::RichMedia);
    }

    #[test]
    fn test_profile_unknown_client_capability_driven_only() {
        let params = init_params(json!({ "capabilities": {} }));
        let profile = ClientProfile::from_initialize(&params, PositionEncoding::Utf16);
        assert!(!profile.supports_resource_operations);
        assert!(!profile.supports_snippets);
        assert!(!profile.supports_semantic_tokens);
        assert!(!profile.supports_semantic_tokens_refresh);
        assert!(profile.needs_watch_fallback);
        assert_eq!(profile.hover_media, HoverMediaProfile::TextFirst);
    }

    /// A minimal valid `textDocument.semanticTokens` client capability object.
    fn semantic_tokens_client_capability() -> serde_json::Value {
        json!({
            "requests": { "full": true, "range": true },
            "tokenTypes": [],
            "tokenModifiers": [],
            "formats": []
        })
    }

    #[test]
    fn test_profile_semantic_tokens_without_refresh_support() {
        // A client that requests semantic tokens but omits refresh support: the
        // request gate is on, the refresh gate stays conservatively off.
        let profile = profile_with(
            PositionEncoding::Utf16,
            json!({ "textDocument": { "semanticTokens": semantic_tokens_client_capability() } }),
        );
        assert!(profile.supports_semantic_tokens);
        assert!(!profile.supports_semantic_tokens_refresh);
    }

    #[test]
    fn test_semantic_tokens_advertised_only_when_supported() {
        // Capable client: the frozen legend is published with full + range and
        // no delta support.
        let capable = profile_with(
            PositionEncoding::Utf16,
            json!({ "textDocument": { "semanticTokens": semantic_tokens_client_capability() } }),
        );
        let Some(SemanticTokensServerCapabilities::SemanticTokensOptions(options)) =
            server_capabilities(&capable).semantic_tokens_provider
        else {
            panic!("expected semantic-token options for a capable client");
        };
        assert_eq!(options.legend, crate::semantic_legend::legend());
        assert_eq!(options.range, Some(true));
        assert_eq!(options.full, Some(SemanticTokensFullOptions::Bool(true)));

        // Incapable client: no semantic-token capability is advertised at all.
        let incapable = profile_with(PositionEncoding::Utf16, json!({}));
        assert!(
            server_capabilities(&incapable)
                .semantic_tokens_provider
                .is_none()
        );
    }
}
