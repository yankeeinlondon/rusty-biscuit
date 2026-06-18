//! Normalization compose stage (Inline Post in phase terms).

use super::super::super::Markdown;
use super::super::super::normalize::{self, NormalizationError};
use tracing::debug;

/// Runs the normalization stage.
///
/// Uses `None` as target level, which means headings are not re-leveled but the
/// document structure is validated.
pub(crate) fn run_stage(
    markdown: &mut Markdown,
) -> Result<normalize::NormalizationReport, NormalizationError> {
    debug!("compose: running normalization");
    let (new_content, report) = normalize::normalize(markdown.content(), None)?;
    *markdown.content_mut() = new_content;
    Ok(report)
}
