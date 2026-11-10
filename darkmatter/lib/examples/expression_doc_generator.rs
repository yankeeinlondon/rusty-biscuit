//! Generator for the expression-function documentation rendered from the
//! catalog (`darkmatter/schemas/partials/functions.yaml`). It has two outputs:
//!
//! - the function table in `docs/topics/darkmatter-expressions.md`, the region
//!   between `<!-- BEGIN GENERATED FUNCTION TABLE -->` and
//!   `<!-- END GENERATED FUNCTION TABLE -->`;
//! - one reference page per entry of `EXPRESSION_FUNCTION_PAGES` (for
//!   example `docs/functions/load-data.md`), each written whole.
//!
//! By default it prints the table and then every page to stdout. `--write`
//! writes both outputs; `--write-pages` writes only the function pages and
//! leaves the topic doc untouched.
//!
//! Run with:
//!
//! ```text
//! cargo run -p darkmatter --example expression_doc_generator
//! cargo run -p darkmatter --example expression_doc_generator -- --write
//! cargo run -p darkmatter --example expression_doc_generator -- --write-pages
//! ```

use std::path::PathBuf;

use darkmatter::markdown::compose::expression::{
    EXPRESSION_FUNCTION_PAGES, generate_expression_function_page, generate_expression_function_table,
};

const START_MARKER: &str = "<!-- BEGIN GENERATED FUNCTION TABLE -->";
const END_MARKER: &str = "<!-- END GENERATED FUNCTION TABLE -->";

fn main() {
    let table = generate_expression_function_table();
    let pages: Vec<(&str, String)> = EXPRESSION_FUNCTION_PAGES
        .iter()
        .map(|page| {
            let content = generate_expression_function_page(page.function)
                .unwrap_or_else(|| panic!("`{}` has a page but is not in the catalog", page.function));
            (page.path, content)
        })
        .collect();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let write_all = args.iter().any(|arg| arg == "--write");
    let write_pages = write_all || args.iter().any(|arg| arg == "--write-pages");

    if !write_pages {
        print!("{table}");
        for (_, content) in &pages {
            print!("\n{content}");
        }
        return;
    }

    let area_dir = PathBuf::from(std::env!("CARGO_MANIFEST_DIR")).join("../../darkmatter");

    if write_all {
        let doc_path = area_dir.join("docs/topics/darkmatter-expressions.md");
        let content = std::fs::read_to_string(&doc_path).expect("failed to read darkmatter-expressions.md");
        let start = content.find(START_MARKER).expect("missing start marker") + START_MARKER.len();
        let end = content.find(END_MARKER).expect("missing end marker");
        let new_content = format!("{}\n\n{}\n{}", &content[..start], table.trim_end(), &content[end..]);
        std::fs::write(&doc_path, new_content).expect("failed to write darkmatter-expressions.md");
    }

    for (path, content) in &pages {
        let page_path = area_dir.join(path);
        if let Some(parent) = page_path.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|error| panic!("failed to create {}: {error}", parent.display()));
        }
        std::fs::write(&page_path, content).unwrap_or_else(|error| panic!("failed to write {path}: {error}"));
    }
}
