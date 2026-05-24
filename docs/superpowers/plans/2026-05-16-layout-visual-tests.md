# Layout Visual Tests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an inspection harness and snapshot-test suite that render every render-tree component under a matrix of layout settings (margins, alignment, width, row-fill, word-wrap), bespoke vs tree, so layout drift is both visible and regression-protected.

**Architecture:** One shared support module per crate (`biscuit-terminal`, `darkmatter`) defines the scenario matrix, per-component case builders, and formatters. A harness example and a snapshot test both include that support file via a module path, so they render through identical code. The harness prints side-by-side ANSI output for humans; the snapshot test captures stacked, ANSI-stripped blocks for `insta`.

**Tech Stack:** Rust, `insta` 1.x (snapshot testing, already a dev-dependency in both crates), `biscuit-terminal` components, `renderable::layout` / `renderable::tree`.

**Spec:** `docs/superpowers/specs/2026-05-16-layout-visual-tests-design.md`

**Deviation from spec:** The spec listed a `margin_auto` scenario using `Margin::Auto`. No such variant exists — `renderable::layout::Margin` is `None | Chars(u32) | Percent(f32) | Offset(..)`. That scenario is dropped; centering is covered by `align_center`. Final matrix: **12 scenarios**.

---

## File Structure

| File | Responsibility |
|------|----------------|
| `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs` | Shared: `Scenario`, `scenarios()`, formatters, `ComponentCase`, `component_cases()` (6 components) |
| `biscuit-terminal/lib/examples/layout_matrix.rs` | Harness binary — side-by-side ANSI output |
| `biscuit-terminal/lib/tests/layout_matrix.rs` | Unit tests for support + snapshot test |
| `biscuit-terminal/lib/tests/snapshots/*.snap` | Generated snapshots (72 files) |
| `darkmatter/lib/tests/layout_matrix_support/mod.rs` | Shared support, scoped to `YamlBlock` (1 component) |
| `darkmatter/lib/examples/layout_matrix.rs` | Harness binary for darkmatter |
| `darkmatter/lib/tests/layout_matrix.rs` | Unit tests + snapshot test |
| `darkmatter/lib/tests/snapshots/*.snap` | Generated snapshots (12 files) |

The support file lives in a **subdirectory** (`layout_matrix_support/mod.rs`) so Cargo does not compile it as a standalone integration-test binary. It carries `#![allow(dead_code)]` because each frontend (example vs test) uses a different subset of its public functions.

---

## Task 1: biscuit-terminal — scenario matrix

**Files:**
- Create: `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs`
- Create: `biscuit-terminal/lib/tests/layout_matrix.rs`

- [ ] **Step 1: Write the failing test**

Create `biscuit-terminal/lib/tests/layout_matrix.rs`:

```rust
//! Unit tests and snapshot tests for the layout visual-test matrix.
//!
//! Regenerate snapshots after intentional changes with:
//! `INSTA_UPDATE=always cargo test -p biscuit-terminal --test layout_matrix`
//! then review the `.snap` diff.

mod layout_matrix_support;

use layout_matrix_support::scenarios;

#[test]
fn scenario_count_is_twelve() {
    assert_eq!(scenarios().len(), 12);
}

#[test]
fn scenario_names_are_unique() {
    let names: Vec<&str> = scenarios().iter().map(|s| s.name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "scenario names must be unique");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p biscuit-terminal --test layout_matrix 2>&1 | tail -5`
Expected: FAIL — compile error, `file not found for module layout_matrix_support`.

- [ ] **Step 3: Create the support module with the scenario matrix**

Create `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs`:

```rust
//! Shared support for the layout visual-test matrix.
//!
//! Included by both the `layout_matrix` harness example and the
//! `layout_matrix` snapshot test so they render through identical code.
#![allow(dead_code)]

use renderable::layout::{Alignment, Layout, Margin, RowFill, WordWrap};

/// One cell of the matrix: a layout configuration applied at a width.
#[derive(Clone)]
pub struct Scenario {
    /// Stable identifier used in harness headers and snapshot names.
    pub name: &'static str,
    /// The full `Layout` applied to the component before rendering.
    pub layout: Layout,
    /// Terminal width, in columns, the component renders at.
    pub width: u32,
}

/// The full scenario list — one layout dimension varied at a time.
pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "baseline",
            layout: Layout::default(),
            width: 80,
        },
        Scenario {
            name: "left_margin_4",
            layout: Layout {
                left_margin: Margin::Chars(4),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "right_margin_4",
            layout: Layout {
                right_margin: Margin::Chars(4),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "top_margin_2",
            layout: Layout {
                top_margin: Margin::Chars(2),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "bottom_margin_2",
            layout: Layout {
                bottom_margin: Margin::Chars(2),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "left_margin_pct_10",
            layout: Layout {
                left_margin: Margin::Percent(10.0),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "align_center",
            layout: Layout {
                alignment: Alignment::Center,
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "align_right",
            layout: Layout {
                alignment: Alignment::Right,
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "row_fill_fill",
            layout: Layout {
                row_fill_strategy: RowFill::Fill,
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "word_wrap_prose",
            layout: Layout {
                word_wrap: WordWrap::WrapProse(None, None),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "width_40",
            layout: Layout::default(),
            width: 40,
        },
        Scenario {
            name: "width_120",
            layout: Layout::default(),
            width: 120,
        },
    ]
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p biscuit-terminal --test layout_matrix 2>&1 | tail -5`
Expected: PASS — `scenario_count_is_twelve` and `scenario_names_are_unique` both pass.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/lib/tests/layout_matrix_support/mod.rs biscuit-terminal/lib/tests/layout_matrix.rs
git commit -m "test(biscuit-terminal): add layout visual-test scenario matrix"
```

---

## Task 2: biscuit-terminal — formatters

**Files:**
- Modify: `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs` (append)
- Modify: `biscuit-terminal/lib/tests/layout_matrix.rs` (append tests)

- [ ] **Step 1: Write the failing tests**

Append to `biscuit-terminal/lib/tests/layout_matrix.rs`:

```rust
use layout_matrix_support::{pad, side_by_side, stacked_stripped, visible_width};

#[test]
fn pad_extends_to_width() {
    assert_eq!(pad("ab", 5), "ab   ");
    assert_eq!(visible_width(&pad("ab", 5)), 5);
}

#[test]
fn pad_is_ansi_aware() {
    let styled = "\x1b[1mab\x1b[0m";
    assert_eq!(visible_width(&pad(styled, 5)), 5);
}

#[test]
fn stacked_stripped_has_no_ansi() {
    let block = stacked_stripped("\x1b[1mhi\x1b[0m", "\x1b[2mbye\x1b[0m");
    assert!(!block.contains('\x1b'), "snapshot block must be ANSI-free");
    assert!(block.contains("hi") && block.contains("bye"));
}

#[test]
fn side_by_side_includes_title_and_both_columns() {
    let out = side_by_side("My Title", "left", "right", 20);
    assert!(out.contains("My Title"));
    assert!(out.contains("left") && out.contains("right"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p biscuit-terminal --test layout_matrix 2>&1 | tail -5`
Expected: FAIL — compile error, unresolved imports `pad`, `side_by_side`, `stacked_stripped`, `visible_width`.

- [ ] **Step 3: Append the formatters to the support module**

Append to `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs`:

```rust
use biscuit_terminal::prelude::strip_escape_codes;

/// Visible (ANSI-stripped) width of a string, in characters.
pub fn visible_width(s: &str) -> usize {
    strip_escape_codes(s).chars().count()
}

/// Pads `s` with trailing spaces to `width` visible cells (ANSI-aware).
pub fn pad(s: &str, width: usize) -> String {
    let visible = visible_width(s);
    if visible >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - visible))
    }
}

/// Formats a bespoke/tree pair side by side, ANSI retained, for the harness.
///
/// The left column is padded to `width` cells — the scenario's render width —
/// so the divider lines up with the right edge of the bespoke output.
pub fn side_by_side(title: &str, bespoke: &str, tree: &str, width: u32) -> String {
    let col = width as usize;
    let bespoke_lines: Vec<&str> = bespoke.lines().collect();
    let tree_lines: Vec<&str> = tree.lines().collect();
    let rows = bespoke_lines.len().max(tree_lines.len());

    let mut out = format!("\n\x1b[1m── {title} ──\x1b[0m\n");
    out.push_str(&format!(
        "\x1b[1;36m{}\x1b[0m \x1b[2m│\x1b[0m \x1b[1;36mTREE\x1b[0m\n",
        pad("BESPOKE", col),
    ));
    for i in 0..rows {
        let left = bespoke_lines.get(i).copied().unwrap_or("");
        let right = tree_lines.get(i).copied().unwrap_or("");
        out.push_str(&format!("{} \x1b[2m│\x1b[0m {right}\n", pad(left, col)));
    }
    out
}

/// Formats a bespoke/tree pair as a stacked, ANSI-stripped block for snapshots.
pub fn stacked_stripped(bespoke: &str, tree: &str) -> String {
    format!(
        "BESPOKE\n{}\n---\nTREE\n{}",
        strip_escape_codes(bespoke).trim_end(),
        strip_escape_codes(tree).trim_end(),
    )
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p biscuit-terminal --test layout_matrix 2>&1 | tail -5`
Expected: PASS — all six tests pass.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/lib/tests/layout_matrix_support/mod.rs biscuit-terminal/lib/tests/layout_matrix.rs
git commit -m "test(biscuit-terminal): add layout matrix formatters"
```

---

## Task 3: biscuit-terminal — component cases

**Files:**
- Modify: `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs` (append)
- Modify: `biscuit-terminal/lib/tests/layout_matrix.rs` (append test)

- [ ] **Step 1: Write the failing test**

Append to `biscuit-terminal/lib/tests/layout_matrix.rs`:

```rust
use layout_matrix_support::component_cases;

#[test]
fn component_case_count_is_six() {
    assert_eq!(component_cases().len(), 6);
}

#[test]
fn every_case_renders_non_empty() {
    for case in component_cases() {
        for scenario in scenarios() {
            let (bespoke, tree) = (case.render)(&scenario);
            assert!(
                !bespoke.is_empty(),
                "{}/{}: bespoke output was empty",
                case.name,
                scenario.name
            );
            assert!(
                !tree.is_empty(),
                "{}/{}: tree output was empty",
                case.name,
                scenario.name
            );
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p biscuit-terminal --test layout_matrix 2>&1 | tail -5`
Expected: FAIL — compile error, unresolved import `component_cases`.

- [ ] **Step 3: Append component cases to the support module**

Append to `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs`:

```rust
use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::progress::Progress;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use renderable::tree::{RenderNode, RenderStrictness, TreeRenderable};

/// A named component with a closure that builds it under a scenario and
/// renders both the bespoke and tree paths.
pub struct ComponentCase {
    /// Component name, used in harness headers and snapshot names.
    pub name: &'static str,
    /// Returns `(bespoke_output, tree_output)`, both with ANSI retained.
    pub render: Box<dyn Fn(&Scenario) -> (String, String)>,
}

/// Folds a `RenderNode` into terminal output at the given width.
fn render_tree_string(node: &RenderNode, width: u32) -> String {
    let term = Terminal::new_optimistic(width);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    match render_terminal_node(node, &opts) {
        Ok(rendered) => rendered.output,
        Err(error) => format!("<render error: {error}>"),
    }
}

/// All six biscuit-terminal components on the render-tree architecture.
pub fn component_cases() -> Vec<ComponentCase> {
    vec![
        ComponentCase {
            name: "Section",
            render: Box::new(|s| {
                let mut section = Section::new(HeadingLevel::h2, "Getting Started");
                section
                    .push("Welcome to the tutorial.")
                    .push("Let's begin with installation.");
                let section = section.with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let bespoke = section.render(&term);
                let tree = section
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (bespoke, tree)
            }),
        },
        ComponentCase {
            name: "UnorderedList",
            render: Box::new(|s| {
                let list = UnorderedList::new(vec![
                    "First item",
                    "Second item",
                    "Third item",
                ])
                .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let bespoke = list.render(&term);
                let tree = list
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (bespoke, tree)
            }),
        },
        ComponentCase {
            name: "TwoColumn",
            render: Box::new(|s| {
                let columns =
                    TwoColumn::new("Left column content.", "Right column content.")
                        .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let bespoke = columns.render(&term);
                let tree = columns
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (bespoke, tree)
            }),
        },
        ComponentCase {
            name: "Progress",
            render: Box::new(|s| {
                let progress = Progress::new(0.75)
                    .with_label("Loading")
                    .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let bespoke = progress.render(&term);
                let tree = progress
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (bespoke, tree)
            }),
        },
        ComponentCase {
            name: "Table",
            render: Box::new(|s| {
                let table = Table::new()
                    .with_columns(vec![
                        TableColumn::new("Name"),
                        TableColumn::new("Score"),
                    ])
                    .with_data(vec![
                        vec![
                            TableCellContent::Text("Ann".into()),
                            TableCellContent::Integer(42),
                        ],
                        vec![
                            TableCellContent::Text("Bob".into()),
                            TableCellContent::Integer(17),
                        ],
                    ])
                    .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let bespoke = table.render(&term);
                let tree = table
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (bespoke, tree)
            }),
        },
        ComponentCase {
            name: "BlockQuote",
            render: Box::new(|s| {
                let quote = BlockQuote::new(
                    "The best way to predict the future is to invent it.".into(),
                    Some("Alan Kay"),
                )
                .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let bespoke = quote.render(&term);
                let tree = render_tree_string(&quote.render_tree(), s.width);
                (bespoke, tree)
            }),
        },
    ]
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p biscuit-terminal --test layout_matrix 2>&1 | tail -8`
Expected: PASS — `component_case_count_is_six` and `every_case_renders_non_empty` pass (the latter exercises 6 × 12 = 72 renders).

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/lib/tests/layout_matrix_support/mod.rs biscuit-terminal/lib/tests/layout_matrix.rs
git commit -m "test(biscuit-terminal): add layout matrix component cases"
```

---

## Task 4: biscuit-terminal — harness example

**Files:**
- Create: `biscuit-terminal/lib/examples/layout_matrix.rs`

- [ ] **Step 1: Create the harness example**

Create `biscuit-terminal/lib/examples/layout_matrix.rs`:

```rust
//! Layout visual-test harness.
//!
//! Renders every render-tree component under the layout scenario matrix,
//! bespoke vs tree, side by side. See the design spec:
//! `docs/superpowers/specs/2026-05-16-layout-visual-tests-design.md`.
//!
//! Run (pipe to a pager — the full matrix is wide and long):
//!
//! ```text
//! cargo run -p biscuit-terminal --example layout_matrix | less -RS
//! cargo run -p biscuit-terminal --example layout_matrix -- table
//! ```
//!
//! An optional argument filters to a single component (case-insensitive).

#[path = "../tests/layout_matrix_support/mod.rs"]
mod support;

use support::{component_cases, scenarios, side_by_side};

fn main() {
    let filter = std::env::args().nth(1);

    for case in component_cases() {
        if let Some(filter) = &filter {
            if !case.name.eq_ignore_ascii_case(filter) {
                continue;
            }
        }
        for scenario in scenarios() {
            let (bespoke, tree) = (case.render)(&scenario);
            let title = format!(
                "{} × {} (w{})",
                case.name, scenario.name, scenario.width
            );
            print!("{}", side_by_side(&title, &bespoke, &tree, scenario.width));
        }
    }
    println!();
}
```

- [ ] **Step 2: Build the example and verify it compiles**

Run: `cargo build -p biscuit-terminal --example layout_matrix 2>&1 | tail -5`
Expected: `Finished` with no errors or warnings.

- [ ] **Step 3: Run the example to verify output**

Run: `cargo run -p biscuit-terminal --example layout_matrix -- section 2>&1 | head -20`
Expected: Side-by-side output headed `── Section × baseline (w80) ──` with `BESPOKE`/`TREE` columns separated by `│`.

- [ ] **Step 4: Commit**

```bash
git add biscuit-terminal/lib/examples/layout_matrix.rs
git commit -m "feat(biscuit-terminal): add layout matrix harness example"
```

---

## Task 5: biscuit-terminal — snapshot test

**Files:**
- Modify: `biscuit-terminal/lib/tests/layout_matrix.rs` (append)
- Create: `biscuit-terminal/lib/tests/snapshots/*.snap` (generated, 72 files)

- [ ] **Step 1: Append the snapshot test**

Append to `biscuit-terminal/lib/tests/layout_matrix.rs`:

```rust
#[test]
fn layout_matrix_snapshots() {
    for case in component_cases() {
        for scenario in scenarios() {
            let (bespoke, tree) = (case.render)(&scenario);
            let block = stacked_stripped(&bespoke, &tree);
            insta::assert_snapshot!(
                format!("{}__{}", case.name, scenario.name),
                block
            );
        }
    }
}
```

- [ ] **Step 2: Generate the snapshots**

Run: `INSTA_UPDATE=always cargo test -p biscuit-terminal --test layout_matrix 2>&1 | tail -5`
Expected: PASS — `INSTA_UPDATE=always` writes 72 `.snap` files into `biscuit-terminal/lib/tests/snapshots/` and the test passes.

- [ ] **Step 3: Verify snapshots are stable on a normal run**

Run: `cargo test -p biscuit-terminal --test layout_matrix 2>&1 | tail -5`
Expected: PASS — with no `INSTA_UPDATE`, the snapshot test compares against the committed `.snap` files and matches.

- [ ] **Step 4: Spot-review generated snapshots**

Run: `ls biscuit-terminal/lib/tests/snapshots/ | wc -l` then inspect two files:
`cat biscuit-terminal/lib/tests/snapshots/layout_matrix__Section__left_margin_4.snap`
`cat biscuit-terminal/lib/tests/snapshots/layout_matrix__TwoColumn__width_120.snap`
Expected: 72 files; each `.snap` contains a `BESPOKE` block, a `---` separator, and a `TREE` block, ANSI-free. (Drift between the two blocks is expected and intentionally recorded — do not "fix" it here.)

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/lib/tests/layout_matrix.rs biscuit-terminal/lib/tests/snapshots/
git commit -m "test(biscuit-terminal): add layout matrix snapshot suite"
```

---

## Task 6: darkmatter — support module

**Files:**
- Create: `darkmatter/lib/tests/layout_matrix_support/mod.rs`
- Create: `darkmatter/lib/tests/layout_matrix.rs`

- [ ] **Step 1: Write the failing tests**

Create `darkmatter/lib/tests/layout_matrix.rs`:

```rust
//! Unit tests and snapshot tests for darkmatter's layout visual-test matrix.
//!
//! Regenerate snapshots after intentional changes with:
//! `INSTA_UPDATE=always cargo test -p darkmatter --test layout_matrix`
//! then review the `.snap` diff.

mod layout_matrix_support;

use layout_matrix_support::{component_cases, scenarios, stacked_stripped};

#[test]
fn scenario_count_is_twelve() {
    assert_eq!(scenarios().len(), 12);
}

#[test]
fn component_case_count_is_one() {
    assert_eq!(component_cases().len(), 1);
}

#[test]
fn every_case_renders_non_empty() {
    for case in component_cases() {
        for scenario in scenarios() {
            let (bespoke, tree) = (case.render)(&scenario);
            assert!(
                !bespoke.is_empty(),
                "{}/{}: bespoke output was empty",
                case.name,
                scenario.name
            );
            assert!(
                !tree.is_empty(),
                "{}/{}: tree output was empty",
                case.name,
                scenario.name
            );
        }
    }
}

#[test]
fn stacked_stripped_has_no_ansi() {
    let block = stacked_stripped("\x1b[1mhi\x1b[0m", "\x1b[2mbye\x1b[0m");
    assert!(!block.contains('\x1b'), "snapshot block must be ANSI-free");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p darkmatter --test layout_matrix 2>&1 | tail -5`
Expected: FAIL — compile error, `file not found for module layout_matrix_support`.

- [ ] **Step 3: Create the darkmatter support module**

Create `darkmatter/lib/tests/layout_matrix_support/mod.rs`:

```rust
//! Shared support for darkmatter's layout visual-test matrix.
//!
//! Scoped to `YamlBlock`, the one darkmatter component on the render-tree
//! architecture. Included by both the `layout_matrix` harness example and the
//! `layout_matrix` snapshot test so they render through identical code.
#![allow(dead_code)]

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::prelude::strip_escape_codes;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::YamlBlock;
use renderable::layout::{Alignment, Layout, Margin, RowFill, WordWrap};
use renderable::tree::{RenderNode, RenderStrictness};

/// One cell of the matrix: a layout configuration applied at a width.
#[derive(Clone)]
pub struct Scenario {
    /// Stable identifier used in harness headers and snapshot names.
    pub name: &'static str,
    /// The full `Layout` applied to the component before rendering.
    pub layout: Layout,
    /// Terminal width, in columns, the component renders at.
    pub width: u32,
}

/// The full scenario list — one layout dimension varied at a time.
pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "baseline",
            layout: Layout::default(),
            width: 80,
        },
        Scenario {
            name: "left_margin_4",
            layout: Layout {
                left_margin: Margin::Chars(4),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "right_margin_4",
            layout: Layout {
                right_margin: Margin::Chars(4),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "top_margin_2",
            layout: Layout {
                top_margin: Margin::Chars(2),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "bottom_margin_2",
            layout: Layout {
                bottom_margin: Margin::Chars(2),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "left_margin_pct_10",
            layout: Layout {
                left_margin: Margin::Percent(10.0),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "align_center",
            layout: Layout {
                alignment: Alignment::Center,
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "align_right",
            layout: Layout {
                alignment: Alignment::Right,
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "row_fill_fill",
            layout: Layout {
                row_fill_strategy: RowFill::Fill,
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "word_wrap_prose",
            layout: Layout {
                word_wrap: WordWrap::WrapProse(None, None),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "width_40",
            layout: Layout::default(),
            width: 40,
        },
        Scenario {
            name: "width_120",
            layout: Layout::default(),
            width: 120,
        },
    ]
}

/// Visible (ANSI-stripped) width of a string, in characters.
pub fn visible_width(s: &str) -> usize {
    strip_escape_codes(s).chars().count()
}

/// Pads `s` with trailing spaces to `width` visible cells (ANSI-aware).
pub fn pad(s: &str, width: usize) -> String {
    let visible = visible_width(s);
    if visible >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - visible))
    }
}

/// Formats a bespoke/tree pair side by side, ANSI retained, for the harness.
pub fn side_by_side(title: &str, bespoke: &str, tree: &str, width: u32) -> String {
    let col = width as usize;
    let bespoke_lines: Vec<&str> = bespoke.lines().collect();
    let tree_lines: Vec<&str> = tree.lines().collect();
    let rows = bespoke_lines.len().max(tree_lines.len());

    let mut out = format!("\n\x1b[1m── {title} ──\x1b[0m\n");
    out.push_str(&format!(
        "\x1b[1;36m{}\x1b[0m \x1b[2m│\x1b[0m \x1b[1;36mTREE\x1b[0m\n",
        pad("BESPOKE", col),
    ));
    for i in 0..rows {
        let left = bespoke_lines.get(i).copied().unwrap_or("");
        let right = tree_lines.get(i).copied().unwrap_or("");
        out.push_str(&format!("{} \x1b[2m│\x1b[0m {right}\n", pad(left, col)));
    }
    out
}

/// Formats a bespoke/tree pair as a stacked, ANSI-stripped block for snapshots.
pub fn stacked_stripped(bespoke: &str, tree: &str) -> String {
    format!(
        "BESPOKE\n{}\n---\nTREE\n{}",
        strip_escape_codes(bespoke).trim_end(),
        strip_escape_codes(tree).trim_end(),
    )
}

/// A named component with a closure that builds it under a scenario and
/// renders both the bespoke and tree paths.
pub struct ComponentCase {
    /// Component name, used in harness headers and snapshot names.
    pub name: &'static str,
    /// Returns `(bespoke_output, tree_output)`, both with ANSI retained.
    pub render: Box<dyn Fn(&Scenario) -> (String, String)>,
}

/// Folds a `RenderNode` into terminal output at the given width.
fn render_tree_string(node: &RenderNode, width: u32) -> String {
    let term = Terminal::new_optimistic(width);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    match render_terminal_node(node, &opts) {
        Ok(rendered) => rendered.output,
        Err(error) => format!("<render error: {error}>"),
    }
}

/// The darkmatter components on the render-tree architecture (`YamlBlock`).
pub fn component_cases() -> Vec<ComponentCase> {
    vec![ComponentCase {
        name: "YamlBlock",
        render: Box::new(|s| {
            let block = YamlBlock::new(
                "name: rusty-biscuit\nversion: 0.1.0\ntags:\n  - cli\n  - terminal",
            )
            .expect("sample YAML is valid")
            .with_layout(s.layout.clone());
            let term = Terminal::new_optimistic(s.width);
            let bespoke = block.render(&term);
            let tree = block
                .render_tree_node()
                .map(|node| render_tree_string(&node, s.width))
                .unwrap_or_else(|| "<no tree projection>".to_string());
            (bespoke, tree)
        }),
    }]
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p darkmatter --test layout_matrix 2>&1 | tail -8`
Expected: PASS — all four tests pass.

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/tests/layout_matrix_support/mod.rs darkmatter/lib/tests/layout_matrix.rs
git commit -m "test(darkmatter): add layout visual-test support module"
```

---

## Task 7: darkmatter — harness example

**Files:**
- Create: `darkmatter/lib/examples/layout_matrix.rs`

- [ ] **Step 1: Create the harness example**

Create `darkmatter/lib/examples/layout_matrix.rs`:

```rust
//! Layout visual-test harness for darkmatter's render-tree components.
//!
//! Renders `YamlBlock` under the layout scenario matrix, bespoke vs tree,
//! side by side. See the design spec:
//! `docs/superpowers/specs/2026-05-16-layout-visual-tests-design.md`.
//!
//! Run (pipe to a pager — the matrix is wide):
//!
//! ```text
//! cargo run -p darkmatter --example layout_matrix | less -RS
//! ```

#[path = "../tests/layout_matrix_support/mod.rs"]
mod support;

use support::{component_cases, scenarios, side_by_side};

fn main() {
    for case in component_cases() {
        for scenario in scenarios() {
            let (bespoke, tree) = (case.render)(&scenario);
            let title = format!(
                "{} × {} (w{})",
                case.name, scenario.name, scenario.width
            );
            print!("{}", side_by_side(&title, &bespoke, &tree, scenario.width));
        }
    }
    println!();
}
```

- [ ] **Step 2: Build and run the example**

Run: `cargo run -p darkmatter --example layout_matrix 2>&1 | head -20`
Expected: Compiles with no warnings; output headed `── YamlBlock × baseline (w80) ──` with `BESPOKE`/`TREE` columns.

- [ ] **Step 3: Commit**

```bash
git add darkmatter/lib/examples/layout_matrix.rs
git commit -m "feat(darkmatter): add layout matrix harness example"
```

---

## Task 8: darkmatter — snapshot test

**Files:**
- Modify: `darkmatter/lib/tests/layout_matrix.rs` (append)
- Create: `darkmatter/lib/tests/snapshots/*.snap` (generated, 12 files)

- [ ] **Step 1: Append the snapshot test**

Append to `darkmatter/lib/tests/layout_matrix.rs`:

```rust
#[test]
fn layout_matrix_snapshots() {
    for case in component_cases() {
        for scenario in scenarios() {
            let (bespoke, tree) = (case.render)(&scenario);
            let block = stacked_stripped(&bespoke, &tree);
            insta::assert_snapshot!(
                format!("{}__{}", case.name, scenario.name),
                block
            );
        }
    }
}
```

- [ ] **Step 2: Generate the snapshots**

Run: `INSTA_UPDATE=always cargo test -p darkmatter --test layout_matrix 2>&1 | tail -5`
Expected: PASS — 12 `.snap` files written into `darkmatter/lib/tests/snapshots/`.

- [ ] **Step 3: Verify snapshots are stable on a normal run**

Run: `cargo test -p darkmatter --test layout_matrix 2>&1 | tail -5`
Expected: PASS — snapshot test matches committed `.snap` files.

- [ ] **Step 4: Spot-review a generated snapshot**

Run: `cat darkmatter/lib/tests/snapshots/layout_matrix__YamlBlock__left_margin_4.snap`
Expected: A `BESPOKE` block, `---` separator, and `TREE` block, ANSI-free. (The bespoke side shows syntax highlighting stripped to plain text; the tree side shows a plain code fence — recorded as-is.)

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/tests/layout_matrix.rs darkmatter/lib/tests/snapshots/
git commit -m "test(darkmatter): add layout matrix snapshot suite"
```

---

## Task 9: Final verification

**Files:** none (verification only)

- [ ] **Step 1: Clippy — biscuit-terminal**

Run: `cargo clippy -p biscuit-terminal --tests --examples 2>&1 | grep -E "^error|^warning:" | head -20`
Expected: No `error` or `warning:` lines for `layout_matrix` files. (A pre-existing unrelated warning elsewhere is acceptable; new ones in the added files are not.)

- [ ] **Step 2: Clippy — darkmatter**

Run: `cargo clippy -p darkmatter --tests --examples 2>&1 | grep -E "^error|^warning:" | head -20`
Expected: No new `error`/`warning:` lines for `layout_matrix` files.

- [ ] **Step 3: Run both snapshot suites clean**

Run: `cargo test -p biscuit-terminal --test layout_matrix && cargo test -p darkmatter --test layout_matrix 2>&1 | tail -10`
Expected: Both test binaries pass — unit tests and `layout_matrix_snapshots` green.

- [ ] **Step 4: Run both harnesses**

Run: `cargo run -p biscuit-terminal --example layout_matrix -- table 2>&1 | head -5`
Run: `cargo run -p darkmatter --example layout_matrix 2>&1 | head -5`
Expected: Both produce side-by-side output without panicking.

- [ ] **Step 5: Confirm working tree is clean**

Run: `git status --short`
Expected: Empty — all work committed across Tasks 1–8.

---

## Self-Review Notes

- **Spec coverage:** Inspection harness (Tasks 4, 7), snapshot tests (Tasks 5, 8), shared `render_pair` path (`ComponentCase::render` in Tasks 3, 6), all 7 components (6 biscuit-terminal + 1 darkmatter), all four setting families (margins, alignment, width, row-fill/word-wrap — Tasks 1, 6 scenarios), side-by-side bespoke vs tree (formatters, Task 2). ANSI stripped in snapshots, retained in harness. Scope is tooling only — drift is recorded, not fixed (Task 5 Step 4, Task 8 Step 4).
- **Deviation:** `margin_auto` dropped — no `Margin::Auto` variant. 12 scenarios, noted in the header.
- **Type consistency:** `Scenario { name, layout, width }`, `ComponentCase { name, render }`, and the `scenarios()` / `component_cases()` / `pad()` / `visible_width()` / `side_by_side()` / `stacked_stripped()` / `render_tree_string()` signatures are identical across both crates' support modules.
