# Protocols, auto-detection, and compatibility

## Protocols viuer can use (best to worst)

1. Kitty Graphics Protocol (full resolution)
1. iTerm2 Graphics Protocol (full resolution)
1. Sixel (feature-gated)
1. Unicode lower half blocks `▄` (universal fallback)

viuer selects automatically based on the environment (notably `$TERM`) and terminal behavior.

## Check what your terminal supports

````rust
use viuer::{get_kitty_support, is_iterm_supported, KittySupport};

fn main() {
    match get_kitty_support() {
        KittySupport::None => println!("Kitty: not supported"),
        KittySupport::Local => println!("Kitty: local files only"),
        KittySupport::Remote => println!("Kitty: full support"),
    }
    println!("iTerm2 supported: {}", is_iterm_supported());
}
````

Sixel checks require the `sixel` feature:

* `viuer::is_sixel_supported()`

## Force a specific protocol

If auto-detection fails (common in non-TTY contexts), you can force:

````rust
use viuer::{print_from_file, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config { use_kitty: true, ..Default::default() };
    print_from_file("image.png", &config)?;
    Ok(())
}
````

Other forcing knobs:

* `use_iterm: true`
* `use_sixel: true` (requires `sixel` feature)

## Compatibility notes

* If you only see block characters: your terminal likely doesn’t support Kitty/iTerm/Sixel (or you’re in a context where those protocols can’t be used).
* Over SSH: depends on the local terminal emulator; fallback may be all you get.
* In CI / redirected output: protocol detection often fails; printing escape sequences might be undesirable—consider guarding on TTY detection in your app.