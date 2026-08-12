# Core usage patterns

## 1) Print a decoded image (`DynamicImage`)

Most common: decode with `image`, then print.

````rust
use viuer::{print, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open("image.jpg")?;

    let config = Config {
        x: 10,
        y: 4,
        ..Default::default()
    };

    print(&img, &config)?;
    Ok(())
}
````

## 2) Print directly from a file (feature: `print-file`)

Use when you don’t need custom decode steps.

````rust
use viuer::{print_from_file, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        width: Some(80),
        height: Some(25),
        ..Default::default()
    };

    print_from_file("img.jpg", &config)?;
    Ok(())
}
````

If you get a compile error that `print_from_file` doesn’t exist, enable the feature in `Cargo.toml`:

* `viuer = { version = "0.11", features = ["print-file"] }`

## 3) Read an image from stdin

Useful for piping, HTTP fetches, or generated images.

````rust
use std::io::{self, Read};
use viuer::{print, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;

    let img = image::load_from_memory(&buf)?;
    print(&img, &Config::default())?;
    Ok(())
}
````

## 4) Transparency + better color

````rust
use viuer::{print_from_file, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        transparent: true,
        truecolor: true,
        ..Default::default()
    };
    print_from_file("transparent.png", &config)?;
    Ok(())
}
````

Be aware: true transparency depends on the terminal protocol; the half-block fallback cannot fully preserve transparency.