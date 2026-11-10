//! Components rendered through `Terminal::from_output_options`: under a width
//! with no columns nothing fills and nothing wraps, and `Columns` is the control
//! that shows each component's fill or wrap policy is the one being switched off.

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::filesystem::FileSystem;
use biscuit_terminal::components::horizontal_rule::{HorizontalRule, RuleAlignment};
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::metrics_tree::{MetricNode, MetricShare, MetricValue, MetricsTree};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::Status;
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::terminal::{Terminal, UnboundedWidth};
use biscuit_terminal::utils::block_constraint::visible_width;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use biscuit_terminal::utils::layout::{Alignment, Edges, Length, WordWrap};
use renderable::{OutputOptions, TerminalProfile, Width};
use std::time::Duration;

const SENTENCE: &str = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen";

const UNBOUNDED: [Width; 2] = [Width::Source, Width::Unlimited];

fn plain(width: Width) -> Terminal {
    Terminal::from_output_options(&OutputOptions::new(width, TerminalProfile::plain()))
}

/// A color terminal that draws no images, so a rule stays text at every width.
fn styled(width: Width) -> Terminal {
    let mut profile = Terminal::new_optimistic(80).output_profile();
    profile.graphics = false;
    Terminal::from_output_options(&OutputOptions::new(width, profile))
}

/// Whether `text` holds a `CSI <n> G`.
fn moves_the_cursor_to_a_column(text: &str) -> bool {
    text.split("\u{1b}[").skip(1).any(|sequence| {
        let digits = sequence.chars().take_while(char::is_ascii_digit).count();
        digits > 0 && sequence[digits..].starts_with('G')
    })
}

fn widest_line(text: &str) -> u32 {
    text.lines().map(visible_width).max().unwrap_or(0)
}

#[test]
fn a_rule_fills_columns_exactly() {
    let text = HorizontalRule::new().render(&plain(Width::Columns(40)));
    assert_eq!(text.chars().count(), 40);
    assert_eq!(text.len(), 120);
    assert_eq!(text, "╌".repeat(40));
}

#[test]
fn a_plain_rule_with_no_width_is_three_dashes() {
    for width in UNBOUNDED {
        assert_eq!(HorizontalRule::new().render(&plain(width)), "---", "{width:?}");
    }
}

#[test]
fn a_styled_rule_with_no_width_is_three_glyphs_and_never_an_image() {
    let mut profile = Terminal::new_optimistic(80).output_profile();
    assert!(profile.graphics, "this terminal would otherwise draw an image");
    profile.is_tty = true;
    for width in UNBOUNDED {
        let terminal = Terminal::from_output_options(&OutputOptions::new(width, profile.clone()));
        assert_eq!(strip_escape_codes(HorizontalRule::new().render(&terminal)), "╌╌╌", "{width:?}");
    }
    assert_eq!(strip_escape_codes(HorizontalRule::new().render(&styled(Width::Columns(40)))), "╌".repeat(40));
}

#[test]
fn an_aligned_rule_with_no_width_has_no_slack_to_place_it_in() {
    for alignment in [RuleAlignment::Right, RuleAlignment::Centered, RuleAlignment::Left, RuleAlignment::Full] {
        let rule = HorizontalRule::new().alignment(alignment);
        assert_eq!(rule.render(&plain(Width::Unlimited)), "---");
        assert_eq!(strip_escape_codes(rule.render(&styled(Width::Unlimited))), "╌╌╌");
    }
    let right = HorizontalRule::new().alignment(RuleAlignment::Right).width("10ch".to_string());
    assert_eq!(right.render(&plain(Width::Columns(40))), format!("{}{}", " ".repeat(30), "╌".repeat(10)));
}

#[test]
fn a_rule_keeps_its_left_margin_with_no_width() {
    let mut rule = HorizontalRule::new();
    rule.layout_mut().margin = Edges::x(Length::ch(2));
    assert_eq!(rule.render(&plain(Width::Unlimited)), "  ---");
}

#[test]
fn a_table_hugs_its_content_with_no_width_and_fills_nothing() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Note")])
        .with_data(vec![vec![TableCellContent::from("alpha"), TableCellContent::from(SENTENCE)]]);
    let natural = table.natural_render_width();
    assert_eq!(natural as usize, "│ alpha │  │".chars().count() + SENTENCE.chars().count());
    for width in UNBOUNDED {
        let text = table.render(&plain(width));
        assert_eq!(widest_line(&text), natural, "{width:?}:\n{text}");
        assert!(text.contains(&format!("│ alpha │ {SENTENCE} │")), "{width:?}:\n{text}");
    }
    let bounded = table.render(&plain(Width::Columns(40)));
    assert!(widest_line(&bounded) <= 40, "{bounded}");
    assert!(!bounded.contains(SENTENCE), "the control wraps its cell:\n{bounded}");
}

#[test]
fn a_cursor_aligned_table_with_no_width_has_no_column_to_move_to() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Name")])
        .with_data(vec![vec![TableCellContent::from(SENTENCE)]])
        .prefer_cursor_alignment();
    let styled_text = table.render(&styled(Width::Unlimited));
    assert!(strip_escape_codes(styled_text.clone()).contains(SENTENCE));
    assert!(!moves_the_cursor_to_a_column(&styled_text), "{styled_text:?}");
    assert!(moves_the_cursor_to_a_column(&table.render(&styled(Width::Columns(120)))), "the control does");
}

#[test]
fn a_table_plan_for_a_terminal_with_no_width_wraps_nothing() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Note")])
        .with_data(vec![vec![TableCellContent::from(SENTENCE)]]);
    let plan = table.plan_widths_for_terminal(&plain(Width::Unlimited)).expect("a natural plan");
    assert_eq!(plan.table_width, SENTENCE.chars().count() + "│  │".chars().count());
    let bounded = table.plan_widths_for_terminal(&plain(Width::Columns(30))).expect("a plan");
    assert!(bounded.table_width <= 30);
}

#[test]
fn wrapping_prose_stays_on_its_line_with_no_width() {
    let prose = Prose::new(SENTENCE).with_word_wrap(WordWrap::WrapProse(None, None));
    for width in UNBOUNDED {
        assert_eq!(prose.render(&plain(width)), SENTENCE, "{width:?}");
    }
    let bounded = prose.render(&plain(Width::Columns(40)));
    assert!(bounded.lines().count() > 1 && widest_line(&bounded) <= 40, "{bounded}");
}

#[test]
fn source_keeps_a_soft_break_and_unlimited_joins_it() {
    let prose = Prose::new("first\nsecond");
    assert_eq!(prose.render(&plain(Width::Source)), "first\nsecond");
    assert_eq!(prose.render(&plain(Width::Unlimited)), "first second");
}

#[test]
fn an_aligned_paragraph_with_no_width_is_not_padded() {
    let mut prose = Prose::new("short");
    prose.layout_mut().alignment = Alignment::Right;
    assert_eq!(prose.render(&plain(Width::Unlimited)), "short");
    assert_eq!(prose.render(&plain(Width::Columns(40))), format!("{}short", " ".repeat(35)));
}

#[test]
fn a_status_description_stays_on_its_line_with_no_width() {
    let status = Status::from_prose(SENTENCE);
    for width in UNBOUNDED {
        let text = status.render(&plain(width));
        assert_eq!(text.lines().count(), 1, "{width:?}:\n{text}");
        assert!(text.ends_with(SENTENCE), "{width:?}:\n{text}");
    }
    assert!(status.render(&plain(Width::Columns(40))).lines().count() > 1);
}

#[test]
fn a_block_quote_stays_on_its_line_with_no_width() {
    let quote = BlockQuote::from(SENTENCE);
    for width in UNBOUNDED {
        assert_eq!(strip_escape_codes(quote.render(&plain(width))), format!("│ {SENTENCE}"), "{width:?}");
    }
    assert!(quote.render(&plain(Width::Columns(40))).lines().count() > 1);
}

#[test]
fn a_custom_border_quote_stays_on_its_line_with_no_width() {
    let mut quote = BlockQuote::from(SENTENCE).with_border("> ");
    quote.layout_mut().word_wrap = WordWrap::WrapProse(None, None);
    for width in UNBOUNDED {
        assert_eq!(strip_escape_codes(quote.render(&plain(width))), format!("> {SENTENCE}"), "{width:?}");
    }
    let bounded = quote.render(&plain(Width::Columns(40)));
    assert!(bounded.lines().count() > 1 && widest_line(&bounded) <= 40, "{bounded}");
}

#[test]
fn a_custom_border_quote_gives_a_nested_rule_no_width() {
    let quote = BlockQuote::new(HorizontalRule::new().into(), None::<String>).with_border("> ");
    assert_eq!(strip_escape_codes(quote.render(&plain(Width::Unlimited))), "> ---");
    assert_eq!(strip_escape_codes(quote.render(&plain(Width::Columns(12)))), format!("> {}", "╌".repeat(10)));
}

#[test]
fn a_list_item_stays_on_its_line_and_a_nested_rule_stays_short() {
    let mut list = UnorderedList::empty();
    list.add(Prose::new(SENTENCE));
    let bounded = list.render(&plain(Width::Columns(40)));
    assert!(bounded.lines().count() > 1, "{bounded}");
    for width in UNBOUNDED {
        let text = list.render(&plain(width));
        assert_eq!(text.lines().count(), 1, "{width:?}:\n{text}");
        assert!(text.ends_with(SENTENCE), "{width:?}:\n{text}");
    }
    let mut rules = UnorderedList::empty();
    rules.add(HorizontalRule::new());
    let text = rules.render(&plain(Width::Unlimited));
    assert!(widest_line(&text) <= 8, "{text}");
}

#[test]
fn two_columns_take_their_content_width_with_no_width() {
    let columns = TwoColumn::new(Prose::new("left"), Prose::new(SENTENCE));
    for width in UNBOUNDED {
        let text = columns.render(&plain(width));
        assert_eq!(text.lines().count(), 1, "{width:?}:\n{text}");
        assert!(text.trim_end().ends_with(SENTENCE), "{width:?}:\n{text}");
        assert!(widest_line(&text) < 2 * SENTENCE.len() as u32, "{width:?}:\n{text}");
    }
    let bounded = columns.render(&plain(Width::Columns(40)));
    assert!(widest_line(&bounded) <= 40 && bounded.lines().count() > 1, "{bounded}");
}

#[test]
fn a_metric_label_keeps_every_character_with_no_width() {
    let label = "a_label_long_enough_that_any_page_width_has_to_truncate_it_somewhere_before_its_final_characters";
    let tree = MetricsTree::new(MetricNode::leaf(
        label,
        MetricValue::Duration(Duration::from_millis(5)),
        MetricShare::Of(0.5),
    ));
    for width in UNBOUNDED {
        assert!(tree.render(&plain(width)).contains(label), "{width:?}");
    }
    assert!(!tree.render(&plain(Width::Columns(40))).contains(label));
}

#[test]
fn a_file_name_keeps_every_character_with_no_width() {
    let long_name = "a_file_name_long_enough_that_a_narrow_terminal_has_to_truncate_it_somewhere.rs";
    let directory = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(directory.path().join(long_name), "").expect("a file");
    let mut tree = FileSystem::new(directory.path()).expect("a valid path");
    tree.ensure_tree_built();
    let unbounded = tree.render(&plain(Width::Unlimited));
    assert!(unbounded.contains(long_name), "{unbounded}");
    assert!(!tree.render(&plain(Width::Columns(30))).contains(long_name));
}

#[test]
fn a_terminal_with_no_width_reports_no_columns_and_a_modest_number() {
    let terminal = plain(Width::Unlimited);
    assert_eq!(terminal.unbounded_width, Some(UnboundedWidth::Unlimited));
    assert_eq!(terminal.output_width(), Width::Unlimited);
    assert_eq!(terminal.columns(), None);
    assert_eq!(terminal.detected_columns(), None);
    assert_eq!(terminal.width(), 80);
    assert_eq!(plain(Width::Source).output_width(), Width::Source);
}

#[test]
fn a_narrowing_outranks_having_no_width() {
    let mut terminal = plain(Width::Unlimited);
    terminal.fixed_width = Some(12);
    assert_eq!(terminal.output_width(), Width::Columns(12));
    assert_eq!(HorizontalRule::new().render(&terminal), "╌".repeat(12));
}

#[test]
fn a_directly_built_terminal_stays_numeric() {
    let terminal = Terminal::new_optimistic(72);
    assert_eq!(terminal.unbounded_width, None);
    assert_eq!(terminal.columns(), Some(72));
    assert_eq!(Terminal::builder().width(33).build().output_width(), Width::Columns(33));
    assert_eq!(Terminal::default().unbounded_width, None);
    assert_eq!(terminal.width(), 72);
}
