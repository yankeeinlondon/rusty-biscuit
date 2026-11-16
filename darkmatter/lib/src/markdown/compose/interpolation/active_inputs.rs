//! The executable inputs of a Markdown body, as opposed to its prose.
//!
//! A child transcluded with `interpolate=false` keeps its prose as written
//! while its directive inputs keep evaluating. Every surface that examines
//! such a child (body interpolation, the passive call check, context-demand
//! discovery, deferred-capability planning, remote-expression discovery)
//! reads the same classification from here, so none of them evaluates, reads,
//! or rejects what another leaves literal.
//!
//! An *active input* is an authored directive line from its keyword on (its
//! target, options, and callback), the
//! option values of a `::disclosure` / `::details` line (whose summary text is
//! displayed, so prose), and a `::shell-block` body. Everything else is
//! prose: headings, paragraphs, lists, tables, inline code, code blocks, and
//! the bodies of page and collection blocks.

use std::ops::Range;

use crate::markdown::compose::block_pairs::{BlockOpenKind, scan_block_pairs_in};
use crate::markdown::compose::body_origin::DataRanges;
use crate::markdown::compose::directives_api::{DirectiveKind, scan_darkmatter_directives_in};

/// The byte ranges of `content` that hold executable directive inputs,
/// sorted by start. Ranges may nest (a directive inside a `::shell-block`
/// body is not a directive, but a reader must not assume disjointness).
///
/// Directives and blocks count only where they are authored: `data` (text an
/// earlier stage inserted) can neither open one nor hide one, exactly as the
/// executing scanners treat it.
pub(crate) fn active_input_ranges(content: &str, data: Option<&DataRanges>) -> Vec<Range<usize>> {
    if !content.contains("::") {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    for directive in scan_darkmatter_directives_in(content, data) {
        match directive.kind {
            // A disclosure's summary is displayed text; only its options are
            // inputs.
            DirectiveKind::Disclosure | DirectiveKind::Details => ranges.extend(
                directive.options.iter().filter_map(|option| option.value.as_ref()).map(|value| value.span.clone()),
            ),
            // Everything from the keyword on is the directive's target,
            // options, and callback. The line is taken whole because a target
            // such as `./{{ lang }}.rs` holds spaces the directive tokenizer
            // splits on, and with its keyword so a masked body still reads as
            // a directive line rather than indented code.
            _ => ranges.push(directive.keyword_span.start..directive.span.end),
        }
    }
    // A malformed block structure fails the executing stage on its own; until
    // then no shell body can be proven, so none is active.
    if let Ok(pairs) = scan_block_pairs_in(content, data) {
        ranges.extend(pairs.into_iter().filter(|pair| pair.kind == BlockOpenKind::Shell).map(|pair| pair.body_span));
    }
    ranges.retain(|range| !range.is_empty());
    ranges.sort_by_key(|range| (range.start, range.end));
    ranges
}

/// Whether `span` overlaps one of `ranges` (as returned by
/// [`active_input_ranges`]).
pub(crate) fn intersects_active(ranges: &[Range<usize>], span: &Range<usize>) -> bool {
    ranges.iter().any(|range| range.start < span.end && span.start < range.end)
}

/// `content` with every prose byte blanked to a space and line breaks kept,
/// so offsets are preserved and only active inputs remain for a passive
/// scanner to read.
///
/// The masked text has lost its Markdown structure (a directive's option
/// value can now sit behind four spaces), so scan it as plain text, never
/// with code-region detection.
pub(crate) fn mask_prose(content: &str, data: Option<&DataRanges>) -> String {
    let ranges = active_input_ranges(content, data);
    let mut masked = content.as_bytes().to_vec();
    let mut keep = vec![false; masked.len()];
    for range in &ranges {
        keep[range.clone()].iter_mut().for_each(|kept| *kept = true);
    }
    for (byte, kept) in masked.iter_mut().zip(keep) {
        if !kept && *byte != b'\n' && *byte != b'\r' {
            *byte = b' ';
        }
    }
    // Every kept range starts and ends on a character boundary, and every
    // other byte became ASCII, so the result is UTF-8.
    String::from_utf8(masked).expect("masking whole characters keeps UTF-8 intact")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text of every active range of `content`, in order.
    fn active(content: &str) -> Vec<&str> {
        active_input_ranges(content, None).into_iter().map(|range| &content[range]).collect()
    }

    #[test]
    fn prose_inline_code_and_code_blocks_hold_no_active_input() {
        let content = "# Title {{ a }}\n\nUse `{{ b }}` here.\n\n```md\n::file {{ c }}\n```\n\n    ::shell echo {{ d }}\n\n| {{ e }} |\n|---|\n";
        assert_eq!(active(content), Vec::<&str>::new());
        assert_eq!(mask_prose(content, None).trim(), "", "nothing survives masking");
    }

    #[test]
    fn file_target_and_every_option_value_are_active() {
        let content = "Intro {{ x }}\n::file {{ target }} set.name=\"{{ name }}\" when=\"{{ show }}\" exclude=\"{{ section }}\"\nOutro\n";
        assert_eq!(
            active(content),
            ["::file {{ target }} set.name=\"{{ name }}\" when=\"{{ show }}\" exclude=\"{{ section }}\""]
        );
    }

    #[test]
    fn code_and_url_targets_and_options_are_active() {
        let content = "::code ./{{ lang }}.rs lines=\"{{ range }}\"\n::url https://example.com/{{ page }}.md\n";
        assert_eq!(active(content), ["::code ./{{ lang }}.rs lines=\"{{ range }}\"", "::url https://example.com/{{ page }}.md"]);
    }

    #[test]
    fn shell_command_is_active() {
        let content = "Run it:\n::shell echo {{ greeting }}\n";
        assert_eq!(active(content), ["::shell echo {{ greeting }}"]);
    }

    #[test]
    fn shell_block_body_is_active_and_page_block_body_is_prose() {
        let content = "::shell-block\necho {{ a }}\n::end-block\n::block when=\"{{ show }}\"\nExample: {{ name }}\n::end-block\n";
        assert_eq!(active(content), ["::shell-block", "echo {{ a }}\n", "::end-block", "::block when=\"{{ show }}\"", "::end-block"]);
    }

    #[test]
    fn collection_source_and_callback_are_active_and_the_body_is_prose() {
        let content = "::map {{ items }} \"item => item.name\"\n- {{ item }}\n::end-map\n::filter {{ rows }} \"row => row.ok\"\n{{ row }}\n::end-filter\n";
        assert_eq!(active(content), ["::map {{ items }} \"item => item.name\"", "::end-map", "::filter {{ rows }} \"row => row.ok\"", "::end-filter"]);
    }

    #[test]
    fn a_directive_inside_a_block_body_keeps_its_inputs_active() {
        let content = "::block when=\"show\"\nExample: {{ name }}\n::file {{ detail_file }} set.name=\"{{ name }}\"\n::end-block\n";
        assert_eq!(active(content), ["::block when=\"show\"", "::file {{ detail_file }} set.name=\"{{ name }}\"", "::end-block"]);
        let masked = mask_prose(content, None);
        assert!(!masked.contains("Example"), "the block's prose is masked: {masked:?}");
        assert!(masked.contains("{{ detail_file }}") && masked.contains("\"{{ name }}\""));
        assert_eq!(masked.len(), content.len(), "offsets are preserved");
        assert_eq!(masked.lines().count(), content.lines().count(), "line breaks are kept");
    }

    #[test]
    fn data_cannot_open_a_directive() {
        let content = "Value:\n::file {{ x }}\n";
        let start = content.find("::file").unwrap();
        let data = DataRanges::covering(content, std::iter::once(start..content.len()).collect());
        assert!(active_input_ranges(content, Some(&data)).is_empty());
        assert_eq!(active_input_ranges(content, None).len(), 1, "the control: authored, it is active");
    }

    #[test]
    fn masking_keeps_multibyte_prose_valid() {
        let content = "Café {{ ü }} — ok\n::shell echo {{ ñ }}\n";
        let masked = mask_prose(content, None);
        assert_eq!(masked.len(), content.len());
        assert!(masked.ends_with("echo {{ ñ }}\n"));
        assert!(!masked.contains('ü'));
    }

    #[test]
    fn intersection_is_half_open() {
        let ranges = [5..10, 20..30];
        assert!(intersects_active(&ranges, &(9..12)));
        assert!(intersects_active(&ranges, &(19..21)));
        assert!(!intersects_active(&ranges, &(10..20)));
        assert!(!intersects_active(&ranges, &(0..5)));
    }
}
