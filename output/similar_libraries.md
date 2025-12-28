`arboard` (Image and Text Clipboard Handling) is currently the most popular and well-maintained general-purpose clipboard library in the Rust ecosystem. However, depending on your specific needs—such as wanting a smaller dependency tree, focusing strictly on Wayland, or needing a simple wrapper around system binaries—there are several alternatives.

Here are the most comparable libraries to `arboard`:

---

### 1. copypasta

`copypasta` is a fork of the original (and now unmaintained) `rust-clipboard` crate. It is primarily maintained by the Alacritty (terminal emulator) team. It is the most direct cross-platform competitor to `arboard`, though it is more focused on text than images.

* **Summary:** A cross-platform library for getting and setting clipboard content, supporting Windows, macOS, and Linux (X11 and Wayland).
* **Pros:**
  * Battle-tested; used by Alacritty and other high-profile CLI tools.
  * Lighter-weight than `arboard` if you only need basic text support.
  * Handles the complexities of X11 and Wayland selection buffers well.
* **Cons:**
  * Image support is not as robust or ergonomic as `arboard`.
  * Maintenance can be slow, as it is primarily updated to serve the needs of Alacritty.
* **Links:**
  * **Repo:** [github.com/alacritty/copypasta](https://github.com/alacritty/copypasta)
  * **Docs.rs:** [docs.rs/copypasta](https://docs.rs/copypasta/)

---

### 2. cli-clipboard

This crate takes a different approach by acting as a wrapper around the standard system clipboard utilities rather than implementing the protocols via FFI.

* **Summary:** A cross-platform library that shells out to system binaries like `pbcopy`/`pbpaste` (macOS), `xclip`/`xsel` (Linux), and `powershell` (Windows).
* **Pros:**
  * Extremely small dependency tree (minimal Rust compilation overhead).
  * Avoids complex FFI issues or linking against X11/Wayland C-libraries.
* **Cons:**
  * Requires runtime dependencies (the user must have `xclip` or `xsel` installed on Linux).
  * Slower performance due to process spawning.
  * Less reliable for binary/image data.
* **Links:**
  * **Repo:** [github.com/p-e-w/cli-clipboard](https://github.com/p-e-w/cli-clipboard)
  * **Docs.rs:** [docs.rs/cli-clipboard](https://docs.rs/cli-clipboard/)

---

### 3. wl-clipboard-rs

If your application is targeting Linux users specifically (especially those on modern desktops like GNOME, KDE, or Sway), this is the "gold standard" for Wayland-native clipboard management.

* **Summary:** A pure-Rust implementation of the Wayland clipboard protocols.
* **Pros:**
  * Native Wayland support without needing an X11 compatibility layer (XWayland).
  * Supports advanced Wayland features like "primary" selection and MIME type negotiation.
  * Very active maintenance.
* **Cons:**
  * Linux-only (Wayland specifically).
  * Does not support Windows or macOS.
* **Links:**
  * **Repo:** [github.com/YaLTeR/wl-clipboard-rs](https://github.com/YaLTeR/wl-clipboard-rs)
  * **Docs.rs:** [docs.rs/wl-clipboard-rs](https://docs.rs/wl-clipboard-rs/)

---

### 4. clipboard-win

If you are building a Windows-only utility, using a cross-platform abstraction like `arboard` can sometimes add unnecessary bloat. `clipboard-win` provides direct access to the Windows API.

* **Summary:** A specialized, high-performance library for the Windows OS clipboard.
* **Pros:**
  * Provides deep access to Windows-specific clipboard formats.
  * Very lightweight with no dependencies outside of the standard Windows API.
  * Extremely stable.
* **Cons:**
  * Windows only.
  * The API is more low-level than `arboard`.
* **Links:**
  * **Repo:** [github.com/vojtechknopp/clipboard-win](https://github.com/vojtechknopp/clipboard-win)
  * **Docs.rs:** [docs.rs/clipboard-win](https://docs.rs/clipboard-win/)

---

### Summary Table: Which one should you choose?

|Requirement|Recommended Library|
|:----------|:------------------|
|**General purpose (Text & Images)**|`arboard`|
|**Stable Text (used in Alacritty)**|`copypasta`|
|**Minimal compile time / No FFI**|`cli-clipboard`|
|**Linux Wayland Native**|`wl-clipboard-rs`|
|**Windows Specialized**|`clipboard-win`|