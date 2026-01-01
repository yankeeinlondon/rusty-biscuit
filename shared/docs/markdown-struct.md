# Markdown struct

```rust
struct Markdown {
    frontmatter: HashMap,
    content: String
}
```

- this struct must implement the `From` trait for 
    - actual markdown content (`String` or `&str`)
- this struct must implement the `TryFrom` trait for
    - a file path reference or file URI (e.g., `file::`) to a markdown file (`String` or `&str`)
    - a URL to a resource which is _expected_ to return a Markdown file (`String`, `&str`, `Url`, `&Url`)

## Markdown Cleanup

This struct should implement a `cleanup` method which will cleanup the markdown content.

- this cleanup will be achieved by using `pulldown-cmark-to-cmark` crate alongside the `pulldown-cmark` crate.
  - `pulldown-cmark` acts as the Producer / Parser
  - `pulldown-cmark-to-cmark` acts as the Consumer / Renderer
- this process must ensure the output Markdown is valid CommonMark markdown (allowing for GFM)
- in addition we do want to ensure that a header, code block, or list is always isolated by blank lines, you need to manually inspect the Event stream and inject a "spacing" logic (see example 1).
- Although not a requirement for CommonMark Markdown, we want to ensure that Markdown tables have blank spaces padded into the cell to align the columns of the tables; making them much more readable by humans.
- The final example show how _both_ requirements could be met.

[Markdown Cleanup Examples](./md/markdown-cleanup.md)

## Frontmatter Mutations

- `fm_merge_with<T>(obj: T)`
    - merges the document's markdown with an external dictionary of key/values
    - the external set has _precedence_ and will override what the document started with 
- `fm_defaults<T>(obj: T)`
    - merges the document's markdown with an external dictionary of key/values
    - the document's key/values are given _precedence_ in the merge process; allowing the external dictionary to only _add_ new key/values but not change the document's original key/value pairings

## Output Formats

The `Markdown` struct has several methods added for exporting to different formats:

- `as_string()` - will merge the frontmatter and content together as text content
- `as_ast()` - exports an AST representation of the Markdown file
- `as_html()` - exports the markdown in HTML format, allows theming
- `for_terminal()` - exports the markdown as text but with Terminal escape codes which support theming


### To AST Format

We will use the `markdown-rs` crate to export an **MDAST** AST representation of the markdown content.

- [MDAST Specification](https://github.com/syntax-tree/mdast)

### AST Code Example

```rust
use markdown::{to_mdast, ParseOptions};
use serde_json;

fn main() -> Result<(), String> {
    // 1. Define your Markdown input
    let markdown_input = "# Hello World\n\nThis is a **serialized** AST.";

    // 2. Parse the Markdown into an MDAST (Markdown Abstract Syntax Tree)
    // We use ParseOptions::default(), but you can enable GFM (GitHub Flavored Markdown) here.
    let ast = to_mdast(markdown_input, &ParseOptions::default())?;

    // 3. Serialize the AST to a JSON string
    // markdown-rs nodes implement serde::Serialize by default
    let json_ast = serde_json::to_string_pretty(&ast)
        .map_err(|e| e.to_string())?;

    // 4. Output the result
    println!("{}", json_ast);

    Ok(())
}
```

**NOTE:** make sure you're using at least the 1.x version of the crate (use an alpha or beta if not yet at stable 1.0). Earlier versions do not support MDAST.


### Exporting to HTML and Terminal

To export our content to either HTML (for the browser) or escaped-coded text (for the terminal) we will need the following crates:

- `pulldown-cmark` - parsing and mutation via pull events
- `pulldown-cmark-to-cmark` - for cleaning up Markdown content
- `syntect` - for code (including Markdown) highlighting


Because this is a bit more involved then the other parts of this `struct` we have included these additional knowledge documents:

- [The export Options hash]()
- [Using syntect to output to Terminal]()
- [Using syntext to output to HTML]()
- [Themes]()
- [Theme Strategy]()
- [Integrating pulldown-cmark with syntect]()
