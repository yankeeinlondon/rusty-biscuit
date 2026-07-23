---
prompt: |-
    Do a deep dive on the `pulldown-cmark` crate:

    - give a robust description of it's functional footprint
    - what set's it apart from other types of Markdown parsers
        - where is it better than a solution like the `markdown` crate? When is `markdown` the better crate to use?
    - identify 5-6 common use cases which `pulldown-cmark` is used for
        - for each use case provide a simple code example that shows how the rust program would interact with this crate
    - what "gotchas" do developers report running into when using tower-lsp? How can these obstacles be avoided?
    - list all the feature flags which are exposed and what each feature flag provides
last_updated: 2026-05-02
---
## `pulldown-cmark`

## Contents

- What Sets It Apart
- Common Use Cases
- Common Gotchas
- Cargo Feature Flags
- Runtime Parser Options
- Sources

Use heading search to jump to the listed topic.

`pulldown-cmark` is a fast, allocation-conscious CommonMark parser for Rust. Its core API is a pull parser: `Parser` implements `Iterator<Item = Event>`, where block and inline structure is represented as a stream of `Event::Start(Tag)`, `Event::End(TagEnd)`, text, code, HTML, links, images, breaks, rules, footnote references, math, and similar events.

Its functional footprint is centered on:

- CommonMark parsing with a goal of full spec compliance.
- Optional Markdown extensions via `Options`.
- Streaming/event-driven inspection and transformation.
- Built-in Markdown-to-HTML rendering when the `html` feature is enabled.
- Source-position support through offset iterators.
- Broken reference-link callbacks.
- Copy-on-write text handling through `CowStr`, avoiding many unnecessary allocations.
- A small CLI binary, enabled by the default `getopts` feature.

It is not an AST-first Markdown toolkit. It is best understood as a Markdown event stream plus an HTML renderer.

## What Sets It Apart

The main differentiator is the pull-parser design. Many Markdown libraries parse into a tree first, then expose transforms or rendering from that tree. `pulldown-cmark` instead yields events lazily, so callers can render, inspect, filter, rewrite, or collect only what they need.

That makes it especially strong when you want:

- Fast Markdown-to-HTML conversion.
- Low-allocation parsing.
- Streaming-style transformations.
- Simple extraction tasks such as headings, links, images, or code blocks.
- Custom rendering pipelines without building a full syntax tree.
- Rust iterator ergonomics: `map`, `filter`, `scan`, `collect`, `peekable`, and friends.

Compared with the `markdown` crate, `pulldown-cmark` is usually better when the task is event-oriented and performance-sensitive: render this Markdown, rewrite these links, extract headings, strip images, collect code blocks, generate a table of contents, or inspect source ranges.

The `markdown` crate is usually better when you want a complete syntax tree. It exposes mdast, supports CommonMark, GFM, MDX, frontmatter, and math, and is designed for complex Markdown tooling where tree shape, positions, and structural transforms matter more than lightweight streaming. If you are building something like an MDX processor, formatter, linter, document editor, or AST-to-AST transform, `markdown` is often the more natural choice.

A practical rule:

- Use `pulldown-cmark` for fast render/extract/stream/transform pipelines.
- Use `markdown` when the document tree itself is the product.

## Common Use Cases

### 1. Render Markdown To HTML

```rust
use pulldown_cmark::{html, Parser};

fn main() {
    let input = "# Hello\n\nThis is **Markdown**.";
    let parser = Parser::new(input);

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    println!("{html_output}");
}
```

### 2. Render With Extensions

Extensions are off by default. Enable them with `Options`.

```rust
use pulldown_cmark::{html, Options, Parser};

fn main() {
    let input = "\
| Task | Done |
| --- | --- |
| Write docs | yes |

- [x] parse ~~markdown~~
";

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH;

    let parser = Parser::new_ext(input, options);

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    println!("{html_output}");
}
```

### 3. Extract Headings For A Table Of Contents

```rust
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

fn main() {
    let input = "# Intro\n\n## Install\n\nText\n\n## Usage";

    let mut in_heading: Option<HeadingLevel> = None;
    let mut current = String::new();

    for event in Parser::new(input) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = Some(level);
                current.clear();
            }
            Event::Text(text) | Event::Code(text) if in_heading.is_some() => {
                current.push_str(&text);
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(level) = in_heading.take() {
                    println!("{level:?}: {current}");
                }
            }
            _ => {}
        }
    }
}
```

### 4. Rewrite Links During Rendering

```rust
use pulldown_cmark::{html, CowStr, Event, Parser, Tag};

fn main() {
    let input = "Read [the guide](/guide).";

    let parser = Parser::new(input).map(|event| match event {
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) if dest_url.starts_with('/') => Event::Start(Tag::Link {
            link_type,
            dest_url: CowStr::from(format!("https://example.com{dest_url}")),
            title,
            id,
        }),
        event => event,
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    println!("{html_output}");
}
```

### 5. Sanitize User Markdown After Rendering

`pulldown-cmark` renders HTML; it does not sanitize it. For untrusted input, sanitize the output.

```rust
use pulldown_cmark::{html, Parser};

fn main() {
    let input = r#"# Hi

<script>alert("bad")</script>

[ok](https://example.com)
"#;

    let parser = Parser::new(input);

    let mut rendered = String::new();
    html::push_html(&mut rendered, parser);

    let safe = ammonia::clean(&rendered);
    println!("{safe}");
}
```

### 6. Use Source Ranges For Diagnostics

```rust
use pulldown_cmark::{Event, Parser, Tag};

fn main() {
    let input = "Visit [Rust](https://www.rust-lang.org/) now.";

    for (event, range) in Parser::new(input).into_offset_iter() {
        if let Event::Start(Tag::Link { dest_url, .. }) = event {
            println!("link to {dest_url} came from bytes {range:?}");
        }
    }
}
```

## Common Gotchas

The prompt mentions `tower-lsp`, but that appears unrelated to this crate. Interpreting this section as `pulldown-cmark` gotchas:

- Extensions are off by default. Tables, task lists, strikethrough, footnotes, math, heading attributes, wikilinks, and metadata blocks require `Parser::new_ext` with explicit `Options`.
- `Event::End` now carries `TagEnd`, not `Tag`. Older examples may match `Event::End(Tag::Paragraph)`; modern code should match `Event::End(TagEnd::Paragraph)`.
- Consecutive `Event::Text` values can appear. Use `TextMergeStream` or collect text deliberately when contiguous user-visible text matters.
- The HTML renderer does not sanitize. Use a sanitizer such as `ammonia` for untrusted Markdown or disable/pass through raw HTML according to your own policy.
- The `html` module depends on the `html` Cargo feature. If using `default-features = false`, enable `features = ["html"]` or write your own renderer.
- Rendering directly to stdout, files, or sockets can be slow because the renderer performs many small writes. Use `String`, `Vec<u8>`, or `BufWriter`.
- Event transforms must preserve structural balance. If you drop `Event::Start(Tag::Image { .. })`, also drop the matching `Event::End(TagEnd::Image)`.
- Source ranges are byte ranges, not character indices. Convert carefully when reporting line/column diagnostics.
- It is not a Markdown emitter. For parse-transform-write-Markdown workflows, pair it with `pulldown-cmark-to-cmark` or use an AST-oriented crate.
- It is CommonMark plus selected extensions, not a full clone of every platform flavor. If exact GitHub, Obsidian, MDX, or custom dialect behavior is required, test those cases explicitly.

## Cargo Feature Flags

As of `pulldown-cmark` `0.13.3`, the exposed Cargo features are:

| Feature                 | Default            | Provides                                                                                                                                                    |
|-------------------------|--------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `default`               | yes                | Enables `getopts` and `html`.                                                                                                                               |
| `getopts`               | yes                | Enables the optional `getopts` dependency and the `pulldown-cmark` CLI binary.                                                                              |
| `html`                  | yes                | Enables built-in HTML rendering support and the `pulldown-cmark-escape` dependency.                                                                         |
| `pulldown-cmark-escape` | yes through `html` | Enables the optional escaping helper crate used by HTML output. Usually consumed through `html`.                                                            |
| `gen-tests`             | no                 | Internal/test-generation feature; it does not expose additional public crate functionality.                                                                 |
| `serde`                 | no                 | Enables the optional `serde` dependency for serializing/deserializing supported public data structures.                                                     |
| `simd`                  | no                 | Enables SIMD acceleration in `pulldown-cmark-escape` when that dependency is active. Intended for higher-performance release builds on supported platforms. |

Do not confuse Cargo features with parser `Options`. Cargo features change what code is compiled. `Options` change what Markdown syntax the parser recognizes at runtime.

## Runtime Parser Options

Important `Options` flags include:

| Option                                    | Purpose                                                                      |
|-------------------------------------------|------------------------------------------------------------------------------|
| `ENABLE_TABLES`                           | GitHub-style pipe tables.                                                    |
| `ENABLE_FOOTNOTES`                        | GitHub-compatible footnotes.                                                 |
| `ENABLE_OLD_FOOTNOTES`                    | Older footnote parsing behavior; implies footnote support.                   |
| `ENABLE_STRIKETHROUGH`                    | `~~deleted text~~`.                                                          |
| `ENABLE_TASKLISTS`                        | GitHub-style task list checkboxes.                                           |
| `ENABLE_SMART_PUNCTUATION`                | Replaces ASCII punctuation with typographic punctuation.                     |
| `ENABLE_HEADING_ATTRIBUTES`               | Allows heading IDs, classes, and attributes.                                 |
| `ENABLE_YAML_STYLE_METADATA_BLOCKS`       | YAML-style metadata delimited by `---`.                                      |
| `ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS` | Metadata delimited by `+++`.                                                 |
| `ENABLE_MATH`                             | Emits inline and display math events.                                        |
| `ENABLE_GFM`                              | Miscellaneous GFM behavior, currently including alert-style blockquote tags. |
| `ENABLE_DEFINITION_LIST`                  | Definition lists.                                                            |
| `ENABLE_SUPERSCRIPT`                      | Superscript parsing.                                                         |
| `ENABLE_SUBSCRIPT`                        | Subscript parsing.                                                           |
| `ENABLE_WIKILINKS`                        | Obsidian-style wikilinks.                                                    |

## Sources

- [`pulldown-cmark` crate docs](https://docs.rs/crate/pulldown-cmark/latest)
- [`pulldown-cmark` feature flags](https://docs.rs/crate/pulldown-cmark/latest/features)
- [`pulldown_cmark::Options`](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html)
- [`pulldown-cmark` README](https://github.com/pulldown-cmark/pulldown-cmark)
- [`markdown` crate docs](https://docs.rs/crate/markdown/latest)
- [`markdown` public API docs](https://docs.rs/markdown/latest/markdown/)
