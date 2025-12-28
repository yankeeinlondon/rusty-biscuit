The `usvg` crate is unique because it doesn't just parse SVG; it **simplifies** and **normalizes** them (converting shapes to paths, resolving CSS, etc.) to make rendering easier.

Here are the most comparable libraries in the Rust ecosystem, ranging from low-level parsers to full rendering engines.

---

### 1. librsvg

`librsvg` is the industry-standard library used by the GNOME project to render SVG vector graphics. While originally written in C, it has been largely rewritten in Rust. It is the most feature-complete and spec-compliant SVG engine available.

* **Pros:**
  
  * Unrivaled compliance with SVG 1.1 and 2.0 specifications.
  * Highly mature and battle-tested in Linux desktop environments.
  * Handles complex CSS and filter effects better than almost any other library.
* **Cons:**
  
  * Heavy dependency chain (requires C libraries like Cairo, Pango, and GObject).
  * Difficult to use in a "pure Rust" or cross-compilation environment (e.g., WASM).
  * Large binary size.
* **Repo:** [https://gitlab.gnome.org/GNOME/librsvg](https://gitlab.gnome.org/GNOME/librsvg)

* **Docs.rs:** [https://docs.rs/librsvg/latest/librsvg/](https://docs.rs/librsvg/latest/librsvg/)

* **Doc Site:** [https://gnome.pages.gitlab.gnome.org/librsvg/devel/librsvg/](https://gnome.pages.gitlab.gnome.org/librsvg/devel/librsvg/)

---

### 2. svg (crate)

The `svg` crate is a simple, low-level parser and generator for SVG files. Unlike `usvg`, it does not attempt to "simplify" the SVG or resolve CSS; it provides an event-based parser and a DOM-like structure for manipulation.

* **Pros:**
  
  * Pure Rust with zero dependencies.
  * Very lightweight and fast for simple parsing/generation tasks.
  * Excellent for programmatically creating SVGs from scratch.
* **Cons:**
  
  * Does not "understand" the SVG (e.g., it won't tell you the absolute coordinates of a relative path).
  * No support for CSS styling or complex SVG normalization.
  * Requires the developer to handle all the math and logic of SVG elements.
* **Repo:** [https://github.com/bodil/svg](https://github.com/bodil/svg)

* **Docs.rs:** [https://docs.rs/svg/latest/svg/](https://docs.rs/svg/latest/svg/)

---

### 3. svgtypes

Also maintained by the author of `usvg`, `svgtypes` is a collection of low-level parsers for SVG data types. It doesn't parse the whole file into a tree, but it helps you parse specific strings like path data, transforms, or color values.

* **Pros:**
  
  * Extremely high performance and memory efficient.
  * Zero-copy parsing where possible.
  * The "building blocks" for creating your own SVG engine.
* **Cons:**
  
  * It is not a full SVG parser; it only parses attribute values.
  * Does not handle XML structure or CSS.
* **Repo:** [https://github.com/RazrFalcon/resvg/tree/master/svgtypes](https://github.com/RazrFalcon/resvg/tree/master/svgtypes)

* **Docs.rs:** [https://docs.rs/svgtypes/latest/svgtypes/](https://docs.rs/svgtypes/latest/svgtypes/)

---

### 4. Vello (vello_svg)

`Vello` is a high-performance, experimental vector graphics engine that uses compute shaders (via `wgpu`). It has a companion crate, `vello_svg`, specifically for parsing and rendering SVGs.

* **Pros:**
  
  * Hardware-accelerated rendering using the GPU.
  * Part of the modern "Linebender" ecosystem (high performance focused).
  * Pure Rust and works on any platform supported by `wgpu` (Web, Metal, Vulkan, DX12).
* **Cons:**
  
  * Still in early development; API is subject to frequent change.
  * SVG support is currently partial (many complex filters or features aren't implemented yet).
  * Heavier runtime requirement (GPU drivers/API access).
* **Repo:** [https://github.com/linebender/vello](https://github.com/linebender/vello)

* **Docs.rs:** [https://docs.rs/vello/latest/vello/](https://docs.rs/vello/latest/vello/)

* **Doc Site:** [https://linebender.org/vello/](https://linebender.org/vello/)

---

### 5. roxmltree

While `roxmltree` is a general XML parser, it is the underlying engine that `usvg` uses. If your goal is to handle SVG files that have non-standard elements or you want total control over the XML tree without the "normalization" `usvg` forces on you, this is the tool.

* **Pros:**
  
  * One of the fastest XML parsers in the Rust ecosystem.
  * Very small footprint and safe (no `unsafe` blocks).
  * Maintains the original structure of the document perfectly.
* **Cons:**
  
  * Not SVG-specific; it treats `<path d="..." />` as just another tag with a string.
  * No built-in vector math or graphics logic.
* **Repo:** [https://github.com/RazrFalcon/roxmltree](https://github.com/RazrFalcon/roxmltree)

* **Docs.rs:** [https://docs.rs/roxmltree/latest/roxmltree/](https://docs.rs/roxmltree/latest/roxmltree/)

---

### Summary Table

|Library|Primary Use Case|Complexity|Pure Rust?|
|:------|:---------------|:---------|:---------|
|**usvg**|Normalizing/Simplifying SVG for rendering|Medium|Yes|
|**librsvg**|High-fidelity rendering (Linux/Standard)|High|No (C deps)|
|**svg**|Simple XML parsing/generation|Low|Yes|
|**svgtypes**|Parsing attribute strings (Paths/Colors)|Low|Yes|
|**Vello**|GPU-accelerated rendering|High|Yes|