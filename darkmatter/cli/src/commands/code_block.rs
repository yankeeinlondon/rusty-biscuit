//! `md code-block` subcommand implementation.

use crate::args::Cli;
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::request::MdRequest;
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use biscuit_terminal::terminal::Terminal;
use clap::CommandFactory as _;
use color_eyre::eyre::{Context, Result};
use darkmatter::markdown::CodeBlock;
use darkmatter::markdown::dsl::{CodeBlockMeta, parse_highlight_spec};
use darkmatter::markdown::highlighting::ThemePair;
use darkmatter::markdown::output::command::{CodeBlockData, CommandValue, terminal_fold};
use crate::io::open_argument;
use biscuit_file::FileReference;
use renderable::OutputOptions;
use renderable::tree::{Document, TreeRenderable};
use std::path::{Path, PathBuf};
use tracing::instrument;

/// `md code-block`'s flags.
#[derive(Debug, Clone)]
pub struct CodeBlockFlags {
    /// `--file`.
    pub force_file: bool,
    /// `--content`.
    pub force_content: bool,
    /// `--language`.
    pub language: Option<String>,
    /// `--theme`.
    pub theme: Option<ThemePair>,
    /// `--title`.
    pub title: Option<String>,
    /// `--line-numbering`.
    pub line_numbering: bool,
    /// `--highlight`.
    pub highlight: Option<String>,
}

/// Run the `md code-block` subcommand: report a single [`CodeBlock`] from a file
/// or literal content, built directly without synthesizing a Markdown document.
#[instrument(skip_all, fields(command = "code-block"))]
pub fn run_code_block(input: &str, flags: &CodeBlockFlags, output: OutputTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    let run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::typed(CodeBlockData {
        file: None,
        code: None,
        language: None,
        title: flags.title.clone(),
        line_numbering: flags.line_numbering,
        highlight: Vec::new(),
    }))?;
    // A malformed range is an argument problem: no command runs.
    let highlight = flags
        .highlight
        .as_deref()
        .map(parse_highlight_spec)
        .transpose()
        .map_err(|error| {
            Cli::command().error(
                clap::error::ErrorKind::ValueValidation,
                format!("Invalid --highlight range: {error} (expected comma-separated lines and ranges, e.g. `1,4-6`)"),
            )
        })?;
    let result = code_block_value(input, flags, highlight, request);
    run.finish(result)
}

fn code_block_value(
    input: &str,
    flags: &CodeBlockFlags,
    highlight: Option<darkmatter::markdown::dsl::HighlightSpec>,
    request: &MdRequest,
) -> Result<Completed<CommandValue<CodeBlockData>>> {
    // `--content` keeps the input literal and `--file` requires a file; with
    // neither, see `default_input_file` for when the input names a file.
    let source_file = if flags.force_content {
        None
    } else if flags.force_file {
        Some(
            open_argument(Path::new(input), request)
                .wrap_err_with(|| format!("`{input}` is not a valid file path (--file was passed)"))?
                .into_path(),
        )
    } else {
        default_input_file(input, request)?
    };
    let file = source_file.as_ref().map(|path| path.display().to_string());
    let (code, inferred_lang_token) = match source_file {
        Some(path) => {
            let body = std::fs::read_to_string(&path)
                .wrap_err_with(|| format!("Failed to read code source from `{}`", path.display()))?;
            let lang_token = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|s| s.to_string());
            (body, lang_token)
        }
        None => (input.to_string(), None),
    };

    // Build the CodeBlock. The CLI's explicit --language wins over both the
    // file-extension heuristic and any literal-content guess.
    let lang_token: Option<&str> = flags
        .language
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or(inferred_lang_token.as_deref());
    let mut block = match lang_token {
        Some(lang) => CodeBlock::new(code).with_fence_language(lang),
        None => CodeBlock::new(code),
    };

    // Apply theme override. When set, the block uses a pinned theme instead
    // of resolving through the page context — useful for one-off renders.
    if let Some(theme) = flags.theme {
        block = block.with_theme(theme);
    }

    if flags.title.is_some() || flags.line_numbering || highlight.is_some() {
        let meta = CodeBlockMeta {
            title: flags.title.clone(),
            line_numbering: flags.line_numbering,
            highlight: highlight.unwrap_or_default(),
            ..CodeBlockMeta::default()
        };
        block = block.with_meta(meta);
    }

    let data = CodeBlockData::new(file, block.code(), block.language().map(ToString::to_string), block.meta());
    Ok(Completed::success(code_value(data, block)))
}

/// A code block's value: a fence from the generic fold in `plain`, and the
/// block's own panel in a styled terminal.
pub(crate) fn code_value<D>(data: D, block: CodeBlock) -> CommandValue<D> {
    let markdown = code_block_markdown(&block);
    let html = BrowserRenderable::render_html_fragment(&block).render();
    let prose = Document::new(block.render_tree());
    let document = prose.clone();
    CommandValue::new(data, prose)
        .with_terminal(move |options| code_terminal(&block, &document, options))
        .with_markdown(markdown)
        .with_html(html)
}

fn code_terminal(block: &CodeBlock, document: &Document, options: &OutputOptions) -> String {
    if options.terminal_profile.is_plain() {
        terminal_fold(document, options)
    } else {
        block.render(&Terminal::from_output_options(options))
    }
}

/// Serializes a [`CodeBlock`] back to a Markdown fenced code block.
///
/// The returned string always ends with exactly one trailing newline. The
/// fence length is chosen so that any run of backticks inside the code body
/// is shorter than the fence itself, guaranteeing a safe round-trip.
fn code_block_markdown(block: &CodeBlock) -> String {
    let lang = block
        .language()
        .map(|g| g.to_string())
        .unwrap_or_default();
    let meta = block.meta();
    let mut parts: Vec<String> = Vec::new();
    if !lang.is_empty() {
        parts.push(lang);
    }
    if let Some(title) = &meta.title {
        parts.push(format!("title=\"{}\"", title.replace('"', "\\\"")));
    }
    if meta.line_numbering {
        parts.push("line-numbering=true".to_string());
    }
    if !meta.highlight.is_empty() {
        parts.push(format!("highlight={}", meta.highlight));
    }
    let info = parts.join(" ");
    let code = block.code();
    // A file's last line ending is the fence's own line ending, not a blank
    // line of code.
    let code = code.strip_suffix('\n').map_or(code, |body| body.strip_suffix('\r').unwrap_or(body));

    let longest_backtick_run = code
        .chars()
        .fold((0usize, 0usize), |(max, current), c| {
            if c == '`' {
                (max.max(current + 1), current + 1)
            } else {
                (max, 0)
            }
        })
        .0;
    let fence_len = if longest_backtick_run >= 3 {
        longest_backtick_run + 1
    } else {
        3
    };
    let fence = "`".repeat(fence_len);

    if info.is_empty() {
        format!("{fence}\n{code}\n{fence}\n")
    } else {
        format!("{fence}{info}\n{code}\n{fence}\n")
    }
}

/// The file an input given without `--file` or `--content` names, if any.
///
/// The input names a file only when it is one line of valid reference syntax
/// that resolves, in the launch context, to an existing file; `&src/main.rs`
/// and `^lib.rs` work like any file argument. Everything else is literal
/// code: multi-line text, text that is not reference syntax (so a file
/// literally named `@` is read only as `./@`), a reference that matches no
/// file, and a reference that fails to resolve, such as a relative path
/// climbing out of the launch repository or a missing `{{VAR}}`. Such input
/// is rendered as typed, never read.
///
/// ## Errors
///
/// The launch context builder's error.
fn default_input_file(input: &str, request: &MdRequest) -> Result<Option<PathBuf>> {
    if input.contains(['\n', '\r']) {
        return Ok(None);
    }
    let Ok(reference) = FileReference::new(input) else {
        return Ok(None);
    };
    Ok(reference.resolve_in_context(request.launch_context()?).ok().flatten())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_block_markdown_language_only() {
        let block = CodeBlock::new("fn main() {}").with_fence_language("rust");
        assert_eq!(code_block_markdown(&block), "```rust\nfn main() {}\n```\n");
    }

    #[test]
    fn code_block_markdown_quoted_title_with_double_quote() {
        let meta = CodeBlockMeta {
            title: Some(r#"say "hi""#.to_string()),
            ..Default::default()
        };
        let block = CodeBlock::new("x").with_fence_language("rust").with_meta(meta);
        let md = code_block_markdown(&block);
        assert!(md.contains(r#"title="say \"hi\"""#), "escaped title not found in: {md:?}");
    }

    #[test]
    fn code_block_markdown_line_numbering() {
        let meta = CodeBlockMeta {
            line_numbering: true,
            ..Default::default()
        };
        let block = CodeBlock::new("x").with_fence_language("rust").with_meta(meta);
        assert_eq!(code_block_markdown(&block), "```rust line-numbering=true\nx\n```\n");
    }

    #[test]
    fn code_block_markdown_highlight_ranges() {
        let meta = CodeBlockMeta {
            highlight: parse_highlight_spec("1,3-5").unwrap(),
            ..Default::default()
        };
        let block = CodeBlock::new("x").with_fence_language("rust").with_meta(meta);
        assert_eq!(code_block_markdown(&block), "```rust highlight=1,3-5\nx\n```\n");
    }

    #[test]
    fn code_block_markdown_empty_language_with_metadata() {
        let meta = CodeBlockMeta {
            title: Some("Untitled".to_string()),
            ..Default::default()
        };
        let block = CodeBlock::new("x").with_meta(meta);
        assert_eq!(code_block_markdown(&block), "```title=\"Untitled\"\nx\n```\n");
    }

    #[test]
    fn code_block_markdown_escapes_embedded_fences() {
        let block = CodeBlock::new("```\ninner\n```").with_fence_language("text");
        let md = code_block_markdown(&block);
        assert!(md.starts_with("````text\n"), "expected longer opening fence, got: {md:?}");
        assert!(md.ends_with("\n````\n"), "expected longer closing fence, got: {md:?}");
        assert!(md.contains("```\ninner\n```"));
    }

    #[test]
    fn parse_highlight_spec_rejects_inverted_range() {
        let err = parse_highlight_spec("6-4").unwrap_err();
        assert!(err.to_string().contains("start must be <= end"), "{err}");
    }
}
