//! The `::file` / `::url` `interpolate=` option in the editor: completion,
//! hover, the malformed-value diagnostic (checked against `md compose`'s own
//! parser), and an included file that keeps its own expression treatment
//! whichever way its parents include it.
//!
//! Each case runs in process through the provider registry, as an editor's
//! request does, over documents written to one temporary Git repository.
//! The process environment, home, and current directory are never read.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::canonicalize_simplified;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, RequestSnapshot};
use dmls::capabilities::ClientProfile;
use dmls::config::DmlsConfig;
use dmls::context::RepositoryContexts;
use dmls::graph::{DocumentIndex, WorkspaceGraph, index_document};
use dmls::providers::semantic_tokens::semantic_tokens_full;
use dmls::providers::{DocumentContext, ProviderRegistry};
use dmls::source_map::{PositionEncoding, SourceMap};
use lsp_types::{
    CompletionItem, Diagnostic, DiagnosticSeverity, Hover, HoverContents, InitializeParams, NumberOrString, Uri,
};

const INVALID_VALUE: &str = "dm.directive.invalid_option_value";
const UNKNOWN_OPTION: &str = "dm.directive.malformed_option";

/// The author-ruled hover sentence for `interpolate=false`.
const RULED_SENTENCE: &str = "interpolation in this file will not be interpolated but instead kept as literal text";

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("create directory");
    std::fs::write(path, text).expect("write file");
}

/// A minimal repository Git discovery accepts, without running `git`.
fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).expect("objects");
    std::fs::create_dir_all(root.join(".git/refs/heads")).expect("refs");
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
}

fn uri_of(path: &Path) -> Uri {
    url::Url::from_file_path(path).expect("file URL").as_str().parse().expect("URI")
}

fn markdown_of(hover: &Hover) -> &str {
    match &hover.contents {
        HoverContents::Markup(markup) => &markup.value,
        other => panic!("a Markdown hover expected: {other:?}"),
    }
}

fn code_of(diagnostic: &Diagnostic) -> Option<&str> {
    match &diagnostic.code {
        Some(NumberOrString::String(code)) => Some(code),
        _ => None,
    }
}

struct Editor {
    _dir: tempfile::TempDir,
    repo: PathBuf,
    contexts: RepositoryContexts,
    config: DmlsConfig,
    registry: ProviderRegistry,
}

impl Editor {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let repo = root.join("repo");
        init_repository(&repo);
        std::fs::create_dir_all(root.join("home")).expect("home");
        let snapshot = RequestSnapshot::new(&root).with_home(Some(root.join("home")));
        write(&repo.join("child.md"), "Child prose.\n");
        Self {
            _dir: dir,
            repo,
            contexts: RepositoryContexts::new(snapshot),
            config: DmlsConfig::default(),
            registry: ProviderRegistry::with_substrate(),
        }
    }

    fn graph(&self, files: &[(&Path, &str)]) -> WorkspaceGraph {
        let indices: BTreeMap<PathBuf, DocumentIndex> =
            files.iter().map(|(path, text)| (path.to_path_buf(), index_document(path, text))).collect();
        WorkspaceGraph::build(&indices, 1, &self.contexts)
    }

    /// Writes `text` to `document` and runs `f` over it within `graph`.
    fn with_document<T>(
        &self,
        document: &Path,
        text: &str,
        graph: &WorkspaceGraph,
        f: impl FnOnce(&DocumentContext, &ProviderRegistry) -> T,
    ) -> T {
        write(document, text);
        let resolution = self.contexts.for_document(document);
        let uri = uri_of(document);
        let workspace = [self.repo.clone()];
        let overlays = dmls::overlay::OverlayState::default();
        let overlay = overlays.for_document(&uri, text, document, &self.config, &workspace, &resolution);
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
        let profile = ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
        let ctx = DocumentContext {
            uri: &uri,
            path: document,
            text,
            source_map: &source_map,
            graph,
            doc_id: graph.document_id(document),
            config: &self.config,
            profile: &profile,
            overlay: overlay.as_ref(),
            resolution: &resolution,
            host: None,
        };
        f(&ctx, &self.registry)
    }

    /// A parent document `parent.md` with `text`, in a graph of just itself.
    fn parent<T>(&self, text: &str, f: impl FnOnce(&DocumentContext, &ProviderRegistry) -> T) -> T {
        let path = self.repo.join("parent.md");
        let graph = self.graph(&[(&path, text)]);
        self.with_document(&path, text, &graph, f)
    }

    fn completion_labels(&self, text: &str) -> Vec<String> {
        self.parent(text, |ctx, registry| {
            registry.completion(ctx, text.trim_end().len()).into_iter().map(|item: CompletionItem| item.label).collect()
        })
    }

    fn diagnostics(&self, text: &str) -> Vec<Diagnostic> {
        self.parent(text, |ctx, registry| registry.diagnostics(ctx))
    }

    /// Whether `md compose`'s library parser accepts `text` as `parent.md`.
    fn composes(&self, text: &str) -> Result<String, String> {
        let path = self.repo.join("parent.md");
        write(&path, text);
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&self.repo, ""))
            .with_source_file(path.clone());
        let snapshot = RequestSnapshot::new(&self.repo).with_home(Some(self.repo.join("home")));
        let request = ComposeRequest::prepare(options, &snapshot).expect("request");
        Markdown::try_from(path.as_path())
            .expect("document loads")
            .compose_with(&request)
            .map(|(composed, _)| composed.content().to_string())
            .map_err(|error| error.to_string())
    }
}

// ── Completion ──────────────────────────────────────────────────────────────

#[test]
fn file_and_url_offer_the_interpolate_key_and_code_does_not() {
    let editor = Editor::new();
    for line in ["::file ./child.md interp", "::url https://example.com/a.md interp"] {
        let labels = editor.completion_labels(&format!("{line}\n"));
        assert!(labels.iter().any(|label| label == "interpolate"), "`{line}` offers interpolate: {labels:?}");
    }
    let labels = editor.completion_labels("::code ./child.md interp\n");
    assert!(!labels.iter().any(|label| label == "interpolate"), "`::code` never offers it: {labels:?}");
}

#[test]
fn the_interpolate_value_completes_to_true_and_false() {
    let editor = Editor::new();
    let mut labels = editor.completion_labels("::file ./child.md interpolate=\n");
    labels.sort();
    assert_eq!(labels, ["false", "true"]);
    assert_eq!(editor.completion_labels("::file ./child.md interpolate=f\n"), ["false"]);
}

// ── Hover ───────────────────────────────────────────────────────────────────

#[test]
fn hovering_interpolate_false_explains_literal_prose_defaulting_and_inheritance() {
    let editor = Editor::new();
    let text = "::file ./child.md interpolate=false\n";
    for (needle, nudge) in [("interpolate", 2), ("false", 1)] {
        let offset = text.find(needle).expect("needle") + nudge;
        let hover = editor.parent(text, |ctx, registry| registry.hover(ctx, offset)).expect("an option hover");
        let markdown = markdown_of(&hover);
        assert!(markdown.contains(RULED_SENTENCE), "the ruled sentence: {markdown}");
        assert!(markdown.contains("Omitted or `null` means `true`"), "defaulting: {markdown}");
        assert!(markdown.contains("inherits `false`"), "inheritance: {markdown}");
        assert!(markdown.contains("cannot re-enable it and adds no warning"), "nested true: {markdown}");
        let range = hover.range.expect("a hover range");
        let start = text.find("interpolate").unwrap() as u32;
        assert_eq!((range.start.character, range.end.character), (start, start + "interpolate=false".len() as u32));
    }
    // Elsewhere on the line the directive's own hover is unchanged.
    let hover = editor.parent(text, |ctx, registry| registry.hover(ctx, 2)).expect("the directive hover");
    assert!(markdown_of(&hover).contains("Transclude another Markdown file"));
    assert!(!markdown_of(&hover).contains(RULED_SENTENCE));
}

#[test]
fn hovering_interpolate_on_code_gives_no_option_hover() {
    let editor = Editor::new();
    let text = "::code ./child.md interpolate=false\n";
    let offset = text.find("interpolate").unwrap() + 2;
    let hover = editor.parent(text, |ctx, registry| registry.hover(ctx, offset));
    assert!(hover.is_none_or(|hover| !markdown_of(&hover).contains(RULED_SENTENCE)));
}

// ── Diagnostic, in agreement with composition ───────────────────────────────

/// One option spelling on `::file ./child.md …`, whether composition accepts
/// it, and the text the editor must flag when it does not.
struct Spelling {
    options: &'static str,
    accepted: bool,
    flagged: &'static str,
}

const SPELLINGS: &[Spelling] = &[
    Spelling { options: "interpolate=false", accepted: true, flagged: "" },
    Spelling { options: "interpolate=true", accepted: true, flagged: "" },
    Spelling { options: "interpolate=null", accepted: true, flagged: "" },
    Spelling { options: "interpolate=No", accepted: true, flagged: "" },
    Spelling { options: "interpolate=0", accepted: true, flagged: "" },
    Spelling { options: "interpolate=\"false\"", accepted: true, flagged: "" },
    Spelling { options: "interpolate=true interpolate=false", accepted: true, flagged: "" },
    Spelling { options: "interpolate=\"null\"", accepted: false, flagged: "\"null\"" },
    Spelling { options: "interpolate=banana", accepted: false, flagged: "banana" },
    Spelling { options: "interpolate=", accepted: false, flagged: "interpolate" },
    Spelling { options: "interpolate=\"\"", accepted: false, flagged: "\"\"" },
    Spelling { options: "interpolate=banana interpolate=false", accepted: false, flagged: "banana" },
];

#[test]
fn the_editor_flags_exactly_the_values_composition_rejects_at_the_option() {
    let editor = Editor::new();
    for spelling in SPELLINGS {
        let text = format!("Intro.\n\n::file ./child.md {}\n", spelling.options);
        let composed = editor.composes(&text);
        assert_eq!(composed.is_ok(), spelling.accepted, "composition of `{}`: {composed:?}", spelling.options);

        let invalid: Vec<Diagnostic> =
            editor.diagnostics(&text).into_iter().filter(|d| code_of(d) == Some(INVALID_VALUE)).collect();
        if spelling.accepted {
            assert!(invalid.is_empty(), "`{}` is not flagged: {invalid:#?}", spelling.options);
            assert!(composed.unwrap().contains("Child prose."));
            continue;
        }
        let [diagnostic] = invalid.as_slice() else {
            panic!("`{}` is flagged once: {invalid:#?}", spelling.options);
        };
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        let line_start = text.find("::file").unwrap();
        let start = text[line_start..].find(spelling.flagged).unwrap() as u32;
        assert_eq!(diagnostic.range.start.line, 2, "`{}`", spelling.options);
        assert_eq!(
            (diagnostic.range.start.character, diagnostic.range.end.character),
            (start, start + spelling.flagged.len() as u32),
            "`{}` is flagged at `{}`",
            spelling.options,
            spelling.flagged
        );
        // The editor's reason is the one composition reports.
        let error = composed.unwrap_err();
        assert!(error.contains(&diagnostic.message), "`{}`: `{error}` carries `{}`", spelling.options, diagnostic.message);
    }
}

/// Expression-valued spellings, each evaluated by composition before the
/// option is validated; `{fixed}` becomes the literal `true` or `false`.
const EXPRESSION_SPELLINGS: &[&str] = &[
    "interpolate={{ flag }}",
    "interpolate={{flag}}",
    "interpolate=\"{{ flag }}\"",
    "interpolate=\"{{ fixed }}\"",
];

fn invalid_values(diagnostics: &[Diagnostic]) -> Vec<&Diagnostic> {
    diagnostics.iter().filter(|d| code_of(d) == Some(INVALID_VALUE)).collect()
}

/// The editor cannot know an expression's result, so it never judges an
/// expression-valued option, while composition evaluates and accepts it.
#[test]
fn an_expression_valued_option_is_not_judged_before_composition_evaluates_it() {
    let editor = Editor::new();
    for flag in ["false", "true"] {
        for spelling in EXPRESSION_SPELLINGS {
            let options = spelling.replace("fixed", flag);
            let front = format!("---\nflag: {flag}\n---\n");
            let file_text = format!("{front}::file ./child.md {options}\n");
            let composed = editor.composes(&file_text);
            assert!(composed.is_ok(), "composition of `{options}` with flag {flag}: {composed:?}");

            for line in [format!("::file ./child.md {options}"), format!("::url https://example.com/a.md {options}")] {
                let diagnostics = editor.diagnostics(&format!("{front}{line}\n"));
                assert!(invalid_values(&diagnostics).is_empty(), "`{line}` (flag {flag}): {diagnostics:#?}");
                assert!(!diagnostics.iter().any(|d| code_of(d) == Some(UNKNOWN_OPTION)), "`{line}`: {diagnostics:#?}");
            }
        }
    }
}

/// An expression whose result is not a boolean word fails composition at
/// the directive; the editor, which never evaluates it, stays silent.
#[test]
fn an_expression_with_a_non_boolean_result_fails_only_in_composition() {
    let editor = Editor::new();
    let text = "Intro.\n\n::file ./child.md interpolate={{ \"banana\" }}\n";
    let error = editor.composes(text).expect_err("composition rejects the result");
    assert!(error.contains("not banana"), "{error}");
    let diagnostics = editor.diagnostics(text);
    assert!(invalid_values(&diagnostics).is_empty(), "{diagnostics:#?}");
}

/// `::url` judges literal values exactly as `::file` does.
#[test]
fn url_flags_the_same_literal_values_as_file() {
    let editor = Editor::new();
    for spelling in SPELLINGS {
        let file = invalid_values(&editor.diagnostics(&format!("::file ./child.md {}\n", spelling.options)))
            .into_iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>();
        let url = invalid_values(&editor.diagnostics(&format!("::url https://example.com/a.md {}\n", spelling.options)))
            .into_iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>();
        assert_eq!(url, file, "`{}`", spelling.options);
        assert_eq!(url.is_empty(), spelling.accepted, "`{}`: {url:?}", spelling.options);
    }
}

#[test]
fn interpolate_on_code_is_an_unknown_option_not_a_bad_value() {
    let editor = Editor::new();
    let literal = std::iter::once("interpolate=banana".to_string()).chain(["interpolate=false".to_string()]);
    let expressions = EXPRESSION_SPELLINGS.iter().map(|spelling| spelling.replace("fixed", "false"));
    for options in literal.chain(expressions) {
        let diagnostics = editor.diagnostics(&format!("---\nflag: false\n---\n::code ./child.md {options}\n"));
        assert!(diagnostics.iter().any(|d| code_of(d) == Some(UNKNOWN_OPTION)), "`{options}`: {diagnostics:#?}");
        assert!(invalid_values(&diagnostics).is_empty(), "`{options}`: {diagnostics:#?}");
    }
}

// ── An open included file keeps its own treatment ───────────────────────────

/// What the editor shows for an open `child.md`: its diagnostics, its
/// semantic tokens, and the hover on its `{{ title }}`.
fn child_view(editor: &Editor, parents: &[(&str, &str)]) -> (Vec<Diagnostic>, Vec<u32>, String) {
    let child = editor.repo.join("child.md");
    let text = "---\ntitle: Child\n---\n\nHello {{ title }} and {{ undeclared_name }} and {{ title | }}.\n";
    let parent_paths: Vec<(PathBuf, &str)> =
        parents.iter().map(|(name, body)| (editor.repo.join(name), *body)).collect();
    for (path, body) in &parent_paths {
        write(path, body);
    }
    let mut files: Vec<(&Path, &str)> = parent_paths.iter().map(|(path, body)| (path.as_path(), *body)).collect();
    files.push((child.as_path(), text));
    let graph = editor.graph(&files);
    editor.with_document(&child, text, &graph, |ctx, registry| {
        let diagnostics = registry.diagnostics(ctx);
        let tokens = semantic_tokens_full(ctx)
            .data
            .iter()
            .flat_map(|token| [token.delta_line, token.delta_start, token.length, token.token_type])
            .collect();
        let offset = text.find("{{ title }}").unwrap() + 4;
        let hover = registry.hover(ctx, offset).map(|hover| markdown_of(&hover).to_string()).unwrap_or_default();
        (diagnostics, tokens, hover)
    })
}

#[test]
fn an_open_child_is_analyzed_the_same_whichever_way_its_parents_include_it() {
    let editor = Editor::new();
    let alone = child_view(&editor, &[]);
    assert!(
        alone.0.iter().any(|d| code_of(d).is_some_and(|code| code.starts_with("dm.expression."))),
        "the child's malformed expression is diagnosed: {:#?}",
        alone.0
    );
    assert!(!alone.1.is_empty(), "the child's expressions are highlighted");
    assert!(alone.2.contains("title"), "the child's expression hovers: {}", alone.2);

    let disabled = ("literal.md", "::file ./child.md interpolate=false\n");
    let normal = ("normal.md", "::file ./child.md\n");
    for parents in [vec![disabled], vec![normal], vec![disabled, normal]] {
        let view = child_view(&editor, &parents);
        assert_eq!(view, alone, "parents {parents:?} change nothing in the open child");
    }
}
