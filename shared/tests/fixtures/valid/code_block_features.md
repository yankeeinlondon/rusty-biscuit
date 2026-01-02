# Code Block Features Test Fixture

## Basic Code Block
```rust
fn main() {}
```

## Code Block with Title
```rust title="Example"
fn example() {}
```

## Code Block with Line Numbers
```rust line-numbering=true
fn line1() {}
fn line2() {}
fn line3() {}
```

## Code Block with Highlighting
```rust highlight=2
fn normal() {}
fn highlighted() {}
fn normal() {}
```

## Code Block with Range Highlighting
```rust highlight=2-4
fn line1() {}
fn line2() {}
fn line3() {}
fn line4() {}
fn line5() {}
```

## Code Block with All Features
```rust title="Complete Example" line-numbering=true highlight=2,4-5
fn line1() {}
fn line2() {}
fn line3() {}
fn line4() {}
fn line5() {}
fn line6() {}
```
