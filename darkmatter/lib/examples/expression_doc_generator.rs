//! Generator for the expression function table in `darkmatter-expressions.md`.
//!
//! By default this prints the generated Markdown table to stdout. With `--write`
//! it replaces the region between `<!-- BEGIN GENERATED FUNCTION TABLE -->` and
//! `<!-- END GENERATED FUNCTION TABLE -->` in the topic doc.
//!
//! Run with:
//!
//! ```text
//! cargo run -p darkmatter --example expression_doc_generator
//! cargo run -p darkmatter --example expression_doc_generator -- --write
//! ```

use std::path::PathBuf;

use darkmatter::markdown::compose::expression::generate_expression_function_table;

const START_MARKER: &str = "<!-- BEGIN GENERATED FUNCTION TABLE -->";
const END_MARKER: &str = "<!-- END GENERATED FUNCTION TABLE -->";

fn main() {
    let generated = generate_expression_function_table();
    let write_mode = std::env::args().skip(1).any(|arg| arg == "--write");

    if write_mode {
        let manifest_dir = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"));
        let doc_path = manifest_dir
            .join("../../darkmatter/docs/topics/darkmatter-expressions.md");

        let content = std::fs::read_to_string(&doc_path)
            .expect("failed to read darkmatter-expressions.md");

        let start = content
            .find(START_MARKER)
            .expect("missing start marker")
            + START_MARKER.len();
        let end = content
            .find(END_MARKER)
            .expect("missing end marker");

        let new_content = format!(
            "{}\n\n{}\n{}",
            &content[..start],
            generated.trim_end(),
            &content[end..]
        );

        std::fs::write(&doc_path, new_content)
            .expect("failed to write darkmatter-expressions.md");
    } else {
        print!("{}", generated);
    }
}
