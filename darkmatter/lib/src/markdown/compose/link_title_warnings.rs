//! The located warnings of a document's link titles.
//!
//! A title's notice is found on the body as it stands, so a link a stage wrote
//! is seen too, and is reported in authored coordinates: at the construct that
//! wrote the title when a stage did, marked inserted; otherwise at the link when
//! its first character is authored, or at the construct that wrote it. A
//! document with no file behind it has no authored position to report.

use super::body_origin::BodyOrigin;
use super::context::report::ComposeWarning;
use super::ComposeSource;
use crate::markdown::Markdown;
use crate::markdown::cleanup::link_titles;
use crate::markdown::reference::AuthoredLocator;

/// The warnings of `markdown`'s current body. `origin` maps that body to the
/// loaded text; without a map that describes it the warnings carry no position.
pub(crate) fn link_title_warnings(markdown: &Markdown, origin: Option<&BodyOrigin>) -> Vec<ComposeWarning> {
    let body = markdown.content();
    let findings = link_titles::scan(body);
    if findings.is_empty() {
        return Vec::new();
    }
    let origin = origin.filter(|origin| origin.describes(body));
    let locator = origin
        .zip(markdown.source().as_ref())
        .and_then(|(origin, source)| AuthoredLocator::new(markdown, source, body, Some(origin)));
    let file = match markdown.source() {
        Some(ComposeSource::File(path)) => Some(path.clone()),
        _ => None,
    };
    findings
        .iter()
        .map(|finding| {
            let first = finding.span.start..finding.span.start + 1;
            let start = origin
                .and_then(|origin| origin.project(&first).or_else(|| origin.producer_of(finding.span.start)))
                .map_or(finding.span.start, |authored| authored.start);
            let location = file.clone().zip(locator.as_ref().and_then(|locator| {
                let written_title = finding.title.as_ref().and_then(|title| locator.produced_location(title));
                written_title.or_else(|| locator.body_location(&finding.span))
            }));
            ComposeWarning::from_link_title(&finding.notice, &finding.name, start, location)
        })
        .collect()
}
