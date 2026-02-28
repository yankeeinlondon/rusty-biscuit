//! TwoColumn example: image on the left, descriptive text on the right.
//!
//! Run with: `cargo run -p biscuit-terminal --example two_column_with_image`

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::terminal_image::{ImageWidth, TerminalImage};
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Detect terminal capabilities (image support, width, etc.).
    let term = Terminal::new();

    // Image is rendered inline when the terminal supports it.
    let image_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/biscuit-terminal.png");
    let left_column = TerminalImage::new(&image_path)?
        .with_width(ImageWidth::Fill)
        .with_alt_text("biscuit-terminal.png");

    // Right-hand column with a short description.
    let description = Prose::new(concat!(
        "{{bold}}biscuit-terminal{{reset}}\n\n",
        "Terminal-aware rendering with capability detection, inline images, \n",
        "Mermaid diagrams, and graceful fallbacks when features are missing."
    ))
    .with_word_wrap(WordWrap::WrapProse(None, None));

    // Compose the two columns and render for the detected terminal width.
    let two_col = TwoColumn::new(left_column, description).with_left_percent(0.35);
    let output = two_col.render(&term);

    println!("{}", output);
    Ok(())
}
