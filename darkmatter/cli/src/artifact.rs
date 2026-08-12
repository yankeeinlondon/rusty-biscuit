//! Output artifact plumbing for the `md` CLI.
//!
//! Builds, emits, opens, and writes the temporary files produced by
//! non-terminal output paths (Markdown, MarkdownPlus, HTML, JSON).

use crate::args::Cli;
use crate::render::{ResolvedTheme, apply_cli_layout_flags, apply_style_frontmatter};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result};
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// An output artifact produced by a non-terminal render path.
#[derive(Debug)]
pub struct OutputArtifact {
    pub content: String,
    pub extension: &'static str,
    pub label: &'static str,
}

/// Build a Markdown artifact from the already-cleaned document.
pub fn markdown_artifact(md: &Markdown) -> OutputArtifact {
    OutputArtifact {
        content: md.as_string(),
        extension: "md",
        label: "markdown",
    }
}

/// Build an HTML artifact.
pub fn html_artifact(
    md: &Markdown,
    cli: &Cli,
    input_path: Option<&PathBuf>,
) -> Result<OutputArtifact> {
    let term = Terminal::new_optimistic(120);
    let theme = ResolvedTheme::from_cli(cli, &term);
    let mut page = apply_cli_layout_flags(
        DarkmatterPage::new(&term)
            .with_prose_theme(theme.prose.kebab_name())
            .with_code_theme(theme.code.kebab_name())
            .with_color_mode(theme.color_mode),
        cli,
    );

    page = apply_style_frontmatter(page, md, cli, input_path)?;

    let content = page
        .render_to_browser_document(md)
        .context("Failed to convert to HTML")?;

    Ok(OutputArtifact {
        content,
        extension: "html",
        label: "html",
    })
}

/// Build a MarkdownPlus artifact.
pub fn markdown_plus_artifact(
    md: &Markdown,
    cli: &Cli,
    input_path: Option<&PathBuf>,
) -> Result<OutputArtifact> {
    let term = Terminal::new_optimistic(120);
    let theme = ResolvedTheme::from_cli(cli, &term);
    let mut page = apply_cli_layout_flags(
        DarkmatterPage::new(&term)
            .with_prose_theme(theme.prose.kebab_name())
            .with_code_theme(theme.code.kebab_name())
            .with_color_mode(theme.color_mode),
        cli,
    );

    page = apply_style_frontmatter(page, md, cli, input_path)?;

    let content = page
        .render_to_markdown_plus(md)
        .context("Failed to convert to MarkdownPlus")?;

    Ok(OutputArtifact {
        content,
        extension: "md",
        label: "markdown-plus",
    })
}

/// Build a JSON artifact from the render-tree document.
pub fn json_artifact(md: &Markdown) -> Result<OutputArtifact> {
    let document = md.as_document().context("Failed to fold to render tree document")?;
    let content = serde_json::to_string_pretty(&document)?;
    Ok(OutputArtifact {
        content,
        extension: "json",
        label: "json",
    })
}

/// Print an artifact to stdout, or open it in the system default app when
/// `--show` is set.
pub fn emit_or_show_artifact(artifact: OutputArtifact, show: bool) -> Result<()> {
    if show {
        open_output_artifact(&artifact)
    } else {
        println!("{}", artifact.content);
        Ok(())
    }
}

/// Open an artifact in the system default application via a temporary file.
pub fn open_output_artifact(artifact: &OutputArtifact) -> Result<()> {
    let temp_path = write_output_artifact_file(artifact)?;

    // MD_DRY_RUN=1 writes the temp file but skips launching the viewer.
    // Useful in tests and CI where opening a GUI app is undesirable.
    if std::env::var("MD_DRY_RUN").is_ok_and(|v| !v.is_empty()) {
        return Ok(());
    }

    // Non-blocking open, graceful error handling
    if let Err(error) = open::that(&temp_path) {
        eprintln!("Failed to open {} output: {}", artifact.label, error);
        eprintln!("Preview available at: {}", temp_path.display());
    }

    Ok(())
}

fn write_output_artifact_file(artifact: &OutputArtifact) -> Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let filename = format!(
        "md-output-{}-{}.{}",
        std::process::id(),
        timestamp,
        artifact.extension
    );
    let path = std::env::temp_dir().join(filename);

    std::fs::write(&path, &artifact.content)
        .wrap_err_with(|| format!("Failed to write {} output file", artifact.label))?;

    Ok(path)
}
