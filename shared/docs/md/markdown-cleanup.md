# Markdown Cleanup


## Code Examples

### Code Example: Airy Feel

This code example shows how you can ensure that Markdown is both valid and has an "airy" quality to it that some parsers (and most humans) prefer.

> Guarantees that a header, code block, or list is always isolated by blank lines, you need to manually inspect the Event stream and inject a "spacing" logic.

```rust
use pulldown_cmark::{Parser, Options, Event, Tag, TagEnd};
use pulldown_cmark_to_cmark::cmark;

fn main() {
    let messy_markdown = "# Title\n> Quote\n```rust\nfn main() {}\n```\n* List item";

    let mut options = Options::all();
    let parser = Parser::new_ext(messy_markdown, options);

    // We use a simple state tracker to know when to inject blank lines
    let mut last_was_end_block = false;
    
    let filtered_events = parser.flat_map(|event| {
        let mut result = Vec::new();

        match &event {
            // 1. Detect the start of a new block-level element
            Event::Start(tag) if is_block_tag(tag) => {
                if last_was_end_block {
                    // Inject a blank line before the new block starts
                    // Note: cmark handles one newline; we inject an extra one via BlankLine
                    result.push(Event::BlankLine);
                }
                result.push(event);
                last_was_end_block = false;
            }
            // 2. Detect the end of a block
            Event::End(tag) if is_block_end_tag(tag) => {
                result.push(event);
                last_was_end_block = true;
            }
            _ => {
                result.push(event);
                // If we hit text or other inline events, we aren't "between" blocks anymore
                if !matches!(result.last(), Some(Event::BlankLine)) {
                    last_was_end_block = false;
                }
            }
        }
        result
    });

    let mut buf = String::new();
    cmark(filtered_events, &mut buf).expect("Error rendering");

    println!("{}", buf);
}

/// Helper to identify block-level elements that need spacing
fn is_block_tag(tag: &Tag) -> bool {
    matches!(tag, Tag::Heading { .. } | Tag::BlockQuote(_) | Tag::CodeBlock(_) | Tag::List(_))
}

fn is_block_end_tag(tag: &TagEnd) -> bool {
    matches!(tag, TagEnd::Heading(_) | TagEnd::BlockQuote | TagEnd::CodeBlock | TagEnd::List(_))
}
```

### Code Example: Airy and Pretty Tables

```rust
use pulldown_cmark::{Parser, Options, Event, Tag, TagEnd};
use pulldown_cmark_to_cmark::cmark;

fn main() {
    let messy_markdown = "# Title\n> Quote\n```rust\nfn main() {}\n```\n|ID|Name|\n|---|---|\n|1|Alice|\n* Item 1";

    let mut options = Options::all();
    let parser = Parser::new_ext(messy_markdown, options);

    // Step 1: Buffer and Align Tables
    let events = align_tables_in_stream(parser);

    // Step 2: Inject Blank Lines between blocks
    let mut last_was_end_block = false;
    let final_stream = events.into_iter().flat_map(|event| {
        let mut res = Vec::new();
        match &event {
            Event::Start(tag) if is_block_tag(tag) => {
                if last_was_end_block { res.push(Event::BlankLine); }
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
    println!("{}", buf);
}

fn is_block_tag(tag: &Tag) -> bool {
    matches!(tag, Tag::Heading { .. } | Tag::BlockQuote(_) | Tag::CodeBlock(_) | Tag::List(_) | Tag::Table(_))
}

fn is_block_end_tag(tag: &TagEnd) -> bool {
    matches!(tag, TagEnd::Heading(_) | TagEnd::BlockQuote | TagEnd::CodeBlock | TagEnd::List(_) | TagEnd::Table)
}

fn align_tables_in_stream(parser: Parser) -> Vec<Event> {
    let mut out = Vec::new();
    let events: Vec<Event> = parser.collect();
    let mut i = 0;

    while i < events.len() {
        if let Event::Start(Tag::Table(_)) = &events[i] {
            let mut table_events = Vec::new();
            while i < events.len() {
                let ev = events[i].clone();
                table_events.push(ev.clone());
                i += 1;
                if matches!(ev, Event::End(TagEnd::Table)) { break; }
            }
            out.extend(process_single_table(table_events));
        } else {
            out.push(events[i].clone());
            i += 1;
        }
    }
    out
}

fn process_single_table(events: Vec<Event>) -> Vec<Event> {
    let mut col_widths = Vec::new();
    let mut current_col = 0;

    // Pass 1: Measure
    for ev in &events {
        match ev {
            Event::Text(t) => {
                if col_widths.len() <= current_col { col_widths.push(0); }
                col_widths[current_col] = col_widths[current_col].max(t.len());
            }
            Event::End(TagEnd::TableCell) => current_col += 1,
            Event::End(TagEnd::TableRow) => current_col = 0,
            _ => {}
        }
    }

    // Pass 2: Pad
    let mut current_col = 0;
    events.into_iter().map(|ev| match ev {
        Event::Text(t) => {
            let padding = col_widths[current_col] - t.len();
            let padded = format!(" {} {} ", "", t + &" ".repeat(padding)); 
            Event::Text(padded.into())
        }
        Event::End(TagEnd::TableCell) => { current_col += 1; ev }
        Event::End(TagEnd::TableRow) => { current_col = 0; ev }
        _ => ev
    }).collect()
}
```
