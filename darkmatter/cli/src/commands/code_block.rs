//! `md code-block` subcommand implementation.

use crate::args::{CodeBlockOutput, Cli};
use crate::artifact::{OutputArtifact, open_output_artifact};
use crate::request::MdRequest;
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::CodeBlock;
use darkmatter::markdown::dsl::{CodeBlockMeta, parse_highlight_spec};
use darkmatter::markdown::highlighting::ThemePair;
use std::io::{self, IsTerminal};
use crate::io::open_argument;
use biscuit_file::FileReference;
use std::path::{Path, PathBuf};
use tracing::instrument;

/// Run the `md code-block` subcommand: render a single [`CodeBlock`] from a
/// file or literal content. Constructs the [`CodeBlock`] directly (without
/// synthesizing a Markdown document) and emits the requested output format.
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all, fields(command = "code-block"))]
pub fn run_code_block(
    input: &str,
    force_file: bool,
    force_content: bool,
    language: Option<&str>,
    theme: Option<ThemePair>,
    title: Option<&str>,
    line_numbering: bool,
    highlight: Option<&str>,
    output: CodeBlockOutput,
    cli: &Cli,
    request: &MdRequest,
) -> Result<()> {
    // `--content` keeps the input literal and `--file` requires a file; with
    // neither, see `default_input_file` for when the input names a file.
    let source_file = if force_content {
        None
    } else if force_file {
        Some(
            open_argument(Path::new(input), request)
                .wrap_err_with(|| format!("`{input}` is not a valid file path (--file was passed)"))?
                .into_path(),
        )
    } else {
        default_input_file(input, request)?
    };
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
    let lang_token: Option<&str> = language
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .or(inferred_lang_token.as_deref());
    let mut block = match lang_token {
        Some(lang) => CodeBlock::new(code).with_fence_language(lang),
        None => CodeBlock::new(code),
    };

    // Apply theme override. When set, the block uses a pinned theme instead
    // of resolving through the page context — useful for one-off renders.
    if let Some(theme) = theme {
        block = block.with_theme(theme);
    }

    // Apply metadata (title, line numbering, highlight) on the block. A
    // direct `CodeBlockMeta` is constructed (rather than routed through
    // `parse_code_info`) so we can surface malformed --highlight ranges
    // as a clear error instead of silently dropping the directive.
    if title.is_some() || line_numbering || highlight.is_some() {
        let mut meta = CodeBlockMeta::default();
        if let Some(t) = title {
            meta.title = Some(t.to_string());
        }
        if line_numbering {
            meta.line_numbering = true;
        }
        if let Some(h) = highlight {
            meta.highlight = parse_highlight_spec(h).map_err(|e| {
                eyre!("Invalid --highlight range: {e} (expected comma-separated lines and ranges, e.g. `1,4-6`)")
            })?;
        }
        block = block.with_meta(meta);
    }

    // Pre-compute the Markdown representation so the stdout and `--show`
    // paths emit byte-identical artifacts.
    let markdown_artifact = code_block_markdown(&block);

    // Render to the requested output format.
    match output {
        CodeBlockOutput::Terminal => {
            let stdout_is_tty = io::stdout().is_terminal();
            let term = if stdout_is_tty {
                Terminal::new()
            } else {
                Terminal::new_optimistic(80)
            };
            let rendered = TerminalRenderable::render(&block, &term);
            print!("{rendered}");
        }
        CodeBlockOutput::Html => {
            let fragment = BrowserRenderable::render_html_fragment(&block);
            print!("{}", fragment.render());
        }
        CodeBlockOutput::Markdown => {
            print!("{markdown_artifact}");
        }
    }

    // `--show` opens the rendered output in the default app via a temp file.
    // Reuse the markdown/HTML artifact flow: HTML goes to a `.html` temp file
    // and markdown to a `.md` temp file.
    if cli.show {
        match output {
            CodeBlockOutput::Html => {
                let fragment = BrowserRenderable::render_html_fragment(&block);
                let artifact = OutputArtifact {
                    content: fragment.render(),
                    extension: "html",
                    label: "code-block-html",
                };
                open_output_artifact(&artifact)?;
            }
            CodeBlockOutput::Markdown => {
                let artifact = OutputArtifact {
                    content: markdown_artifact,
                    extension: "md",
                    label: "code-block-markdown",
                };
                open_output_artifact(&artifact)?;
            }
            CodeBlockOutput::Terminal => {
                // Terminal output is already on stdout; opening it as a file
                // would just write the same ANSI to a `.txt` and `xdg-open`
                // it, which is rarely useful. Emit a hint instead.
                eprintln!(
                    "--show is a no-op for terminal output; pass --output html or --output markdown to open in an app"
                );
            }
        }
    }

    Ok(())
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
    fn code_block_markdown_show_artifact_matches_stdout() {
        let block = CodeBlock::new("fn main() {}").with_fence_language("rust");
        let markdown = code_block_markdown(&block);
        let artifact = OutputArtifact {
            content: markdown.clone(),
            extension: "md",
            label: "code-block-markdown",
        };
        assert_eq!(artifact.content, "```rust\nfn main() {}\n```\n");
        assert_eq!(artifact.content, markdown);
    }

    #[test]
    fn parse_highlight_spec_rejects_inverted_range() {
        let err = parse_highlight_spec("6-4").unwrap_err();
        assert!(err.to_string().contains("start must be <= end"), "{err}");
    }
}
