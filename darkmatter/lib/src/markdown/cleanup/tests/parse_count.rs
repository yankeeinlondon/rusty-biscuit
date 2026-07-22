//! Acceptance criterion 15's structural half: the number of Markdown parses
//! each cleanup entry point performs.
//!
//! AC 15 caps parse counts, not wall-clock time. Timing is host-dependent and
//! needs a quiet machine; parse count is exact and holds under any load, so it
//! is asserted here rather than inferred from a benchmark.
//!
//! The budget:
//!
//! | Entry point | Parses |
//! |---|---|
//! | `cleanup_content` and its indent/spacing variants | 1 |
//! | `strip_incidental_newlines` | 1 |
//! | `reflow_to_width` | 1 |
//! | `cleanup_to_fixed_width` (strip + reflow) | 2 |
//! | `cleanup_content` then `reflow_to_width` (`md clean --fixed-width`) | 2 |
//!
//! Counts come from `cleanup::parse_count`, which tallies the single
//! constructor every cleanup-path parse is routed through. The fixtures are the
//! four classes the specification's performance section names, so a count that
//! varied by document shape would surface as a failure rather than as an
//! untested branch.

use super::*;
use crate::markdown::cleanup::parse_count::measure;

/// Top-level prose only: the baseline shape, no list containers involved.
const PROSE: &str = "\
# Title

A paragraph whose prose is wrapped across
two source lines, then continued onto
a third.

Another paragraph, also wrapped
across source lines.
";

/// Flat unordered list with wrapped item prose.
const FLAT_LIST: &str = "\
- flat item one whose prose is wrapped
  across two source lines
- flat item two
- flat item three whose prose is wrapped
  across two source lines
";

/// Deeply nested list, each level carrying wrapped prose.
const NESTED_LIST: &str = "\
- level one whose prose is wrapped
  across two source lines
  - level two whose prose is wrapped
    across two source lines
    - level three whose prose is wrapped
      across two source lines
      - level four whose prose is wrapped
        across two source lines
";

/// Blockquoted task list: composite container prefixes plus task boxes.
const BLOCKQUOTED_TASKS: &str = "\
> - [ ] quoted task whose prose is wrapped
>   across two source lines
> - [x] quoted done task whose prose is wrapped
>   across two source lines
>   - [ ] nested quoted task whose prose is wrapped
>     across two source lines
";

/// Reference definitions never reach the event stream, so the reflow map reads
/// them off the offset iterator's definition table. This fixture is what proves
/// that read costs no second parse.
const WITH_REFERENCE_DEFINITIONS: &str = "\
- item referencing [one][first] whose prose is wrapped
  across two source lines
- item referencing [two][second]

[first]: https://example.com/first \"First\"
[second]: https://example.com/second \"Second\"
";

/// The four fixture classes the specification's performance section requires.
const CLASSES: [(&str, &str); 4] = [
    ("prose", PROSE),
    ("flat_list", FLAT_LIST),
    ("nested_list", NESTED_LIST),
    ("blockquoted_tasks", BLOCKQUOTED_TASKS),
];

#[test]
fn default_cleanup_parses_once() {
    for (label, fixture) in CLASSES {
        let (_, parses) = measure(|| cleanup_content(fixture));
        assert_eq!(parses, 1, "default cleanup of {label} must reuse its single parse");
    }
}

#[test]
fn default_cleanup_parses_once_for_every_indent_and_spacing_variant() {
    for (label, fixture) in CLASSES {
        for (variant, parses) in [
            ("compact", measure(|| cleanup_content_compact(fixture)).1),
            ("loose", measure(|| cleanup_content_loose(fixture)).1),
            ("indent", measure(|| cleanup_content_with_indent(fixture, 4)).1),
            (
                "indent_compact",
                measure(|| cleanup_content_with_indent_compact(fixture, 4)).1,
            ),
            (
                "indent_loose",
                measure(|| cleanup_content_with_indent_loose(fixture, 4)).1,
            ),
            (
                "indent_preserve_incidental",
                measure(|| cleanup_content_with_indent_preserving_incidental(fixture, 4)).1,
            ),
            (
                "indent_compact_preserve_incidental",
                measure(|| cleanup_content_with_indent_compact_preserving_incidental(fixture, 4)).1,
            ),
            (
                "indent_loose_preserve_incidental",
                measure(|| cleanup_content_with_indent_loose_preserving_incidental(fixture, 4)).1,
            ),
        ] {
            assert_eq!(
                parses, 1,
                "{variant} cleanup of {label} must reuse its single parse",
            );
        }
    }
}

#[test]
fn strip_incidental_newlines_parses_once() {
    for (label, fixture) in CLASSES {
        let (_, parses) = measure(|| strip_incidental_newlines(fixture));
        assert_eq!(parses, 1, "stripping {label} must parse once");
    }
}

#[test]
fn reflow_to_width_parses_once() {
    for (label, fixture) in CLASSES {
        let (_, parses) = measure(|| reflow_to_width(fixture, 60));
        assert_eq!(parses, 1, "reflowing {label} must parse once");
    }
}

#[test]
fn fixed_width_cleanup_parses_twice() {
    for (label, fixture) in CLASSES {
        let (_, parses) = measure(|| cleanup_to_fixed_width(fixture, 60));
        assert_eq!(
            parses, 2,
            "fixed-width cleanup of {label} is strip-plus-reflow and must add no third parse",
        );
    }
}

/// Mirrors `apply_cleanup` in `darkmatter/cli/src/commands/clean.rs`, which is
/// the sequence `md clean --fixed-width` actually runs.
#[test]
fn cli_fixed_width_sequence_parses_twice() {
    for (label, fixture) in CLASSES {
        let (_, parses) = measure(|| {
            let cleaned = cleanup_content(fixture);
            reflow_to_width(&cleaned, 60)
        });
        assert_eq!(
            parses, 2,
            "the CLI cleanup-plus-reflow sequence on {label} must add no third parse",
        );
    }
}

#[test]
fn reference_definitions_add_no_parse() {
    let (_, cleanup_parses) = measure(|| cleanup_content(WITH_REFERENCE_DEFINITIONS));
    assert_eq!(
        cleanup_parses, 1,
        "reference-definition protection must not cost a parse in cleanup",
    );

    let (_, reflow_parses) = measure(|| reflow_to_width(WITH_REFERENCE_DEFINITIONS, 60));
    assert_eq!(
        reflow_parses, 1,
        "the reflow map must read reference definitions off its existing offset iterator",
    );
}

/// A guard on the guard: the counter has to be able to observe extra parses, or
/// every assertion above passes vacuously.
#[test]
fn counter_observes_repeated_parses() {
    let (_, parses) = measure(|| {
        strip_incidental_newlines(FLAT_LIST);
        strip_incidental_newlines(FLAT_LIST);
        strip_incidental_newlines(FLAT_LIST);
    });
    assert_eq!(parses, 3, "the parse counter must actually count parses");
}
