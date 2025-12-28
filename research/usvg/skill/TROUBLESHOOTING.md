# Troubleshooting usvg

### 1. Missing or Invisible Text

* **Cause**: No fonts loaded in `fontdb` or the specific `font-family` requested in the SVG isn't available.
* **Solution**: Always call `fontdb.load_system_fonts()` and check if your fonts are bundled.

### 2. SVG Appears "Broken" or Missing Elements

* **Cause**: `usvg` is a "Micro" library. It does not support:
  * CSS `calc()` or CSS Variables.
  * Complex CSS selectors (e.g., `:hover`, `nth-child`).
  * SMIL Animations (`<animate>`).
* **Solution**: Pre-process the SVG using a browser-based tool or stick to static SVG 1.1 features.

### 3. Coordinate Space Mismatch

* **Issue**: The output image is empty or the SVG is clipped.
* **Check**: Look at `tree.size()`. If the SVG lacks `width`/`height` and `viewBox`, `usvg` might resolve to a 0x0 or default size. Always provide a `viewBox` in your source SVG for best results.

### 4. Memory Usage with High-Res Images

* **Issue**: SVGs with many large embedded `<image>` tags.
* **Note**: `usvg` decodes images during parsing. If you are parsing many SVGs in parallel, monitor your RAM.

### 5. Licensing

* `usvg` uses **MPL-2.0**.
* You can use it in closed-source apps, but modifications to `usvg` itself must be shared. Linking against it does not require you to open-source your own code.

````
````