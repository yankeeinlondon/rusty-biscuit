//! Test-only binary that renders a sample dirty-file tree through Prose.
//!
//! Used by Level 2 tests to verify the tree markup survives a real terminal's
//! display path (SGR color codes, box-drawing glyphs, etc.).

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use std::path::PathBuf;

fn main() {
    // Source and non-source files, and a non-last directory, so every
    // connector glyph and both file colors appear.
    let paths = vec![
        PathBuf::from("src/lib.rs"),
        PathBuf::from("docs/guide.md"),
        PathBuf::from("README.md"),
    ];
    let markup = worktree_cli::commands::dirty_tree::render_markup(&paths);
    let terminal = Terminal::default();
    println!("{}", Prose::new(markup).render(&terminal));
}
