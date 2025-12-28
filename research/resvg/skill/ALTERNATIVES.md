# Alternatives

While `resvg` is the best general-purpose choice, consider these alternatives for specific needs:

### 1. Vello (GPU-Accelerated)

* **Use when:** You need maximum performance for complex, high-resolution scenes and have access to a modern GPU (WGPU).
* **Cons:** Still experimental; API changes frequently.

### 2. librsvg (The Veteran)

* **Use when:** You need maximum legacy support or output formats like PDF/PS/SVG via Cairo. Used by GNOME and Wikipedia.
* **Cons:** Heavy C-library dependencies (Cairo, Pango); difficult to cross-compile.

### 3. ThorVG (Embedded)

* **Use when:** You are on extremely low-power hardware (microcontrollers) or need Lottie animation support.
* **Cons:** Lower spec compliance; requires C++ FFI.

### 4. tiny-skia (Direct Drawing)

* **Use when:** You don't need to parse SVG files, but just want to draw shapes, paths, and gradients directly in Rust.
* **Note:** `resvg` uses this internally.