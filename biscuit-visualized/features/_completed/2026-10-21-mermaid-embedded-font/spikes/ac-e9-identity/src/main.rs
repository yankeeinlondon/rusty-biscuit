//! AC-E9 evidence: writes one Mermaid page through every HTML path that does
//! not opt into the embedded font, so the output of a pre-feature tree and of
//! the current tree can be compared byte for byte. Point
//! `BISCUIT_VISUALIZED_CACHE_DIR` at an empty directory for each run.

use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::{HtmlOptions, MermaidMode};

const PAGE: &str = "# Diagrams\n\n```mermaid\nflowchart LR\n    A[Start here] --> B{Is it?}\n    B -->|Yes| C[Café]\n```\n\n```mermaid\nclassDiagram\n    class Animal {\n        +String name\n        +eat() void\n    }\n```\n\n```mermaid\nsequenceDiagram\n    Alice->>Bob: Hello\n```\n";

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&out).expect("create output directory");
    let md: Markdown = PAGE.into();
    let term = Terminal::new_optimistic(100);
    for (name, mode) in [("page-off", MermaidMode::Off), ("page-image", MermaidMode::Image), ("page-text", MermaidMode::Text)] {
        let html = DarkmatterPage::new(&term).with_mermaid_mode(mode).render_to_browser(&md).expect("render_to_browser");
        std::fs::write(out.join(format!("{name}.html")), html).expect("write");
    }
    let html = md.as_html(HtmlOptions::default()).expect("as_html default");
    std::fs::write(out.join("html-default.html"), html).expect("write");
    let mut options = HtmlOptions::default();
    options.mermaid_mode = MermaidMode::Image;
    let html = md.as_html(options).expect("as_html image");
    std::fs::write(out.join("html-image.html"), html).expect("write");
}
