//! Output artifact plumbing for the `md` CLI: the page behind HTML and
//! MarkdownPlus output, and the temporary files `--show` opens.

use crate::render::{PageInputs, PagePresentation, apply_features, apply_presentation};
use biscuit_terminal::output::OutputTarget;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::layout::DarkmatterPage;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// A rendered output on its way to a file.
#[derive(Debug)]
pub struct OutputArtifact {
    pub content: String,
    pub extension: &'static str,
    pub label: &'static str,
}

/// Build the page that renders HTML and MarkdownPlus output: the effective
/// style, then `features`.
///
/// Only this page reads a local `style.page.stylesheet`; it resolves in the
/// context of the layer that supplied it.
///
/// ## Errors
///
/// The style cannot be applied, or a local stylesheet cannot be read.
pub fn html_page(inputs: &PageInputs, presentation: &PagePresentation<'_>) -> Result<DarkmatterPage> {
    // A browser page has no terminal; this one only seeds the page's defaults.
    let term = Terminal::new_optimistic(120);
    let page = DarkmatterPage::new(&term).with_color_mode(term.color_mode());
    let page = apply_presentation(page, inputs, presentation, OutputTarget::Html)?;
    Ok(apply_features(page, presentation.features))
}

/// How an artifact reaches the host's default application.
pub trait ArtifactOpener {
    /// Opens `path`.
    ///
    /// ## Errors
    ///
    /// The host could not open it (for example, no default application).
    fn open(&self, path: &Path) -> std::io::Result<()>;
}

/// The running process's opener.
///
/// `MD_DRY_RUN` stops any application launching, so a test never takes focus:
/// any value succeeds without opening anything, and `fail` reports what a host
/// with no default application reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOpener {
    dry_run: Option<String>,
}

impl ProcessOpener {
    /// The opener this process's environment selects.
    #[must_use]
    pub fn detect() -> Self {
        Self { dry_run: std::env::var("MD_DRY_RUN").ok().filter(|value| !value.is_empty()) }
    }
}

impl ArtifactOpener for ProcessOpener {
    fn open(&self, path: &Path) -> std::io::Result<()> {
        match self.dry_run.as_deref() {
            Some("fail") => Err(std::io::Error::other("no default application")),
            Some(_) => Ok(()),
            None => open::that(path),
        }
    }
}

/// Open an artifact in the system default application via a temporary file.
///
/// ## Errors
///
/// The file could not be written, or the host could not open it; the error
/// names the file so the output is still reachable.
pub fn open_output_artifact(artifact: &OutputArtifact) -> Result<()> {
    open_artifact_with(artifact, &ProcessOpener::detect())
}

/// [`open_output_artifact`] through `opener`.
///
/// ## Errors
///
/// See [`open_output_artifact`].
pub fn open_artifact_with(artifact: &OutputArtifact, opener: &dyn ArtifactOpener) -> Result<()> {
    let temp_path = write_output_artifact_file(artifact)?;
    opener.open(&temp_path).map_err(|error| {
        eyre!("Failed to open {} output at {}: {error}", artifact.label, temp_path.display())
    })
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
