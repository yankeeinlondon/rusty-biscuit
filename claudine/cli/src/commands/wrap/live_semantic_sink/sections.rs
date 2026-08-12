//! 9-section model emitters.

use super::LiveSemanticSink;
use super::Section;

impl LiveSemanticSink {
    /// Section-aware stderr emit used by every status render path.
    ///
    /// Delegates to [`SectionTracker::classify`] for the dedup and
    /// section-transition logic so this sink stays in sync with the
    /// [`super::section::SectionStream`] reference implementation.
    ///
    /// ## Rules
    /// - Section transitions are separated by exactly one blank line.
    /// - Consecutive blank lines inside a section collapse to one.
    /// - No leading blank line is emitted before the first rendered line.
    /// - The section-transition separator is suppressed when the combined
    ///   output is already at a visual blank row (stdout ended with `\n\n`,
    ///   or the last stderr line was blank), so injecting another blank
    ///   would produce a doubled blank line.
    pub(crate) fn emit_section_line(&mut self, section: Section, line: &str) {
        let result = {
            let mut tracker = self
                .section_tracker
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            tracker.classify(section, line)
        };
        if let Some((needs_separator, _)) = result {
            if needs_separator && !self.at_visual_blank() {
                (self.emit_stderr)("");
                self.at_blank_row = true;
            }
            (self.emit_stderr)(line);
            if !line.trim().is_empty() {
                self.stdout_trailing_newlines = 0;
                self.at_blank_row = false;
            } else {
                self.at_blank_row = true;
            }
        }
    }

    /// Emit a line tagged as [`Section::TrailerMetadata`], using the
    /// section tracker for spacing. Intended for post-execution summary
    /// lines that belong in the trailer section (cost, duration, tool
    /// rollup).
    pub(crate) fn emit_trailer_line(&mut self, line: &str) {
        self.emit_section_line(Section::TrailerMetadata, line);
    }
}
