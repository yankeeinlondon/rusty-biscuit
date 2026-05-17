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
        if let Some(filter) = &filter
            && !case.name.eq_ignore_ascii_case(filter)
        {
            continue;
        }
        for scenario in scenarios() {
            let (bespoke, tree) = (case.render)(&scenario);
            let title = format!("{} × {} (w{})", case.name, scenario.name, scenario.width);
            print!("{}", side_by_side(&title, &bespoke, &tree, scenario.width));
        }
    }
    println!();
}
