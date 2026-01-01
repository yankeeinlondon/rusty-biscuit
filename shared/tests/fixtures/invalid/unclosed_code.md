# Document with Unclosed Code Blocks

This document contains edge cases with code blocks that are not properly closed.

## Scenario 1: Missing Closing Fence

Here's a code block that never closes:

```rust
fn main() {
    println!("This code block is never closed");
}

The parser should handle this gracefully. What happens to the rest of the document?

## Scenario 2: Mismatched Fences

```javascript
const x = 42;
``

This uses two backticks to close instead of three.

## Scenario 3: Nested Code Blocks

```markdown
# This is inside a code block

```rust
// What happens with this nested block?
fn nested() {}
```

Back to the outer block?
```

## Scenario 4: Code Block with Invalid Language

```thisisnotareallanguage!!!
some code here
```

Should still parse as a code block, just with an unusual language identifier.

## Scenario 5: Indented Code Block Followed by Fenced

    This is an indented code block
    It uses 4 spaces

```rust
// And this is a fenced code block
fn fenced() {}
```

Both should be recognized separately.

## Scenario 6: Unclosed at End of File

Here's what happens if the file ends with an unclosed code block:

```python
def incomplete():
    print("This function and code block are never closed")
