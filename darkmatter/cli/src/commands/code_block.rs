//! `md code-block` subcommand implementation.

use crate::args::{CodeBlockOutput, Cli};
use crate::output::{OutputArtifact, open_output_artifact};
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::highlighting::ThemePair;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
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
) -> Result<()> {
    use darkmatter::markdown::CodeBlock;
    use darkmatter::markdown::dsl::CodeBlockMeta;

    // Resolve input source: --file / --content force the interpretation;
    // otherwise prefer filesystem existence and fall back to literal content.
    let (code, inferred_lang_token) = if force_content {
        (input.to_string(), None)
    } else if force_file {
        let path = resolve_file_path_raw(input).wrap_err_with(|| {
            format!("`{input}` is not a valid file path (--file was passed)")
        })?;
        let body = std::fs::read_to_string(&path)
            .wrap_err_with(|| format!("Failed to read code source from `{}`", path.display()))?;
        let lang_token = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_string());
        (body, lang_token)
    } else {
        let path = std::path::Path::new(input);
        if path.is_file() {
            let body = std::fs::read_to_string(path)
                .wrap_err_with(|| format!("Failed to read code source from `{}`", path.display()))?;
            let lang_token = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|s| s.to_string());
            (body, lang_token)
        } else {
            (input.to_string(), None)
        }
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
            meta.highlight = parse_highlight_cli(h).map_err(|e| {
                eyre!("Invalid --highlight range: {e} (expected comma-separated lines and ranges, e.g. `1,4-6`)")
            })?;
        }
        block = block.with_meta(meta);
    }

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
            let lang = block
                .language()
                .map(|g| g.to_string())
                .unwrap_or_default();
            let meta = block.meta();
            let mut parts: Vec<String> = Vec::new();
            if !lang.is_empty() {
                parts.push(lang);
            }
            if let Some(t) = &meta.title {
                parts.push(format!("title=\"{}\"", t.replace('"', "\\\"")));
            }
            if meta.line_numbering {
                parts.push("line-numbering=true".to_string());
            }
            if !meta.highlight.is_empty() {
                parts.push(format!("highlight={}", meta.highlight));
            }
            let info = parts.join(" ");
            let code = block.code();
            println!("```{info}\n{code}\n```");
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
                let lang = block
                    .language()
                    .map(|g| g.to_string())
                    .unwrap_or_default();
                let meta = block.meta();
                let mut parts: Vec<String> = Vec::new();
                if !lang.is_empty() {
                    parts.push(lang);
                }
                if let Some(t) = &meta.title {
                    parts.push(format!("title=\"{}\"", t.replace('"', "\\\"")));
                }
                if meta.line_numbering {
                    parts.push("line-numbering=true".to_string());
                }
                if !meta.highlight.is_empty() {
                    parts.push(format!("highlight={}", meta.highlight));
                }
                let info = parts.join(" ");
                let code = block.code();
                let content = format!("```{info}\n{code}\n```\n");
                let artifact = OutputArtifact {
                    content,
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

/// Parses a `--highlight` value (e.g. `1,4-6`) into a [`HighlightSpec`].
///
/// This is a CLI-local parser that mirrors the syntax
/// [`parse_code_info`](darkmatter::markdown::dsl::parse_code_info) accepts
/// for the `highlight=…` directive, but with explicit error messages on the
/// CLI path so users see a clear failure rather than a downstream render
/// error.
fn parse_highlight_cli(
    raw: &str,
) -> Result<darkmatter::markdown::dsl::HighlightSpec, String> {
    use darkmatter::markdown::dsl::{HighlightSpec, ValidLineRange};
    let mut spec = HighlightSpec::new();

    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if part.contains('-') {
            let range_parts: Vec<&str> = part.split('-').collect();
            if range_parts.len() != 2 {
                return Err(format!("Invalid range format: {part}"));
            }
            let start = range_parts[0]
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("Invalid start number: {}", range_parts[0]))?;
            let end = range_parts[1]
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("Invalid end number: {}", range_parts[1]))?;
            let range = ValidLineRange::range(start, end)
                .map_err(|e| format!("{e}"))?;
            spec.add_line(range.start());
            if range.end() != range.start() {
                spec.add_range(range.start(), range.end())
                    .map_err(|e| format!("{e}"))?;
            }
        } else {
            let line = part
                .parse::<usize>()
                .map_err(|_| format!("Invalid line number: {part}"))?;
            spec.add_line(line);
        }
    }

    Ok(spec)
}

/// Resolves a raw file path string (no FileReference syntax) to a `PathBuf`.
///
/// The plan's `code-block` command treats the positional input as a plain
/// file path when `--file` is passed, so we deliberately skip the
/// `FileReference` indirection and resolve relative paths against the
/// current working directory.
fn resolve_file_path_raw(raw: &str) -> Result<PathBuf> {
    let p = PathBuf::from(raw);
    if p.is_absolute() {
        Ok(p)
    } else {
        Ok(std::env::current_dir()
            .wrap_err("Failed to get current directory")?
            .join(p))
    }
}
