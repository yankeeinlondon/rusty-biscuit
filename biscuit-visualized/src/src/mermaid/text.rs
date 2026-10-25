//! Merman's text target: which diagrams may be drawn as text, and what one
//! text render ([`MermaidDiagram::render_text`]) reports.
//!
//! Eligibility is read from Merman's own metadata
//! ([`ascii_capabilities`]), never from a list of family names kept here: a
//! family is eligible when its primary text projection is a drawing
//! (`Diagrammatic`), not a report about the diagram (`StructuredText`) or
//! nothing (`None`). A Merman rev bump that changes the metadata changes
//! eligibility with it; the eligibility snapshot test makes that visible in
//! review.
//!
//! [`MermaidDiagram::render_text`]: super::MermaidDiagram::render_text

use merman::ascii::{
    AsciiCapability, AsciiLayoutProfile, AsciiPrimaryProjection, AsciiProjection, AsciiSemanticCoverage,
    ascii_capabilities,
};

/// How much of a family's meaning its text drawing keeps, from Merman's
/// metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextCoverage {
    /// Merman draws every feature of the family.
    Full,
    /// Some features are dropped or approximated (for example a flowchart's
    /// click links).
    Partial,
}

/// A text layout profile of Merman's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextLayout {
    /// The family's ordinary layout.
    Canonical,
    /// A narrower layout of the same diagram.
    Compact,
    /// Canonical first, then compact once when canonical is wider than the
    /// maximum width; one request either way.
    Auto,
}

#[cfg_attr(not(feature = "terminal"), allow(dead_code))]
impl TextLayout {
    /// `None` for a profile this crate does not know (the Merman enum is
    /// `#[non_exhaustive]`).
    fn from_merman(profile: AsciiLayoutProfile) -> Option<Self> {
        match profile {
            AsciiLayoutProfile::Canonical => Some(Self::Canonical),
            AsciiLayoutProfile::Compact => Some(Self::Compact),
            AsciiLayoutProfile::Auto => Some(Self::Auto),
            _ => None,
        }
    }

    fn to_merman(self) -> AsciiLayoutProfile {
        match self {
            Self::Canonical => AsciiLayoutProfile::Canonical,
            Self::Compact => AsciiLayoutProfile::Compact,
            Self::Auto => AsciiLayoutProfile::Auto,
        }
    }
}

/// What a text render produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextProjection {
    /// A drawing of the diagram.
    Diagrammatic,
    /// A report about the diagram (for example a dense class diagram's
    /// `relations:` list), which never replaces the source.
    StructuredText,
}

#[cfg_attr(not(feature = "terminal"), allow(dead_code))]
impl TextProjection {
    fn from_merman(projection: AsciiProjection) -> Option<Self> {
        match projection {
            AsciiProjection::Diagrammatic => Some(Self::Diagrammatic),
            AsciiProjection::StructuredText => Some(Self::StructuredText),
            _ => None,
        }
    }
}

/// A family whose text drawing may replace its source.
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::mermaid::{TextCoverage, text_eligibility};
///
/// let flowchart = text_eligibility("flowchart").expect("flowcharts are drawn as text");
/// assert_eq!(flowchart.coverage(), TextCoverage::Partial);
/// assert!(flowchart.admits_auto_layout());
/// assert!(text_eligibility("pie").is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextEligibility {
    family: &'static str,
    coverage: TextCoverage,
    auto_layout: bool,
}

#[allow(missing_docs)]
impl TextEligibility {
    /// Merman's metadata id for the family (`flowchart`, `er`, `state`, …).
    pub fn family(&self) -> &'static str {
        self.family
    }

    pub fn coverage(&self) -> TextCoverage {
        self.coverage
    }

    /// Whether the family is requested with [`TextLayout::Auto`]; every other
    /// eligible family is requested canonically.
    pub fn admits_auto_layout(&self) -> bool {
        self.auto_layout
    }

    /// The layout a render of this family requests.
    #[cfg_attr(not(feature = "terminal"), allow(dead_code))]
    pub(crate) fn requested_layout(&self) -> TextLayout {
        if self.auto_layout { TextLayout::Auto } else { TextLayout::Canonical }
    }
}

/// The eligibility of the family whose Merman metadata id is `diagram_type`
/// (the `diagram_type` of an [`AsciiCapability`]), or `None` when it may not
/// replace its source: no capability entry, a primary projection other than
/// `Diagrammatic`, or a value this crate does not know.
pub fn text_eligibility(diagram_type: &str) -> Option<TextEligibility> {
    ascii_capabilities()
        .iter()
        .find(|capability| capability.diagram_type == diagram_type)
        .and_then(eligibility_of)
}

/// One capability row's eligibility (D1). A `Diagrammatic` family with no
/// coverage value is partial; an unknown coverage or projection value is not
/// eligible.
pub(crate) fn eligibility_of(capability: &AsciiCapability) -> Option<TextEligibility> {
    if !matches!(capability.primary_projection, AsciiPrimaryProjection::Diagrammatic) {
        return None;
    }
    let coverage = match capability.semantic_coverage {
        Some(AsciiSemanticCoverage::Full) => TextCoverage::Full,
        Some(AsciiSemanticCoverage::Partial) | None => TextCoverage::Partial,
        Some(_) => return None,
    };
    Some(TextEligibility {
        family: capability.diagram_type,
        coverage,
        auto_layout: capability.supports_layout_profile(AsciiLayoutProfile::Auto),
    })
}

/// The characters a text drawing's borders and connectors are made of.
/// Authored labels are kept as written under either.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TextCharset {
    /// Box-drawing characters.
    #[default]
    Unicode,
    /// `+`, `-`, `|` and similar.
    Ascii,
}

/// The settings of one text render.
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::mermaid::{TextCharset, TextOptions};
///
/// let options = TextOptions::new(80).with_charset(TextCharset::Ascii);
/// assert_eq!(options.max_columns(), 80);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextOptions {
    max_columns: u32,
    charset: TextCharset,
}

#[allow(missing_docs)]
impl TextOptions {
    /// Options for a drawing at most `max_columns` terminal cells wide, in
    /// [`TextCharset::Unicode`].
    pub fn new(max_columns: u32) -> Self {
        Self { max_columns, charset: TextCharset::default() }
    }

    pub fn with_charset(mut self, charset: TextCharset) -> Self {
        self.charset = charset;
        self
    }

    pub fn max_columns(&self) -> u32 {
        self.max_columns
    }

    pub fn charset(&self) -> TextCharset {
        self.charset
    }
}

/// What Merman reported for one text request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextReport {
    /// The layout profile requested ([`TextLayout::Auto`] or
    /// [`TextLayout::Canonical`], from the family's eligibility).
    pub requested_layout: TextLayout,
    /// The layout Merman drew; `None` for a profile this crate does not know.
    pub layout: Option<TextLayout>,
    /// What Merman produced; `None` for a projection this crate does not know.
    pub projection: Option<TextProjection>,
    /// The widest row, in terminal display cells.
    pub width: u32,
    /// Whether Merman reported the output wider than the maximum width.
    pub overflowed: bool,
}

/// A text drawing that fits the requested columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDrawing {
    text: String,
    family: &'static str,
    coverage: TextCoverage,
    report: TextReport,
}

#[allow(missing_docs)]
impl TextDrawing {
    /// The drawing's rows, joined by `\n`, with no terminal control
    /// sequences.
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn into_text(self) -> String {
        self.text
    }

    /// The family's coverage; a [`TextCoverage::Partial`] drawing may have
    /// dropped features of the source.
    pub fn coverage(&self) -> TextCoverage {
        self.coverage
    }

    pub fn report(&self) -> &TextReport {
        &self.report
    }

    /// Whether the drawing draws `title` as the diagram's title.
    ///
    /// Of the families drawn as text, only two draw a title, and only from a
    /// `title` statement (no family draws a frontmatter `title:`): a
    /// sequence diagram on its first row, alone, and an XY chart as its
    /// `titleDisplay: chart(bytes=N)="…"` disclosure row. Each is matched
    /// exactly where it is drawn, so a node, lane, participant, edge label,
    /// or axis title that equals `title` does not count. A title Merman
    /// draws escaped (a control character) does not match, so it is shown
    /// twice rather than not at all.
    pub(crate) fn shows_title(&self, title: &str) -> bool {
        let title = title.trim();
        if title.is_empty() {
            return false;
        }
        match self.family {
            "sequence" => self.text.lines().next().is_some_and(|row| row.trim() == title),
            "xychart" => {
                let disclosure = format!("titleDisplay: chart(bytes={})=\"{title}\"", title.len());
                self.text.lines().any(|row| row.trim_end() == disclosure)
            }
            _ => false,
        }
    }
}

/// Why a text render chose the source instead of a drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextSource {
    /// The family may not be drawn as text ([`text_eligibility`]); no request
    /// was made.
    Ineligible,
    /// The drawing has no columns; no request was made.
    NoColumns,
    /// Merman produced something other than a drawing.
    NotDiagrammatic(TextReport),
    /// The drawing is wider than the columns, after the layouts the family
    /// allows.
    TooWide(TextReport),
    /// Merman drew nothing for a source with content.
    Empty(TextReport),
}

/// The outcome of a text render that did not fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextRender {
    /// A drawing to show instead of the source.
    Drawing(TextDrawing),
    /// Show the source, for this reason.
    Source(TextSource),
}

/// Merman's text output judged against `options` (D1, D7): a drawing only
/// when Merman drew a diagram, drew something, and every row fits.
///
/// Label text reaches the terminal only through here, so terminal control
/// characters are neutralized first, and the width is measured after that,
/// in biscuit-terminal's display cells.
#[cfg(feature = "terminal")]
pub(crate) fn judge(output: &merman::ascii::AsciiOutput, eligibility: TextEligibility, options: TextOptions) -> TextRender {
    use biscuit_terminal::utils::block_constraint::visible_width;

    let text = merman::ascii::normalize_terminal_text(&output.text).into_owned();
    let report = TextReport {
        requested_layout: eligibility.requested_layout(),
        layout: TextLayout::from_merman(output.layout_profile),
        projection: TextProjection::from_merman(output.projection),
        width: text.lines().map(visible_width).max().unwrap_or(0),
        overflowed: output.overflowed,
    };
    if report.projection != Some(TextProjection::Diagrammatic) {
        TextRender::Source(TextSource::NotDiagrammatic(report))
    } else if text.trim().is_empty() {
        TextRender::Source(TextSource::Empty(report))
    } else if report.overflowed || report.width > options.max_columns {
        TextRender::Source(TextSource::TooWide(report))
    } else {
        TextRender::Drawing(TextDrawing { text, family: eligibility.family, coverage: eligibility.coverage, report })
    }
}

/// The Merman request options for `eligibility` under `options`: plain
/// output, the family's layout, and the standard (non-CJK) display widths,
/// which are the widths biscuit-terminal measures in.
#[cfg(feature = "terminal")]
pub(crate) fn ascii_request(
    eligibility: TextEligibility,
    options: TextOptions,
    resources: merman::ascii::AsciiResourcePolicy,
) -> merman::AsciiRequest {
    use merman::ascii::{AsciiCharset, AsciiColorMode, AsciiRenderOptions, AsciiViewportPolicy, TerminalWidthProfile};

    let mut render_options = AsciiRenderOptions::default();
    render_options.charset = match options.charset {
        TextCharset::Unicode => AsciiCharset::Unicode,
        TextCharset::Ascii => AsciiCharset::Ascii,
    };
    render_options.layout_profile = eligibility.requested_layout().to_merman();
    render_options.terminal_width_profile = TerminalWidthProfile::Unicode;
    render_options.color_mode = AsciiColorMode::Plain;
    merman::AsciiRequest {
        options: render_options,
        resources,
        viewport: AsciiViewportPolicy {
            max_width: Some(options.max_columns as usize),
            ..AsciiViewportPolicy::default()
        },
    }
}
