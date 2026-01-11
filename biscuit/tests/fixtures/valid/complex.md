# Complex Markdown Document

This document contains all major markdown features for comprehensive testing.

## Headers

### Level 3 Header

#### Level 4 Header

##### Level 5 Header

###### Level 6 Header

## Text Formatting

This paragraph contains **bold text**, *italic text*, ***bold and italic***, ~~strikethrough~~, and `inline code`.

## Lists

### Unordered List

- Top level item 1
  - Nested item 1.1
  - Nested item 1.2
    - Double nested 1.2.1
- Top level item 2
- Top level item 3

### Ordered List

1. First item
2. Second item
   1. Nested ordered item 2.1
   2. Nested ordered item 2.2
3. Third item

### Task List

- [x] Completed task
- [ ] Incomplete task
- [ ] Another incomplete task

## Code Blocks

### Rust Code

```rust
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    map.insert("key", "value");
    println!("{:?}", map);
}
```

### JavaScript Code

```javascript
const greet = (name) => {
  console.log(`Hello, ${name}!`);
};

greet("World");
```

### Plain Code Block

```
This is a plain code block
without syntax highlighting
```

## Tables

| Feature       | Status      | Priority |
|---------------|-------------|----------|
| Parsing       | In Progress | High     |
| Serialization | Pending     | Medium   |
| Validation    | Completed   | Low      |

## Links and Images

### Links

- [Rust Programming Language](https://www.rust-lang.org/)
- [GitHub](https://github.com/)
- Reference-style link to [Rust][rust-ref]

[rust-ref]: https://www.rust-lang.org/ "Rust Reference Link"

### Images

![Rust Logo](https://www.rust-lang.org/static/images/rust-logo-blk.svg)

![Alt text for image][image-ref]

[image-ref]: https://via.placeholder.com/150 "Placeholder Image"

## Blockquotes

> This is a blockquote.
> It can span multiple lines.
>
> > Nested blockquote

## Horizontal Rules

---

***

___

## Inline HTML

<div class="custom-class">
  <p>HTML content inside markdown</p>
</div>

## Footnotes

Here's a sentence with a footnote[^1].

[^1]: This is the footnote content.

## Definition Lists

Term 1
: Definition 1

Term 2
: Definition 2a
: Definition 2b

## Escape Characters

Use backslash to escape: \* \_ \` \[ \] \( \) \# \+ \- \. \!

## End

This concludes the complex markdown fixture.
