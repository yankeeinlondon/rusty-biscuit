//! The number of Markdown parses each cleanup entry point performs.
//!
//! Parse count, not wall-clock time, is the budget: it is exact and holds
//! under any load. Two counters are kept apart so the cleanup budget cannot
//! hide repeated scanning:
//!
//! - **cleanup parses**, through `cleanup::cleanup_parser`;
//! - **scanner parses**, through `compose::parse_utils::scan_code_regions`,
//!   which every directive and expression recognizer derives its code
//!   regions from.
//!
//! | Entry point | Cleanup parses | Scanner parses |
//! |---|---|---|
//! | `clean_body` / `cleanup_content*`, unchanged or malformed input | 1 (0 when malformed) | 1 |
//! | `clean_body` / `cleanup_content*`, changed | 2 | 2 |
//! | the same with `fixed-width` after a first-stage edit | at most 3 | at most 3 |
//! | `clean_document` | the body's counts | the body's counts |
//! | `strip_incidental_newlines` (standalone, unguarded) | 1 | 1 |
//! | `reflow_to_width` (standalone, unguarded) | 1 | 1 |
//!
//! The fixtures are the four classes the specification's performance section
//! names, so a count that varied by document shape would surface as a
//! failure rather than as an untested branch.

use super::*;
use crate::markdown::cleanup::rules::{
    FixedWidth, Indentation, Leaf, ListItemPadding, MarkdownRuleOverrides, ResolvedMarkdownRules, resolve,
};
use crate::markdown::cleanup::parse_count::measure;
use crate::markdown::compose::parse_utils::scanner_parse_count;

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

/// Runs `operation` and returns its cleanup and scanner parse counts.
fn counts<T>(operation: impl FnOnce() -> T) -> (T, usize, usize) {
    let ((value, cleanup), scanner) = scanner_parse_count::measure(|| measure(operation));
    (value, cleanup, scanner)
}

fn overrides_with(edit: impl FnOnce(&mut MarkdownRuleOverrides)) -> MarkdownRuleOverrides {
    let mut overrides = MarkdownRuleOverrides::default();
    edit(&mut overrides);
    overrides
}

#[test]
fn every_wrapper_stays_within_the_guarded_body_budget() {
    let variants = [
        ("defaults", MarkdownRuleOverrides::default()),
        ("compact", overrides_with(|o| o.list_item_padding = Leaf::Value(ListItemPadding::None))),
        ("loose", overrides_with(|o| o.list_item_padding = Leaf::Value(ListItemPadding::One))),
        ("indent 2", overrides_with(|o| o.indentation = Leaf::Value(Indentation::Two))),
        ("indent 8", overrides_with(|o| o.indentation = Leaf::Value(Indentation::Eight))),
        ("unwrap", overrides_with(|o| o.unwrap = Leaf::Value(true))),
        ("all off", MarkdownRuleOverrides::all_body_rules_off()),
    ];
    for (label, fixture) in CLASSES {
        for (variant, overrides) in &variants {
            let (cleaned, cleanup, scanner) = counts(|| cleanup_content_with(fixture, overrides));
            let changed = cleaned != fixture;
            let ceiling = if changed { 2 } else { 1 };
            assert_eq!(cleanup, ceiling, "{variant} cleanup of {label}: cleanup parses");
            assert_eq!(scanner, ceiling, "{variant} cleanup of {label}: scanner parses");
        }
    }
}

#[test]
fn default_cleanup_of_an_unchanged_body_short_circuits_verification() {
    let (cleaned, cleanup, scanner) = counts(|| cleanup_content(PROSE));
    assert_eq!(cleaned, PROSE);
    assert_eq!((cleanup, scanner), (1, 1), "no edit means no verification parse");
}

#[test]
fn a_malformed_region_declines_before_any_cleanup_parse() {
    let malformed = "::block when=\"x\"\n- a\n";
    let (cleaned, cleanup, scanner) = counts(|| clean_body(malformed, &ResolvedMarkdownRules::defaults()));
    assert_eq!(cleaned.outcome, CleanOutcome::Declined);
    assert_eq!((cleanup, scanner), (0, 1));
}

#[test]
fn fixed_width_cleanup_verifies_after_rewrapping_within_three_parses() {
    let width = overrides_with(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(30)));
    for (label, fixture) in CLASSES {
        let (cleaned, cleanup, scanner) = counts(|| clean_body(fixture, &resolve(&width, None, None)));
        assert_eq!(cleaned.outcome, CleanOutcome::Changed, "{label}: {:?}", cleaned.diagnostics);
        assert!((2..=3).contains(&cleanup), "{label}: {cleanup} cleanup parses");
        assert_eq!(scanner, cleanup, "{label}: one scan per text scanned");
    }
}

#[test]
fn clean_document_adds_no_parse_to_the_body() {
    let source = format!("---\ntitle: Doc\n---\n{NESTED_LIST}");
    let overrides = MarkdownRuleOverrides::default();
    let request = CleanRequest { source: &source, overrides: &overrides, workspace: None, document_path: None };
    let config = || {
        Ok::<_, std::convert::Infallible>(
            crate::markdown::schemas::CleanSchemaConfig::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
                .without_baseline_schema(),
        )
    };
    let (result, cleanup, scanner) = counts(|| clean_document(&request, config).expect("clean"));
    assert_eq!(result.outcome, CleanOutcome::Changed);
    assert_eq!((cleanup, scanner), (2, 2));
}

#[test]
fn strip_incidental_newlines_parses_once() {
    for (label, fixture) in CLASSES {
        let (_, cleanup, scanner) = counts(|| strip_incidental_newlines(fixture));
        assert_eq!((cleanup, scanner), (1, 1), "stripping {label}");
    }
}

#[test]
fn reflow_to_width_parses_once() {
    for (label, fixture) in CLASSES {
        let (_, cleanup, scanner) = counts(|| reflow_to_width(fixture, 60));
        assert_eq!((cleanup, scanner), (1, 1), "reflowing {label}");
    }
}

#[test]
fn reference_definitions_add_no_parse() {
    let (_, cleanup, scanner) = counts(|| cleanup_content(WITH_REFERENCE_DEFINITIONS));
    assert!(cleanup <= 2 && scanner == cleanup, "{cleanup}/{scanner}");

    let (_, reflow_parses) = measure(|| reflow_to_width(WITH_REFERENCE_DEFINITIONS, 60));
    assert_eq!(
        reflow_parses, 1,
        "the reflow map must read reference definitions off its existing offset iterator",
    );
}

/// A guard on the guard: both counters have to be able to observe extra
/// parses, or every assertion above passes vacuously.
#[test]
fn counters_observe_repeated_parses() {
    let (_, cleanup, scanner) = counts(|| {
        strip_incidental_newlines(FLAT_LIST);
        strip_incidental_newlines(FLAT_LIST);
        strip_incidental_newlines(FLAT_LIST);
    });
    assert_eq!((cleanup, scanner), (3, 3), "the parse counters must actually count parses");
}

mod guarded_body {
    use super::{CLASSES, measure};
    use crate::markdown::cleanup::rules::{FixedWidth, Leaf, MarkdownRuleOverrides, ResolvedMarkdownRules, resolve};
    use crate::markdown::cleanup::{CleanOutcome, clean_body};

    fn width(columns: u16, body_rules_off: bool) -> ResolvedMarkdownRules {
        let mut overrides =
            if body_rules_off { MarkdownRuleOverrides::all_body_rules_off() } else { MarkdownRuleOverrides::default() };
        overrides.fixed_width = Leaf::Value(FixedWidth::Columns(columns));
        resolve(&overrides, None, None)
    }

    #[test]
    fn an_unchanged_body_is_parsed_once() {
        let rules = ResolvedMarkdownRules::all_body_rules_off();
        for (label, fixture) in CLASSES {
            let (cleaned, parses) = measure(|| clean_body(fixture, &rules));
            assert_eq!(cleaned.outcome, CleanOutcome::Unchanged, "{label}");
            assert_eq!(parses, 1, "{label}: an unchanged body needs no verification parse");
        }
    }

    #[test]
    fn a_changed_body_is_parsed_twice() {
        // The planning parse doubles as the input side of the comparison; the
        // second parse verifies the final candidate.
        let (cleaned, parses) = measure(|| clean_body(super::NESTED_LIST, &ResolvedMarkdownRules::defaults()));
        assert_eq!(cleaned.outcome, CleanOutcome::Changed);
        assert_eq!(parses, 2);
    }

    #[test]
    fn rewrapping_adds_at_most_one_parse_and_verifies_after_it() {
        for (label, fixture) in CLASSES {
            let (cleaned, parses) = measure(|| clean_body(fixture, &width(30, false)));
            assert_ne!(cleaned.outcome, CleanOutcome::Declined, "{label}: {:?}", cleaned.diagnostics);
            assert!(parses <= 3, "{label}: {parses} parses");
        }
        // With no first-stage edit (one long line, nothing to join) the input
        // parse serves the re-wrap stage.
        let long_line = "one two three four five six seven eight nine ten eleven twelve\n";
        let (cleaned, parses) = measure(|| clean_body(long_line, &width(30, true)));
        assert_eq!(cleaned.outcome, CleanOutcome::Changed);
        assert_eq!(parses, 2);
    }
}
