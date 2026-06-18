//! Layout visual-test harness for darkmatter's render-tree components.
//!
//! Renders `CodeBlock::yaml` under the layout scenario matrix, bespoke vs tree,
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
            let title = format!("{} × {} (w{})", case.name, scenario.name, scenario.width);
            print!("{}", side_by_side(&title, &bespoke, &tree, scenario.width));
        }
    }
    println!();
}
