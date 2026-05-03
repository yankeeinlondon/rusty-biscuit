# YAML Parsing

The Rust ecosystem offers different crates to help with process of parsing YAML data. In this document we're focusing on providing an overview of some of those which are most suited for consideration when designing an LSP based solution (like the IWES LSP).

## `serde-saphyr`

The [serde-saphyr](https://github.com/bourumir-wyngs/serde-saphyr) is "type driven parser" for YAML that is used by the [IWES Language Server]().

The distinguishing feature of **serde-saphyr** is its "type-driven parsing" philosophy. Unlike traditional parsers that build an intermediate "Value" tree, serde-saphyr deserializes YAML directly into Rust data structures, rejecting non-matching input immediately. This approach is significantly safer and faster, as it eliminates the overhead of dynamic "any" objects and protects against common YAML exploits.

| Feature | Functional value for IWES / LSP's | 
| ------- | --------------------------------- |
| Snippet Rendering | Automatically generates visual excerpts of the YAML source where errors occur, including line/column markers. |
| Spanned Wrapper | Allows specific fields to retain their exact byte offsets and line/column positions from the source text. |
| Garde/Validator Integration | Provides native hooks for declarative validation, enabling strict constraints on strings and numbers. |
| Zero Copy Support | Enables deserialization into borrowed &str fields, reducing memory allocations in the liwe arena. |
| Anchor Replay | Sophisticated handling of YAML anchors that allows for recursive or reused data structures without performance penalties. |


### Example Interaction

In a standard Rust application, **serde-saphyr** is used much like other Serde-compatible crates, but with a focus on capturing the errors that occur during validation or parsing.

```rust
use serde::Deserialize;
use serde_saphyr::from_str;

#
struct Frontmatter {
    title: String,
    tags: Vec<String>,
    #[serde(default)]
    priority: i32,
}

fn main() {
    let input = "---\ntitle: 'IWE Research'\ntags: [lsp, rust]\npriority: 'high'\n---";
    // This will fail because priority expects an i32, but gets a string.
    let result: Result<Frontmatter, _> = from_str(input);
    
    if let Err(e) = result {
        // Prints a detailed snippet showing the exact location of 'high'
        eprintln!("{}", e); [6, 12]
    }
}
```

### Natural Bridging Approach with IWES

The bridge between **serde-saphyr** and **IWES** should reside in the document ingestion pipeline. When **liwe** detects a frontmatter block (delimited by ---), it should pass the raw string slice to a specialized Metadata parser. This parser uses **serde-saphyr** to populate a struct that mirrors the system's "Simple Schema Configuration".

```rust
use liwe::Arena;
use serde_saphyr::{from_str_valid, Spanned};
use garde::Validate;

#
pub struct IWEFrontmatter {
    #[garde(length(min = 1))]
    pub title: Spanned<String>, // Captures position for navigation 
    pub parent_id: Option<Spanned<String>>,
}

pub fn process_frontmatter(arena: &mut Arena, yaml: &str) -> Result<IWEFrontmatter, String> {
    // from_str_valid performs both deserialization and garde validation
    from_str_valid(yaml).map_err(|e| e.to_string()) [6, 12]
}
```



## `serde_yaml_ng`

The [serde_yaml_ng](https://github.com/acatton/serde-yaml-ng) is the "next generation" of the defacto standard `serde_yaml` parser for using with `serde`. We use this in Darkmatter via the `biscuit-file` library.

## `rlsp-yaml-parser`

The [rlsp-yaml-parser](https://github.com/chdalski/rlsp-yaml-parser) is a YAML 1.2 parser specifically designed for Language Server Protocol implementations.  It focuses on transliterating the formal YAML grammar into parser combinators, ensuring that comments and source spans are treated as first-class data.

This crate is particularly valuable for features like _block folding_ and _clickable links_ because it maintains a high-fidelity representation of the document's physical structure, not just its semantic data.

| Feature | Functional Value for IWES |
| ------- | ---- |
| Span-as-Data | Every node in the AST contains the precise byte and line/column range it occupies. |
| YAML 1.2 Compliance | Supports the full spectrum of YAML 1.2 productions, ensuring stability with complex user input. |
| Event-to-AST Loader | Allows the server to reconstruct a navigable tree from a stream of tokens, essential for "Go-to-Definition". |
| Comment Preservation | Ensures that user comments within the frontmatter are not lost during processing or formatting. |

