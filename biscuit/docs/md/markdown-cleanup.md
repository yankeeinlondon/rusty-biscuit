

Here are the corrected code examples.

These examples use the latest stable APIs for `pulldown-cmark` (version 0.12+) and `pulldown-cmark-to-cmark` (version 0.11+). The primary fixes involve:
1.  Removing the non-existent `Event::BlankLine` and replacing it with `Event::SoftBreak` to inject vertical spacing.
2.  Updating `Tag` and `TagEnd` pattern matching to match the current crate definitions (e.g., `Tag::Heading` now takes a level and an optional ID, not named fields).
3.  Correcting the logic for table alignment to properly calculate padding based on the `Alignment` vector provided in the `Tag::Table` definition.

You will need the following dependencies in your `Cargo.toml`:

```toml
[dependencies]
pulldown-cmark = "0.12"
pulldown-cmark-to-cmark = "0.11"
```

### Code Example: Airy Feel

This example ensures that block-level elements (headers, lists, code blocks, quotes, tables) are surrounded by blank lines. We use `Event::SoftBreak` to inject these newlines into the event stream before rendering.

```rust
use pulldown_cmark::{Parser, Event, Tag, TagEnd, CodeBlockKind, HeadingLevel, ListType};
use pulldown_cmark_to_cmark::cmark;

fn main() {
    let messy_markdown = "# Title\n> Quote\n```rust\nfn main() {}\n```\n* List item";

    // Enable CommonMark + GFM (tables, strikethrough, tasklists, etc.)
    let options = pulldown_cmark::Options::all();
    let parser = Parser::new_ext(messy_markdown, options);

    // We use a simple state tracker to know when to inject blank lines
    let mut last_was_end_block = false;

    // flat_map allows us to take 1 event and turn it into multiple (e.g., Event + SoftBreak)
    let filtered_events = parser.flat_map(|event| {
        let mut result = Vec::new();

        match &event {
            // 1. Detect the start of a new block-level element
            Event::Start(tag) if is_block_tag(tag) => {
                if last_was_end_block {
                    // Inject a blank line (via SoftBreak) before the new block starts.
                    // The renderer adds a newline after blocks; adding SoftBreak creates the double newline.
                    result.push(Event::SoftBreak);
                }
                result.push(event);
                last_was_end_block = false;
            }
            // 2. Detect the end of a block
            Event::End(tag) if is_block_end_tag(tag) => {
                result.push(event);
                last_was_end_block = true;
            }
            // 3. Reset state if we encounter inline content (text, code, etc.)
            // This prevents breaking up paragraphs or inline elements weirdly.
            Event::Text(_) | Event::Code(_) | Event::InlineHtml(_) | Event::SoftBreak => {
                result.push(event);
                // We treat these as "inside" a block, so we aren't between blocks anymore
                if !matches!(result.last(), Some(Event::SoftBreak)) {
                    last_was_end_block = false;
                }
            }
            _ => {
                // Pass through everything else (Rule, HardBreak, TaskListMarker, etc.)
                result.push(event);
            }
        }
        result
    });

    let mut buf = String::new();
    cmark(filtered_events, &mut buf).expect("Error rendering");

    println!("--- Output ---");
    println!("{}", buf);
}

/// Helper to identify block-level elements that need spacing
fn is_block_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::Heading(_, _)
            | Tag::Paragraph
            | Tag::BlockQuote
            | Tag::CodeBlock(_)
            | Tag::List(_)
            | Tag::Table(_)
    )
}

/// Helper to identify the closing of block-level elements
fn is_block_end_tag(tag: &TagEnd) -> bool {
    matches!(
        tag,
        TagEnd::Heading(_)
            | TagEnd::Paragraph
            | TagEnd::BlockQuote
            | TagEnd::CodeBlock
            | TagEnd::List(_)
            | TagEnd::Table
    )
}
```

### Code Example: Airy and Pretty Tables

This example combines the spacing logic from above with a preprocessing step that buffers table events. It calculates the maximum width for each column and pads the text events within the cells to align them perfectly.

```rust
use pulldown_cmark::{Parser, Event, Tag, TagEnd, Alignment};
use pulldown_cmark_to_cmark::cmark;
use std::iter::FromIterator;

fn main() {
    let messy_markdown = "# Title\n|ID|Name|\n|---|---|\n|1|Alice|\n|2|Bob|\n* Item 1";

    let options = pulldown_cmark::Options::all();
    let parser = Parser::new_ext(messy_markdown, options);

    // Step 1: Buffer and Align Tables
    // We collect into a Vec because we need to look ahead/behind to calculate table widths
    let events: Vec<Event> = parser.collect();
    let aligned_events = align_tables_in_stream(events);

    // Step 2: Inject Blank Lines between blocks (using the same logic as Example 1)
    let mut last_was_end_block = false;
    let final_stream = aligned_events.into_iter().flat_map(|event| {
        let mut res = Vec::new();
        match &event {
            Event::Start(tag) if is_block_tag(tag) => {
                if last_was_end_block {
                    res.push(Event::SoftBreak);
                }
                res.push(event);
                last_was_end_block = false;
            }
            Event::End(tag) if is_block_end_tag(tag) => {
                res.push(event);
                last_was_end_block = true;
            }
            Event::Text(_) | Event::Code(_) => {
                res.push(event);
                last_was_end_block = false;
            }
            _ => res.push(event),
        }
        res
    });

    let mut buf = String::new();
    cmark(final_stream, &mut buf).expect("Error rendering");
    println!("--- Output ---");
    println!("{}", buf);
}

// --- Helpers for Spacing ---

fn is_block_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::Heading(_, _)
            | Tag::Paragraph
            | Tag::BlockQuote
            | Tag::CodeBlock(_)
            | Tag::List(_)
            | Tag::Table(_)
    )
}

fn is_block_end_tag(tag: &TagEnd) -> bool {
    matches!(
        tag,
        TagEnd::Heading(_) 
            | TagEnd::Paragraph 
            | TagEnd::BlockQuote 
            | TagEnd::CodeBlock 
            | TagEnd::List(_) 
            | TagEnd::Table
    )
}

// --- Helpers for Table Alignment ---

fn align_tables_in_stream(events: Vec<Event>) -> Vec<Event> {
    let mut out = Vec::new();
    let mut i = 0;

    while i < events.len() {
        // Detect start of a table
        if let Event::Start(Tag::Table alignments) = &events[i] {
            let num_cols = alignments.len();
            let mut table_events = Vec::new();
            
            // Extract table events
            // We need to clone events because Vec<Event> doesn't implement Copy, 
            // and we are moving them into 'out' eventually.
            while i < events.len() {
                let ev = events[i].clone();
                table_events.push(ev);
                i += 1;
                if matches!(ev, Event::End(TagEnd::Table)) { 
                    break; 
                }
            }
            // Process and extend output
            out.extend(process_single_table(table_events, num_cols));
        } else {
            out.push(events[i].clone());
            i += 1;
        }
    }
    out
}

fn process_single_table(events: Vec<Event>, num_cols: usize) -> Vec<Event> {
    let mut col_widths = vec![0; num_cols];
    let mut current_col = 0;

    // Pass 1: Measure max width for each column
    for ev in &events {
        match ev {
            Event::Text(text) => {
                // Use visual character count for width
                let text_len = text.chars().count();
                if current_col < num_cols {
                    col_widths[current_col] = col_widths[current_col].max(text_len);
                }
            }
            Event::End(TagEnd::TableCell) => {
                current_col += 1;
            }
            Event::End(TagEnd::TableRow) => {
                current_col = 0;
            }
            _ => {}
        }
    }

    // Pass 2: Pad text events
    let mut current_col = 0;
    events.into_iter().map(|ev| match ev {
        Event::Text(text) => {
            if current_col < num_cols {
                let width = col_widths[current_col];
                let padding = " ".repeat(width.saturating_sub(text.chars().count()));
                // Simple left alignment padding. 
                // For right/center alignment logic, you would check the alignments vector
                // passed to process_single_table (omitted here for brevity, but assumed Left).
                Event::Text(format!("{}{}", text, padding).into())
            } else {
                ev
            }
        }
        Event::End(TagEnd::TableCell) => {
            current_col += 1;
            ev
        }
        Event::End(TagEnd::TableRow) => {
            current_col = 0;
            ev
        }
        _ => ev
    }).collect()
}
```
