# Fix: UTF-8-Safe Byte Scanning in Frontmatter Fallback Helpers

## Summary

The frontmatter fallback scanners in `darkmatter/lib/src/markdown/frontmatter.rs` panic on non-ASCII input because they index the raw YAML by byte offsets and then copy single-byte slices (`yaml[pos..pos + 1]`) into a `String`. When `pos` lands inside a multi-byte UTF-8 scalar, slicing on a non-char boundary panics with "byte index N is not a char boundary." This fix updates both `protect_shell_expressions()` and `protect_interpolation_expressions()` to advance by full UTF-8 scalar boundaries while preserving all existing sentinel-detection, placeholder, and balancing behavior.

## Background / Reproduction

`parse_yaml_with_fallbacks()` falls back to the two byte-scanning helpers when raw YAML parsing fails (e.g., YAML-significant characters inside `{{...}}` or `$(...)`). Both helpers walk `yaml.as_bytes()` and, for any byte that is not part of a sentinel sequence, execute:

```rust
result.push(yaml[pos..pos + 1].chars().next().unwrap_or('?'));
pos += 1;
```

That assumes every character is one byte. Two bugs follow:

1. When `pos` lands on a continuation byte of a multi-byte scalar (e.g., the bytes of an emoji or accented character), the slice straddles a non-char boundary and panics.
2. Even when slicing succeeds, `unwrap_or('?')` silently substitutes `'?'` rather than preserving the original character — a lossy fallback that should not exist in either helper.

The bug occurs at:

- `darkmatter/lib/src/markdown/frontmatter.rs:328` — inside `protect_shell_expressions()`
- `darkmatter/lib/src/markdown/frontmatter.rs:371` — inside `protect_interpolation_expressions()`

Reproduction case (the user's exact failing input): a frontmatter block containing both an unquoted `area: {{ctx.current_package_area}}` (which forces the fallback path because `{{` is not valid raw YAML at that position) and a quoted `success.message` value beginning with the multi-byte emoji `🖥️`. The unquoted interpolation triggers the fallback; the emoji panics the scanner.

## Scope

### In scope

- `darkmatter/lib/src/markdown/frontmatter.rs`
    - `protect_shell_expressions()`
    - `protect_interpolation_expressions()`
- A targeted audit of `darkmatter/lib/src/markdown/*` for the same anti-pattern (byte-indexed scanners that copy `bytes[pos..pos + 1]` into a `String` after sentinel matches on `bytes[pos]`). If sibling scanners share the bug, fix them in the same change. If they do not, note that explicitly in the implementation notes.

### Out of scope

- Anything outside `darkmatter/lib/src/markdown/`.
- The YAML parser itself (`serde_yaml_ng`).
- Public API or behavioral changes to frontmatter parsing beyond the helper internals.
- CHANGELOG or fix-log entries.

## Implementation Requirements

- Rewrite the non-matching branch in both helpers to iterate by full UTF-8 scalars. Use `char_indices()` over the remaining slice, or read the next `char` and advance `pos` by `char.len_utf8()`. Either approach is acceptable as long as `pos` always lands on a char boundary.
- Preserve existing sentinel detection on raw bytes: `b'$'` followed by `b'('` for shell, `b'{' b'{'` and `b'}' b'}'` for interpolation. Sentinels are ASCII and remain safe to detect via `bytes[pos]` lookups.
- Preserve placeholder generation (`__DM_SHELL_N__`, `__DM_EXPR_N__`) and the existing `Vec<(String, String)>` replacement format.
- Preserve nested-bracket handling: shell tracks balanced `(` / `)` depth; interpolation tracks balanced `{{` / `}}` depth.
- Preserve unbalanced-expression behavior in `protect_shell_expressions()` (emit the unmatched tail verbatim and stop scanning rather than emitting an unterminated placeholder).
- Treat both helpers consistently: neither helper may substitute `'?'` for unmatched content. The original character must be appended unchanged.
- Audit the rest of `darkmatter/lib/src/markdown/*` for the same byte-indexed-copy pattern. If found, apply the same fix; if not, record the negative finding in implementation notes.

## Testing Requirements

All new tests live in the existing `#[cfg(test)] mod tests` block in `frontmatter.rs`.

### Helper unit tests (one per helper)

Each test calls the helper directly with non-ASCII input and asserts:

- The helper does not panic.
- ASCII sentinel sequences relevant to that helper (`$(` / `)` for shell; `{{` / `}}` for interpolation) are still detected and replaced with placeholders.
- Non-ASCII characters (emoji, accented text) appearing outside sentinel spans are preserved byte-identically in the returned `String`.

### End-to-end test

Exercises `parse_frontmatter` (or whichever existing public entry point already routes through `parse_yaml_with_fallbacks`) using the user's exact reproduction input:

- An unquoted `area: {{ctx.current_package_area}}` line.
- A quoted `success.message` value beginning with `🖥️`.

Asserts: parse succeeds, no panic, original Unicode is preserved verbatim, and the `{{...}}` template expression is preserved verbatim.

## Documentation Requirements

- Add a one-line rustdoc note on `protect_shell_expressions()` stating that non-matched content must advance by full UTF-8 scalar boundaries (e.g., via `char_indices()` or `char.len_utf8()`).
- Add the same one-line rustdoc note on `protect_interpolation_expressions()`.
- No CHANGELOG entry.
- No fix-log entry beyond this spec.

## Acceptance Criteria

- Both helpers no longer panic on multi-byte UTF-8 input.
- Non-ASCII characters round-trip byte-identically through the fallback path.
- Sentinel detection and placeholder replacement still work for ASCII inputs; all existing tests in the `tests` module continue to pass.
- The reproduction case (unquoted `area: {{ctx.current_package_area}}` plus quoted message starting with `🖥️`) parses successfully and preserves both the emoji and the `{{...}}` expression.
- No `'?'` substitution remains in either helper.
- Rustdoc UTF-8-boundary notes are present on both helpers.

## Out of Scope

- Changes to the YAML parser itself.
- Audit beyond `darkmatter/lib/src/markdown/*`.
- CHANGELOG entries.
- Public API changes.
