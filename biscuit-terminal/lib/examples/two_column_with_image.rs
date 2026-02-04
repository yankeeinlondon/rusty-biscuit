//! TwoColumn example: image on the left, descriptive text on the right.
//!
//! Run with: `cargo run -p biscuit-terminal --example two_column_with_image`

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::terminal_image::TerminalImage;
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::terminal::Terminal;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Detect terminal capabilities (image support, width, etc.).
    let term = Terminal::new();

    // Local image (no remote URLs) with an alt text fallback. Path is anchored to
    // the crate manifest directory so it works regardless of the current working
    // directory (e.g., when run from workspace root or target/).
    let image_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/biscuit-terminal.png");
    let image_spec = format!("{}|45%", image_path.display());
    let image = TerminalImage::from_spec(&image_spec)?.with_alt_text("biscuit-terminal logo");

    // TwoColumn renders text content. TerminalImage performs side-effect rendering for
    // inline images, so for the column content we use the alt text. If your terminal
    // supports Kitty/iTerm2 images, you can render inline before/after this layout via
    // `image.render_with_viuer(&term)`.
    let left_column = Prose::new(image.generate_alt_text());

    // Right-hand column with a short description.
    let description = Prose::new(concat!(
        "{{bold}}biscuit-terminal{{reset}}\n\n",
        "Terminal-aware rendering with capability detection, inline images, \n",
        "Mermaid diagrams, and graceful fallbacks when features are missing."
    ));

    // Compose the two columns and render for the detected terminal width.
    let two_col = TwoColumn::new(left_column, description).with_left_percent(0.45);
    let output = two_col.fallback_render(&term);

    println!("{}", output);
    Ok(())
}
