//! Whole-document formatting: the library clean transaction over the
//! editor's current buffer.
//!
//! `textDocument/formatting` runs [`clean_document`], the transaction `md
//! clean` runs, on the exact unsaved buffer (never a disk read): frontmatter
//! repair, one passive read of the document's `style.markdown`, rule
//! resolution (document over the workspace `formatting.markdown` over the
//! runtime default; DMLS has no caller-level overrides), guarded body
//! formatting, and reassembly. Nothing is composed, executed, or fetched.
//!
//! The transaction either accepts the whole candidate or keeps the source, so
//! the provider returns one whole-document edit only for
//! [`CleanOutcome::Changed`]. A refusal (`Declined`), frontmatter Darkmatter
//! cannot load (`Invalid`), or a repair error returns no edits and one warning
//! for the router to show; the provider itself sends nothing. Warning positions are
//! converted from source bytes into the client's negotiated position
//! encoding.

use darkmatter::markdown::cleanup::diagnostic::{CleanDiagnostic, CleanStage};
use darkmatter::markdown::cleanup::rules::MarkdownRuleOverrides;
use darkmatter::markdown::cleanup::{CleanOutcome, CleanRequest, CleanResult, clean_document};
use darkmatter::markdown::schemas::CleanSchemaConfig;
use lsp_types::{Position, Range, TextEdit};

use super::DocumentContext;
use crate::workspace::uri_to_file_path;

/// What a formatting request produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormattingOutcome {
    /// One whole-document replacement, or none.
    pub edits: Vec<TextEdit>,
    /// Messages the router shows at `Warning` level; at most one, and only
    /// when formatting could not run or declined.
    pub warnings: Vec<String>,
}

/// Formats the current buffer of `ctx`.
///
/// Only a `file:` document passes its path to the transaction; an
/// `untitled:` buffer gets no trigger-schema discovery.
pub fn formatting(ctx: &DocumentContext) -> FormattingOutcome {
    let document_path = uri_to_file_path(ctx.uri).is_some().then_some(ctx.path);
    let request = CleanRequest {
        source: ctx.text,
        overrides: &MarkdownRuleOverrides::default(),
        workspace: Some(&ctx.config.formatting.markdown),
        document_path,
    };
    let result = clean_document(&request, || {
        ctx.file_context()
            .map(|context| CleanSchemaConfig::new(context.clone()))
            .map_err(Clone::clone)
    });
    match result {
        Ok(result) => outcome(ctx, result),
        Err(error) => FormattingOutcome {
            edits: Vec::new(),
            warnings: vec![format!(
                "formatting left the document unchanged: {error}"
            )],
        },
    }
}

fn outcome(ctx: &DocumentContext, result: CleanResult) -> FormattingOutcome {
    match result.outcome {
        CleanOutcome::Changed => FormattingOutcome {
            edits: whole_document_edit(ctx, result.text).into_iter().collect(),
            warnings: Vec::new(),
        },
        CleanOutcome::Unchanged => FormattingOutcome::default(),
        CleanOutcome::Declined => FormattingOutcome {
            edits: Vec::new(),
            warnings: vec![declined_message(ctx, &result.diagnostics)],
        },
        CleanOutcome::Invalid => FormattingOutcome {
            edits: Vec::new(),
            warnings: vec![invalid_message(ctx, &result.diagnostics)],
        },
    }
}

/// The fidelity refusal, restated at the client's 1-based line and column.
fn declined_message(ctx: &DocumentContext, diagnostics: &[CleanDiagnostic]) -> String {
    let Some((diagnostic, fallback)) = diagnostics
        .iter()
        .rev()
        .find_map(|diagnostic| diagnostic.fallback.as_ref().map(|fallback| (diagnostic, fallback)))
    else {
        return "formatting left the document unchanged".to_string();
    };
    match display_position(ctx, fallback.offset) {
        Some((line, column)) => fallback.message(line, column),
        None => diagnostic.message.clone(),
    }
}

/// The first YAML syntax finding that kept the frontmatter from parsing.
fn invalid_message(ctx: &DocumentContext, diagnostics: &[CleanDiagnostic]) -> String {
    let finding = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.stage == CleanStage::Syntax)
        .or_else(|| diagnostics.first());
    let Some(finding) = finding else {
        return "formatting left the document unchanged: the frontmatter is not valid YAML"
            .to_string();
    };
    match display_position(ctx, finding.span.start) {
        Some((line, column)) => format!(
            "formatting left the document unchanged: the frontmatter is not valid YAML at line \
             {line}, column {column}: {}",
            finding.message
        ),
        None => format!(
            "formatting left the document unchanged: the frontmatter is not valid YAML: {}",
            finding.message
        ),
    }
}

/// `offset` as a 1-based line and column in the negotiated encoding.
fn display_position(ctx: &DocumentContext, offset: usize) -> Option<(usize, usize)> {
    let position = ctx.source_map.byte_to_lsp(offset.min(ctx.text.len()))?;
    Some((position.line as usize + 1, position.character as usize + 1))
}

fn whole_document_edit(ctx: &DocumentContext, new_text: String) -> Option<TextEdit> {
    let end = ctx.source_map.byte_to_lsp(ctx.text.len())?;
    Some(TextEdit {
        range: Range::new(Position::new(0, 0), end),
        new_text,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use darkmatter::markdown::cleanup::rules::{FixedWidth, Leaf, ListItemPadding};
    use lsp_types::{InitializeParams, Uri};

    use super::*;
    use crate::capabilities::ClientProfile;
    use crate::config::DmlsConfig;
    use crate::context::test_support::resolution_in;
    use crate::graph::WorkspaceGraph;
    use crate::source_map::{PositionEncoding, SourceMap};

    /// Runs the provider over `text` saved nowhere but named `doc.md` inside
    /// a fresh directory, under `config`.
    fn format(text: &str, config: &DmlsConfig) -> FormattingOutcome {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join("doc.md");
        let uri: Uri = url::Url::from_file_path(&path)
            .expect("file url")
            .as_str()
            .parse()
            .expect("uri");
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
        let graph = WorkspaceGraph::build(
            &BTreeMap::new(),
            1,
            &crate::context::test_support::workspace_contexts(),
        );
        let profile =
            ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
        let resolution = resolution_in(root.path(), &path);
        let ctx = DocumentContext {
            uri: &uri,
            path: &path,
            text,
            source_map: &source_map,
            graph: &graph,
            doc_id: None,
            config,
            profile: &profile,
            overlay: None,
            resolution: &resolution,
        };
        formatting(&ctx)
    }

    /// The single edit's text, asserting it replaces the whole document.
    fn formatted(text: &str, config: &DmlsConfig) -> String {
        let outcome = format(text, config);
        assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
        assert_eq!(outcome.edits.len(), 1, "one whole-document edit");
        assert_eq!(outcome.edits[0].range.start, Position::new(0, 0));
        outcome.edits[0].new_text.clone()
    }

    fn workspace(configure: impl FnOnce(&mut DmlsConfig)) -> DmlsConfig {
        let mut config = DmlsConfig::default();
        configure(&mut config);
        config
    }

    #[test]
    fn default_rules_renumber_an_ordered_list() {
        let source = "1. a\n3. b\n8. c\n";

        assert_eq!(formatted(source, &DmlsConfig::default()), "1. a\n2. b\n3. c\n");
    }

    #[test]
    fn workspace_list_item_padding_none_tightens_a_list() {
        let config = workspace(|config| {
            config.formatting.markdown.list_item_padding = Leaf::Value(ListItemPadding::None);
        });

        assert_eq!(formatted("- a\n\n- b\n", &config), "- a\n- b\n");
    }

    #[test]
    fn workspace_fixed_width_reflows_prose() {
        let config = workspace(|config| {
            config.formatting.markdown.fixed_width = Leaf::Value(FixedWidth::Columns(20));
        });
        let source = "One two three four five six seven eight nine ten.\n";

        assert_eq!(
            formatted(source, &config),
            "One two three four\nfive six seven eight\nnine ten.\n"
        );
    }

    #[test]
    fn document_style_markdown_beats_the_workspace() {
        let config = workspace(|config| {
            config.formatting.markdown.list_item_padding = Leaf::Value(ListItemPadding::None);
        });
        let source = "---\nstyle:\n  markdown:\n    list-item-padding: one\n---\n- a\n- b\n";

        assert_eq!(
            formatted(source, &config),
            "---\nstyle:\n  markdown:\n    list-item-padding: one\n---\n- a\n\n- b\n"
        );
    }

    #[test]
    fn invalid_document_rules_are_ignored_while_workspace_rules_apply() {
        let config = workspace(|config| {
            config.formatting.markdown.list_item_padding = Leaf::Value(ListItemPadding::None);
        });
        // `indentation: 3` is not a valid value, so the whole object is
        // ignored, including its otherwise valid `list-item-padding`.
        let source = concat!(
            "---\nstyle:\n  markdown:\n    list-item-padding: one\n    indentation: 3\n---\n",
            "- a\n\n- b\n"
        );

        assert_eq!(
            formatted(source, &config),
            concat!(
                "---\nstyle:\n  markdown:\n    list-item-padding: one\n    indentation: 3\n---\n",
                "- a\n- b\n"
            )
        );
    }

    #[test]
    fn repairable_frontmatter_is_repaired_as_clean_document_repairs_it() {
        let source = "---\ntitle: Doc   \n---\nBody.\n";
        let expected = clean_document(
            &CleanRequest {
                source,
                overrides: &MarkdownRuleOverrides::default(),
                workspace: Some(&MarkdownRuleOverrides::default()),
                document_path: None,
            },
            || Ok::<_, std::convert::Infallible>(CleanSchemaConfig::new(
                crate::context::test_support::workspace_context(),
            )),
        )
        .expect("clean");
        assert_eq!(expected.outcome, CleanOutcome::Changed);

        assert_eq!(formatted(source, &DmlsConfig::default()), expected.text);
        assert_eq!(expected.text, "---\ntitle: Doc\n---\nBody.\n");
    }

    #[test]
    fn frontmatter_false_leaves_the_block_byte_identical() {
        let source = concat!(
            "---\ntitle: Doc   \nstyle:\n  markdown:\n    frontmatter: false\n---\n",
            "1. a\n3. b\n"
        );

        assert_eq!(
            formatted(source, &DmlsConfig::default()),
            concat!(
                "---\ntitle: Doc   \nstyle:\n  markdown:\n    frontmatter: false\n---\n",
                "1. a\n2. b\n"
            )
        );
    }

    #[test]
    fn an_unchanged_document_gets_no_edit_and_no_warning() {
        let outcome = format("# Title\n\nParagraph.\n", &DmlsConfig::default());

        assert_eq!(outcome, FormattingOutcome::default());
    }
}
