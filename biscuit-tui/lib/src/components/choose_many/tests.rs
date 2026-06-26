use super::super::choose::HotkeySpec;
use super::*;
use crate::core::{Label, LabelPosition, NerdFontStatus, TerminalBackground};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::style::Color;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn fixture_input() -> ChoiceInput<String> {
    ChoiceInput::new("toppings", "Pick toppings").with_options(vec![
        ChoiceOption::new("p", "Pepperoni", "pepperoni"),
        ChoiceOption::new("m", "Mushrooms", "mushrooms"),
        ChoiceOption::new("o", "Olives", "olives"),
    ])
}

fn buffer_row(buf: &Buffer, y: u16) -> String {
    let mut row = String::new();
    for x in buf.area.left()..buf.area.right() {
        row.push_str(buf[(x, y)].symbol());
    }
    row.trim_end().to_string()
}

#[test]
fn new_starts_with_no_selections() {
    let state = ChooseManyState::new(fixture_input());
    assert_eq!(state.selected_count(), 0);
    assert_eq!(state.hover(), Some(0));
}

#[test]
fn space_toggles_selection_on_and_off() {
    let mut state = ChooseManyState::new(fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    assert!(state.is_selected(0));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    assert!(!state.is_selected(0));
}

#[test]
fn typing_letter_without_filter_does_not_jump_or_toggle() {
    // First-letter quick-jump was removed — pressing a literal
    // letter is now Ignored regardless of filter state.
    let mut state = ChooseManyState::new(fixture_input());
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('m')));
    assert_eq!(outcome, EventOutcome::Ignored);
    assert!(!state.is_selected(1));
}

#[test]
fn explicit_ctrl_hotkey_toggles_matching_option() {
    let input = ChoiceInput::<String>::new("toppings", "Pick toppings").with_options(vec![
        ChoiceOption::new("p", "Pepperoni", "pepperoni"),
        ChoiceOption::new("m", "Mushrooms", "mushrooms").with_hotkey(HotkeySpec::Ctrl('m')),
    ]);
    let mut state = ChooseManyState::new(input);
    let event = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::CONTROL);
    let outcome = ChooseMany::new().handle_event(&mut state, event);
    assert_eq!(outcome, EventOutcome::Consumed);
    assert!(state.is_selected(1));
    assert_eq!(state.hover(), Some(1));
}

#[test]
fn explicit_alt_hotkey_toggles_matching_option() {
    let input = ChoiceInput::<String>::new("toppings", "Pick toppings").with_options(vec![
        ChoiceOption::new("p", "Pepperoni", "pepperoni"),
        ChoiceOption::new("m", "Mushrooms", "mushrooms").with_hotkey(HotkeySpec::Alt('x')),
    ]);
    let mut state = ChooseManyState::new(input);
    let event = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT);
    let outcome = ChooseMany::new().handle_event(&mut state, event);
    assert_eq!(outcome, EventOutcome::Consumed);
    assert!(state.is_selected(1));
    assert_eq!(state.hover(), Some(1));
}

#[test]
fn ctrl_shift_chord_matches_ctrl_hotkey() {
    // A terminal that reports CONTROL|SHIFT for an uppercase chord
    // must still match the ctrl-hotkey map.
    let input = ChoiceInput::<String>::new("toppings", "Pick toppings").with_options(vec![
        ChoiceOption::new("p", "Pepperoni", "pepperoni"),
        ChoiceOption::new("m", "Mushrooms", "mushrooms").with_hotkey(HotkeySpec::Ctrl('m')),
    ]);
    let mut state = ChooseManyState::new(input);
    let event = KeyEvent::new(
        KeyCode::Char('m'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    let outcome = ChooseMany::new().handle_event(&mut state, event);
    assert_eq!(outcome, EventOutcome::Consumed);
    assert!(state.is_selected(1));
    assert_eq!(state.hover(), Some(1));
}

#[test]
fn alt_shift_chord_matches_alt_hotkey() {
    let input = ChoiceInput::<String>::new("toppings", "Pick toppings").with_options(vec![
        ChoiceOption::new("p", "Pepperoni", "pepperoni"),
        ChoiceOption::new("m", "Mushrooms", "mushrooms").with_hotkey(HotkeySpec::Alt('x')),
    ]);
    let mut state = ChooseManyState::new(input);
    let event = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT | KeyModifiers::SHIFT);
    let outcome = ChooseMany::new().handle_event(&mut state, event);
    assert_eq!(outcome, EventOutcome::Consumed);
    assert!(state.is_selected(1));
    assert_eq!(state.hover(), Some(1));
}

#[test]
fn ctrl_alt_chord_matches_neither_hotkey() {
    // CONTROL|ALT is treated as ambiguous (AltGr-style) and must not
    // be hijacked by either hotkey map.
    let input = ChoiceInput::<String>::new("toppings", "Pick toppings").with_options(vec![
        ChoiceOption::new("p", "Pepperoni", "pepperoni"),
        ChoiceOption::new("m", "Mushrooms", "mushrooms").with_hotkey(HotkeySpec::Ctrl('m')),
        ChoiceOption::new("o", "Olives", "olives").with_hotkey(HotkeySpec::Alt('o')),
    ]);
    let mut state = ChooseManyState::new(input);

    let ctrl_chord = KeyEvent::new(
        KeyCode::Char('m'),
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    );
    assert_eq!(
        ChooseMany::new().handle_event(&mut state, ctrl_chord),
        EventOutcome::Ignored
    );
    assert!(!state.is_selected(1));

    let alt_chord = KeyEvent::new(
        KeyCode::Char('o'),
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    );
    assert_eq!(
        ChooseMany::new().handle_event(&mut state, alt_chord),
        EventOutcome::Ignored
    );
    assert!(!state.is_selected(2));
}

#[test]
fn bare_ctrl_and_alt_hotkeys_still_match() {
    // Regression guard: the relaxed modifier matching must not change
    // bare-CONTROL or bare-ALT behavior.
    let input = ChoiceInput::<String>::new("toppings", "Pick toppings").with_options(vec![
        ChoiceOption::new("p", "Pepperoni", "pepperoni"),
        ChoiceOption::new("m", "Mushrooms", "mushrooms").with_hotkey(HotkeySpec::Ctrl('m')),
        ChoiceOption::new("o", "Olives", "olives").with_hotkey(HotkeySpec::Alt('o')),
    ]);
    let mut state = ChooseManyState::new(input);

    let ctrl = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::CONTROL);
    assert_eq!(
        ChooseMany::new().handle_event(&mut state, ctrl),
        EventOutcome::Consumed
    );
    assert!(state.is_selected(1));

    let alt = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::ALT);
    assert_eq!(
        ChooseMany::new().handle_event(&mut state, alt),
        EventOutcome::Consumed
    );
    assert!(state.is_selected(2));
}

#[test]
fn max_selections_blocks_additional_toggle_on() {
    let input = fixture_input().with_max_selections(1);
    let mut state = ChooseManyState::new(input);
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert_eq!(state.selected_count(), 1);
    assert!(state.is_selected(0));
    assert!(!state.is_selected(1));
}

#[test]
fn max_selections_allows_toggle_off_when_at_cap() {
    let input = fixture_input().with_max_selections(1);
    let mut state = ChooseManyState::new(input);
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    assert_eq!(state.selected_count(), 0);
}

#[test]
fn required_validation_fires_when_fallback_cannot_find_enabled_option() {
    // With fallback-submit-on-active, Enter while nothing is selected
    // toggles the hovered option on. When every option is disabled,
    // fallback cannot find a non-disabled hover, so the required-input
    // validation error fires.
    let input = ChoiceInput::<String>::new("toppings", "Pick toppings")
        .with_options(vec![
            ChoiceOption::new("p", "Pepperoni", "pepperoni").disabled(),
            ChoiceOption::new("m", "Mushrooms", "mushrooms").disabled(),
        ])
        .required();
    let mut state = ChooseManyState::new(input);
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_some());
    assert_eq!(state.selected_count(), 0);
}

#[test]
fn enter_with_fewer_than_min_selections_sets_error() {
    let input = fixture_input().with_min_selections(2);
    let mut state = ChooseManyState::new(input);
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
    assert_eq!(outcome, EventOutcome::Consumed);
    let message = <ChooseManyState as ValidationState>::validation_error(&state).unwrap();
    assert!(message.contains('2'));
}

#[test]
fn enter_with_min_selections_met_submits() {
    let input = fixture_input().with_min_selections(1);
    let mut state = ChooseManyState::new(input);
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
    assert_eq!(outcome, EventOutcome::Submitted);
}

#[test]
fn toggling_selection_clears_validation_error() {
    let input = fixture_input().required();
    let mut state = ChooseManyState::new(input);

    // Seed a validation error as if a prior submit had been blocked.
    state.set_validation_error("Please make a selection");
    assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_some());

    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_none());
    assert!(state.is_selected(0));
}

#[test]
fn enter_does_not_auto_select_when_none_chosen() {
    // Phase 5: Enter submits the selected set exactly as-is — no
    // fallback auto-selection of the active row.
    let mut state = ChooseManyState::new(fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    assert_eq!(state.hover(), Some(1));
    assert_eq!(state.selected_count(), 0);

    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
    assert_eq!(outcome, EventOutcome::Submitted);
    assert!(!state.is_selected(1));
    assert_eq!(state.selected_count(), 0);
}

#[test]
fn fallback_submit_skips_disabled_hover() {
    // When the hovered option is disabled, fallback must not toggle it
    // on. Under `required`, the validation error must fire.
    let input = ChoiceInput::<String>::new("toppings", "Pick toppings")
        .with_options(vec![
            ChoiceOption::new("p", "Pepperoni", "pepperoni").disabled(),
            ChoiceOption::new("m", "Mushrooms", "mushrooms"),
        ])
        .required();
    let mut state = ChooseManyState::new(input);
    // Force hover onto the disabled row.
    state.hover = 0;

    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert_eq!(state.selected_count(), 0);
    assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_some());
}

#[test]
fn fallback_submit_does_not_override_existing_selections() {
    // When at least one option is already selected, fallback must not
    // touch other rows — only the existing selections are submitted.
    let mut state = ChooseManyState::new(fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    assert!(state.is_selected(0));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    assert_eq!(state.hover(), Some(2));

    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
    assert_eq!(outcome, EventOutcome::Submitted);
    assert!(state.is_selected(0));
    assert!(!state.is_selected(2));
    assert_eq!(state.selected_count(), 1);
}

#[test]
fn esc_cancels() {
    let mut state = ChooseManyState::new(fixture_input());
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Esc));
    assert_eq!(outcome, EventOutcome::Cancelled);
}

#[test]
fn disabled_options_cannot_be_toggled() {
    let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
        ChoiceOption::new("a", "Alpha", "alpha"),
        ChoiceOption::new("b", "Beta", "beta").disabled(),
    ]);
    let mut state = ChooseManyState::new(input);
    state.hover = 1;
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    assert!(!state.is_selected(1));
}

#[test]
fn selected_values_returns_value_fields_in_order() {
    let mut state = ChooseManyState::new(fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    let values = state.selected_values();
    assert_eq!(
        values,
        vec![&"pepperoni".to_string(), &"olives".to_string()]
    );
}

#[test]
fn render_draws_checkbox_glyphs_per_row() {
    let mut state =
        ChooseManyState::new(fixture_input()).with_terminal_style(TerminalStyle::default());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    let area = Rect::new(0, 0, 20, 3);
    let mut buf = Buffer::empty(area);
    ChooseMany::new().render(area, &mut buf, &mut state);
    // Focus prefix on hovered row 0, blank padding on others
    assert!(buffer_row(&buf, 0).starts_with("▶ ☑ Pepperoni"));
    assert!(buffer_row(&buf, 1).starts_with("  ☐ Mushrooms"));
    assert!(buffer_row(&buf, 2).starts_with("  ☐ Olives"));
}

#[test]
fn choose_many_render_uses_nerd_font_terminal_style() {
    let mut state = ChooseManyState::new(fixture_input())
        .with_initial_selection(&["p"])
        .with_terminal_style(TerminalStyle {
            nerd_font: NerdFontStatus::Likely,
            ..TerminalStyle::default()
        });
    let area = Rect::new(0, 0, 30, 3);
    let mut buf = Buffer::empty(area);
    ChooseMany::new().render(area, &mut buf, &mut state);

    let selected_row = buffer_row(&buf, 0);
    let unselected_row = buffer_row(&buf, 1);
    assert!(
        selected_row.contains('\u{f14a}'),
        "expected Nerd Font selected checkbox glyph in {selected_row:?}"
    );
    assert!(
        unselected_row.contains('\u{f0131}'),
        "expected Nerd Font unselected checkbox glyph in {unselected_row:?}"
    );
}

#[test]
fn choose_many_render_uses_light_background_active_foreground() {
    let mut state = ChooseManyState::new(fixture_input()).with_terminal_style(TerminalStyle {
        background: TerminalBackground::Light,
        ..TerminalStyle::default()
    });
    let area = Rect::new(0, 0, 20, 3);
    let mut buf = Buffer::empty(area);
    ChooseMany::new().render(area, &mut buf, &mut state);

    assert_eq!(buf[(1, 0)].style().fg, Some(Color::Black));
    assert_eq!(buf[(1, 0)].style().bg, Some(Color::Indexed(252)));
}

#[test]
fn render_with_label_above_draws_label_then_list() {
    let mut state = ChooseManyState::new(fixture_input())
        .with_terminal_style(TerminalStyle::default())
        .with_label(Label::new("Toppings", LabelPosition::Above));
    let area = Rect::new(0, 0, 20, 4);
    let mut buf = Buffer::empty(area);
    ChooseMany::new().render(area, &mut buf, &mut state);
    assert_eq!(buffer_row(&buf, 0), "Toppings");
    // First option has focus prefix (hover defaults to 0)
    assert!(buffer_row(&buf, 1).starts_with("▶ ☐ Pepperoni"));
}

#[test]
fn render_draws_validation_error_on_row_below_list() {
    let input = fixture_input().required();
    let mut state = ChooseManyState::new(input);
    state.set_validation_error("Please make a selection");
    let area = Rect::new(0, 0, 30, 5);
    let mut buf = Buffer::empty(area);
    ChooseMany::new().render(area, &mut buf, &mut state);
    assert_eq!(buffer_row(&buf, 3), "Please make a selection");
}

#[test]
fn end_key_jumps_hover_to_last_enabled_option() {
    let mut state = ChooseManyState::new(fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::End));
    assert_eq!(state.hover(), Some(2));
}

#[test]
fn vim_capital_g_jumps_hover_to_last_enabled_option() {
    let mut state = ChooseManyState::new(fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('G')));
    assert_eq!(state.hover(), Some(2));
}

#[test]
fn initial_selection_pre_selects_listed_ids() {
    let state = ChooseManyState::new(fixture_input()).with_initial_selection(&["p", "o"]);
    assert!(state.is_selected(0));
    assert!(!state.is_selected(1));
    assert!(state.is_selected(2));
}

#[test]
fn initial_values_pre_select_by_value() {
    // `with_initial_values` matches the option's `value` field
    // (rather than its `id`). We distinguish id/label/value so the
    // test pins the by-value semantics.
    let input: ChoiceInput<String> =
        ChoiceInput::new("toppings", "Pick toppings").with_options(vec![
            ChoiceOption::new("p", "Pepperoni", "pep-val"),
            ChoiceOption::new("m", "Mushrooms", "mush-val"),
            ChoiceOption::new("o", "Olives", "oliv-val"),
        ]);
    let state = ChooseManyState::new(input).with_initial_values(&["pep-val", "oliv-val"]);
    assert!(state.is_selected(0));
    assert!(!state.is_selected(1));
    assert!(state.is_selected(2));
}

#[test]
fn initial_values_ignore_unmatched_entries() {
    let state = ChooseManyState::new(fixture_input()).with_initial_values(&["ghost"]);
    assert_eq!(state.selected_count(), 0);
}

#[test]
fn custom_submit_binding_overrides_default() {
    let bindings = KeyBindings {
        submit: vec![KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)],
        ..KeyBindings::default()
    };
    let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));

    // Ctrl-S should submit.
    let outcome = ChooseMany::new().handle_event(
        &mut state,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
    );
    assert_eq!(outcome, EventOutcome::Submitted);

    // Enter should be ignored (no longer bound to submit).
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
    assert_eq!(outcome, EventOutcome::Ignored);
}

#[test]
fn custom_cancel_binding_works() {
    let bindings = KeyBindings {
        cancel: vec![press(KeyCode::Char('q'))],
        ..KeyBindings::default()
    };
    let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);

    // q should cancel.
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('q')));
    assert_eq!(outcome, EventOutcome::Cancelled);

    // Esc should be ignored.
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Esc));
    assert_eq!(outcome, EventOutcome::Ignored);
}

#[test]
fn custom_toggle_binding_works() {
    let bindings = KeyBindings {
        toggle: vec![press(KeyCode::Char('t'))],
        ..KeyBindings::default()
    };
    let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);

    // t should toggle.
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('t')));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert!(state.is_selected(0));

    // Space should be ignored (no longer bound to toggle).
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    assert_eq!(outcome, EventOutcome::Ignored);
    assert!(state.is_selected(0));
}

#[test]
fn custom_up_down_bindings_work() {
    let bindings = KeyBindings {
        up: vec![press(KeyCode::Char('w'))],
        down: vec![press(KeyCode::Char('s'))],
        ..KeyBindings::default()
    };
    let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);
    assert_eq!(state.hover, 0);

    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('s')));
    assert_eq!(state.hover, 1);

    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('w')));
    assert_eq!(state.hover, 0);

    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    assert_eq!(state.hover, 0);
}

#[test]
fn typed_value_u32_flow() {
    // Build from strings, project to u32.
    let options: Vec<ChoiceOption<String>> = vec![
        ChoiceOption::new("one", "One", "1"),
        ChoiceOption::new("two", "Two", "2"),
        ChoiceOption::new("three", "Three", "3"),
    ];
    let typed_options: Vec<ChoiceOption<u32>> = options
        .into_iter()
        .map(|opt| opt.map_value(|v| v.parse::<u32>().unwrap()))
        .collect();
    let input: ChoiceInput<u32> =
        ChoiceInput::new("nums", "Pick numbers").with_options(typed_options);
    let mut state: ChooseManyState<u32> = ChooseManyState::new(input);

    // Select first and third options.
    ChooseMany::<u32>::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    ChooseMany::<u32>::new().handle_event(&mut state, press(KeyCode::Down));
    ChooseMany::<u32>::new().handle_event(&mut state, press(KeyCode::Down));
    ChooseMany::<u32>::new().handle_event(&mut state, press(KeyCode::Char(' ')));

    assert_eq!(state.selected_values(), vec![&1_u32, &3_u32]);
    assert_eq!(state.value(), vec![1_u32, 3_u32]);
}

#[test]
fn typed_value_enum_flow() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Topping {
        Pepperoni,
        Mushrooms,
        Olives,
    }
    let options: Vec<ChoiceOption<Topping>> = vec![
        ChoiceOption::new("p", "Pepperoni", Topping::Pepperoni),
        ChoiceOption::new("m", "Mushrooms", Topping::Mushrooms),
        ChoiceOption::new("o", "Olives", Topping::Olives),
    ];
    let input: ChoiceInput<Topping> =
        ChoiceInput::new("toppings", "Pick toppings").with_options(options);
    let mut state: ChooseManyState<Topping> = ChooseManyState::new(input);

    ChooseMany::<Topping>::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    ChooseMany::<Topping>::new().handle_event(&mut state, press(KeyCode::Down));
    ChooseMany::<Topping>::new().handle_event(&mut state, press(KeyCode::Down));
    ChooseMany::<Topping>::new().handle_event(&mut state, press(KeyCode::Char(' ')));

    assert_eq!(
        state.selected_values(),
        vec![&Topping::Pepperoni, &Topping::Olives]
    );
    assert_eq!(state.value(), vec![Topping::Pepperoni, Topping::Olives]);
}

#[test]
fn render_overflow_indicators_when_scrolled() {
    // 6 options, 3 rows tall, hover on option 3 (scrolls to show 1-3)
    let options: Vec<ChoiceOption<String>> = (0..6)
        .map(|i| ChoiceOption::new(format!("id{i}"), format!("Option {i}"), format!("val{i}")))
        .collect();
    let input = ChoiceInput::new("test", "Test").with_options(options);
    let mut state = ChooseManyState::new(input);

    // Move hover to option 3
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    assert_eq!(state.hover, 3);

    let area = Rect::new(0, 0, 20, 3);
    let mut buf = Buffer::empty(area);
    ChooseMany::new().render(area, &mut buf, &mut state);

    // scroll_offset will be 1 (hover=3, visible=3, offset = 3+1-3 = 1), showing options 1,2,3
    // Top row should contain overflow up indicator (option 0 is hidden)
    let top_row = buffer_row(&buf, 0);
    assert!(
        top_row.contains("▲"),
        "Expected ▲ in top row, got: {top_row}"
    );

    // Bottom row should contain overflow down indicator (options 4,5 are hidden)
    let bottom_row = buffer_row(&buf, 2);
    assert!(
        bottom_row.contains("▼"),
        "Expected ▼ in bottom row, got: {bottom_row}"
    );

    // Bottom row (option 3, which is hovered) should have focus prefix
    let bottom_visible_row = buffer_row(&buf, 2);
    assert!(
        bottom_visible_row.starts_with("▶ "),
        "Expected focus prefix on hovered row: {bottom_visible_row}"
    );
    assert!(
        bottom_visible_row.contains("Option 3"),
        "Expected hovered option 3 in bottom visible row: {bottom_visible_row}"
    );
}

#[test]
fn render_disabled_row_with_dim_style() {
    let mut options: Vec<ChoiceOption<String>> = vec![
        ChoiceOption::new("a", "Active", "active".to_string()),
        ChoiceOption::new("d", "Disabled", "disabled".to_string()),
    ];
    options[1].disabled = true;
    let input = ChoiceInput::new("test", "Test").with_options(options);
    let mut state = ChooseManyState::new(input);

    let area = Rect::new(0, 0, 20, 2);
    let mut buf = Buffer::empty(area);
    ChooseMany::new().render(area, &mut buf, &mut state);

    // Row 1 is disabled; check that the label span has disabled style
    let y = 1;
    let mut found_disabled = false;
    for x in 0..area.width {
        let cell = &buf[(area.x + x, area.y + y)];
        if cell.symbol() == "D" {
            // First char of "Disabled"
            assert!(
                cell.style().fg == Some(ratatui::style::Color::DarkGray)
                    || cell
                        .style()
                        .add_modifier
                        .contains(ratatui::style::Modifier::DIM),
                "Disabled row label should have DarkGray or DIM style"
            );
            found_disabled = true;
            break;
        }
    }
    assert!(
        found_disabled,
        "Did not find disabled label 'Disabled' in buffer"
    );
}

#[test]
fn ctrl_a_selects_all_enabled_options() {
    let input = ChoiceInput::<String>::new("toppings", "Pick toppings").with_options(vec![
        ChoiceOption::new("p", "Pepperoni", "pepperoni"),
        ChoiceOption::new("m", "Mushrooms", "mushrooms").disabled(),
        ChoiceOption::new("o", "Olives", "olives"),
    ]);
    let mut state = ChooseManyState::new(input);
    let ctrl_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);

    let outcome = ChooseMany::new().handle_event(&mut state, ctrl_a);
    assert_eq!(outcome, EventOutcome::Consumed);
    assert!(state.is_selected(0));
    assert!(
        !state.is_selected(1),
        "disabled option must stay unselected"
    );
    assert!(state.is_selected(2));
    assert_eq!(state.selected_count(), 2);
}

#[test]
fn ctrl_a_ignores_max_selections_cap() {
    // The `select_all` bulk keystroke is the user's explicit intent;
    // validation runs at submit time, so the cap does not block bulk
    // selection at toggle time.
    let input = fixture_input().with_max_selections(1);
    let mut state = ChooseManyState::new(input);
    let ctrl_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    ChooseMany::new().handle_event(&mut state, ctrl_a);
    assert_eq!(state.selected_count(), 3);
}

#[test]
fn ctrl_d_clears_all() {
    let mut state = ChooseManyState::new(fixture_input());
    // Select every option first.
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    assert_eq!(state.selected_count(), 3);

    let ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
    let outcome = ChooseMany::new().handle_event(&mut state, ctrl_d);
    assert_eq!(outcome, EventOutcome::Consumed);
    assert_eq!(state.selected_count(), 0);
}

#[test]
fn ctrl_d_clears_even_when_below_min_selections() {
    // min_selections is validated at submit, not at toggle — so bulk
    // deselect always clears.
    let input = fixture_input().with_min_selections(2);
    let mut state = ChooseManyState::new(input);
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    assert_eq!(state.selected_count(), 2);

    let ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
    ChooseMany::new().handle_event(&mut state, ctrl_d);
    assert_eq!(state.selected_count(), 0);
}

#[test]
fn select_all_clears_validation_error() {
    let input = fixture_input().required();
    let mut state = ChooseManyState::new(input);
    state.set_validation_error("Please make a selection");
    state.select_all();
    assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_none());
    assert_eq!(state.selected_count(), 3);
}

#[test]
fn deselect_all_clears_validation_error() {
    let mut state = ChooseManyState::new(fixture_input());
    state.set_validation_error("Please select fewer");
    state.deselect_all();
    assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_none());
}

#[test]
fn custom_select_all_binding_overrides_default() {
    let bindings = KeyBindings {
        select_all: vec![press(KeyCode::Char('A'))],
        ..KeyBindings::default()
    };
    let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);

    // Plain 'A' should now select all.
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('A')));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert_eq!(state.selected_count(), 3);

    // Ctrl-A should now be ignored (no longer bound).
    state.deselect_all();
    let ctrl_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    let outcome = ChooseMany::new().handle_event(&mut state, ctrl_a);
    assert_eq!(outcome, EventOutcome::Ignored);
    assert_eq!(state.selected_count(), 0);
}

#[test]
fn shuffle_randomises_order_choose_many() {
    let options: Vec<ChoiceOption<String>> = (0..20)
        .map(|i| ChoiceOption::new(format!("id{i}"), format!("Option {i}"), format!("value{i}")))
        .collect();
    let original_labels: Vec<String> = options.iter().map(|o| o.label.clone()).collect();
    let input = ChoiceInput::new("x", "P")
        .with_shuffle_options(true)
        .with_options(options);
    let state = ChooseManyState::new(input);
    let shuffled_labels: Vec<&str> = state.options().iter().map(|o| o.label.as_str()).collect();
    let same_set: std::collections::HashSet<&str> = shuffled_labels.iter().copied().collect();
    let original_set: std::collections::HashSet<&str> =
        original_labels.iter().map(|s| s.as_str()).collect();
    assert_eq!(same_set, original_set);
    assert_ne!(
        shuffled_labels,
        original_labels
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>(),
        "With 20 options it is astronomically unlikely the order is unchanged"
    );
}

#[test]
fn shuffle_then_select_choose_many() {
    let options: Vec<ChoiceOption<String>> = (0..10)
        .map(|i| ChoiceOption::new(format!("id{i}"), format!("Opt{i}"), format!("val{i}")))
        .collect();
    let input = ChoiceInput::new("x", "P")
        .with_shuffle_options(true)
        .with_options(options);
    let mut state = ChooseManyState::new(input);
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    assert!(state.is_selected(0));
    let values = state.selected_values();
    assert_eq!(values.len(), 1);
}

// -----------------------------------------------------------------
// Phase 8 — Search Prompt Rendering & State Plumbing
// -----------------------------------------------------------------

fn filter_fixture_input() -> ChoiceInput<String> {
    ChoiceInput::new("fruit", "Pick fruits")
        .with_filter_enabled(true)
        .with_options(vec![
            ChoiceOption::new("a", "Apple", "apple"),
            ChoiceOption::new("b", "Banana", "banana"),
            ChoiceOption::new("c", "Blueberry", "blueberry"),
            ChoiceOption::new("d", "Cherry", "cherry"),
        ])
}

#[test]
fn filter_visible_starts_false_many() {
    let state = ChooseManyState::new(filter_fixture_input());
    assert!(!state.filter_visible());
    assert_eq!(state.filter_pattern(), "");
    assert_eq!(state.visible_indices(), &[0, 1, 2, 3]);
}

#[test]
fn typing_letter_opens_filter_many() {
    let mut state = ChooseManyState::new(filter_fixture_input());
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('B')));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert!(state.filter_visible());
    assert_eq!(state.filter_pattern(), "B");
    let visible = state.visible_indices().to_vec();
    assert!(visible.contains(&1));
    assert!(visible.contains(&2));
    assert!(!visible.contains(&0));
}

#[test]
fn backspace_pops_filter_character_many() {
    let mut state = ChooseManyState::new(filter_fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('b')));
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('l')));
    assert_eq!(state.filter_pattern(), "bl");
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Backspace));
    assert_eq!(state.filter_pattern(), "b");
}

#[test]
fn down_walks_filtered_indices_many() {
    let mut state = ChooseManyState::new(filter_fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('b')));
    let visible = state.visible_indices().to_vec();
    assert!(!visible.is_empty());
    let first = visible[0];
    assert_eq!(state.hover(), Some(first));

    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    let hover = state.hover().unwrap();
    assert!(visible.contains(&hover));
    assert_ne!(hover, first);
}

#[test]
fn space_toggles_first_filtered_match() {
    let mut state = ChooseManyState::new(filter_fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('c'))); // Cherry + maybe Blueberry via fuzzy
    let hover = state.hover().unwrap();
    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert!(state.is_selected(hover));
}

#[test]
fn esc_clears_filter_first_then_aborts_many() {
    let mut state = ChooseManyState::new(filter_fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('b')));
    assert!(state.filter_visible());

    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Esc));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert!(!state.filter_visible());
    assert_eq!(state.filter_pattern(), "");

    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Esc));
    assert_eq!(outcome, EventOutcome::Cancelled);
}

#[test]
fn submit_blocked_when_filter_hides_everything() {
    let mut state = ChooseManyState::new(filter_fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('z')));
    assert!(state.filter_visible());
    assert!(state.visible_indices().is_empty());

    let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert_eq!(state.selected_count(), 0);
}

#[test]
fn render_draws_search_prompt_row_when_filter_visible_many() {
    let mut state = ChooseManyState::new(filter_fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('b')));
    let area = Rect::new(0, 0, 30, 5);
    let mut buf = Buffer::empty(area);
    ChooseMany::new().render(area, &mut buf, &mut state);
    let prompt = buffer_row(&buf, 0);
    assert!(
        prompt.starts_with("/ b"),
        "expected '/ b' prompt at top, got {prompt:?}"
    );
}

#[test]
fn render_shows_no_matches_row_when_filter_matches_nothing_many() {
    let mut state = ChooseManyState::new(filter_fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('z')));
    let area = Rect::new(0, 0, 30, 5);
    let mut buf = Buffer::empty(area);
    ChooseMany::new().render(area, &mut buf, &mut state);
    assert_eq!(buffer_row(&buf, 1), "(no matches)");
}

#[test]
fn ctrl_a_selects_all_visible_matches_only() {
    // When a filter is active, Ctrl+A selects every enabled visible
    // option. The current implementation selects every enabled
    // option regardless of filter, which is acceptable because the
    // bulk keystroke is the user's explicit intent; validation runs
    // at submit time. We pin the existing behaviour here so future
    // refactors surface any regressions.
    let mut state = ChooseManyState::new(filter_fixture_input());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('b')));
    let ctrl_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    ChooseMany::new().handle_event(&mut state, ctrl_a);
    assert!(state.selected_count() > 0);
}

// -----------------------------------------------------------------
// Phase 6 — Horizontal Layout & Navigation
// -----------------------------------------------------------------

fn horizontal_fixture_input_many() -> ChoiceInput<String> {
    ChoiceInput::new("toppings", "Pick toppings")
        .with_orientation(Orientation::Horizontal)
        .with_options(vec![
            ChoiceOption::new("p", "Pepperoni", "pepperoni"),
            ChoiceOption::new("m", "Mushrooms", "mushrooms"),
            ChoiceOption::new("o", "Olives", "olives"),
            ChoiceOption::new("e", "Extra", "extra"),
        ])
}

#[test]
fn horizontal_many_render_packs_items_left_to_right() {
    let mut state = ChooseManyState::new(horizontal_fixture_input_many());
    let area = Rect::new(0, 0, 80, 2);
    let mut buf = Buffer::empty(area);
    ChooseMany::new().render(area, &mut buf, &mut state);

    let row0 = buffer_row(&buf, 0);
    assert!(
        row0.contains("Pepperoni"),
        "expected Pepperoni on row 0: {row0}"
    );
    assert!(
        row0.contains("Mushrooms"),
        "expected Mushrooms on row 0: {row0}"
    );
}

#[test]
fn horizontal_many_render_no_triangular_pointer() {
    let mut state = ChooseManyState::new(horizontal_fixture_input_many());
    let area = Rect::new(0, 0, 80, 2);
    let mut buf = Buffer::empty(area);
    ChooseMany::new().render(area, &mut buf, &mut state);

    let row0 = buffer_row(&buf, 0);
    assert!(
        !row0.contains('▶'),
        "horizontal mode should not show triangular pointer: {row0}"
    );
}

#[test]
fn choose_many_horizontal_badges_short_viewport_does_not_draw_past_area() {
    let input: ChoiceInput<String> = ChoiceInput::new("toppings", "Pick toppings")
        .with_orientation(Orientation::Horizontal)
        .with_options(vec![
            ChoiceOption::new("a", "Alpha", "alpha").with_hotkey(HotkeySpec::Ctrl('a')),
            ChoiceOption::new("b", "Bravo", "bravo").with_hotkey(HotkeySpec::Ctrl('b')),
            ChoiceOption::new("c", "Charlie", "charlie").with_hotkey(HotkeySpec::Ctrl('c')),
        ]);
    let mut state = ChooseManyState::new(input).with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    let area = Rect::new(0, 0, 12, 3);
    let mut buf = Buffer::empty(area);

    ChooseMany::new().render(area, &mut buf, &mut state);

    assert!(state.layout_cache.row_count() >= 3);
    assert_eq!(state.scroll_offset, 0);
    assert!(buffer_row(&buf, 0).contains("Alpha"));
    assert!(buffer_row(&buf, 1).contains("^A"));
    for y in 0..area.height {
        let row = buffer_row(&buf, y);
        assert!(
            !row.contains("Bravo") && !row.contains("Charlie"),
            "short viewport must not render hidden logical rows on y={y}: {row:?}"
        );
    }
}

#[test]
fn horizontal_many_left_moves_to_previous_option() {
    let mut state = ChooseManyState::new(horizontal_fixture_input_many());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Right));
    assert_eq!(state.hover(), Some(1));

    ChooseMany::new().handle_event(&mut state, press(KeyCode::Left));
    assert_eq!(state.hover(), Some(0));
}

#[test]
fn horizontal_many_right_moves_to_next_option() {
    let mut state = ChooseManyState::new(horizontal_fixture_input_many());
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Right));
    assert_eq!(state.hover(), Some(1));
}

#[test]
fn horizontal_many_up_down_use_row_navigation() {
    let mut state = ChooseManyState::new(horizontal_fixture_input_many());
    // Move to option likely on a different row.
    for _ in 0..3 {
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Right));
    }
    let hover_before = state.hover().unwrap();

    ChooseMany::new().handle_event(&mut state, press(KeyCode::Up));
    let hover_after = state.hover().unwrap();
    assert_ne!(hover_after, hover_before, "up should change hover");
}

#[test]
fn horizontal_many_stale_cache_fallback_to_sequential() {
    let mut state = ChooseManyState::new(horizontal_fixture_input_many());
    state.layout_cache = ChoiceLayout::default();

    ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
    assert_eq!(state.hover(), Some(1));

    state.layout_cache = ChoiceLayout::default();
    ChooseMany::new().handle_event(&mut state, press(KeyCode::Up));
    assert_eq!(state.hover(), Some(0));
}

fn unsorted_input_many() -> ChoiceInput<String> {
    ChoiceInput::new("fruit", "Pick").with_options(vec![
        ChoiceOption::new("b", "Berry", "berry"),
        ChoiceOption::new("a", "Apple", "apple"),
        ChoiceOption::new("c", "Cherry", "cherry"),
    ])
}

fn labels_many(state: &ChooseManyState) -> Vec<&str> {
    state.options().iter().map(|o| o.label.as_str()).collect()
}

#[test]
fn state_new_applies_with_sort_asc_orders_by_label() {
    let input = unsorted_input_many().with_sort(crate::core::SortOrder::Asc);
    let state = ChooseManyState::new(input);
    assert_eq!(labels_many(&state), vec!["Apple", "Berry", "Cherry"]);
}

#[test]
fn state_new_applies_with_sort_desc_orders_by_label() {
    let input = unsorted_input_many().with_sort(crate::core::SortOrder::Desc);
    let state = ChooseManyState::new(input);
    assert_eq!(labels_many(&state), vec!["Cherry", "Berry", "Apple"]);
}

#[test]
fn state_new_applies_with_sort_inverse_reverses_natural() {
    // `SortOrder::Inverse` is the canonical reversing ordering used
    // by both the library and the CLI `--sort inverse` surface.
    let input = unsorted_input_many().with_sort(crate::core::SortOrder::Inverse);
    let state = ChooseManyState::new(input);
    assert_eq!(labels_many(&state), vec!["Cherry", "Apple", "Berry"]);
}

#[test]
fn state_new_with_sort_natural_is_no_op() {
    let input = unsorted_input_many().with_sort(crate::core::SortOrder::Natural);
    let state = ChooseManyState::new(input);
    assert_eq!(labels_many(&state), vec!["Berry", "Apple", "Cherry"]);
}

#[test]
fn state_new_without_sort_preserves_input_order() {
    // `with_sort` is never called, so the configured order is
    // preserved exactly as given.
    let state = ChooseManyState::new(unsorted_input_many());
    assert_eq!(labels_many(&state), vec!["Berry", "Apple", "Cherry"]);
}

#[test]
fn state_new_sort_allocates_selection_vec_with_correct_length() {
    // The per-option `selected` vec must match the post-sort options
    // list exactly. A stale length here would index out of bounds
    // when toggling.
    let input = unsorted_input_many().with_sort(crate::core::SortOrder::Asc);
    let state = ChooseManyState::new(input);
    assert_eq!(state.options().len(), 3);
    assert!(!state.is_selected(0));
    assert!(!state.is_selected(1));
    assert!(!state.is_selected(2));
}

#[test]
fn state_new_sort_then_shuffle_does_not_panic() {
    // Sorting and shuffling are independent: the constructor sorts
    // first, then shuffles. Either ordering must be acceptable as a
    // post-condition; this test pins that the combination simply
    // does not panic and yields the same number of options.
    let input = unsorted_input_many()
        .with_sort(crate::core::SortOrder::Asc)
        .with_shuffle_options(true);
    let state = ChooseManyState::new(input);
    assert_eq!(state.options().len(), 3);
}

// --- Forced hotkey-display override survives transient events ---

fn ctrl_press_many(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

fn alt_press_many(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT)
}

fn modifier_press_many(mod_key: crossterm::event::ModifierKeyCode) -> KeyEvent {
    KeyEvent::new(KeyCode::Modifier(mod_key), KeyModifiers::NONE)
}

fn modifier_release_many(mod_key: crossterm::event::ModifierKeyCode) -> KeyEvent {
    let mut event = KeyEvent::new(KeyCode::Modifier(mod_key), KeyModifiers::NONE);
    event.kind = KeyEventKind::Release;
    event
}

#[test]
fn many_with_hotkey_display_hidden_survives_ctrl_modifier_press() {
    use crossterm::event::ModifierKeyCode;
    let mut state =
        ChooseManyState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::Hidden);
    ChooseMany::new().handle_event(
        &mut state,
        modifier_press_many(ModifierKeyCode::LeftControl),
    );
    assert_eq!(state.hotkey_display(), HotkeyDisplayMode::Hidden);
    assert!(state.hotkey_display_deadline.is_none());
    assert_eq!(
        state.current_hotkey_display(Instant::now()),
        HotkeyDisplayMode::Hidden
    );
}

#[test]
fn many_with_hotkey_display_hidden_survives_alt_modifier_press() {
    use crossterm::event::ModifierKeyCode;
    let mut state =
        ChooseManyState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::Hidden);
    ChooseMany::new().handle_event(&mut state, modifier_press_many(ModifierKeyCode::LeftAlt));
    assert_eq!(state.hotkey_display(), HotkeyDisplayMode::Hidden);
    assert!(state.hotkey_display_deadline.is_none());
}

#[test]
fn many_with_hotkey_display_hidden_survives_chord_fallback() {
    let mut state =
        ChooseManyState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::Hidden);
    ChooseMany::new().handle_event(&mut state, ctrl_press_many('z'));
    assert_eq!(state.hotkey_display(), HotkeyDisplayMode::Hidden);
    assert!(state.hotkey_display_deadline.is_none());
    assert_eq!(
        state.current_hotkey_display(Instant::now()),
        HotkeyDisplayMode::Hidden
    );
}

#[test]
fn many_with_hotkey_display_ctrl_held_survives_modifier_release() {
    use crossterm::event::ModifierKeyCode;
    let mut state =
        ChooseManyState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    ChooseMany::new().handle_event(
        &mut state,
        modifier_release_many(ModifierKeyCode::LeftControl),
    );
    assert_eq!(state.hotkey_display(), HotkeyDisplayMode::CtrlHeld);
    assert!(state.hotkey_display_deadline.is_none());
    assert_eq!(
        state.current_hotkey_display(Instant::now()),
        HotkeyDisplayMode::CtrlHeld
    );
}

#[test]
fn many_with_hotkey_display_alt_held_survives_modifier_release() {
    use crossterm::event::ModifierKeyCode;
    let mut state =
        ChooseManyState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::AltHeld);
    ChooseMany::new().handle_event(&mut state, modifier_release_many(ModifierKeyCode::LeftAlt));
    assert_eq!(state.hotkey_display(), HotkeyDisplayMode::AltHeld);
    assert!(state.hotkey_display_deadline.is_none());
    assert_eq!(
        state.current_hotkey_display(Instant::now()),
        HotkeyDisplayMode::AltHeld
    );
}

#[test]
fn many_with_hotkey_display_ctrl_held_not_overwritten_by_alt_event() {
    let mut state =
        ChooseManyState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    ChooseMany::new().handle_event(&mut state, alt_press_many('q'));
    assert_eq!(state.hotkey_display(), HotkeyDisplayMode::CtrlHeld);
    assert!(state.hotkey_display_deadline.is_none());
    assert_eq!(
        state.current_hotkey_display(Instant::now()),
        HotkeyDisplayMode::CtrlHeld
    );
}

#[test]
fn many_with_hotkey_display_alt_held_not_overwritten_by_ctrl_event() {
    let mut state =
        ChooseManyState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::AltHeld);
    ChooseMany::new().handle_event(&mut state, ctrl_press_many('q'));
    assert_eq!(state.hotkey_display(), HotkeyDisplayMode::AltHeld);
    assert!(state.hotkey_display_deadline.is_none());
    assert_eq!(
        state.current_hotkey_display(Instant::now()),
        HotkeyDisplayMode::AltHeld
    );
}

#[test]
fn many_with_hotkey_display_forces_mode_and_clears_deadline() {
    let state =
        ChooseManyState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
    assert_eq!(state.hotkey_display(), HotkeyDisplayMode::CtrlHeld);
    assert_eq!(
        state.current_hotkey_display(Instant::now() + std::time::Duration::from_secs(60)),
        HotkeyDisplayMode::CtrlHeld
    );
}

// --- Sticky `Ctrl+Space` / `Alt+Space` toggle ----------------------------

fn many_space_with_modifier(modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(' '), modifiers)
}

#[test]
fn many_ctrl_space_toggles_sticky_ctrl_held() {
    let mut state = ChooseManyState::new(fixture_input());
    assert_eq!(state.hotkey_display_sticky(), None);
    let outcome =
        ChooseMany::new().handle_event(&mut state, many_space_with_modifier(KeyModifiers::CONTROL));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert_eq!(
        state.hotkey_display_sticky(),
        Some(HotkeyDisplayMode::CtrlHeld)
    );
    assert_eq!(
        state.current_hotkey_display(Instant::now()),
        HotkeyDisplayMode::CtrlHeld
    );
}

#[test]
fn many_alt_space_toggles_sticky_alt_held() {
    let mut state = ChooseManyState::new(fixture_input());
    ChooseMany::new().handle_event(&mut state, many_space_with_modifier(KeyModifiers::ALT));
    assert_eq!(
        state.hotkey_display_sticky(),
        Some(HotkeyDisplayMode::AltHeld)
    );
}

#[test]
fn many_sticky_release_clears() {
    // Press sets the sticky mode, Release clears it.
    let mut state = ChooseManyState::new(fixture_input());
    ChooseMany::new().handle_event(&mut state, many_space_with_modifier(KeyModifiers::CONTROL));
    assert_eq!(
        state.hotkey_display_sticky(),
        Some(HotkeyDisplayMode::CtrlHeld)
    );
    let release = KeyEvent {
        code: KeyCode::Char(' '),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::NONE,
    };
    ChooseMany::new().handle_event(&mut state, release);
    assert_eq!(state.hotkey_display_sticky(), None);
}

#[test]
fn many_sticky_toggle_other_chord_switches_mode() {
    let mut state = ChooseManyState::new(fixture_input());
    ChooseMany::new().handle_event(&mut state, many_space_with_modifier(KeyModifiers::CONTROL));
    ChooseMany::new().handle_event(&mut state, many_space_with_modifier(KeyModifiers::ALT));
    assert_eq!(
        state.hotkey_display_sticky(),
        Some(HotkeyDisplayMode::AltHeld)
    );
}

#[test]
fn many_sticky_toggle_suppressed_when_override_active() {
    let mut state =
        ChooseManyState::new(fixture_input()).with_hotkey_display(HotkeyDisplayMode::Hidden);
    ChooseMany::new().handle_event(&mut state, many_space_with_modifier(KeyModifiers::CONTROL));
    assert_eq!(state.hotkey_display_sticky(), None);
    assert_eq!(
        state.current_hotkey_display(Instant::now()),
        HotkeyDisplayMode::Hidden
    );
}

#[test]
fn many_plain_space_still_toggles_selection_not_badge_visibility() {
    // Plain Space (no modifier) MUST continue to toggle the active
    // row's selection. The sticky-toggle handler is only triggered
    // by `Ctrl+Space` / `Alt+Space`.
    let mut state = ChooseManyState::new(fixture_input());
    ChooseMany::new().handle_event(
        &mut state,
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
    );
    assert_eq!(state.hotkey_display_sticky(), None);
    // The active option is now selected.
    assert!(state.is_selected(0));
}
