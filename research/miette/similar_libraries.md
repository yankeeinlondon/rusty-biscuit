`miette` is a popular crate because it combines three things: an error trait, a diagnostic reporter, and beautiful terminal output (with source code snippets).

Depending on whether you need a library to **define** errors or a library to **render** errors (diagnostics), here are the most comparable libraries in the Rust ecosystem.

---

### 1. Ariadne

Ariadne is arguably the closest competitor to `miette` regarding the visual quality of error reports. It focuses strictly on the "pretty printing" aspect of diagnostics and source code snippets.

* **Summary:** A library for high-quality, flexible terminal diagnostics. It allows for multi-line annotations, multiple colors, and can pull snippets from multiple files in a single report. It is known for its beautiful, "cleaner" visual style compared to the Rust compiler's default look.
* **Pros:**
  * Produces arguably the most beautiful terminal output in the ecosystem.
  * Extremely flexible; supports complex overlapping labels and multi-file diagnostics.
  * Does not force a specific error-handling pattern (like `thiserror` or `anyhow`).
* **Cons:**
  * The API is more "manual" and verbose than `miette` (it doesn't use derive macros).
  * Strictly a renderer; it doesn't provide an "Error" trait or integration with `Result` like `miette` does.
* **URLs:**
  * **Repo:** [https://github.com/zesterer/ariadne](https://github.com/zesterer/ariadne)
  * **Docs.rs:** [https://docs.rs/ariadne/latest/ariadne/](https://docs.rs/ariadne/latest/ariadne/)

---

### 2. Codespan-reporting

For a long time, `codespan-reporting` was the gold standard for anyone building a language frontend or compiler in Rust.

* **Summary:** A crate designed to render beautiful diagnostics with a look and feel very similar to the Rust compiler (`rustc`). It provides a data structure for files and a "diagnostic" struct that you fill with labels and notes.
* **Pros:**
  * Industry-tested and very stable.
  * Familiar output format for Rust users.
  * Excellent performance and low overhead.
* **Cons:**
  * Maintenance has slowed down significantly.
  * The API is imperative and requires you to manage a "File Database" manually.
  * Requires more boilerplate to get a basic error on screen than `miette`.
* **URLs:**
  * **Repo:** [https://github.com/brendanzab/codespan](https://github.com/brendanzab/codespan)
  * **Docs.rs:** [https://docs.rs/codespan-reporting/latest/codespan_reporting/](https://docs.rs/codespan-reporting/latest/codespan_reporting/)

---

### 3. Eyre (and Color-eyre)

While `miette` is often used for compilers/CLIs, `eyre` is the main alternative for **application-level** error handling.

* **Summary:** `eyre` is a fork of the famous `anyhow` crate. It provides a `Report` type for easy error propagation but is designed to be customizable. The `color-eyre` handler specifically provides beautiful, colorful error reports, backtraces, and "suggestions" for users.
* **Pros:**
  * The standard choice for "Applications" (rather than libraries).
  * Excellent support for "Suggestions" (e.g., "Help: try doing X instead").
  * Seamless integration with backtraces and panic hooks.
* **Cons:**
  * Does not natively render source code snippets (spans) like `miette` or `ariadne`.
  * Focused more on "what went wrong in the code" rather than "what did the user type wrong."
* **URLs:**
  * **Repo:** [https://github.com/eyre-rs/eyre](https://github.com/eyre-rs/eyre)
  * **Docs.rs:** [https://docs.rs/eyre/latest/eyre/](https://docs.rs/eyre/latest/eyre/)
  * **Documentation Site:** [https://eyre.rs/](https://eyre.rs/)

---

### 4. Annotate-snippets

If you want exactly what the Rust compiler uses, this is the crate.

* **Summary:** This crate is maintained by the Rust project and is used by `rustc` itself to render error messages. It is a very "pure" implementation of the Rust diagnostic style.
* **Pros:**
  * Guarantees a 1:1 look with the official Rust compiler output.
  * Extremely stable and well-maintained by the core team.
  * Very low dependency footprint.
* **Cons:**
  * The API is somewhat clunky and data-heavy (it requires building up a complex `Snippet` struct).
  * Not as "user-friendly" or "magical" to implement as `miette`'s derive macros.
* **URLs:**
  * **Repo:** [https://github.com/rust-lang/annotate-snippets-rs](https://github.com/rust-lang/annotate-snippets-rs)
  * **Docs.rs:** [https://docs.rs/annotate-snippets/latest/annotate_snippets/](https://docs.rs/annotate-snippets/latest/annotate_snippets/)

---

### 5. Thiserror

`miette` is often described as "`thiserror` plus diagnostics." If you find `miette` too heavy, you might use `thiserror` alone.

* **Summary:** The standard macro-based crate for defining custom error types in Rust. It generates the `Display` and `Error` trait implementations for you.
* **Pros:**
  * The most widely used error library in the entire Rust ecosystem.
  * Zero runtime cost (it just generates code).
  * Extremely lightweight.
* **Cons:**
  * No built-in visual diagnostic rendering (no colors, no source code snippets).
  * You must combine it with another library (like `ariadne`) if you want "fancy" reports.
* **URLs:**
  * **Repo:** [https://github.com/dtolnay/thiserror](https://github.com/dtolnay/thiserror)
  * **Docs.rs:** [https://docs.rs/thiserror/latest/thiserror/](https://docs.rs/thiserror/latest/thiserror/)

---

### Summary Comparison Table

|Feature|Miette|Ariadne|Codespan-reporting|Eyre/Color-eyre|
|:------|:-----|:------|:-----------------|:--------------|
|**Best For**|All-in-one CLIs|Beautiful layout|Compilers|Apps/Services|
|**Derive Macros**|Yes|No|No|No|
|**Source Snippets**|Yes|Yes (Excellent)|Yes|No|
|**Philosophy**|"Magic" & Fast|Manual & Precise|Stable & Standard|Context & Backtrace|