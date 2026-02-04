//! Example: TwoColumn prose layout.
//!
//! Run with: `cargo run -p biscuit-terminal --example two-column-prose`

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;

fn main() {
    let term = Terminal::new();

    let left_column = Prose::new(concat!(
        "{{bold}}Two-Column Prose{{reset}}\n\n",
        "Use short labels and lists on the left to anchor the reader.\n",
        "- Balanced columns\n",
        "- Predictable wrapping\n",
        "- Stacks when narrow"
    ))
    .with_word_wrap(WordWrap::WrapProse(None, None));

    let right_column = Prose::new(concat!(
        "{{bold}}Details{{reset}}\n\n",
        "TwoColumn keeps related text aligned side by side. ",
        "It measures the terminal width at render time and ",
        "uses fallback rendering so content stays readable when ",
        "the window is too narrow."
    ))
    .with_word_wrap(WordWrap::WrapProse(None, None));

    let two_col = TwoColumn::new(left_column, right_column).with_left_percent(0.45);
    let output = two_col.fallback_render(&term);

    println!("{}", output);
}
