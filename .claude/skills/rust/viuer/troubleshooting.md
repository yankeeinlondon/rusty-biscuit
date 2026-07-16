# Troubleshooting & gotchas checklist

## 1) `print_from_file` missing at compile time

**Symptom**: “cannot find function `print_from_file` in crate `viuer`”  
**Fix**: enable feature flag:

````toml
viuer = { version = "0.11", features = ["print-file"] }
````

## 2) Sixel doesn’t work

Checklist:

* Enabled feature: `features = ["sixel"]`
* Terminal supports Sixel (verify with `is_sixel_supported()` when feature enabled)
* If unsupported, viuer will fall back.

## 3) Image displays as colored blocks / low fidelity

This is the intended **universal fallback** (lower half blocks `▄`) when advanced protocols aren’t available. Options:

* Use a terminal with Kitty/iTerm/Sixel support (Kitty, iTerm2, WezTerm, foot, etc.)
* If you must use a non-graphics terminal but want better character-art quality, consider `chafa`.

## 4) Kitty not detected even in Kitty

Common causes:

* Not running in a TTY (piped output, redirected stdout, some IDE terminals)
* Environment variables differ or detection heuristics fail

Fix options:

* Ensure you run interactively in the terminal
* Force: `Config { use_kitty: true, ..Default::default() }`

## 5) “Half height” / wrong aspect ratio

Reminder:

* `Config.height` is in terminal cells.
* Fallback uses 2 vertical pixels per cell; adjust your sizing expectations.
* When resizing to terminal, use `(rows * 2)` for pixel-ish target height.

## 6) Transparency looks wrong (black background)

* Half-block fallback cannot do true transparency.
* Try `transparent: true` + `truecolor: true`, but expect limitations unless using a graphics protocol terminal.

## 7) Rendering is slow

* Pre-resize with `viuer::resize()`
* Avoid decoding the file repeatedly in tight loops; cache the `DynamicImage`
* Prefer printing less frequently in interactive apps (e.g., only on selection change)