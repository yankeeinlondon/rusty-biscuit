# Setup & feature flags

## Cargo.toml essentials

`viuer` focuses on rendering; you typically pair it with `image` for decoding.

````toml
[dependencies]
viuer = "0.11"
image = "0.25"
````

## Feature flags you may need

### `print-file` (enables `print_from_file`)

If you want to print directly from a path without manually decoding via `image`:

````toml
[dependencies]
viuer = { version = "0.11", features = ["print-file"] }
image = "0.25"
````

### `sixel` (enables Sixel protocol support)

Only enable if you want Sixel support and accept extra deps:

````toml
[dependencies]
viuer = { version = "0.11", features = ["sixel"] }
image = "0.25"
````

You can combine:

````toml
viuer = { version = "0.11", features = ["print-file", "sixel"] }
````

## Minimal compile check snippet

````rust
use viuer::{Config, print};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open("image.jpg")?;
    print(&img, &Config::default())?;
    Ok(())
}
````

## Notes on dependencies & environment

* viuer internally uses terminal/control crates (`crossterm`, `console`, etc.).
* Capability detection depends on environment + TTY context; behavior differs in CI, pipes, and redirected output.