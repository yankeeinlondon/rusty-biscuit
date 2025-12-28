# Gotchas & fixes

## 1) “Fancy” output not showing

**Symptom:** output looks like plain `Debug` without frames/colors/snippets.

**Fix:** enable the `fancy` feature.

````toml
miette = { version = "7", features = ["fancy"] }
````

## 2) “Source code not available” / “Unable to determine source span”

Common causes:

1. You didn’t include source text in the diagnostic.
   * Add a field annotated with `#[source_code]` (often `NamedSource<String>`).
1. Your `SourceSpan` is out of bounds.
   * Ensure `offset + len <= src.len()` (bytes).
1. You computed spans using character indices rather than byte offsets.
   * Use byte offsets (parser positions are usually bytes).

## 3) Mixing `anyhow` and `miette`

**Symptom:** errors print, but you don’t get diagnostic richness and conversions are awkward.

**Fix:**

* Prefer `miette::Result` in CLIs and `.into_diagnostic()` on dependency errors.
* Avoid using `anyhow::Result` and `miette::Result` in the same module unless you must.
* Convert at boundaries explicitly.

## 4) Performance / size overhead

**Symptom:** concern about heavy formatting/deps (especially `fancy`).

**Fix / guidance:**

* Use `miette` for **user-facing** errors (CLI/parsers).
* Use lightweight errors (`thiserror`) internally; convert to `miette` at the UX boundary.
* Avoid constructing diagnostics in hot loops.

---