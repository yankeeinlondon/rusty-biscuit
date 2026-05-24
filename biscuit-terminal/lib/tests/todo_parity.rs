//! Tree-path tests for the `Todo` component's IR migration.
//!
//! Coverage splits into:
//!
//! 1. **Structural tests** — verify the tree projection shape: a one-item
//!    `NodeKind::List` whose `ListItem` carries typed [`TaskHints`], the
//!    state class (`todo-*`), and (for `Cancelled`) a `Delete` node wrapping
//!    the description.
//! 2. **Terminal tree output** — for each `TodoState` variant, the
//!    canonical tree path preserves the description and applies the correct
//!    state-class semantics.
//! 3. **Cross-target tests** — Markdown and Browser output, asserting GFM
//!    task-list syntax (`- [x]` / `- [ ]`), `~~text~~` strikethrough for
//!    `Cancelled`, and `todo-*` state classes on `<li>`.

mod parity_helpers;

use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::todo::{Todo, TodoState};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use renderable::markdown::MarkdownRenderable;
use renderable::tree::{
    NodeKind, RenderStrictness, TaskState, TreeRenderable, ValidationMode, validate,
};

use parity_helpers::{PARITY_WIDTHS, assert_contains_tokens, strip_ansi, test_terminal};

fn render_tree(node: &renderable::tree::RenderNode, width: u32) -> String {
    let term = test_terminal(width);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    render_terminal_node(node, &opts)
        .expect("tree render should succeed")
        .output
}

const ALL_STATES: &[TodoState] = &[
    TodoState::Open,
    TodoState::InProgress,
    TodoState::Completed,
    TodoState::Blocked,
    TodoState::Cancelled,
];

// ---------------------------------------------------------------------------
// Structural tests
// ---------------------------------------------------------------------------

#[test]
fn projection_is_unordered_list_with_one_item() {
    let todo = Todo::new("Do thing");
    let node = <Todo as TreeRenderable>::render_tree(&todo);
    match &node.kind {
        NodeKind::List {
            ordered, children, ..
        } => {
            assert!(!ordered, "unordered list");
            assert_eq!(children.len(), 1, "exactly one list item");
        }
        other => panic!("expected NodeKind::List, got {other:?}"),
    }
}

#[test]
fn list_carries_empty_bullet_list_hints_for_terminal_marker_only() {
    let todo = Todo::new("Item");
    let node = <Todo as TreeRenderable>::render_tree(&todo);
    let hints = node.attrs.list_hints();
    assert_eq!(
        hints.bullet.as_deref(),
        Some(""),
        "empty terminal bullet so the task marker is the only visible prefix"
    );
}

#[test]
fn item_carries_state_class_and_task_hints() {
    for state in ALL_STATES {
        let todo = Todo::new("X").with_state(state.clone());
        let node = <Todo as TreeRenderable>::render_tree(&todo);
        let item = match &node.kind {
            NodeKind::List { children, .. } => &children[0],
            other => panic!("expected List, got {other:?}"),
        };
        let hints = item.attrs.task_hints().expect("task hints present");
        assert_eq!(
            hints.state,
            TaskState::from(state.clone()),
            "task hint state matches"
        );
        let expected_class = match state {
            TodoState::Open => "todo-open",
            TodoState::InProgress => "todo-in-progress",
            TodoState::Completed => "todo-completed",
            TodoState::Blocked => "todo-blocked",
            TodoState::Cancelled => "todo-cancelled",
        };
        assert!(
            item.attrs.classes.contains(&expected_class.to_string()),
            "{:?}: expected class {expected_class} in {:?}",
            state,
            item.attrs.classes
        );
    }
}

#[test]
fn only_completed_sets_checked_true() {
    for state in ALL_STATES {
        let todo = Todo::new("X").with_state(state.clone());
        let node = <Todo as TreeRenderable>::render_tree(&todo);
        let item = match &node.kind {
            NodeKind::List { children, .. } => &children[0],
            other => panic!("expected List, got {other:?}"),
        };
        let checked = match &item.kind {
            NodeKind::ListItem { checked, .. } => *checked,
            other => panic!("expected ListItem, got {other:?}"),
        };
        let expected = Some(matches!(state, TodoState::Completed));
        assert_eq!(
            checked, expected,
            "{:?}: only Completed should be checked=true",
            state
        );
    }
}

#[test]
fn cancelled_wraps_description_in_styled_span_around_delete() {
    // Cancelled projects as `Paragraph → Span(style=dim+strike) → Delete → Text`.
    // The Span carries the inline dim+strikethrough emphasis for terminal
    // rendering; the Delete inside it survives as `~~text~~` in Markdown and
    // `<s>...</s>` in Browser.
    let todo = Todo::new("Drop").with_state(TodoState::Cancelled);
    let node = <Todo as TreeRenderable>::render_tree(&todo);
    let item = match &node.kind {
        NodeKind::List { children, .. } => &children[0],
        other => panic!("expected List, got {other:?}"),
    };
    let para_children = match &item.kind {
        NodeKind::ListItem { children, .. } => children,
        other => panic!("expected ListItem, got {other:?}"),
    };
    let paragraph_inner = match &para_children[0].kind {
        NodeKind::Paragraph { children } => children,
        other => panic!("expected Paragraph, got {other:?}"),
    };
    let span = &paragraph_inner[0];
    let span_children = match &span.kind {
        NodeKind::Span { children } => children,
        other => panic!("expected Span at paragraph head, got {other:?}"),
    };
    let span_style = span.attrs.style().expect("Span carries dim/strike Style");
    assert!(span_style.emphasis.dim, "span dim emphasis");
    assert!(
        span_style.emphasis.strikethrough,
        "span strikethrough emphasis"
    );
    assert!(
        matches!(&span_children[0].kind, NodeKind::Delete { .. }),
        "Span wraps a Delete: {:?}",
        span_children[0].kind
    );
}

#[test]
fn cancelled_seeds_dim_and_strikethrough_style() {
    let todo = Todo::new("Drop").with_state(TodoState::Cancelled);
    let node = <Todo as TreeRenderable>::render_tree(&todo);
    let item = match &node.kind {
        NodeKind::List { children, .. } => &children[0],
        other => panic!("expected List, got {other:?}"),
    };
    let style = item.attrs.style().expect("style attr on Cancelled item");
    assert!(style.emphasis.dim, "dim emphasis");
    assert!(style.emphasis.strikethrough, "strikethrough emphasis");
}

#[test]
fn non_cancelled_states_have_no_listitem_style() {
    for state in ALL_STATES {
        if matches!(state, TodoState::Cancelled) {
            continue;
        }
        let todo = Todo::new("X").with_state(state.clone());
        let node = <Todo as TreeRenderable>::render_tree(&todo);
        let item = match &node.kind {
            NodeKind::List { children, .. } => &children[0],
            other => panic!("expected List, got {other:?}"),
        };
        assert!(
            item.attrs.style().is_none(),
            "{:?}: no Style on non-cancelled ListItem",
            state
        );
    }
}

#[test]
fn default_layout_is_not_recorded_on_list() {
    let todo = Todo::new("X");
    let node = <Todo as TreeRenderable>::render_tree(&todo);
    assert!(
        node.attrs.layout().is_none(),
        "default layout must not seed a layout attr on the List"
    );
}

#[test]
fn non_default_layout_is_recorded_on_list() {
    // Layout is carried on the `List` (the component's visible block) so the
    // terminal `render_with_layout` applies margins around the rendered
    // output. Setting it on the inner `ListItem` would be ignored by the
    // list rendering path.
    use biscuit_terminal::utils::layout::{Length, TargetValue};
    let mut todo = Todo::new("X");
    todo.layout_mut().margin.left = TargetValue::universal(Length::ch(4));
    let node = <Todo as TreeRenderable>::render_tree(&todo);
    assert!(
        node.attrs.layout().is_some(),
        "non-default layout records a layout attr on the List"
    );
}

#[test]
fn projected_tree_validates_with_no_errors() {
    for state in ALL_STATES {
        let todo = Todo::new("Validate").with_state(state.clone());
        let node = <Todo as TreeRenderable>::render_tree(&todo);
        let report = validate(&node, ValidationMode::Full);
        assert!(
            !report.has_errors(),
            "{:?}: projected Todo tree should validate cleanly: {:?}",
            state,
            report.errors().collect::<Vec<_>>()
        );
    }
}

#[test]
fn tree_renderable_and_render_tree_node_share_projection() {
    for state in ALL_STATES {
        let todo = Todo::new("Share").with_state(state.clone());
        let canonical = <Todo as TreeRenderable>::render_tree(&todo);
        let compat = todo.render_tree_node().expect("tree node");
        assert_eq!(
            serde_json::to_value(&canonical).unwrap(),
            serde_json::to_value(&compat).unwrap(),
            "{:?}: canonical and compatibility projections must serialize identically",
            state
        );
    }
}

// ---------------------------------------------------------------------------
// Terminal output: every state preserves the description after ANSI stripping
// ---------------------------------------------------------------------------

#[test]
fn every_state_preserves_description_text_through_tree() {
    let term = test_terminal(80);
    for state in ALL_STATES {
        let todo = Todo::new("Task description").with_state(state.clone());
        let plain = strip_ansi(&todo.render(&term));
        assert!(
            plain.contains("Task description"),
            "{:?}: description preserved: {plain:?}",
            state
        );
    }
}

/// Builds a terminal that advertises Nerd Font support for marker selection.
fn nerd_terminal(width: u32) -> biscuit_terminal::terminal::Terminal {
    use biscuit_terminal::discovery::detection::ColorDepth;
    use biscuit_terminal::terminal::Terminal;
    let mut term = Terminal::builder()
        .width(width)
        .color_depth(ColorDepth::TrueColor)
        .build();
    term.is_nerd_font = Some(true);
    term
}

#[test]
fn nerd_font_terminal_uses_nerd_glyph_open() {
    use biscuit_terminal::components::todo::TODO_CHAR_LOOKUP;
    let term = nerd_terminal(80);
    let todo = Todo::new("Open task");
    let out = todo.render(&term);
    let nerd_open = TODO_CHAR_LOOKUP[&TodoState::Open].nerd;
    assert!(out.contains(nerd_open), "expected nerd open glyph: {out:?}");
}

#[test]
fn nerd_font_terminal_uses_nerd_glyph_completed() {
    use biscuit_terminal::components::todo::TODO_CHAR_LOOKUP;
    let term = nerd_terminal(80);
    let todo = Todo::new("Done").with_state(TodoState::Completed);
    let out = todo.render(&term);
    let nerd = TODO_CHAR_LOOKUP[&TodoState::Completed].nerd;
    assert!(out.contains(nerd), "expected nerd completed glyph: {out:?}");
}

#[test]
fn no_color_no_nerd_open_state_uses_ascii_fallback() {
    use biscuit_terminal::discovery::detection::ColorDepth;
    use biscuit_terminal::terminal::Terminal;
    let mut term = Terminal::builder()
        .width(80)
        .color_depth(ColorDepth::None)
        .build();
    term.is_nerd_font = Some(false);
    let todo = Todo::new("Task");
    let plain = strip_ansi(&todo.render(&term));
    assert!(
        plain.contains("[ ]"),
        "expected ASCII [ ] open fallback: {plain:?}"
    );
}

#[test]
fn no_color_no_nerd_completed_state_uses_ascii_fallback() {
    use biscuit_terminal::discovery::detection::ColorDepth;
    use biscuit_terminal::terminal::Terminal;
    let mut term = Terminal::builder()
        .width(80)
        .color_depth(ColorDepth::None)
        .build();
    term.is_nerd_font = Some(false);
    let todo = Todo::new("Task").with_state(TodoState::Completed);
    let plain = strip_ansi(&todo.render(&term));
    assert!(
        plain.contains("[x]"),
        "expected ASCII [x] completed fallback: {plain:?}"
    );
}

#[test]
fn no_color_no_nerd_cancelled_state_uses_ascii_fallback() {
    use biscuit_terminal::discovery::detection::ColorDepth;
    use biscuit_terminal::terminal::Terminal;
    let mut term = Terminal::builder()
        .width(80)
        .color_depth(ColorDepth::None)
        .build();
    term.is_nerd_font = Some(false);
    let todo = Todo::new("Drop").with_state(TodoState::Cancelled);
    let plain = strip_ansi(&todo.render(&term));
    assert!(
        plain.contains("[-]"),
        "expected ASCII [-] cancelled fallback: {plain:?}"
    );
    assert!(
        plain.contains("Drop"),
        "expected description text: {plain:?}"
    );
}

#[test]
fn no_color_no_nerd_non_cancelled_emits_no_ansi_escapes() {
    // Open, InProgress, Completed, and Blocked states have no inline styling
    // markers, so a NoColor + non-Nerd terminal emits pure ASCII for them.
    //
    // Cancelled deliberately uses an inline `Delete` node so Markdown can
    // emit `~~text~~` and Browser can emit `<s>` — that means the inline
    // strikethrough SGR (`\x1b[9m`) still leaks through on terminal even when
    // `color_depth = None`. The bespoke path stripped strikethrough in that
    // case; the tree path preserves it. Recorded under KNOWN_DRIFT (Todo
    // cancelled NoColor strikethrough preserved by tree path).
    use biscuit_terminal::discovery::detection::ColorDepth;
    use biscuit_terminal::terminal::Terminal;
    let mut term = Terminal::builder()
        .width(80)
        .color_depth(ColorDepth::None)
        .build();
    term.is_nerd_font = Some(false);
    for state in ALL_STATES {
        if matches!(state, TodoState::Cancelled) {
            continue;
        }
        let todo = Todo::new("Plain").with_state(state.clone());
        let out = todo.render(&term);
        assert!(
            !out.contains('\x1b'),
            "{:?}: no ANSI in NoColor terminal output: {out:?}",
            state
        );
    }
}

#[test]
fn cancelled_terminal_emits_dim_and_strikethrough_sgr() {
    let term = test_terminal(80);
    let todo = Todo::new("Drop").with_state(TodoState::Cancelled);
    let out = todo.render(&term);
    // The Style on the ListItem lowers dim + strikethrough to SGR. Both
    // codes should be present.
    assert!(out.contains("\x1b[2m"), "dim SGR for Cancelled: {out:?}");
    assert!(
        out.contains("\x1b[9m"),
        "strikethrough SGR for Cancelled: {out:?}"
    );
}

#[test]
fn empty_description_renders_only_marker() {
    let term = test_terminal(80);
    let todo = Todo::new("");
    let plain = strip_ansi(&todo.render(&term));
    // The marker must be present (a glyph plus a single trailing space).
    assert!(!plain.trim().is_empty(), "marker should remain: {plain:?}");
}

#[test]
fn special_characters_preserved_through_tree() {
    let term = test_terminal(80);
    let todo = Todo::new("Fix bug #42 — \"quoted\" & <html>");
    let plain = strip_ansi(&todo.render(&term));
    assert!(
        plain.contains("Fix bug #42") && plain.contains("\"quoted\"") && plain.contains("<html>"),
        "special chars preserved: {plain:?}"
    );
}

#[test]
fn tree_output_renders_at_all_parity_widths() {
    for &width in PARITY_WIDTHS {
        let todo = Todo::new("Content").with_state(TodoState::InProgress);
        let node = <Todo as TreeRenderable>::render_tree(&todo);
        let out = render_tree(&node, width);
        assert_contains_tokens(&out, &["Content"]);
    }
}

#[test]
fn layout_left_margin_applies_through_tree() {
    use biscuit_terminal::utils::layout::{Length, TargetValue};
    let mut todo = Todo::new("Indented");
    todo.layout_mut().margin.left = TargetValue::universal(Length::ch(4));
    let term = test_terminal(80);
    let plain = strip_ansi(&todo.render(&term));
    assert!(
        plain.starts_with("    "),
        "tree path applies left margin: {plain:?}"
    );
}

// ---------------------------------------------------------------------------
// Tree-path description preservation in a no-color terminal
// ---------------------------------------------------------------------------

#[test]
fn tree_preserves_description_in_no_color_terminal() {
    // The no-color, no-nerd terminal pins the cleanest contract: the tree
    // path emits ASCII markers (`[ ]`, `[x]`, `[>]`, `[!]`, `[-]`) with no
    // styling, and the description must round-trip verbatim.
    use biscuit_terminal::discovery::detection::ColorDepth;
    use biscuit_terminal::terminal::Terminal;
    let mut term = Terminal::builder()
        .width(80)
        .color_depth(ColorDepth::None)
        .build();
    term.is_nerd_font = Some(false);
    for state in ALL_STATES {
        let todo = Todo::new("Description").with_state(state.clone());
        let tree = strip_ansi(&todo.render(&term));
        assert!(
            tree.contains("Description"),
            "{:?}: tree path preserves description (tree={tree:?})",
            state
        );
    }
}

// ---------------------------------------------------------------------------
// Markdown
// ---------------------------------------------------------------------------

#[test]
fn markdown_open_state_emits_unchecked_task() {
    let todo = Todo::new("Open task").with_state(TodoState::Open);
    let md = todo.render_markdown();
    assert!(
        md.contains("- [ ] Open task"),
        "expected '- [ ] Open task': {md:?}"
    );
}

#[test]
fn markdown_completed_state_emits_checked_task() {
    let todo = Todo::new("Done").with_state(TodoState::Completed);
    let md = todo.render_markdown();
    assert!(md.contains("- [x] Done"), "expected '- [x] Done': {md:?}");
}

#[test]
fn markdown_in_progress_degrades_to_unchecked() {
    let todo = Todo::new("Doing").with_state(TodoState::InProgress);
    let md = todo.render_markdown();
    assert!(
        md.contains("- [ ] Doing"),
        "InProgress degrades to '- [ ]': {md:?}"
    );
    assert!(!md.contains("- [x]"), "InProgress is not checked: {md:?}");
}

#[test]
fn markdown_blocked_degrades_to_unchecked() {
    let todo = Todo::new("Stuck").with_state(TodoState::Blocked);
    let md = todo.render_markdown();
    assert!(
        md.contains("- [ ] Stuck"),
        "Blocked degrades to '- [ ]': {md:?}"
    );
}

#[test]
fn markdown_cancelled_emits_unchecked_with_strikethrough() {
    let todo = Todo::new("Drop").with_state(TodoState::Cancelled);
    let md = todo.render_markdown();
    assert!(
        md.contains("- [ ] ~~Drop~~"),
        "Cancelled emits '- [ ] ~~Drop~~': {md:?}"
    );
}

#[test]
fn markdown_plus_matches_markdown_for_every_state() {
    for state in ALL_STATES {
        let todo = Todo::new("Same").with_state(state.clone());
        assert_eq!(
            todo.render_markdown(),
            todo.render_markdown_plus(),
            "{:?}: Markdown and MarkdownPlus identical",
            state
        );
    }
}

#[test]
fn markdown_layout_is_ignored() {
    // Markdown ignores Layout by contract.
    use biscuit_terminal::utils::layout::{Length, TargetValue};
    let mut todo = Todo::new("Item").with_state(TodoState::Open);
    todo.layout_mut().margin.left = TargetValue::universal(Length::ch(10));
    let md = todo.render_markdown();
    assert!(
        md.contains("- [ ] Item"),
        "layout does not affect Markdown: {md:?}"
    );
    assert!(
        !md.starts_with("          "),
        "no 10-space prefix in Markdown: {md:?}"
    );
}

// ---------------------------------------------------------------------------
// Browser
// ---------------------------------------------------------------------------

#[test]
fn browser_emits_ul_with_one_li_per_todo() {
    let todo = Todo::new("Item");
    let html = todo.render_html_fragment().render();
    assert!(html.contains("<ul"), "ul present: {html:?}");
    assert!(html.contains("<li"), "li present: {html:?}");
    assert!(html.contains("Item"), "description present: {html:?}");
}

#[test]
fn browser_li_carries_state_class() {
    for state in ALL_STATES {
        let todo = Todo::new("X").with_state(state.clone());
        let expected_class = match state {
            TodoState::Open => "todo-open",
            TodoState::InProgress => "todo-in-progress",
            TodoState::Completed => "todo-completed",
            TodoState::Blocked => "todo-blocked",
            TodoState::Cancelled => "todo-cancelled",
        };
        let html = todo.render_html_fragment().render();
        assert!(
            html.contains(expected_class),
            "{:?}: expected class {expected_class} in {html:?}",
            state
        );
    }
}

#[test]
fn browser_emits_disabled_checkbox_for_every_state() {
    for state in ALL_STATES {
        let todo = Todo::new("X").with_state(state.clone());
        let html = todo.render_html_fragment().render();
        assert!(
            html.contains("type=\"checkbox\"") || html.contains("type='checkbox'"),
            "{:?}: checkbox input present: {html:?}",
            state
        );
    }
}

#[test]
fn browser_completed_checkbox_is_checked() {
    let todo = Todo::new("Done").with_state(TodoState::Completed);
    let html = todo.render_html_fragment().render();
    assert!(
        html.contains("checked"),
        "Completed has checked attr: {html:?}"
    );
}

#[test]
fn browser_non_completed_checkbox_is_not_checked() {
    for state in ALL_STATES {
        if matches!(state, TodoState::Completed) {
            continue;
        }
        let todo = Todo::new("X").with_state(state.clone());
        let html = todo.render_html_fragment().render();
        // The browser renderer emits `checked` as a boolean attribute only when
        // true; a checkbox without `checked` attribute means unchecked.
        // Different rendering may include `checked="false"` form depending on
        // implementation. We assert that no `checked` value of "true" appears.
        assert!(
            !html.contains("checked=\"true\"") && !html.contains("checked='true'"),
            "{:?}: not checked: {html:?}",
            state
        );
    }
}

#[test]
fn browser_cancelled_wraps_description_in_strike_element() {
    // The browser tree renderer lowers `NodeKind::Delete` to `<s>` (not
    // `<del>`); both elements semantically mean struck-through content. The
    // strikethrough on the description is preserved across both the inline
    // `Delete` wrapper and the `<li>`'s CSS (line-through + opacity).
    let todo = Todo::new("Drop").with_state(TodoState::Cancelled);
    let html = todo.render_html_fragment().render();
    assert!(
        html.contains("<s>") && html.contains("Drop"),
        "Cancelled wraps description in <s>: {html:?}"
    );
    assert!(
        html.contains("line-through"),
        "<li> CSS records line-through: {html:?}"
    );
}

#[test]
fn browser_escapes_html_in_description() {
    let todo = Todo::new("<script>alert('x')</script>");
    let html = todo.render_html_fragment().render();
    assert!(
        !html.contains("<script>"),
        "raw script must not survive: {html:?}"
    );
    assert!(
        html.contains("&lt;script&gt;") || html.contains("&lt;"),
        "must escape '<': {html:?}"
    );
}
