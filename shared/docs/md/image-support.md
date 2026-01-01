# Image Support for Terminals

Integrating Kitty image support into a Rust-based Markdown pipeline is a great way to elevate the terminal experience. Since you are already using `pulldown-cmark`, the most efficient path is to intercept image events during the parsing phase and convert them into Kitty-compatible escape codes.

### 1. The Core Strategy

The Kitty graphics protocol works by sending specialized terminal escape sequences. To implement this in your pipeline, you generally follow these steps:

1. Identify an `Event::Start(Tag::Image(...))` during Markdown parsing.
2. Fetch the image data (local path or URL).
3. Encode the image (usually as Base64) or point the terminal to the file path.
4. Wrap the data in the Kitty escape protocol: `\x1b_G... \x1b\`.

### 2. Implementation Options

You have two main paths depending on how much "manual lifting" you want to do:

#### Option A: Using `viuer` (Recommended)

The [viuer](https://crates.io/crates/viuer) crate is the industry standard for Rust terminal image rendering. It abstracts away the complex escape codes and automatically detects if the terminal supports Kitty, Sixel, or falls back to block characters.

```rust
use viuer::{print_from_file, Config};

// Inside your pulldown-cmark loop:
match event {
    Event::Start(Tag::Image(_, src, _)) => {
        let conf = Config {
            transparent: true,
            absolute_offset: false,
            // Kitty protocol is handled automatically if supported
            ..Default::with_terminal_size()
        };
        // Note: You'll need to handle async or local path resolution
        let _ = print_from_file(&src.to_string(), &conf);
    }
    _ => {}
}

```

#### Option B: Manual Kitty Encoding

If you want to keep dependencies low and strictly target Kitty, you can use the protocol directly. The basic "transfer and display" command looks like this:

**The Escape Sequence Structure:**

* **Prefix:** `\x1b_G` (Start of graphics command)
* **Control Data:** `a=T,f=100` (`a=T` means "transmit and display", `f=100` means PNG)
* **Payload:** `;` followed by Base64 encoded image data.
* **Suffix:** `\x1b\` (String Terminator)

---

### 3. Integrating with `pulldown-cmark`

Since your pipeline likely outputs a stream of strings, you should create a custom `push_tty` function that handles the conversion.

| Component | Responsibility |
| --- | --- |
| **Parser** | Extracts the `src` URL from the Markdown Image tag. |
| **Fetcher** | Resolves the path (local file vs. remote URL). |
| **Encoder** | Re-saves/resizes the image if necessary (using the `image` crate). |
| **Renderer** | Wraps the bytes in `\x1b_G` sequences and writes to `stdout`. |

### 4. Handling Constraints

* **Terminal Detection:** Before sending Kitty codes, check the `TERM` env var or use `is_terminal` to ensure you aren't dumping binary data into a terminal that doesn't support it.
* **Sizing:** Terminal cells are non-square. To prevent stretching, you may need to query the terminal size (using `terminal_size` crate) to calculate the correct aspect ratio for the Kitty `c` (columns) and `r` (rows) parameters.
* **Remote Images:** `pulldown-cmark` provides the URL string. You will need an HTTP client (like `reqwest`) to download the image to a buffer before passing it to the Kitty encoder.

---

### Suggested Next Step

Would you like me to provide a complete code example showing how to integrate `viuer` into a `pulldown-cmark` event loop?
