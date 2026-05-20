//! Layout visual-test harness.
//!
//! Renders every render-tree component under the layout scenario matrix,
//! `render(&term)` vs an explicit tree fold, side by side. After the Stage 2
//! IR flip every component's `render()` itself routes through the tree, so
//! the two halves agree by construction; the harness is now an *informational*
//! view that highlights regressions in the public render surface, not a
//! parity oracle. See the design spec:
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
            let (via_render, tree) = (case.render)(&scenario);
            let title = format!("{} × {} (w{})", case.name, scenario.name, scenario.width);
            print!("{}", side_by_side(&title, &via_render, &tree, scenario.width));
        }
    }
    println!();
}
