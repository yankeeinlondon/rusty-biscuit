# Gotchas and Solutions

### 1. The "Invisible Text" Problem

**Symptom:** SVG renders correctly, but all text is missing or falls back to an ugly default.
**Cause:** `resvg` does not bundle fonts. If it can't find the `font-family` in the system, it skips it.
**Solution:** Explicitly load your fonts into a `fontdb::Database`.

````rust
let mut fontdb = fontdb::Database::new();
fontdb.load_font_file("assets/my-font.ttf")?;

let opt = usvg::Options {
    fontdb: std::sync::Arc::new(fontdb),
    ..Default::default()
};
````

### 2. CSS Limitations

`resvg` supports a strict subset of SVG/CSS.

* **Unsupported:** JavaScript, external CSS files, `:hover` states, or complex CSS selectors.
* **Solution:** Flatten your SVGs using `svgo` with the `convertStyleToAttrs` plugin or use inline `style="..."` attributes.

### 3. Performance Bottlenecks

**Symptom:** Rendering many icons in a loop is slow.
**Cause:** Re-parsing the XML and re-creating the `fontdb` is expensive.
**Solution:**

* Cache the `usvg::Tree` for static assets.
* Reuse the `fontdb` (it is thread-safe via `Arc`).
* Only the `Pixmap` allocation and `resvg::render` should happen per frame.

### 4. Licensing (MPL-2.0)

`resvg` uses the **Mozilla Public License 2.0**.

* **Commercial use:** Allowed in closed-source apps.
* **Requirement:** If you modify `resvg`'s source code, you must share those specific file modifications. Linking to it statically does not require open-sourcing your entire project.